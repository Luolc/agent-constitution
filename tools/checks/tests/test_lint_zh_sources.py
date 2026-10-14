"""Tests for lint_zh_sources on a small source tree, with the real limae."""

import contextlib
import dataclasses
import io
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from repo_checks import lint_zh_sources as lzs

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
  "constitution/2026-10/index.toml": INDEX,
  "constitution/2026-10/articles/01-a/article.toml": ARTICLE,
  "constitution/2026-10/articles/01-a/01-one.toml": SECTION,
  "rules/r.toml": RULE,
}
S = "constitution/2026-10/articles/01-a/01-one.toml"


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

  def test_limae_only_from_the_environment(self):
    """A limae on PATH is never run when the environment has none."""
    with tempfile.TemporaryDirectory() as tmp:
      bin_dir, scripts = Path(tmp, "bin"), Path(tmp, "scripts")
      bin_dir.mkdir()
      scripts.mkdir()
      ran = Path(tmp, "ran")
      fake = bin_dir / "limae"
      fake.write_text(f"#!/bin/sh\n: > {ran}\n", encoding="utf-8")
      fake.chmod(0o755)
      with (
        mock.patch.dict(os.environ, {"PATH": str(bin_dir)}),
        mock.patch("sysconfig.get_path", return_value=str(scripts)),
      ):
        code, out = self.run_lint()
      self.assertEqual(code, 2)
      self.assertIn("limae is not installed in this Python environment", out)
      self.assertFalse(ran.exists())

  def test_invalid_toml(self):
    self.edit(S, 'id = "one"', "id = one")
    code, out = self.run_lint()
    self.assertEqual(code, 2)
    self.assertTrue(out.startswith(f"{S}:1: not valid TOML: "), out)

  def test_clean_tree_passes(self):
    self.assertEqual(self.run_lint(), (0, "limae: 6 zh texts clean\n"))

  def test_findings_name_file_line_and_field(self):
    self.edit(S, "钱。", "钱agent。")
    self.edit(
      "constitution/2026-10/articles/01-a/article.toml",
      'zh = "边界"',
      'zh = "边界abc"',
    )
    code, out = self.run_lint()
    self.assertEqual(code, 1)
    lines = out.splitlines()
    self.assertEqual(len(lines), 2, out)
    self.assertTrue(
      lines[0].startswith(
        "constitution/2026-10/articles/01-a/01-one.toml:21:"
        " items.money.content: error:"
        " [zh-typography-4"
      ),
      out,
    )
    self.assertTrue(
      lines[1].startswith(
        "constitution/2026-10/articles/01-a/article.toml:5: title: error:"
        " [zh-typography-4"
      ),
      out,
    )

  def test_fix_changes_only_the_fixed_texts(self):
    self.edit(S, "钱。", "钱agent。")
    self.edit(
      "constitution/2026-10/articles/01-a/article.toml",
      'zh = "边界"',
      'zh = "边界abc"',
    )
    self.assertEqual(self.run_lint("--fix")[0], 0)
    self.assertEqual(
      (self.root / S).read_text(encoding="utf-8"),
      SECTION.replace("钱。", "钱 agent。"),
    )
    self.assertEqual(
      (self.root / "constitution/2026-10/articles/01-a/article.toml").read_text(
        encoding="utf-8"
      ),
      ARTICLE.replace('"边界"', '"边界 abc"'),
    )
    self.assertEqual(self.run_lint()[0], 0)

  def test_citation_between_chinese_gets_no_space(self):
    self.assertEqual(self.run_lint("--fix")[0], 0)
    self.assertIn(
      "依照{@amendment-procedure}批准。",
      (self.root / S).read_text(encoding="utf-8"),
    )

  def test_citation_is_judged_as_its_rendering(self):
    # Rendered, `agent{@x}` reads `agent第十条`, which needs a space;
    # the braces alone would hide that from limae.
    self.edit(
      S,
      "依照{@amendment-procedure}批准。",
      "依照agent{@amendment-procedure}批准。",
    )
    code, out = self.run_lint()
    self.assertEqual(code, 1)
    self.assertIn(f"{S}:8: content: error: [zh-typography-4", out)
    self.run_lint("--fix")
    self.assertIn(
      "依照 agent {@amendment-procedure}批准。",
      (self.root / S).read_text(encoding="utf-8"),
    )

  def test_fix_puts_citations_back_in_order(self):
    self.edit(
      S,
      "依照{@amendment-procedure}批准。",
      "依照{@amendment-procedure}和{@must-ask}批准agent。",
    )
    self.assertEqual(self.run_lint("--fix")[0], 0)
    self.assertEqual(
      (self.root / S).read_text(encoding="utf-8"),
      SECTION.replace(
        "依照{@amendment-procedure}批准。",
        "依照{@amendment-procedure}和{@must-ask}批准 agent。",
      ),
    )

  def test_write_back_refuses_a_change_outside_the_texts(self):
    texts, problem = lzs.texts(self.root, Path(S))
    self.assertIsNone(problem)
    source = (self.root / S).read_text(encoding="utf-8")
    # The content's text written over another key's: the same text over
    # an `en` text, and a fix over the item's `zh` text.
    for other, fixed in [
      ("One, under", texts[0].value),
      ("第一行。", "改了。"),
    ]:
      with self.subTest(other):
        start = source.index(other)
        misplaced = dataclasses.replace(
          texts[0], start=start, end=source.index("\n", start)
        )
        self.assertEqual(
          lzs.write_back(self.root, Path(S), [(misplaced, fixed)]),
          f"{S}:1: writing the fixes back would change more than the fixed"
          " texts; file left as it was",
        )
        self.assertEqual((self.root / S).read_text(encoding="utf-8"), SECTION)

  def test_unsupported_form_is_refused(self):
    self.edit("rules/r.toml", "zh = '''\n规则。\n'''", 'zh = """\n规则。\n"""')
    self.assertEqual(
      self.run_lint(),
      (
        2,
        'rules/r.toml:1: a `zh` text is not `zh = "..."` on one line'
        " or `zh = '''` on its own line\n",
      ),
    )


if __name__ == "__main__":
  unittest.main()
