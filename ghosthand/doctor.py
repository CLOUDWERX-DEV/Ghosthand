"""`ghosthand doctor`, `ghosthand setup` and `ghosthand skill`: check what's needed, install the overlay and the agent skill."""
import os
import shutil
import sys
from pathlib import Path

from . import dialog, geometry, keyboard, layout, screen
from . import gate as g
from .config import load
from .paths import config_file

PKG = Path(__file__).parent
OVERLAY_SRC = PKG / "overlay"
SKILL_SRC = PKG / "skill" / "SKILL.md"


def overlay_dir():
    return Path(os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config") / "quickshell" / load()["overlay"]


def session_label():
    for key, label in (("NIRI_SOCKET", "niri"), ("HYPRLAND_INSTANCE_SIGNATURE", "Hyprland"), ("SWAYSOCK", "sway")):
        if os.environ.get(key):
            return label
    return os.environ.get("XDG_CURRENT_DESKTOP") or os.environ.get("XDG_SESSION_TYPE") or "unknown session"


def checks():
    """[(name, ok, how to fix)] for everything Ghosthand needs here."""
    out = [("write access to /dev/uinput", os.access("/dev/uinput", os.W_OK), "see docs/setup.md: a udev rule and the input group")]
    tools = screen.order()
    out.append((f"a screenshot tool ({tools[0] if tools else 'none'})", bool(tools) or bool(load().get("screenshot")), "install grim (Wayland) or maim/scrot/ImageMagick (X11)"))
    out.append((f"the screen size ({geometry.detected()})", True, ""))
    out.append((f"keyboard layout ({layout.current().name})", True, ""))
    out.append((f"a clipboard for `paste` ({'found' if keyboard.clipboard_tools() else 'none: install wl-clipboard, xclip or xsel'})", True, ""))
    overlay_up = shutil.which("qs") is not None and g.qs("status") is not None
    dlg = dialog.available()
    out.append((f"something to ask you ({'the overlay' if overlay_up else dlg or 'nothing yet'})", overlay_up or bool(dlg),
                "run `ghosthand overlay` (Wayland with layer-shell, needs Quickshell) or install zenity, yad or kdialog"))
    out.append(("the session isn't locked", not g.locked(), "unlock the session"))
    return out


def doctor(write=print):
    write(f"session: {session_label()}")
    ok = True
    for name, good, fix in checks():
        write(f"  {'ok ' if good else 'NO '} {name}" + ("" if good else f"   -> {fix}"))
        ok = ok and good
    write("everything is in place" if ok else "some things are missing (see above)")
    return 0 if ok else 1


def setup(write=print):
    """Copy the overlay into ~/.config/quickshell/<name> (replacing a previous copy) and write a default config if there isn't one."""
    dest = overlay_dir()
    if dest.is_symlink() or dest.exists():
        if dest.is_symlink():
            dest.unlink()
        else:
            shutil.rmtree(dest)
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(OVERLAY_SRC, dest)
    write(f"overlay installed in {dest}")
    cfg = config_file()
    if not cfg.exists():
        cfg.parent.mkdir(parents=True, exist_ok=True)
        cfg.write_text('{\n  "mode": "ask",\n  "countdown": 8,\n  "grant_minutes": 10\n}\n')
        write(f"wrote {cfg}")
    write("next: start the overlay (`ghosthand overlay`, or autostart it), then run `ghosthand doctor`. No Quickshell? Install zenity and skip the overlay.")
    return 0


def skill(args, write=print):
    """`ghosthand skill` prints the agent skill; `ghosthand skill install [DIR]` puts it in DIR/ghosthand/ (default ~/.claude/skills)."""
    text = SKILL_SRC.read_text()
    if not args:
        write(text)
        return 0
    if args[0] != "install":
        print("usage: ghosthand skill [install [DIR]]", file=sys.stderr)
        return g.EXIT_USAGE
    root = Path(args[1]).expanduser() if len(args) > 1 else Path.home() / ".claude" / "skills"
    (root / "ghosthand").mkdir(parents=True, exist_ok=True)
    (root / "ghosthand" / "SKILL.md").write_text(text)
    write(f"skill installed in {root / 'ghosthand'}")
    return 0
