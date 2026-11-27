//! Small shared helpers: the clock, pauses (switched off in tests) and reading a number from a file.
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// When set, `pause` returns at once. Tests turn it on so they run in milliseconds.
pub static FAST: AtomicBool = AtomicBool::new(false);

pub fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

pub fn pause(seconds: f64) {
    if seconds > 0.0 && !FAST.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_secs_f64(seconds));
    }
}

/// A number written in a small file, or 0 when the file is missing or isn't one.
pub fn read_float(path: &Path) -> f64 {
    std::fs::read_to_string(path).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0.0)
}

/// Is a program on PATH?
pub fn which(program: &str) -> bool {
    which_path(program).is_some()
}

pub fn which_path(program: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).map(|d| d.join(program)).find(|p| {
            use std::os::unix::fs::PermissionsExt;
            p.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
        })
    })
}

/// Run a program and return its stdout, or None if it can't run or fails.
pub fn output_of(program: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(program).args(args).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Why a run stopped early: the person took the pointer back, or something failed.
#[derive(Debug, PartialEq)]
pub enum Stop {
    Revoked,
    Failed(String),
}

impl From<std::io::Error> for Stop {
    fn from(e: std::io::Error) -> Stop { Stop::Failed(e.to_string()) }
}

impl From<String> for Stop {
    fn from(e: String) -> Stop { Stop::Failed(e) }
}

/// Called between steps: Err(Stop::Revoked) when the person has taken the pointer back.
pub type Check<'a> = &'a mut dyn FnMut() -> Result<(), Stop>;
