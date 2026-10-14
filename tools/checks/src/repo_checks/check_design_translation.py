"""Check that the Chinese translation of the design matches the English.

The design is docs/design/*.md, in English; its translation is
docs/design/zh/*.md, one file of the same name for each. Checked:

- The two directories hold the same file names.
- The first line of each translation is `<!-- en-sha256: <hex> -->`, the
  SHA-256 of the English file it was translated from, and equals that of
  the English file now: any change to the English file fails the check
  until the translation follows it and the line is updated.
- The translation's second line is its one `# ` title, and below it it
  has the English file's headings, in the same order
  and at the same levels: each heading of the translation starts with the
  English heading's anchor, `## <a id="checks"></a>检查`, the anchor
  GitHub and Hugo give the English heading. Links to `#<anchor>` thus work
  in both languages. Lines in fenced code blocks are not headings.

Without docs/design/zh/ there is no translation yet: the check says so
and passes.

Usage: uv run check-design-translation [--root <dir>]. Errors print as
`path:line: message`; the exit status is 1 on any error.
"""

import argparse
import hashlib
from pathlib import Path
import re
import sys

DESIGN = Path("docs/design")
TRANSLATION = DESIGN / "zh"
HASH_LINE = re.compile(r"^<!-- en-sha256: ([0-9a-f]{64}) -->$")
HEADING = re.compile(r"^(#{1,6}) +(.*?) *#* *$")
ANCHOR = re.compile(r'^<a id="([^"]+)"></a>')
FENCE = re.compile(r"^ {0,3}(```|~~~)")


def slug(heading: str) -> str:
  """Return the anchor GitHub and Hugo give a heading.

  Args:
    heading: The heading's text, without the `#`s.

  Returns:
    The text lowercased, without its punctuation, spaces as hyphens.
  """
  text = re.sub(r"[^\w\- ]", "", heading.lower())
  return text.replace(" ", "-")


def headings(text: str) -> list[tuple[int, int, str]]:
  """Return the headings of a Markdown file outside fenced code blocks.

  Args:
    text: The file.

  Returns:
    Each heading's line number, level and text.
  """
  found = []
  fence = ""
  for number, line in enumerate(text.splitlines(), 1):
    if m := FENCE.match(line):
      if not fence:
        fence = m.group(1)
      elif m.group(1) == fence:
        fence = ""
      continue
    if not fence and (m := HEADING.match(line)):
      found.append((number, len(m.group(1)), m.group(2)))
  return found


def english_anchors(text: str) -> list[tuple[int, str]]:
  """Return the level and anchor of each heading below the title.

  A repeated anchor gets `-1`, `-2`, ... as GitHub and Hugo give it.

  Args:
    text: The English file.

  Returns:
    Each heading's level and anchor.
  """
  seen: dict[str, int] = {}
  anchors = []
  for _, level, heading in headings(text):
    if level == 1:
      continue
    anchor = slug(heading)
    if anchor in seen:
      seen[anchor] += 1
      anchor = f"{anchor}-{seen[anchor]}"
    else:
      seen[anchor] = 0
    anchors.append((level, anchor))
  return anchors


def check_head(english: str, lines: list[str]) -> list[tuple[int, str]]:
  """Check the first two lines of a translation: the hash and the title.

  Args:
    english: The English file.
    lines: The translation's lines.

  Returns:
    The errors, each the line of the translation and a message.
  """
  errors: list[tuple[int, str]] = []
  expected = hashlib.sha256(english.encode()).hexdigest()
  m = HASH_LINE.match(lines[0]) if lines else None
  if not m:
    errors.append((1, f"first line is not `<!-- en-sha256: {expected} -->`"))
  elif m.group(1) != expected:
    errors.append(
      (
        1,
        f"the English file changed since this translation (en-sha256 is"
        f" {expected} now): bring the translation up to date, then the line",
      )
    )
  if len(lines) < 2 or not lines[1].startswith("# "):
    errors.append((2, "second line is not the `# ` title"))
  return errors


def check_heading(
  number: int, level: int, heading: str, want: tuple[int, str]
) -> list[tuple[int, str]]:
  """Check one heading of a translation against the English one.

  Args:
    number: The heading's line.
    level: Its level.
    heading: Its text.
    want: The English heading's level and anchor.

  Returns:
    The errors, each the line of the translation and a message.
  """
  errors: list[tuple[int, str]] = []
  want_level, want_anchor = want
  if level != want_level:
    errors.append(
      (number, f"level {level}, the English heading's is {want_level}")
    )
  m = ANCHOR.match(heading)
  if not m:
    errors.append((number, f'does not start with `<a id="{want_anchor}"></a>`'))
  elif m.group(1) != want_anchor:
    errors.append(
      (
        number,
        f"anchor {m.group(1)!r}, the English heading's is {want_anchor!r}",
      )
    )
  return errors


def check_file(english: str, translation: str) -> list[tuple[int, str]]:
  """Check one translation against its English file.

  Args:
    english: The English file.
    translation: The translation.

  Returns:
    The errors, each the line of the translation and a message.
  """
  lines = translation.splitlines()
  errors = check_head(english, lines)
  found = headings(translation)
  errors += [
    (number, "a second `# ` title")
    for number, level, _ in found
    if number > 2 and level == 1
  ]
  anchors = english_anchors(english)
  below = [(n, level, h) for n, level, h in found if level > 1]
  for i, (number, level, heading) in enumerate(below):
    if i >= len(anchors):
      errors.append((number, "a heading the English file does not have"))
      break
    errors += check_heading(number, level, heading, anchors[i])
  if len(below) < len(anchors):
    level, anchor = anchors[len(below)]
    errors.append(
      (
        len(lines),
        f"{len(anchors) - len(below)} heading(s) missing, from the level-"
        f"{level} heading {anchor!r} on",
      )
    )
  return errors


def check(root: Path) -> tuple[list[str], str | None]:
  """Check the translation of the design in a checkout.

  Args:
    root: The checkout.

  Returns:
    The errors as `path:line: message`, and a note when there is no
    translation to check.
  """
  if not (root / TRANSLATION).is_dir():
    return [], f"{TRANSLATION}/ does not exist: no translation to check"
  english = {p.name for p in (root / DESIGN).glob("*.md")}
  translated = {p.name for p in (root / TRANSLATION).glob("*.md")}
  errors = [
    f"{TRANSLATION / name}: missing, the translation of {DESIGN / name}"
    for name in sorted(english - translated)
  ]
  errors += [
    f"{TRANSLATION / name}: no English file {DESIGN / name}"
    for name in sorted(translated - english)
  ]
  for name in sorted(english & translated):
    for line, message in check_file(
      (root / DESIGN / name).read_text(encoding="utf-8"),
      (root / TRANSLATION / name).read_text(encoding="utf-8"),
    ):
      errors.append(f"{TRANSLATION / name}:{line}: {message}")
  return errors, None


def main(argv: list[str] | None = None) -> int:
  """Check the translation and print the errors.

  Args:
    argv: The command-line arguments, or None for sys.argv.

  Returns:
    The exit status: 1 on any error, else 0.
  """
  parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
  parser.add_argument(
    "--root",
    type=Path,
    default=Path(__file__).resolve().parents[4],
    help="the checkout (default: the repo this script is in)",
  )
  args = parser.parse_args(argv)
  errors, note = check(args.root)
  if note:
    print(note)
    return 0
  if errors:
    print("\n".join(errors), file=sys.stderr)
    return 1
  print(f"{TRANSLATION}/ matches {DESIGN}/")
  return 0


if __name__ == "__main__":
  sys.exit(main())
