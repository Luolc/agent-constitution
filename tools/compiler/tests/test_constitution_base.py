"""Tests for check 7.

They run on a temporary git repository against a base commit.
"""

import contextlib
import io
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest
from unittest import mock

from test_constitution_fixture import (
  A,
  B,
  BETA_ONE,
  BETA_TWO,
  SECTIONS,
  Tree,
)

from constitution_compiler import base as cb
from constitution_compiler import build as bc
from constitution_compiler import git_env
from constitution_compiler import sources as cs


class BaseTest(Tree):
  """Check 7 against a base commit in a temporary git repository."""

  def git(self, *args):
    subprocess.run(
      [
        "git",
        "-C",
        str(self.root),
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@example.test",
        *args,
      ],
      check=True,
      capture_output=True,
      env=git_env.clean(),
    )

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
    assert src is not None
    note = cb.check_base(self.root, "HEAD", src, errors)
    return errors, note

  def revision_check(self):
    """Run only the revision part of check 7, on the tree against HEAD.

    Returns:
      The errors it found.
    """
    errors, src, _ = bc.build(self.root)
    self.assertEqual(errors, [])
    assert src is not None
    cb.check_revision(self.root, "HEAD", src, errors)
    return errors

  def test_control_passes(self):
    self.commit_base()
    self.assertEqual(self.base_errors(), ([], ""))

  def test_main_runs_check_with_base(self):
    self.commit_base()
    self.assertEqual(
      bc.main(["--root", str(self.root), "--check", "--base", "HEAD"]), 0
    )
    self.move(f"{B}/01a-beta-two.toml", f"{B}/01b-beta-two.toml")
    self.assertEqual(bc.main(["--root", str(self.root)]), 0)
    with contextlib.redirect_stderr(io.StringIO()) as err:
      self.assertEqual(
        bc.main(["--root", str(self.root), "--check", "--base", "HEAD"]), 1
      )
    self.assertIn(
      "check 7: section beta-two was renumbered from 1a to 1b", err.getvalue()
    )

  def test_renumbered_section(self):
    self.commit_base()
    self.move(f"{B}/01a-beta-two.toml", f"{B}/02-beta-two.toml")
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        f"{B}/02-beta-two.toml:1: check 7:"
        " section beta-two was renumbered from 1a to 2 since base HEAD"
      ],
    )

  def test_renumbered_article_and_item(self):
    self.commit_base()
    self.edit(f"{A}/01-alpha-one.toml", 'number = "2"', 'number = "3"')
    self.move(B, "constitution/articles/03-beta")
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        f"{A}/01-alpha-one.toml:27: check 7: item alpha-one.machines was "
        "renumbered from 2 to 3 since base HEAD",
        "constitution/articles/03-beta/article.toml:1: check 7:"
        " article beta was renumbered from 2 to 3 since base HEAD",
      ],
    )

  def test_section_moved_to_another_article(self):
    self.commit_base()
    self.move(f"{B}/01a-beta-two.toml", f"{A}/02-beta-two.toml")
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        f"{A}/02-beta-two.toml:1: check 7:"
        " section beta-two moved from article beta to alpha since base HEAD"
      ],
    )

  def test_item_gone_or_moved(self):
    self.commit_base()
    path = f"{A}/01-alpha-one.toml"
    text = (self.root / path).read_text(encoding="utf-8")
    machines = text[text.index('[[items]]\nid = "machines"') :]
    self.write(path, text.replace("\n" + machines, ""))
    gone = (
      f"{path}:12: check 7: item alpha-one.machines existed at base HEAD "
      "and is gone; repeal it instead"
    )
    self.assertEqual(self.base_errors(), ([gone], ""))
    # Moved to another section: still gone from this one.
    self.write(
      f"{B}/01-beta-one.toml",
      (self.root / B / "01-beta-one.toml").read_text(encoding="utf-8")
      + "\n"
      + machines,
    )
    self.assertEqual(self.base_errors(), ([gone], ""))

  def test_section_gone_and_number_reused(self):
    self.commit_base()
    (self.root / B / "01a-beta-two.toml").unlink()
    self.write(
      f"{B}/01a-new.toml",
      BETA_TWO.replace('"beta-two"', '"new"').replace('"draft"', '"active"'),
    )
    self.edit(f"{A}/article.toml", "{@beta-two}", "{@new}", 2)
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        f"{B}/01a-beta-two.toml:1: check 7:"
        " section beta-two existed at base HEAD and is gone; repeal it instead"
      ],
    )

  def test_unrepealed_section(self):
    self.activate()
    self.edit(
      f"{B}/01a-beta-two.toml",
      'status = "active"',
      'status = "repealed"\nrepealed = "2026-11"',
    )
    self.edit(f"{A}/article.toml", "{@beta-two}", "{@beta-one}", 2)
    self.commit_base(active=False)
    self.edit(
      f"{B}/01a-beta-two.toml",
      'status = "repealed"\nrepealed = "2026-11"',
      'status = "active"',
    )
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        f"{B}/01a-beta-two.toml:2: check 7:"
        " section beta-two was repealed at base HEAD and is no longer"
      ],
    )

  def test_deleted_rule_must_be_retired(self):
    self.commit_base()
    (self.root / "rules/rule.toml").unlink()
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        "rules/rule.toml:1: check 7: rule rule existed at base HEAD and is gone"
        " without being listed in rules/retired-ids.txt"
      ],
    )
    self.write("rules/retired-ids.txt", "rule\n")
    self.assertEqual(self.base_errors(), ([], ""))

  def test_retired_id_stays_retired(self):
    self.write("rules/retired-ids.txt", "old-rule\n")
    self.commit_base()
    self.write("rules/retired-ids.txt", "")
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        "rules/retired-ids.txt:1: check 7:"
        " old-rule was retired at base HEAD and is no longer listed"
      ],
    )

  def test_unrepealed_rule(self):
    self.activate()
    self.edit("rules/rule.toml", 'status = "active"', 'status = "repealed"')
    self.commit_base(active=False)
    self.edit("rules/rule.toml", 'status = "repealed"', 'status = "active"')
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        "rules/rule.toml:2: check 7: rule rule was repealed at base HEAD and "
        "is no longer"
      ],
    )

  def test_back_to_draft(self):
    self.commit_base()
    self.edit(
      "constitution/index.toml", 'status = "active"', 'status = "draft"'
    )
    for name in SECTIONS + ["rules/rule.toml"]:
      self.edit(name, 'status = "active"', 'status = "draft"', -1)
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        "constitution/index.toml:1: check 7:"
        " status went back to draft from active at base HEAD"
      ],
    )

  def test_appended_section_and_repeal_pass(self):
    self.commit_base()
    self.write(
      f"{A}/02-new.toml",
      BETA_ONE.replace('"beta-one"', '"new"').replace('"draft"', '"active"'),
    )
    self.edit(
      f"{B}/01a-beta-two.toml",
      'status = "active"',
      'status = "repealed"\nrepealed = "2026-11"',
    )
    self.edit(f"{A}/article.toml", "{@beta-two}", "{@new}", 2)
    self.assertEqual(self.base_errors(), ([], ""))

  def test_new_edition_skips(self):
    self.commit_base()
    self.move(f"{B}/01a-beta-two.toml", f"{B}/02-beta-two.toml")
    self.edit(
      "constitution/index.toml", 'edition = "2026-10"', 'edition = "2027-03"'
    )
    errors, note = self.base_errors()
    self.assertEqual(errors, [])
    self.assertEqual(
      note,
      "base HEAD is edition 2026-10, this is edition 2027-03; numbering not "
      "compared",
    )

  def test_earlier_edition(self):
    self.commit_base()
    self.edit(
      "constitution/index.toml", 'edition = "2026-10"', 'edition = "2026-09"'
    )
    self.assertEqual(
      self.base_errors(),
      (
        [
          "constitution/index.toml:2: check 7:"
          " edition 2026-09 is not later than edition 2026-10 at base HEAD"
        ],
        "",
      ),
    )

  def test_main_runs_revision_check(self):
    self.commit_base()
    self.change_text()
    self.assertEqual(bc.main(["--root", str(self.root)]), 0)
    self.assertEqual(
      bc.main(["--root", str(self.root), "--check", "--base", "HEAD"]), 1
    )
    self.set_revision("1", "2")
    self.assertEqual(bc.main(["--root", str(self.root)]), 0)
    self.assertEqual(
      bc.main(["--root", str(self.root), "--check", "--base", "HEAD"]), 0
    )

  def change_text(self):
    self.edit(f"{A}/article.toml", 'en = "First"', 'en = "First one"')

  def set_revision(self, old, new):
    self.edit(
      "constitution/index.toml", f'revision = "{old}"', f'revision = "{new}"'
    )

  def test_text_change_with_higher_revision_passes(self):
    self.commit_base()
    self.change_text()
    self.set_revision("1", "2")
    self.assertEqual(self.revision_check(), [])

  def test_text_change_with_same_revision_fails(self):
    self.commit_base()
    self.change_text()
    self.assertEqual(
      self.revision_check(),
      [
        "constitution/index.toml:3: check 7: the constitution text changed "
        "since base HEAD"
        " but revision 1 is not greater than the base's 1"
      ],
    )

  def test_text_change_with_lower_revision_fails(self):
    self.activate()
    self.set_revision("1", "2")
    self.commit_base(active=False)
    self.change_text()
    self.set_revision("2", "1")
    self.assertEqual(
      self.revision_check(),
      [
        "constitution/index.toml:3: check 7: the constitution text changed "
        "since base HEAD"
        " but revision 1 is not greater than the base's 2"
      ],
    )

  def test_revision_change_without_text_change_fails(self):
    self.commit_base()
    self.set_revision("1", "2")
    self.assertEqual(
      self.revision_check(),
      [
        "constitution/index.toml:3: check 7: revision 2 differs from revision "
        "1 at base HEAD"
        " but the constitution text did not change"
      ],
    )

  def test_nothing_changed_passes(self):
    self.commit_base()
    self.assertEqual(self.revision_check(), [])

  def test_index_text_change_counts(self):
    self.commit_base()
    self.edit(
      "constitution/index.toml",
      'en = "Test Constitution"',
      'en = "The Constitution"',
    )
    self.assertEqual(len(self.revision_check()), 1)
    self.set_revision("1", "2")
    self.assertEqual(self.revision_check(), [])

  def test_added_and_removed_files_count(self):
    self.commit_base()
    self.write(
      f"{A}/02-new.toml",
      BETA_ONE.replace('"beta-one"', '"new"').replace('"draft"', '"active"'),
    )
    self.git("add", "-A")
    self.assertEqual(len(self.revision_check()), 1)
    self.set_revision("1", "2")
    self.assertEqual(self.revision_check(), [])

  def test_changes_outside_the_constitution_text_are_ignored(self):
    self.commit_base()
    self.edit("rules/rule.toml", 'en = "A rule"', 'en = "A changed rule"')
    self.write("constitution/constitution.en.md", "changed\n")
    self.assertEqual(self.revision_check(), [])

  def test_draft_base_text_change_needs_higher_revision(self):
    self.commit_base(active=False)
    self.change_text()
    self.assertEqual(
      self.revision_check(),
      [
        "constitution/index.toml:3: check 7: the constitution text changed "
        "since base HEAD"
        " but revision 0.2 is not greater than the base's 0.2"
      ],
    )
    self.set_revision("0.2", "0.3")
    self.assertEqual(self.revision_check(), [])

  def test_draft_base_revision_without_text_change_fails(self):
    self.commit_base(active=False)
    self.set_revision("0.2", "0.3")
    self.assertEqual(len(self.revision_check()), 1)

  def test_draft_revisions_compare_by_number(self):
    self.edit("constitution/index.toml", 'revision = "0.2"', 'revision = "0.9"')
    self.commit_base(active=False)
    self.change_text()
    self.set_revision("0.9", "0.10")
    self.assertEqual(self.revision_check(), [])
    self.set_revision("0.10", "0.8")
    self.assertEqual(len(self.revision_check()), 1)

  def test_draft_revisions_sort_below_whole_numbers(self):
    revisions = ("0.2", "0.9", "0.10", "1", "2", "10")
    keys = [k for r in revisions if (k := cb.revision_key(r)) is not None]
    self.assertEqual(keys, sorted(keys))
    self.assertEqual(len(set(keys)), 6)

  def test_new_edition_revision_is_not_compared(self):
    self.activate()
    self.set_revision("1", "5")
    self.commit_base(active=False)
    self.change_text()
    self.edit(
      "constitution/index.toml", 'edition = "2026-10"', 'edition = "2027-03"'
    )
    self.set_revision("5", "1")
    self.assertEqual(self.revision_check(), [])

  def name_edition(self, edition):
    index = self.root / "constitution/index.toml"
    text = re.sub(r'edition = ".*"\n', "", index.read_text(encoding="utf-8"))
    index.write_text(
      text.replace("revision =", f'edition = "{edition}"\nrevision ='),
      encoding="utf-8",
    )

  def test_draft_renaming_its_edition_compares_revisions(self):
    """Renaming the edition a draft prepares does not restart its revisions."""
    self.name_edition("2026-10")
    self.commit_base(active=False)
    self.name_edition("2026-11")
    self.assertEqual(
      self.revision_check(),
      [
        "constitution/index.toml:3: check 7: the constitution text changed "
        "since base HEAD"
        " but revision 0.2 is not greater than the base's 0.2"
      ],
    )
    self.set_revision("0.2", "0.3")
    self.assertEqual(self.revision_check(), [])

  def test_approval_compares_with_the_draft(self):
    """The approved text's revision is greater than the draft's."""
    self.name_edition("2026-10")
    self.commit_base(active=False)
    self.edit(
      "constitution/index.toml", 'status = "draft"', 'status = "active"'
    )
    for name in SECTIONS + ["rules/rule.toml"]:
      self.edit(name, 'status = "draft"', 'status = "active"', -1)
    self.assertEqual(
      self.revision_check(),
      [
        "constitution/index.toml:3: check 7: the constitution text changed "
        "since base HEAD"
        " but revision 0.2 is not greater than the base's 0.2"
      ],
    )
    self.set_revision("0.2", "1")
    self.assertEqual(self.revision_check(), [])

  def test_malformed_revision_fails(self):
    self.commit_base(active=False)
    self.change_text()
    self.set_revision("0.2", "0.x")
    self.assertEqual(
      self.revision_check(),
      [
        "constitution/index.toml:3: check 7: revision 0.x is neither `0.<n>` "
        "nor a whole number from 1"
      ],
    )

  def test_base_without_edition(self):
    self.activate()
    self.edit("constitution/index.toml", 'edition = "2026-10"\n', "")
    self.git("init", "-q")
    self.git("add", "-A")
    self.git("commit", "-q", "-m", "base")
    self.edit(
      "constitution/index.toml",
      'revision = "1"',
      'edition = "2026-10"\nrevision = "1"',
    )
    self.assertEqual(
      self.base_errors(),
      (
        [
          "constitution/index.toml:2: check 7:"
          " edition 2026-10 is not later than edition None at base HEAD"
        ],
        "",
      ),
    )

  def test_non_ascii_digit_revision_is_not_compared(self):
    self.activate()
    self.edit("constitution/index.toml", 'revision = "1"', 'revision = "²"')
    self.commit_base(active=False)
    self.edit("constitution/index.toml", 'revision = "²"', 'revision = "1"')
    self.assertEqual(self.base_errors(), ([], ""))

  def test_draft_base_skips(self):
    self.commit_base(active=False)
    self.move(f"{B}/01a-beta-two.toml", f"{B}/02-beta-two.toml")
    errors, note = self.base_errors()
    self.assertEqual(errors, [])
    self.assertEqual(note, "base HEAD is draft; numbering not compared")

  def test_base_without_article_directories_skips(self):
    self.activate()
    self.git("init", "-q")
    self.git("add", "constitution/index.toml")
    self.git("commit", "-q", "-m", "old layout")
    errors, note = self.base_errors()
    self.assertEqual(errors, [])
    self.assertEqual(
      note,
      "base HEAD has no constitution/articles/<NN>-<id>/article.toml;"
      " numbering not compared",
    )

  def test_base_without_toml_skips(self):
    self.activate()
    self.git("init", "-q")
    self.write("README.md", "x\n")
    self.git("add", "README.md")
    self.git("commit", "-q", "-m", "old")
    errors, note = self.base_errors()
    self.assertEqual(errors, [])
    self.assertEqual(
      note, "base HEAD has no constitution/index.toml; numbering not compared"
    )

  def test_unreadable_base_file(self):
    """A file the base lists but git cannot read fails check 7."""
    self.commit_base()
    blob = subprocess.run(
      ["git", "-C", str(self.root), "rev-parse", f"HEAD:{cs.INDEX}"],
      check=True,
      capture_output=True,
      text=True,
      env=git_env.clean(),
    ).stdout.strip()
    (self.root / ".git/objects" / blob[:2] / blob[2:]).unlink()
    with contextlib.redirect_stderr(io.StringIO()) as err:
      self.assertEqual(
        bc.main(["--root", str(self.root), "--check", "--base", "HEAD"]), 1
      )
    self.assertIn(
      f"check 7: `git show HEAD:{cs.INDEX}` failed:", err.getvalue()
    )

  def test_unknown_base(self):
    self.commit_base()
    errors, src, _ = bc.build(self.root)
    assert src is not None
    cb.check_base(self.root, "no-such-ref", src, errors)
    self.assertEqual(
      errors,
      [
        "constitution/index.toml:1: check 7: base `no-such-ref` is not a commit"
      ],
    )


class HookEnvironment(unittest.TestCase):
  """The tests and check 7 leave the repository of a git hook alone.

  They run from a git hook, which in a worktree sets GIT_DIR and
  GIT_INDEX_FILE to the worktree's; they must leave its repository alone.
  One test that runs check 7 covers its git calls and its test helper's.
  """

  def test_repository_of_the_hook_untouched(self):
    tmp = tempfile.TemporaryDirectory()
    self.addCleanup(tmp.cleanup)
    outer = Path(tmp.name)
    for args in (
      ["init", "-q", "main"],
      ["-C", "main", "commit", "-q", "--allow-empty", "-m", "x"],
      ["-C", "main", "worktree", "add", "-q", "../wt"],
    ):
      subprocess.run(
        [
          "git",
          "-C",
          str(outer),
          "-c",
          "user.name=Test",
          "-c",
          "user.email=test@example.test",
          *args,
        ],
        check=True,
        capture_output=True,
        env=git_env.clean(),
      )
    gitdir = outer / "main/.git/worktrees/wt"
    config, index = (
      (outer / "main/.git/config").read_bytes(),
      (gitdir / "index").read_bytes(),
    )
    result = unittest.TestResult()
    with mock.patch.dict(
      os.environ,
      {"GIT_DIR": str(gitdir), "GIT_INDEX_FILE": str(gitdir / "index")},
    ):
      BaseTest("test_main_runs_check_with_base").run(result)
    self.assertEqual((outer / "main/.git/config").read_bytes(), config)
    self.assertEqual((gitdir / "index").read_bytes(), index)
    self.assertTrue(result.wasSuccessful(), result.errors + result.failures)
    self.assertEqual(result.testsRun, 1)


if __name__ == "__main__":
  unittest.main()
