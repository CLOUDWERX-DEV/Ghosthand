//! Virtual input devices through /dev/uinput. The display server sees real hardware: hover, press, release, drag, the wheel and keys all work,
//! on layer-shell surfaces as well as windows. Needs write access to /dev/uinput (docs/setup.md).
use std::io;
use std::os::unix::io::RawFd;

use crate::util::pause;

pub const EV_SYN: u16 = 0;
pub const EV_KEY: u16 = 1;
pub const EV_REL: u16 = 2;
pub const EV_ABS: u16 = 3;
pub const BTN_LEFT: u16 = 0x110;
pub const BTN_RIGHT: u16 = 0x111;
pub const BTN_MIDDLE: u16 = 0x112;
pub const REL_HWHEEL: u16 = 6;
pub const REL_WHEEL: u16 = 8;
pub const REL_WHEEL_HI_RES: u16 = 11;
pub const REL_HWHEEL_HI_RES: u16 = 12;
pub const ABS_X: u16 = 0;
pub const ABS_Y: u16 = 1;
pub const ABS_MAX: i32 = 32767;

const UI_SET_EVBIT: libc::c_ulong = 0x4004_5564;
const UI_SET_KEYBIT: libc::c_ulong = 0x4004_5565;
const UI_SET_RELBIT: libc::c_ulong = 0x4004_5566;
const UI_SET_ABSBIT: libc::c_ulong = 0x4004_5567;
const UI_DEV_CREATE: libc::c_ulong = 0x5501;
const UI_DEV_DESTROY: libc::c_ulong = 0x5502;

/// One event: (type, code, value). A batch is sent together and followed by one SYN_REPORT.
pub type Event = (u16, u16, i32);

/// Where input events go: a real uinput device, the device daemon, or a recorder in tests.
pub trait Sink {
    fn send(&mut self, events: &[Event]) -> io::Result<()>;
    /// Seconds to wait after the last event before letting the device go, so they are all delivered. The daemon keeps its devices, so it needs none.
    fn flush_pause(&self) -> f64 { 0.2 }
}

pub struct Uinput {
    fd: RawFd,
}

fn ioctl(fd: RawFd, request: libc::c_ulong, arg: libc::c_int) -> io::Result<()> {
    if unsafe { libc::ioctl(fd, request, arg) } < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

/// Make a device. `absolute`: (axis, max). `settle`: seconds to let the compositor pick it up.
pub fn create_device(name: &str, keys: &[u16], rels: &[u16], absolute: &[(u16, i32)], settle: f64) -> io::Result<Uinput> {
    use std::ffi::CString;
    let path = CString::new("/dev/uinput").unwrap();
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_NONBLOCK) };
    if fd < 0 {
        let e = io::Error::last_os_error();
        return Err(io::Error::new(e.kind(), format!("can't open /dev/uinput: {e} (see docs/setup.md: a udev rule and the input group)")));
    }
    let dev = Uinput { fd }; // dropped (and the fd closed) on any error below
    ioctl(fd, UI_SET_EVBIT, EV_SYN as i32)?;
    for (kind, codes, request) in [(EV_KEY, keys.to_vec(), UI_SET_KEYBIT), (EV_REL, rels.to_vec(), UI_SET_RELBIT), (EV_ABS, absolute.iter().map(|a| a.0).collect(), UI_SET_ABSBIT)] {
        if !codes.is_empty() {
            ioctl(fd, UI_SET_EVBIT, kind as i32)?;
            for c in codes {
                ioctl(fd, request, c as i32)?;
            }
        }
    }
    // struct uinput_user_dev: name[80], id { bustype, vendor, product, version }, ff_effects_max, absmax/absmin/absfuzz/absflat [64] each
    let mut buf = vec![0u8; 80 + 8 + 4 + 4 * 64 * 4];
    buf[..name.len().min(79)].copy_from_slice(&name.as_bytes()[..name.len().min(79)]);
    let product: u16 = if name.contains("pointer") { 0x5678 } else { 0x5679 };
    for (i, v) in [3u16, 0x1234, product, 1].iter().enumerate() {
        buf[80 + i * 2..82 + i * 2].copy_from_slice(&v.to_ne_bytes());
    }
    for (axis, top) in absolute {
        let at = 92 + *axis as usize * 4;
        buf[at..at + 4].copy_from_slice(&top.to_ne_bytes());
    }
    if unsafe { libc::write(fd, buf.as_ptr().cast(), buf.len()) } < 0 {
        return Err(io::Error::last_os_error());
    }
    ioctl(fd, UI_DEV_CREATE, 0)?;
    pause(settle);
    Ok(dev)
}

impl Sink for Uinput {
    fn send(&mut self, events: &[Event]) -> io::Result<()> {
        let mut buf = Vec::with_capacity((events.len() + 1) * 24);
        for (kind, code, value) in events.iter().copied().chain(std::iter::once((EV_SYN, 0, 0))) {
            buf.extend_from_slice(&[0u8; 16]); // the timestamp: the kernel fills it in
            buf.extend_from_slice(&kind.to_ne_bytes());
            buf.extend_from_slice(&code.to_ne_bytes());
            buf.extend_from_slice(&value.to_ne_bytes());
        }
        if unsafe { libc::write(self.fd, buf.as_ptr().cast(), buf.len()) } < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
    }
}

impl Drop for Uinput {
    fn drop(&mut self) {
        unsafe {
            libc::ioctl(self.fd, UI_DEV_DESTROY);
            libc::close(self.fd);
        }
    }
}

pub fn pointer_device(settle: f64) -> io::Result<Uinput> {
    create_device("ghosthand-pointer", &[BTN_LEFT, BTN_RIGHT, BTN_MIDDLE], &[REL_WHEEL, REL_HWHEEL, REL_WHEEL_HI_RES, REL_HWHEEL_HI_RES], &[(ABS_X, ABS_MAX), (ABS_Y, ABS_MAX)], settle)
}

pub fn keyboard_device(settle: f64) -> io::Result<Uinput> {
    create_device("ghosthand-keyboard", &crate::keys::all_keys(), &[], &[], settle)
}

/// Collects what was sent, for tests.
#[cfg(test)]
#[derive(Default, Clone)]
pub struct Recorder(pub std::rc::Rc<std::cell::RefCell<Vec<Event>>>);

#[cfg(test)]
impl Sink for Recorder {
    fn send(&mut self, events: &[Event]) -> io::Result<()> {
        self.0.borrow_mut().extend_from_slice(events);
        Ok(())
    }
}
