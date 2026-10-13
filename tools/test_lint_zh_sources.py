"""Tests for lint_zh_sources on a small source tree, with the real limae."""

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

import lint_zh_sources as lzs

INDEX = """\
status = "draft"

[title]
en = "Test"
zh = "测试"

[front]
en = '''
Front.
'''
zh = '''
序言。
'''
"""

SECTION = """\
id = "one"

[content]
en = '''
One, under {@amendment-procedure}.
'''
zh = '''
依照{@amendment-procedure}批准。
'''

[[items]]
id = "money"
number = "1"

[items.content]
en = '''
Money.
'''
zh = '''
第一行。
钱。
'''
"""

ARTICLE = 'id = "a"\n\n[title]\nen = "A"\nzh = "边界"\n'
RULE = "id = \"r\"\n\n[content]\nen = '''\nRule.\n'''\nzh = '''\n规则。\n'''\n"

FILES = {
    "constitution/index.toml": INDEX,
    "constitution/articles/01-a/article.toml": ARTICLE,
    "constitution/articles/01-a/01-one.toml": SECTION,
    "rules/r.toml": RULE,
}
S = "constitution/articles/01-a/01-one.toml"


class Sources(unittest.TestCase):

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

    def edit(self, name, old, new):
        text = (self.root / name).read_text(encoding="utf-8")
        self.assertIn(old, text)
        self.write(name, text.replace(old, new, 1))

    def run_lint(self, *extra):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
            code = lzs.main(["--root", str(self.root), *extra])
        return code, out.getvalue()

    def test_clean_tree_passes(self):
        self.assertEqual(self.run_lint(), (0, "limae: 6 zh texts clean\n"))

    def test_findings_name_file_line_and_field(self):
        self.edit(S, "钱。", "钱agent。")
        self.edit("constitution/articles/01-a/article.toml", 'zh = "边界"', 'zh = "边界abc"')
        code, out = self.run_lint()
        self.assertEqual(code, 1)
        lines = out.splitlines()
        self.assertEqual(len(lines), 2, out)
        self.assertTrue(lines[0].startswith("constitution/articles/01-a/01-one.toml:21: items.money.content: error:"
                                            " [zh-typography-4"), out)
        self.assertTrue(lines[1].startswith("constitution/articles/01-a/article.toml:5: title: error:"
                                            " [zh-typography-4"), out)

    def test_fix_changes_only_the_fixed_texts(self):
        self.edit(S, "钱。", "钱agent。")
        self.edit("constitution/articles/01-a/article.toml", 'zh = "边界"', 'zh = "边界abc"')
        self.assertEqual(self.run_lint("--fix")[0], 0)
        self.assertEqual((self.root / S).read_text(encoding="utf-8"), SECTION.replace("钱。", "钱 agent。"))
        self.assertEqual((self.root / "constitution/articles/01-a/article.toml").read_text(encoding="utf-8"),
                         ARTICLE.replace('"边界"', '"边界 abc"'))
        self.assertEqual(self.run_lint()[0], 0)

    def test_citation_between_chinese_gets_no_space(self):
        self.assertEqual(self.run_lint("--fix")[0], 0)
        self.assertIn("依照{@amendment-procedure}批准。", (self.root / S).read_text(encoding="utf-8"))

    def test_citation_is_judged_as_its_rendering(self):
        # Rendered, `agent{@x}` reads `agent第十条`, which needs a space;
        # the braces alone would hide that from limae.
        self.edit(S, "依照{@amendment-procedure}批准。", "依照agent{@amendment-procedure}批准。")
        code, out = self.run_lint()
        self.assertEqual(code, 1)
        self.assertIn(f"{S}:8: content: error: [zh-typography-4", out)
        self.run_lint("--fix")
        self.assertIn("依照 agent {@amendment-procedure}批准。", (self.root / S).read_text(encoding="utf-8"))

    def test_unsupported_form_is_refused(self):
        self.edit("rules/r.toml", "zh = '''\n规则。\n'''", 'zh = """\n规则。\n"""')
        self.assertEqual(self.run_lint(), (2, "rules/r.toml:1: a `zh` text is not `zh = \"...\"` on one line"
                                              " or `zh = '''` on its own line\n"))


if __name__ == "__main__":
    unittest.main()
