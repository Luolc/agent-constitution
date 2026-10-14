#!/usr/bin/env python3
"""Build the constitution and its version for agents from the TOML sources.

The outputs are constitution/constitution.<lang>.md and the version for
agents, constitution/constitution.<lang>.agent.md.

Sources: constitution/index.toml (status, revision, languages, title and
front text); constitution/articles/<NN>-<id>/, one directory per article,
holding its article.toml and one <NN>[a-z]-<id>.toml per section, the
section's list items inside it; rules/<id>.toml, one rule each, and
rules/retired-ids.txt, the IDs of deleted rules (checked, not rendered).
Article and section numbers are the name prefixes. The formats, the
numbering and the checks are described in docs/design.md (Source formats,
Checkers).

The version for agents has, in place of the front text, a note of this
script's own that says what the file is and where the full text is; it
leaves out the text of the sections and items marked
`audience = "maintainers"`, keeping their numbers, and has no anchors or
links. The full version marks those provisions with an asterisk.

Without --check the sources are validated and two outputs per language are
written. With --check nothing is written: the sources are validated and the
committed outputs must equal a fresh build byte for byte. With --check
--base <git-ref> the numbering is also compared with that commit: once the
constitution is active, the edition may only move to a later month,
within an edition nothing numbered may
disappear, change number or move, and no deleted rule may go unretired.
Within an edition, and from a draft whatever edition it names, the
revision rises when the constitution text changes and stays when it
does not.
Errors print as `path:line: check N: message`; the exit status is 1 on any
error.
"""

import argparse
from pathlib import Path
import sys

from constitution_base import (
  check_base,
  check_revision,
  GitError,
)
from constitution_checks import (
  check_ids,
  check_languages,
  check_refs,
  check_structure,
)
from constitution_render import (
  render,
)
from constitution_sources import (
  Errors,
  INDEX,
  load,
  Sources,
)

AGENT = ".agent"  # after the language in the name of the version for agents


def output_path(name: str) -> Path:
  """Return where an output goes.

  `en` is the full version, `en.agent` the version for agents.

  Args:
    name: the language, with AGENT after it for the version for agents.

  Returns:
    The path, relative to the checkout.
  """
  return Path(f"constitution/constitution.{name}.md")


def build(root: Path) -> tuple[Errors, Sources | None, dict]:
  """Run checks 1 to 5 in order, stopping after the first that fails.

  Args:
    root: the checkout.

  Returns:
    The errors, the sources (None when check 1 failed) and the outputs,
    name -> text, empty when a check failed.
  """
  errors = Errors()
  src = load(root, errors)
  if errors:
    return errors, None, {}
  assert src is not None
  for check in (check_ids, check_structure, check_languages, check_refs):
    check(src, errors)
    if errors:
      return errors, src, {}
  outputs = {lang: render(lang, src) for lang in src.languages}
  return (
    errors,
    src,
    outputs
    | {lang + AGENT: render(lang, src, agent=True) for lang in src.languages},
  )


# Check 6: outputs.


def check_outputs(root: Path, outputs: dict, errors: Errors) -> None:
  """Check 6: the committed outputs equal a fresh build byte for byte.

  Args:
    root: the checkout.
    outputs: name -> text, as `build` returns them.
    errors: where errors go.
  """
  for name, text in outputs.items():
    path = output_path(name)
    try:
      committed = (root / path).read_bytes()
    except OSError:
      errors.add(
        path, 1, 6, "missing; run `uv run tools/build_constitution.py`"
      )
      continue
    built = text.encode("utf-8")
    if committed != built:
      old, new = committed.split(b"\n"), built.split(b"\n")
      # The lengths may differ: when the shorter is a prefix of the longer,
      # the line reported is the shorter one's last.
      pairs = enumerate(zip(old, new, strict=False), start=1)
      line = next((i for i, (a, b) in pairs if a != b), min(len(old), len(new)))
      errors.add(
        path,
        line,
        6,
        "differs from a fresh build; run `uv run tools/build_constitution.py`",
      )


def main(argv: list[str] | None = None) -> int:
  """Build or check the constitution.

  Args:
    argv: the arguments, None for the command line's.

  Returns:
    The exit status: 1 on any error, else 0.
  """
  parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
  parser.add_argument(
    "--root",
    type=Path,
    default=Path(__file__).resolve().parent.parent,
    help="repository root (default: the repo this script is in)",
  )
  parser.add_argument(
    "--check",
    action="store_true",
    help="write nothing; also fail if the committed outputs are stale",
  )
  parser.add_argument(
    "--base",
    metavar="GIT_REF",
    help="with --check: also compare the numbering with this commit",
  )
  args = parser.parse_args(argv)
  if args.base and not args.check:
    parser.error("--base needs --check")
  errors, src, outputs = build(args.root)
  if not errors and args.check:
    check_outputs(args.root, outputs, errors)
  if not errors and args.base:
    assert src is not None
    try:
      note = check_base(args.root, args.base, src, errors)
      if note:
        print(note)
      check_revision(args.root, args.base, src, errors)
    except GitError as e:
      errors.add(INDEX, 1, 7, str(e))
  if errors:
    print("\n".join(errors), file=sys.stderr)
    return 1
  if not args.check:
    for name, text in outputs.items():
      (args.root / output_path(name)).write_text(
        text, encoding="utf-8", newline="\n"
      )
  return 0


if __name__ == "__main__":
  sys.exit(main())
