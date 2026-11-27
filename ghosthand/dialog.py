"""The gate without the overlay: a plain dialog from zenity, yad or kdialog.

For sessions where the Quickshell overlay can't run (X11, GNOME, anything without wlr-layer-shell). It asks the same question, with the same
answers (Allow, Allow for N min, Deny, silence is a no), speaks the same little protocol as the overlay (`ask`, `status`, `deny`, `end`,
`revoke`), and `ghosthand revoke` still takes the pointer back, through a file in the runtime directory. What it can't do is draw the tag and
ring that follow the cursor: a notification says the pointer is in use instead. Settings come from the same config file as the overlay."""
import os
import shutil
import subprocess
import time

from .config import load
from .paths import runtime_dir

TOOLS = ("zenity", "yad", "kdialog")


def available():
    return next((t for t in TOOLS if shutil.which(t)), None)


def _revoked_file():
    return runtime_dir() / "revoked"


def _grant_file(kind=""):
    """When a standing grant ends. The keyboard has its own: allowing the mouse for ten minutes doesn't let anything type."""
    return runtime_dir() / ("grant" + (f"-{kind}" if kind else ""))


def revoke():
    """Take the pointer back from any process: a running agent notices on its next step, and a standing grant ends."""
    runtime_dir().mkdir(parents=True, exist_ok=True)
    _revoked_file().write_text(f"{time.time()}\n")
    for kind in ("", "keys"):
        try:
            _grant_file(kind).unlink()
        except OSError:
            pass


def _read_float(path):
    try:
        return float(path.read_text().strip())
    except (OSError, ValueError):
        return 0.0


def command(tool, who, reason, seconds, minutes, scope="mouse", detail=""):
    """The dialog's argv. Exit codes differ by tool, see `answer`."""
    text = (f"{who} wants to use your {scope}.\n\n" + (f"{reason}\n\n" if reason else "") + (f"It will: {detail}\n\n" if detail else "")
            + "Nothing happens until you allow it.")
    if tool == "zenity":
        return ["zenity", "--question", "--title=Ghosthand", f"--text={text}", "--ok-label=Allow", "--cancel-label=Deny",
                f"--extra-button=Allow for {minutes} min", f"--timeout={seconds}", "--no-markup"]
    if tool == "yad":
        return ["yad", "--title=Ghosthand", "--image=input-mouse", f"--text={text}", "--button=Deny:1", f"--button=Allow for {minutes} min:2",
                "--button=Allow:0", f"--timeout={seconds}", "--center", "--on-top"]
    return ["kdialog", "--title", "Ghosthand", "--warningyesnocancel", text, "--yes-label", "Allow", "--no-label", f"Allow for {minutes} min",
            "--cancel-label", "Deny"]


def answer(tool, code, out, minutes):
    """Turn a finished dialog into "allow", "grant" (allow for N min), "deny" or "timeout"."""
    if tool == "zenity":
        return "allow" if code == 0 else "grant" if code == 1 and "Allow for" in out else "timeout" if code == 5 else "deny"
    if tool == "yad":
        return {0: "allow", 2: "grant", 70: "timeout"}.get(code, "deny")
    return {0: "allow", 1: "grant"}.get(code, "deny")  # kdialog


class DialogGate:
    """Same calls as the overlay's IPC: `gate(\"ask\", who, reason, seconds)`, `gate(\"status\")`, and so on."""

    def __init__(self, tool=None, settings=None):
        self.tool = tool or available()
        self.settings = settings or load
        self.proc = None
        self.state = "idle"
        self.started = 0.0
        self.deadline = 0.0
        self.minutes = 10
        self.mode = "ask"
        self.who = "Agent"
        self.scope = "mouse"

    def _config(self):
        cfg = self.settings()
        mode = cfg.get("mode")
        self.mode = mode if mode in ("ask", "notify", "off") else "ask"
        self.minutes = max(1, min(240, int(cfg.get("grant_minutes") or 10)))
        return max(2, min(60, int(cfg.get("countdown") or 8)))

    def __call__(self, cmd, *args):
        if cmd == "ask":
            return self.ask(*args)
        if cmd == "status":
            return self.status()
        if cmd == "deny":
            return self.finish("denied")
        if cmd == "revoke":
            revoke()
            return self.finish("revoked")
        if cmd == "end":
            return self.finish("idle")
        return ""  # begin and allow have nothing to do here

    def ask(self, who="Agent", reason="", seconds=0, scope="mouse", detail=""):
        self.who = who or "Agent"
        self.started = time.time()
        countdown = self._config()
        self.scope = str(scope)
        now = time.time()
        covered = ("mouse" not in self.scope or _read_float(_grant_file()) > now) and ("keyboard" not in self.scope or _read_float(_grant_file("keys")) > now)
        if self.mode == "off" or covered:
            self.state = "active"
            self.notify()
            return "allowed"
        if not self.tool:
            return None
        seconds = max(countdown, min(120, int(seconds or 0)))
        self.deadline = time.time() + seconds
        self.state = "asking"
        self.proc = subprocess.Popen(command(self.tool, self.who, reason, seconds, self.minutes, scope, detail), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        return "asking"

    def status(self):
        if self.state == "active" and _read_float(_revoked_file()) > self.started:
            return self.finish("revoked")
        if self.state != "asking":
            return self.state
        if _read_float(_revoked_file()) > self.started:
            return self.finish("revoked")
        code = self.proc.poll()
        if code is None:
            if time.time() < self.deadline:
                return "asking"
            self.proc.kill()
            self.proc.wait()
            return self.decide("timeout")
        out = self.proc.stdout.read() if self.proc.stdout else ""
        return self.decide(answer(self.tool, code, out, self.minutes))

    def decide(self, verdict):
        if verdict == "timeout" and self.mode == "notify":
            verdict = "allow"
        if verdict == "grant":
            runtime_dir().mkdir(parents=True, exist_ok=True)
            for kind, word in (("", "mouse"), ("keys", "keyboard")):
                if word in self.scope:
                    _grant_file(kind).write_text(f"{time.time() + self.minutes * 60}\n")
        if verdict in ("allow", "grant"):
            self.state = "active"
            self.notify()
        else:
            self.state = "timeout" if verdict == "timeout" else "denied"
        self.proc = None
        return self.state

    def finish(self, state):
        if self.proc and self.proc.poll() is None:
            self.proc.kill()
            self.proc.wait()
        self.proc = None
        self.state = state
        return state

    def notify(self):
        if shutil.which("notify-send"):
            subprocess.Popen(["notify-send", "-a", "Ghosthand", "-i", "input-mouse", "Agent Control",
                              f"{self.who} is using your {self.scope}. Run `ghosthand revoke` to take it back."], stderr=subprocess.DEVNULL)
