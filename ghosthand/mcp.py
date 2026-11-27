"""A small MCP server over stdio (one JSON-RPC message per line) so an agent can drive the pointer through Ghosthand.

It speaks both eras of the protocol, chosen from the first message:
  2026-07-28   stateless: no `initialize`; every request carries `_meta["io.modelcontextprotocol/protocolVersion"]`; `server/discover`
               describes the server; results carry `resultType`, and list results `ttlMs` and `cacheScope`.
  2025-11-25   (and 2025-06-18, 2025-03-26, 2024-11-05) `initialize` negotiates a version, then `notifications/initialized`.
Tools:
  pointer      run actions (mouse and keyboard), behind the gate: the person sees a card and decides. `reason` is required.
  screenshot   a PNG of the screen (or a region). Needs no approval: it never touches the pointer.
  screen_info  the output's name, size and scale, to turn screenshot pixels into positions.
  gate_status  idle | asking | active | allowed | denied | timeout | revoked
There is no tool to allow a request or to switch the gate off, on purpose: answering is the person's job.
Python standard library only. Logs go to stderr, never stdout."""
import base64
import json
import sys

from . import __version__
from . import gate as g
from .config import load
from .geometry import output_info
from .screen import screenshot_bytes
from .session import execute

META = "io.modelcontextprotocol/"
LATEST = "2026-07-28"
LEGACY = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"]
SERVER_INFO = {"name": "ghosthand", "title": "Ghosthand", "version": __version__}
INSTRUCTIONS = (
    "Ghosthand drives the person's real mouse and keyboard. Take a screenshot first, work out positions in screenshot pixels, click a field before typing, then use `pointer` with a "
    "clear `reason`: the person sees a card and must allow it. If it is denied or unanswered, stop and tell them; never retry in a loop. "
    "After acting, take another screenshot to check what happened."
)

ACTIONS_HELP = (
    "jump X Y, move X Y (glides, so hover follows), click [X Y], dclick [X Y], rclick [X Y], mclick [X Y], down, up, rdown, rup, "
    "drag X1 Y1 X2 Y2, rdrag X1 Y1 X2 Y2, scroll N [X Y] (positive scrolls the content down), hscroll N [X Y], wait SECONDS, "
    "key COMBO... (e.g. ctrl+s, Return, Tab), keydown COMBO, keyup COMBO, type TEXT (\\n is Enter, \\t is Tab), paste TEXT, "
    "focus (the focused window), shot FILE [X Y W H], pos, info"
)

TOOLS = [
    {
        "name": "pointer",
        "title": "Use the mouse and keyboard",
        "description": ("Move and click the person's real mouse pointer and press keys or type on their real keyboard (click a field first: keys go to the focused window). A card appears on their screen and nothing moves until they allow it; "
                        "if they deny or don't answer you get an error: stop and tell them, don't retry in a loop. "
                        "Coordinates are screenshot pixels. Actions: " + ACTIONS_HELP + "."),
        "inputSchema": {
            "type": "object",
            "properties": {
                "actions": {"type": "array", "items": {"type": "string"}, "minItems": 1, "description": "One action per item, e.g. [\"move 800 40\", \"click\"]"},
                "reason": {"type": "string", "minLength": 1, "description": "Why you need the pointer; shown on the card"},
                "who": {"type": "string", "description": "Who is asking (default: Agent)"},
                "countdown": {"type": "integer", "minimum": 0, "description": "Seconds the card waits (the person's setting, or longer)"},
            },
            "required": ["actions", "reason"],
            "additionalProperties": False,
        },
        "outputSchema": {
            "type": "object",
            "properties": {
                "ok": {"type": "boolean"},
                "code": {"type": "integer", "description": "0 done, 1 failed or locked, 2 bad usage, 3 taken back, 4 denied or no answer, 5 nothing to ask with"},
                "message": {"type": "string"},
                "printed": {"type": "array", "items": {"type": "string"}, "description": "What pos and info printed"},
            },
            "required": ["ok", "code", "message", "printed"],
        },
        "annotations": {"title": "Use the mouse and keyboard", "readOnlyHint": False, "destructiveHint": True, "idempotentHint": False, "openWorldHint": True},
    },
    {
        "name": "screenshot",
        "title": "Take a screenshot",
        "description": "A PNG of the screen, or of the region [x, y, width, height] in screenshot pixels. Needs no approval.",
        "inputSchema": {
            "type": "object",
            "properties": {"region": {"type": "array", "items": {"type": "integer"}, "minItems": 4, "maxItems": 4, "description": "[x, y, width, height]"}},
            "additionalProperties": False,
        },
        "annotations": {"title": "Take a screenshot", "readOnlyHint": True, "destructiveHint": False, "idempotentHint": True, "openWorldHint": False},
    },
    {
        "name": "screen_info",
        "title": "Screen size and scale",
        "description": "The output's name, size in screenshot pixels and scale.",
        "inputSchema": {"type": "object", "additionalProperties": False},
        "outputSchema": {
            "type": "object",
            "properties": {"name": {"type": "string"}, "width": {"type": "integer"}, "height": {"type": "integer"}, "scale": {"type": "number"}},
            "required": ["name", "width", "height", "scale"],
        },
        "annotations": {"title": "Screen size and scale", "readOnlyHint": True, "destructiveHint": False, "idempotentHint": True, "openWorldHint": False},
    },
    {
        "name": "gate_status",
        "title": "Gate status",
        "description": "The gate's state: idle, asking, active, allowed, denied, timeout or revoked.",
        "inputSchema": {"type": "object", "additionalProperties": False},
        "outputSchema": {"type": "object", "properties": {"state": {"type": "string"}}, "required": ["state"]},
        "annotations": {"title": "Gate status", "readOnlyHint": True, "destructiveHint": False, "idempotentHint": True, "openWorldHint": False},
    },
]


def reply(text, error=False, structured=None):
    out = {"content": [{"type": "text", "text": text}], "isError": error}
    if structured is not None:
        out["structuredContent"] = structured
    return out


def call_tool(name, args, run=execute):
    """The result of one tool call: { content, isError, structuredContent? }. Raises KeyError for a tool that doesn't exist."""
    args = args or {}
    if name == "pointer":
        actions = args.get("actions")
        if not isinstance(actions, list) or not actions or not all(isinstance(a, str) for a in actions):
            return reply("actions must be a list of strings", True)
        if not str(args.get("reason", "")).strip():
            return reply("reason is required: say why you need the pointer, the person reads it on the card", True)
        try:
            countdown = int(args.get("countdown") or 0)
        except (TypeError, ValueError):
            return reply("countdown must be a whole number of seconds", True)
        result = run(str(args.get("who") or "Agent"), str(args["reason"]), countdown, actions)
        text = "\n".join([*result.printed, result.message]).strip() or "done"
        return reply(text, result.code != 0, {"ok": result.code == 0, "code": result.code, "message": result.message, "printed": list(result.printed)})
    if name == "screenshot":
        region = args.get("region")
        if region is not None and not (isinstance(region, list) and len(region) == 4 and all(isinstance(v, int) for v in region)):
            return reply("region must be four integers: [x, y, width, height]", True)
        try:
            png = screenshot_bytes(region)
        except Exception as e:  # noqa: BLE001 - any failure goes back to the agent as a tool error
            return reply(f"screenshot failed: {e}", True)
        return {"content": [{"type": "image", "data": base64.b64encode(png).decode(), "mimeType": "image/png"}], "isError": False}
    if name == "screen_info":
        try:
            info = output_info(load().get("output"))
        except Exception as e:  # noqa: BLE001
            return reply(f"couldn't read the output: {e}", True)
        return reply(json.dumps(info), False, info)
    if name == "gate_status":
        state = g.default_ask()("status")
        if state is None:
            return reply("nothing to ask with: run `ghosthand overlay`, or install zenity, yad or kdialog", True)
        return reply(state, False, {"state": state})
    raise KeyError(name)


def error(ident, code, message, data=None):
    err = {"code": code, "message": message}
    if data is not None:
        err["data"] = data
    return {"jsonrpc": "2.0", "id": ident, "error": err}


class Server:
    def __init__(self, run=execute):
        self.run = run
        self.legacy = False  # set by `initialize`: this process then speaks 2025-11-25 style

    def handle(self, message):
        """One JSON-RPC message in, the response out (None for a notification)."""
        if not isinstance(message, dict):
            return error(None, -32600, "invalid request")
        method, ident = message.get("method"), message.get("id")
        if ident is None:  # a notification (initialized, cancelled, ...): nothing to answer
            return None
        params = message.get("params") or {}
        if method == "initialize":
            return self.initialize(ident, params)
        version = (params.get("_meta") or {}).get(META + "protocolVersion")
        if version is not None and version != LATEST:
            return error(ident, -32022, "Unsupported protocol version", {"supported": [LATEST, *LEGACY], "requested": version})
        if version is None and not self.legacy:
            return error(ident, -32602, f"missing params._meta[{META}protocolVersion] (or call initialize first)")
        modern = version is not None
        try:
            result = self.dispatch(method, params, modern)
        except KeyError as e:
            return error(ident, -32602, f"Unknown tool: {e.args[0]}")
        except Exception as e:  # noqa: BLE001 - never let one request take the server down
            print(f"ghosthand mcp: {method} failed: {e}", file=sys.stderr)
            return error(ident, -32603, "internal error")
        if result is None:
            return error(ident, -32601, f"method not found: {method}")
        if modern:
            result = {"resultType": "complete", **result, "_meta": {**result.get("_meta", {}), META + "serverInfo": {"name": "ghosthand", "version": __version__}}}
        return {"jsonrpc": "2.0", "id": ident, "result": result}

    def initialize(self, ident, params):
        asked = params.get("protocolVersion")
        self.legacy = True
        version = asked if asked in LEGACY else LEGACY[0]
        return {"jsonrpc": "2.0", "id": ident, "result": {"protocolVersion": version, "capabilities": {"tools": {"listChanged": False}},
                                                          "serverInfo": SERVER_INFO, "instructions": INSTRUCTIONS}}

    def dispatch(self, method, params, modern):
        if method == "server/discover" and modern:
            return {"supportedVersions": [LATEST], "capabilities": {"tools": {"listChanged": False}}, "instructions": INSTRUCTIONS,
                    "_meta": {META + "serverInfo": SERVER_INFO}, "ttlMs": 3600000, "cacheScope": "public"}
        if method == "ping":
            return {}
        if method == "tools/list":
            return {"tools": TOOLS, **({"ttlMs": 3600000, "cacheScope": "public"} if modern else {})}
        if method == "tools/call":
            return call_tool(params.get("name", ""), params.get("arguments"), self.run)
        return None


def handle(message, run=execute, server=None):
    """One message through a fresh (or given) server. Kept as a function so callers and tests don't need the class."""
    return (server or Server(run)).handle(message)


def serve(stdin=sys.stdin, stdout=sys.stdout):
    server = Server()
    for line in stdin:  # stdin closing is the shutdown signal
        line = line.strip()
        if not line:
            continue
        try:
            out = server.handle(json.loads(line))
        except json.JSONDecodeError:
            out = error(None, -32700, "parse error")
        if out is not None:
            stdout.write(json.dumps(out, separators=(",", ":")) + "\n")
            stdout.flush()
    return 0
