//! Where Ghosthand keeps things.
use std::path::PathBuf;

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// `$XDG_RUNTIME_DIR/ghosthand`, made on first use with mode 0700 (the stream, the revoke file, grants and the device socket live here).
pub fn runtime_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(d) = TEST_DIR.with(|d| d.borrow().clone()) {
        return d;
    }
    match env("XDG_RUNTIME_DIR") {
        Some(d) => PathBuf::from(d).join("ghosthand"),
        None => PathBuf::from(format!("/tmp/ghosthand-{}", unsafe { libc::getuid() })),
    }
}

pub fn ensure_runtime_dir() -> PathBuf {
    use std::os::unix::fs::DirBuilderExt;
    let dir = runtime_dir();
    let _ = std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&dir);
    dir
}

pub fn stream_file() -> PathBuf { runtime_dir().join("stream") }
pub fn last_position_file() -> PathBuf { runtime_dir().join("last.json") }
pub fn revoked_file() -> PathBuf { runtime_dir().join("revoked") }
pub fn session_file() -> PathBuf { runtime_dir().join("session.json") }
pub fn asks_file() -> PathBuf { runtime_dir().join("asks") }
pub fn socket_file() -> PathBuf { runtime_dir().join("devices.sock") }

/// When a standing "Allow for N min" ends. The keyboard has its own, so allowing the mouse never lets anything type.
pub fn grant_file(kind: &str) -> PathBuf {
    runtime_dir().join(if kind.is_empty() { "grant".to_string() } else { format!("grant-{kind}") })
}

pub fn home() -> PathBuf {
    env("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

pub fn config_home() -> PathBuf {
    env("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config"))
}

pub fn config_file() -> PathBuf { config_home().join("ghosthand").join("config.json") }

#[cfg(test)]
thread_local! {
    static TEST_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// Tests run on their own threads, so each gets its own runtime directory instead of sharing (and racing on) an environment variable.
#[cfg(test)]
pub fn use_temp_runtime_dir() -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!("gh-test-{}-{}", std::process::id(), COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    TEST_DIR.with(|d| *d.borrow_mut() = Some(dir.clone()));
    dir
}
