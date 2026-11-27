"""Screen geometry: the output's size in screenshot pixels and its scale, and turning action words into numbers.

Detected from the compositor that's running: niri, Hyprland, sway, then X11 (xrandr), then wlr-randr, and as a last resort from the size
of a screenshot (scale 1). GHOSTHAND_OUTPUT_SIZE="2560x1600" and GHOSTHAND_SCALE="1.25" override all of it."""
import json
import os
import re
import shutil
import subprocess


def _run(*argv):
    return subprocess.run(argv, capture_output=True, text=True, check=True, timeout=5).stdout


def _swap(width, height, transform):
    """Outputs rotated a quarter turn are taller than wide in the screenshot. `transform`: 1/3/5/7 (Hyprland), "90", "flipped-270" (sway), "Flipped90" (niri)."""
    t = str(transform).lower()
    return (height, width) if t in ("1", "3", "5", "7") or t.endswith("90") or t.endswith("270") else (width, height)


def from_niri(name=None, text=None):
    outputs = json.loads(text if text is not None else _run("niri", "msg", "--json", "outputs"))
    out = outputs.get(name) if name and name in outputs else next(iter(outputs.values()))
    mode = out["modes"][out["current_mode"]]
    logical = out.get("logical") or {}
    w, h = _swap(mode["width"], mode["height"], logical.get("transform", "Normal"))
    return {"name": out.get("name", name or ""), "width": w, "height": h, "scale": logical.get("scale") or 1.0}


def from_hyprland(name=None, text=None):
    monitors = json.loads(text if text is not None else _run("hyprctl", "-j", "monitors"))
    pick = next((m for m in monitors if name and m.get("name") == name), None) or next((m for m in monitors if m.get("focused")), monitors[0])
    w, h = _swap(pick["width"], pick["height"], pick.get("transform", 0))
    return {"name": pick["name"], "width": w, "height": h, "scale": float(pick.get("scale") or 1.0)}


def from_sway(name=None, text=None):
    outputs = [o for o in json.loads(text if text is not None else _run("swaymsg", "-t", "get_outputs", "-r")) if o.get("active", True)]
    pick = next((o for o in outputs if name and o.get("name") == name), None) or next((o for o in outputs if o.get("focused")), outputs[0])
    mode = pick["current_mode"]
    w, h = _swap(mode["width"], mode["height"], pick.get("transform", "normal"))
    return {"name": pick["name"], "width": w, "height": h, "scale": float(pick.get("scale") or 1.0)}


def from_xrandr(name=None, text=None):
    """On X11 the screenshot is the whole desktop, so that's the size (all monitors together)."""
    m = re.search(r"current (\d+) x (\d+)", text if text is not None else _run("xrandr"))
    if not m:
        raise ValueError("no screen size in the xrandr output")
    return {"name": "X11", "width": int(m.group(1)), "height": int(m.group(2)), "scale": 1.0}


def from_wlr_randr(name=None, text=None):
    outputs = json.loads(text if text is not None else _run("wlr-randr", "--json"))
    pick = next((o for o in outputs if name and o.get("name") == name), None) or next(o for o in outputs if o.get("enabled", True))
    mode = next(m for m in pick["modes"] if m.get("current"))
    w, h = _swap(mode["width"], mode["height"], pick.get("transform", "normal"))
    return {"name": pick["name"], "width": w, "height": h, "scale": float(pick.get("scale") or 1.0)}


def from_screenshot():
    """The size of a screenshot: always right for the pixels, but the scale is a guess (1, or GHOSTHAND_SCALE)."""
    from .screen import png_size, screenshot_bytes
    size = png_size(screenshot_bytes())
    if not size:
        raise RuntimeError("couldn't tell the screen size: set GHOSTHAND_OUTPUT_SIZE and GHOSTHAND_SCALE")
    return {"name": "", "width": size[0], "height": size[1], "scale": float(os.environ.get("GHOSTHAND_SCALE") or 1.0)}


def detectors():
    """[(name, function)] worth trying in this session, most specific first."""
    env, found = os.environ, []
    if env.get("NIRI_SOCKET") and shutil.which("niri"):
        found.append(("niri", from_niri))
    if env.get("HYPRLAND_INSTANCE_SIGNATURE") and shutil.which("hyprctl"):
        found.append(("Hyprland", from_hyprland))
    if env.get("SWAYSOCK") and shutil.which("swaymsg"):
        found.append(("sway", from_sway))
    if not env.get("WAYLAND_DISPLAY") and env.get("DISPLAY") and shutil.which("xrandr"):
        found.append(("X11", from_xrandr))
    if env.get("WAYLAND_DISPLAY") and shutil.which("wlr-randr"):
        found.append(("wlr-randr", from_wlr_randr))
    return found


def output_info(name=None):
    """An output as { name, width, height, scale }: the size in screenshot pixels, and the scale the compositor reports."""
    size, scale = os.environ.get("GHOSTHAND_OUTPUT_SIZE"), os.environ.get("GHOSTHAND_SCALE")
    if size and scale:
        w, h = size.lower().split("x")
        return {"name": os.environ.get("GHOSTHAND_OUTPUT", name or ""), "width": int(w), "height": int(h), "scale": float(scale)}
    for _label, fn in detectors():
        try:
            return fn(name)
        except (OSError, ValueError, KeyError, StopIteration, subprocess.SubprocessError):
            continue
    return from_screenshot()


def detected():
    """What output_info would use, for `ghosthand doctor`: the name of the detector, or "screenshot size"."""
    for label, fn in detectors():
        try:
            fn(None)
            return label
        except (OSError, ValueError, KeyError, StopIteration, subprocess.SubprocessError):
            continue
    return "screenshot size"


def numbers(words):
    """The numbers among an action's words (floats, negatives and a leading + are fine); anything else is an error."""
    out = []
    for w in words:
        try:
            out.append(float(w))
        except ValueError:
            raise ValueError(f"not a number: {w!r}") from None
    return out
