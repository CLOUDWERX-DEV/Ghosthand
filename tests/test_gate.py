import unittest

from ghosthand import gate as g


def fake(answers):
    """An `ask` that returns the given answers in order (None = the overlay is unreachable) and records its calls."""
    calls, it = [], iter(answers)

    def ask(*args):
        calls.append(args)
        return next(it)
    return ask, calls


class Gate(unittest.TestCase):
    def test_a_standing_grant_goes_straight_in(self):
        ask, calls = fake(["allowed"])
        g.gate("Claude", "why", 5, ask=ask, sleep=lambda s: None)
        self.assertEqual(calls, [("ask", "Claude", "why", 5, "mouse", "")])

    def test_it_waits_for_the_answer_and_polls(self):
        ask, calls = fake(["asking", "asking", "asking", "allowed"])
        g.gate("Claude", "", 5, ask=ask, sleep=lambda s: None)
        self.assertEqual([c[0] for c in calls], ["ask", "status", "status", "status"])

    def test_allow_shows_as_active_and_that_counts(self):
        ask, calls = fake(["asking", "asking", "active"])
        g.gate("Claude", "", 5, ask=ask, sleep=lambda s: None)
        self.assertEqual([c[0] for c in calls], ["ask", "status", "status"])

    def test_denied_timeout_and_revoked_are_all_no(self):
        for verdict in ("denied", "timeout", "revoked"):
            ask, _ = fake(["asking", verdict])
            with self.assertRaises(g.GateError) as e:
                g.gate("Claude", "", 5, ask=ask, sleep=lambda s: None)
            self.assertEqual(e.exception.code, g.EXIT_DENIED, verdict)

    def test_an_unreachable_overlay_is_refused_not_skipped(self):
        for answers in ([None], ["asking", None]):
            ask, _ = fake(answers)
            with self.assertRaises(g.GateError) as e:
                g.gate("Claude", "", 5, ask=ask, sleep=lambda s: None)
            self.assertEqual(e.exception.code, g.EXIT_NO_GATE)

    def test_no_answer_in_time_denies_the_request(self):
        clock = iter(range(0, 1000, 10))
        ask, calls = fake(["asking"] * 50)
        with self.assertRaises(g.GateError) as e:
            g.gate("Claude", "", 2, ask=ask, sleep=lambda s: None, now=lambda: next(clock), patience=30)
        self.assertEqual(e.exception.code, g.EXIT_DENIED)
        self.assertEqual(calls[-1][0], "deny", "the card is closed when the wait ends")

    def test_a_revoke_is_noticed_between_steps(self):
        for state in ("revoked", "denied", "idle", "timeout"):
            ask, _ = fake([state])
            with self.assertRaises(g.Revoked):
                g.check_not_revoked(ask)
        ask, _ = fake(["active"])
        g.check_not_revoked(ask)

    def test_the_lock_check_refuses_when_it_cannot_tell(self):
        self.assertFalse(g.locked("echo no"))
        self.assertTrue(g.locked("echo yes"))
        self.assertTrue(g.locked("echo"), "no answer counts as locked")
        self.assertTrue(g.locked("exit 3"))


if __name__ == "__main__":
    unittest.main()
