"""The actions: what you can say to the pointer, parsed, checked and run."""
import json
import subprocess
import time

from . import keyboard as kb
from .config import load
from .geometry import numbers
from .screen import screenshot
from .uinput import BTN_LEFT, BTN_MIDDLE, BTN_RIGHT

POINTER = ("jump", "move", "click", "dclick", "rclick", "mclick", "down", "up", "rdown", "rup", "drag", "rdrag", "scroll", "hscroll", "pos")
KEYBOARD = ("key", "keydown", "keyup", "type", "paste")
NO_POINTER = ("shot", "wait", "info", "focus")  # these touch nothing, so they need no approval
ACTIONS = POINTER + KEYBOARD + NO_POINTER


def parse_args(argv):
    """(options, actions): --who, --reason and --countdown come before the actions."""
    opts = {"who": "Agent", "reason": "", "countdown": 0}
    i = 0
    while i < len(argv) and argv[i].startswith("--"):
        key = argv[i][2:]
        if key not in opts or i + 1 >= len(argv):
            raise ValueError(f"unknown or incomplete option: {argv[i]}")
        opts[key] = int(argv[i + 1]) if key == "countdown" else argv[i + 1]
        i += 2
    return opts, argv[i:]


def head(action):
    return action.split()[0]


def uses_keyboard(actions):
    return any(head(a) in KEYBOARD for a in actions)


def uses_pointer(actions):
    return any(head(a) in POINTER for a in actions)


def touches_input(actions):
    """Does anything here need the gate? (Everything but screenshots, waits, info and focus.)"""
    return any(head(a) not in NO_POINTER for a in actions)


touches_pointer = touches_input  # the old name


def scope(actions):
    """What the card says is being taken: "mouse", "keyboard" or "mouse and keyboard"."""
    return " and ".join(n for n, used in (("mouse", uses_pointer(actions)), ("keyboard", uses_keyboard(actions))) if used) or "mouse"


def detail(actions, limit=140):
    """What will be typed or pressed, for the card: `type: git status`, `keys: ctrl+c`. Empty when no keyboard is used."""
    parts = []
    for a in actions:
        cmd, _, rest = a.partition(" ")
        if cmd == "type" or cmd == "paste":
            parts.append(f"{cmd}: {kb.unescape(rest)!r}")
        elif cmd in ("key", "keydown", "keyup"):
            parts.append(f"{cmd}: {rest}")
    text = "  ".join(parts)
    return text if len(text) <= limit else text[: limit - 1] + "\u2026"


def validate(actions):
    """A typo (or a character the keyboard layout can't type) is found before anything is touched."""
    for a in actions:
        words = a.split()
        if not words or words[0] not in ACTIONS:
            raise ValueError(f"unknown action: {a!r}")
        if words[0] in KEYBOARD:
            if load().get("keyboard") is False:
                raise ValueError("the keyboard is switched off in the config (\"keyboard\": false)")
            rest = a.partition(" ")[2]
            if not rest.strip():
                raise ValueError(f"{words[0]} needs something to {'type' if words[0] in ('type', 'paste') else 'press'}")
            if words[0] == "type":
                lost = kb.layout.current().missing(kb.unescape(rest))
                if lost:
                    raise ValueError(f"can't type {''.join(lost)!r} with the {kb.layout.current().name} layout: use `paste`, or set keyboard_layout in the config")
            elif words[0] != "paste":
                for combo in words[1:]:
                    kb.parse_combo(combo)


def run(pointer, action, check, info, out=print, keyboard=None):
    """One action. `check()` raises Revoked when the person took the pointer back; it is called between steps."""
    p = action.split()
    if not p:
        return
    cmd, words = p[0], p[1:]
    check()
    if cmd == "shot":
        if not words:
            raise ValueError("shot needs a file")
        region = numbers(words[1:])
        screenshot(words[0], region if len(region) == 4 else None)
        return
    if cmd == "wait":
        end = time.time() + numbers(words)[0]
        while time.time() < end:  # in slices, so a revoke is noticed
            time.sleep(min(0.25, max(0, end - time.time())))
            check()
        return
    if cmd == "info":
        out(json.dumps(info))
        return
    if cmd == "focus":
        from .windows import focused
        out(json.dumps(focused()))
        return
    if cmd in KEYBOARD:
        text = action.partition(" ")[2]
        if cmd == "type":
            keyboard.type(kb.unescape(text), check)
        elif cmd == "paste":
            kb.paste(keyboard, kb.unescape(text), terminal=load().get("paste_terminal", False))
        elif cmd == "key":
            for combo in words:
                keyboard.key(combo)
                check()
        else:
            for combo in words:
                keyboard.hold(combo, down=cmd == "keydown")
        return
    if cmd == "pos":
        out(json.dumps({"x": round(pointer.x), "y": round(pointer.y), "known": pointer.known}))
        return
    args = numbers(words)
    at = args[-2:] if len(args) >= 2 and cmd in ("click", "dclick", "rclick", "mclick", "scroll", "hscroll") else []
    if cmd == "jump":
        pointer.put(*args[:2])
    elif cmd == "move":
        pointer.glide(*args[:2])
    elif cmd in ("click", "dclick", "rclick", "mclick"):
        if len(args) >= 2:
            pointer.glide(*args[:2])
            time.sleep(0.2)
        pointer.click({"click": BTN_LEFT, "dclick": BTN_LEFT, "rclick": BTN_RIGHT, "mclick": BTN_MIDDLE}[cmd], 2 if cmd == "dclick" else 1)
    elif cmd in ("scroll", "hscroll"):
        if len(at) == 2 and len(args) >= 3:
            pointer.glide(*at)
            time.sleep(0.2)
        pointer.wheel(args[0], horizontal=cmd == "hscroll")
    elif cmd == "down":
        pointer.button(BTN_LEFT, True)
    elif cmd == "up":
        pointer.button(BTN_LEFT, False)
    elif cmd == "rdown":
        pointer.button(BTN_RIGHT, True)
    elif cmd == "rup":
        pointer.button(BTN_RIGHT, False)
    elif cmd in ("drag", "rdrag"):
        x1, y1, x2, y2 = args[:4]
        code = BTN_LEFT if cmd == "drag" else BTN_RIGHT
        pointer.put(x1, y1)
        time.sleep(0.3)
        pointer.button(code, True)
        time.sleep(0.2)
        pointer.glide((x1 + x2) / 2, (y1 + y2) / 2 + 20)
        pointer.glide(x2, y2)
        time.sleep(0.4)
        pointer.button(code, False)
    else:
        raise ValueError(f"unknown action: {action!r}")
