"""Tests for check_design_translation on a small design and translation."""

import contextlib
import hashlib
import io
from pathlib import Path
import tempfile
import unittest

from repo_checks import check_design_translation as cdt

ENGLISH = """\
# One

## Goals and non-goals

Text.

```markdown
## Not a heading
```

### Rules, IDs

## Goals and non-goals
"""


def translate(english: str) -> str:
  """Return a translation of ENGLISH that matches it.

  Args:
    english: The English file it was translated from.

  Returns:
    The translation.
  """
  sha = hashlib.sha256(english.encode()).hexdigest()
  return f"""\
<!-- en-sha256: {sha} -->
# 一

## <a id="goals-and-non-goals"></a>目标与非目标

正文。

```markdown
## Not a heading
```

### <a id="rules-ids"></a>规定、ID

## <a id="goals-and-non-goals-1"></a>目标与非目标
"""


class CheckFileTest(unittest.TestCase):
  def test_matching(self):
    self.assertEqual(cdt.check_file(ENGLISH, translate(ENGLISH)), [])

  def test_english_changed(self):
    """A change to the English file only, not its headings, fails."""
    changed = ENGLISH.replace("Text.", "Text, changed.")
    [(line, message)] = cdt.check_file(changed, translate(ENGLISH))
    self.assertEqual(line, 1)
    self.assertIn("the English file changed", message)

  def test_heading_renamed(self):
    changed = ENGLISH.replace("### Rules, IDs", "### Rules and IDs")
    errors = cdt.check_file(changed, translate(changed))
    self.assertEqual(
      errors,
      [(12, "anchor 'rules-ids', the English heading's is 'rules-and-ids'")],
    )

  def test_heading_added_and_level(self):
    changed = ENGLISH + "\n## Open questions\n"
    self.assertEqual(
      cdt.check_file(changed, translate(changed)),
      [
        (
          14,
          "1 heading(s) missing, from the level-2 heading 'open-questions' on",
        )
      ],
    )
    changed = ENGLISH.replace("### Rules", "## Rules")
    self.assertIn(
      (12, "level 3, the English heading's is 2"),
      cdt.check_file(changed, translate(changed)),
    )

  def test_no_hash_line_or_anchor(self):
    translation = translate(ENGLISH).split("\n", 1)[1]
    translation = "<!-- -->\n" + translation.replace(
      '<a id="rules-ids"></a>', ""
    )
    errors = [m for _, m in cdt.check_file(ENGLISH, translation)]
    self.assertTrue(errors[0].startswith("first line is not"))
    self.assertEqual(errors[1], 'does not start with `<a id="rules-ids"></a>`')


class CheckTest(unittest.TestCase):
  def test_tree(self):
    with tempfile.TemporaryDirectory() as tmp:
      root = Path(tmp)
      design = root / "docs/design"
      design.mkdir(parents=True)
      (design / "one.md").write_text(ENGLISH)
      (design / "two.md").write_text(ENGLISH)
      self.assertEqual(
        cdt.check(root),
        ([], "docs/design/zh/ does not exist: no translation to check"),
      )
      (design / "zh").mkdir()
      (design / "zh/one.md").write_text(translate(ENGLISH))
      (design / "zh/three.md").write_text(translate(ENGLISH))
      self.assertEqual(
        cdt.check(root),
        (
          [
            "docs/design/zh/two.md: missing, the translation of"
            " docs/design/two.md",
            "docs/design/zh/three.md: no English file docs/design/three.md",
          ],
          None,
        ),
      )
      (design / "zh/three.md").unlink()
      (design / "zh/two.md").write_text(translate(ENGLISH))
      with contextlib.redirect_stdout(io.StringIO()) as out:
        self.assertEqual(cdt.main(["--root", str(root)]), 0)
      self.assertEqual(out.getvalue(), "docs/design/zh/ matches docs/design/\n")


if __name__ == "__main__":
  unittest.main()
