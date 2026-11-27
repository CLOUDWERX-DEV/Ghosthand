"""Where Ghosthand keeps its small runtime files and its config."""
import os
from pathlib import Path


def runtime_dir():
    return Path(os.environ.get("XDG_RUNTIME_DIR") or "/tmp") / "ghosthand"


def stream_file():
    """The position stream the overlay follows (one line per pointer step)."""
    return runtime_dir() / "stream"


def last_position_file():
    return runtime_dir() / "last-position.json"


def config_file():
    return Path(os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config") / "ghosthand" / "config.json"
