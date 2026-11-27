//! The device daemon: keeps the virtual pointer and keyboard alive between runs.
//!
//! A new uinput device takes about a second to be picked up by the compositor, which used to be paid on every call. The first call now starts
//! `ghosthand daemon` in the background; it creates the devices once and forwards raw events from later calls over a Unix socket, so they start
//! in milliseconds. It exits by itself after `daemon_idle_seconds` (default 120) with nobody connected. If a client goes away holding a key or a
//! button, the daemon lets go. The gate is not in here: it stays in each run, so the daemon forwards events, nothing more. The socket is
//! `$XDG_RUNTIME_DIR/ghosthand/devices.sock`, mode 0600. Set `"daemon": false` in the config to make every run create its own devices instead.
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::paths;
use crate::uinput::{self, Event, Sink, Uinput, EV_KEY};

type Devices = Arc<Mutex<HashMap<char, Uinput>>>;

fn release_held(devices: &Devices, held: &HashSet<(char, u16)>) {
    let mut map = devices.lock().unwrap_or_else(|e| e.into_inner());
    for (dev, code) in held {
        if let Some(d) = map.get_mut(dev) {
            let _ = d.send(&[(EV_KEY, *code, 0)]);
        }
    }
}

/// Make a device if it doesn't exist yet. The lock is not held while the compositor picks it up, so the pointer and keyboard can be made together.
fn open_device(devices: &Devices, dev: char, making: &Arc<Mutex<HashSet<char>>>) -> Result<(), String> {
    if !matches!(dev, 'p' | 'k') {
        return Err(format!("no such device: {dev}"));
    }
    loop {
        if devices.lock().unwrap_or_else(|e| e.into_inner()).contains_key(&dev) {
            return Ok(());
        }
        if making.lock().unwrap_or_else(|e| e.into_inner()).insert(dev) {
            break; // ours to make
        }
        std::thread::sleep(Duration::from_millis(10)); // another client is making it: wait for it
    }
    let made = if dev == 'p' { uinput::pointer_device(1.0) } else { uinput::keyboard_device(0.8) };
    making.lock().unwrap_or_else(|e| e.into_inner()).remove(&dev);
    devices.lock().unwrap_or_else(|e| e.into_inner()).insert(dev, made.map_err(|e| e.to_string())?);
    Ok(())
}

fn handle(stream: UnixStream, devices: Devices, making: Arc<Mutex<HashSet<char>>>) {
    let mut writer = match stream.try_clone() { Ok(w) => w, Err(_) => return };
    let mut held: HashSet<(char, u16)> = HashSet::new();
    for line in BufReader::new(stream).lines().map_while(Result::ok) {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("open") => {
                let reply = match words.next().and_then(|w| w.chars().next()).ok_or_else(|| "open what?".to_string()).and_then(|d| open_device(&devices, d, &making)) {
                    Ok(()) => "ok\n".to_string(),
                    Err(e) => format!("err {e}\n"),
                };
                if writer.write_all(reply.as_bytes()).is_err() { break; }
            }
            Some("e") => {
                let Some(dev) = words.next().and_then(|w| w.chars().next()) else { continue };
                let nums: Vec<i64> = words.filter_map(|w| w.parse().ok()).collect();
                let events: Vec<Event> = nums.chunks_exact(3).map(|c| (c[0] as u16, c[1] as u16, c[2] as i32)).collect();
                for (kind, code, value) in &events {
                    if *kind == EV_KEY {
                        match value { 1 => { held.insert((dev, *code)); } 0 => { held.remove(&(dev, *code)); } _ => {} }
                    }
                }
                if let Some(d) = devices.lock().unwrap_or_else(|e| e.into_inner()).get_mut(&dev) {
                    let _ = d.send(&events);
                }
            }
            Some("bye") => break,
            _ => {}
        }
    }
    release_held(&devices, &held); // never leave a key or button held
}

/// Run the daemon in the foreground. Returns an exit code.
pub fn serve(idle: Duration) -> i32 {
    paths::ensure_runtime_dir();
    let path = paths::socket_file();
    if UnixStream::connect(&path).is_ok() {
        return 0; // one is already running
    }
    let _ = std::fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => { eprintln!("ghosthand daemon: can't listen on {}: {e}", path.display()); return 1; }
    };
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    let _ = listener.set_nonblocking(true);
    let devices: Devices = Arc::new(Mutex::new(HashMap::new()));
    let making: Arc<Mutex<HashSet<char>>> = Arc::new(Mutex::new(HashSet::new()));
    let active = Arc::new(AtomicUsize::new(0));
    let last = Arc::new(AtomicU64::new(0));
    let start = Instant::now();
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = stream.set_nonblocking(false);
                active.fetch_add(1, Ordering::SeqCst);
                let (devices, active, last, making) = (devices.clone(), active.clone(), last.clone(), making.clone());
                std::thread::spawn(move || {
                    handle(stream, devices, making);
                    last.store(start.elapsed().as_millis() as u64, Ordering::SeqCst);
                    active.fetch_sub(1, Ordering::SeqCst);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(25)),
            Err(_) => break,
        }
        let quiet_for = start.elapsed().saturating_sub(Duration::from_millis(last.load(Ordering::SeqCst)));
        if !idle.is_zero() && active.load(Ordering::SeqCst) == 0 && quiet_for > idle {
            break;
        }
    }
    let _ = std::fs::remove_file(&path);
    0
}

/// Start the daemon in the background (a new session, no terminal).
fn spawn() -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    let mut cmd = Command::new(exe);
    cmd.arg("daemon").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    unsafe { cmd.pre_exec(|| { libc::setsid(); Ok(()) }); }
    cmd.spawn().is_ok()
}

fn connect() -> Option<UnixStream> {
    let path = paths::socket_file();
    if let Ok(s) = UnixStream::connect(&path) {
        return Some(s);
    }
    if !spawn() {
        return None;
    }
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(20));
        if let Ok(s) = UnixStream::connect(&path) {
            return Some(s);
        }
    }
    None
}

/// A device on the daemon: events go over the socket. Letting go of it (dropping) lets the daemon release anything still held.
pub struct DaemonSink {
    stream: UnixStream,
    dev: char,
}

impl DaemonSink {
    /// Connect (starting the daemon if needed) and make sure the device exists. None means "use a device of your own instead".
    pub fn open(cfg: &Config, dev: char) -> Option<Result<DaemonSink, String>> {
        if !cfg.daemon() {
            return None;
        }
        let stream = connect()?;
        stream.set_read_timeout(Some(Duration::from_secs(15))).ok()?;
        let mut reader = BufReader::new(stream.try_clone().ok()?);
        let mut writer = stream.try_clone().ok()?;
        writer.write_all(format!("open {dev}\n").as_bytes()).ok()?;
        let mut reply = String::new();
        reader.read_line(&mut reply).ok()?;
        Some(match reply.trim() {
            "ok" => Ok(DaemonSink { stream, dev }),
            other => Err(other.strip_prefix("err ").unwrap_or(other).to_string()),
        })
    }
}

impl Sink for DaemonSink {
    fn send(&mut self, events: &[Event]) -> std::io::Result<()> {
        let mut line = format!("e {}", self.dev);
        for (k, c, v) in events {
            line += &format!(" {k} {c} {v}");
        }
        line.push('\n');
        self.stream.write_all(line.as_bytes())
    }
    fn flush_pause(&self) -> f64 { 0.0 }
}

impl Drop for DaemonSink {
    fn drop(&mut self) {
        let _ = self.stream.write_all(b"bye\n");
    }
}
