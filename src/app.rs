//! The real services: the gate, devices, lock check, screen and stream a run uses on a real desktop.
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::Config;
use crate::daemon::DaemonSink;
use crate::gate::{self, default_gate, locked, revoked_since};
use crate::geometry::output_info;
use crate::layout;
use crate::session::{execute, Devices, FileStream, Outcome, Services};
use crate::uinput::{self, Sink};
use crate::util::now;

type Made = std::thread::JoinHandle<Result<Box<dyn Sink + Send>, String>>;

/// Devices from the daemon when it is on (fast: they already exist), else devices of our own.
pub struct RealDevices {
    pub cfg: Config,
    pending: Vec<(char, Made)>,
}

impl RealDevices {
    pub fn new(cfg: Config) -> RealDevices { RealDevices { cfg, pending: Vec::new() } }

    fn start(&mut self, dev: char) {
        let cfg = self.cfg.clone();
        self.pending.push((dev, std::thread::spawn(move || -> Result<Box<dyn Sink + Send>, String> {
            match DaemonSink::open(&cfg, dev) {
                Some(Ok(s)) => Ok(Box::new(s)),
                Some(Err(e)) => Err(e),
                None => (if dev == 'p' { uinput::pointer_device(1.0) } else { uinput::keyboard_device(0.8) }).map(|d| Box::new(d) as Box<dyn Sink + Send>).map_err(|e| e.to_string()),
            }
        })));
    }

    fn take(&mut self, dev: char) -> Result<Box<dyn Sink>, String> {
        if !self.pending.iter().any(|(d, _)| *d == dev) {
            self.start(dev);
        }
        let at = self.pending.iter().position(|(d, _)| *d == dev).ok_or("no device")?;
        let sink = self.pending.remove(at).1.join().map_err(|_| "the device thread crashed".to_string())??;
        Ok(Box::new(Forward(sink)))
    }
}

/// A `Send` sink as a plain one.
struct Forward(Box<dyn Sink + Send>);

impl Sink for Forward {
    fn send(&mut self, events: &[uinput::Event]) -> std::io::Result<()> { self.0.send(events) }
    fn flush_pause(&self) -> f64 { self.0.flush_pause() }
}

impl Devices for RealDevices {
    fn warm(&mut self, pointer: bool, keyboard: bool) {
        if pointer { self.start('p'); }
        if keyboard { self.start('k'); }
    }
    fn pointer(&mut self) -> Result<Box<dyn Sink>, String> { self.take('p') }
    fn keyboard(&mut self) -> Result<Box<dyn Sink>, String> { self.take('k') }
}

pub fn services(cfg: &Config) -> Services {
    let since = now();
    let (lock_cmd, output) = (cfg.lock_check(), cfg.output());
    Services {
        gate: default_gate(cfg),
        devices: Box::new(RealDevices::new(cfg.clone())),
        locked: Box::new(move || locked(&lock_cmd)),
        info: Box::new(move || output_info(output.as_deref())),
        stream: Rc::new(RefCell::new(FileStream::open())),
        revoked: Box::new(move || revoked_since(since)),
        gate_patience: None,
    }
}

/// Run actions on the real desktop: what the command line and the MCP server both do.
pub fn run_actions(who: &str, reason: &str, countdown: u32, actions: &[String]) -> Outcome {
    let cfg = Config::load();
    let mut svc = services(&cfg);
    let outcome = execute(who, reason, countdown, actions, &mut svc, &cfg, layout::current());
    let _ = gate::EXIT_FAILED;
    outcome
}

/// The shape the MCP server wants (it builds its own owned arguments).
pub fn run_actions_dyn(who: &str, reason: &str, countdown: u32, actions: &[String]) -> Outcome {
    run_actions(who, reason, countdown, actions)
}
