"""Key names, as Linux evdev codes. Names are case-insensitive and follow xdotool/xkb style: Return, Escape, ctrl, super, F5, Page_Down, KP_Enter...

A single character (a, A, 7, /, é) is not in this table: it is looked up in the keyboard layout (layout.py), so `key ctrl+z` presses the
key that makes "z" on YOUR layout, QWERTY, AZERTY or Dvorak."""

CODES = {
    "escape": 1, "esc": 1, "backspace": 14, "tab": 15, "return": 28, "enter": 28, "space": 57, "capslock": 58, "caps_lock": 58,
    "numlock": 69, "num_lock": 69, "scrolllock": 70, "scroll_lock": 70, "print": 99, "sysrq": 99, "pause": 119, "menu": 127, "compose": 127,
    "home": 102, "up": 103, "pageup": 104, "page_up": 104, "prior": 104, "left": 105, "right": 106, "end": 107, "down": 108,
    "pagedown": 109, "page_down": 109, "next": 109, "insert": 110, "ins": 110, "delete": 111, "del": 111,
    "kp_multiply": 55, "kp_7": 71, "kp_8": 72, "kp_9": 73, "kp_subtract": 74, "kp_4": 75, "kp_5": 76, "kp_6": 77, "kp_add": 78,
    "kp_1": 79, "kp_2": 80, "kp_3": 81, "kp_0": 82, "kp_decimal": 83, "kp_enter": 96, "kp_divide": 98,
    "mute": 113, "volumedown": 114, "volumeup": 115, "xf86audiomute": 113, "xf86audiolowervolume": 114, "xf86audioraisevolume": 115,
    "xf86audionext": 163, "xf86audioplay": 164, "xf86audioprev": 165, "xf86audiostop": 166,
    "xf86monbrightnessdown": 224, "xf86monbrightnessup": 225,
}
# F1 to F10, F11, F12, F13 to F24
CODES.update({f"f{i}": 58 + i for i in range(1, 11)})
CODES.update({"f11": 87, "f12": 88})
CODES.update({f"f{i}": 170 + i for i in range(13, 25)})

MODIFIERS = {
    "ctrl": 29, "control": 29, "ctrl_l": 29, "control_l": 29, "ctrl_r": 97, "control_r": 97,
    "shift": 42, "shift_l": 42, "shift_r": 54,
    "alt": 56, "alt_l": 56, "alt_r": 100, "altgr": 100, "iso_level3_shift": 100,
    "super": 125, "super_l": 125, "super_r": 126, "meta": 125, "win": 125, "mod": 125, "mod4": 125, "logo": 125,
}
CODES.update(MODIFIERS)

# Every code the virtual keyboard may send: the main block, keypad, F keys, navigation and media keys.
ALL_KEYS = tuple(sorted(set(CODES.values()) | set(range(1, 84)) | {86, 87, 88, 96, 97, 98, 99, 100, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 125, 126, 127}))


def named(name):
    """The evdev code for a key NAME (not a single character), or None."""
    return CODES.get(name.strip().lower())


def is_modifier(name):
    return name.strip().lower() in MODIFIERS
