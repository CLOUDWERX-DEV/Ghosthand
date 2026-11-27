//! `ghosthand doctor`, `setup` and `skill`: check what's needed, install the overlay and the agent skill.
use std::path::PathBuf;

use crate::config::Config;
use crate::dialog;
use crate::gate::{locked, Overlay, EXIT_USAGE};
use crate::paths;
use crate::util::which;
use crate::{geometry, keyboard, layout, screen};

/// The overlay's files, built into the binary so `ghosthand setup` needs nothing else.
const OVERLAY: [(&str, &str); 9] = [
    ("Btn.qml", include_str!("../overlay/Btn.qml")),
    ("Gate.qml", include_str!("../overlay/Gate.qml")),
    ("Overlay.qml", include_str!("../overlay/Overlay.qml")),
    ("PointerGlyph.qml", include_str!("../overlay/PointerGlyph.qml")),
    ("PillWindow.qml", include_str!("../overlay/PillWindow.qml")),
    ("SessionPill.qml", include_str!("../overlay/SessionPill.qml")),
    ("Theme.qml", include_str!("../overlay/Theme.qml")),
    ("qmldir", include_str!("../overlay/qmldir")),
    ("shell.qml", include_str!("../overlay/shell.qml")),
];

pub const SKILL: &str = include_str!("../skills/ghosthand/SKILL.md");

fn overlay_dir(cfg: &Config) -> PathBuf {
    paths::config_home().join("quickshell").join(cfg.overlay())
}

fn session_label() -> String {
    for (key, label) in [("NIRI_SOCKET", "niri"), ("HYPRLAND_INSTANCE_SIGNATURE", "Hyprland"), ("SWAYSOCK", "sway")] {
        if std::env::var_os(key).is_some_and(|v| !v.is_empty()) {
            return label.into();
        }
    }
    std::env::var("XDG_CURRENT_DESKTOP").or_else(|_| std::env::var("XDG_SESSION_TYPE")).unwrap_or_else(|_| "unknown session".into())
}

/// [(what, ok, how to fix)] for everything Ghosthand needs here.
pub fn checks(cfg: &Config) -> Vec<(String, bool, &'static str)> {
    let mut out = vec![("write access to /dev/uinput".to_string(), writable("/dev/uinput"), "see docs/setup.md: a udev rule and the input group")];
    let tools = screen::order();
    out.push((format!("a screenshot tool ({})", tools.first().copied().unwrap_or("none")), !tools.is_empty() || cfg.screenshot_command().is_some(), "install grim (Wayland) or maim/scrot/ImageMagick (X11)"));
    out.push((format!("the screen size ({})", geometry::detected()), true, ""));
    out.push((format!("keyboard layout ({})", layout::current().name), true, ""));
    out.push((format!("a clipboard for `paste` ({})", if keyboard::clipboard_tools().is_some() { "found" } else { "none: install wl-clipboard, xclip or xsel" }), true, ""));
    let overlay_up = which("qs") && Overlay { name: cfg.overlay() }.call(&["status"]).is_some();
    let dlg = dialog::available();
    out.push((format!("something to ask you ({})", if overlay_up { "the overlay" } else { dlg.unwrap_or("nothing yet") }), overlay_up || dlg.is_some(), "run `ghosthand overlay` (Wayland with layer-shell, needs Quickshell) or install zenity, yad or kdialog"));
    out.push(("the session isn't locked".into(), !locked(&cfg.lock_check()), "unlock the session"));
    out
}

fn writable(path: &str) -> bool {
    let c = std::ffi::CString::new(path).unwrap();
    unsafe { libc::access(c.as_ptr(), libc::W_OK) == 0 }
}

pub fn doctor(cfg: &Config) -> i32 {
    println!("session: {}", session_label());
    let mut ok = true;
    for (name, good, fix) in checks(cfg) {
        println!("  {} {name}{}", if good { "ok " } else { "NO " }, if good { String::new() } else { format!("   -> {fix}") });
        ok &= good;
    }
    println!("{}", if ok { "everything is in place" } else { "some things are missing (see above)" });
    if ok { 0 } else { 1 }
}

/// Put the overlay where Quickshell finds it (replacing a previous copy) and write a default config if there isn't one.
pub fn setup(cfg: &Config) -> i32 {
    let dest = overlay_dir(cfg);
    let _ = std::fs::remove_file(&dest); // a symlink from a checkout
    let _ = std::fs::remove_dir_all(&dest);
    if let Err(e) = std::fs::create_dir_all(&dest) {
        eprintln!("can't create {}: {e}", dest.display());
        return 1;
    }
    for (name, text) in OVERLAY {
        if let Err(e) = std::fs::write(dest.join(name), text) {
            eprintln!("can't write {name}: {e}");
            return 1;
        }
    }
    println!("overlay installed in {}", dest.display());
    let config = paths::config_file();
    if !config.exists() {
        let _ = std::fs::create_dir_all(config.parent().unwrap());
        if std::fs::write(&config, "{\n  \"mode\": \"ask\",\n  \"countdown\": 8,\n  \"grant_minutes\": 10\n}\n").is_ok() {
            println!("wrote {}", config.display());
        }
    }
    println!("next: start the overlay (`ghosthand overlay`, or autostart it), then run `ghosthand doctor`. No Quickshell? Install zenity and skip the overlay.");
    0
}

/// `ghosthand skill` prints the agent skill; `ghosthand skill install [DIR]` puts it in DIR/ghosthand/ (default ~/.claude/skills).
pub fn skill(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        None => { print!("{SKILL}"); 0 }
        Some("install") => {
            let root = args.get(1).map(|d| expand(d)).unwrap_or_else(|| paths::home().join(".claude").join("skills"));
            let dir = root.join("ghosthand");
            match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(dir.join("SKILL.md"), SKILL)) {
                Ok(()) => { println!("skill installed in {}", dir.display()); 0 }
                Err(e) => { eprintln!("can't install the skill in {}: {e}", dir.display()); 1 }
            }
        }
        Some(_) => { eprintln!("usage: ghosthand skill [install [DIR]]"); EXIT_USAGE }
    }
}

fn expand(dir: &str) -> PathBuf {
    match dir.strip_prefix("~/") {
        Some(rest) => paths::home().join(rest),
        None => PathBuf::from(dir),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The skill's frontmatter must be valid, so any skill loader accepts it.
    fn frontmatter() -> (std::collections::HashMap<String, String>, &'static str) {
        let rest = SKILL.strip_prefix("---\n").expect("no frontmatter");
        let (head, body) = rest.split_once("\n---\n").expect("frontmatter isn't closed");
        (head.lines().filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect(), body)
    }

    #[test]
    fn the_skill_frontmatter_is_valid() {
        let (fm, body) = frontmatter();
        let name = &fm["name"];
        assert_eq!(name, "ghosthand");
        assert!(name.len() <= 64 && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
        let d = &fm["description"];
        assert!(!d.is_empty() && d.len() <= 1024, "{}", d.len());
        assert!(d.contains("Use when") && !d.contains('<'));
        assert!(body.lines().count() < 500);
    }

    #[test]
    fn the_skill_documents_every_action_and_exit_code() {
        for name in crate::actions::POINTER.iter().chain(crate::actions::KEYBOARD).chain(crate::actions::NO_INPUT) {
            assert!(SKILL.contains(&format!("`{name}")), "the skill doesn't mention {name}");
        }
        for code in 0..=5 {
            assert!(SKILL.contains(&format!("| {code} |")), "exit code {code}");
        }
    }

    #[test]
    fn the_skill_has_no_dashes_as_punctuation() {
        assert!(!SKILL.contains('\u{2014}') && !SKILL.contains('\u{2013}'));
    }

    #[test]
    fn the_embedded_overlay_is_complete() {
        for (name, text) in OVERLAY {
            assert!(!text.is_empty(), "{name}");
        }
        let qmldir = OVERLAY.iter().find(|(n, _)| *n == "qmldir").unwrap().1;
        for (name, _) in OVERLAY.iter().filter(|(n, _)| n.ends_with(".qml") && *n != "shell.qml") {
            assert!(qmldir.contains(name), "{name} isn't in the qmldir");
        }
    }
}
