"""Fail when a change mixes instruction files with other files.

The change is `git diff <base>...HEAD`: what HEAD changed since it left the
base. A rename counts as a deletion of the old path and an addition of the
new one. Each changed path is an instruction file, a generated file or
another file, by the tables below; a change with both instruction files and
other files fails, generated files not counted. Instruction files only,
other files only, or nothing, passes. docs/design/checkers.md says what
this proves and how another repo runs it.

Exit status: 0 on pass, 1 on a mixed change, 2 when git cannot give the diff.
"""

import argparse
from pathlib import Path
import re
import subprocess
import sys

from repo_checks import git_env

# Instruction files in any repo: what an agent harness reads as
# instructions, skills, subagents, commands, hooks or permissions, in any
# directory.
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
]
# This repo's own instruction files: the constitution, rule and skill
# sources. The constitution's are in its edition directory,
# constitution/<YYYY-MM>/.
REPO = [
  r"constitution/[^/]+/index\.toml",
  r"constitution/[^/]+/articles/.+",
  r"rules/.+",
  r"\.statute/rules/.+",
  r"skills/.+",
]
# Generated files: CI proves they equal a build of the sources with the
# tools, so they go with whichever side changed them.
GENERATED = [
  r"constitution/[^/]+/generated/constitution\.[^/]+\.md",
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
    help="a path pattern the repo declares as instructions; with this or"
    " --generated, the patterns of this repo are not used",
  )
  parser.add_argument(
    "--generated",
    action="append",
    metavar="REGEX",
    help="a path pattern of the repo's generated files",
  )
  args = parser.parse_args(argv)
  own = args.instructions is not None or args.generated is not None
  instructions = GENERIC + ((args.instructions or []) if own else REPO)
  generated = (args.generated or []) if own else GENERATED

  files = changed(args.root, args.base)
  if files is None:
    return 2
  groups = {"instruction": [], "generated": [], "other": []}
  for status, path in files:
    kind = (
      "generated"
      if matches(path, generated)
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
    f" {len(groups['generated'])} generated)"
  )
  return 0


if __name__ == "__main__":
  sys.exit(main())
