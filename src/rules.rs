//! `ghosthand rules`: the short block of rules for an agent's global instructions file, the one it reads on every start.
//!
//! A skill is only read when an agent decides to load it. These rules sit in the file the agent always reads, so "ask for a session first"
//! and "never go around the gate" are in front of it before its first call. The block is marked, so installing it again updates it in place
//! and removing it leaves the rest of the file as it was. Nothing here ever creates a file or touches anything outside the markers.
use std::path::PathBuf;

use crate::gate::EXIT_USAGE;
use crate::paths;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BEGIN: &str = "<!-- ghosthand:rules:begin";
const END: &str = "<!-- ghosthand:rules:end -->";

/// The rules, without the markers. Plain text a person can read: no dashes as punctuation, no tool names but ghosthand's own.
pub const RULES: &str = r#"### Ghosthand: the user's real mouse and keyboard

Use `ghosthand` (run `ghosthand skill` once for the full guide) for anything that needs the actual desktop: testing a native app, right-click menus, drag and drop, hover states, typing into an app, shortcuts, checking how a panel really looks.

- The loop: look (`ghosthand 'shot /tmp/a.png'`, then read it), aim from that screenshot (coordinates are screenshot pixels), act, check with another screenshot. Click a field before typing and check `ghosthand focus` to see where the keys will land.
- One card per task, not per call. Before the first input action of any task that needs more than one call, ask once: `ghosthand session start --who <you> --reason "<the whole job>" --for 30m`, do all the work inside it, and run `ghosthand session end` when you are done. Never send plain calls one after another so the user has to click Allow again and again. If a reply carries a hint, or a session ran out, start a new one right away.
- Every action that moves the mouse or presses a key needs the user's approval on a card. Pass `--who <you>` and a specific `--reason`. Never answer your own request and never go around the gate: no wtype, xdotool or ydotool, and no writing to /dev/uinput.
- Exit 4 (denied or no answer) or 3 (taken back): stop and tell the user. Do not retry in a loop.
- `shot`, `wait`, `info`, `pos` and `focus` need no approval.
- Never touch anything destructive (delete, send, buy, close unsaved work, power) unless asked for exactly that, and never type secrets unless told to.
"#;

/// The global instruction files an agent reads on every start. Only the ones that exist are touched.
fn candidates() -> Vec<PathBuf> {
    let h = paths::home();
    [
        ".claude/CLAUDE.md",
        ".codex/AGENTS.md",
        ".gemini/GEMINI.md",
        ".gemini/antigravity-cli/AGENTS.md",
        ".gemini/antigravity-cli/GEMINI.md",
        ".config/opencode/AGENTS.md",
    ]
    .iter()
    .map(|p| h.join(p))
    .collect()
}

pub fn block() -> String {
    format!("{BEGIN} (ghosthand {VERSION}), managed by `ghosthand rules install`, edit outside this block -->\n{}{END}\n", RULES)
}

#[derive(Debug, PartialEq)]
pub enum State {
    Missing,
    Outdated,
    Current,
}

/// Where the managed block sits in `text`: byte range from the start of its first line to the end of its last.
fn find(text: &str) -> Option<(usize, usize)> {
    let start = text.find(BEGIN)?;
    let end = start + text[start..].find(END)? + END.len();
    let end = if text[end..].starts_with('\n') { end + 1 } else { end };
    Some((start, end))
}

pub fn state(text: &str) -> State {
    match find(text) {
        None => State::Missing,
        Some((s, e)) if text[s..e] == block() => State::Current,
        Some(_) => State::Outdated,
    }
}

/// `text` with the block added (at the end) or replaced in place.
pub fn upsert(text: &str) -> String {
    match find(text) {
        Some((s, e)) => format!("{}{}{}", &text[..s], block(), &text[e..]),
        None => {
            let mut out = text.trim_end().to_string();
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(&block());
            out
        }
    }
}

/// `text` without the block (and the blank line before it).
pub fn remove(text: &str) -> String {
    match find(text) {
        Some((s, e)) => {
            let head = text[..s].trim_end();
            let tail = text[e..].trim_start_matches('\n');
            if head.is_empty() { tail.to_string() } else if tail.is_empty() { format!("{head}\n") } else { format!("{head}\n\n{tail}") }
        }
        None => text.to_string(),
    }
}

fn write_atomic(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    // keep a symlink a symlink: write the file it points to
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let tmp = target.with_extension("ghosthand-tmp");
    std::fs::write(&tmp, text)?;
    if let Ok(meta) = std::fs::metadata(&target) {
        let _ = std::fs::set_permissions(&tmp, meta.permissions());
    }
    std::fs::rename(tmp, target)
}

fn targets(files: &[String]) -> Vec<PathBuf> {
    let mut seen = Vec::new();
    let list: Vec<PathBuf> = if files.is_empty() { candidates().into_iter().filter(|p| p.exists()).collect() } else { files.iter().map(PathBuf::from).collect() };
    list.into_iter().filter(|p| {
        let real = std::fs::canonicalize(p).unwrap_or_else(|_| p.clone());
        if seen.contains(&real) { false } else { seen.push(real); true }
    }).collect()
}

pub fn run(args: &[String]) -> i32 {
    let usage = || { eprintln!("usage: ghosthand rules [install|remove|status] [FILE...]   (no FILE: every global agent file that exists)"); EXIT_USAGE };
    match args.first().map(String::as_str) {
        None | Some("print") => { print!("{}", block()); 0 }
        Some(cmd @ ("install" | "remove" | "status")) => {
            let dry = args.iter().any(|a| a == "--dry-run");
            let files: Vec<String> = args[1..].iter().filter(|a| !a.starts_with("--")).cloned().collect();
            let list = targets(&files);
            if list.is_empty() {
                println!("none of the known agent instruction files exist here. Pass a FILE, or run `ghosthand rules` and paste the block yourself.");
                return 1;
            }
            let mut failed = false;
            for path in list {
                let text = match std::fs::read_to_string(&path) {
                    Ok(t) => t,
                    Err(e) => { eprintln!("{}: can't read it ({e})", path.display()); failed = true; continue; }
                };
                let label = match state(&text) { State::Missing => "no rules", State::Outdated => "rules out of date", State::Current => "rules up to date" };
                let new = match cmd { "install" => upsert(&text), "remove" => remove(&text), _ => { println!("{}: {label}", path.display()); continue; } };
                if new == text {
                    println!("{}: already {}", path.display(), if cmd == "install" { "up to date" } else { "clean" });
                } else if dry {
                    println!("{}: would {}", path.display(), if cmd == "install" { "update the rules" } else { "remove the rules" });
                } else if let Err(e) = write_atomic(&path, &new) {
                    eprintln!("{}: can't write it ({e})", path.display());
                    failed = true;
                } else {
                    println!("{}: {}", path.display(), if cmd == "install" { "rules installed" } else { "rules removed" });
                }
            }
            if failed { 1 } else { 0 }
        }
        Some(_) => usage(),
    }
}

/// For `doctor`: (file, state) for every global agent file that exists.
pub fn report() -> Vec<(PathBuf, State)> {
    targets(&[]).into_iter().filter_map(|p| std::fs::read_to_string(&p).ok().map(|t| (p, state(&t)))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_is_added_once_and_replaced_in_place() {
        let base = "# Rules\n\n1. Be kind.\n";
        let once = upsert(base);
        assert!(once.starts_with(base.trim_end()) && once.contains("ghosthand session start"));
        assert_eq!(upsert(&once), once, "installing twice changes nothing");
        assert_eq!(state(&once), State::Current);
        let stale = once.replace("(ghosthand ", "(ghosthand OLD ");
        assert_eq!(state(&stale), State::Outdated);
        assert_eq!(upsert(&stale), once, "an out of date block is replaced where it stands");
        let sandwiched = format!("{}\nAfter.\n", once);
        assert!(upsert(&sandwiched).ends_with("\nAfter.\n"), "what follows the block is kept");
    }

    #[test]
    fn removing_leaves_the_rest_of_the_file_alone() {
        let base = "# Rules\n\n1. Be kind.\n";
        assert_eq!(remove(&upsert(base)), base);
        assert_eq!(remove(base), base);
        let middle = upsert("Top\n") + "\nBottom\n";
        assert_eq!(remove(&middle), "Top\n\nBottom\n");
        assert_eq!(remove(&upsert("")), "");
    }

    #[test]
    fn the_rules_say_the_important_things_and_use_no_dashes_as_punctuation() {
        for needle in ["session start", "session end", "Never answer your own request", "Exit 4", "never type secrets", "ghosthand skill"] {
            assert!(RULES.contains(needle), "{needle}");
        }
        assert!(!RULES.contains('\u{2014}') && !RULES.contains('\u{2013}'));
        assert!(RULES.lines().count() < 25, "short enough to live in a global file");
    }

    #[test]
    fn install_remove_and_status_work_on_real_files_and_skip_missing_ones() {
        let dir = std::env::temp_dir().join(format!("gh-rules-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (a, b) = (dir.join("A.md"), dir.join("B.md"));
        std::fs::write(&a, "# A\n").unwrap();
        std::fs::write(&b, "# B\n").unwrap();
        let args = |c: &str| vec![c.to_string(), a.display().to_string(), b.display().to_string()];
        assert_eq!(run(&args("install")), 0);
        assert_eq!(state(&std::fs::read_to_string(&a).unwrap()), State::Current);
        assert_eq!(run(&args("status")), 0);
        assert_eq!(run(&args("remove")), 0);
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "# B\n");
        assert_eq!(run(&["install".into(), dir.join("nope.md").display().to_string()]), 1, "a file that isn't there is an error, never created");
        assert!(!dir.join("nope.md").exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
