//! A safe, end-to-end check of the real devices on your machine.
//!
//! It runs the real `ghosthand` binary in a throwaway runtime directory, then grabs the virtual pointer and keyboard exclusively (EVIOCGRAB) the
//! moment they appear, so every event goes to this program and none reach your desktop. It then checks the events are what they should be, that
//! the second call is fast (the device daemon), and that the daemon and its devices go away when idle.
//!
//! Reading /dev/input needs root (or the input group), so build first and run the example binary with sudo:
//!
//!     cargo build --release --examples
//!     sudo ./target/release/examples/live_test            # or: sudo ./target/release/examples/live_test path/to/ghosthand
//!
//! Nothing is typed or clicked in your session. If it can't grab the devices it refuses to start.
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type Events = Arc<Mutex<HashMap<String, Vec<(u16, u16, i32)>>>>;

fn input_names() -> Vec<(String, PathBuf)> {
    let Ok(dir) = fs::read_dir("/sys/class/input") else { return vec![] };
    dir.flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("event"))
        .filter_map(|e| Some((fs::read_to_string(e.path().join("device/name")).ok()?.trim().to_string(), PathBuf::from("/dev/input").join(e.file_name()))))
        .collect()
}

fn grab_and_read(name: String, node: PathBuf, events: Events) {
    let Ok(mut file) = fs::File::open(&node) else { eprintln!("can't read {}", node.display()); std::process::exit(2) };
    // EVIOCGRAB: from now on this process is the only reader, the desktop gets nothing
    if unsafe { libc::ioctl(file.as_raw_fd(), 0x4004_4590, 1) } < 0 {
        eprintln!("can't grab {name}: refusing to go on, events could reach your desktop");
        std::process::exit(2);
    }
    let mut buf = [0u8; 24 * 64];
    while let Ok(n) = file.read(&mut buf) {
        for chunk in buf[..n].chunks_exact(24) {
            let (kind, code, value) = (u16::from_ne_bytes([chunk[16], chunk[17]]), u16::from_ne_bytes([chunk[18], chunk[19]]), i32::from_ne_bytes([chunk[20], chunk[21], chunk[22], chunk[23]]));
            if kind != 0 {
                events.lock().unwrap().entry(name.clone()).or_default().push((kind, code, value));
            }
        }
    }
}

fn main() {
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("this reads /dev/input, so it needs root: sudo ./target/release/examples/live_test");
        std::process::exit(2);
    }
    let bin = std::env::args().nth(1).unwrap_or_else(|| "target/release/ghosthand".into());
    let dir = PathBuf::from(format!("/tmp/ghosthand-live-{}", std::process::id()));
    let (run, cfg) = (dir.join("run"), dir.join("cfg"));
    fs::create_dir_all(cfg.join("ghosthand")).unwrap();
    fs::create_dir_all(&run).unwrap();
    fs::set_permissions(&run, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(cfg.join("ghosthand/config.json"), r#"{"mode":"off","gate":"dialog","daemon_idle_seconds":3}"#).unwrap(); // the gate is off for this test only

    let events: Events = Arc::new(Mutex::new(HashMap::new()));
    let ev = events.clone();
    std::thread::spawn(move || {
        let mut seen: Vec<String> = vec![];
        loop {
            for (name, node) in input_names().into_iter().filter(|(n, _)| n.starts_with("ghosthand-")) {
                if !seen.contains(&name) {
                    seen.push(name.clone());
                    let ev = ev.clone();
                    std::thread::spawn(move || grab_and_read(name, node, ev));
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    });

    let call = |args: &[&str]| {
        let start = Instant::now();
        let out = Command::new(&bin).args(["--who", "Live test", "--reason", "isolated device check"]).args(args)
            .env("XDG_RUNTIME_DIR", &run).env("XDG_CONFIG_HOME", &cfg).env("GHOSTHAND_OUTPUT_SIZE", "1000x500").env("GHOSTHAND_SCALE", "1")
            .env_remove("WAYLAND_DISPLAY").env_remove("DISPLAY").output().expect("can't run ghosthand");
        (start.elapsed().as_secs_f64(), out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).trim().to_string())
    };

    let mut failures = 0;
    let mut check = |ok: bool, what: &str| {
        println!("  {} {what}", if ok { "ok " } else { "NO " });
        if !ok { failures += 1; }
    };

    let (cold, code, _) = call(&["jump 100 50", "click", "type Hi!"]);
    println!("first call:  {cold:.2} s (exit {code})");
    check(code == 0, "the first call succeeds");
    let (warm, code, _) = call(&["jump 500 250", "click", "type ok"]);
    println!("second call: {warm:.2} s (exit {code})");
    check(code == 0, "the second call succeeds");
    check(warm < 1.0, "the second call is fast: the devices are kept alive");
    let (pos, _, out) = call(&["pos"]);
    println!("pos:         {pos:.3} s  {out}");
    check(out.contains("\"x\":500") && out.contains("\"known\":true"), "pos reads back where the pointer was put");
    check(pos < 0.5, "a bare pos is quick");
    std::thread::sleep(Duration::from_millis(300));

    let map = events.lock().unwrap();
    let pointer = map.get("ghosthand-pointer").cloned().unwrap_or_default();
    let keyboard = map.get("ghosthand-keyboard").cloned().unwrap_or_default();
    drop(map);
    check(pointer.starts_with(&[(3, 0, 3276), (3, 1, 3276)]), "jump 100 50 on a 1000x500 screen is a tenth across the device range");
    check(pointer.contains(&(1, 0x110, 1)) && pointer.contains(&(1, 0x110, 0)), "click presses and releases the left button");
    let keys: Vec<(u16, i32)> = keyboard.iter().filter(|e| e.0 == 1).map(|e| (e.1, e.2)).collect();
    check(keys.starts_with(&[(42, 1), (35, 1), (35, 0), (42, 0), (23, 1), (23, 0), (42, 1), (2, 1), (2, 0), (42, 0)]), "type Hi! is Shift+h, i, Shift+1");
    check(keys.iter().filter(|k| k.1 == 1).count() == keys.iter().filter(|k| k.1 == 0).count(), "every key that went down came back up");

    println!("waiting for the daemon to go idle...");
    std::thread::sleep(Duration::from_secs(6));
    check(!run.join("ghosthand/devices.sock").exists(), "the daemon exits on its own when idle");
    check(input_names().iter().all(|(n, _)| !n.starts_with("ghosthand-")), "and its devices are gone");
    let _ = fs::remove_dir_all(&dir);
    println!("{}", if failures == 0 { "all good" } else { "some checks failed" });
    std::process::exit(if failures == 0 { 0 } else { 1 });
}
