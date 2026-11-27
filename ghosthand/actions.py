"""The actions: what you can say to the pointer, parsed, checked and run."""
import json
import subprocess
import time

from .geometry import numbers
from .uinput import BTN_LEFT, BTN_MIDDLE, BTN_RIGHT

ACTIONS = ("jump", "move", "click", "dclick", "rclick", "mclick", "down", "up", "rdown", "rup", "drag", "rdrag",
           "scroll", "hscroll", "wait", "shot", "pos", "info")
NO_POINTER = ("shot", "wait", "info")  # these never touch the pointer, so they need no approval


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


def validate(actions):
    """A typo is found before the pointer is ever touched."""
    for a in actions:
        words = a.split()
        if not words or words[0] not in ACTIONS:
            raise ValueError(f"unknown action: {a!r}")


def touches_pointer(actions):
    return any(a.split()[0] not in NO_POINTER for a in actions)


def screenshot(path, region=None):
    """A picture of the screen (or of x, y, w, h) with grim. `path` "-" writes PNG bytes to stdout of the process: use screenshot_bytes."""
    args = ["grim"]
    if region:
        x, y, w, h = (int(v) for v in region)
        args += ["-g", f"{x},{y} {w}x{h}"]
    subprocess.run(args + [path], check=True)


def screenshot_bytes(region=None):
    args = ["grim"]
    if region:
        x, y, w, h = (int(v) for v in region)
        args += ["-g", f"{x},{y} {w}x{h}"]
    return subprocess.run(args + ["-"], check=True, capture_output=True).stdout


def run(pointer, action, check, info, out=print):
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
