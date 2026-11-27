import os
import unittest

from ghosthand import session
from ghosthand import gate as g


class FakePointer:
    instances = []

    def __init__(self, info, announce):
        self.announce, self.log, self.closed = announce, [], False
        self.x = self.y = 0
        self.known = True
        FakePointer.instances.append(self)

    def put(self, x, y):
        self.log.append(("put", x, y))
        self.announce(x, y)

    glide = put

    def click(self, code, times=1): self.log.append(("click",))
    def close(self): self.closed = True


class Session(unittest.TestCase):
    def setUp(self):
        os.environ["GHOSTHAND_OUTPUT_SIZE"], os.environ["GHOSTHAND_SCALE"] = "1000x800", "1.0"
        os.environ["XDG_RUNTIME_DIR"] = __import__("tempfile").mkdtemp()
        self._locked, self._gate = g.locked, g.gate
        g.locked = lambda command=None: False
        FakePointer.instances.clear()

    def tearDown(self):
        g.locked, g.gate = self._locked, self._gate
        for k in ("GHOSTHAND_OUTPUT_SIZE", "GHOSTHAND_SCALE", "XDG_RUNTIME_DIR"):
            os.environ.pop(k, None)

    def ask(self, answers):
        it = iter(answers)
        return lambda *args: next(it, "active")

    def test_a_typo_never_reaches_the_gate_or_the_pointer(self):
        r = session.execute("Claude", "x", 0, ["jump 1 2", "clikc"], make_pointer=FakePointer, ask=lambda *a: self.fail("asked"))
        self.assertEqual(r.code, g.EXIT_USAGE)
        self.assertEqual(FakePointer.instances, [])

    def test_a_denied_request_never_makes_a_device(self):
        r = session.execute("Claude", "x", 0, ["jump 1 2"], make_pointer=FakePointer, ask=self.ask(["asking", "denied"]))
        self.assertEqual(r.code, g.EXIT_DENIED)
        self.assertEqual(FakePointer.instances, [])

    def test_an_unreachable_overlay_is_refused(self):
        r = session.execute("Claude", "x", 0, ["jump 1 2"], make_pointer=FakePointer, ask=lambda *a: None)
        self.assertEqual(r.code, g.EXIT_NO_GATE)
        self.assertEqual(FakePointer.instances, [])

    def test_a_locked_session_is_refused(self):
        g.locked = lambda command=None: True
        r = session.execute("Claude", "x", 0, ["jump 1 2"], make_pointer=FakePointer, ask=lambda *a: "allowed")
        self.assertEqual(r.code, g.EXIT_FAILED)
        self.assertEqual(FakePointer.instances, [])

    def test_an_allowed_run_moves_streams_and_always_cleans_up(self):
        calls = []

        def ask(*args):
            calls.append(args[0])
            return "allowed" if args[0] == "ask" else "active"
        r = session.execute("Claude", "x", 0, ["jump 10 20", "click"], make_pointer=FakePointer, ask=ask)
        self.assertEqual(r.code, 0)
        p = FakePointer.instances[0]
        self.assertEqual(p.log[0], ("put", 10.0, 20.0))
        self.assertTrue(p.closed)
        self.assertEqual(calls[-1], "end", "the overlay is told the run ended")
        with open(os.path.join(os.environ["XDG_RUNTIME_DIR"], "ghosthand", "stream")) as f:
            stream = f.read().splitlines()
        self.assertEqual(stream[0], "begin Claude")
        self.assertTrue(stream[1].startswith("pos 10.0 20.0"))
        self.assertEqual(stream[-1], "end")

    def test_a_revoke_mid_run_stops_it_and_still_cleans_up(self):
        answers = iter(["allowed", "active", "revoked"])
        r = session.execute("Claude", "x", 0, ["jump 1 1", "jump 2 2", "jump 3 3"], make_pointer=FakePointer, ask=lambda *a: next(answers, "revoked"))
        self.assertEqual(r.code, g.EXIT_REVOKED)
        self.assertIn("taken back", r.message)
        self.assertTrue(FakePointer.instances[0].closed)

    def test_shot_wait_and_info_alone_need_no_gate(self):
        r = session.execute("Claude", "x", 0, ["info"], make_pointer=FakePointer, ask=lambda *a: self.fail("asked"))
        self.assertEqual(r.code, 0)
        self.assertIn('"scale": 1.0', r.printed[0])


if __name__ == "__main__":
    unittest.main()
