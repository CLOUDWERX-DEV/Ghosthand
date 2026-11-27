"""A small MCP server (stdio, newline-delimited JSON-RPC) so an agent can drive the pointer through Ghosthand.

Tools:
  pointer      run actions on the pointer, behind the gate: the person sees a card and decides. `reason` is required.
  screenshot   a PNG of the screen (or a region). Needs no approval: it never touches the pointer.
  screen_info  the output's name, size and scale, so you can turn screenshot pixels into positions.
  gate_status  idle | asking | active | allowed | denied | timeout | revoked

There is no tool to allow a request or to switch the gate off, on purpose: answering is the person's job.
Python standard library only."""
import base64
import json
import sys

from . import __version__
from . import actions as act
from . import gate as g
from .geometry import output_info
from .config import load
from .session import execute

PROTOCOL = "2024-11-05"

TOOLS = [
    {
        "name": "pointer",
        "description": ("Move and click the person's real mouse pointer. A card appears on their screen and nothing moves until they allow it; "
                       "if they deny or don't answer you get an error: stop and tell them, don't retry in a loop. Coordinates are screenshot pixels. "
                       "Actions: jump X Y, move X Y, click [X Y], dclick [X Y], rclick [X Y], mclick [X Y], down, up, rdown, rup, drag X1 Y1 X2 Y2, "
                       "rdrag X1 Y1 X2 Y2, scroll N [X Y], hscroll N [X Y], wait SECONDS, shot FILE [X Y W H], pos, info."),
        "inputSchema": {
            "type": "object",
            "properties": {
                "actions": {"type": "array", "items": {"type": "string"}, "minItems": 1, "description": "One action per item, e.g. [\"move 800 40\", \"click\"]"},
                "reason": {"type": "string", "description": "Why you need the pointer; shown on the card"},
                "who": {"type": "string", "description": "Who is asking (default: Agent)"},
                "countdown": {"type": "integer", "description": "Seconds the card waits (the person's setting, or longer)"},
            },
            "required": ["actions", "reason"],
        },
    },
    {
        "name": "screenshot",
        "description": "A PNG of the screen, or of the region [x, y, width, height] in screenshot pixels. Needs no approval.",
        "inputSchema": {"type": "object", "properties": {"region": {"type": "array", "items": {"type": "integer"}, "minItems": 4, "maxItems": 4}}},
    },
    {"name": "screen_info", "description": "The output's name, size in screenshot pixels and scale.", "inputSchema": {"type": "object", "properties": {}}},
    {"name": "gate_status", "description": "The gate's state: idle, asking, active, allowed, denied, timeout or revoked.", "inputSchema": {"type": "object", "properties": {}}},
]


def text(message, error=False):
    return {"content": [{"type": "text", "text": message}], "isError": error}


def call_tool(name, args, run=execute):
    """The result of one tool call: { content, isError }."""
    args = args or {}
    if name == "pointer":
        actions = args.get("actions")
        if not isinstance(actions, list) or not actions or not all(isinstance(a, str) for a in actions):
            return text("actions must be a list of strings", True)
        if not str(args.get("reason", "")).strip():
            return text("reason is required: say why you need the pointer, the person reads it on the card", True)
        result = run(str(args.get("who") or "Agent"), str(args["reason"]), int(args.get("countdown") or 0), actions)
        out = "\n".join([*result.printed, result.message]).strip() or "done"
        return text(out, result.code != 0)
    if name == "screenshot":
        region = args.get("region")
        try:
            png = act.screenshot_bytes(region if isinstance(region, list) and len(region) == 4 else None)
        except Exception as e:  # noqa: BLE001 - report any failure to the agent as a tool error
            return text(f"screenshot failed: {e}", True)
        return {"content": [{"type": "image", "data": base64.b64encode(png).decode(), "mimeType": "image/png"}], "isError": False}
    if name == "screen_info":
        try:
            return text(json.dumps(output_info(load().get("output"))))
        except Exception as e:  # noqa: BLE001
            return text(f"couldn't read the output: {e}", True)
    if name == "gate_status":
        state = g.qs("status")
        return text(state if state is not None else "unreachable: is `ghosthand overlay` running?", state is None)
    return text(f"unknown tool: {name}", True)


def handle(message, run=execute):
    """One JSON-RPC message in, the response out (None for a notification)."""
    method, ident = message.get("method"), message.get("id")
    if ident is None:  # a notification: nothing to answer
        return None
    if method == "initialize":
        result = {"protocolVersion": PROTOCOL, "capabilities": {"tools": {}}, "serverInfo": {"name": "ghosthand", "version": __version__}}
    elif method == "ping":
        result = {}
    elif method == "tools/list":
        result = {"tools": TOOLS}
    elif method == "tools/call":
        params = message.get("params") or {}
        result = call_tool(params.get("name", ""), params.get("arguments"), run)
    else:
        return {"jsonrpc": "2.0", "id": ident, "error": {"code": -32601, "message": f"method not found: {method}"}}
    return {"jsonrpc": "2.0", "id": ident, "result": result}


def serve(stdin=sys.stdin, stdout=sys.stdout):
    for line in stdin:
        line = line.strip()
        if not line:
            continue
        try:
            reply = handle(json.loads(line))
        except json.JSONDecodeError:
            reply = {"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": "parse error"}}
        if reply is not None:
            stdout.write(json.dumps(reply) + "\n")
            stdout.flush()
    return 0
