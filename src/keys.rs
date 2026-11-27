//! Key names, as Linux evdev codes. Names are case-insensitive and follow xdotool/xkb style: Return, Escape, ctrl, super, F5, Page_Down, KP_Enter...
//!
//! A single character (a, A, 7, /, é) is not here: it is looked up in the keyboard layout (layout.rs), so `key ctrl+z` presses the key that makes
//! "z" on YOUR layout, QWERTY, AZERTY or Dvorak.

pub const MODIFIERS: &[(&str, u16)] = &[
    ("ctrl", 29), ("control", 29), ("ctrl_l", 29), ("control_l", 29), ("ctrl_r", 97), ("control_r", 97),
    ("shift", 42), ("shift_l", 42), ("shift_r", 54),
    ("alt", 56), ("alt_l", 56), ("alt_r", 100), ("altgr", 100), ("iso_level3_shift", 100),
    ("super", 125), ("super_l", 125), ("super_r", 126), ("meta", 125), ("win", 125), ("mod", 125), ("mod4", 125), ("logo", 125),
];

const NAMED: &[(&str, u16)] = &[
    ("escape", 1), ("esc", 1), ("backspace", 14), ("tab", 15), ("return", 28), ("enter", 28), ("space", 57), ("capslock", 58), ("caps_lock", 58),
    ("numlock", 69), ("num_lock", 69), ("scrolllock", 70), ("scroll_lock", 70), ("print", 99), ("sysrq", 99), ("pause", 119), ("menu", 127), ("compose", 127),
    ("home", 102), ("up", 103), ("pageup", 104), ("page_up", 104), ("prior", 104), ("left", 105), ("right", 106), ("end", 107), ("down", 108),
    ("pagedown", 109), ("page_down", 109), ("next", 109), ("insert", 110), ("ins", 110), ("delete", 111), ("del", 111),
    ("kp_multiply", 55), ("kp_7", 71), ("kp_8", 72), ("kp_9", 73), ("kp_subtract", 74), ("kp_4", 75), ("kp_5", 76), ("kp_6", 77), ("kp_add", 78),
    ("kp_1", 79), ("kp_2", 80), ("kp_3", 81), ("kp_0", 82), ("kp_decimal", 83), ("kp_enter", 96), ("kp_divide", 98),
    ("mute", 113), ("volumedown", 114), ("volumeup", 115), ("xf86audiomute", 113), ("xf86audiolowervolume", 114), ("xf86audioraisevolume", 115),
    ("xf86audionext", 163), ("xf86audioplay", 164), ("xf86audioprev", 165), ("xf86audiostop", 166),
    ("xf86monbrightnessdown", 224), ("xf86monbrightnessup", 225),
];

/// The evdev code for a key NAME (not a single character), or None.
pub fn named(name: &str) -> Option<u16> {
    let n = name.trim().to_ascii_lowercase();
    if let Some(num) = n.strip_prefix('f').and_then(|d| d.parse::<u16>().ok()) {
        return match num {
            1..=10 => Some(58 + num),
            11 => Some(87),
            12 => Some(88),
            13..=24 => Some(170 + num),
            _ => None,
        };
    }
    MODIFIERS.iter().chain(NAMED.iter()).find(|(k, _)| *k == n).map(|(_, c)| *c)
}

/// Every code the virtual keyboard may send: the main block, keypad, F keys, navigation and media keys.
pub fn all_keys() -> Vec<u16> {
    let mut keys: Vec<u16> = (1..=88).chain([96, 97, 98, 99, 100, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 113, 114, 115, 119, 125, 126, 127]).collect();
    keys.extend(183..=194);
    keys.extend([163, 164, 165, 166, 224, 225]);
    keys.sort_unstable();
    keys.dedup();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_keys_and_function_keys() {
        assert_eq!(named("Return"), Some(28));
        assert_eq!(named("ESC"), Some(1));
        assert_eq!(named("Page_Down"), Some(109));
        assert_eq!(named("F5"), Some(63));
        assert_eq!(named("f12"), Some(88));
        assert_eq!(named("F13"), Some(183));
        assert_eq!(named("F24"), Some(194));
        assert_eq!(named("XF86AudioPlay"), Some(164));
        assert_eq!(named("F25"), None);
        assert_eq!(named("nonsense"), None);
        assert_eq!(named("a"), None, "single characters go through the layout");
    }

    #[test]
    fn the_virtual_keyboard_can_send_every_key_we_name() {
        let all = all_keys();
        for (name, code) in MODIFIERS.iter().chain(NAMED.iter()) {
            assert!(all.contains(code), "{name}");
        }
        for n in 1..=24 {
            assert!(all.contains(&named(&format!("f{n}")).unwrap()), "f{n}");
        }
    }
}
