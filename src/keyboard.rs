//! The keyboard: `key`, `keydown`, `keyup`, `type` and `paste`, as key presses on a virtual keyboard.
//!
//! It sends key codes, like any keyboard, so the compositor's own layout decides what comes out. Characters are turned into key presses with the
//! layout (layout.rs), which is why `type` is right on QWERTY, AZERTY and Dvorak. Keys go to whatever has focus: click first.
use std::io::Write;
use std::process::{Command, Stdio};

use crate::keys;
use crate::layout::{self, Layout};
use crate::uinput::{Sink, EV_KEY};
use crate::util::{output_of, pause, which, Check, Stop};

const KEY_LEFTCTRL: u16 = 29;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_RIGHTALT: u16 = 100;
const KEY_V: u16 = 47;

/// `\n`, `\t` and `\\` in typed text mean a newline, a tab and a backslash (an action is one line, so they have to be written like this).
pub fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('\\', Some('n')) => { out.push('\n'); chars.next(); }
            ('\\', Some('t')) => { out.push('\t'); chars.next(); }
            ('\\', Some('\\')) => { out.push('\\'); chars.next(); }
            _ => out.push(c),
        }
    }
    out
}

/// One key of a combo as (code, extra modifier bits): a named key (Return, F5, ctrl) or a single character through the layout.
fn resolve(name: &str, lay: &Layout) -> Result<(u16, u8), String> {
    if let Some(code) = keys::named(name) {
        return Ok((code, 0));
    }
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if let Some(hit) = lay.find(c) {
            return Ok(hit);
        }
    }
    Err(format!("unknown key: '{name}' (names like Return, Escape, Tab, F5, Page_Down, ctrl, shift, alt, super; or one character)"))
}

/// `ctrl+shift+t` -> the codes in press order. Modifiers a character needs (Shift for "A") are added in front.
pub fn parse_combo(combo: &str, lay: &Layout) -> Result<Vec<u16>, String> {
    let parts: Vec<&str> = if combo.len() > 1 && combo.ends_with('+') {
        combo[..combo.len() - 1].split('+').filter(|p| !p.is_empty()).chain(std::iter::once("+")).collect() // "ctrl++" is ctrl and the plus key
    } else {
        let p: Vec<&str> = combo.split('+').filter(|p| !p.is_empty()).collect();
        if p.is_empty() && !combo.is_empty() { vec![combo] } else { p }
    };
    if parts.is_empty() {
        return Err("empty key".into());
    }
    let (mut codes, mut extra) = (Vec::new(), 0u8);
    for part in parts {
        let (code, mods) = resolve(part, lay)?;
        codes.push(code);
        extra |= mods;
    }
    let mut out: Vec<u16> = [(KEY_LEFTSHIFT, layout::SHIFT), (KEY_RIGHTALT, layout::ALTGR)].iter().filter(|(c, bit)| extra & bit != 0 && !codes.contains(c)).map(|(c, _)| *c).collect();
    out.extend(codes);
    Ok(out)
}

pub struct Keyboard {
    sink: Box<dyn Sink>,
    pub layout: Layout,
    delay: f64,
    down: Vec<u16>,
}

impl Keyboard {
    pub fn new(sink: Box<dyn Sink>, layout: Layout, delay: f64) -> Keyboard {
        Keyboard { sink, layout, delay, down: Vec::new() }
    }

    fn press(&mut self, code: u16) -> Result<(), Stop> {
        if !self.down.contains(&code) {
            self.down.push(code);
        }
        Ok(self.sink.send(&[(EV_KEY, code, 1)])?)
    }

    fn release(&mut self, code: u16) -> Result<(), Stop> {
        self.down.retain(|c| *c != code);
        Ok(self.sink.send(&[(EV_KEY, code, 0)])?)
    }

    /// Press every code in order, then release them in reverse: a chord.
    pub fn tap(&mut self, codes: &[u16], hold: f64) -> Result<(), Stop> {
        for c in codes {
            self.press(*c)?;
            pause(0.01);
        }
        pause(hold);
        for c in codes.iter().rev() {
            self.release(*c)?;
        }
        pause(self.delay);
        Ok(())
    }

    pub fn key(&mut self, codes: &[u16]) -> Result<(), Stop> {
        self.tap(codes, 0.04)
    }

    pub fn hold(&mut self, codes: &[u16], down: bool) -> Result<(), Stop> {
        for c in codes {
            if down { self.press(*c)?; } else { self.release(*c)?; }
        }
        Ok(())
    }

    /// Type text, one key press per character. Fails (before pressing anything) for characters this layout can't make.
    pub fn type_text(&mut self, text: &str, check: Check) -> Result<(), Stop> {
        let lost = self.layout.missing(text);
        if !lost.is_empty() {
            return Err(Stop::Failed(format!("can't type '{}' with the {} layout: use `paste`, or set keyboard_layout in the config", lost.iter().collect::<String>(), self.layout.name)));
        }
        for (i, ch) in text.chars().enumerate() {
            let (code, mods) = self.layout.find(ch).ok_or_else(|| Stop::Failed(format!("can't type '{ch}'")))?;
            let mut codes: Vec<u16> = [(KEY_LEFTSHIFT, layout::SHIFT), (KEY_RIGHTALT, layout::ALTGR)].iter().filter(|(_, b)| mods & b != 0).map(|(c, _)| *c).collect();
            codes.push(code);
            self.tap(&codes, 0.006)?;
            if i % 24 == 23 {
                check()?; // a revoke stops typing within a couple of dozen characters
            }
        }
        pause(0.05);
        Ok(())
    }

    /// Let go of everything still held, then let the device go.
    pub fn close(mut self) {
        for c in self.down.clone().into_iter().rev() {
            let _ = self.release(c);
        }
        pause(self.sink.flush_pause());
    }
}

/// (copy command, paste command) for the session's clipboard, or None.
pub fn clipboard_tools() -> Option<(Vec<&'static str>, Vec<&'static str>)> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && which("wl-copy") && which("wl-paste") {
        return Some((vec!["wl-copy"], vec!["wl-paste", "-n"]));
    }
    if which("xclip") {
        return Some((vec!["xclip", "-selection", "clipboard", "-i"], vec!["xclip", "-selection", "clipboard", "-o"]));
    }
    if which("xsel") {
        return Some((vec!["xsel", "-b", "-i"], vec!["xsel", "-b", "-o"]));
    }
    None
}

fn set_clipboard(copy: &[&str], text: &[u8]) -> Result<(), String> {
    let mut child = Command::new(copy[0]).args(&copy[1..]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
    child.stdin.take().ok_or("no stdin")?.write_all(text).map_err(|e| e.to_string())?;
    // wl-copy and xclip stay running to serve the clipboard: don't wait for them
    pause(0.1);
    let _ = child.try_wait();
    Ok(())
}

/// Put `text` on the clipboard, press Ctrl+V (Ctrl+Shift+V for terminals), then put back what was there. For text `type` can't make.
pub fn paste(kb: &mut Keyboard, text: &str, terminal: bool) -> Result<(), Stop> {
    let (copy, read) = clipboard_tools().ok_or_else(|| Stop::Failed("paste needs wl-clipboard (Wayland) or xclip or xsel (X11)".into()))?;
    let before = output_of(read[0], &read[1..]);
    set_clipboard(&copy, text.as_bytes())?;
    pause(0.15);
    let mut codes = vec![KEY_LEFTCTRL];
    if terminal {
        codes.push(KEY_LEFTSHIFT);
    }
    codes.push(KEY_V);
    kb.tap(&codes, 0.04)?;
    pause(0.4); // let the app read it before the old clipboard comes back
    if let Some(old) = before {
        set_clipboard(&copy, old.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uinput::Recorder;
    use std::sync::atomic::Ordering;

    fn kb() -> (Keyboard, Recorder) {
        crate::util::FAST.store(true, Ordering::Relaxed);
        let rec = Recorder::default();
        (Keyboard::new(Box::new(rec.clone()), layout::us(), 0.0), rec)
    }

    fn events(rec: &Recorder) -> Vec<(u16, i32)> {
        rec.0.borrow().iter().map(|e| (e.1, e.2)).collect()
    }

    #[test]
    fn a_chord_is_modifiers_then_the_key() {
        let us = layout::us();
        assert_eq!(parse_combo("ctrl+shift+t", &us).unwrap(), vec![29, 42, 20]);
        assert_eq!(parse_combo("alt+F4", &us).unwrap(), vec![56, 62]);
        assert_eq!(parse_combo("super+Left", &us).unwrap(), vec![125, 105]);
        assert_eq!(parse_combo("Return", &us).unwrap(), vec![28]);
    }

    #[test]
    fn a_character_that_needs_shift_brings_it() {
        let us = layout::us();
        assert_eq!(parse_combo("A", &us).unwrap(), vec![42, 30]);
        assert_eq!(parse_combo("ctrl+A", &us).unwrap(), vec![42, 29, 30]);
        assert_eq!(parse_combo("@", &us).unwrap(), vec![42, 3]);
        assert_eq!(parse_combo("ctrl++", &us).unwrap(), vec![42, 29, 13], "ctrl and the plus key");
    }

    #[test]
    fn unknown_keys_are_errors() {
        let us = layout::us();
        for bad in ["ctrl+nonsense", "", "ctrl+Hyper"] {
            assert!(parse_combo(bad, &us).is_err(), "{bad}");
        }
    }

    #[test]
    fn single_characters_follow_the_layout() {
        let Some(de) = layout::from_xkb("de", "", "") else { return };
        assert_eq!(parse_combo("z", &de).unwrap(), vec![21]);
        assert_eq!(parse_combo("ctrl+z", &de).unwrap(), vec![29, 21]);
    }

    #[test]
    fn text_becomes_key_presses_with_shift() {
        let (mut k, rec) = kb();
        k.type_text("Hi!\n", &mut || Ok(())).unwrap();
        assert_eq!(events(&rec), vec![(42, 1), (35, 1), (35, 0), (42, 0), (23, 1), (23, 0), (42, 1), (2, 1), (2, 0), (42, 0), (28, 1), (28, 0)]);
    }

    #[test]
    fn unmappable_text_is_refused_before_a_single_key_is_pressed() {
        let (mut k, rec) = kb();
        let err = k.type_text("fine \u{e9}", &mut || Ok(())).unwrap_err();
        assert!(matches!(err, Stop::Failed(m) if m.contains("paste")));
        assert!(events(&rec).is_empty());
    }

    #[test]
    fn a_chord_releases_in_reverse() {
        let (mut k, rec) = kb();
        k.key(&[29, 42, 20]).unwrap();
        assert_eq!(events(&rec), vec![(29, 1), (42, 1), (20, 1), (20, 0), (42, 0), (29, 0)]);
    }

    #[test]
    fn held_keys_are_released_on_close_in_reverse() {
        let (mut k, rec) = kb();
        k.hold(&[42], true).unwrap();
        k.hold(&[29], true).unwrap();
        k.close();
        assert_eq!(events(&rec), vec![(42, 1), (29, 1), (29, 0), (42, 0)]);
    }

    #[test]
    fn a_revoke_is_noticed_while_typing() {
        let (mut k, rec) = kb();
        let mut calls = 0;
        let result = k.type_text(&"a".repeat(100), &mut || { calls += 1; Err(Stop::Revoked) });
        assert_eq!(result, Err(Stop::Revoked));
        assert_eq!(calls, 1);
        assert!(events(&rec).len() <= 50, "it stopped within a couple of dozen characters");
    }

    #[test]
    fn escapes() {
        assert_eq!(unescape(r"one\ntwo\tthree\\four\q"), "one\ntwo\tthree\\four\\q");
        assert_eq!(unescape("trailing\\"), "trailing\\");
    }
}
