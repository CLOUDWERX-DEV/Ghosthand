"""`ghosthand doctor` and `ghosthand setup`: check what's needed, and put the overlay where Quickshell finds it."""
import os
import shutil
import subprocess
import sys
from pathlib import Path

from . import gate as g
from .config import load
from .paths import config_file

OVERLAY_SRC = Path(__file__).parent / "overlay"


def overlay_dir():
    return Path(os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config") / "quickshell" / load()["overlay"]


def checks():
    """[(name, ok, how to fix)] for everything Ghosthand needs."""
    out = []
    uinput = os.access("/dev/uinput", os.W_OK)
    out.append(("write access to /dev/uinput", uinput, "see docs/setup.md: a udev rule and the input group, or an ACL for your user"))
    for tool, why in (("grim", "screenshots"), ("qs", "the overlay (Quickshell)"), ("niri", "the output's scale (or set GHOSTHAND_OUTPUT_SIZE and GHOSTHAND_SCALE)")):
        out.append((f"{tool} on PATH ({why})", shutil.which(tool) is not None, f"install {tool}"))
    out.append(("the overlay is installed", (overlay_dir() / "shell.qml").exists(), "run `ghosthand setup`"))
    answer = g.qs("status")
    out.append(("the overlay is running", answer is not None, "run `ghosthand overlay` (autostart it: see docs/setup.md)"))
    out.append(("the session isn't locked", not g.locked(), "unlock the session"))
    return out


def doctor(write=print):
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
    write("next: start the overlay (`ghosthand overlay`, or autostart it), then run `ghosthand doctor`")
    return 0
