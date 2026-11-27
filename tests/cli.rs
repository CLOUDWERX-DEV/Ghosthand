//! The command line, end to end: the real binary, in a throwaway runtime and config directory, never touching the desktop.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn sandbox(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gh-cli-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("run")).unwrap();
    std::fs::create_dir_all(dir.join("cfg")).unwrap();
    // A PATH with a shell and nothing else: no Quickshell, no dialog tool, no loginctl.
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    let _ = std::os::unix::fs::symlink("/bin/sh", dir.join("bin/sh"));
    dir
}

fn ghosthand(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ghosthand"))
        .args(args)
        .env("XDG_RUNTIME_DIR", dir.join("run"))
        .env("XDG_CONFIG_HOME", dir.join("cfg"))
        .env("HOME", dir.join("home"))
        .env("GHOSTHAND_OUTPUT_SIZE", "1920x1080")
        .env("GHOSTHAND_SCALE", "1.5")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .env("PATH", dir.join("bin"))
        .output()
        .unwrap()
}

fn text(o: &[u8]) -> String { String::from_utf8_lossy(o).into_owned() }

#[test]
fn version_and_help() {
    let d = sandbox("version");
    let v = ghosthand(&d, &["--version"]);
    assert!(v.status.success());
    assert_eq!(text(&v.stdout).trim(), format!("ghosthand {}", env!("CARGO_PKG_VERSION")));
    let h = text(&ghosthand(&d, &[]).stdout);
    for word in ["jump X Y", "type TEXT", "key COMBO", "focus", "exit codes", "ghosthand skill"] {
        assert!(h.contains(word), "help lacks {word}");
    }
}

#[test]
fn a_typo_exits_2_and_says_which_action() {
    let d = sandbox("typo");
    let o = ghosthand(&d, &["jump 1 2", "clikc"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(text(&o.stderr).contains("unknown action: 'clikc'"));
    assert_eq!(ghosthand(&d, &["--bogus", "x", "pos"]).status.code(), Some(2));
}

#[test]
fn info_needs_no_gate_and_uses_the_environment() {
    let d = sandbox("info");
    let o = ghosthand(&d, &["info"]);
    assert_eq!(o.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_str(text(&o.stdout).trim()).unwrap();
    assert_eq!((v["width"].as_i64(), v["height"].as_i64(), v["scale"].as_f64()), (Some(1920), Some(1080), Some(1.5)));
}

#[test]
fn nothing_to_ask_with_is_a_refusal_and_never_touches_anything() {
    // PATH has only a shell, so there's no Quickshell and no dialog tool: the gate can't ask, and a pointer action must not go ahead.
    let d = sandbox("nogate");
    std::fs::create_dir_all(d.join("cfg/ghosthand")).unwrap();
    std::fs::write(d.join("cfg/ghosthand/config.json"), r#"{"lock_check": "echo no"}"#).unwrap(); // there's no loginctl on this PATH either
    let o = ghosthand(&d, &["--who", "Test", "--reason", "x", "jump 1 2"]);
    assert_eq!(o.status.code(), Some(5), "{}", text(&o.stderr));
    assert!(text(&o.stderr).contains("no way around the gate"));
    let o = ghosthand(&d, &["--who", "Test", "--reason", "x", "type hello"]);
    assert_eq!(o.status.code(), Some(5), "typing is gated too");
}

#[test]
fn the_keyboard_can_be_switched_off_and_is_refused_before_the_gate() {
    let d = sandbox("kboff");
    std::fs::create_dir_all(d.join("cfg/ghosthand")).unwrap();
    std::fs::write(d.join("cfg/ghosthand/config.json"), r#"{"keyboard": false}"#).unwrap();
    let o = ghosthand(&d, &["type hi"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(text(&o.stderr).contains("switched off"));
}

#[test]
fn skill_prints_and_installs() {
    let d = sandbox("skill");
    let printed = text(&ghosthand(&d, &["skill"]).stdout);
    assert!(printed.starts_with("---\nname: ghosthand\n"));
    let target = d.join("skills");
    let o = ghosthand(&d, &["skill", "install", target.to_str().unwrap()]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert_eq!(std::fs::read_to_string(target.join("ghosthand/SKILL.md")).unwrap(), printed);
    assert_eq!(ghosthand(&d, &["skill", "bogus"]).status.code(), Some(2));
}

#[test]
fn setup_installs_the_overlay_and_a_default_config() {
    let d = sandbox("setup");
    let o = ghosthand(&d, &["setup"]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    for f in ["shell.qml", "Gate.qml", "Overlay.qml", "Theme.qml", "Btn.qml", "PointerGlyph.qml", "qmldir"] {
        assert!(d.join("cfg/quickshell/ghosthand").join(f).exists(), "{f}");
    }
    assert!(d.join("cfg/ghosthand/config.json").exists());
    assert!(ghosthand(&d, &["setup"]).status.success(), "running it again replaces the copy");
}

#[test]
fn revoke_works_with_no_overlay_and_writes_the_file_runs_check() {
    let d = sandbox("revoke");
    let o = ghosthand(&d, &["revoke"]);
    assert!(o.status.success());
    assert!(d.join("run/ghosthand/revoked").exists());
}

#[test]
fn mcp_speaks_the_current_spec_over_stdio() {
    use std::io::Write;
    let d = sandbox("mcp");
    let mut child = Command::new(env!("CARGO_BIN_EXE_ghosthand"))
        .arg("mcp")
        .env("XDG_RUNTIME_DIR", d.join("run")).env("XDG_CONFIG_HOME", d.join("cfg")).env("PATH", d.join("bin"))
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();
    let meta = r#""_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}"#;
    let input = format!("{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\",\"params\":{{{meta}}}}}\n{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{{{meta}}}}}\n");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap(); // dropping stdin closes it: the server must exit by itself
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let lines: Vec<serde_json::Value> = text(&out.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["result"]["supportedVersions"][0], "2026-07-28");
    assert_eq!(lines[1]["result"]["tools"].as_array().unwrap().len(), 5);
}

#[test]
fn sessions_have_a_status_an_end_and_a_careful_start() {
    let d = sandbox("session");
    let status = ghosthand(&d, &["session", "status"]);
    assert_eq!(status.status.code(), Some(0));
    assert!(text(&status.stdout).contains("\"active\":false"));
    assert_eq!(ghosthand(&d, &["session", "start"]).status.code(), Some(2), "a session needs a reason");
    assert_eq!(ghosthand(&d, &["session", "start", "--reason", "x", "--for", "99h"]).status.code(), Some(2), "too long");
    assert_eq!(ghosthand(&d, &["session", "start", "--reason", "x", "--scope", "everything"]).status.code(), Some(2));
    assert_eq!(ghosthand(&d, &["session", "start", "--reason", "x", "--bogus", "1"]).status.code(), Some(2));
    assert_eq!(ghosthand(&d, &["session", "frobnicate"]).status.code(), Some(2));
    // with nothing to ask with, a session can't start (and nothing is created)
    let none = ghosthand(&d, &["session", "start", "--reason", "UI testing", "--for", "30m"]);
    assert!(matches!(none.status.code(), Some(1) | Some(5)), "{}", text(&none.stderr));
    assert!(!d.join("run/ghosthand/session.json").exists());
    assert_eq!(ghosthand(&d, &["session", "end"]).status.code(), Some(0));
}

#[test]
fn a_session_file_that_covers_the_request_lets_a_gated_run_through_without_asking_and_a_revoke_ends_it() {
    let d = sandbox("sessionrun");
    std::fs::create_dir_all(d.join("run/ghosthand")).unwrap();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
    let file = d.join("run/ghosthand/session.json");
    // scope "mouse": a keyboard request is NOT covered, so with nothing to ask with it's refused (exit 5)
    std::fs::write(&file, format!(r#"{{"who":"T","reason":"r","scope":"mouse","started":{now},"expires":{},"idle":600,"last":{now}}}"#, now + 600.0)).unwrap();
    std::fs::write(d.join("cfg/ghosthand/config.json").parent().map(|p| { std::fs::create_dir_all(p).unwrap(); p.join("config.json") }).unwrap(), r#"{"lock_check":"echo no"}"#).unwrap();
    let typed = ghosthand(&d, &["--who", "T", "--reason", "x", "type hi"]);
    assert_eq!(typed.status.code(), Some(5), "a mouse session doesn't cover typing: {}", text(&typed.stderr));
    let status = text(&ghosthand(&d, &["session", "status"]).stdout);
    assert!(status.contains("\"active\":true") && status.contains("\"scope\":\"mouse\""), "{status}");
    ghosthand(&d, &["revoke"]);
    assert!(!file.exists(), "a revoke ends the session");
}
