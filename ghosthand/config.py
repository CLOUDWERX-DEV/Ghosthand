"""The small config file, ~/.config/ghosthand/config.json. Every key is optional.

  mode            "ask" (default), "notify" or "off": read by the overlay, never taken from a client, so a program can't switch its own gate off
  countdown       seconds the card waits (2 to 60, default 8)
  grant_minutes   how long "Allow for" lasts (1 to 240, default 10)
  gate            "auto" (default: the overlay if it answers, else a zenity/yad/kdialog dialog), "overlay" or "dialog"
  screenshot      a command that writes a PNG to {file}, to replace the built-in tool search (grim, maim, scrot, import, spectacle, gnome-screenshot)
  keyboard        false switches the keyboard actions off entirely (default true)
  keyboard_layout the xkb layout `type` should assume ("us", "de", "fr"...), with keyboard_variant ("dvorak") and keyboard_options (default: detected)
  type_delay_ms   milliseconds between typed characters (default 8)
  paste_terminal  true makes `paste` press Ctrl+Shift+V instead of Ctrl+V (terminals)
  overlay         the Quickshell config name of the overlay (default "ghosthand")
  lock_check      a shell command that prints "no" while the session is unlocked (anything else counts as locked) (default: loginctl's LockedHint)
  output          the output to drive, by name (default: the first one the compositor lists)
"""
import json

from .paths import config_file

DEFAULTS = {
    "overlay": "ghosthand",
    "gate": "auto",
    "lock_check": 'loginctl show-session "${XDG_SESSION_ID:-auto}" -p LockedHint --value',
    "output": None,
}


def load(path=None):
    """The config with defaults filled in. A missing or broken file gives the defaults."""
    out = dict(DEFAULTS)
    try:
        with open(path or config_file()) as f:
            data = json.load(f)
        if isinstance(data, dict):
            out.update({k: v for k, v in data.items() if v is not None})
    except (OSError, ValueError):
        pass
    return out
