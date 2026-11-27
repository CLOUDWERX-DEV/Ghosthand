import io
import json
import unittest

from ghosthand import mcp
from ghosthand.session import Result


class Handshake(unittest.TestCase):
    def ask(self, method, params=None, ident=1, run=None):
        return mcp.handle({"jsonrpc": "2.0", "id": ident, "method": method, "params": params or {}}, run=run or (lambda *a: Result()))

    def test_initialize_lists_tools(self):
        r = self.ask("initialize")["result"]
        self.assertEqual(r["serverInfo"]["name"], "ghosthand")
        self.assertIn("tools", r["capabilities"])
        names = [t["name"] for t in self.ask("tools/list")["result"]["tools"]]
        self.assertEqual(names, ["pointer", "screenshot", "screen_info", "gate_status"])

    def test_there_is_no_way_to_allow_or_switch_the_gate_off(self):
        names = " ".join(t["name"] + t["description"].lower() for t in mcp.TOOLS)
        for banned in ("allow a request", "approve", "disable the gate", "gate off"):
            self.assertNotIn(banned, names)
        self.assertEqual({t["name"] for t in mcp.TOOLS}, {"pointer", "screenshot", "screen_info", "gate_status"})

    def test_notifications_get_no_answer_and_unknown_methods_an_error(self):
        self.assertIsNone(mcp.handle({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        self.assertEqual(self.ask("nope")["error"]["code"], -32601)

    def test_the_serve_loop_answers_line_by_line(self):
        stdin = io.StringIO('{"jsonrpc":"2.0","id":1,"method":"ping"}\n\nnot json\n{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
        stdout = io.StringIO()
        mcp.serve(stdin, stdout)
        lines = [json.loads(x) for x in stdout.getvalue().splitlines()]
        self.assertEqual(lines[0]["result"], {})
        self.assertEqual(lines[1]["error"]["code"], -32700)
        self.assertEqual(len(lines), 2)


class PointerTool(unittest.TestCase):
    def test_it_needs_actions_and_a_reason(self):
        self.assertTrue(mcp.call_tool("pointer", {"reason": "x"})["isError"])
        self.assertTrue(mcp.call_tool("pointer", {"actions": ["click"], "reason": "  "})["isError"])
        self.assertTrue(mcp.call_tool("pointer", {"actions": [1], "reason": "x"})["isError"])

    def test_it_passes_who_reason_and_actions_and_reports_the_outcome(self):
        seen = []

        def run(who, reason, countdown, actions):
            seen.append((who, reason, countdown, actions))
            return Result(4, "Ghosthand: not allowed (denied).")

        r = mcp.call_tool("pointer", {"actions": ["move 1 2", "click"], "reason": "testing", "who": "Claude", "countdown": 5}, run)
        self.assertEqual(seen, [("Claude", "testing", 5, ["move 1 2", "click"])])
        self.assertTrue(r["isError"])
        self.assertIn("denied", r["content"][0]["text"])

    def test_success_returns_what_pos_printed(self):
        r = mcp.call_tool("pointer", {"actions": ["pos"], "reason": "x"}, lambda *a: Result(0, "", ['{"x": 1}']))
        self.assertFalse(r["isError"])
        self.assertIn('"x": 1', r["content"][0]["text"])

    def test_unknown_tool_is_an_error(self):
        self.assertTrue(mcp.call_tool("rm", {})["isError"])


if __name__ == "__main__":
    unittest.main()
