"""Tests for check_instruction_changes on a temporary git repository.

A base commit with one file of each kind, then a change on top of it.
"""

import contextlib
import io
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

from repo_checks import check_instruction_changes as cic
from repo_checks import git_env

BASE = {
  "CLAUDE.md": "claude\n",
  "rules/a.toml": "rule\n",
  "constitution/2026-10/articles/01-a/article.toml": "article\n",
  "constitution/2026-10/generated/constitution.en.md": "generated\n",
  "tools/build_constitution.py": "tool\n",
  "docs/design/README.md": "design\n",
}


class Change(unittest.TestCase):
  def setUp(self):
    tmp = tempfile.TemporaryDirectory()
    self.addCleanup(tmp.cleanup)
    self.root = Path(tmp.name)
    self.git("init", "-q")
    for name, text in BASE.items():
      self.write(name, text)
    self.commit("base")
    self.git("tag", "base")

  def git(self, *args):
    # No user-level ignore file: it may ignore paths these tests change.
    # No automatic maintenance: run in the background after a commit, it can
    # still be writing .git/objects/pack when the test removes the directory.
    subprocess.run(
      [
        "git",
        "-C",
        str(self.root),
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.test",
        "-c",
        "maintenance.auto=false",
        "-c",
        "core.excludesFile=/dev/null",
        *args,
      ],
      check=True,
      capture_output=True,
      env=git_env.clean(),
    )

  def write(self, name, text):
    path = self.root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")

  def commit(self, message):
    self.git("add", "-A")
    self.git("commit", "-q", "-m", message)

  def run_check(self, *extra):
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
      code = cic.main(["--root", str(self.root), "--base", "base", *extra])
    return code, out.getvalue()

  def change(self, *names):
    for name in names:
      self.write(name, "changed\n")
    self.commit("change")

  def test_instructions_only_pass(self):
    self.change(
      "CLAUDE.md",
      "sub/CLAUDE.md",
      ".claude/skills/x/SKILL.md",
      "rules/a.toml",
      "constitution/2026-10/articles/01-a/article.toml",
      "skills/x/SKILL.md",
    )
    self.assertEqual(
      self.run_check(),
      (
        0,
        "instruction-only check: instruction files only since base"
        " (6 instruction, 0 other, 0 generated, 0 hook and CI)\n",
      ),
    )

  def test_other_files_only_pass(self):
    self.change(
      "tools/build_constitution.py",
      "docs/design/README.md",
      "README.md",
    )
    self.assertEqual(
      self.run_check(),
      (
        0,
        "instruction-only check: other files only since base"
        " (0 instruction, 3 other, 0 generated, 0 hook and CI)\n",
      ),
    )

  def test_generated_with_sources_pass(self):
    self.change(
      "constitution/2026-10/articles/01-a/article.toml",
      "constitution/2026-10/generated/constitution.en.md",
    )
    self.assertEqual(self.run_check()[0], 0)

  def test_generated_with_tools_pass(self):
    self.change(
      "tools/build_constitution.py",
      "constitution/2026-10/generated/constitution.en.md",
    )
    self.assertEqual(
      self.run_check(),
      (
        0,
        "instruction-only check: other files only since base"
        " (0 instruction, 1 other, 1 generated, 0 hook and CI)\n"
        "generated files:\n  M"
        " constitution/2026-10/generated/constitution.en.md\n",
      ),
    )

  def test_hook_and_ci_files_go_with_either_side(self):
    hooks = (".pre-commit-config.yaml", ".github/workflows/ci.yml")
    for names, code in [
      ([*hooks, "statute.toml"], 0),
      ([*hooks, "tools/build_constitution.py"], 0),
      ([*hooks, "statute.toml", "tools/build_constitution.py"], 1),
    ]:
      with self.subTest(names):
        self.git("reset", "-q", "--hard", "base")
        self.change(*names)
        code_out = self.run_check()
        self.assertEqual(code_out[0], code, code_out[1])
        self.assertIn(
          "hook and CI files:\n  A .github/workflows/ci.yml\n"
          "  A .pre-commit-config.yaml\n",
          code_out[1],
        )

  def test_hook_and_ci_files_are_other_unless_declared(self):
    self.change(".pre-commit-config.yaml", "statute.toml")
    own = ("--instructions", r"policy/.+")
    self.assertEqual(self.run_check(*own)[0], 1)
    self.assertEqual(
      self.run_check(*own, "--hooks-and-ci", r"\.pre-commit-config\.yaml")[0],
      0,
    )

  def test_mixed_fails(self):
    self.change(
      "rules/a.toml",
      "constitution/2026-10/generated/constitution.en.md",
      "docs/design/README.md",
    )
    self.assertEqual(
      self.run_check(),
      (
        1,
        "instruction-only check: this change mixes instruction files"
        " with other files since base; split it into two changes\n"
        "instruction files:\n  M rules/a.toml\n"
        "generated files:\n  M"
        " constitution/2026-10/generated/constitution.en.md\n"
        "other files:\n  M docs/design/README.md\n",
      ),
    )

  def test_each_generic_path_counts_as_instructions(self):
    for name in [
      "CLAUDE.md",
      "sub/AGENTS.md",
      ".agents/skills/x/SKILL.md",
      "sub/.claude/skills/x/SKILL.md",
      ".claude/agents/x.md",
      ".claude/commands/x.md",
      ".claude/settings.json",
      ".claude/settings.local.json",
      ".claude/hooks/x.sh",
      ".codex/config.toml",
    ]:
      with self.subTest(name):
        self.git("reset", "-q", "--hard", "base")
        self.change(name, "docs/design/README.md")
        self.assertEqual(self.run_check()[0], 1)

  def test_each_statute_path_counts_as_instructions(self):
    for name in [
      "statute.toml",
      "statute.lock",
      ".statute/rules/1.toml",
      ".statute/.gitignore",
      "AGENTS.legacy.md",
      "sub/CLAUDE.legacy.md",
    ]:
      with self.subTest(name):
        self.git("reset", "-q", "--hard", "base")
        self.change(name)
        self.assertEqual(self.run_check()[0], 0)
        self.git("reset", "-q", "--hard", "base")
        self.change(name, "docs/design/README.md")
        self.assertEqual(self.run_check()[0], 1)

  def test_checks_file_goes_with_either_side(self):
    for names, code in [
      (["statute-checks.toml", "crates/statute/Cargo.toml"], 0),
      (["statute-checks.toml", "statute.toml"], 0),
      (["statute.toml", "crates/statute/Cargo.toml"], 1),
    ]:
      with self.subTest(names):
        self.git("reset", "-q", "--hard", "base")
        self.change(*names)
        self.assertEqual(self.run_check()[0], code)

  def test_lock_file_changed_only_in_statute_entries_goes_with_either_side(
    self,
  ):
    lock = 'statute = "{}"\n\n[constitution]\nedition = "{}"\n{}'
    command = '\n[[commands]]\nrule = "R5"\nrun = "./check.sh"\ntools = []\n'
    self.write("statute.lock", lock.format("0.1.17", "2026-10", ""))
    self.commit("lock")
    self.git("tag", "-f", "base")
    for name, text, code in [
      ("version", lock.format("0.1.18", "2026-10", ""), 0),
      ("commands", lock.format("0.1.17", "2026-10", command), 0),
      ("beyond", lock.format("0.1.18", "2026-11", ""), 1),
      ("not toml", "changed\n", 1),
    ]:
      with self.subTest(name):
        self.git("reset", "-q", "--hard", "base")
        self.write("statute.lock", text)
        self.change("crates/statute/Cargo.toml")
        code_out = self.run_check()
        self.assertEqual(code_out[0], code, code_out[1])
        group = "instruction" if code else "generated"
        self.assertIn(f"{group} files:\n  M statute.lock\n", code_out[1])

  def test_lock_file_stays_instructions_with_repo_declared_patterns(self):
    lock = 'statute = "{}"\n\n[constitution]\nedition = "2026-10"\n'
    self.write("statute.lock", lock.format("0.1.17"))
    self.commit("lock")
    self.git("tag", "-f", "base")
    self.write("statute.lock", lock.format("0.1.18"))
    self.change("crates/statute/Cargo.toml")
    self.assertEqual(self.run_check()[0], 0)
    self.assertEqual(self.run_check("--instructions", r"policy/.+")[0], 1)

  def test_checks_file_counts_as_instructions_unless_declared(self):
    self.change("statute-checks.toml", "crates/statute/Cargo.toml")
    own = ("--instructions", r"policy/.+")
    self.assertEqual(self.run_check(*own)[0], 1)
    self.assertEqual(
      self.run_check(*own, "--generated", r"statute-checks\.toml")[0], 0
    )

  def test_compiled_agents_file_goes_with_either_side(self):
    for names, code in [
      (["AGENTS.md", "crates/statute/src/compile.rs"], 0),
      (["AGENTS.md", "statute.toml"], 0),
      (["sub/AGENTS.md", "crates/statute/src/compile.rs"], 1),
    ]:
      with self.subTest(names):
        self.git("reset", "-q", "--hard", "base")
        self.change(*names)
        self.assertEqual(self.run_check()[0], code)

  def test_statute_paths_count_with_repo_declared_patterns(self):
    self.change("statute.toml", "docs/design/README.md")
    self.assertEqual(self.run_check("--instructions", r"policy/.+")[0], 1)

  def test_rename_of_instruction_file_to_other_path_fails(self):
    self.git("mv", "CLAUDE.md", "docs/agents.md")
    self.commit("rename")
    code, out = self.run_check()
    self.assertEqual(code, 1)
    self.assertIn("instruction files:\n  D CLAUDE.md\n", out)
    self.assertIn("other files:\n  A docs/agents.md\n", out)

  def test_repo_declared_patterns_replace_this_repos(self):
    self.change("rules/a.toml", "docs/design/README.md")
    self.assertEqual(self.run_check()[0], 1)
    self.assertEqual(self.run_check("--instructions", r"policy/.+")[0], 0)

  def test_unknown_base(self):
    with contextlib.redirect_stderr(io.StringIO()):
      self.assertEqual(
        cic.main(["--root", str(self.root), "--base", "nope"]), 2
      )


class HookEnvironment(unittest.TestCase):
  """The tests and the script leave the repository of a git hook alone.

  They run from a git hook, which in a worktree sets GIT_DIR and
  GIT_INDEX_FILE to the worktree's; they must leave its repository alone.
  One test of the script covers its git calls and its test helper's.
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
          "-c",
          "maintenance.auto=false",
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
      Change("test_instructions_only_pass").run(result)
    self.assertEqual((outer / "main/.git/config").read_bytes(), config)
    self.assertEqual((gitdir / "index").read_bytes(), index)
    self.assertTrue(result.wasSuccessful(), result.errors + result.failures)
    self.assertEqual(result.testsRun, 1)


if __name__ == "__main__":
  unittest.main()
