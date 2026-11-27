//! A session: one approval for a stretch of work, instead of one per call.
//!
//! `ghosthand session start --for 30m` asks once (the card says what, for how long, and why), and writes `$XDG_RUNTIME_DIR/ghosthand/session.json`.
//! While it's active, runs that fit what was approved skip the card. It ends when the time is up, after a quiet spell with no use (idle stop),
//! on `ghosthand session end`, on a revoke, or when the session is locked. The overlay shows a pill with the time left and an End button while one
//! is on. The session never grants more than was asked for: a mouse-only session doesn't cover the keyboard.
use serde_json::{json, Value};

use crate::paths;
use crate::util::now;

pub const DEFAULT_SECONDS: f64 = 30.0 * 60.0;
pub const MAX_SECONDS: f64 = 8.0 * 3600.0;
pub const DEFAULT_IDLE: f64 = 10.0 * 60.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub who: String,
    pub reason: String,
    /// "mouse", "keyboard" or "mouse and keyboard".
    pub scope: String,
    pub started: f64,
    pub expires: f64,
    /// Seconds of no use after which it ends by itself.
    pub idle: f64,
    /// The last time a run used it.
    pub last: f64,
}

impl Session {
    pub fn new(who: &str, reason: &str, scope: &str, seconds: f64, idle: f64) -> Session {
        let t = now();
        Session { who: who.into(), reason: reason.into(), scope: scope.into(), started: t, expires: t + seconds.clamp(60.0, MAX_SECONDS), idle: idle.clamp(60.0, MAX_SECONDS), last: t }
    }

    pub fn active_at(&self, t: f64) -> bool {
        t < self.expires && t - self.last < self.idle
    }

    /// Does what this session approved include everything a request needs?
    pub fn covers(&self, scope: &str) -> bool {
        ["mouse", "keyboard"].iter().all(|w| !scope.contains(w) || self.scope.contains(w))
    }

    pub fn remaining(&self, t: f64) -> f64 {
        (self.expires - t).max(0.0)
    }

    fn to_json(&self) -> Value {
        json!({"who": self.who, "reason": self.reason, "scope": self.scope, "started": self.started, "expires": self.expires, "idle": self.idle, "last": self.last})
    }

    fn from_json(v: &Value) -> Option<Session> {
        Some(Session {
            who: v["who"].as_str()?.into(), reason: v["reason"].as_str().unwrap_or("").into(), scope: v["scope"].as_str()?.into(),
            started: v["started"].as_f64()?, expires: v["expires"].as_f64()?, idle: v["idle"].as_f64()?, last: v["last"].as_f64()?,
        })
    }
}

/// The active session, if there is one. An expired or idle one is removed on the way.
pub fn current() -> Option<Session> {
    let s = Session::from_json(&serde_json::from_str(&std::fs::read_to_string(paths::session_file()).ok()?).ok()?)?;
    if s.active_at(now()) {
        Some(s)
    } else {
        end();
        None
    }
}

pub fn start(s: &Session) -> std::io::Result<()> {
    paths::ensure_runtime_dir();
    let tmp = paths::session_file().with_extension("tmp");
    std::fs::write(&tmp, s.to_json().to_string())?;
    std::fs::rename(tmp, paths::session_file())
}

/// A run used it: the idle clock starts over.
pub fn touch() {
    if let Some(mut s) = current() {
        s.last = now();
        let _ = start(&s);
    }
}

pub fn end() {
    let _ = std::fs::remove_file(paths::session_file());
}

/// How far back "the person keeps clicking Allow" looks, in seconds.
pub const NUDGE_WINDOW: f64 = 600.0;
/// This many card approvals inside the window, with no session on, and the agent is told to ask for a session instead.
pub const NUDGE_AFTER: usize = 2;

/// A plain run was just allowed through the card. Remembers when, and returns how many were allowed inside the window (this one included).
pub fn note_gate_approval() -> usize {
    paths::ensure_runtime_dir();
    let t = now();
    let file = paths::asks_file();
    let mut times: Vec<f64> = std::fs::read_to_string(&file).unwrap_or_default().lines().filter_map(|l| l.trim().parse().ok()).filter(|x: &f64| t - x < NUDGE_WINDOW).collect();
    times.push(t);
    let keep = times.len().saturating_sub(20);
    let text: String = times[keep..].iter().map(|x| format!("{x:.3}\n")).collect();
    let _ = std::fs::write(&file, text);
    times.len()
}

/// What an agent is told when the person has had to click Allow again and again: one session instead of a card per call.
pub fn nudge(count: usize) -> String {
    format!(
        "Ghosthand: the person has now had to approve {count} separate runs in the last {} minutes. Stop sending a card per call. Ask ONCE for a session, then keep working inside it: \
         `ghosthand session start --who NAME --reason \"what you are doing\" --for 30m` (over MCP: the `session` tool, action \"start\"), and `ghosthand session end` when you are done.",
        (NUDGE_WINDOW / 60.0) as u32
    )
}

/// "30m", "1h", "1h30m", "90s", "45" (minutes) into seconds.
pub fn parse_duration(text: &str) -> Result<f64, String> {
    let t = text.trim().to_ascii_lowercase();
    if t.is_empty() {
        return Err("a duration like 30m, 1h or 90s".into());
    }
    let (mut total, mut num) = (0.0, String::new());
    for c in t.chars() {
        match c {
            '0'..='9' | '.' => num.push(c),
            'h' | 'm' | 's' => {
                let n: f64 = num.parse().map_err(|_| format!("'{text}' isn't a duration like 30m, 1h or 90s"))?;
                total += n * match c { 'h' => 3600.0, 'm' => 60.0, _ => 1.0 };
                num.clear();
            }
            _ => return Err(format!("'{text}' isn't a duration like 30m, 1h or 90s")),
        }
    }
    if !num.is_empty() {
        total += num.parse::<f64>().map_err(|_| format!("'{text}' isn't a duration"))? * 60.0; // a bare number is minutes
    }
    if total < 60.0 || total > MAX_SECONDS {
        return Err(format!("a session lasts from 1 minute to {} hours", MAX_SECONDS / 3600.0));
    }
    Ok(total)
}

pub fn describe(seconds: f64) -> String {
    let m = (seconds / 60.0).round() as u64;
    if m >= 60 && m % 60 == 0 { format!("{} h", m / 60) } else if m >= 60 { format!("{} h {} min", m / 60, m % 60) } else { format!("{m} min") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(parse_duration("30m").unwrap(), 1800.0);
        assert_eq!(parse_duration("1h").unwrap(), 3600.0);
        assert_eq!(parse_duration("1h30m").unwrap(), 5400.0);
        assert_eq!(parse_duration("90s").unwrap(), 90.0);
        assert_eq!(parse_duration("45").unwrap(), 2700.0, "a bare number is minutes");
        assert_eq!(parse_duration("1.5h").unwrap(), 5400.0);
        for bad in ["", "soon", "30x", "0m", "10s", "9h", "-5m", "m"] {
            assert!(parse_duration(bad).is_err(), "{bad}");
        }
        assert_eq!(describe(1800.0), "30 min");
        assert_eq!(describe(3600.0), "1 h");
        assert_eq!(describe(5400.0), "1 h 30 min");
    }

    #[test]
    fn a_session_covers_only_what_was_approved() {
        let both = Session::new("C", "r", "mouse and keyboard", 600.0, 300.0);
        assert!(both.covers("mouse") && both.covers("keyboard") && both.covers("mouse and keyboard"));
        let mouse = Session::new("C", "r", "mouse", 600.0, 300.0);
        assert!(mouse.covers("mouse"));
        assert!(!mouse.covers("keyboard") && !mouse.covers("mouse and keyboard"), "a mouse session never lets anything type");
    }

    #[test]
    fn it_ends_on_time_and_on_idle() {
        let s = Session { who: "C".into(), reason: "".into(), scope: "mouse".into(), started: 1000.0, expires: 2000.0, idle: 300.0, last: 1000.0 };
        assert!(s.active_at(1200.0));
        assert!(!s.active_at(1300.0), "quiet for the idle time");
        assert!(!s.active_at(2000.0), "out of time");
        let busy = Session { last: 1900.0, ..s.clone() };
        assert!(busy.active_at(1950.0) && !busy.active_at(2001.0));
        assert_eq!(s.remaining(1500.0), 500.0);
        assert_eq!(s.remaining(3000.0), 0.0);
    }

    #[test]
    fn it_round_trips_through_the_file_and_ends() {
        crate::paths::use_temp_runtime_dir();
        assert!(current().is_none());
        let s = Session::new("Claude", "UI testing", "mouse and keyboard", 1800.0, 600.0);
        start(&s).unwrap();
        assert_eq!(current().unwrap().reason, "UI testing");
        let before = current().unwrap().last;
        std::thread::sleep(std::time::Duration::from_millis(5));
        touch();
        assert!(current().unwrap().last > before, "using it restarts the idle clock");
        end();
        assert!(current().is_none());
    }

    #[test]
    fn repeated_card_approvals_are_counted_inside_the_window_only() {
        crate::paths::use_temp_runtime_dir();
        assert_eq!(note_gate_approval(), 1);
        assert_eq!(note_gate_approval(), 2);
        let old = format!("{:.1}\n", now() - NUDGE_WINDOW - 5.0);
        let fresh = std::fs::read_to_string(crate::paths::asks_file()).unwrap();
        std::fs::write(crate::paths::asks_file(), old + &fresh).unwrap();
        assert_eq!(note_gate_approval(), 3, "an approval older than the window is forgotten");
        assert!(nudge(3).contains("session start") && nudge(3).contains("3 separate runs"));
    }

    #[test]
    fn an_expired_file_is_cleaned_up_and_junk_is_ignored() {
        crate::paths::use_temp_runtime_dir();
        let old = Session { who: "C".into(), reason: "".into(), scope: "mouse".into(), started: 1.0, expires: 2.0, idle: 60.0, last: 1.0 };
        start(&old).unwrap();
        assert!(current().is_none());
        assert!(!crate::paths::session_file().exists(), "removed");
        crate::paths::ensure_runtime_dir();
        std::fs::write(crate::paths::session_file(), "not json").unwrap();
        assert!(current().is_none());
    }
}
