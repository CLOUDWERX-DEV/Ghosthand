"""A virtual keyboard made through /dev/uinput: `key`, `keydown`, `keyup`, `type` and `paste`.

It sends key codes, like any keyboard, so the compositor's own layout decides what comes out. Characters are turned into key presses with the
layout (layout.py), which is why `type` is right on QWERTY, AZERTY and Dvorak. Keys go to whatever has focus: click first."""
import os
import shutil
import struct
import subprocess
import time

from . import keys, layout
from .config import load
from .uinput import EV_KEY, EV_SYN, create_device, destroy_device

KEY_LEFTCTRL, KEY_LEFTSHIFT, KEY_RIGHTALT, KEY_V, KEY_LEFTMETA = 29, 42, 100, 47, 125


def unescape(text):
    r"""`\n`, `\t` and `\\` in typed text mean a newline, a tab and a backslash (an action is one line, so they have to be written like this)."""
    out, i = [], 0
    while i < len(text):
        if text[i] == "\\" and i + 1 < len(text) and text[i + 1] in "nt\\":
            out.append({"n": "\n", "t": "\t", "\\": "\\"}[text[i + 1]])
            i += 2
        else:
            out.append(text[i])
            i += 1
    return "".join(out)


def resolve(name, lay):
    """One key of a combo as (code, extra modifier bits): a named key (Return, F5, ctrl) or a single character through the layout."""
    code = keys.named(name)
    if code is not None:
        return code, 0
    if len(name) == 1:
        hit = lay.find(name)
        if hit:
            return hit
    raise ValueError(f"unknown key: {name!r} (names like Return, Escape, Tab, F5, Page_Down, ctrl, shift, alt, super; or one character)")


def parse_combo(combo, lay=None):
    """`ctrl+shift+t` -> [codes in press order]. Modifiers a character needs (Shift for "A") are added in front."""
    lay = lay or layout.current()
    if combo.endswith("+") and len(combo) > 1:  # "ctrl++" is ctrl and the plus key
        parts = [p for p in combo[:-1].split("+") if p] + ["+"]
    else:
        parts = [p for p in combo.split("+") if p] or ([combo] if combo else [])
    if not parts:
        raise ValueError("empty key")
    codes, extra = [], 0
    for part in parts:
        code, mods = resolve(part, lay)
        codes.append(code)
        extra |= mods
    front = [c for c, bit in ((KEY_LEFTSHIFT, layout.SHIFT), (KEY_RIGHTALT, layout.ALTGR)) if extra & bit and c not in codes]
    return front + codes


class Keyboard:
    def __init__(self, lay=None, delay=None, settle=0.8):
        self.layout = lay or layout.current()
        self.delay = delay if delay is not None else max(0, min(500, float(load().get("type_delay_ms") or 8))) / 1000
        self.down = []  # codes held, in the order they went down
        self.fd = create_device("ghosthand-keyboard", keys=keys.ALL_KEYS, settle=settle)

    def emit(self, code, value):
        os.write(self.fd, struct.pack("llHHi", 0, 0, EV_KEY, code, value) + struct.pack("llHHi", 0, 0, EV_SYN, 0, 0))

    def press(self, code):
        if code not in self.down:
            self.down.append(code)
        self.emit(code, 1)

    def release(self, code):
        if code in self.down:
            self.down.remove(code)
        self.emit(code, 0)

    def close(self):
        for code in reversed(list(self.down)):  # never leave a key held
            self.emit(code, 0)
        self.down.clear()
        time.sleep(0.15)
        destroy_device(self.fd)

    def tap(self, codes, hold=0.04):
        """Press every code in order, then release them in reverse: a chord."""
        for c in codes:
            self.press(c)
            time.sleep(0.01)
        time.sleep(hold)
        for c in reversed(codes):
            self.release(c)
        time.sleep(self.delay)

    def key(self, combo):
        self.tap(parse_combo(combo, self.layout))

    def hold(self, combo, down=True):
        for c in parse_combo(combo, self.layout):
            (self.press if down else self.release)(c)

    def type(self, text, check=lambda: None):
        """Type text, one key press per character. Raises ValueError (before typing anything) for characters this layout can't make."""
        lost = self.layout.missing(text)
        if lost:
            raise ValueError(f"can't type {''.join(lost)!r} with the {self.layout.name} layout: use `paste`, or set keyboard_layout in the config")
        for i, ch in enumerate(text):
            code, mods = self.layout.find(ch)
            codes = [c for c, bit in ((KEY_LEFTSHIFT, layout.SHIFT), (KEY_RIGHTALT, layout.ALTGR)) if mods & bit] + [code]
            self.tap(codes, hold=0.006)
            if i % 24 == 23:  # a revoke stops typing within a couple of dozen characters
                check()
        time.sleep(0.05)


def clipboard_tools():
    """(copy argv, paste argv) for the session's clipboard, or None."""
    if os.environ.get("WAYLAND_DISPLAY") and shutil.which("wl-copy") and shutil.which("wl-paste"):
        return ["wl-copy"], ["wl-paste", "-n"]
    if shutil.which("xclip"):
        return ["xclip", "-selection", "clipboard", "-i"], ["xclip", "-selection", "clipboard", "-o"]
    if shutil.which("xsel"):
        return ["xsel", "-b", "-i"], ["xsel", "-b", "-o"]
    return None


def paste(kb, text, terminal=False):
    """Put `text` on the clipboard, press Ctrl+V (Ctrl+Shift+V for terminals), then put back what was there. For text `type` can't make."""
    tools = clipboard_tools()
    if not tools:
        raise ValueError("paste needs wl-clipboard (Wayland) or xclip or xsel (X11)")
    copy, read = tools
    try:
        before = subprocess.run(read, capture_output=True, timeout=3).stdout
    except (OSError, subprocess.SubprocessError):
        before = None
    subprocess.run(copy, input=text.encode(), check=True, timeout=3)
    time.sleep(0.15)
    kb.tap([KEY_LEFTCTRL] + ([KEY_LEFTSHIFT] if terminal else []) + [KEY_V])
    time.sleep(0.4)  # let the app read it before the old clipboard comes back
    if before is not None:
        subprocess.run(copy, input=before, timeout=3)
