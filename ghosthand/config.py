"""The small config file, ~/.config/ghosthand/config.json. Every key is optional.

  mode            "ask" (default), "notify" or "off": read by the overlay, never taken from a client, so a program can't switch its own gate off
  countdown       seconds the card waits (2 to 60, default 8)
  grant_minutes   how long "Allow for" lasts (1 to 240, default 10)
  overlay         the Quickshell config name of the overlay (default "ghosthand")
  lock_check      a shell command that prints "no" while the session is unlocked (anything else counts as locked) (default: loginctl's LockedHint)
  output          the output to drive, by name (default: the first one niri lists)
"""
import json

from .paths import config_file

DEFAULTS = {
    "overlay": "ghosthand",
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
