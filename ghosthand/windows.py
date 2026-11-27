"""Which window has keyboard focus: `focus` prints it, so an agent knows where its keys will land."""
import json
import os
import shutil
import subprocess


def _run(*argv):
    return subprocess.run(argv, capture_output=True, text=True, check=True, timeout=4).stdout


def _sway_focused(node):
    if node.get("focused"):
        return node
    for child in node.get("nodes", []) + node.get("floating_nodes", []):
        found = _sway_focused(child)
        if found:
            return found
    return None


def focused():
    """{ app, title } of the focused window, or None when the session can't say (GNOME, KDE and others don't expose it)."""
    env = os.environ
    try:
        if env.get("NIRI_SOCKET") and shutil.which("niri"):
            w = json.loads(_run("niri", "msg", "--json", "focused-window"))
            return {"app": w.get("app_id") or "", "title": w.get("title") or ""} if w else None
        if env.get("HYPRLAND_INSTANCE_SIGNATURE") and shutil.which("hyprctl"):
            w = json.loads(_run("hyprctl", "-j", "activewindow"))
            return {"app": w.get("class") or "", "title": w.get("title") or ""} if w.get("address") else None
        if env.get("SWAYSOCK") and shutil.which("swaymsg"):
            w = _sway_focused(json.loads(_run("swaymsg", "-t", "get_tree", "-r")))
            return {"app": w.get("app_id") or (w.get("window_properties") or {}).get("class") or "", "title": w.get("name") or ""} if w and w.get("pid") else None
        if env.get("DISPLAY") and shutil.which("xdotool"):
            return {"app": "", "title": _run("xdotool", "getactivewindow", "getwindowname").strip()}
    except (OSError, ValueError, KeyError, subprocess.SubprocessError):
        return None
    return None
