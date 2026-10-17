"""Fail when a change mixes instruction files with other files.

The change is `git diff <base>...HEAD`: what HEAD changed since it left the
base. A rename counts as a deletion of the old path and an addition of the
new one. Each changed path is an instruction file, a generated file, a
hook or CI file or another file, by the tables below; a change with both
instruction files and other files fails, generated and hook or CI files
not counted.
With this repo's tables, the top statute.lock is generated when its TOML
differs from the base's only in the entries statute writes from its own
version and the rules' checks, ENTRIES, and an instruction file otherwise.
Instruction files only, other files only, or nothing, passes.
docs/design/checkers.md says what this proves and how another repo runs
it.

Exit status: 0 on pass, 1 on a mixed change, 2 when git cannot give the diff.
"""

import argparse
from pathlib import Path
import re
import subprocess
import sys
import tomllib

from repo_checks import git_env

# Instruction files in any repo: what an agent harness reads as
# instructions, skills, subagents, commands, hooks or permissions, in any
# directory; and statute's files at the top of the repo: what it compiles
# AGENTS.md from, the rule set, its lock file and .statute/ (the local
# rules and the ignore file), and the checks file statute before 0.1.18
# wrote beside them, with the legacy copies of the hand-written files it
# replaces.
GENERIC = [
  r"(.*/)?AGENTS\.md",
  r"(.*/)?CLAUDE\.md",
  r"(.*/)?\.agents/skills/.+",
  r"(.*/)?\.claude/skills/.+",
  r"(.*/)?\.claude/agents/.+",
  r"(.*/)?\.claude/commands/.+",
  r"(.*/)?\.claude/settings(\.local)?\.json",
  r"(.*/)?\.claude/hooks/.+",
  r"(.*/)?\.codex/.+",
  r"statute\.toml",
  r"statute\.lock",
  r"statute-checks\.toml",
  r"\.statute/.+",
  r"(.*/)?[^/]+\.legacy\.md",
]
# This repo's own instruction files: the constitution, rule and skill
# sources. The constitution's are in its edition directory,
# constitution/<YYYY-MM>/.
REPO = [
  r"constitution/[^/]+/index\.toml",
  r"constitution/[^/]+/articles/.+",
  r"rules/.+",
  r"skills/.+",
]
# Generated files: CI proves they equal a build of the sources with the
# tools, so they go with whichever side changed them. A path here is not an
# instruction file even when GENERIC lists it. The AGENTS.md at the top is
# one: `statute compile --check` proves it equals a compile of the rule set,
# and it changes with how statute renders the rule set as well as with the
# rule set; this repo's rule L32, in place of R334, counts it so. Another
# repo declares it with --generated once its CI runs that check.
# statute-checks.toml is another, as this repo's rule L27 provides: statute
# wrote it before the lock file recorded its version and the checks, and
# now deletes it.
GENERATED = [
  r"constitution/[^/]+/generated/constitution\.[^/]+\.md",
  r"statute-checks\.toml",
  r"AGENTS\.md",
]

# Hook and CI files: under R334 the entries that install, pin or run the
# compiler of the instructions are part of the instruction system and the
# rest is not, so such a file counts by what the change alters in it,
# which a path cannot tell. They go with neither side, and the pass names
# them for the reviewer. Another repo declares its own with --hooks-and-ci.
HOOKS_AND_CI = [
  r"\.pre-commit-config\.yaml",
  r"\.github/workflows/.+",
]


def matches(path: str, patterns: list[str]) -> bool:
  """Tell whether one of the patterns matches the whole path.

  Args:
    path: A path relative to the repository.
    patterns: Regular expressions.

  Returns:
    True when a pattern matches.
  """
  return any(re.fullmatch(p, path) for p in patterns)


# The top-level keys of statute.lock that statute writes from its own
# version and the rules' checks rather than from the rule set and the
# sources; `statute compile --check` proves them, and the version changes
# with the tools. A change to the lock file in nothing else is generated in
# this repo, as its rule L32 provides; another repo's lock file stays an
# instruction file.
LOCK = "statute.lock"
ENTRIES = ("statute", "commands")


def git(root: str, *args: str) -> str | None:
  """Run git in `root` and return what it prints.

  Args:
    root: The repository.
    *args: The arguments of git.

  Returns:
    The standard output, or None when git fails.
  """
  result = subprocess.run(
    ["git", "-C", root, *args],
    capture_output=True,
    text=True,
    env=git_env.clean(),
  )
  return result.stdout if result.returncode == 0 else None


def entries_only(root: str, base: str) -> bool:
  """Tell whether the lock file changed in nothing but ENTRIES.

  Args:
    root: The repository.
    base: The git ref HEAD is compared with.

  Returns:
    True when the lock file is TOML at the merge base of `base` and HEAD
    and at HEAD, and the same at both with ENTRIES left out; False when
    git or the parse fails, so that nothing passes as generated unproven.
  """
  merge_base = git(root, "merge-base", base, "HEAD")
  if merge_base is None:
    return False
  tables = []
  for rev in (merge_base.strip(), "HEAD"):
    text = git(root, "show", f"{rev}:{LOCK}")
    if text is None:
      return False
    try:
      table = tomllib.loads(text)
    except tomllib.TOMLDecodeError:
      return False
    for key in ENTRIES:
      table.pop(key, None)
    tables.append(table)
  return tables[0] == tables[1]


def changed(root: str, base: str) -> list[tuple[str, str]] | None:
  """List the files changed in `base...HEAD`.

  Args:
    root: The repository.
    base: The git ref HEAD is compared with.

  Returns:
    (status letter, path) for each changed file, or None when git fails.
  """
  result = subprocess.run(
    [
      "git",
      "-C",
      root,
      "diff",
      "--name-status",
      "--no-renames",
      "-z",
      f"{base}...HEAD",
    ],
    capture_output=True,
    text=True,
    env=git_env.clean(),
  )
  if result.returncode != 0:
    print(result.stderr.strip(), file=sys.stderr)
    return None
  fields = result.stdout.split("\0")[:-1]
  return list(zip(fields[0::2], fields[1::2], strict=True))


def main(argv: list[str] | None = None) -> int:
  """Run the check and print its result.

  Args:
    argv: The command-line arguments, or None for sys.argv.

  Returns:
    The exit status: 0 on pass, 1 on a mixed change, 2 when git fails.
  """
  parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
  parser.add_argument(
    "--base", required=True, help="the git ref to compare HEAD with"
  )
  parser.add_argument(
    "--root",
    default=str(Path(__file__).resolve().parents[4]),
    help="the repository (default: the repo this script is in)",
  )
  parser.add_argument(
    "--instructions",
    action="append",
    metavar="REGEX",
    help="a path pattern the repo declares as instructions; with this,"
    " --generated or --hooks-and-ci, the patterns of this repo are not used",
  )
  parser.add_argument(
    "--generated",
    action="append",
    metavar="REGEX",
    help="a path pattern of the repo's generated files",
  )
  parser.add_argument(
    "--hooks-and-ci",
    action="append",
    metavar="REGEX",
    help="a path pattern of a commit hook or CI file, which counts by what"
    " the change alters in it and is left to the reviewer",
  )
  args = parser.parse_args(argv)
  own = (
    args.instructions is not None
    or args.generated is not None
    or args.hooks_and_ci is not None
  )
  instructions = GENERIC + ((args.instructions or []) if own else REPO)
  generated = (args.generated or []) if own else GENERATED
  hooks_and_ci = (args.hooks_and_ci or []) if own else HOOKS_AND_CI

  files = changed(args.root, args.base)
  if files is None:
    return 2
  groups = {"instruction": [], "generated": [], "hook and CI": [], "other": []}
  for status, path in files:
    kind = (
      "generated"
      if matches(path, generated)
      or (not own and path == LOCK and entries_only(args.root, args.base))
      else "hook and CI"
      if matches(path, hooks_and_ci)
      else "instruction"
      if matches(path, instructions)
      else "other"
    )
    groups[kind].append(f"{status} {path}")

  if groups["instruction"] and groups["other"]:
    print(
      "instruction-only check: this change mixes instruction files with"
      f" other files since {args.base}; split it into two changes"
    )
    for group, lines in groups.items():
      if lines:
        print(f"{group} files:")
        print("\n".join(f"  {line}" for line in lines))
    return 1
  summary = (
    "instruction files only"
    if groups["instruction"]
    else "other files only"
    if groups["other"]
    else "no instruction or other files"
  )
  print(
    f"instruction-only check: {summary} since {args.base}"
    f" ({len(groups['instruction'])} instruction, {len(groups['other'])} other,"
    f" {len(groups['generated'])} generated,"
    f" {len(groups['hook and CI'])} hook and CI)"
  )
  # A reviewer checks that a generated file changed only in rendering
  # when tools changed it, and on which side each change to a hook or CI
  # file falls, so the pass names them.
  for group in ("generated", "hook and CI"):
    if groups[group]:
      print(f"{group} files:")
      print("\n".join(f"  {line}" for line in groups[group]))
  return 0


if __name__ == "__main__":
  sys.exit(main())
