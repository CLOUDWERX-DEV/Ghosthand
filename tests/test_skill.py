import re
import unittest
from pathlib import Path

from ghosthand import actions

ROOT = Path(__file__).resolve().parent.parent
SKILL = ROOT / "skills" / "ghosthand" / "SKILL.md"


def frontmatter(text):
    m = re.match(r"^---\n(.*?)\n---\n", text, re.S)
    assert m, "no frontmatter"
    out = {}
    for line in m.group(1).splitlines():
        k, _, v = line.partition(":")
        out[k.strip()] = v.strip()
    return out, text[m.end():]


class Skill(unittest.TestCase):
    def test_the_frontmatter_is_valid(self):
        fm, body = frontmatter(SKILL.read_text())
        self.assertEqual(fm["name"], "ghosthand")
        self.assertRegex(fm["name"], r"^[a-z0-9]+(-[a-z0-9]+)*$")
        self.assertLessEqual(len(fm["name"]), 64)
        self.assertTrue(0 < len(fm["description"]) <= 1024, len(fm["description"]))
        self.assertIn("Use when", fm["description"])
        self.assertNotIn("<", fm["description"])
        self.assertEqual(SKILL.parent.name, fm["name"], "the folder is named after the skill")
        self.assertLess(len(body.splitlines()), 500)

    def test_the_package_copy_is_identical(self):
        self.assertEqual(SKILL.read_text(), (ROOT / "ghosthand" / "skill" / "SKILL.md").read_text())

    def test_it_documents_every_action_and_exit_code(self):
        body = SKILL.read_text()
        for name in actions.ACTIONS:
            self.assertIn(f"`{name}", body, f"the skill doesn't mention {name}")
        for code in "012345":
            self.assertRegex(body, rf"\| {code} \|")

    def test_no_dashes_as_punctuation(self):
        self.assertNotIn("\u2014", SKILL.read_text())
        self.assertNotIn("\u2013", SKILL.read_text())


if __name__ == "__main__":
    unittest.main()
