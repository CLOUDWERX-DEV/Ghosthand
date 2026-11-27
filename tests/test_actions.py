import json
import os
import tempfile
import time
import unittest

from ghosthand import actions as a
from ghosthand import gate as g
from ghosthand import uinput
from ghosthand.geometry import numbers, output_info


class Parsing(unittest.TestCase):
    def test_numbers(self):
        self.assertEqual(numbers(["10", "-5.5", "+3"]), [10.0, -5.5, 3.0])
        with self.assertRaises(ValueError):
            numbers(["12", "x"])

    def test_options_come_before_the_actions(self):
        opts, actions = a.parse_args(["--who", "Claude", "--reason", "testing", "click 1 2", "wait 1"])
        self.assertEqual(opts, {"who": "Claude", "reason": "testing", "countdown": 0})
        self.assertEqual(actions, ["click 1 2", "wait 1"])
        self.assertEqual(a.parse_args(["--countdown", "12", "pos"])[0]["countdown"], 12)
        with self.assertRaises(ValueError):
            a.parse_args(["--bogus", "x", "pos"])

    def test_a_typo_is_caught_before_anything_moves(self):
        with self.assertRaises(ValueError):
            a.validate(["jump 1 2", "clikc"])
        a.validate(["jump 1 2", "dclick", "scroll -3", "shot /tmp/x.png 0 0 10 10"])

    def test_only_shot_wait_and_info_skip_the_gate(self):
        self.assertFalse(a.touches_pointer(["shot /tmp/x.png", "wait 1", "info"]))
        self.assertTrue(a.touches_pointer(["wait 1", "pos"]))
        self.assertTrue(a.touches_pointer(["click"]))

    def test_the_output_can_be_given_by_environment(self):
        os.environ["GHOSTHAND_OUTPUT_SIZE"], os.environ["GHOSTHAND_SCALE"] = "1920x1080", "1.5"
        try:
            self.assertEqual(output_info(), {"name": "", "width": 1920, "height": 1080, "scale": 1.5})
        finally:
            del os.environ["GHOSTHAND_OUTPUT_SIZE"], os.environ["GHOSTHAND_SCALE"]

    def test_last_position_is_remembered_only_while_fresh(self):
        path = os.path.join(tempfile.mkdtemp(), "last.json")
        with open(path, "w") as f:
            json.dump({"x": 10, "y": 20, "t": 1000}, f)
        self.assertEqual(uinput.read_last(path, now=lambda: 1100), (10.0, 20.0))
        self.assertIsNone(uinput.read_last(path, now=lambda: 1000 + uinput.LAST_MAX_AGE + 1))
        self.assertIsNone(uinput.read_last(path + ".missing"))


class Fake:
    x = y = 0.0
    known = True

    def __init__(self):
        self.log = []

    def put(self, x, y): self.log.append(("put", x, y))
    def glide(self, x, y): self.log.append(("glide", x, y))
    def click(self, code, times=1): self.log.append(("click", code, times))
    def wheel(self, n, horizontal=False): self.log.append(("wheel", n, horizontal))
    def button(self, code, is_down): self.log.append(("button", code, is_down))


class Actions(unittest.TestCase):
    def go(self, *actions, check=lambda: None):
        fake = Fake()
        for x in actions:
            a.run(fake, x, check, {"name": "X", "width": 100, "height": 100, "scale": 1})
        return fake.log

    def test_clicks_scrolls_and_drags(self):
        self.assertEqual(self.go("dclick 5 6"), [("glide", 5.0, 6.0), ("click", uinput.BTN_LEFT, 2)])
        self.assertEqual(self.go("mclick"), [("click", uinput.BTN_MIDDLE, 1)])
        self.assertEqual(self.go("scroll 3"), [("wheel", 3.0, False)])
        self.assertEqual(self.go("scroll -2 40 50"), [("glide", 40.0, 50.0), ("wheel", -2.0, False)])
        self.assertEqual(self.go("hscroll 1"), [("wheel", 1.0, True)])
        log = self.go("rdrag 1 2 3 4")
        self.assertEqual(log[1], ("button", uinput.BTN_RIGHT, True))
        self.assertEqual(log[-1], ("button", uinput.BTN_RIGHT, False))

    def test_a_revoke_stops_before_the_next_action(self):
        def check():
            raise g.Revoked()
        with self.assertRaises(g.Revoked):
            self.go("click 1 1", check=check)

    def test_a_long_wait_is_sliced_so_a_revoke_is_noticed(self):
        calls = []

        def check():
            calls.append(1)
            if len(calls) > 3:
                raise g.Revoked()
        start = time.time()
        with self.assertRaises(g.Revoked):
            self.go("wait 30", check=check)
        self.assertLess(time.time() - start, 3)

    def test_unknown_actions_and_bad_numbers_are_errors(self):
        with self.assertRaises(ValueError):
            self.go("teleport 1 2")
        with self.assertRaises(ValueError):
            self.go("jump 1 x")


if __name__ == "__main__":
    unittest.main()
