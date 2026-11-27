"""The gate: ask the overlay for the pointer and wait for a person to answer.

The overlay (overlay/, a Quickshell config) draws the card and the cursor tag. The client talks to it over Quickshell IPC
(`qs -c ghosthand ipc call ghosthand ...`) and streams the pointer position as lines in a file the overlay follows."""
import os
import subprocess
import time

from .config import load
from .paths import runtime_dir, stream_file

EXIT_FAILED, EXIT_USAGE, EXIT_REVOKED, EXIT_DENIED, EXIT_NO_GATE = 1, 2, 3, 4, 5


class Revoked(Exception):
    """The person took the pointer back (Deny, or the revoke key)."""


class GateError(Exception):
    """The gate said no, or could not be reached. `code` is the exit code."""

    def __init__(self, message, code):
        super().__init__(message)
        self.code = code


def qs(*args, timeout=5):
    """`qs -c <overlay> ipc call ghosthand ...`: the answer, or None when the overlay can't be reached."""
    try:
        r = subprocess.run(["qs", "-c", load()["overlay"], "ipc", "call", "ghosthand", *map(str, args)], capture_output=True, text=True, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return r.stdout.strip() if r.returncode == 0 else None


def locked(command=None):
    """Is the session locked? Runs `lock_check` from the config, which prints "yes" while it is. Unknown counts as locked: refuse, don't guess."""
    command = command or load()["lock_check"]
    try:
        out = subprocess.run(["sh", "-c", command], capture_output=True, text=True, timeout=5).stdout.strip().lower()
    except (OSError, subprocess.TimeoutExpired):
        return True
    return out not in ("no", "false", "0")


def gate(who, reason, countdown, ask=qs, sleep=time.sleep, now=time.time, patience=None):
    """Ask for the pointer and wait for the answer. Returns when allowed; raises GateError otherwise."""
    first = ask("ask", who, reason, countdown or 0)
    if first is None:
        raise GateError("Ghosthand: the overlay isn't answering, so the pointer isn't touched (there is no way around the gate). Is `ghosthand overlay` running?", EXIT_NO_GATE)
    if first == "allowed":
        return
    limit = now() + (patience if patience is not None else max(20, (countdown or 8) + 15))
    while now() < limit:
        sleep(0.15)
        state = ask("status")
        if state in ("allowed", "active"):  # the overlay moves straight on to "active" once you allow it
            return
        if state in ("denied", "timeout", "revoked"):
            raise GateError(f"Ghosthand: not allowed ({state}).", EXIT_DENIED)
        if state is None:
            raise GateError("Ghosthand: lost the overlay while waiting for an answer.", EXIT_NO_GATE)
    ask("deny")
    raise GateError("Ghosthand: no answer in time.", EXIT_DENIED)


def stream_writer():
    """Append the position lines the overlay follows. Returns (write, close)."""
    runtime_dir().mkdir(parents=True, exist_ok=True)
    f = open(stream_file(), "a", buffering=1)
    return f.write, f.close


def check_not_revoked(ask=qs):
    """Raises Revoked when the person took the pointer back ("idle" means the overlay restarted or the card was closed)."""
    if ask("status") in ("revoked", "denied", "idle", "timeout"):
        raise Revoked()
