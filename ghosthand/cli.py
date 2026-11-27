"""The `ghosthand` command."""
import json
import subprocess
import sys

from . import __version__
from . import actions as act
from . import gate as g
from .config import load
from .geometry import output_info
from .session import execute

HELP = f"""ghosthand {__version__}: a real hand on a real pointer and keyboard for coding agents, with a gate in front of them.

usage:
  ghosthand [--who NAME] [--reason TEXT] [--countdown N] ACTION...    drive the pointer (each action is one quoted argument)
  ghosthand overlay                 run the overlay: the allow/deny card and the cursor tag (Quickshell; X11/GNOME use a dialog instead)
  ghosthand revoke                  take the pointer back now, from anywhere (bind it to a key)
  ghosthand status                  idle | asking | active | allowed | denied | timeout | revoked
  ghosthand mcp                     serve the tools to an agent over MCP (stdio)
  ghosthand skill [install [DIR]]   print the agent skill, or install it (default DIR: ~/.claude/skills)
  ghosthand setup                   install the overlay into ~/.config/quickshell and write a default config
  ghosthand doctor                  check what's needed: uinput, grim, Quickshell, niri, the overlay
  ghosthand --version

options:
  --who NAME        who is asking, shown on the card and the tag (default: Agent)
  --reason TEXT     why, shown on the card ("Testing the Tabs panel")
  --countdown N     seconds the card waits (default: the overlay's setting, 8)

actions (coordinates are SCREENSHOT PIXELS of the output, the ones `grim` writes; `info` prints the scale):
  jump X Y             put the pointer there at once
  move X Y             glide there in small steps (use it before a press so hover follows)
  click [X Y]          left click          dclick [X Y]  double click      rclick [X Y]  right click      mclick [X Y]  middle click
  down | up            left button press | release         rdown | rup    right button
  drag X1 Y1 X2 Y2     press at 1, glide to 2, release     rdrag X1 Y1 X2 Y2  the same with the right button
  scroll N [X Y]       wheel: N notches, positive scrolls the content down, negative up       hscroll N [X Y]  sideways
  wait SECONDS         sleep (animations, panels opening); a revoke still stops it
  shot FILE [X Y W H]  screenshot of the output, or of that region
  pos                  print where the pointer is (screenshot pixels)
  info                 print the output's name, size and scale as JSON
  focus                print the focused window as JSON (where keys will land)
  key COMBO...         press keys and chords in order: key ctrl+s   key Escape   key ctrl+shift+t   key Tab Tab Return
  keydown COMBO        hold keys (release with keyup): keydown shift, click, keyup shift
  type TEXT            type text with your real layout (\\n is Enter, \\t is Tab): type git status\\n
  paste TEXT           clipboard + Ctrl+V, for text `type` can't make (emoji); the clipboard is put back

example:
  ghosthand --who Claude --reason "Checking the bar menu" 'jump 800 26' 'wait 0.4' 'rclick' 'wait 0.8' 'shot /tmp/menu.png'

exit codes: 0 done, 1 an action failed or the session is locked, 2 bad usage, 3 taken back, 4 denied or no answer, 5 the overlay can't be reached.
"""


def run_overlay():
    """Run the overlay in the foreground (autostart it, or use the systemd user unit in packaging/)."""
    return subprocess.call(["qs", "-c", load()["overlay"]])


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if not argv or argv[0] in ("-h", "--help", "help"):
        print(HELP)
        return 0
    if argv[0] in ("-V", "--version", "version"):
        print(f"ghosthand {__version__}")
        return 0
    head = argv[0]
    if head == "overlay":
        return run_overlay()
    if head == "revoke":
        from . import dialog
        dialog.revoke()  # works with no overlay at all: a running agent notices on its next step
        g.qs("revoke")  # and the overlay, if there is one, closes its card and ends its grant
        print("pointer taken back")
        return 0
    if head == "status":
        out = g.default_ask()("status")
        print(out if out is not None else "unreachable")
        return 0 if out is not None else g.EXIT_NO_GATE
    if head == "setup":
        from .doctor import setup
        return setup()
    if head == "doctor":
        from .doctor import doctor
        return doctor()
    if head == "skill":
        from .doctor import skill
        return skill(argv[1:])
    if head == "mcp":
        from .mcp import serve
        return serve()
    try:
        opts, action_list = act.parse_args(argv)
    except ValueError as e:
        print(e, file=sys.stderr)
        return g.EXIT_USAGE
    result = execute(opts["who"], opts["reason"], opts["countdown"], action_list)
    for line in result.printed:
        print(line)
    if result.message:
        print(result.message, file=sys.stderr)
    return result.code
