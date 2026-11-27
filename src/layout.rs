//! The keyboard layout: which key (and Shift or AltGr) makes each character, so `type` is right on QWERTY, AZERTY, Dvorak, German...
//!
//! Built from the real layout with libxkbcommon, loaded at run time (dlopen), so it needs no headers to build and the binary still works
//! without it: then a built-in US layout is used. Which layout: `keyboard_layout`, `keyboard_variant` and `keyboard_options` in the config,
//! else the environment (XKB_DEFAULT_LAYOUT...), else setxkbmap or localectl, else "us". Only the first of a list ("us,de") is used.
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CString};
use std::sync::OnceLock;

use crate::config::Config;
use crate::util::output_of;

pub const SHIFT: u8 = 1;
pub const ALTGR: u8 = 2;
const KEYPAD: &[u16] = &[55, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 96, 98, 179, 180];

#[derive(Debug, Clone)]
pub struct Layout {
    table: HashMap<char, (u16, u8)>,
    /// Where it came from, for messages: "us (built in)", "de", "us(dvorak)".
    pub name: String,
}

impl Layout {
    /// (evdev code, modifier bits) for one character, or None when this layout can't type it.
    pub fn find(&self, ch: char) -> Option<(u16, u8)> {
        self.table.get(&ch).copied()
    }

    /// The characters of `text` this layout can't type (newline and tab are always fine), sorted, once each.
    pub fn missing(&self, text: &str) -> Vec<char> {
        let mut lost: Vec<char> = text.chars().filter(|c| *c != '\n' && *c != '\t' && !self.table.contains_key(c)).collect();
        lost.sort_unstable();
        lost.dedup();
        lost
    }

    #[cfg(test)]
    pub fn entries(&self) -> &HashMap<char, (u16, u8)> { &self.table }
}

pub fn us() -> Layout {
    let mut table: HashMap<char, (u16, u8)> = HashMap::new();
    table.insert(' ', (57, 0));
    table.insert('\n', (28, 0));
    table.insert('\t', (15, 0));
    table.insert('`', (41, 0));
    table.insert('~', (41, SHIFT));
    let rows: [(u16, &str, &str); 5] = [
        (2, "1234567890-=", "!@#$%^&*()_+"),
        (16, "qwertyuiop[]", "QWERTYUIOP{}"),
        (30, "asdfghjkl;'", "ASDFGHJKL:\""),
        (43, "\\", "|"),
        (44, "zxcvbnm,./", "ZXCVBNM<>?"),
    ];
    for (first, plain, shifted) in rows {
        for (i, (a, b)) in plain.chars().zip(shifted.chars()).enumerate() {
            table.insert(a, (first + i as u16, 0));
            table.insert(b, (first + i as u16, SHIFT));
        }
    }
    Layout { table, name: "us (built in)".into() }
}

#[repr(C)]
struct Names {
    rules: *const c_char,
    model: *const c_char,
    layout: *const c_char,
    variant: *const c_char,
    options: *const c_char,
}

/// The layout read by libxkbcommon, or None (no library, or no such layout).
pub fn from_xkb(layout: &str, variant: &str, options: &str) -> Option<Layout> {
    let lib = ["libxkbcommon.so.0", "libxkbcommon.so"].iter().find_map(|n| unsafe { libloading::Library::new(n).ok() })?;
    unsafe {
        macro_rules! sym {
            ($name:literal, $ty:ty) => {
                *lib.get::<$ty>($name).ok()?
            };
        }
        let context_new = sym!(b"xkb_context_new\0", unsafe extern "C" fn(c_int) -> *mut c_void);
        let set_log_level = sym!(b"xkb_context_set_log_level\0", unsafe extern "C" fn(*mut c_void, c_int));
        let context_unref = sym!(b"xkb_context_unref\0", unsafe extern "C" fn(*mut c_void));
        let keymap_new = sym!(b"xkb_keymap_new_from_names\0", unsafe extern "C" fn(*mut c_void, *const Names, c_int) -> *mut c_void);
        let keymap_unref = sym!(b"xkb_keymap_unref\0", unsafe extern "C" fn(*mut c_void));
        let min_key = sym!(b"xkb_keymap_min_keycode\0", unsafe extern "C" fn(*mut c_void) -> u32);
        let max_key = sym!(b"xkb_keymap_max_keycode\0", unsafe extern "C" fn(*mut c_void) -> u32);
        let num_levels = sym!(b"xkb_keymap_num_levels_for_key\0", unsafe extern "C" fn(*mut c_void, u32, u32) -> u32);
        let syms_by_level = sym!(b"xkb_keymap_key_get_syms_by_level\0", unsafe extern "C" fn(*mut c_void, u32, u32, u32, *mut *const u32) -> c_int);
        let to_utf32 = sym!(b"xkb_keysym_to_utf32\0", unsafe extern "C" fn(u32) -> u32);
        let mod_index = sym!(b"xkb_keymap_mod_get_index\0", unsafe extern "C" fn(*mut c_void, *const c_char) -> u32);
        let mods_for_level = sym!(b"xkb_keymap_key_get_mods_for_level\0", unsafe extern "C" fn(*mut c_void, u32, u32, u32, *mut u32, usize) -> usize);

        let ctx = context_new(0);
        if ctx.is_null() {
            return None;
        }
        set_log_level(ctx, 10); // only critical errors: a layout that doesn't exist is handled, not shouted about
        let c = |s: &str| CString::new(s).ok();
        let (rules, model, lay, var, opt) = (c("evdev")?, c("pc105")?, c(layout.split(',').next().unwrap_or("us"))?, c(variant.split(',').next().unwrap_or(""))?, c(options)?);
        let names = Names { rules: rules.as_ptr(), model: model.as_ptr(), layout: lay.as_ptr(), variant: var.as_ptr(), options: opt.as_ptr() };
        let keymap = keymap_new(ctx, &names, 0);
        if keymap.is_null() {
            context_unref(ctx);
            return None;
        }
        let shift = mod_index(keymap, b"Shift\0".as_ptr().cast());
        let altgr = mod_index(keymap, b"Mod5\0".as_ptr().cast());
        let mut bits: Vec<(u32, u8)> = Vec::new();
        if shift != u32::MAX {
            bits.push((1 << shift, SHIFT));
        }
        if altgr != u32::MAX {
            bits.push((1 << altgr, ALTGR));
        }
        let allowed: u32 = bits.iter().fold(0, |a, b| a | b.0);
        let mut table: HashMap<char, (u16, u8)> = HashMap::new();
        let mut ranks: HashMap<char, (bool, u32)> = HashMap::new();
        for key in min_key(keymap)..=max_key(keymap) {
            let code = key as i64 - 8; // xkb keycodes are evdev + 8
            if !(1..=247).contains(&code) || KEYPAD.contains(&(code as u16)) {
                continue; // not a real key, or the keypad (whose digits depend on NumLock)
            }
            for level in 0..num_levels(keymap, key, 0) {
                let mut syms: *const u32 = std::ptr::null();
                if syms_by_level(keymap, key, 0, level, &mut syms) < 1 || syms.is_null() {
                    continue;
                }
                let cp = to_utf32(*syms);
                if cp < 32 && cp != 9 && cp != 10 {
                    continue;
                }
                let mut masks = [0u32; 4];
                let count = mods_for_level(keymap, key, 0, level, masks.as_mut_ptr(), 4).min(4);
                let found = masks[..count].iter().find(|m| **m & !allowed == 0).map(|m| bits.iter().filter(|(b, _)| m & b != 0).fold(0u8, |a, (_, v)| a | v));
                let (Some(found), Some(ch)) = (found, char::from_u32(cp)) else { continue };
                let rank = (code > 83, found.count_ones());
                if ranks.get(&ch).map_or(true, |r| rank < *r) {
                    table.insert(ch, (code as u16, found));
                    ranks.insert(ch, rank);
                }
            }
        }
        table.insert('\n', (28, 0));
        table.insert('\t', (15, 0));
        keymap_unref(keymap);
        context_unref(ctx);
        let name = if variant.is_empty() { layout.to_string() } else { format!("{layout}({variant})") };
        Some(Layout { table, name })
    }
}

/// (layout, variant, options) the person uses: the config, then the environment, then setxkbmap or localectl, then us.
pub fn detect(cfg: &Config) -> (String, String, String) {
    if let Some(l) = cfg.string("keyboard_layout") {
        return (l, cfg.string("keyboard_variant").unwrap_or_default(), cfg.string("keyboard_options").unwrap_or_default());
    }
    if let Ok(l) = std::env::var("XKB_DEFAULT_LAYOUT") {
        if !l.is_empty() {
            return (l, std::env::var("XKB_DEFAULT_VARIANT").unwrap_or_default(), std::env::var("XKB_DEFAULT_OPTIONS").unwrap_or_default());
        }
    }
    let field = |text: &str, key: &str| text.lines().find_map(|l| l.trim().strip_prefix(key).map(|v| v.trim().to_string())).filter(|v| !v.is_empty());
    if let Some(out) = output_of("setxkbmap", &["-query"]) {
        if let Some(l) = field(&out, "layout:") {
            return (l, field(&out, "variant:").unwrap_or_default(), String::new());
        }
    }
    if let Some(out) = output_of("localectl", &["status"]) {
        if let Some(l) = field(&out, "X11 Layout:") {
            return (l, field(&out, "X11 Variant:").unwrap_or_default(), String::new());
        }
    }
    ("us".into(), String::new(), String::new())
}

/// The layout in use (found once per process).
pub fn current() -> &'static Layout {
    static LAYOUT: OnceLock<Layout> = OnceLock::new();
    LAYOUT.get_or_init(|| {
        let (l, v, o) = detect(&Config::load());
        from_xkb(&l, &v, &o).unwrap_or_else(us)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_us_layout_is_complete() {
        let us = us();
        for ch in "abcxyzABCXYZ0123456789 `~!@#$%^&*()-_=+[]{}\\|;:'\",<.>/?\n\t".chars() {
            assert!(us.find(ch).is_some(), "{ch:?}");
        }
        assert_eq!(us.find('A'), Some((30, SHIFT)));
        assert_eq!(us.find('a'), Some((30, 0)));
    }

    #[test]
    fn missing_reports_what_it_cannot_type() {
        assert_eq!(us().missing("hello \u{e9}\u{20ac}\u{e9}\n"), vec!['\u{e9}', '\u{20ac}']);
        assert!(us().missing("plain text\twith tabs\n").is_empty());
    }

    #[test]
    fn xkb_agrees_with_the_built_in_table_for_us() {
        let Some(real) = from_xkb("us", "", "") else { return };
        for (ch, hit) in us().entries() {
            assert_eq!(real.find(*ch), Some(*hit), "{ch:?}");
        }
    }

    #[test]
    fn xkb_knows_other_layouts_altgr_and_variants() {
        let Some(de) = from_xkb("de", "", "") else { return };
        assert_eq!(de.find('z'), Some((21, 0)), "z is where y is on QWERTY");
        assert_eq!(de.find('@'), Some((16, ALTGR)));
        assert_eq!(de.find('\u{20ac}'), Some((18, ALTGR)));
        assert!(de.find('\u{e4}').is_some());
        assert_eq!(from_xkb("fr", "", "").unwrap().find('a'), Some((16, 0)), "a is where q is on AZERTY");
        assert_eq!(from_xkb("us", "dvorak", "").unwrap().find('s'), Some((39, 0)));
    }

    #[test]
    fn a_layout_that_does_not_exist_is_none() {
        assert!(from_xkb("no-such-layout", "", "").is_none());
    }

    #[test]
    fn the_layout_comes_from_the_config_first() {
        let cfg = Config::from_json(r#"{"keyboard_layout":"de","keyboard_variant":"nodeadkeys"}"#);
        assert_eq!(detect(&cfg), ("de".into(), "nodeadkeys".into(), String::new()));
    }
}
