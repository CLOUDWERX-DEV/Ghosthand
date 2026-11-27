import json
import os
import unittest
from unittest import mock

from ghosthand import dialog, geometry, screen

NIRI = json.dumps({"eDP-1": {"name": "eDP-1", "modes": [{"width": 2560, "height": 1600}], "current_mode": 0, "logical": {"scale": 1.25, "transform": "Normal"}}})
NIRI_ROTATED = json.dumps({"DP-1": {"name": "DP-1", "modes": [{"width": 1920, "height": 1080}], "current_mode": 0, "logical": {"scale": 1.0, "transform": "90"}}})
HYPR = json.dumps([{"name": "DP-2", "width": 1920, "height": 1080, "scale": 1.0, "focused": False, "transform": 0},
                   {"name": "eDP-1", "width": 2880, "height": 1800, "scale": 1.5, "focused": True, "transform": 0}])
SWAY = json.dumps([{"name": "HDMI-A-1", "active": True, "focused": True, "scale": 2.0, "transform": "normal", "current_mode": {"width": 3840, "height": 2160}}])
XRANDR = "Screen 0: minimum 16 x 16, current 3840 x 1080, maximum 32767 x 32767\neDP-1 connected primary 1920x1080+0+0\n"
WLR = json.dumps([{"name": "DP-1", "enabled": True, "scale": 1.0, "transform": "normal", "modes": [{"width": 800, "height": 600, "current": False}, {"width": 1280, "height": 720, "current": True}]}])


class Geometry(unittest.TestCase):
    def test_niri(self):
        self.assertEqual(geometry.from_niri(text=NIRI), {"name": "eDP-1", "width": 2560, "height": 1600, "scale": 1.25})

    def test_a_rotated_output_is_taller_than_wide(self):
        self.assertEqual(geometry.from_niri(text=NIRI_ROTATED)["width"], 1080)

    def test_hyprland_picks_the_focused_monitor_or_the_named_one(self):
        self.assertEqual(geometry.from_hyprland(text=HYPR), {"name": "eDP-1", "width": 2880, "height": 1800, "scale": 1.5})
        self.assertEqual(geometry.from_hyprland("DP-2", text=HYPR)["name"], "DP-2")

    def test_sway(self):
        self.assertEqual(geometry.from_sway(text=SWAY), {"name": "HDMI-A-1", "width": 3840, "height": 2160, "scale": 2.0})

    def test_x11_is_the_whole_desktop(self):
        self.assertEqual(geometry.from_xrandr(text=XRANDR), {"name": "X11", "width": 3840, "height": 1080, "scale": 1.0})

    def test_wlr_randr_uses_the_current_mode(self):
        self.assertEqual(geometry.from_wlr_randr(text=WLR)["width"], 1280)

    def test_the_environment_overrides_everything(self):
        with mock.patch.dict(os.environ, {"GHOSTHAND_OUTPUT_SIZE": "1920x1080", "GHOSTHAND_SCALE": "1.5"}):
            self.assertEqual(geometry.output_info(), {"name": "", "width": 1920, "height": 1080, "scale": 1.5})

    def test_detectors_follow_the_session(self):
        env = {"HYPRLAND_INSTANCE_SIGNATURE": "x", "WAYLAND_DISPLAY": "wayland-1"}
        with mock.patch.dict(os.environ, env, clear=True), mock.patch("shutil.which", lambda t: "/usr/bin/" + t):
            self.assertEqual([n for n, _ in geometry.detectors()], ["Hyprland", "wlr-randr"])
        with mock.patch.dict(os.environ, {"DISPLAY": ":0"}, clear=True), mock.patch("shutil.which", lambda t: "/usr/bin/" + t):
            self.assertEqual([n for n, _ in geometry.detectors()], ["X11"])

    def test_the_last_resort_is_the_size_of_a_screenshot(self):
        png = b"\x89PNG\r\n\x1a\n" + b"\x00\x00\x00\rIHDR" + (1366).to_bytes(4, "big") + (768).to_bytes(4, "big")
        with mock.patch.dict(os.environ, {}, clear=True), mock.patch("ghosthand.screen.screenshot_bytes", lambda: png), mock.patch("shutil.which", lambda t: None):
            self.assertEqual(geometry.output_info(), {"name": "", "width": 1366, "height": 768, "scale": 1.0})


class Screenshots(unittest.TestCase):
    def test_each_tool_asks_for_a_region_in_its_own_words(self):
        r = [10, 20, 300, 200]
        self.assertEqual(screen._grim("f.png", r), ["grim", "-g", "10,20 300x200", "f.png"])
        self.assertEqual(screen._maim("f.png", r), ["maim", "-g", "300x200+10+20", "f.png"])
        self.assertEqual(screen._scrot("f.png", r), ["scrot", "-o", "-a", "10,20,300,200", "f.png"])
        self.assertEqual(screen._import("f.png", r)[-4:], ["-crop", "300x200+10+20", "+repage", "f.png"][-4:])
        self.assertIsNone(screen._spectacle("f.png", r), "needs a crop afterwards")
        self.assertIsNone(screen._gnome("f.png", r))
        self.assertEqual(screen._grim("f.png", None), ["grim", "f.png"])

    def test_wayland_tries_grim_first_and_x11_never_tries_it_before_the_x_tools(self):
        have = lambda t: "/usr/bin/" + t
        with mock.patch.dict(os.environ, {"WAYLAND_DISPLAY": "wayland-1"}, clear=True), mock.patch("shutil.which", have):
            self.assertEqual(screen.order()[0], "grim")
        with mock.patch.dict(os.environ, {"DISPLAY": ":0"}, clear=True), mock.patch("shutil.which", have):
            self.assertEqual(screen.order()[0], "maim")
            self.assertLess(screen.order().index("maim"), screen.order().index("grim"))

    def test_the_first_tool_that_works_wins_and_failures_are_reported(self):
        calls = []

        def fake_run(argv, **kw):
            calls.append(argv[0])
            if argv[0] == "grim":
                raise screen.subprocess.CalledProcessError(1, argv, stderr=b"compositor doesn't support wlr-screencopy")
            with open(argv[-1], "wb") as f:
                f.write(b"png")

        with mock.patch.dict(os.environ, {"WAYLAND_DISPLAY": "wayland-1"}, clear=True), mock.patch("shutil.which", lambda t: "/usr/bin/" + t if t in ("grim", "spectacle") else None), \
                mock.patch("subprocess.run", fake_run), mock.patch("ghosthand.config.load", lambda: {}):
            import tempfile
            path = tempfile.mktemp()
            screen.screenshot(path)
            self.assertEqual(calls, ["grim", "spectacle"])

    def test_nothing_installed_says_what_to_install(self):
        with mock.patch.dict(os.environ, {"WAYLAND_DISPLAY": "wayland-1"}, clear=True), mock.patch("shutil.which", lambda t: None), mock.patch("ghosthand.screen.load", lambda: {}):
            with self.assertRaises(RuntimeError) as e:
                screen.screenshot("/tmp/x.png")
            self.assertIn("grim", str(e.exception))

    def test_png_size(self):
        self.assertIsNone(screen.png_size(b"nope"))


class FakeProc:
    def __init__(self, code=None, out=""):
        self.code, self.stdout, self.killed = code, __import__("io").StringIO(out), False

    def poll(self): return self.code
    def kill(self): self.killed, self.code = True, -9
    def wait(self): pass


class Dialogs(unittest.TestCase):
    def test_answers_are_read_the_way_each_tool_reports_them(self):
        self.assertEqual(dialog.answer("zenity", 0, "", 10), "allow")
        self.assertEqual(dialog.answer("zenity", 1, "Allow for 10 min\n", 10), "grant")
        self.assertEqual(dialog.answer("zenity", 1, "", 10), "deny")
        self.assertEqual(dialog.answer("zenity", 5, "", 10), "timeout")
        self.assertEqual(dialog.answer("yad", 2, "", 10), "grant")
        self.assertEqual(dialog.answer("yad", 70, "", 10), "timeout")
        self.assertEqual(dialog.answer("yad", 252, "", 10), "deny", "closing the window is a no")
        self.assertEqual(dialog.answer("kdialog", 0, "", 10), "allow")
        self.assertEqual(dialog.answer("kdialog", 2, "", 10), "deny")

    def test_the_card_names_who_and_why_and_says_nothing_moves_until_allowed(self):
        argv = dialog.command("zenity", "Claude", "Checking the bar", 8, 10)
        text = " ".join(argv)
        self.assertIn("Claude wants to use your mouse", text)
        self.assertIn("Checking the bar", text)
        self.assertIn("--timeout=8", text)

    def gate(self, cfg=None, proc=None):
        g = dialog.DialogGate(tool="zenity", settings=lambda: cfg or {})
        self.proc = proc
        return g

    def run_dir(self):
        import tempfile
        d = tempfile.mkdtemp()
        return mock.patch.dict(os.environ, {"XDG_RUNTIME_DIR": d})

    def test_ask_then_allow_then_end(self):
        with self.run_dir(), mock.patch("subprocess.Popen", lambda *a, **k: FakeProc(0)), mock.patch("shutil.which", lambda t: None):
            g = self.gate()
            self.assertEqual(g("ask", "Claude", "why", 5), "asking")
            self.assertEqual(g("status"), "active")
            self.assertEqual(g("end"), "idle")

    def test_deny_and_silence_are_both_a_no(self):
        with self.run_dir(), mock.patch("subprocess.Popen", lambda *a, **k: FakeProc(1)), mock.patch("shutil.which", lambda t: None):
            g = self.gate()
            g("ask", "Claude", "", 5)
            self.assertEqual(g("status"), "denied")
        with self.run_dir(), mock.patch("subprocess.Popen", lambda *a, **k: FakeProc(None)), mock.patch("shutil.which", lambda t: None):
            g = self.gate()
            g("ask", "Claude", "", 5)
            g.deadline = 0
            self.assertEqual(g("status"), "timeout")

    def test_notify_mode_goes_ahead_when_nobody_answers(self):
        with self.run_dir(), mock.patch("subprocess.Popen", lambda *a, **k: FakeProc(None)), mock.patch("shutil.which", lambda t: None):
            g = self.gate({"mode": "notify"})
            g("ask", "Claude", "", 5)
            g.deadline = 0
            self.assertEqual(g("status"), "active")

    def test_a_grant_lets_the_next_request_straight_in_and_revoke_ends_it(self):
        with self.run_dir(), mock.patch("subprocess.Popen", lambda *a, **k: FakeProc(1, "Allow for 10 min\n")), mock.patch("shutil.which", lambda t: None):
            g = self.gate()
            g("ask", "Claude", "", 5)
            self.assertEqual(g("status"), "active")
            again = self.gate()
            self.assertEqual(again("ask", "Claude", "", 5), "allowed")
            dialog.revoke()
            self.assertEqual(again("status"), "revoked", "a revoke from another process stops a running agent")
            self.assertEqual(self.gate()("ask", "Claude", "", 5), "asking", "and the grant is gone")

    def test_a_revoke_while_the_card_is_open_closes_it(self):
        with self.run_dir(), mock.patch("shutil.which", lambda t: None):
            proc = FakeProc(None)
            with mock.patch("subprocess.Popen", lambda *a, **k: proc):
                g = self.gate()
                g("ask", "Claude", "", 5)
                __import__("time").sleep(0.01)
                dialog.revoke()
                self.assertEqual(g("status"), "revoked")
                self.assertTrue(proc.killed)

    def test_mode_off_needs_no_dialog_and_no_tool_means_no_answer(self):
        with self.run_dir(), mock.patch("shutil.which", lambda t: None):
            self.assertEqual(self.gate({"mode": "off"})("ask", "Claude", "", 5), "allowed")
            self.assertIsNone(dialog.DialogGate(tool=None, settings=lambda: {})("ask", "Claude", "", 5), "never a silent pass")


if __name__ == "__main__":
    unittest.main()
