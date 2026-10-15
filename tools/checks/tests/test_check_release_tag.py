"""Tests for check_release_tag on a temporary git repository.

A main branch with the statute manifest at version 1.2.3 and a tag on it;
the runs gh would list are passed in place of the gh call.
"""

import contextlib
import io
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

from repo_checks import check_release_tag as crt
from repo_checks import git_env

MANIFEST = '[package]\nname = "statute"\nversion = "1.2.3"\n'
CI = {
  "id": 7,
  "path": ".github/workflows/ci.yml",
  "status": "completed",
  "conclusion": "success",
  "run_attempt": 1,
}
PAGES = {**CI, "id": 8, "path": ".github/workflows/pages.yml"}


class Release(unittest.TestCase):
  def setUp(self):
    tmp = tempfile.TemporaryDirectory()
    self.addCleanup(tmp.cleanup)
    self.root = Path(tmp.name)
    self.git("init", "-q", "-b", "main")
    self.write(crt.MANIFEST, MANIFEST)
    self.commit("base")
    self.git("tag", "statute-v1.2.3")

  def git(self, *args):
    # As in test_check_instruction_changes: no user-level ignore file, no
    # automatic maintenance.
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

  def run_check(self, tag="statute-v1.2.3", runs=(CI, PAGES)):
    out = io.StringIO()
    with (
      mock.patch.object(crt, "ci_runs", return_value=list(runs)) as gh,
      contextlib.redirect_stdout(out),
    ):
      code = crt.main(
        [
          "--root",
          str(self.root),
          "--repo",
          "example/repo",
          "--tag",
          tag,
          "--main",
          "main",
        ]
      )
    self.gh = gh
    return code, out.getvalue()

  def test_passes(self):
    code, out = self.run_check()
    self.assertEqual(code, 0, out)
    self.assertIn("ci.yml passed on its push to main; run 7", out)

  def test_tag_differs_from_manifest(self):
    self.git("tag", "statute-v1.2.4")
    code, out = self.run_check(tag="statute-v1.2.4")
    self.assertEqual(code, 1)
    self.assertIn("is not statute-v1.2.3", out)
    self.gh.assert_not_called()

  def test_manifest_missing(self):
    (self.root / crt.MANIFEST).unlink()
    code, out = self.run_check()
    self.assertEqual(code, 4)
    self.assertIn("no version to read", out)

  def test_manifest_without_version(self):
    self.write(crt.MANIFEST, '[package]\nname = "statute"\n')
    code, _ = self.run_check()
    self.assertEqual(code, 4)

  def test_commit_not_on_main(self):
    self.git("checkout", "-q", "-b", "side")
    self.write("other.txt", "side\n")
    self.commit("side")
    self.git("tag", "-f", "statute-v1.2.3")
    code, out = self.run_check()
    self.assertEqual(code, 1)
    self.assertIn("is not on main", out)
    self.gh.assert_not_called()

  def test_ancestor_of_main_passes(self):
    self.write("later.txt", "later\n")
    self.commit("later")
    code, out = self.run_check()
    self.assertEqual(code, 0, out)

  def test_no_ci_run(self):
    code, out = self.run_check(runs=[PAGES])
    self.assertEqual(code, 1)
    self.assertIn("0 runs of .github/workflows/ci.yml", out)

  def test_two_ci_runs(self):
    code, out = self.run_check(runs=[CI, {**CI, "id": 9}])
    self.assertEqual(code, 1)
    self.assertIn("2 runs of .github/workflows/ci.yml", out)

  def test_ci_not_finished(self):
    code, _ = self.run_check(
      runs=[{**CI, "status": "in_progress", "conclusion": None}]
    )
    self.assertEqual(code, 1)

  def test_ci_failed(self):
    code, out = self.run_check(runs=[{**CI, "conclusion": "failure"}])
    self.assertEqual(code, 1)
    self.assertIn("conclusion failure", out)

  def test_ci_green_on_rerun(self):
    code, out = self.run_check(runs=[{**CI, "run_attempt": 2}])
    self.assertEqual(code, 1)
    self.assertIn("attempt 2", out)


if __name__ == "__main__":
  unittest.main()
