"""Screenshots on whatever the session has: grim (wlroots compositors: niri, Hyprland, sway, river...), maim, scrot or ImageMagick on X11,
spectacle (KDE) or gnome-screenshot (GNOME). The first tool that is installed and works wins; `screenshot` in the config overrides it
with your own command (`{file}` is where it writes, e.g. "mytool --out {file}")."""
import os
import shutil
import subprocess
import tempfile

from .config import load


def _grim(f, r):
    return ["grim", *(["-g", f"{r[0]},{r[1]} {r[2]}x{r[3]}"] if r else []), f]


def _maim(f, r):
    return ["maim", *(["-g", f"{r[2]}x{r[3]}+{r[0]}+{r[1]}"] if r else []), f]


def _scrot(f, r):
    return ["scrot", "-o", *(["-a", f"{r[0]},{r[1]},{r[2]},{r[3]}"] if r else []), f]


def _import(f, r):
    return ["import", "-window", "root", *(["-crop", f"{r[2]}x{r[3]}+{r[0]}+{r[1]}", "+repage"] if r else []), f]


def _spectacle(f, r):
    return ["spectacle", "-b", "-n", "-f", "-o", f] if not r else None  # no region: the caller crops


def _gnome(f, r):
    return ["gnome-screenshot", "-f", f] if not r else None


BACKENDS = {"grim": _grim, "maim": _maim, "scrot": _scrot, "import": _import, "spectacle": _spectacle, "gnome-screenshot": _gnome}
WAYLAND_ORDER = ["grim", "spectacle", "gnome-screenshot", "maim", "scrot", "import"]
X11_ORDER = ["maim", "scrot", "import", "gnome-screenshot", "spectacle", "grim"]


def order():
    """The tools worth trying here, best first, only those installed."""
    wayland = bool(os.environ.get("WAYLAND_DISPLAY")) or os.environ.get("XDG_SESSION_TYPE") == "wayland"
    return [t for t in (WAYLAND_ORDER if wayland else X11_ORDER) if shutil.which(t)]


def crop(path, region):
    """Cut a region out of a full screenshot, for tools that can't (needs Pillow, or ImageMagick's convert)."""
    x, y, w, h = region
    if shutil.which("convert"):
        subprocess.run(["convert", path, "-crop", f"{w}x{h}+{x}+{y}", "+repage", path], check=True, capture_output=True)
        return
    try:
        from PIL import Image  # optional
    except ImportError:
        raise RuntimeError("this screenshot tool can't take a region: install grim, maim, scrot or ImageMagick, or ask for the whole screen") from None
    with Image.open(path) as im:
        im.crop((x, y, x + w, y + h)).save(path)


def _custom(path, region, command):
    if region:
        raise RuntimeError("a custom `screenshot` command in the config always captures the whole screen")
    subprocess.run(command.replace("{file}", path), shell=True, check=True, capture_output=True, timeout=30)


def screenshot(path, region=None):
    """Write a PNG of the screen (or of region = x, y, w, h in pixels) to `path`. Raises RuntimeError when nothing works."""
    region = [int(v) for v in region] if region else None
    command = load().get("screenshot")
    if command:
        return _custom(path, region, command)
    errors = []
    for tool in order():
        argv = BACKENDS[tool](path, region)
        full = argv or BACKENDS[tool](path, None)
        try:
            subprocess.run(full, check=True, capture_output=True, timeout=30)
            if os.path.getsize(path) == 0:
                raise OSError("empty file")
            if not argv:
                crop(path, region)
            return
        except (OSError, subprocess.SubprocessError, RuntimeError) as e:
            detail = (getattr(e, "stderr", b"") or b"").decode(errors="replace").strip().splitlines()
            errors.append(f"{tool}: {detail[-1] if detail else e}")
    raise RuntimeError("no screenshot tool worked (" + ("; ".join(errors) or "none installed: try grim, maim, scrot or ImageMagick") + ")")


def screenshot_bytes(region=None):
    fd, path = tempfile.mkstemp(suffix=".png", prefix="ghosthand-")
    os.close(fd)
    try:
        screenshot(path, region)
        with open(path, "rb") as f:
            return f.read()
    finally:
        os.unlink(path)


def png_size(data):
    """(width, height) from a PNG's header, or None."""
    if len(data) >= 24 and data[:8] == b"\x89PNG\r\n\x1a\n":
        return int.from_bytes(data[16:20], "big"), int.from_bytes(data[20:24], "big")
    return None
