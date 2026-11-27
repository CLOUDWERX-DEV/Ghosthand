import os
import struct
import unittest
from unittest import mock

from ghosthand import actions as act
from ghosthand import dialog, keyboard as kbd, keys, layout
from ghosthand import gate as g
from ghosthand.session import execute

US = layout.us_layout()
SHIFT, CTRL, ALT, ENTER, TAB = 42, 29, 56, 28, 15


def presses(read_fd):
    """What a Keyboard wrote to its (fake) device: [(code, value)] for key events only."""
    os.set_blocking(read_fd, False)
    try:
        data = os.read(read_fd, 1 << 16)
    except BlockingIOError:
        return []
    out = []
    for i in range(0, len(data), 24):
        _, _, kind, code, value = struct.unpack("llHHi", data[i:i + 24])
        if kind == 1:
            out.append((code, value))
    return out


def fake_keyboard(lay=US):
    r, w = os.pipe()
    with mock.patch("ghosthand.keyboard.create_device", lambda *a, **k: w), mock.patch("ghosthand.keyboard.destroy_device", lambda fd: None):
        kb = kbd.Keyboard(lay=lay, delay=0, settle=0)
    kb.read_fd = r
    return kb


class Names(unittest.TestCase):
    def test_named_keys_and_modifiers(self):
        self.assertEqual(keys.named("Return"), 28)
        self.assertEqual(keys.named("ESC"), 1)
        self.assertEqual(keys.named("Page_Down"), 109)
        self.assertEqual(keys.named("F5"), 63)
        self.assertEqual(keys.named("f12"), 88)
        self.assertEqual(keys.named("F13"), 183)
        self.assertEqual(keys.named("XF86AudioPlay"), 164)
        self.assertIsNone(keys.named("nonsense"))
        self.assertTrue(keys.is_modifier("Ctrl") and not keys.is_modifier("a"))

    def test_the_virtual_keyboard_can_send_every_key_we_name(self):
        for name, code in keys.CODES.items():
            self.assertIn(code, keys.ALL_KEYS, name)


class Combos(unittest.TestCase):
    def test_a_chord_is_modifiers_then_the_key(self):
        self.assertEqual(kbd.parse_combo("ctrl+shift+t", US), [CTRL, SHIFT, 20])
        self.assertEqual(kbd.parse_combo("alt+F4", US), [ALT, 62])
        self.assertEqual(kbd.parse_combo("super+Left", US), [125, 105])

    def test_a_character_that_needs_shift_brings_it(self):
        self.assertEqual(kbd.parse_combo("A", US), [SHIFT, 30])
        self.assertEqual(kbd.parse_combo("ctrl+A", US), [SHIFT, CTRL, 30])

    def test_the_plus_key_itself(self):
        self.assertEqual(kbd.parse_combo("ctrl++", US), [SHIFT, CTRL, 13])

    def test_unknown_keys_are_errors(self):
        for bad in ("ctrl+nonsense", "", "ctrl+Hyper"):
            with self.assertRaises(ValueError, msg=bad):
                kbd.parse_combo(bad, US)

    def test_single_characters_follow_the_layout(self):
        de = layout._xkb("de")
        if not de:
            self.skipTest("libxkbcommon isn't available")
        self.assertEqual(kbd.parse_combo("z", de), [21], "z is where y is on a QWERTY board")
        self.assertEqual(kbd.parse_combo("ctrl+z", de), [CTRL, 21])
        fr = layout._xkb("fr")
        self.assertEqual(kbd.parse_combo("a", fr), [16], "and a is where q is on AZERTY")


class Layouts(unittest.TestCase):
    def test_the_built_in_us_layout_is_complete(self):
        for ch in "abcxyzABCXYZ0123456789 `~!@#$%^&*()-_=+[]{}\\|;:'\",<.>/?\n\t":
            self.assertIsNotNone(US.find(ch), repr(ch))

    def test_missing_reports_what_it_cannot_type(self):
        self.assertEqual(US.missing("hello é€\n"), ["é", "€"])
        self.assertEqual(US.missing("plain text\twith tabs\n"), [])

    def test_xkb_agrees_with_the_built_in_table_for_us(self):
        real = layout._xkb("us")
        if not real:
            self.skipTest("libxkbcommon isn't available")
        for ch, hit in US.table.items():
            self.assertEqual(real.find(ch), hit, repr(ch))

    def test_xkb_knows_altgr_and_other_layouts(self):
        de = layout._xkb("de")
        if not de:
            self.skipTest("libxkbcommon isn't available")
        self.assertEqual(de.find("@"), (16, layout.ALTGR))
        self.assertEqual(de.find("€"), (18, layout.ALTGR))
        self.assertIsNotNone(de.find("ä"))
        dvorak = layout._xkb("us", "dvorak")
        self.assertEqual(dvorak.find("s"), (39, 0))

    def test_a_layout_that_does_not_exist_falls_back_to_us(self):
        self.assertIsNone(layout._xkb("no-such-layout"))

    def test_the_layout_comes_from_the_config_first(self):
        with mock.patch("ghosthand.layout.load", lambda: {"keyboard_layout": "de", "keyboard_variant": "nodeadkeys"}):
            self.assertEqual(layout.detect(), ("de", "nodeadkeys", ""))
        with mock.patch("ghosthand.layout.load", lambda: {}), mock.patch.dict(os.environ, {"XKB_DEFAULT_LAYOUT": "fr"}):
            self.assertEqual(layout.detect()[0], "fr")


class Typing(unittest.TestCase):
    def test_text_becomes_key_presses_with_shift(self):
        kb = fake_keyboard()
        kb.type("Hi!\n")
        self.assertEqual(presses(kb.read_fd), [(SHIFT, 1), (35, 1), (35, 0), (SHIFT, 0), (23, 1), (23, 0), (SHIFT, 1), (2, 1), (2, 0), (SHIFT, 0), (ENTER, 1), (ENTER, 0)])

    def test_unmappable_text_is_refused_before_a_single_key_is_pressed(self):
        kb = fake_keyboard()
        with self.assertRaises(ValueError) as e:
            kb.type("fine é")
        self.assertIn("paste", str(e.exception))
        self.assertEqual(presses(kb.read_fd), [])

    def test_a_chord_releases_in_reverse(self):
        kb = fake_keyboard()
        kb.key("ctrl+shift+t")
        self.assertEqual(presses(kb.read_fd), [(CTRL, 1), (SHIFT, 1), (20, 1), (20, 0), (SHIFT, 0), (CTRL, 0)])

    def test_held_keys_are_released_on_close(self):
        kb = fake_keyboard()
        kb.hold("shift")
        kb.hold("ctrl")
        with mock.patch("time.sleep"), mock.patch("ghosthand.keyboard.destroy_device", lambda fd: None):
            kb.close()
        self.assertEqual(presses(kb.read_fd), [(SHIFT, 1), (CTRL, 1), (CTRL, 0), (SHIFT, 0)])

    def test_a_revoke_is_noticed_while_typing(self):
        kb = fake_keyboard()
        calls = []

        def check():
            calls.append(1)
            raise g.Revoked()
        with self.assertRaises(g.Revoked):
            kb.type("a" * 100, check)
        self.assertEqual(len(calls), 1)
        self.assertLessEqual(len(presses(kb.read_fd)), 50, "it stopped within a couple of dozen characters")

    def test_escapes(self):
        self.assertEqual(kbd.unescape(r"one\ntwo\tthree\\four\q"), "one\ntwo\tthree\\four\\q")


class Actions(unittest.TestCase):
    def test_scope_names_what_is_taken(self):
        self.assertEqual(act.scope(["click"]), "mouse")
        self.assertEqual(act.scope(["type hi"]), "keyboard")
        self.assertEqual(act.scope(["click", "type hi"]), "mouse and keyboard")
        self.assertEqual(act.scope(["key ctrl+s", "wait 1"]), "keyboard")

    def test_the_card_shows_what_will_be_typed(self):
        self.assertEqual(act.detail(["click", "type git status\\n"]), "type: 'git status\\n'".replace("\\n", "\n").replace("'git status\n'", repr("git status\n")))
        self.assertEqual(act.detail(["key ctrl+c", "key Return"]), "key: ctrl+c  key: Return")
        self.assertEqual(act.detail(["click"]), "")
        self.assertTrue(act.detail(["type " + "x" * 500]).endswith("…"))
        self.assertLessEqual(len(act.detail(["type " + "x" * 500])), 140)

    def test_focus_and_screenshots_need_no_gate_but_keys_do(self):
        self.assertFalse(act.touches_input(["focus", "shot /tmp/x.png", "wait 1", "info"]))
        self.assertTrue(act.touches_input(["key Return"]))
        self.assertTrue(act.touches_input(["type a"]))

    def test_validation_catches_bad_keyboard_actions_up_front(self):
        for bad in ("type", "key", "keydown", "key ctrl+nonsense", "type café", "paste"):
            with mock.patch("ghosthand.actions.kb.layout.current", lambda: US):
                with self.assertRaises(ValueError, msg=bad):
                    act.validate([bad])
        with mock.patch("ghosthand.actions.kb.layout.current", lambda: US):
            act.validate(["key ctrl+s Return", "type hello world", "keydown shift", "keyup shift", "paste café €", "focus"])

    def test_the_keyboard_can_be_switched_off(self):
        with mock.patch("ghosthand.actions.load", lambda: {"keyboard": False}):
            with self.assertRaises(ValueError) as e:
                act.validate(["type hi"])
            self.assertIn("switched off", str(e.exception))
            act.validate(["click 1 2"])

    def test_type_keeps_the_text_exactly_as_written(self):
        kb = fake_keyboard()
        act.run(None, "type  two  spaces ", lambda: None, {}, keyboard=kb)
        codes = [c for c, v in presses(kb.read_fd) if v == 1]
        self.assertEqual(codes.count(57), 4, "a leading space, two inner and one trailing are all typed")


class Session(unittest.TestCase):
    class Dev:
        def __init__(self, *a):
            self.closed, self.log = False, []

        def close(self):
            self.closed = True

    def setUp(self):
        import tempfile
        env = {"GHOSTHAND_OUTPUT_SIZE": "1000x800", "GHOSTHAND_SCALE": "1.0", "XDG_RUNTIME_DIR": tempfile.mkdtemp()}
        self.env = mock.patch.dict(os.environ, env)
        self.env.start()
        self.addCleanup(self.env.stop)
        self.lock = mock.patch.object(g, "locked", lambda command=None: False)
        self.lock.start()
        self.addCleanup(self.lock.stop)
        self.layout = mock.patch("ghosthand.actions.kb.layout.current", lambda: US)
        self.layout.start()
        self.addCleanup(self.layout.stop)

    def test_keyboard_only_runs_make_no_pointer_and_mouse_only_runs_no_keyboard(self):
        made = []
        asks = []

        def ask(*args):
            asks.append(args)
            return "allowed" if args[0] == "ask" else "active"

        class Kb(self.Dev):
            def key(self, c): made.append(("key", c))
            def type(self, t, check=None): made.append(("type", t))

        def no_pointer(*a):
            raise AssertionError("no pointer wanted")

        r = execute("Claude", "x", 0, ["type hi\\n", "key ctrl+s"], make_pointer=no_pointer, ask=ask, make_keyboard=Kb)
        self.assertEqual(r.code, 0)
        self.assertEqual(made, [("type", "hi\n"), ("key", "ctrl+s")])
        self.assertEqual(asks[0][4:], ("keyboard", "type: 'hi\\n'  key: ctrl+s".replace("'hi\\n'", repr("hi\n"))))

        def no_keyboard():
            raise AssertionError("no keyboard wanted")

        r = execute("Claude", "x", 0, ["pos"], make_pointer=lambda i, a: type("P", (self.Dev,), {"x": 1, "y": 2, "known": True})(), ask=ask, make_keyboard=no_keyboard)
        self.assertEqual(r.code, 0)
        self.assertEqual([a for a in asks if a[0] == "ask"][-1][4], "mouse")

    def test_the_keyboard_is_always_closed_even_when_an_action_fails(self):
        kb = self.Dev()
        kb.type = lambda t, check=None: (_ for _ in ()).throw(ValueError("boom"))
        r = execute("Claude", "x", 0, ["type hi"], ask=lambda *a: "allowed" if a[0] == "ask" else "active", make_keyboard=lambda: kb)
        self.assertEqual(r.code, g.EXIT_FAILED)
        self.assertTrue(kb.closed)

    def test_a_denied_run_never_creates_a_keyboard(self):
        r = execute("Claude", "x", 0, ["type hi"], ask=lambda *a: "denied" if a[0] == "status" else "asking", make_keyboard=lambda: self.fail("created"))
        self.assertEqual(r.code, g.EXIT_DENIED)


class Grants(unittest.TestCase):
    """Allowing the mouse for ten minutes never lets anything type."""

    def test_a_mouse_grant_does_not_cover_the_keyboard(self):
        import tempfile
        with mock.patch.dict(os.environ, {"XDG_RUNTIME_DIR": tempfile.mkdtemp()}), mock.patch("shutil.which", lambda t: None):
            class Proc:
                stdout = __import__("io").StringIO("Allow for 10 min\n")

                def poll(self): return 1

            with mock.patch("subprocess.Popen", lambda *a, **k: Proc()):
                first = dialog.DialogGate(tool="zenity", settings=lambda: {})
                first("ask", "Claude", "", 5, "mouse", "")
                self.assertEqual(first("status"), "active")
            again = dialog.DialogGate(tool="zenity", settings=lambda: {})
            with mock.patch("subprocess.Popen", lambda *a, **k: Proc()):
                self.assertEqual(again("ask", "Claude", "", 5, "mouse", ""), "allowed", "the mouse is covered")
                self.assertEqual(again("ask", "Claude", "", 5, "mouse and keyboard", "type: 'x'"), "asking", "the keyboard is not")

    def test_the_card_text_says_keyboard_and_what_will_be_typed(self):
        text = " ".join(dialog.command("zenity", "Claude", "why", 8, 10, "mouse and keyboard", "type: 'rm -rf x'"))
        self.assertIn("Claude wants to use your mouse and keyboard", text)
        self.assertIn("type: 'rm -rf x'", text)

    def test_a_standing_keyboard_grant_ends_on_revoke(self):
        import tempfile
        with mock.patch.dict(os.environ, {"XDG_RUNTIME_DIR": tempfile.mkdtemp()}):
            dialog.runtime_dir().mkdir(parents=True, exist_ok=True)
            dialog._grant_file("keys").write_text("9999999999\n")
            dialog.revoke()
            self.assertFalse(dialog._grant_file("keys").exists())


if __name__ == "__main__":
    unittest.main()
