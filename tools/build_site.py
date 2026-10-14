#!/usr/bin/env python3
"""Build the static site: count the tokens, run Hugo, check the links.

The site is Hugo's (web/, docs/design.md, Static site); `hugo-0.167.0 -s
web` alone builds it into site/, with the tokens not counted unless
web/data/lengths.json counted the same text. This script runs the three
steps CI runs: count_tokens.py, which writes that file (its --require-tokens
passes through), Hugo, and check_site_anchors.py on the result.
"""

import argparse
from pathlib import Path
import subprocess
import sys

import check_site_anchors
import count_tokens

ROOT = Path(__file__).resolve().parent.parent
HUGO = "hugo-0.167.0"  # the same version as the CI step that builds the site
HUGO_TIMEOUT = 120  # seconds; the build takes well under one


def main(argv: list[str] | None = None) -> int:
  """Build the site and check it.

  Args:
    argv: The command-line arguments, or None for sys.argv.

  Returns:
    The exit status: 0 on success, 2 without Hugo, else non-zero.
  """
  parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
  parser.add_argument(
    "--out",
    type=Path,
    default=ROOT / "site",
    help="the output directory (default: site/)",
  )
  parser.add_argument(
    "--require-tokens",
    action="store_true",
    help="fail when a text's tokens are not counted (the token-count workflow)",
  )
  args = parser.parse_args(argv)
  if status := count_tokens.main(
    ["--require-tokens"] if args.require_tokens else []
  ):
    return status
  command = [
    HUGO,
    "--source",
    str(ROOT / "web"),
    "--destination",
    str(args.out.resolve()),
    "--cleanDestinationDir",
    "--panicOnWarning",
  ]
  try:
    build = subprocess.run(command, timeout=HUGO_TIMEOUT)
  except FileNotFoundError:
    print(
      f"{HUGO} not found; it builds the site (docs/design.md, Static site)",
      file=sys.stderr,
    )
    return 2
  except subprocess.TimeoutExpired:
    print(f"{HUGO} ran past {HUGO_TIMEOUT} seconds", file=sys.stderr)
    return 1
  if build.returncode:
    return build.returncode
  return check_site_anchors.main([str(args.out)])


if __name__ == "__main__":
  sys.exit(main())
