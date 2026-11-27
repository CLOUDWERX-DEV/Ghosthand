//! `~/.config/ghosthand/config.json`. Every key is optional; a missing or broken file gives the defaults.
//!
//! mode (ask | notify | off), countdown, grant_minutes, accent and ui_scale are read by the overlay, never taken from a client, so a program can't
//! switch its own gate off. The rest is read here.
use serde_json::Value;

use crate::paths;

pub const DEFAULT_LOCK_CHECK: &str = "loginctl show-session \"${XDG_SESSION_ID:-auto}\" -p LockedHint --value";

#[derive(Debug, Clone, Default)]
pub struct Config(Value);

impl Config {
    pub fn load() -> Config {
        match std::fs::read_to_string(paths::config_file()).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()) {
            Some(v) if v.is_object() => Config(v),
            _ => Config(Value::Null),
        }
    }

    #[cfg(test)]
    pub fn from_json(text: &str) -> Config {
        Config(serde_json::from_str(text).unwrap_or(Value::Null))
    }

    fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key).filter(|v| !v.is_null())
    }

    pub fn string(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| v.as_str()).map(str::to_string).filter(|s| !s.is_empty())
    }

    pub fn number(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_f64())
    }

    pub fn flag(&self, key: &str, default: bool) -> bool {
        self.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    pub fn overlay(&self) -> String { self.string("overlay").unwrap_or_else(|| "ghosthand".into()) }
    pub fn gate(&self) -> String { self.string("gate").unwrap_or_else(|| "auto".into()) }
    pub fn lock_check(&self) -> String { self.string("lock_check").unwrap_or_else(|| DEFAULT_LOCK_CHECK.into()) }
    pub fn output(&self) -> Option<String> { self.string("output") }
    pub fn screenshot_command(&self) -> Option<String> { self.string("screenshot") }
    pub fn keyboard_enabled(&self) -> bool { self.flag("keyboard", true) }
    pub fn paste_terminal(&self) -> bool { self.flag("paste_terminal", false) }
    pub fn daemon(&self) -> bool { self.flag("daemon", true) }
    pub fn daemon_idle_seconds(&self) -> u64 { self.number("daemon_idle_seconds").map(|n| n.max(0.0) as u64).unwrap_or(120) }
    pub fn type_delay_seconds(&self) -> f64 { self.number("type_delay_ms").unwrap_or(8.0).clamp(0.0, 500.0) / 1000.0 }

    /// "ask" (default), "notify" or "off".
    pub fn mode(&self) -> String {
        match self.string("mode").as_deref() {
            Some(m @ ("ask" | "notify" | "off")) => m.to_string(),
            _ => "ask".into(),
        }
    }
    pub fn countdown(&self) -> u32 { self.number("countdown").unwrap_or(8.0).clamp(2.0, 60.0) as u32 }
    pub fn grant_minutes(&self) -> u32 { self.number("grant_minutes").unwrap_or(10.0).clamp(1.0, 240.0) as u32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_missing_or_broken() {
        for c in [Config::from_json("not json"), Config::from_json("[]"), Config::default()] {
            assert_eq!(c.mode(), "ask");
            assert_eq!(c.countdown(), 8);
            assert_eq!(c.overlay(), "ghosthand");
            assert!(c.keyboard_enabled() && c.daemon());
            assert_eq!(c.daemon_idle_seconds(), 120);
        }
    }

    #[test]
    fn values_are_clamped_and_validated() {
        let c = Config::from_json(r#"{"mode":"bogus","countdown":500,"grant_minutes":0,"type_delay_ms":9999,"keyboard":false,"daemon":false}"#);
        assert_eq!(c.mode(), "ask");
        assert_eq!(c.countdown(), 60);
        assert_eq!(c.grant_minutes(), 1);
        assert_eq!(c.type_delay_seconds(), 0.5);
        assert!(!c.keyboard_enabled() && !c.daemon());
        assert_eq!(Config::from_json(r#"{"mode":"notify"}"#).mode(), "notify");
    }

    #[test]
    fn nulls_and_empty_strings_count_as_unset() {
        let c = Config::from_json(r#"{"overlay":"","output":null}"#);
        assert_eq!(c.overlay(), "ghosthand");
        assert_eq!(c.output(), None);
    }
}
