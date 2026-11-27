"""The keyboard layout: which key (and Shift or AltGr) makes each character, so `type` is right on QWERTY, AZERTY, Dvorak, German...

Built from the real layout with libxkbcommon (through ctypes, so no Python dependency). If libxkbcommon isn't there, or the layout can't be
read, a built-in US layout is used. Which layout: `keyboard_layout`, `keyboard_variant` and `keyboard_options` in the config, else the
environment (XKB_DEFAULT_LAYOUT, ...), else setxkbmap or localectl, else "us". Only the first layout of a list ("us,de") is used."""
import ctypes
import ctypes.util
import os
import re
import shutil
import subprocess

from .config import load

SHIFT, ALTGR = 1, 2  # the modifiers a character may need, as bits
KEYPAD = {55, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 96, 98, 179, 180}


class Layout:
    """`table` maps a character to (evdev code, modifier bits). `name` says where it came from."""

    def __init__(self, table, name):
        self.table, self.name = table, name

    def find(self, ch):
        """(code, mods) for one character, or None when this layout can't type it."""
        return self.table.get(ch)

    def missing(self, text):
        """The characters of `text` this layout can't type (newline and tab are always fine)."""
        return sorted({c for c in text if c not in "\n\t" and c not in self.table})


_US_ROWS = [
    (2, "1234567890-=", "!@#$%^&*()_+"), (16, "qwertyuiop[]", "QWERTYUIOP{}"), (30, "asdfghjkl;'", 'ASDFGHJKL:"'),
    (43, "\\", "|"), (44, "zxcvbnm,./", "ZXCVBNM<>?"),
]


def us_layout():
    table = {" ": (57, 0), "\n": (28, 0), "\t": (15, 0), "`": (41, 0), "~": (41, SHIFT)}
    for first, plain, shifted in _US_ROWS:
        for i, (a, b) in enumerate(zip(plain, shifted)):
            table[a], table[b] = (first + i, 0), (first + i, SHIFT)
    return Layout(table, "us (built in)")


def _xkb(layout, variant="", options="", model="pc105", rules="evdev"):
    """The layout read by libxkbcommon, or None."""
    path = ctypes.util.find_library("xkbcommon") or "libxkbcommon.so.0"
    try:
        lib = ctypes.CDLL(path)
    except OSError:
        return None

    class Names(ctypes.Structure):
        _fields_ = [("rules", ctypes.c_char_p), ("model", ctypes.c_char_p), ("layout", ctypes.c_char_p), ("variant", ctypes.c_char_p), ("options", ctypes.c_char_p)]

    lib.xkb_context_new.restype = ctypes.c_void_p
    lib.xkb_context_new.argtypes = [ctypes.c_int]
    lib.xkb_keymap_new_from_names.restype = ctypes.c_void_p
    lib.xkb_keymap_new_from_names.argtypes = [ctypes.c_void_p, ctypes.POINTER(Names), ctypes.c_int]
    lib.xkb_keymap_min_keycode.restype = lib.xkb_keymap_max_keycode.restype = ctypes.c_uint32
    lib.xkb_keymap_min_keycode.argtypes = lib.xkb_keymap_max_keycode.argtypes = [ctypes.c_void_p]
    lib.xkb_keymap_num_levels_for_key.restype = ctypes.c_uint32
    lib.xkb_keymap_num_levels_for_key.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32]
    lib.xkb_keymap_key_get_syms_by_level.restype = ctypes.c_int
    lib.xkb_keymap_key_get_syms_by_level.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.POINTER(ctypes.POINTER(ctypes.c_uint32))]
    lib.xkb_keysym_to_utf32.restype = ctypes.c_uint32
    lib.xkb_keysym_to_utf32.argtypes = [ctypes.c_uint32]
    lib.xkb_keymap_mod_get_index.restype = ctypes.c_uint32
    lib.xkb_keymap_mod_get_index.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    lib.xkb_keymap_key_get_mods_for_level.restype = ctypes.c_size_t
    lib.xkb_keymap_key_get_mods_for_level.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.POINTER(ctypes.c_uint32), ctypes.c_size_t]
    lib.xkb_keymap_unref.argtypes = [ctypes.c_void_p]
    lib.xkb_context_unref.argtypes = [ctypes.c_void_p]

    ctx = lib.xkb_context_new(0)
    if not ctx:
        return None
    names = Names(rules.encode(), model.encode(), layout.split(",")[0].encode(), (variant.split(",")[0]).encode(), options.encode())
    keymap = lib.xkb_keymap_new_from_names(ctx, ctypes.byref(names), 0)
    if not keymap:
        lib.xkb_context_unref(ctx)
        return None
    try:
        shift, altgr = lib.xkb_keymap_mod_get_index(keymap, b"Shift"), lib.xkb_keymap_mod_get_index(keymap, b"Mod5")
        bits = {1 << shift: SHIFT} if shift != 0xFFFFFFFF else {}
        if altgr != 0xFFFFFFFF:
            bits[1 << altgr] = ALTGR
        table, ranks = {}, {}
        for key in range(lib.xkb_keymap_min_keycode(keymap), lib.xkb_keymap_max_keycode(keymap) + 1):
            code = key - 8  # xkb keycodes are evdev + 8
            if code < 1 or code > 247 or code in KEYPAD:  # not a real key, or the keypad (whose digits depend on NumLock)
                continue
            for level in range(lib.xkb_keymap_num_levels_for_key(keymap, key, 0)):
                syms = ctypes.POINTER(ctypes.c_uint32)()
                if lib.xkb_keymap_key_get_syms_by_level(keymap, key, 0, level, ctypes.byref(syms)) < 1:
                    continue
                cp = lib.xkb_keysym_to_utf32(syms[0])
                if cp < 32 and cp not in (9, 10):
                    continue
                masks = (ctypes.c_uint32 * 4)()
                found = None
                for i in range(min(4, lib.xkb_keymap_key_get_mods_for_level(keymap, key, 0, level, masks, 4))):
                    if all((1 << b) & sum(bits) or not (masks[i] >> b) & 1 for b in range(32)):  # only Shift and AltGr: nothing else held
                        found = sum(v for m, v in bits.items() if masks[i] & m)
                        break
                if found is None:
                    continue
                ch = "\n" if cp == 10 else chr(cp)
                rank = (code > 83, bin(found).count("1"))  # the main block first, then the fewest modifiers
                if ch not in table or rank < ranks[ch]:
                    table[ch], ranks[ch] = (code, found), rank
        table.update({"\n": (28, 0), "\t": (15, 0)})
        return Layout(table, f"{layout}{'(' + variant + ')' if variant else ''}")
    finally:
        lib.xkb_keymap_unref(keymap)
        lib.xkb_context_unref(ctx)


def detect():
    """(layout, variant, options) the person uses: config, then environment, then setxkbmap or localectl, then us."""
    cfg = load()
    if cfg.get("keyboard_layout"):
        return cfg["keyboard_layout"], cfg.get("keyboard_variant") or "", cfg.get("keyboard_options") or ""
    env = os.environ
    if env.get("XKB_DEFAULT_LAYOUT"):
        return env["XKB_DEFAULT_LAYOUT"], env.get("XKB_DEFAULT_VARIANT", ""), env.get("XKB_DEFAULT_OPTIONS", "")
    for argv, rx in ((["setxkbmap", "-query"], r"^layout:\s*(\S+)"), (["localectl", "status"], r"X11 Layout:\s*(\S+)")):
        if shutil.which(argv[0]):
            try:
                out = subprocess.run(argv, capture_output=True, text=True, timeout=3).stdout
            except (OSError, subprocess.SubprocessError):
                continue
            m = re.search(rx, out, re.M)
            if m:
                variant = re.search(r"^variant:\s*(\S+)", out, re.M) or re.search(r"X11 Variant:\s*(\S+)", out)
                return m.group(1), variant.group(1) if variant else "", ""
    return "us", "", ""


_cache = {}


def current():
    """The layout in use (cached for the process)."""
    if "layout" not in _cache:
        layout, variant, options = detect()
        _cache["layout"] = _xkb(layout, variant, options) or us_layout()
    return _cache["layout"]
