"""Tests for build_constitution: each check fails on a broken tree and passes
on its control, a valid tree renders as expected in both languages, and the
numbering check against a base commit runs on a temporary git repository."""

import contextlib
import io
import subprocess
import tempfile
import unittest
from pathlib import Path

import build_constitution as bc

INDEX = """\
status = "draft"
revision = "0.2"
languages = ["en", "zh"]

[title]
en = "Test Constitution"
zh = "测试宪法"

[front]
en = '''
The preamble cites {@20260102-beta}.
'''
zh = '''
序言引用{@20260102-beta}。
'''
"""

ALPHA = """\
id = "20260101-alpha"
number = "1"

[title]
en = "First"
zh = "第一"

[intro]
en = '''
Intro, see {@20260102-beta-two}.
'''
zh = '''
导语，见{@20260102-beta-two}。
'''
"""

BETA = """\
id = "20260102-beta"
number = "2"

[title]
en = "Second"
zh = "第二"
"""

ALPHA_ONE = """\
id = "20260101-alpha-one"
article = "20260101-alpha"
number = "1"
status = "draft"

[content]
en = '''
One, see {@20260102-beta-one}.
- {#20260101-money} Money.
- {#20260101-machines} Machines; {braces} and {@nope} stay.
'''
zh = '''
一，见{@20260102-beta-one}。
- {#20260101-money} 钱。
- {#20260101-machines} 机器；{braces} 与 {@nope} 不变。
'''

[[items]]
id = "20260101-money"
number = "H1"

[[items]]
id = "20260101-machines"
number = "H2"
"""

BETA_ONE = """\
id = "20260102-beta-one"
article = "20260102-beta"
number = "1"
status = "draft"

[content]
en = '''
Four, see {@20260101-money} and {@20260101-alpha}.
'''
zh = '''
四，见{@20260101-money}和{@20260101-alpha}。
'''
"""

BETA_TWO = """\
id = "20260102-beta-two"
article = "20260102-beta"
number = "1a"
status = "draft"

[content]
en = '''
Between:
1. first

   more
2. second
'''
zh = '''
之间：
1. 第一

   更多
2. 第二
'''
"""

RULE = """\
id = "20260103-rule"
status = "draft"
scope = "all"
authority = ["20260101-alpha", "20260101-money"]

[title]
en = "A rule"
zh = "一条规则"

[content]
en = '''
Cites {@20260102-beta-one} and {@20260103-rule}.
'''
zh = '''
引用{@20260102-beta-one}与{@20260103-rule}。
'''
"""

EN = """\
Generated from `constitution/index.toml`, `constitution/articles/` and \
`constitution/sections/` by `tools/build_constitution.py`. Do not edit this \
file; edit the sources and rebuild. Status: draft. Revision: 0.2. Language: \
English.

# Test Constitution

The preamble cites Art. II.

## Article I. First

Intro, see Art. II §1a.

- §1 One, see Art. II §1.
  - H1 Money.
  - H2 Machines; {braces} and {@nope} stay.

## Article II. Second

- §1 Four, see H1 and Art. I.
- §1a Between:
  1. first

     more
  2. second
"""

ZH = """\
本文件由 `tools/build_constitution.py` 从 `constitution/index.toml`、\
`constitution/articles/` 和 `constitution/sections/` 生成，不要手改；\
改源文件后重新生成。状态：draft。修订版：0.2。语言：中文。

# 测试宪法

序言引用第二条。

## 第一条 第一

导语，见第二条第一款之一。

- 第一款 一，见第二条第一款。
  - H1 钱。
  - H2 机器；{braces} 与 {@nope} 不变。

## 第二条 第二

- 第一款 四，见H1和第一条。
- 第一款之一 之间：
  1. 第一

     更多
  2. 第二
"""

FILES = {
    "constitution/index.toml": INDEX,
    "constitution/articles/20260101-alpha.toml": ALPHA,
    "constitution/articles/20260102-beta.toml": BETA,
    "constitution/sections/20260101-alpha-one.toml": ALPHA_ONE,
    "constitution/sections/20260102-beta-one.toml": BETA_ONE,
    "constitution/sections/20260102-beta-two.toml": BETA_TWO,
    "rules/20260103-rule.toml": RULE,
}


class Tree(unittest.TestCase):
    """A valid source tree in a temporary directory, edited per test."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        for name, text in FILES.items():
            self.write(name, text)

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def edit(self, name, old, new, count=1):
        path = self.root / name
        text = path.read_text(encoding="utf-8")
        self.assertIn(old, text)
        path.write_text(text.replace(old, new, count), encoding="utf-8")

    def activate(self):
        """Make the tree active: index and every section and rule."""
        self.edit("constitution/index.toml", 'status = "draft"', 'status = "active"')
        self.edit("constitution/index.toml", 'revision = "0.2"', 'revision = "1"')
        for name in FILES:
            if "/sections/" in name or name.startswith("rules/"):
                self.edit(name, 'status = "draft"', 'status = "active"')

    def errors(self):
        errors, _, _ = bc.build(self.root)
        return errors

    def assertFails(self, check, location, message):
        errors = self.errors()
        self.assertTrue(errors, "expected an error, got none")
        self.assertTrue(all(f": check {check}: " in e for e in errors), errors)
        self.assertIn(f"{location}: check {check}: {message}", errors)


class RenderTest(Tree):
    def test_valid_tree_renders_both_languages(self):
        errors, _, outputs = bc.build(self.root)
        self.assertEqual(errors, [])
        self.assertEqual(outputs, {"en": EN, "zh": ZH})

    def test_write_then_check(self):
        self.assertEqual(bc.main(["--root", str(self.root)]), 0)
        self.assertEqual((self.root / "constitution/constitution.zh.md").read_bytes(), ZH.encode())
        self.assertEqual(bc.main(["--root", str(self.root), "--check"]), 0)

    def test_repealed_placeholders(self):
        self.activate()
        self.edit("constitution/sections/20260102-beta-one.toml", 'status = "active"',
                  'status = "repealed"\nrepealed = "2026-11"')
        self.edit("constitution/sections/20260101-alpha-one.toml",
                  'id = "20260101-machines"\nnumber = "H2"',
                  'id = "20260101-machines"\nnumber = "H2"\nrepealed = "2026-11"')
        for lang, line in (("en", "- {#20260101-machines} Machines; {braces} and {@nope} stay."),
                           ("zh", "- {#20260101-machines} 机器；{braces} 与 {@nope} 不变。")):
            self.edit("constitution/sections/20260101-alpha-one.toml", line, "- {#20260101-machines}")
        self.write("constitution/articles/20260104-gamma.toml",
                   BETA.replace("20260102-beta", "20260104-gamma").replace('"2"', '"3"')
                   .replace("[title]", 'repealed = "2026-12"\n\n[title]'))
        # A citation of a repealed section would be an error; move it.
        self.edit("constitution/sections/20260101-alpha-one.toml", "{@20260102-beta-one}",
                  "{@20260102-beta-two}", 2)
        self.edit("rules/20260103-rule.toml", "{@20260102-beta-one}", "{@20260102-beta-two}", 2)
        errors, _, outputs = bc.build(self.root)
        self.assertEqual(errors, [])
        self.assertIn("\n  - H2 (repealed November 2026)\n", outputs["en"])
        self.assertIn("\n- §1 (repealed November 2026)\n", outputs["en"])
        self.assertTrue(outputs["en"].endswith("## Article III (repealed December 2026)\n"))
        self.assertIn("\n  - H2 (2026 年 11 月废止)\n", outputs["zh"])
        self.assertIn("\n- 第一款 (2026 年 11 月废止)\n", outputs["zh"])
        self.assertTrue(outputs["zh"].endswith("## 第三条 (2026 年 12 月废止)\n"))

    def test_nested_definition_keeps_its_marker(self):
        for lang, text in (("en", "- {#20260101-machines} Machines; {braces} and {@nope} stay."),
                           ("zh", "- {#20260101-machines} 机器；{braces} 与 {@nope} 不变。")):
            self.edit("constitution/sections/20260101-alpha-one.toml", text, "  " + text.replace("- ", "* ", 1))
        errors, _, outputs = bc.build(self.root)
        self.assertEqual(errors, [])
        self.assertIn("\n    * H2 Machines; {braces} and {@nope} stay.\n", outputs["en"])

    def test_numerals(self):
        self.assertEqual([bc.roman(n) for n in (1, 4, 9, 14, 40)], ["I", "IV", "IX", "XIV", "XL"])
        self.assertEqual([bc.zh_number(n) for n in (1, 10, 11, 20, 23)],
                         ["一", "十", "十一", "二十", "二十三"])
        en, zh = bc.LANGUAGES["en"], bc.LANGUAGES["zh"]
        self.assertEqual(en.article_number(4, "b"), "IVb")
        self.assertEqual(zh.article_number(4, "b"), "第四条之二")
        self.assertEqual(en.section_number(12, ""), "§12")
        self.assertEqual(zh.section_number(12, "a"), "第十二款之一")


class CheckTest(Tree):
    def test_controls_pass(self):
        self.assertEqual(self.errors(), [])

    def test_1_invalid_toml(self):
        self.edit("constitution/articles/20260102-beta.toml", "[title]", "[title")
        errors = self.errors()
        self.assertEqual(len(errors), 1, errors)
        self.assertTrue(errors[0].startswith("constitution/articles/20260102-beta.toml:1: check 1: not valid TOML: "), errors)

    def test_1_missing_and_unknown_key(self):
        self.edit("constitution/sections/20260102-beta-one.toml", 'number = "1"', 'order = "1"')
        errors = self.errors()
        self.assertIn("constitution/sections/20260102-beta-one.toml:1: check 1: lacks `number`", errors)
        self.assertIn("constitution/sections/20260102-beta-one.toml:3: check 1: unknown key `order`", errors)

    def test_1_languages_differ(self):
        self.edit("constitution/articles/20260102-beta.toml", 'zh = "第二"', 'fr = "Deuxième"')
        self.assertFails(1, "constitution/articles/20260102-beta.toml:4",
                         "`title` has languages ['en', 'fr'], index.toml lists ['en', 'zh']")

    def test_1_id_differs_from_file_name(self):
        self.edit("rules/20260103-rule.toml", 'id = "20260103-rule"', 'id = "20260103-other"')
        self.assertFails(1, "rules/20260103-rule.toml:1", "`id` must equal the file name `20260103-rule`")

    def test_1_bad_number(self):
        self.edit("constitution/sections/20260102-beta-two.toml", 'number = "1a"', 'number = "1A"')
        self.assertFails(1, "constitution/sections/20260102-beta-two.toml:3",
                         "`number` `1A` is not an integer with an optional letter suffix (2, 2a)")

    def test_1_status_against_index(self):
        self.edit("constitution/sections/20260102-beta-one.toml", 'status = "draft"', 'status = "active"')
        self.assertFails(1, "constitution/sections/20260102-beta-one.toml:4",
                         "a section is active while index.toml is draft")

    def test_1_repealed_date(self):
        self.activate()
        self.edit("constitution/sections/20260102-beta-one.toml", 'status = "active"',
                  'status = "repealed"\nrepealed = "2026-13"')
        self.assertFails(1, "constitution/sections/20260102-beta-one.toml:5",
                         "`repealed` must be `YYYY-MM`, got `2026-13`")

    def test_1_repealed_without_status(self):
        self.activate()
        self.edit("constitution/sections/20260102-beta-one.toml", 'status = "active"',
                  'status = "active"\nrepealed = "2026-11"')
        self.assertFails(1, "constitution/sections/20260102-beta-one.toml:5",
                         "has `repealed` but status is active")

    def test_2_id_format(self):
        self.write("constitution/articles/20261301-x.toml", BETA.replace("20260102-beta", "20261301-x")
                   .replace('"2"', '"3"'))
        self.assertFails(2, "constitution/articles/20261301-x.toml:1",
                         "`20261301-x` is not an ID `YYYYMMDD-words` with a real date")

    def test_2_duplicate_across_constitution_and_rules(self):
        self.write("rules/20260101-money.toml", RULE.replace("20260103-rule", "20260101-money"))
        self.assertFails(2, "rules/20260101-money.toml:1",
                         "20260101-money is already defined at constitution/sections/20260101-alpha-one.toml:19")

    def test_3_unknown_article(self):
        self.edit("constitution/sections/20260102-beta-two.toml", 'article = "20260102-beta"',
                  'article = "20260102-gamma"')
        self.assertFails(3, "constitution/sections/20260102-beta-two.toml:2",
                         "article 20260102-gamma does not exist")

    def test_3_duplicate_number(self):
        self.edit("constitution/sections/20260102-beta-two.toml", 'number = "1a"', 'number = "1"')
        self.assertFails(3, "constitution/sections/20260102-beta-two.toml:3",
                         "section of 20260102-beta number 1 is already used at"
                         " constitution/sections/20260102-beta-one.toml:3")

    def test_3_duplicate_item_number_across_sections(self):
        self.edit("constitution/sections/20260102-beta-one.toml", "Four, see",
                  "Four:\n- {#20260102-cash} Cash, see")
        self.edit("constitution/sections/20260102-beta-one.toml", "四，见", "四：\n- {#20260102-cash} 现金，见")
        self.write("constitution/sections/20260102-beta-one.toml",
                   (self.root / "constitution/sections/20260102-beta-one.toml").read_text(encoding="utf-8")
                   + '\n[[items]]\nid = "20260102-cash"\nnumber = "H1"\n')
        self.assertFails(3, "constitution/sections/20260102-beta-one.toml:18",
                         "item number H1 is already used at constitution/sections/20260101-alpha-one.toml:20")

    def test_3_duplicate_article_number(self):
        self.edit("constitution/articles/20260102-beta.toml", 'number = "2"', 'number = "1"')
        self.assertFails(3, "constitution/articles/20260102-beta.toml:2",
                         "article number 1 is already used at constitution/articles/20260101-alpha.toml:2")

    def test_3_repealed_article_with_live_section(self):
        self.activate()
        self.edit("constitution/articles/20260102-beta.toml", "[title]", 'repealed = "2026-11"\n\n[title]')
        self.assertFails(3, "constitution/sections/20260102-beta-one.toml:4",
                         "20260102-beta-one is active but its article 20260102-beta is repealed")

    def test_3_definition_not_at_item_start(self):
        self.edit("constitution/sections/20260101-alpha-one.toml", "- {#20260101-money} Money.",
                  "- Money {#20260101-money}.")
        self.assertFails(3, "constitution/sections/20260101-alpha-one.toml:9",
                         "a `{#...}` definition is not at the start of a list item")

    def test_3_definition_misplaced_in_one_language_only(self):
        self.edit("constitution/sections/20260101-alpha-one.toml", "- {#20260101-money} 钱。",
                  "- 钱 {#20260101-money}。")
        self.assertFails(3, "constitution/sections/20260101-alpha-one.toml:14",
                         "a `{#...}` definition is not at the start of a list item")

    def test_3_definitions_differ_from_items(self):
        self.edit("constitution/sections/20260101-alpha-one.toml", 'number = "H2"', 'number = "H0"')
        # H0 fails check 1; use a legal number that sorts first instead.
        self.edit("constitution/sections/20260101-alpha-one.toml", 'number = "H0"', 'number = "A1"')
        self.assertFails(3, "constitution/sections/20260101-alpha-one.toml:6",
                         "en defines ['20260101-money', '20260101-machines'];"
                         " `items` by number are ['20260101-machines', '20260101-money']")

    def test_3_definition_outside_section(self):
        self.edit("constitution/articles/20260101-alpha.toml", "Intro,", "- {#20260101-x} Intro,")
        self.edit("constitution/articles/20260101-alpha.toml", "导语，", "- {#20260101-x} 导语，")
        self.assertFails(3, "constitution/articles/20260101-alpha.toml:10",
                         "en `intro` defines 20260101-x; only a section's content defines items")

    def test_4_definition_order(self):
        self.edit("constitution/sections/20260101-alpha-one.toml",
                  "- {#20260101-money} 钱。\n- {#20260101-machines} 机器；",
                  "- {#20260101-machines} 钱。\n- {#20260101-money} 机器；")
        self.assertFails(4, "constitution/sections/20260101-alpha-one.toml:6",
                         "zh `content` defines ['20260101-machines', '20260101-money']"
                         " where en defines ['20260101-money', '20260101-machines']")

    def test_4_reference_set(self):
        self.edit("constitution/index.toml", "序言引用{@20260102-beta}。", "序言引用{@20260101-alpha}。")
        errors = self.errors()
        self.assertEqual(errors, [
            "constitution/index.toml:11: check 4: en `front` cites 20260102-beta; zh does not",
            "constitution/index.toml:14: check 4: zh `front` cites 20260101-alpha; en does not",
        ])

    def test_5_undefined(self):
        self.edit("constitution/sections/20260102-beta-one.toml", "{@20260101-money}", "{@20260101-cash}", 2)
        self.assertFails(5, "constitution/sections/20260102-beta-one.toml:8", "cites undefined 20260101-cash")

    def test_5_repealed(self):
        self.activate()
        self.edit("constitution/sections/20260102-beta-one.toml", 'status = "active"',
                  'status = "repealed"\nrepealed = "2026-11"')
        errors = self.errors()
        self.assertIn("constitution/sections/20260101-alpha-one.toml:8: check 5:"
                      " cites repealed 20260102-beta-one", errors)
        self.assertIn("rules/20260103-rule.toml:12: check 5: cites repealed 20260102-beta-one", errors)

    def test_5_constitution_cites_rule(self):
        self.edit("constitution/sections/20260102-beta-one.toml", "{@20260101-money}", "{@20260103-rule}", 2)
        self.assertFails(5, "constitution/sections/20260102-beta-one.toml:8",
                         "the constitution cites rule 20260103-rule; it may only cite itself")

    def test_5_authority(self):
        self.edit("rules/20260103-rule.toml", '"20260101-money"]', '"20260103-rule"]')
        self.assertFails(5, "rules/20260103-rule.toml:4",
                         "authority 20260103-rule is not an article, section or item")

    def test_6_outputs_match(self):
        self.assertEqual(bc.main(["--root", str(self.root)]), 0)
        errors = bc.Errors()
        bc.check_outputs(self.root, bc.build(self.root)[2], errors)
        self.assertEqual(errors, [])

    def test_6_stale_output(self):
        self.assertEqual(bc.main(["--root", str(self.root)]), 0)
        path = self.root / "constitution/constitution.en.md"
        path.write_bytes(path.read_bytes().replace(b"Money.", b"Money!"))
        errors = bc.Errors()
        bc.check_outputs(self.root, bc.build(self.root)[2], errors)
        self.assertEqual(errors, ["constitution/constitution.en.md:12: check 6: differs from a"
                                  " fresh build; run tools/build_constitution.py"])
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(bc.main(["--root", str(self.root), "--check"]), 1)


class BaseTest(Tree):
    """Check 7 against a base commit in a temporary git repository."""

    def git(self, *args):
        subprocess.run(["git", "-C", str(self.root), "-c", "user.name=t",
                        "-c", "user.email=t@example.test", *args], check=True, capture_output=True)

    def commit_base(self, active=True):
        if active:
            self.activate()
        self.assertEqual(bc.main(["--root", str(self.root)]), 0)
        self.git("init", "-q")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "base")

    def base_errors(self):
        errors, src, _ = bc.build(self.root)
        self.assertEqual(errors, [])
        note = bc.check_base(self.root, "HEAD", src, errors)
        return errors, note

    def test_control_passes(self):
        self.commit_base()
        self.assertEqual(self.base_errors(), ([], ""))

    def test_main_runs_check_with_base(self):
        self.commit_base()
        self.assertEqual(bc.main(["--root", str(self.root), "--check", "--base", "HEAD"]), 0)
        self.edit("constitution/sections/20260102-beta-two.toml", 'number = "1a"', 'number = "1b"')
        self.assertEqual(bc.main(["--root", str(self.root)]), 0)
        with contextlib.redirect_stderr(io.StringIO()) as err:
            self.assertEqual(bc.main(["--root", str(self.root), "--check", "--base", "HEAD"]), 1)
        self.assertIn("check 7: 20260102-beta-two changed `number` from `1a` to `1b`", err.getvalue())

    def test_changed_number(self):
        self.commit_base()
        self.edit("constitution/sections/20260102-beta-two.toml", 'number = "1a"', 'number = "2"')
        errors, _ = self.base_errors()
        self.assertEqual(errors, ["constitution/sections/20260102-beta-two.toml:3: check 7:"
                                  " 20260102-beta-two changed `number` from `1a` to `2` since base HEAD"])

    def test_moved_item_and_renumbered_article(self):
        self.commit_base()
        self.edit("constitution/sections/20260101-alpha-one.toml", 'number = "H2"', 'number = "H3"')
        self.edit("constitution/articles/20260102-beta.toml", 'number = "2"', 'number = "3"')
        errors, _ = self.base_errors()
        self.assertEqual(errors, [
            "constitution/articles/20260102-beta.toml:2: check 7:"
            " 20260102-beta changed `number` from `2` to `3` since base HEAD",
            "constitution/sections/20260101-alpha-one.toml:24: check 7:"
            " item 20260101-machines changed `number` from `H2` to `H3` since base HEAD",
        ])

    def test_section_moved_to_another_article(self):
        self.commit_base()
        self.edit("constitution/sections/20260102-beta-two.toml", 'article = "20260102-beta"',
                  'article = "20260101-alpha"')
        self.edit("constitution/sections/20260102-beta-two.toml", 'number = "1a"', 'number = "2"')
        errors, _ = self.base_errors()
        self.assertEqual(errors, [
            "constitution/sections/20260102-beta-two.toml:3: check 7:"
            " 20260102-beta-two changed `number` from `1a` to `2` since base HEAD",
            "constitution/sections/20260102-beta-two.toml:2: check 7:"
            " 20260102-beta-two changed `article` from `20260102-beta` to `20260101-alpha` since base HEAD",
        ])

    def test_item_removed_or_moved(self):
        self.commit_base()
        path = "constitution/sections/20260101-alpha-one.toml"
        self.edit(path, "- {#20260101-machines} Machines; {braces} and {@nope} stay.\n", "")
        self.edit(path, "- {#20260101-machines} 机器；{braces} 与 {@nope} 不变。\n", "")
        self.edit(path, '\n[[items]]\nid = "20260101-machines"\nnumber = "H2"\n', "")
        gone = ("constitution/sections/20260101-alpha-one.toml:16: check 7:"
                " item 20260101-machines of 20260101-alpha-one at base HEAD is gone; repeal it instead")
        self.assertEqual(self.base_errors(), ([gone], ""))
        # Moved to another section: still gone from this one.
        other = "constitution/sections/20260102-beta-one.toml"
        self.edit(other, "Four, see", "Four:\n- {#20260101-machines} Machines, see")
        self.edit(other, "四，见", "四：\n- {#20260101-machines} 机器，见")
        self.write(other, (self.root / other).read_text(encoding="utf-8")
                   + '\n[[items]]\nid = "20260101-machines"\nnumber = "H2"\n')
        self.assertEqual(self.base_errors(), ([gone], ""))

    def test_reused_number_by_replacing_a_section(self):
        self.commit_base()
        (self.root / "constitution/sections/20260102-beta-two.toml").unlink()
        self.write("constitution/sections/20260105-new.toml",
                   BETA_TWO.replace("20260102-beta-two", "20260105-new").replace('"draft"', '"active"'))
        self.edit("constitution/articles/20260101-alpha.toml", "{@20260102-beta-two}", "{@20260105-new}", 2)
        errors, _ = self.base_errors()
        self.assertEqual(errors, ["constitution/sections/20260102-beta-two.toml:1: check 7:"
                                  " existed at base HEAD and is gone; repeal it instead"])

    def test_deleted_repealed_section(self):
        self.activate()
        self.edit("constitution/sections/20260102-beta-two.toml", 'status = "active"',
                  'status = "repealed"\nrepealed = "2026-11"')
        self.edit("constitution/articles/20260101-alpha.toml", "{@20260102-beta-two}", "{@20260102-beta-one}", 2)
        self.commit_base(active=False)
        (self.root / "constitution/sections/20260102-beta-two.toml").unlink()
        errors, _ = self.base_errors()
        self.assertEqual(errors, ["constitution/sections/20260102-beta-two.toml:1: check 7:"
                                  " existed at base HEAD and is gone; repeal it instead"])

    def test_unrepealed_rule(self):
        self.activate()
        self.edit("rules/20260103-rule.toml", 'status = "active"', 'status = "repealed"')
        self.commit_base(active=False)
        self.edit("rules/20260103-rule.toml", 'status = "repealed"', 'status = "active"')
        errors, _ = self.base_errors()
        self.assertEqual(errors, ["rules/20260103-rule.toml:2: check 7:"
                                  " 20260103-rule was repealed at base HEAD and is no longer"])

    def test_back_to_draft(self):
        self.commit_base()
        self.edit("constitution/index.toml", 'status = "active"', 'status = "draft"')
        for name in FILES:
            if "/sections/" in name or name.startswith("rules/"):
                self.edit(name, 'status = "active"', 'status = "draft"')
        errors, _ = self.base_errors()
        self.assertEqual(errors, ["constitution/index.toml:1: check 7:"
                                  " status went back to draft from active at base HEAD"])

    def test_appended_section_and_repeal_pass(self):
        self.commit_base()
        self.write("constitution/sections/20260105-new.toml",
                   BETA_ONE.replace("20260102-beta-one", "20260105-new").replace('number = "1"', 'number = "2"')
                   .replace('"draft"', '"active"'))
        self.edit("constitution/sections/20260102-beta-two.toml", 'status = "active"',
                  'status = "repealed"\nrepealed = "2026-11"')
        self.edit("constitution/articles/20260101-alpha.toml", "{@20260102-beta-two}", "{@20260105-new}", 2)
        self.assertEqual(self.base_errors(), ([], ""))

    def test_draft_base_skips(self):
        self.commit_base(active=False)
        self.edit("constitution/sections/20260102-beta-two.toml", 'number = "1a"', 'number = "2"')
        errors, note = self.base_errors()
        self.assertEqual(errors, [])
        self.assertEqual(note, "base HEAD is draft; numbering not compared")

    def test_base_without_toml_skips(self):
        self.activate()
        self.git("init", "-q")
        self.write("README.md", "x\n")
        self.git("add", "README.md")
        self.git("commit", "-q", "-m", "old")
        errors, note = self.base_errors()
        self.assertEqual(errors, [])
        self.assertEqual(note, "base HEAD has no constitution/index.toml; numbering not compared")

    def test_unknown_base(self):
        self.commit_base()
        errors, src, _ = bc.build(self.root)
        bc.check_base(self.root, "no-such-ref", src, errors)
        self.assertEqual(errors, ["constitution/index.toml:1: check 7: base `no-such-ref` is not a commit"])


if __name__ == "__main__":
    unittest.main()
