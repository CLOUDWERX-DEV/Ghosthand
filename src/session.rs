//! A session: ask the gate, move the pointer and press keys through a list of actions, always put everything back.
//!
//! Used by the command line and by the MCP server, so both behave the same and share every safety.
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use crate::actions::{self, Ctx};
use crate::config::Config;
use crate::gate::{self, Gate, Request, EXIT_FAILED, EXIT_NO_GATE, EXIT_REVOKED, EXIT_USAGE};
use crate::geometry::Info;
use crate::keyboard::Keyboard;
use crate::layout::Layout;
use crate::paths;
use crate::pointer::{read_last, Pointer};
use crate::uinput::Sink;
use crate::util::Stop;

#[derive(Debug, Default)]
pub struct Outcome {
    pub code: i32,
    pub message: String,
    /// What pos, info and focus printed.
    pub printed: Vec<String>,
}

fn fail(code: i32, message: impl Into<String>) -> Outcome {
    Outcome { code, message: message.into(), printed: vec![] }
}

/// The position stream the overlay follows: `begin WHO`, `pos X Y OUTPUT` (logical pixels), `end`.
pub trait Stream {
    fn begin(&mut self, who: &str);
    fn pos(&mut self, x: f64, y: f64, output: &str);
    fn end(&mut self);
}

/// Where the virtual devices come from: the daemon, or devices of our own.
pub trait Devices {
    /// Start making the devices a run will need, all at once (a new device takes about a second to be picked up). Optional.
    fn warm(&mut self, _pointer: bool, _keyboard: bool) {}
    fn pointer(&mut self) -> Result<Box<dyn Sink>, String>;
    fn keyboard(&mut self) -> Result<Box<dyn Sink>, String>;
}

pub struct FileStream(Option<std::fs::File>);

impl FileStream {
    pub fn open() -> FileStream {
        paths::ensure_runtime_dir();
        FileStream(std::fs::OpenOptions::new().create(true).append(true).open(paths::stream_file()).ok())
    }
}

impl Stream for FileStream {
    fn begin(&mut self, who: &str) { if let Some(f) = self.0.as_mut() { let _ = writeln!(f, "begin {who}"); } }
    fn pos(&mut self, x: f64, y: f64, output: &str) { if let Some(f) = self.0.as_mut() { let _ = writeln!(f, "pos {x:.1} {y:.1} {output}"); } }
    fn end(&mut self) { if let Some(f) = self.0.as_mut() { let _ = writeln!(f, "end"); } }
}

pub struct Services {
    pub gate: Option<Box<dyn Gate>>,
    pub devices: Box<dyn Devices>,
    pub locked: Box<dyn Fn() -> bool>,
    pub info: Box<dyn Fn() -> Result<Info, String>>,
    pub stream: Rc<RefCell<dyn Stream>>,
    /// Was the pointer taken back since the run started? A file check: no process is spawned between steps.
    pub revoked: Box<dyn Fn() -> bool>,
    pub gate_patience: Option<f64>,
}

/// Run the actions. The outcome's code is the exit code (0 done, see gate.rs for the rest).
pub fn execute(who: &str, reason: &str, countdown: u32, raw: &[String], svc: &mut Services, cfg: &Config, lay: &Layout) -> Outcome {
    let steps = match actions::parse_all(raw, lay, cfg) {
        Ok(s) => s,
        Err(e) => return fail(EXIT_USAGE, e),
    };
    let info = match (svc.info)() {
        Ok(i) => i,
        Err(e) => return fail(EXIT_FAILED, format!("couldn't read the screen: {e}")),
    };
    let mut out = Vec::new();
    let revoked = &svc.revoked;
    let mut check = || if revoked() { Err(Stop::Revoked) } else { Ok(()) };
    if !actions::touches_input(&steps) {
        // nothing touches the mouse or keyboard: no gate, no device
        for s in &steps {
            let mut ctx = Ctx { pointer: None, keyboard: None, info: &info, out: &mut out, check: &mut || Ok(()), cfg };
            if let Err(Stop::Failed(e)) = actions::run(s, &mut ctx) {
                return fail(EXIT_FAILED, e);
            }
        }
        return Outcome { printed: out, ..Default::default() };
    }
    if (svc.locked)() {
        crate::sessions::end(); // locking the screen ends a session
        return fail(EXIT_FAILED, "the session is locked: nothing is touched");
    }
    let scope = actions::scope(&steps);
    let in_session = crate::sessions::current().is_some_and(|s| s.covers(&scope));
    if !in_session {
        let Some(gate) = svc.gate.as_mut() else {
            return fail(EXIT_NO_GATE, "Ghosthand: nothing is there to ask you, so nothing is touched (there is no way around the gate). Run `ghosthand overlay`, or install zenity, yad or kdialog");
        };
        let req = Request { who: who.into(), reason: reason.into(), seconds: countdown, scope, detail: actions::detail(&steps, 140) };
        if let Err(e) = gate::wait_for_allow(gate.as_mut(), &req, svc.gate_patience) {
            return fail(e.code, e.message);
        }
    }
    let stream = svc.stream.clone();
    stream.borrow_mut().begin(who);
    let (mut pointer, mut keyboard, mut done) = (None::<Pointer>, None::<Keyboard>, 0usize);
    svc.devices.warm(actions::uses_pointer(&steps), actions::uses_keyboard(&steps)); // only now that it is allowed
    let outcome = (|| -> Result<(), Stop> {
        if actions::uses_pointer(&steps) {
            let (s, name, scale) = (stream.clone(), info.name.clone(), info.scale);
            let sink = svc.devices.pointer()?;
            pointer = Some(Pointer::new(sink, &info, Some(Box::new(move |x, y| { let _ = scale; s.borrow_mut().pos(x, y, &name); }))));
        } else if let Some((x, y)) = read_last() {
            stream.borrow_mut().pos(x / info.scale, y / info.scale, &info.name); // keyboard only: still show the tag, where the pointer was last seen
        }
        if actions::uses_keyboard(&steps) {
            keyboard = Some(Keyboard::new(svc.devices.keyboard()?, lay.clone(), cfg.type_delay_seconds()));
        }
        for s in &steps {
            let mut ctx = Ctx { pointer: pointer.as_mut(), keyboard: keyboard.as_mut(), info: &info, out: &mut out, check: &mut check, cfg };
            actions::run(s, &mut ctx)?;
            done += 1;
        }
        Ok(())
    })();
    if let Some(k) = keyboard { k.close(); }
    if let Some(p) = pointer { p.close(); }
    stream.borrow_mut().end();
    if in_session {
        crate::sessions::touch();
    }
    match outcome {
        Ok(()) => Outcome { printed: out, ..Default::default() },
        Err(Stop::Revoked) => Outcome { code: EXIT_REVOKED, message: format!("Ghosthand: taken back after {done} of {} actions.", steps.len()), printed: out },
        Err(Stop::Failed(e)) => Outcome { code: EXIT_FAILED, message: format!("action {} failed: {e}", done + 1), printed: out },
    }
}

/// What `ghosthand session start` asks for.
pub struct SessionRequest {
    pub who: String,
    pub reason: String,
    /// "mouse", "keyboard" or "mouse and keyboard".
    pub scope: String,
    pub seconds: f64,
    pub idle: f64,
    pub countdown: u32,
}

/// Ask once for a stretch of work. On Allow, runs that fit the scope skip the card until it ends (see sessions.rs).
pub fn start_session(req: &SessionRequest, svc: &mut Services) -> Outcome {
    if (svc.locked)() {
        return fail(EXIT_FAILED, "the session is locked: nothing is touched");
    }
    let Some(gate) = svc.gate.as_mut() else {
        return fail(EXIT_NO_GATE, "Ghosthand: nothing is there to ask you, so nothing is touched (there is no way around the gate). Run `ghosthand overlay`, or install zenity, yad or kdialog");
    };
    // the card reads "session ..." from the detail line, and then offers just Allow and Deny (no "Allow for N min": the session is the time)
    let detail = format!("session for {} (stops after {} with no use)", crate::sessions::describe(req.seconds), crate::sessions::describe(req.idle));
    let ask = Request { who: req.who.clone(), reason: req.reason.clone(), seconds: req.countdown, scope: req.scope.clone(), detail };
    if let Err(e) = gate::wait_for_allow(gate.as_mut(), &ask, svc.gate_patience) {
        return fail(e.code, e.message);
    }
    let session = crate::sessions::Session::new(&req.who, &req.reason, &req.scope, req.seconds, req.idle);
    if let Err(e) = crate::sessions::start(&session) {
        return fail(EXIT_FAILED, format!("couldn't start the session: {e}"));
    }
    svc.stream.borrow_mut().end(); // the card was "active": hand over to the pill
    Outcome { printed: vec![serde_json::json!({"session": "started", "scope": req.scope, "minutes": (req.seconds / 60.0).round()}).to_string()], ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::{EXIT_DENIED, EXIT_USAGE};
    use crate::layout;
    use crate::uinput::{Recorder, EV_KEY};
    use std::collections::VecDeque;

    #[derive(Default)]
    struct Log(Vec<String>);

    struct Fake { answers: VecDeque<Option<&'static str>>, asked: Rc<RefCell<Vec<Request>>> }

    impl Gate for Fake {
        fn ask(&mut self, r: &Request) -> Option<String> { self.asked.borrow_mut().push(r.clone()); self.answers.pop_front().flatten().map(String::from) }
        fn status(&mut self) -> Option<String> { self.answers.pop_front().flatten().map(String::from) }
        fn deny(&mut self) {}
    }

    struct Dev { p: Recorder, k: Recorder, made: Rc<RefCell<Vec<&'static str>>> }

    impl Devices for Dev {
        fn pointer(&mut self) -> Result<Box<dyn Sink>, String> { self.made.borrow_mut().push("pointer"); Ok(Box::new(self.p.clone())) }
        fn keyboard(&mut self) -> Result<Box<dyn Sink>, String> { self.made.borrow_mut().push("keyboard"); Ok(Box::new(self.k.clone())) }
    }

    struct Rec(Rc<RefCell<Log>>);

    impl Stream for Rec {
        fn begin(&mut self, who: &str) { self.0.borrow_mut().0.push(format!("begin {who}")); }
        fn pos(&mut self, x: f64, y: f64, o: &str) { self.0.borrow_mut().0.push(format!("pos {x:.1} {y:.1} {o}")); }
        fn end(&mut self) { self.0.borrow_mut().0.push("end".into()); }
    }

    struct T { svc: Services, asked: Rc<RefCell<Vec<Request>>>, made: Rc<RefCell<Vec<&'static str>>>, stream: Rc<RefCell<Log>>, p: Recorder, k: Recorder }

    fn rig(answers: &[Option<&'static str>]) -> T {
        crate::util::FAST.store(true, std::sync::atomic::Ordering::Relaxed);
        crate::paths::use_temp_runtime_dir();
        let (asked, made, log) = (Rc::new(RefCell::new(vec![])), Rc::new(RefCell::new(vec![])), Rc::new(RefCell::new(Log::default())));
        let (p, k) = (Recorder::default(), Recorder::default());
        let svc = Services {
            gate: Some(Box::new(Fake { answers: answers.iter().copied().collect(), asked: asked.clone() })),
            devices: Box::new(Dev { p: p.clone(), k: k.clone(), made: made.clone() }),
            locked: Box::new(|| false),
            info: Box::new(|| Ok(Info { name: "X".into(), width: 1000, height: 800, scale: 1.0 })),
            stream: Rc::new(RefCell::new(Rec(log.clone()))),
            revoked: Box::new(|| false),
            gate_patience: Some(0.2),
        };
        T { svc, asked, made, stream: log, p, k }
    }

    fn run(t: &mut T, actions: &[&str]) -> Outcome {
        let raw: Vec<String> = actions.iter().map(|s| s.to_string()).collect();
        execute("Claude", "why", 0, &raw, &mut t.svc, &Config::default(), &layout::us())
    }

    #[test]
    fn a_typo_never_reaches_the_gate_or_a_device() {
        let mut t = rig(&[]);
        let r = run(&mut t, &["jump 1 2", "clikc"]);
        assert_eq!(r.code, EXIT_USAGE);
        assert!(t.asked.borrow().is_empty() && t.made.borrow().is_empty());
    }

    #[test]
    fn a_denied_request_never_makes_a_device() {
        let mut t = rig(&[Some("asking"), Some("denied")]);
        assert_eq!(run(&mut t, &["jump 1 2"]).code, EXIT_DENIED);
        assert!(t.made.borrow().is_empty());
    }

    #[test]
    fn no_gate_at_all_is_a_refusal() {
        let mut t = rig(&[]);
        t.svc.gate = None;
        let r = run(&mut t, &["jump 1 2"]);
        assert_eq!(r.code, EXIT_NO_GATE);
        assert!(t.made.borrow().is_empty());
    }

    #[test]
    fn a_locked_session_is_refused() {
        let mut t = rig(&[Some("allowed")]);
        t.svc.locked = Box::new(|| true);
        assert_eq!(run(&mut t, &["jump 1 2"]).code, EXIT_FAILED);
        assert!(t.asked.borrow().is_empty(), "it doesn't even ask");
    }

    #[test]
    fn an_allowed_run_moves_streams_and_cleans_up() {
        let mut t = rig(&[Some("allowed")]);
        let r = run(&mut t, &["jump 10 20", "click"]);
        assert_eq!(r.code, 0, "{}", r.message);
        assert_eq!(t.p.0.borrow()[0].2, 327, "10 of 1000 px");
        let log = &t.stream.borrow().0;
        assert_eq!(log[0], "begin Claude");
        assert_eq!(log[1], "pos 10.0 20.0 X");
        assert_eq!(log.last().unwrap(), "end");
        assert_eq!(*t.made.borrow(), vec!["pointer"], "no keyboard was made");
    }

    #[test]
    fn keyboard_only_runs_make_no_pointer_and_the_card_shows_what_is_typed() {
        let mut t = rig(&[Some("allowed")]);
        let r = run(&mut t, &["type hi\\n", "key ctrl+s"]);
        assert_eq!(r.code, 0, "{}", r.message);
        assert_eq!(*t.made.borrow(), vec!["keyboard"]);
        let asked = t.asked.borrow();
        assert_eq!(asked[0].scope, "keyboard");
        assert!(asked[0].detail.contains("type: \"hi\\n\"") && asked[0].detail.contains("key: ctrl+s"));
        assert!(t.k.0.borrow().contains(&(EV_KEY, 35, 1)), "h was pressed");
    }

    #[test]
    fn mouse_and_keyboard_asks_for_both() {
        let mut t = rig(&[Some("allowed")]);
        run(&mut t, &["click 1 1", "type a"]);
        assert_eq!(t.asked.borrow()[0].scope, "mouse and keyboard");
        assert_eq!(*t.made.borrow(), vec!["pointer", "keyboard"]);
    }

    #[test]
    fn a_revoke_mid_run_stops_it_and_still_cleans_up() {
        let mut t = rig(&[Some("allowed")]);
        let n = Rc::new(RefCell::new(0));
        let n2 = n.clone();
        t.svc.revoked = Box::new(move || { *n2.borrow_mut() += 1; *n2.borrow() > 2 });
        let r = run(&mut t, &["jump 1 1", "jump 2 2", "jump 3 3", "jump 4 4"]);
        assert_eq!(r.code, EXIT_REVOKED);
        assert!(r.message.contains("taken back"));
        assert_eq!(t.stream.borrow().0.last().unwrap(), "end");
    }

    #[test]
    fn held_keys_and_buttons_are_released_even_when_an_action_fails() {
        let mut t = rig(&[Some("allowed")]);
        let r = run(&mut t, &["keydown shift", "down", "shot /nonexistent-dir/x.png"]);
        assert_eq!(r.code, EXIT_FAILED);
        assert_eq!(t.k.0.borrow().last(), Some(&(EV_KEY, 42, 0)));
        assert_eq!(t.p.0.borrow().last(), Some(&(EV_KEY, crate::uinput::BTN_LEFT, 0)));
    }

    #[test]
    fn info_and_focus_need_no_gate() {
        let mut t = rig(&[]);
        let r = run(&mut t, &["info"]);
        assert_eq!(r.code, 0);
        assert!(r.printed[0].contains("\"scale\":1.0"));
        assert!(t.asked.borrow().is_empty());
    }

    fn session_request(scope: &str) -> SessionRequest {
        SessionRequest { who: "Claude".into(), reason: "UI testing".into(), scope: scope.into(), seconds: 1800.0, idle: 600.0, countdown: 0 }
    }

    #[test]
    fn a_session_asks_once_then_runs_skip_the_card() {
        let mut t = rig(&[Some("allowed")]);
        let r = start_session(&session_request("mouse and keyboard"), &mut t.svc);
        assert_eq!(r.code, 0, "{}", r.message);
        assert!(t.asked.borrow()[0].detail.starts_with("session for 30 min"), "the card says it's a session and for how long");
        assert_eq!(t.asked.borrow()[0].scope, "mouse and keyboard");
        t.asked.borrow_mut().clear();
        // no answers left in the gate: a run that needed one would be refused
        let a = run(&mut t, &["jump 10 10", "type hi"]);
        assert_eq!(a.code, 0, "{}", a.message);
        assert!(t.asked.borrow().is_empty(), "inside the session nothing was asked");
        assert_eq!(*t.made.borrow(), vec!["pointer", "keyboard"]);
    }

    #[test]
    fn a_session_never_covers_more_than_it_was_approved_for() {
        let mut t = rig(&[Some("allowed"), Some("denied")]);
        assert_eq!(start_session(&session_request("mouse"), &mut t.svc).code, 0);
        assert_eq!(run(&mut t, &["jump 1 1"]).code, 0, "the mouse is covered");
        t.asked.borrow_mut().clear();
        let typed = run(&mut t, &["type hi"]);
        assert_eq!(t.asked.borrow().len(), 1, "typing asks again");
        assert_ne!(typed.code, 0);
    }

    #[test]
    fn a_denied_session_does_not_start_and_a_revoke_ends_one() {
        let mut t = rig(&[Some("asking"), Some("denied")]);
        assert_eq!(start_session(&session_request("mouse"), &mut t.svc).code, EXIT_DENIED);
        assert!(crate::sessions::current().is_none());
        let mut t = rig(&[Some("allowed")]);
        start_session(&session_request("mouse"), &mut t.svc);
        assert!(crate::sessions::current().is_some());
        crate::gate::revoke_local();
        assert!(crate::sessions::current().is_none(), "a revoke ends it");
    }

    #[test]
    fn a_locked_screen_ends_a_session() {
        let mut t = rig(&[Some("allowed")]);
        start_session(&session_request("mouse"), &mut t.svc);
        t.svc.locked = Box::new(|| true);
        assert_eq!(run(&mut t, &["jump 1 1"]).code, EXIT_FAILED);
        assert!(crate::sessions::current().is_none());
    }

    #[test]
    fn a_session_is_not_started_with_nothing_to_ask_with_or_a_locked_screen() {
        let mut t = rig(&[]);
        t.svc.gate = None;
        assert_eq!(start_session(&session_request("mouse"), &mut t.svc).code, EXIT_NO_GATE);
        let mut t = rig(&[Some("allowed")]);
        t.svc.locked = Box::new(|| true);
        assert_eq!(start_session(&session_request("mouse"), &mut t.svc).code, EXIT_FAILED);
    }

    #[test]
    fn a_device_that_cannot_be_made_is_reported() {
        struct Broken;
        impl Devices for Broken {
            fn pointer(&mut self) -> Result<Box<dyn Sink>, String> { Err("can't open /dev/uinput".into()) }
            fn keyboard(&mut self) -> Result<Box<dyn Sink>, String> { Err("no".into()) }
        }
        let mut t = rig(&[Some("allowed")]);
        t.svc.devices = Box::new(Broken);
        let r = run(&mut t, &["jump 1 1"]);
        assert_eq!(r.code, EXIT_FAILED);
        assert!(r.message.contains("/dev/uinput"));
    }
}
