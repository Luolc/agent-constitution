"""Tests for checks 1 to 6.

Each check fails on a broken tree and passes on its control.
"""

import contextlib
import io
import os
import subprocess
import sys
import unittest

from test_constitution_fixture import (
  A,
  ALPHA_ONE,
  B,
  BETA,
  BETA_ONE,
  Tree,
)

from constitution_compiler import build as bc
from constitution_compiler import checks as cc
from constitution_compiler import model as cm
from constitution_compiler import read as cr


class CheckTest(Tree):
  def test_controls_pass(self):
    self.assertEqual(self.errors(), [])

  def test_1_invalid_toml(self):
    self.edit(f"{B}/article.toml", "[title]", "[title")
    errors = self.errors()
    self.assertEqual(len(errors), 1, errors)
    self.assertTrue(
      errors[0].startswith(f"{B}/article.toml:1: check 1: not valid TOML: "),
      errors,
    )

  def test_1_missing_and_unknown_key(self):
    self.edit(f"{B}/01-beta-one.toml", 'status = "draft"', 'state = "draft"')
    errors = self.errors()
    self.assertIn(f"{B}/01-beta-one.toml:1: check 1: lacks `status`", errors)
    self.assertIn(
      f"{B}/01-beta-one.toml:2: check 1: unknown key `state`", errors
    )

  def test_1_unknown_keys_in_sorted_order(self):
    """The order does not depend on the hash seed of the run."""
    self.edit(
      f"{B}/01-beta-one.toml",
      'status = "draft"',
      'zeta = 1\nalpha = 2\nstatus = "draft"',
    )
    runs = {
      subprocess.run(
        [
          sys.executable,
          "-m",
          "constitution_compiler.build",
          "--root",
          str(self.root),
          "--check",
        ],
        capture_output=True,
        text=True,
        env=os.environ | {"PYTHONHASHSEED": str(seed)},
      ).stderr
      for seed in range(10)
    }
    self.assertEqual(len(runs), 1, runs)
    lines = runs.pop().splitlines()
    self.assertEqual(
      [line.split(": ", 2)[2] for line in lines if "unknown key" in line],
      ["unknown key `alpha`", "unknown key `zeta`"],
    )

  def test_1_number_field_is_gone(self):
    self.edit(
      f"{B}/01-beta-one.toml",
      'status = "draft"',
      'number = "1"\nstatus = "draft"',
    )
    self.assertFails(1, f"{B}/01-beta-one.toml:2", "unknown key `number`")

  def test_1_item_lacks_status(self):
    self.edit(
      f"{A}/01-alpha-one.toml",
      'number = "1"\nstatus = "draft"\n',
      'number = "1"\n',
    )
    # The line of the item's `[[items]]` header.
    self.assertFails(
      1, f"{A}/01-alpha-one.toml:12", "`items[0]` lacks `status`"
    )

  def test_1_languages_differ(self):
    self.edit(f"{B}/article.toml", 'zh = "第二"', 'fr = "Deuxième"')
    self.assertFails(
      1,
      f"{B}/article.toml:3",
      "`title` has languages ['en', 'fr'], index.toml lists ['en', 'zh']",
    )

  def test_1_bad_article_directory_name(self):
    self.move(B, "constitution/2026-10/articles/2-beta")
    self.assertFails(
      1,
      "constitution/2026-10/articles/2-beta:1",
      "article directory name is not two digits from 01, a hyphen and the ID "
      "(04-<id>)",
    )

  def test_1_article_number_has_no_suffix(self):
    self.move(B, "constitution/2026-10/articles/01a-beta")
    self.assertFails(
      1,
      "constitution/2026-10/articles/01a-beta:1",
      "article directory name is not two digits from 01, a hyphen and the ID "
      "(04-<id>)",
    )

  def test_1_bad_section_file_name(self):
    self.move(f"{B}/01a-beta-two.toml", f"{B}/00-beta-two.toml")
    self.assertFails(
      1,
      f"{B}/00-beta-two.toml:1",
      "section file name is not two digits from 01, an optional"
      " lowercase letter, a hyphen and the ID (02-<id>, 02a-<id>),"
      " then `.toml`",
    )

  def test_1_misplaced_files(self):
    self.write("constitution/2026-10/articles/gamma.toml", BETA)
    self.write("constitution/2026-10/sections/gamma-one.toml", BETA_ONE)
    self.write(f"{B}/notes.md", "x\n")
    self.write(f"{B}/old/01-beta-three.toml", BETA_ONE)
    self.assertEqual(
      self.errors(),
      [
        "constitution/2026-10/sections:1: check 1: sections belong in their"
        " article's "
        "directory,"
        " constitution/2026-10/articles/<NN>-<id>/<NN>[a-z]-<id>.toml",
        f"{B}/notes.md:1: check 1: not a section file; an article's directory "
        "holds only its"
        " article.toml and <NN>[a-z]-<id>.toml section files",
        f"{B}/old:1: check 1: not a section file; an article's directory holds "
        "only its"
        " article.toml and <NN>[a-z]-<id>.toml section files",
        "constitution/2026-10/articles/gamma.toml:1: check 1: not an article "
        "directory; an article is"
        " constitution/2026-10/articles/<NN>-<id>/ with its article.toml and"
        " section "
        "files",
      ],
    )

  def test_hidden_files_are_skipped(self):
    self.write("constitution/2026-10/articles/.DS_Store", "x")
    self.write(f"{B}/.01-beta-one.toml.swp", "x")
    self.assertEqual(self.errors(), [])

  def test_1_article_toml_missing(self):
    (self.root / B / "article.toml").unlink()
    self.assertFails(1, f"{B}:1", "article directory lacks `article.toml`")

  def test_1_id_differs_from_name(self):
    self.edit(f"{B}/01a-beta-two.toml", 'id = "beta-two"', 'id = "beta-2"')
    self.assertFails(
      1,
      f"{B}/01a-beta-two.toml:1",
      "`id` must be `beta-two`, as in the name `01a-beta-two.toml`",
    )
    self.edit(f"{B}/01a-beta-two.toml", 'id = "beta-2"', 'id = "beta-two"')
    self.edit(f"{B}/article.toml", 'id = "beta"', 'id = "gamma"')
    self.assertFails(
      1, f"{B}/article.toml:1", "`id` must be `beta`, as in the name `02-beta`"
    )

  def test_1_bad_item_number(self):
    self.edit(f"{A}/01-alpha-one.toml", 'number = "2"', 'number = "H2"')
    self.assertFails(
      1,
      f"{A}/01-alpha-one.toml:27",
      "`items[1]` `number` `H2` is not an integer with an optional letter "
      "suffix (2, 2a)",
    )

  def test_1_status_against_index(self):
    self.edit(f"{B}/01-beta-one.toml", 'status = "draft"', 'status = "active"')
    self.assertFails(
      1,
      f"{B}/01-beta-one.toml:2",
      "a section is active while index.toml is draft",
    )
    self.edit(f"{B}/01-beta-one.toml", 'status = "active"', 'status = "draft"')
    self.edit(
      f"{A}/01-alpha-one.toml",
      'number = "2"\nstatus = "draft"',
      'number = "2"\nstatus = "active"',
    )
    # The item's own `status`, not the section's on line 2.
    self.assertFails(
      1,
      f"{A}/01-alpha-one.toml:28",
      "`items[1]` is active while index.toml is draft",
    )

  def test_1_draft_rule_under_active_index(self):
    """A rule may stay draft once the constitution is active; a section not."""
    self.activate()
    self.edit("rules/10.toml", 'status = "active"', 'status = "draft"')
    self.assertEqual(self.errors(), [])
    self.edit(f"{B}/01-beta-one.toml", 'status = "active"', 'status = "draft"')
    self.assertFails(
      1,
      f"{B}/01-beta-one.toml:2",
      "a section is draft while index.toml is active",
    )

  def test_1_edition(self):
    index = "constitution/2026-10/index.toml"
    for bad in ("2026-13", "2026-10-20"):
      self.edit(index, 'edition = "2026-10"', f'edition = "{bad}"')
      self.assertFails(
        1, f"{index}:2", f"`edition` must be `YYYY-MM`, got `{bad}`"
      )
      self.edit(index, f'edition = "{bad}"', 'edition = "2026-10"')
    self.assertEqual(self.errors(), [])
    self.edit(index, 'edition = "2026-10"\n', "")
    self.assertFails(1, f"{index}:1", "lacks `edition`")
    self.activate()
    self.assertFails(1, f"{index}:1", "lacks `edition`")

  def test_1_audience(self):
    self.edit(
      f"{B}/01a-beta-two.toml",
      'status = "draft"',
      'status = "draft"\naudience = "people"',
    )
    self.assertFails(
      1,
      f"{B}/01a-beta-two.toml:3",
      "a section `audience` must be `maintainers`",
    )
    self.edit(
      f"{B}/01a-beta-two.toml",
      'audience = "people"',
      'audience = "maintainers"',
    )
    self.edit(
      f"{A}/01-alpha-one.toml",
      'id = "money"',
      'id = "money"\naudience = "agents"',
    )
    self.assertFails(
      1,
      f"{A}/01-alpha-one.toml:14",
      "`items[0]` `audience` must be `maintainers`",
    )

  def test_1_repealed_date(self):
    self.activate()
    self.edit(
      f"{B}/01-beta-one.toml",
      'status = "active"',
      'status = "repealed"\nrepealed = "2026-13"',
    )
    self.assertFails(
      1,
      f"{B}/01-beta-one.toml:3",
      "`repealed` must be `YYYY-MM`, got `2026-13`",
    )

  def test_1_repealed_without_status(self):
    self.activate()
    self.edit(
      f"{B}/01-beta-one.toml",
      'status = "active"',
      'status = "active"\nrepealed = "2026-11"',
    )
    self.assertFails(
      1, f"{B}/01-beta-one.toml:3", "has `repealed` but status is active"
    )

  def test_2_id_format(self):
    self.write(
      "constitution/2026-10/articles/03-Gamma/article.toml",
      BETA.replace('"beta"', '"Gamma"'),
    )
    self.assertFails(
      2,
      "constitution/2026-10/articles/03-Gamma/article.toml:1",
      "`Gamma` is not an ID: lowercase words joined by hyphens",
    )

  def test_2_duplicate_in_constitution(self):
    self.write(
      f"{B}/02-alpha-one.toml", BETA_ONE.replace('"beta-one"', '"alpha-one"')
    )
    self.assertFails(
      2,
      f"{B}/02-alpha-one.toml:1",
      f"alpha-one is already defined at {A}/01-alpha-one.toml:1",
    )

  def test_2_item_ids_are_per_section(self):
    self.edit(f"{A}/01-alpha-one.toml", 'id = "machines"', 'id = "money"')
    self.assertFails(
      2,
      f"{A}/01-alpha-one.toml:13",
      f"money is already defined at {A}/01-alpha-one.toml:13",
    )
    self.edit(
      f"{A}/01-alpha-one.toml",
      'id = "money"\nnumber = "2"',
      'id = "machines"\nnumber = "2"',
    )
    # The same item ID in another section, and a slug like a section's ID,
    # are fine.
    self.write(
      f"{B}/02-beta-three.toml",
      ALPHA_ONE.replace('"alpha-one"', '"beta-three"'),
    )
    self.edit("rules/10.toml", 'slug = "new-rule"', 'slug = "beta-one"')
    self.assertEqual(self.errors(), [])

  def test_3_duplicate_section_number(self):
    self.move(f"{B}/01a-beta-two.toml", f"{B}/01-beta-two.toml")
    self.assertFails(
      3,
      f"{B}/01-beta-two.toml:1",
      f"section of beta number 1 is already used at {B}/01-beta-one.toml:1",
    )

  def test_3_duplicate_article_number(self):
    self.move(B, "constitution/2026-10/articles/01-beta")
    self.assertFails(
      3,
      "constitution/2026-10/articles/01-beta/article.toml:1",
      f"article number 1 is already used at {A}/article.toml:1",
    )

  def test_3_duplicate_item_number(self):
    self.edit(f"{A}/01-alpha-one.toml", 'number = "2"', 'number = "1"')
    self.assertFails(
      3,
      f"{A}/01-alpha-one.toml:14",
      f"item of alpha-one number 1 is already used at {A}/01-alpha-one.toml:14",
    )

  def test_3_repealed_article_with_live_section(self):
    self.activate()
    self.edit(f"{B}/article.toml", "[title]", 'repealed = "2026-11"\n\n[title]')
    self.assertFails(
      3,
      f"{B}/01-beta-one.toml:2",
      "beta-one is active but its article beta is repealed",
    )

  def test_4_reference_set(self):
    self.edit(
      "constitution/2026-10/index.toml",
      "序言引用{@beta}。",
      "序言引用{@alpha}。",
    )
    self.assertEqual(
      self.errors(),
      [
        "constitution/2026-10/index.toml:12: check 4: en `front` cites beta;"
        " zh does "
        "not",
        "constitution/2026-10/index.toml:15: check 4: zh `front` cites alpha;"
        " en does "
        "not",
      ],
    )

  def test_4_item_reference_set(self):
    self.edit(f"{A}/01-alpha-one.toml", "钱。", "钱，见{@alpha}。")
    self.assertFails(
      4,
      f"{A}/01-alpha-one.toml:22",
      "zh `money.content` cites alpha; en does not",
    )

  def test_5_on_its_own(self):
    """Check 5 needs no other check to have run before it."""
    errors = cm.Errors()
    src = cr.load(self.root, errors)
    assert src is not None
    cc.check_refs(src, errors)
    self.assertEqual(errors, [])

  def test_5_undefined(self):
    self.edit(
      f"{B}/01-beta-one.toml", "{@alpha-one.money}", "{@alpha-one.cash}", 2
    )
    self.assertFails(
      5, f"{B}/01-beta-one.toml:6", "cites undefined alpha-one.cash"
    )

  def test_5_repealed(self):
    self.activate()
    self.edit(
      f"{B}/01-beta-one.toml",
      'status = "active"',
      'status = "repealed"\nrepealed = "2026-11"',
    )
    errors = self.errors()
    self.assertIn(
      f"{A}/01-alpha-one.toml:6: check 5: cites repealed beta-one", errors
    )
    self.assertIn(
      "rules/10.toml:19: check 5: cites repealed constitution:beta-one",
      errors,
    )

  def test_5_repealed_item(self):
    self.activate()
    self.edit(
      f"{A}/01-alpha-one.toml",
      'number = "1"\nstatus = "active"',
      'number = "1"\nstatus = "repealed"\nrepealed = "2026-11"',
    )
    errors = self.errors()
    self.assertIn(
      f"{B}/01-beta-one.toml:6: check 5: cites repealed alpha-one.money", errors
    )
    self.assertIn(
      "rules/10.toml:8: check 5: authority cites repealed alpha-one.money",
      errors,
    )

  def test_5_constitution_cites_with_namespace(self):
    self.edit(f"{B}/01-beta-one.toml", "{@alpha}", "{@rule:10}", 2)
    self.assertFails(
      5,
      f"{B}/01-beta-one.toml:6",
      "cites rule:10; the constitution cites only itself, without a namespace",
    )

  def test_5_rule_cites_without_namespace(self):
    self.edit("rules/10.toml", "{@rule:3}", "{@3}", 2)
    self.rehash("rules/10.toml")
    self.assertFails(
      5, "rules/10.toml:19", "cites 3 without `constitution:` or `rule:`"
    )

  def test_1_no_edition_directory(self):
    (self.root / "constitution/2026-10/index.toml").unlink()
    self.assertFails(
      1,
      "constitution:1",
      "no edition directory, constitution/<YYYY-MM>/ with its index.toml",
    )

  def test_1_two_edition_directories(self):
    self.write(
      "constitution/2027-03/index.toml",
      (self.root / "constitution/2026-10/index.toml").read_text(
        encoding="utf-8"
      ),
    )
    self.assertFails(
      1,
      "constitution:1",
      "more than one edition directory:"
      " constitution/2026-10, constitution/2027-03",
    )

  def test_6_outputs_match(self):
    self.assertEqual(bc.main(["--root", str(self.root)]), 0)
    errors = cm.Errors()
    _, src, outputs = bc.build(self.root)
    assert src is not None
    bc.check_outputs(self.root, src, outputs, errors)
    self.assertEqual(errors, [])

  def test_6_stale_output(self):
    self.assertEqual(bc.main(["--root", str(self.root)]), 0)
    path = self.root / "constitution/2026-10/generated/constitution.en.md"
    path.write_bytes(path.read_bytes().replace(b"Money.", b"Money!"))
    errors = cm.Errors()
    _, src, outputs = bc.build(self.root)
    assert src is not None
    bc.check_outputs(self.root, src, outputs, errors)
    self.assertEqual(
      errors,
      [
        "constitution/2026-10/generated/constitution.en.md:12: check 6:"
        " differs from a"
        " fresh build; run `uv run constitution-build`"
      ],
    )
    with contextlib.redirect_stderr(io.StringIO()):
      self.assertEqual(bc.main(["--root", str(self.root), "--check"]), 1)


if __name__ == "__main__":
  unittest.main()
