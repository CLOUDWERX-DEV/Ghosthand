//! The gate: ask for the pointer and keyboard, and wait for a person to answer.
//!
//! Two ways to ask, both answering the same few calls (ask, status, deny):
//!   the overlay   (overlay/, a Quickshell config) draws the card and the cursor tag; the client talks to it over Quickshell IPC
//!                 (`qs -c ghosthand ipc call ghosthand ...`) and streams the pointer position as lines in a file it follows.
//!   a dialog      (dialog.rs) from zenity, yad or kdialog, for X11, GNOME and anything else without layer-shell. No tag.
//! A revoke does not go through either: `ghosthand revoke` writes a file, and every run checks it between steps (a stat, not a process).
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::dialog::{self, DialogGate};
use crate::paths;
use crate::util::{now, pause, read_float, which};

pub const EXIT_FAILED: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_REVOKED: i32 = 3;
pub const EXIT_DENIED: i32 = 4;
pub const EXIT_NO_GATE: i32 = 5;

/// What is being asked for.
#[derive(Debug, Clone, Default)]
pub struct Request {
    pub who: String,
    pub reason: String,
    pub seconds: u32,
    /// "mouse", "keyboard" or "mouse and keyboard".
    pub scope: String,
    /// What the keyboard will type or press, for the card.
    pub detail: String,
}

pub trait Gate {
    /// "allowed" (a standing grant, or the gate is off), "asking" (poll `status`), or None when there is nothing to ask with.
    fn ask(&mut self, req: &Request) -> Option<String>;
    /// idle | asking | active | allowed | denied | timeout | revoked, or None when the gate went away.
    fn status(&mut self) -> Option<String>;
    fn deny(&mut self);
}

#[derive(Debug)]
pub struct GateError {
    pub message: String,
    pub code: i32,
}

/// Wait for the person's answer. Returns when allowed; an error otherwise (denied, no answer in time, or the gate is gone).
pub fn wait_for_allow(gate: &mut dyn Gate, req: &Request, patience: Option<f64>) -> Result<(), GateError> {
    let no_gate = |m: &str| GateError { message: m.into(), code: EXIT_NO_GATE };
    let first = gate.ask(req).ok_or_else(|| no_gate("Ghosthand: nothing is there to ask you, so nothing is touched (there is no way around the gate). Run `ghosthand overlay`, or install zenity, yad or kdialog"))?;
    if first == "allowed" {
        return Ok(());
    }
    let limit = now() + patience.unwrap_or_else(|| f64::from(if req.seconds == 0 { 8 } else { req.seconds }).max(5.0) + 15.0);
    while now() < limit {
        pause(0.15);
        match gate.status().as_deref() {
            Some("allowed" | "active") => return Ok(()), // the overlay moves straight on to "active" once you allow it
            Some(s @ ("denied" | "timeout" | "revoked")) => return Err(GateError { message: format!("Ghosthand: not allowed ({s})."), code: EXIT_DENIED }),
            None => return Err(no_gate("Ghosthand: lost the gate while waiting for an answer.")),
            _ => {}
        }
        if crate::util::FAST.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    gate.deny();
    Err(GateError { message: "Ghosthand: no answer in time.".into(), code: EXIT_DENIED })
}

/// `ghosthand revoke`, from any process: a running agent notices on its next step, and every standing grant ends.
pub fn revoke_local() {
    paths::ensure_runtime_dir();
    let _ = std::fs::write(paths::revoked_file(), format!("{}\n", now()));
    for kind in ["", "keys"] {
        let _ = std::fs::remove_file(paths::grant_file(kind));
    }
}

/// Was the pointer taken back after `since`?
pub fn revoked_since(since: f64) -> bool {
    read_float(&paths::revoked_file()) > since
}

/// Is the session locked? Runs `lock_check` from the config, which prints "no" while it is not. Anything else, or no answer, counts as locked.
pub fn locked(command: &str) -> bool {
    let Ok(mut child) = Command::new("sh").args(["-c", command]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else { return true };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < Duration::from_secs(5) => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                return true;
            }
        }
    }
    let mut out = String::new();
    if let Some(mut so) = child.stdout.take() {
        let _ = std::io::Read::read_to_string(&mut so, &mut out);
    }
    !matches!(out.trim().to_ascii_lowercase().as_str(), "no" | "false" | "0")
}

/// The Quickshell overlay.
pub struct Overlay {
    pub name: String,
}

impl Overlay {
    /// `qs -c <name> ipc call ghosthand ...`: the answer, or None when the overlay can't be reached.
    pub fn call(&self, args: &[&str]) -> Option<String> {
        let mut child = Command::new("qs").args(["-c", &self.name, "ipc", "call", "ghosthand"]).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
        let start = Instant::now();
        loop {
            match child.try_wait().ok()? {
                Some(status) => {
                    let mut out = String::new();
                    std::io::Read::read_to_string(&mut child.stdout.take()?, &mut out).ok()?;
                    return status.success().then(|| out.trim().to_string());
                }
                None if start.elapsed() < Duration::from_secs(5) => std::thread::sleep(Duration::from_millis(3)),
                None => {
                    let _ = child.kill();
                    return None;
                }
            }
        }
    }
}

impl Gate for Overlay {
    fn ask(&mut self, r: &Request) -> Option<String> {
        self.call(&["ask", &r.who, &r.reason, &r.seconds.to_string(), &r.scope, &r.detail])
    }
    fn status(&mut self) -> Option<String> { self.call(&["status"]) }
    fn deny(&mut self) { self.call(&["deny"]); }
}

/// The overlay when it answers, else a dialog tool if one is installed. One process spawn on the usual path (no separate probe).
pub struct Auto {
    overlay: Overlay,
    dialog: Option<DialogGate>,
    using_dialog: bool,
}

impl Gate for Auto {
    fn ask(&mut self, r: &Request) -> Option<String> {
        let answer = self.overlay.ask(r);
        if answer.is_none() {
            if let Some(d) = self.dialog.as_mut() {
                self.using_dialog = true;
                return d.ask(r);
            }
        }
        answer
    }
    fn status(&mut self) -> Option<String> {
        if self.using_dialog { self.dialog.as_mut()?.status() } else { self.overlay.status() }
    }
    fn deny(&mut self) {
        if self.using_dialog { if let Some(d) = self.dialog.as_mut() { d.deny() } } else { self.overlay.deny() }
    }
}

/// The gate to use, from the config's "gate": "auto", "overlay" or "dialog". None when there is nothing to ask with.
pub fn default_gate(cfg: &Config) -> Option<Box<dyn Gate>> {
    let overlay = Overlay { name: cfg.overlay() };
    match cfg.gate().as_str() {
        "dialog" => dialog::available().map(|t| Box::new(DialogGate::new(t)) as Box<dyn Gate>),
        "overlay" => Some(Box::new(overlay)),
        _ => {
            let dialog = dialog::available().map(DialogGate::new);
            if which("qs") || dialog.is_some() { Some(Box::new(Auto { overlay, dialog, using_dialog: false })) } else { None }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// A gate that answers from a script and records its calls.
    struct Script {
        answers: VecDeque<Option<String>>,
        calls: Vec<String>,
    }

    fn script(answers: &[Option<&str>]) -> Script {
        Script { answers: answers.iter().map(|a| a.map(String::from)).collect(), calls: vec![] }
    }

    impl Gate for Script {
        fn ask(&mut self, r: &Request) -> Option<String> {
            self.calls.push(format!("ask {} {} {} {}", r.who, r.reason, r.scope, r.detail));
            self.answers.pop_front().flatten()
        }
        fn status(&mut self) -> Option<String> {
            self.calls.push("status".into());
            self.answers.pop_front().flatten()
        }
        fn deny(&mut self) { self.calls.push("deny".into()); }
    }

    fn req() -> Request { Request { who: "Claude".into(), reason: "why".into(), seconds: 5, scope: "mouse".into(), detail: String::new() } }

    fn fast() { crate::util::FAST.store(true, std::sync::atomic::Ordering::Relaxed); }

    #[test]
    fn a_standing_grant_goes_straight_in() {
        let mut g = script(&[Some("allowed")]);
        wait_for_allow(&mut g, &req(), None).unwrap();
        assert_eq!(g.calls, vec!["ask Claude why mouse "]);
    }

    #[test]
    fn it_waits_for_the_answer_and_polls() {
        fast();
        let mut g = script(&[Some("asking"), Some("asking"), Some("asking"), Some("allowed")]);
        wait_for_allow(&mut g, &req(), None).unwrap();
        assert_eq!(g.calls.iter().filter(|c| *c == "status").count(), 3);
    }

    #[test]
    fn allow_shows_as_active_and_that_counts() {
        fast();
        wait_for_allow(&mut script(&[Some("asking"), Some("active")]), &req(), None).unwrap();
    }

    #[test]
    fn denied_timeout_and_revoked_are_all_no() {
        fast();
        for verdict in ["denied", "timeout", "revoked"] {
            let e = wait_for_allow(&mut script(&[Some("asking"), Some(verdict)]), &req(), None).unwrap_err();
            assert_eq!(e.code, EXIT_DENIED, "{verdict}");
        }
    }

    #[test]
    fn an_unreachable_gate_is_refused_not_skipped() {
        fast();
        for answers in [vec![None], vec![Some("asking"), None]] {
            let e = wait_for_allow(&mut script(&answers), &req(), None).unwrap_err();
            assert_eq!(e.code, EXIT_NO_GATE);
        }
    }

    #[test]
    fn no_answer_in_time_denies_and_closes_the_card() {
        fast();
        let mut g = script(&[Some("asking")]);
        g.answers.extend(std::iter::repeat(Some("asking".to_string())).take(100_000));
        let e = wait_for_allow(&mut g, &req(), Some(0.05)).unwrap_err();
        assert_eq!(e.code, EXIT_DENIED);
        assert_eq!(g.calls.last().unwrap(), "deny");
    }

    #[test]
    fn the_card_gets_the_scope_and_what_will_be_typed() {
        let mut g = script(&[Some("allowed")]);
        let r = Request { scope: "mouse and keyboard".into(), detail: "type: 'ls'".into(), ..req() };
        wait_for_allow(&mut g, &r, None).unwrap();
        assert_eq!(g.calls[0], "ask Claude why mouse and keyboard type: 'ls'");
    }

    #[test]
    fn the_lock_check_refuses_when_it_cannot_tell() {
        assert!(!locked("echo no"));
        assert!(locked("echo yes"));
        assert!(locked("echo"), "no answer counts as locked");
        assert!(locked("exit 3"));
        assert!(locked("this-command-does-not-exist-xyz"));
    }

    #[test]
    fn revoke_is_seen_only_by_runs_that_started_before_it() {
        crate::paths::use_temp_runtime_dir();
        let before = now() - 1.0;
        assert!(!revoked_since(before));
        revoke_local();
        assert!(revoked_since(before));
        assert!(!revoked_since(now() + 1.0), "a run that starts later is not affected");
    }
}
