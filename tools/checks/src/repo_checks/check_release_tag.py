"""Check that a tag may publish statute, before release.yml publishes it.

Three steps, each run only when the one before it passed:

1. The tag is `statute-v` followed by the version in
   crates/statute/Cargo.toml.
2. The commit the tag points to is the main branch or an ancestor of it.
3. That commit's run of ci.yml, triggered by its push to main, completed
   with `success` on its first attempt.

docs/design/amendments-and-releases.md says how a release goes.

Exit status: 0 when the tag may publish; 1 when a step says it may not;
2 when git or gh fails; 4 when the manifest has no version to read.
"""

import argparse
import json
from pathlib import Path
import subprocess
import sys
import tomllib

from repo_checks import git_env

PREFIX = "statute-v"
MANIFEST = "crates/statute/Cargo.toml"
WORKFLOW = ".github/workflows/ci.yml"


def manifest_version(root: Path) -> str | None:
  """Read the package version of the statute manifest.

  Args:
    root: The repository.

  Returns:
    The version, or None when the file is missing, is not TOML or has no
    version string under [package].
  """
  try:
    data = tomllib.loads((root / MANIFEST).read_text(encoding="utf-8"))
  except OSError, tomllib.TOMLDecodeError:
    return None
  version = data.get("package", {}).get("version")
  return version if isinstance(version, str) else None


def git(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
  """Run git on the repository.

  Args:
    root: The repository.
    *args: The arguments after `git -C <root>`.

  Returns:
    The finished process, with its output as text.
  """
  return subprocess.run(
    ["git", "-C", str(root), *args],
    capture_output=True,
    text=True,
    env=git_env.clean(),
  )


def ci_runs(repo: str, sha: str) -> list[dict[str, object]] | None:
  """List the workflow runs of a commit pushed to main.

  Args:
    repo: The GitHub repository, `<owner>/<name>`.
    sha: The commit.

  Returns:
    The runs the API gives, or None when gh fails.
  """
  result = subprocess.run(
    [
      "gh",
      "api",
      f"repos/{repo}/actions/runs?head_sha={sha}&event=push&branch=main",
    ],
    capture_output=True,
    text=True,
  )
  if result.returncode != 0:
    print(result.stderr.strip(), file=sys.stderr)
    return None
  return json.loads(result.stdout)["workflow_runs"]


def ci_passed(runs: list[dict[str, object]]) -> tuple[bool, str]:
  """Tell whether the one ci.yml run completed with success, first time.

  Args:
    runs: The runs of the commit's push to main.

  Returns:
    Whether it did, and a line that says what was found.
  """
  ci = [r for r in runs if r.get("path") == WORKFLOW]
  if len(ci) != 1:
    return False, f"{len(ci)} runs of {WORKFLOW}, expected one"
  run = ci[0]
  found = (
    f"run {run.get('id')}: status {run.get('status')}, conclusion"
    f" {run.get('conclusion')}, attempt {run.get('run_attempt')}"
  )
  ok = (
    run.get("status") == "completed"
    and run.get("conclusion") == "success"
    and run.get("run_attempt") == 1
  )
  return ok, found


def main(argv: list[str] | None = None) -> int:
  """Run the three steps and print each result.

  Args:
    argv: The command-line arguments, or None for sys.argv.

  Returns:
    The exit status: 0 when the tag may publish, 1 when it may not, 2
    when git or gh fails, 4 when the manifest has no version to read.
  """
  parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
  parser.add_argument("--tag", required=True, help="the tag, statute-v<x.y.z>")
  parser.add_argument(
    "--repo", required=True, help="the GitHub repository, <owner>/<name>"
  )
  parser.add_argument(
    "--root",
    type=Path,
    default=Path(__file__).resolve().parents[4],
    help="the repository (default: the repo this script is in)",
  )
  parser.add_argument(
    "--main",
    default="origin/main",
    help="the ref of the main branch (default: origin/main)",
  )
  args = parser.parse_args(argv)

  version = manifest_version(args.root)
  if version is None:
    print(f"release check: no version to read in {MANIFEST}")
    return 4
  if args.tag != PREFIX + version:
    print(
      f"release check: tag {args.tag} is not {PREFIX}{version},"
      f" the version in {MANIFEST}"
    )
    return 1
  print(f"release check: tag {args.tag} matches {MANIFEST}")

  resolved = git(args.root, "rev-parse", "--verify", f"{args.tag}^{{commit}}")
  if resolved.returncode != 0:
    print(resolved.stderr.strip(), file=sys.stderr)
    return 2
  sha = resolved.stdout.strip()
  ancestor = git(args.root, "merge-base", "--is-ancestor", sha, args.main)
  if ancestor.returncode == 1:
    print(f"release check: {sha} is not on {args.main}")
    return 1
  if ancestor.returncode != 0:
    print(ancestor.stderr.strip(), file=sys.stderr)
    return 2
  print(f"release check: {sha} is on {args.main}")

  runs = ci_runs(args.repo, sha)
  if runs is None:
    return 2
  ok, found = ci_passed(runs)
  if not ok:
    print(
      f"release check: {sha} has no first-attempt success of {WORKFLOW}"
      f" on its push to main; {found}"
    )
    return 1
  print(f"release check: {WORKFLOW} passed on its push to main; {found}")
  return 0


if __name__ == "__main__":
  sys.exit(main())
