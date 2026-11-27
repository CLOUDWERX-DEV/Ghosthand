import io
import json
import unittest

from ghosthand import mcp
from ghosthand.session import Result

K = mcp.META + "protocolVersion"


def modern(method, params=None, ident=1, version=mcp.LATEST):
    p = dict(params or {})
    p["_meta"] = {K: version, mcp.META + "clientCapabilities": {}}
    return {"jsonrpc": "2.0", "id": ident, "method": method, "params": p}


def server(run=None):
    return mcp.Server(run or (lambda *a: Result()))


class Modern(unittest.TestCase):
    """2026-07-28: stateless, no initialize, `_meta` on every request."""

    def test_discover_describes_the_server(self):
        r = server().handle(modern("server/discover"))["result"]
        self.assertEqual(r["resultType"], "complete")
        self.assertEqual(r["supportedVersions"], [mcp.LATEST])
        self.assertIn("tools", r["capabilities"])
        self.assertIn("ttlMs", r)
        self.assertIn(r["cacheScope"], ("public", "private"))
        self.assertEqual(r["_meta"][mcp.META + "serverInfo"]["name"], "ghosthand")

    def test_every_result_says_complete_and_carries_server_info(self):
        r = server().handle(modern("tools/list"))["result"]
        self.assertEqual(r["resultType"], "complete")
        self.assertEqual(r["_meta"][mcp.META + "serverInfo"]["name"], "ghosthand")
        self.assertIn("ttlMs", r)

    def test_a_request_without_meta_is_malformed(self):
        r = server().handle({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})
        self.assertEqual(r["error"]["code"], -32602)

    def test_an_unknown_version_gets_the_supported_list(self):
        r = server().handle(modern("tools/list", version="1900-01-01"))
        self.assertEqual(r["error"]["code"], -32022)
        self.assertEqual(r["error"]["data"]["requested"], "1900-01-01")
        self.assertIn(mcp.LATEST, r["error"]["data"]["supported"])
        self.assertIn("2025-11-25", r["error"]["data"]["supported"])

    def test_unknown_methods_and_tools_are_protocol_errors(self):
        self.assertEqual(server().handle(modern("nope"))["error"]["code"], -32601)
        r = server().handle(modern("tools/call", {"name": "rm", "arguments": {}}))
        self.assertEqual(r["error"]["code"], -32602)

    def test_the_server_never_sends_requests_and_notifications_get_no_answer(self):
        self.assertIsNone(server().handle({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": 1}}))


class Legacy(unittest.TestCase):
    """2025-11-25 and older: initialize, then everything else."""

    def test_initialize_negotiates_the_version(self):
        s = server()
        r = s.handle({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}}})["result"]
        self.assertEqual(r["protocolVersion"], "2025-06-18")
        self.assertEqual(r["serverInfo"]["name"], "ghosthand")
        newer = s.handle({"jsonrpc": "2.0", "id": 2, "method": "initialize", "params": {"protocolVersion": "2030-01-01"}})["result"]
        self.assertEqual(newer["protocolVersion"], mcp.LEGACY[0], "an unknown version gets the newest one we know")

    def test_after_initialize_requests_need_no_meta_and_results_stay_plain(self):
        s = server()
        s.handle({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-11-25"}})
        r = s.handle({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})["result"]
        self.assertNotIn("resultType", r)
        self.assertEqual([t["name"] for t in r["tools"]], ["pointer", "screenshot", "screen_info", "gate_status"])
        self.assertEqual(s.handle({"jsonrpc": "2.0", "id": 3, "method": "ping"})["result"], {})


class Tools(unittest.TestCase):
    def test_every_tool_is_well_formed(self):
        for t in mcp.TOOLS:
            self.assertRegex(t["name"], r"^[A-Za-z0-9_.-]{1,128}$")
            self.assertEqual(t["inputSchema"]["type"], "object")
            self.assertTrue(t["title"] and t["description"])
            if "outputSchema" in t:
                self.assertEqual(t["outputSchema"]["type"], "object")
            for hint in ("readOnlyHint", "destructiveHint", "idempotentHint", "openWorldHint"):
                self.assertIn(hint, t["annotations"])
        self.assertEqual([t["name"] for t in mcp.TOOLS], ["pointer", "screenshot", "screen_info", "gate_status"], "a stable order")

    def test_the_pointer_is_marked_as_acting_and_the_rest_as_read_only(self):
        hints = {t["name"]: t["annotations"]["readOnlyHint"] for t in mcp.TOOLS}
        self.assertEqual(hints, {"pointer": False, "screenshot": True, "screen_info": True, "gate_status": True})

    def test_there_is_no_way_to_allow_or_switch_the_gate_off(self):
        self.assertEqual({t["name"] for t in mcp.TOOLS}, {"pointer", "screenshot", "screen_info", "gate_status"})
        for t in mcp.TOOLS:
            self.assertNotIn("approve", t["description"].lower())
            self.assertNotIn("mode", t["inputSchema"].get("properties", {}))

    def test_pointer_needs_actions_and_a_reason(self):
        self.assertTrue(mcp.call_tool("pointer", {"reason": "x"})["isError"])
        self.assertTrue(mcp.call_tool("pointer", {"actions": ["click"], "reason": "  "})["isError"])
        self.assertTrue(mcp.call_tool("pointer", {"actions": [1], "reason": "x"})["isError"])
        self.assertTrue(mcp.call_tool("pointer", {"actions": ["click"], "reason": "x", "countdown": "soon"})["isError"])

    def test_pointer_passes_who_reason_and_actions_and_reports_structured_output(self):
        seen = []

        def run(who, reason, countdown, actions):
            seen.append((who, reason, countdown, actions))
            return Result(4, "Ghosthand: not allowed (denied).")

        r = mcp.call_tool("pointer", {"actions": ["move 1 2", "click"], "reason": "testing", "who": "Claude", "countdown": 5}, run)
        self.assertEqual(seen, [("Claude", "testing", 5, ["move 1 2", "click"])])
        self.assertTrue(r["isError"])
        self.assertEqual(r["structuredContent"], {"ok": False, "code": 4, "message": "Ghosthand: not allowed (denied).", "printed": []})
        self.assertIn("denied", r["content"][0]["text"])

    def test_success_returns_what_pos_printed(self):
        r = mcp.call_tool("pointer", {"actions": ["pos"], "reason": "x"}, lambda *a: Result(0, "", ['{"x": 1}']))
        self.assertFalse(r["isError"])
        self.assertIn('"x": 1', r["content"][0]["text"])
        self.assertEqual(r["structuredContent"]["printed"], ['{"x": 1}'])

    def test_a_bad_region_is_a_tool_error_the_agent_can_fix(self):
        self.assertTrue(mcp.call_tool("screenshot", {"region": [1, 2, 3]})["isError"])

    def test_structured_output_matches_its_schema_keys(self):
        from ghosthand import gate
        real = gate.default_ask
        gate.default_ask = lambda: (lambda *a: "idle")
        try:
            r = mcp.call_tool("gate_status", {})
        finally:
            gate.default_ask = real
        self.assertEqual(r["structuredContent"], {"state": "idle"})
        self.assertEqual(r["content"][0]["text"], "idle")


class Serve(unittest.TestCase):
    def test_the_loop_answers_line_by_line_and_stops_at_eof(self):
        stdin = io.StringIO("\n".join([json.dumps(modern("tools/list")), "", "not json", '{"jsonrpc":"2.0","method":"notifications/initialized"}', ""]))
        stdout = io.StringIO()
        self.assertEqual(mcp.serve(stdin, stdout), 0)
        lines = [json.loads(x) for x in stdout.getvalue().splitlines()]
        self.assertEqual(len(lines), 2)
        self.assertEqual(lines[0]["result"]["resultType"], "complete")
        self.assertEqual(lines[1]["error"]["code"], -32700)
        for raw in stdout.getvalue().splitlines():
            self.assertNotIn("\n", raw)


if __name__ == "__main__":
    unittest.main()
