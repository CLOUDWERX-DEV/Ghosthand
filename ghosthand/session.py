"""A session: ask the gate, move the pointer through a list of actions, always put everything back.

Used by the command line and by the MCP server, so both behave the same and share every safety."""
import subprocess
from dataclasses import dataclass, field

from . import actions as act
from . import gate as g
from .config import load
from .geometry import output_info
from .uinput import Pointer


@dataclass
class Result:
    code: int = 0
    message: str = ""
    printed: list = field(default_factory=list)  # what pos and info printed


def execute(who, reason, countdown, action_list, make_pointer=Pointer, ask=g.qs):
    """Run the actions. Returns a Result whose code is the exit code (0 done, see gate.py for the rest)."""
    result = Result()
    try:
        act.validate(action_list)
    except ValueError as e:
        return Result(g.EXIT_USAGE, str(e))
    if not action_list:
        return Result(g.EXIT_USAGE, "no actions given")
    info = output_info(load().get("output"))
    if not act.touches_pointer(action_list):  # nothing touches the pointer: no gate, no device
        try:
            for a in action_list:
                act.run(None, a, lambda: None, info, out=result.printed.append)
        except (ValueError, subprocess.CalledProcessError, OSError) as e:
            return Result(g.EXIT_FAILED, str(e))
        return result
    if g.locked():
        return Result(g.EXIT_FAILED, "the session is locked: not touching the pointer")
    try:
        g.gate(who, reason, countdown, ask=ask)
    except g.GateError as e:
        return Result(e.code, str(e))
    write, close_stream = g.stream_writer()
    write(f"begin {who}\n")
    pointer, done = None, 0

    def announce(x, y):
        write(f"pos {x:.1f} {y:.1f} {info['name']}\n")

    try:
        pointer = make_pointer(info, announce)
        for a in action_list:
            act.run(pointer, a, lambda: g.check_not_revoked(ask), info, out=result.printed.append)
            done += 1
    except g.Revoked:
        result.code, result.message = g.EXIT_REVOKED, f"Ghosthand: taken back after {done} of {len(action_list)} actions."
    except (ValueError, subprocess.CalledProcessError, OSError) as e:
        result.code, result.message = g.EXIT_FAILED, f"action {done + 1} failed: {e}"
    finally:
        if pointer:
            pointer.close()
        write("end\n")
        close_stream()
        ask("end")
    return result
