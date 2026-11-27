//! The pointer: coordinates are screenshot pixels of the output; `announce(x, y)` gets logical pixels after every step (for the cursor tag).
use std::collections::BTreeSet;
use std::io;

use crate::geometry::Info;
use crate::paths;
use crate::uinput::*;
use crate::util::{now, pause};

/// A remembered position older than this is not trusted: the real pointer has moved since.
pub const LAST_MAX_AGE: f64 = 600.0;

pub fn read_last() -> Option<(f64, f64)> {
    read_last_at(&paths::last_position_file(), now())
}

pub fn read_last_at(path: &std::path::Path, at: f64) -> Option<(f64, f64)> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    let (x, y, t) = (v.get("x")?.as_f64()?, v.get("y")?.as_f64()?, v.get("t")?.as_f64()?);
    (at - t <= LAST_MAX_AGE).then_some((x, y))
}

pub struct Pointer {
    sink: Box<dyn Sink>,
    w: f64,
    h: f64,
    scale: f64,
    pub x: f64,
    pub y: f64,
    pub known: bool,
    down: BTreeSet<u16>,
    announce: Option<Box<dyn FnMut(f64, f64)>>,
}

impl Pointer {
    pub fn new(sink: Box<dyn Sink>, info: &Info, announce: Option<Box<dyn FnMut(f64, f64)>>) -> Pointer {
        let last = read_last();
        Pointer {
            sink, w: info.width as f64, h: info.height as f64, scale: info.scale,
            x: last.map_or(0.0, |l| l.0), y: last.map_or(0.0, |l| l.1), known: last.is_some(), down: BTreeSet::new(), announce,
        }
    }

    pub fn put(&mut self, x: f64, y: f64) -> io::Result<()> {
        self.x = x;
        self.y = y;
        self.known = true;
        let axis = |v: f64, max: f64| ((v / max).clamp(0.0, 1.0) * ABS_MAX as f64) as i32;
        self.sink.send(&[(EV_ABS, ABS_X, axis(x, self.w)), (EV_ABS, ABS_Y, axis(y, self.h))])?;
        if let Some(a) = self.announce.as_mut() {
            a(x / self.scale, y / self.scale);
        }
        Ok(())
    }

    /// Glide in small steps so hover follows. With no idea where the pointer is, jump: gliding from a corner would sweep across the screen.
    pub fn glide(&mut self, x: f64, y: f64) -> io::Result<()> {
        if !self.known {
            return self.put(x, y);
        }
        let (x0, y0, steps) = (self.x, self.y, 14);
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            self.put(x0 + (x - x0) * t, y0 + (y - y0) * t)?;
            pause(0.012);
        }
        Ok(())
    }

    pub fn button(&mut self, code: u16, is_down: bool) -> io::Result<()> {
        if is_down { self.down.insert(code); } else { self.down.remove(&code); }
        self.sink.send(&[(EV_KEY, code, is_down as i32)])
    }

    pub fn click(&mut self, code: u16, times: u32) -> io::Result<()> {
        for i in 0..times {
            if i > 0 {
                pause(0.08);
            }
            self.button(code, true)?;
            pause(0.12);
            self.button(code, false)?;
        }
        Ok(())
    }

    /// Positive scrolls the content down (or right). One notch is 120 hi-res units.
    pub fn wheel(&mut self, notches: f64, horizontal: bool) -> io::Result<()> {
        let count = notches.round().abs() as i32;
        let step = if notches > 0.0 { 1 } else { -1 };
        for _ in 0..count {
            let (hi, lo, sign) = if horizontal { (REL_HWHEEL_HI_RES, REL_HWHEEL, step) } else { (REL_WHEEL_HI_RES, REL_WHEEL, -step) };
            self.sink.send(&[(EV_REL, hi, 120 * sign), (EV_REL, lo, sign)])?;
            pause(0.05);
        }
        Ok(())
    }

    /// Release anything held, remember where the pointer is, and let the device go.
    pub fn close(mut self) {
        for code in self.down.clone() {
            let _ = self.button(code, false);
        }
        pause(self.sink.flush_pause());
        let _ = paths::ensure_runtime_dir();
        let saved = serde_json::json!({"x": self.x, "y": self.y, "t": now()});
        let _ = std::fs::write(paths::last_position_file(), saved.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> Info { Info { name: "X".into(), width: 1000, height: 500, scale: 2.0 } }

    #[allow(clippy::type_complexity)]
    fn pointer() -> (Pointer, Recorder, std::rc::Rc<std::cell::RefCell<Vec<(f64, f64)>>>) {
        crate::util::FAST.store(true, std::sync::atomic::Ordering::Relaxed);
        let rec = Recorder::default();
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let s2 = seen.clone();
        let mut p = Pointer::new(Box::new(rec.clone()), &info(), Some(Box::new(move |x, y| s2.borrow_mut().push((x, y)))));
        p.known = false;
        (p, rec, seen)
    }

    #[test]
    fn put_scales_to_the_device_range_and_announces_logical_pixels() {
        let (mut p, rec, seen) = pointer();
        p.put(500.0, 250.0).unwrap();
        assert_eq!(*rec.0.borrow(), vec![(EV_ABS, ABS_X, 16383), (EV_ABS, ABS_Y, 16383)]);
        assert_eq!(*seen.borrow(), vec![(250.0, 125.0)]);
        p.put(5000.0, -10.0).unwrap();
        assert_eq!(rec.0.borrow()[2], (EV_ABS, ABS_X, 32767), "clamped");
        assert_eq!(rec.0.borrow()[3], (EV_ABS, ABS_Y, 0));
    }

    #[test]
    fn glide_without_a_known_position_jumps() {
        let (mut p, rec, _) = pointer();
        p.glide(10.0, 10.0).unwrap();
        assert_eq!(rec.0.borrow().len(), 2);
        p.glide(110.0, 10.0).unwrap();
        assert_eq!(rec.0.borrow().len(), 2 + 14 * 2, "then it glides in 14 steps");
    }

    #[test]
    fn clicks_and_double_clicks() {
        let (mut p, rec, _) = pointer();
        p.click(BTN_LEFT, 2).unwrap();
        assert_eq!(*rec.0.borrow(), vec![(EV_KEY, BTN_LEFT, 1), (EV_KEY, BTN_LEFT, 0), (EV_KEY, BTN_LEFT, 1), (EV_KEY, BTN_LEFT, 0)]);
    }

    #[test]
    fn positive_scroll_moves_the_content_down() {
        let (mut p, rec, _) = pointer();
        p.wheel(2.0, false).unwrap();
        assert_eq!(rec.0.borrow()[0], (EV_REL, REL_WHEEL_HI_RES, -120));
        assert_eq!(rec.0.borrow()[1], (EV_REL, REL_WHEEL, -1));
        let n = rec.0.borrow().len();
        p.wheel(-1.0, true).unwrap();
        assert_eq!(rec.0.borrow()[n], (EV_REL, REL_HWHEEL_HI_RES, -120));
    }

    #[test]
    fn close_releases_held_buttons() {
        let (mut p, rec, _) = pointer();
        p.button(BTN_RIGHT, true).unwrap();
        crate::paths::use_temp_runtime_dir();
        p.close();
        assert_eq!(rec.0.borrow().last(), Some(&(EV_KEY, BTN_RIGHT, 0)));
    }

    #[test]
    fn a_remembered_position_is_trusted_only_while_fresh() {
        let path = std::env::temp_dir().join(format!("gh-last-{}.json", std::process::id()));
        std::fs::write(&path, r#"{"x": 10, "y": 20, "t": 1000}"#).unwrap();
        assert_eq!(read_last_at(&path, 1100.0), Some((10.0, 20.0)));
        assert_eq!(read_last_at(&path, 1000.0 + LAST_MAX_AGE + 1.0), None);
        assert_eq!(read_last_at(&path.with_extension("missing"), 0.0), None);
    }
}
