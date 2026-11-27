//! The gate without the overlay: a plain dialog from zenity, yad or kdialog.
//!
//! For sessions where the Quickshell overlay can't run (X11, GNOME, anything without wlr-layer-shell). It asks the same question with the same
//! answers (Allow, Allow for N min, Deny, silence is a no), and `ghosthand revoke` still takes everything back, through a file in the runtime
//! directory. What it can't do is draw the tag and ring that follow the cursor: a notification says the pointer is in use instead.
use std::io::Read;
use std::process::{Child, Command, Stdio};

use crate::config::Config;
use crate::gate::{revoked_since, Gate, Request};
use crate::paths;
use crate::util::{now, read_float, which};

/// In order of preference. notify-send is the last resort: it needs a notification daemon that shows action buttons.
const TOOLS: [&str; 4] = ["zenity", "yad", "kdialog", "notify-send"];

pub fn available() -> Option<&'static str> {
    TOOLS.into_iter().find(|t| which(t))
}

/// The dialog's argv. Exit codes differ by tool, see `answer`.
pub fn command(tool: &str, who: &str, reason: &str, seconds: u32, minutes: u32, scope: &str, detail: &str) -> Vec<String> {
    let mut text = format!("{who} wants to use your {scope}.\n\n");
    if !reason.is_empty() {
        text += &format!("{reason}\n\n");
    }
    if !detail.is_empty() {
        text += &format!("It will: {detail}\n\n");
    }
    text += "Nothing happens until you allow it.";
    let grant = format!("Allow for {minutes} min");
    let v: Vec<String> = match tool {
        "zenity" => vec!["zenity".into(), "--question".into(), "--default-cancel".into(), "--title=Ghosthand".into(), format!("--text={text}"), "--ok-label=Allow".into(), "--cancel-label=Deny".into(), format!("--extra-button={grant}"), format!("--timeout={seconds}"), "--no-markup".into()],
        "yad" => vec!["yad".into(), "--title=Ghosthand".into(), "--image=input-mouse".into(), format!("--text={text}"), "--button=Deny:1".into(), format!("--button={grant}:2"), "--button=Allow:0".into(), format!("--timeout={seconds}"), "--center".into(), "--on-top".into()],
        "notify-send" => vec!["notify-send".into(), "-a".into(), "Ghosthand".into(), "-u".into(), "critical".into(), "-t".into(), (seconds * 1000).to_string(), "-A".into(), "allow=Allow".into(), "-A".into(), format!("grant={grant}"), "-A".into(), "deny=Deny".into(), format!("{who} wants to use your {scope}"), text.replacen(&format!("{who} wants to use your {scope}.\n\n"), "", 1)],
        _ => vec!["kdialog".into(), "--title".into(), "Ghosthand".into(), "--warningyesnocancel".into(), text, "--yes-label".into(), "Allow".into(), "--no-label".into(), grant, "--cancel-label".into(), "Deny".into()],
    };
    v
}

#[derive(Debug, PartialEq)]
pub enum Verdict { Allow, Grant, Deny, Timeout }

/// Turn a finished dialog into a verdict.
pub fn answer(tool: &str, code: i32, out: &str) -> Verdict {
    match tool {
        "zenity" => match code { 0 => Verdict::Allow, 1 if out.contains("Allow for") => Verdict::Grant, 5 => Verdict::Timeout, _ => Verdict::Deny },
        "yad" => match code { 0 => Verdict::Allow, 2 => Verdict::Grant, 70 => Verdict::Timeout, _ => Verdict::Deny },
        "notify-send" => match out.trim() { "allow" => Verdict::Allow, "grant" => Verdict::Grant, _ => Verdict::Deny }, // prints the chosen action; nothing when it was dismissed or expired
        _ => match code { 0 => Verdict::Allow, 1 => Verdict::Grant, _ => Verdict::Deny }, // kdialog
    }
}

/// A running dialog.
pub trait Proc {
    fn poll(&mut self) -> Option<i32>;
    fn output(&mut self) -> String;
    fn kill(&mut self);
}

struct Spawned(Child);

impl Proc for Spawned {
    fn poll(&mut self) -> Option<i32> { self.0.try_wait().ok().flatten().map(|s| s.code().unwrap_or(-1)) }
    fn output(&mut self) -> String {
        let mut out = String::new();
        if let Some(mut so) = self.0.stdout.take() { let _ = so.read_to_string(&mut out); }
        out
    }
    fn kill(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); }
}

type Spawn = Box<dyn FnMut(&[String]) -> Option<Box<dyn Proc>>>;

pub struct DialogGate {
    tool: String,
    config: Box<dyn Fn() -> Config>,
    spawn: Spawn,
    notify: bool,
    proc: Option<Box<dyn Proc>>,
    state: String,
    started: f64,
    /// An Allow that comes back sooner than this after the dialog was shown is a stray keypress or click, not a person reading it.
    min_read: f64,
    deadline: f64,
    minutes: u32,
    mode: String,
    who: String,
    scope: String,
}

impl DialogGate {
    pub fn new(tool: &str) -> DialogGate {
        DialogGate {
            tool: tool.to_string(),
            config: Box::new(Config::load),
            spawn: Box::new(|argv| {
                Command::new(&argv[0]).args(&argv[1..]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok().map(|c| Box::new(Spawned(c)) as Box<dyn Proc>)
            }),
            notify: true,
            proc: None, state: "idle".into(), started: 0.0, min_read: 0.7, deadline: 0.0, minutes: 10, mode: "ask".into(), who: "Agent".into(), scope: "mouse".into(),
        }
    }

    fn decide(&mut self, verdict: Verdict) -> String {
        let verdict = if verdict == Verdict::Timeout && self.mode == "notify" { Verdict::Allow } else { verdict };
        if verdict == Verdict::Grant {
            paths::ensure_runtime_dir();
            for (kind, word) in [("", "mouse"), ("keys", "keyboard")] {
                if self.scope.contains(word) {
                    let _ = std::fs::write(paths::grant_file(kind), format!("{}\n", now() + f64::from(self.minutes) * 60.0));
                }
            }
        }
        self.state = match verdict {
            Verdict::Allow | Verdict::Grant => { self.notify(); "active" }
            Verdict::Timeout => "timeout",
            Verdict::Deny => "denied",
        }.into();
        self.proc = None;
        self.state.clone()
    }

    fn finish(&mut self, state: &str) -> String {
        if let Some(p) = self.proc.as_mut() {
            if p.poll().is_none() { p.kill(); }
        }
        self.proc = None;
        self.state = state.into();
        self.state.clone()
    }

    fn notify(&self) {
        if self.notify && which("notify-send") {
            let _ = Command::new("notify-send").args(["-a", "Ghosthand", "-i", "input-mouse", "Agent Control", &format!("{} is using your {}. Run `ghosthand revoke` to take it back.", self.who, self.scope)]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map(|mut c| std::thread::spawn(move || { let _ = c.wait(); }));
        }
    }
}

impl Gate for DialogGate {
    fn ask(&mut self, r: &Request) -> Option<String> {
        let cfg = (self.config)();
        self.who = if r.who.is_empty() { "Agent".into() } else { r.who.clone() };
        self.scope = if r.scope.is_empty() { "mouse".into() } else { r.scope.clone() };
        self.started = now();
        self.mode = cfg.mode();
        self.minutes = cfg.grant_minutes();
        let (t, covered) = (now(), (!self.scope.contains("mouse") || read_float(&paths::grant_file("")) > now()) && (!self.scope.contains("keyboard") || read_float(&paths::grant_file("keys")) > now()));
        let _ = t;
        if self.mode == "off" || covered {
            self.state = "active".into();
            self.notify();
            return Some("allowed".into());
        }
        let seconds = cfg.countdown().max(r.seconds.min(120));
        self.deadline = now() + f64::from(seconds);
        self.state = "asking".into();
        let argv = command(&self.tool, &self.who, &r.reason, seconds, self.minutes, &self.scope, &r.detail);
        self.proc = Some((self.spawn)(&argv)?);
        Some("asking".into())
    }

    fn status(&mut self) -> Option<String> {
        if matches!(self.state.as_str(), "active" | "asking") && revoked_since(self.started) {
            return Some(self.finish("revoked"));
        }
        if self.state != "asking" {
            return Some(self.state.clone());
        }
        let code = self.proc.as_mut()?.poll();
        match code {
            None if now() < self.deadline => Some("asking".into()),
            None => {
                self.proc.as_mut()?.kill();
                Some(self.decide(Verdict::Timeout))
            }
            Some(code) => {
                let out = self.proc.as_mut()?.output();
                let v = answer(&self.tool, code, &out);
                let too_fast = now() - self.started < self.min_read;
                Some(self.decide(if too_fast && matches!(v, Verdict::Allow | Verdict::Grant) { Verdict::Deny } else { v }))
            }
        }
    }

    fn deny(&mut self) { self.finish("denied"); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Fake { code: Rc<RefCell<Option<i32>>>, out: String, killed: Rc<RefCell<bool>> }

    impl Proc for Fake {
        fn poll(&mut self) -> Option<i32> { *self.code.borrow() }
        fn output(&mut self) -> String { self.out.clone() }
        fn kill(&mut self) { *self.killed.borrow_mut() = true; }
    }

    #[allow(clippy::type_complexity)]
    fn gate(cfg: &'static str, code: Option<i32>, out: &str) -> (DialogGate, Rc<RefCell<Option<i32>>>, Rc<RefCell<bool>>) {
        crate::paths::use_temp_runtime_dir();
        let (code_cell, killed) = (Rc::new(RefCell::new(code)), Rc::new(RefCell::new(false)));
        let (c2, k2, out) = (code_cell.clone(), killed.clone(), out.to_string());
        let mut g = DialogGate::new("zenity");
        g.config = Box::new(move || Config::from_json(cfg));
        g.spawn = Box::new(move |_| Some(Box::new(Fake { code: c2.clone(), out: out.clone(), killed: k2.clone() })));
        g.notify = false;
        g.min_read = 0.0;
        (g, code_cell, killed)
    }

    fn req(scope: &str) -> Request { Request { who: "Claude".into(), reason: "why".into(), seconds: 5, scope: scope.into(), detail: String::new() } }

    #[test]
    fn answers_are_read_the_way_each_tool_reports_them() {
        assert_eq!(answer("zenity", 0, ""), Verdict::Allow);
        assert_eq!(answer("zenity", 1, "Allow for 10 min\n"), Verdict::Grant);
        assert_eq!(answer("zenity", 1, ""), Verdict::Deny);
        assert_eq!(answer("zenity", 5, ""), Verdict::Timeout);
        assert_eq!(answer("yad", 2, ""), Verdict::Grant);
        assert_eq!(answer("yad", 70, ""), Verdict::Timeout);
        assert_eq!(answer("yad", 252, ""), Verdict::Deny, "closing the window is a no");
        assert_eq!(answer("kdialog", 0, ""), Verdict::Allow);
        assert_eq!(answer("kdialog", 2, ""), Verdict::Deny);
        assert_eq!(answer("notify-send", 0, "allow\n"), Verdict::Allow);
        assert_eq!(answer("notify-send", 0, "grant\n"), Verdict::Grant);
        assert_eq!(answer("notify-send", 0, ""), Verdict::Deny, "dismissed or expired");
        assert_eq!(answer("notify-send", 0, "deny\n"), Verdict::Deny);
    }

    #[test]
    fn the_notification_carries_three_actions_and_the_request() {
        let argv = command("notify-send", "Claude", "why", 8, 10, "mouse and keyboard", "type: 'ls'");
        let text = argv.join("|");
        assert!(text.contains("-A|allow=Allow") && text.contains("grant=Allow for 10 min") && text.contains("deny=Deny"));
        assert!(text.contains("Claude wants to use your mouse and keyboard") && text.contains("type: 'ls'") && text.contains("8000"));
    }

    #[test]
    fn the_card_names_who_why_and_what_and_says_nothing_happens_until_allowed() {
        let text = command("zenity", "Claude", "Checking the bar", 8, 10, "mouse and keyboard", "type: 'rm -rf x'").join(" ");
        assert!(text.contains("Claude wants to use your mouse and keyboard"));
        assert!(text.contains("Checking the bar") && text.contains("type: 'rm -rf x'") && text.contains("--timeout=8"));
    }

    #[test]
    fn an_allow_that_comes_back_too_fast_to_have_been_read_is_a_no() {
        let (mut g, _, _) = gate("{}", Some(0), "");
        g.min_read = 5.0;
        g.ask(&req("mouse"));
        assert_eq!(g.status().as_deref(), Some("denied"), "a stray Enter on a dialog that just stole focus");
        let (mut g, _, _) = gate("{}", Some(1), "Allow for 10 min\n");
        g.min_read = 5.0;
        g.ask(&req("mouse"));
        assert_eq!(g.status().as_deref(), Some("denied"));
        assert!(!paths::grant_file("").exists(), "and no grant is written");
    }

    #[test]
    fn the_dialog_defaults_to_deny() {
        assert!(command("zenity", "C", "", 8, 10, "mouse", "").contains(&"--default-cancel".to_string()));
    }

    #[test]
    fn ask_then_allow() {
        let (mut g, _, _) = gate("{}", Some(0), "");
        assert_eq!(g.ask(&req("mouse")).as_deref(), Some("asking"));
        assert_eq!(g.status().as_deref(), Some("active"));
    }

    #[test]
    fn deny_and_silence_are_both_a_no() {
        let (mut g, _, _) = gate("{}", Some(1), "");
        g.ask(&req("mouse"));
        assert_eq!(g.status().as_deref(), Some("denied"));
        let (mut g, _, killed) = gate("{}", None, "");
        g.ask(&req("mouse"));
        g.deadline = 0.0;
        assert_eq!(g.status().as_deref(), Some("timeout"));
        assert!(*killed.borrow(), "the card is closed");
    }

    #[test]
    fn notify_mode_goes_ahead_when_nobody_answers() {
        let (mut g, _, _) = gate(r#"{"mode":"notify"}"#, None, "");
        g.ask(&req("mouse"));
        g.deadline = 0.0;
        assert_eq!(g.status().as_deref(), Some("active"));
    }

    #[test]
    fn mode_off_needs_no_dialog() {
        let (mut g, _, _) = gate(r#"{"mode":"off"}"#, None, "");
        assert_eq!(g.ask(&req("mouse and keyboard")).as_deref(), Some("allowed"));
    }

    #[test]
    fn a_dialog_that_cannot_start_is_no_answer_never_a_silent_pass() {
        let (mut g, _, _) = gate("{}", None, "");
        g.spawn = Box::new(|_| None);
        assert_eq!(g.ask(&req("mouse")), None);
    }

    #[test]
    fn a_mouse_grant_does_not_cover_the_keyboard_and_revoke_ends_it() {
        let (mut g, _, _) = gate("{}", Some(1), "Allow for 10 min\n");
        g.ask(&req("mouse"));
        assert_eq!(g.status().as_deref(), Some("active"));
        let (mut again, _, _) = (g, 0, 0);
        assert_eq!(again.ask(&req("mouse")).as_deref(), Some("allowed"), "the mouse is covered");
        assert_eq!(again.ask(&req("mouse and keyboard")).as_deref(), Some("asking"), "the keyboard is not");
        crate::gate::revoke_local();
        assert_eq!(again.ask(&req("mouse")).as_deref(), Some("asking"), "and the grant is gone");
    }

    #[test]
    fn a_revoke_while_the_card_is_open_closes_it() {
        let (mut g, _, killed) = gate("{}", None, "");
        g.ask(&req("mouse"));
        std::thread::sleep(std::time::Duration::from_millis(10));
        crate::gate::revoke_local();
        assert_eq!(g.status().as_deref(), Some("revoked"));
        assert!(*killed.borrow());
    }

    #[test]
    fn a_revoke_stops_a_running_agent_too() {
        let (mut g, _, _) = gate("{}", Some(0), "");
        g.ask(&req("mouse"));
        g.status();
        std::thread::sleep(std::time::Duration::from_millis(10));
        crate::gate::revoke_local();
        assert_eq!(g.status().as_deref(), Some("revoked"));
    }
}
