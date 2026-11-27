//! The actions: what you can say to the pointer and keyboard, parsed, checked and run.
use serde_json::json;

use crate::config::Config;
use crate::geometry::{numbers, Info};
use crate::keyboard::{self, Keyboard};
use crate::layout::Layout;
use crate::pointer::Pointer;
use crate::screen::screenshot;
use crate::uinput::{BTN_LEFT, BTN_MIDDLE, BTN_RIGHT};
use crate::util::{pause, Check, Stop, now};

pub const POINTER: &[&str] = &["jump", "move", "click", "dclick", "rclick", "mclick", "down", "up", "rdown", "rup", "drag", "rdrag", "scroll", "hscroll", "pos"];
pub const KEYBOARD: &[&str] = &["key", "keydown", "keyup", "type", "paste"];
/// These touch nothing, so they need no approval.
pub const NO_INPUT: &[&str] = &["shot", "wait", "info", "focus"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Btn { Left, Right, Middle }

impl Btn {
    fn code(self) -> u16 { match self { Btn::Left => BTN_LEFT, Btn::Right => BTN_RIGHT, Btn::Middle => BTN_MIDDLE } }
}

type At = Option<(f64, f64)>;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Jump(f64, f64),
    Move(f64, f64),
    Click { btn: Btn, times: u32, at: At },
    Down(Btn),
    Up(Btn),
    Drag { btn: Btn, from: (f64, f64), to: (f64, f64) },
    Scroll { notches: f64, horizontal: bool, at: At },
    Wait(f64),
    Shot { file: String, region: Option<[i64; 4]> },
    Pos,
    Info,
    Focus,
    Key(Vec<Vec<u16>>),
    KeyDown(Vec<Vec<u16>>),
    KeyUp(Vec<Vec<u16>>),
    Type(String),
    Paste(String),
}

/// One parsed action with what the person wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub raw: String,
    pub head: String,
    pub action: Action,
}

#[derive(Debug, PartialEq)]
pub struct Opts {
    pub who: String,
    pub reason: String,
    pub countdown: u32,
}

/// (options, actions): --who, --reason and --countdown come before the actions.
pub fn parse_args(argv: &[String]) -> Result<(Opts, Vec<String>), String> {
    let mut opts = Opts { who: "Agent".into(), reason: String::new(), countdown: 0 };
    let mut i = 0;
    while i < argv.len() && argv[i].starts_with("--") {
        let key = &argv[i][2..];
        let value = argv.get(i + 1).ok_or_else(|| format!("unknown or incomplete option: {}", argv[i]))?;
        match key {
            "who" => opts.who = value.clone(),
            "reason" => opts.reason = value.clone(),
            "countdown" => opts.countdown = value.parse().map_err(|_| format!("--countdown needs a whole number of seconds, not '{value}'"))?,
            _ => return Err(format!("unknown or incomplete option: {}", argv[i])),
        }
        i += 2;
    }
    Ok((opts, argv[i..].to_vec()))
}

fn pair(args: &[f64], what: &str) -> Result<(f64, f64), String> {
    match args {
        [x, y] => Ok((*x, *y)),
        _ => Err(format!("{what} needs X Y")),
    }
}

fn optional_at(args: &[f64], what: &str) -> Result<At, String> {
    match args.len() {
        0 => Ok(None),
        2 => Ok(Some((args[0], args[1]))),
        _ => Err(format!("{what} takes X Y or nothing")),
    }
}

fn combos(words: &[&str], lay: &Layout, what: &str) -> Result<Vec<Vec<u16>>, String> {
    if words.is_empty() {
        return Err(format!("{what} needs something to press"));
    }
    words.iter().map(|w| keyboard::parse_combo(w, lay)).collect()
}

/// Parse one action. A typo, an unknown key, or a character the layout can't type is an error here, before anything is touched.
pub fn parse(raw: &str, lay: &Layout, cfg: &Config) -> Result<Step, String> {
    let words: Vec<&str> = raw.split_whitespace().collect();
    let Some((&cmd, args)) = words.split_first() else { return Err(format!("unknown action: '{raw}'")) };
    let rest = raw.trim_start().split_once(' ').map_or("", |(_, r)| r);
    if KEYBOARD.contains(&cmd) && !cfg.keyboard_enabled() {
        return Err("the keyboard is switched off in the config (\"keyboard\": false)".into());
    }
    let nums = || numbers(args);
    let action = match cmd {
        "jump" => { let (x, y) = pair(&nums()?, "jump")?; Action::Jump(x, y) }
        "move" => { let (x, y) = pair(&nums()?, "move")?; Action::Move(x, y) }
        "click" | "dclick" | "rclick" | "mclick" => Action::Click {
            btn: match cmd { "rclick" => Btn::Right, "mclick" => Btn::Middle, _ => Btn::Left },
            times: if cmd == "dclick" { 2 } else { 1 },
            at: optional_at(&nums()?, cmd)?,
        },
        "down" | "up" | "rdown" | "rup" => {
            let btn = if cmd.starts_with('r') { Btn::Right } else { Btn::Left };
            if cmd.ends_with("down") { Action::Down(btn) } else { Action::Up(btn) }
        }
        "drag" | "rdrag" => match nums()?.as_slice() {
            [x1, y1, x2, y2] => Action::Drag { btn: if cmd == "rdrag" { Btn::Right } else { Btn::Left }, from: (*x1, *y1), to: (*x2, *y2) },
            _ => return Err(format!("{cmd} needs X1 Y1 X2 Y2")),
        },
        "scroll" | "hscroll" => {
            let n = nums()?;
            let (notches, at) = n.split_first().map(|(n, rest)| (*n, rest)).ok_or_else(|| format!("{cmd} needs a number of notches"))?;
            Action::Scroll { notches, horizontal: cmd == "hscroll", at: optional_at(at, cmd)? }
        }
        "wait" => match nums()?.as_slice() {
            [s] if *s >= 0.0 => Action::Wait(*s),
            _ => return Err("wait needs a number of seconds".into()),
        },
        "shot" => {
            let (file, region) = args.split_first().ok_or("shot needs a file")?;
            let r = numbers(region)?;
            match r.len() {
                0 => Action::Shot { file: file.to_string(), region: None },
                4 => Action::Shot { file: file.to_string(), region: Some([r[0] as i64, r[1] as i64, r[2] as i64, r[3] as i64]) },
                _ => return Err("shot takes a file, then X Y W H or nothing".into()),
            }
        }
        "pos" => Action::Pos,
        "info" => Action::Info,
        "focus" => Action::Focus,
        "key" => Action::Key(combos(args, lay, "key")?),
        "keydown" => Action::KeyDown(combos(args, lay, "keydown")?),
        "keyup" => Action::KeyUp(combos(args, lay, "keyup")?),
        "type" | "paste" => {
            if rest.trim().is_empty() {
                return Err(format!("{cmd} needs something to type"));
            }
            let text = keyboard::unescape(rest);
            if cmd == "type" {
                let lost = lay.missing(&text);
                if !lost.is_empty() {
                    return Err(format!("can't type '{}' with the {} layout: use `paste`, or set keyboard_layout in the config", lost.iter().collect::<String>(), lay.name));
                }
                Action::Type(text)
            } else {
                Action::Paste(text)
            }
        }
        _ => return Err(format!("unknown action: '{raw}'")),
    };
    Ok(Step { raw: raw.to_string(), head: cmd.to_string(), action })
}

pub fn parse_all(raw: &[String], lay: &Layout, cfg: &Config) -> Result<Vec<Step>, String> {
    if raw.is_empty() {
        return Err("no actions given".into());
    }
    raw.iter().map(|a| parse(a, lay, cfg)).collect()
}

pub fn uses_keyboard(steps: &[Step]) -> bool { steps.iter().any(|s| KEYBOARD.contains(&s.head.as_str())) }
pub fn uses_pointer(steps: &[Step]) -> bool { steps.iter().any(|s| POINTER.contains(&s.head.as_str())) }
/// Does anything here need the gate? (Everything but screenshots, waits, info and focus.)
pub fn touches_input(steps: &[Step]) -> bool { steps.iter().any(|s| !NO_INPUT.contains(&s.head.as_str())) }

/// What the card says is being taken: "mouse", "keyboard" or "mouse and keyboard".
pub fn scope(steps: &[Step]) -> String {
    let parts: Vec<&str> = [("mouse", uses_pointer(steps)), ("keyboard", uses_keyboard(steps))].iter().filter(|(_, u)| *u).map(|(n, _)| *n).collect();
    if parts.is_empty() { "mouse".into() } else { parts.join(" and ") }
}

/// What will be typed or pressed, for the card: `type: 'git status'  key: ctrl+c`. Empty when no keyboard is used.
pub fn detail(steps: &[Step], limit: usize) -> String {
    let parts: Vec<String> = steps.iter().filter_map(|s| match &s.action {
        Action::Type(t) => Some(format!("type: {t:?}")),
        Action::Paste(t) => Some(format!("paste: {t:?}")),
        Action::Key(_) | Action::KeyDown(_) | Action::KeyUp(_) => Some(format!("{}: {}", s.head, s.raw.split_once(' ').map_or("", |x| x.1))),
        _ => None,
    }).collect();
    let text = parts.join("  ");
    if text.chars().count() <= limit { text } else { format!("{}\u{2026}", text.chars().take(limit - 1).collect::<String>()) }
}

pub struct Ctx<'a> {
    pub pointer: Option<&'a mut Pointer>,
    pub keyboard: Option<&'a mut Keyboard>,
    pub info: &'a Info,
    pub out: &'a mut Vec<String>,
    pub check: Check<'a>,
    pub cfg: &'a Config,
}

fn need<'b, T>(device: &'b mut Option<&mut T>, what: &str) -> Result<&'b mut T, Stop> {
    device.as_deref_mut().ok_or_else(|| Stop::Failed(format!("no {what} for this action")))
}

/// One action. `ctx.check` returns Revoked when the person took the pointer back; it is called between steps.
pub fn run(step: &Step, ctx: &mut Ctx) -> Result<(), Stop> {
    (ctx.check)()?;
    match &step.action {
        Action::Shot { file, region } => screenshot(file, *region)?,
        Action::Wait(seconds) => {
            let end = now() + seconds;
            while now() < end {
                pause(0.25_f64.min((end - now()).max(0.0))); // in slices, so a revoke is noticed
                (ctx.check)()?;
                if crate::util::FAST.load(std::sync::atomic::Ordering::Relaxed) { break; }
            }
        }
        Action::Info => ctx.out.push(ctx.info.to_json()),
        Action::Focus => ctx.out.push(crate::windows::focused().unwrap_or(serde_json::Value::Null).to_string()),
        Action::Pos => {
            let p = need(&mut ctx.pointer, "pointer")?;
            ctx.out.push(json!({"x": p.x.round(), "y": p.y.round(), "known": p.known}).to_string());
        }
        Action::Key(list) => {
            let kb = need(&mut ctx.keyboard, "keyboard")?;
            for codes in list {
                kb.key(codes)?;
                (ctx.check)()?;
            }
        }
        Action::KeyDown(list) | Action::KeyUp(list) => {
            let kb = need(&mut ctx.keyboard, "keyboard")?;
            for codes in list {
                kb.hold(codes, matches!(step.action, Action::KeyDown(_)))?;
            }
        }
        Action::Type(text) => need(&mut ctx.keyboard, "keyboard")?.type_text(text, &mut *ctx.check)?,
        Action::Paste(text) => keyboard::paste(need(&mut ctx.keyboard, "keyboard")?, text, ctx.cfg.paste_terminal())?,
        Action::Jump(x, y) => need(&mut ctx.pointer, "pointer")?.put(*x, *y)?,
        Action::Move(x, y) => need(&mut ctx.pointer, "pointer")?.glide(*x, *y)?,
        Action::Click { btn, times, at } => {
            let p = need(&mut ctx.pointer, "pointer")?;
            if let Some((x, y)) = at {
                p.glide(*x, *y)?;
                pause(0.2);
            }
            p.click(btn.code(), *times)?;
        }
        Action::Down(b) => need(&mut ctx.pointer, "pointer")?.button(b.code(), true)?,
        Action::Up(b) => need(&mut ctx.pointer, "pointer")?.button(b.code(), false)?,
        Action::Drag { btn, from, to } => {
            let p = need(&mut ctx.pointer, "pointer")?;
            p.put(from.0, from.1)?;
            pause(0.3);
            p.button(btn.code(), true)?;
            pause(0.2);
            p.glide((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0 + 20.0)?;
            p.glide(to.0, to.1)?;
            pause(0.4);
            p.button(btn.code(), false)?;
        }
        Action::Scroll { notches, horizontal, at } => {
            let p = need(&mut ctx.pointer, "pointer")?;
            if let Some((x, y)) = at {
                p.glide(*x, *y)?;
                pause(0.2);
            }
            p.wheel(*notches, *horizontal)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout;
    use crate::uinput::{Recorder, EV_KEY};

    fn lay() -> Layout { layout::us() }
    fn p(raw: &str) -> Result<Step, String> { parse(raw, &lay(), &Config::default()) }
    fn steps(list: &[&str]) -> Vec<Step> { list.iter().map(|a| p(a).unwrap()).collect() }

    #[test]
    fn options_come_before_the_actions() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let (o, rest) = parse_args(&a(&["--who", "Claude", "--reason", "testing", "click 1 2", "wait 1"])).unwrap();
        assert_eq!(o, Opts { who: "Claude".into(), reason: "testing".into(), countdown: 0 });
        assert_eq!(rest, a(&["click 1 2", "wait 1"]));
        assert_eq!(parse_args(&a(&["--countdown", "12", "pos"])).unwrap().0.countdown, 12);
        assert!(parse_args(&a(&["--bogus", "x", "pos"])).is_err());
        assert!(parse_args(&a(&["--who"])).is_err());
        assert!(parse_args(&a(&["--countdown", "soon", "pos"])).is_err());
    }

    #[test]
    fn a_typo_is_caught_up_front() {
        assert!(p("clikc").is_err());
        assert!(p("").is_err());
        assert!(p("jump 1").is_err());
        assert!(p("jump 1 x").is_err());
        assert!(p("drag 1 2 3").is_err());
        assert!(p("wait -1").is_err());
        assert!(p("click 1").is_err());
        assert!(p("shot").is_err());
        assert!(parse_all(&["jump 1 2".into(), "clikc".into()], &lay(), &Config::default()).is_err());
        assert!(parse_all(&[], &lay(), &Config::default()).is_err());
        for ok in ["jump 1 2", "dclick", "scroll -3", "scroll 2 40 50", "shot /tmp/x.png 0 0 10 10", "drag 1 2 3 4", "rdrag 1 2 3 4", "hscroll 1", "mclick 5 5", "pos", "info", "focus"] {
            assert!(p(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn scope_names_what_is_taken() {
        assert_eq!(scope(&steps(&["click"])), "mouse");
        assert_eq!(scope(&steps(&["type hi"])), "keyboard");
        assert_eq!(scope(&steps(&["click", "type hi"])), "mouse and keyboard");
        assert_eq!(scope(&steps(&["key ctrl+s", "wait 1"])), "keyboard");
    }

    #[test]
    fn the_card_shows_what_will_be_typed() {
        assert_eq!(detail(&steps(&["click", "type git status\\n"]), 140), "type: \"git status\\n\"");
        assert_eq!(detail(&steps(&["key ctrl+c", "key Return"]), 140), "key: ctrl+c  key: Return");
        assert_eq!(detail(&steps(&["click"]), 140), "");
        let long = detail(&steps(&[&format!("type {}", "x".repeat(500))]), 140);
        assert!(long.ends_with('\u{2026}') && long.chars().count() == 140);
    }

    #[test]
    fn only_shot_wait_info_and_focus_need_no_gate() {
        assert!(!touches_input(&steps(&["focus", "shot /tmp/x.png", "wait 1", "info"])));
        assert!(touches_input(&steps(&["key Return"])));
        assert!(touches_input(&steps(&["wait 1", "pos"])));
        assert!(touches_input(&steps(&["type a"])));
    }

    #[test]
    fn keyboard_actions_are_validated_up_front() {
        for bad in ["type", "key", "keydown", "key ctrl+nonsense", "type caf\u{e9}", "paste"] {
            assert!(p(bad).is_err(), "{bad}");
        }
        for ok in ["key ctrl+s Return", "type hello world", "keydown shift", "keyup shift", "paste caf\u{e9} \u{20ac}"] {
            assert!(p(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn the_keyboard_can_be_switched_off() {
        let cfg = Config::from_json(r#"{"keyboard":false}"#);
        let e = parse("type hi", &lay(), &cfg).unwrap_err();
        assert!(e.contains("switched off"));
        assert!(parse("click 1 2", &lay(), &cfg).is_ok());
    }

    #[test]
    fn type_keeps_the_text_exactly_as_written() {
        assert_eq!(p("type  two  spaces ").unwrap().action, Action::Type(" two  spaces ".into()));
    }

    struct Rig { pointer: Pointer, keyboard: Keyboard, p: Recorder, k: Recorder, info: Info }

    fn rig() -> Rig {
        crate::util::FAST.store(true, std::sync::atomic::Ordering::Relaxed);
        crate::paths::use_temp_runtime_dir();
        let (p, k) = (Recorder::default(), Recorder::default());
        let info = Info { name: "X".into(), width: 100, height: 100, scale: 1.0 };
        Rig { pointer: Pointer::new(Box::new(p.clone()), &info, None), keyboard: Keyboard::new(Box::new(k.clone()), lay(), 0.0), p, k, info }
    }

    fn go(r: &mut Rig, raw: &str, check: &mut dyn FnMut() -> Result<(), Stop>) -> Result<Vec<String>, Stop> {
        let mut out = Vec::new();
        let cfg = Config::default();
        let step = p(raw).unwrap();
        run(&step, &mut Ctx { pointer: Some(&mut r.pointer), keyboard: Some(&mut r.keyboard), info: &r.info, out: &mut out, check, cfg: &cfg })?;
        Ok(out)
    }

    #[test]
    fn clicks_scrolls_and_drags() {
        let mut r = rig();
        go(&mut r, "dclick 5 6", &mut || Ok(())).unwrap();
        assert_eq!(r.p.0.borrow().iter().filter(|e| e.0 == EV_KEY && e.2 == 1).count(), 2);
        let n = r.p.0.borrow().len();
        go(&mut r, "scroll -2 40 50", &mut || Ok(())).unwrap();
        assert!(r.p.0.borrow()[n..].iter().any(|e| e.1 == crate::uinput::REL_WHEEL_HI_RES && e.2 == 120), "negative scrolls up");
        let n = r.p.0.borrow().len();
        go(&mut r, "rdrag 1 2 3 4", &mut || Ok(())).unwrap();
        let ev = r.p.0.borrow()[n..].to_vec();
        assert!(ev.contains(&(EV_KEY, BTN_RIGHT, 1)) && ev.last() == Some(&(EV_KEY, BTN_RIGHT, 0)));
    }

    #[test]
    fn a_revoke_stops_before_the_next_action() {
        let mut r = rig();
        assert_eq!(go(&mut r, "click 1 1", &mut || Err(Stop::Revoked)), Err(Stop::Revoked));
        assert!(r.p.0.borrow().is_empty());
    }

    #[test]
    fn pos_info_and_keys_print_or_press() {
        let mut r = rig();
        assert!(go(&mut r, "info", &mut || Ok(())).unwrap()[0].contains("\"scale\""));
        assert!(go(&mut r, "pos", &mut || Ok(())).unwrap()[0].contains("\"known\""));
        go(&mut r, "key ctrl+s", &mut || Ok(())).unwrap();
        assert_eq!(r.k.0.borrow().first(), Some(&(EV_KEY, 29, 1)));
        go(&mut r, "keydown shift", &mut || Ok(())).unwrap();
        go(&mut r, "keyup shift", &mut || Ok(())).unwrap();
        assert_eq!(r.k.0.borrow().last(), Some(&(EV_KEY, 42, 0)));
    }

    #[test]
    fn a_pointer_action_with_no_pointer_is_an_error_not_a_panic() {
        let cfg = Config::default();
        let info = Info { name: String::new(), width: 1, height: 1, scale: 1.0 };
        let mut out = vec![];
        let r = run(&p("click").unwrap(), &mut Ctx { pointer: None, keyboard: None, info: &info, out: &mut out, check: &mut || Ok(()), cfg: &cfg });
        assert!(matches!(r, Err(Stop::Failed(_))));
    }
}
