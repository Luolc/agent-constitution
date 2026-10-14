"""Run limae on the Chinese text of the constitution and rule sources.

Every `zh` text in index.toml and articles/ of the constitution's
edition directory, constitution/<YYYY-MM>/, and in rules/ goes to limae
(Chinese Markdown typography) as a Markdown file of its own, with its
default configuration. Each citation `{@...}` is first replaced by
PLACEHOLDER, Chinese text shaped like the rendering of a constitution
citation (`第十条`), so limae judges the spacing around it as it will read
once built, and is put back afterwards. A rule citation gets the same
placeholder until its rendering is set.

limae is the one installed in this interpreter's environment, the version
uv.lock pins, never one found on PATH.

Without --fix nothing is written. With --fix limae fixes what it can and
each changed text is written back in place; no other byte of the file
changes, which is checked by parsing the file again. Findings print as
`path:line: field: <limae's message>`; the exit status is limae's (0
clean, 1 with errors left), or 2 when a text cannot be checked or fixed.
"""

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import subprocess
import sys
import sysconfig
import tempfile
import tomllib

SOURCES = [
  "constitution/*/index.toml",
  "constitution/*/articles/*/*.toml",
  "rules/*.toml",
]
CITATION = re.compile(r"\{@[^}]*\}")
PLACEHOLDER = "第某条"
# The two forms the sources use: a one-line basic string, for titles, and a
# multi-line literal string, for everything else.
BASIC = re.compile(r'^zh = "((?:[^"\\\n]|\\.)*)"$', re.M)
LITERAL = re.compile(r"^zh = '''\n(.*?)'''$", re.M | re.S)
HEADER = re.compile(r"^\[\[?([^\]]+)\]\]?$")
ITEM_ID = re.compile(r'^id = "([^"]*)"$')
FINDING = re.compile(r"^(.*?):(\d+): (.*)$")


@dataclass(frozen=True)
class Text:
  """One `zh` text of a source file and where it is.

  Attributes:
    path: The file, relative to the checkout.
    start: The offset of the text in the file.
    end: The offset just after the text.
    line: The line of the text's first line in the file.
    field: `title`, `content`, `items.<id>.content`, ...
    basic: Whether the text is a basic string rather than a literal one.
    value: The text as TOML parses it.
  """

  path: Path
  start: int
  end: int
  line: int
  field: str
  basic: bool
  value: str

  def masked(self) -> str:
    """Replace each citation in the text by PLACEHOLDER.

    Returns:
      The masked text.
    """
    return CITATION.sub(PLACEHOLDER, self.value)

  def unmasked(self, text: str) -> str:
    """Put this text's citations back in place of the placeholders.

    Args:
      text: The masked text, possibly fixed by limae.

    Returns:
      The text with its citations.
    """
    citations = iter(CITATION.findall(self.value))
    return re.sub(re.escape(PLACEHOLDER), lambda _: next(citations), text)


def field_at(source: str, offset: int) -> str:
  """Name the table holding the `zh` key at `offset`.

  Args:
    source: The text of a source file.
    offset: The offset of a `zh` key in it.

  Returns:
    The dotted name of the table, with an item's ID in place of `items`.
  """
  table, item = "", None
  for line in source[:offset].split("\n"):
    if m := HEADER.match(line):
      table = m.group(1)
      if line.startswith("[[items]]"):
        item = None
    elif table == "items" and (m := ITEM_ID.match(line)):
      item = m.group(1)
  return (
    table.replace("items", f"items.{item}", 1)
    if table.startswith("items")
    else table
  )


Keys = tuple[str | int, ...]


def zh_leaves(data: object, keys: Keys = ()) -> list[tuple[Keys, str]]:
  """List the values under a `zh` key, in TOML's own order.

  Args:
    data: Parsed TOML, or a part of it.
    keys: The key path of `data`.

  Returns:
    Every (key path, value) under a `zh` key.
  """
  if isinstance(data, dict):
    return [leaf for k, v in data.items() for leaf in zh_leaves(v, keys + (k,))]
  if isinstance(data, list):
    return [
      leaf for i, v in enumerate(data) for leaf in zh_leaves(v, keys + (i,))
    ]
  if keys and keys[-1] == "zh" and isinstance(data, str):
    return [(keys, data)]
  return []


def texts(root: Path, path: Path) -> tuple[list[Text], str | None]:
  """Find the `zh` texts of one file.

  Args:
    root: The checkout.
    path: The file, relative to the checkout.

  Returns:
    The texts, and an error when the file is not valid TOML or one of
    them is in a form this script cannot write back, else None.
  """
  source = (root / path).read_text(encoding="utf-8")
  try:
    data = tomllib.loads(source)
  except tomllib.TOMLDecodeError as e:
    return [], f"{path}:1: not valid TOML: {e}"
  found = []
  for m in BASIC.finditer(source):
    value = tomllib.loads(f"v = {m.group(0)[5:]}")["v"]
    found.append(
      Text(
        path,
        m.start(1),
        m.end(1),
        source.count("\n", 0, m.start()) + 1,
        field_at(source, m.start()),
        True,
        value,
      )
    )
  for m in LITERAL.finditer(source):
    found.append(
      Text(
        path,
        m.start(1),
        m.end(1),
        source.count("\n", 0, m.start()) + 2,
        field_at(source, m.start()),
        False,
        m.group(1),
      )
    )
  found.sort(key=lambda t: t.start)
  parsed = sorted(value for _, value in zh_leaves(data))
  if parsed != sorted(t.value for t in found):
    return (
      found,
      f'{path}:1: a `zh` text is not `zh = "..."` on one line'
      " or `zh = '''` on its own line",
    )
  for t in found:
    if PLACEHOLDER in t.value:
      return (
        found,
        f"{path}:{t.line}: {t.field}: contains `{PLACEHOLDER}`,"
        " the citation placeholder",
      )
  return found, None


def write_back(
  root: Path, path: Path, changed: list[tuple[Text, str]]
) -> str | None:
  """Put each fixed text in place of its original.

  Then check that the file parses to the same data apart from those texts.

  Args:
    root: The checkout.
    path: The file, relative to the checkout.
    changed: Each text with its fixed version.

  Returns:
    An error when the file is left as it was, else None.
  """
  file = root / path
  source = file.read_text(encoding="utf-8")
  new = source
  for t, fixed in sorted(changed, key=lambda c: -c[0].start):
    if t.basic:
      fixed = fixed.replace("\\", "\\\\").replace('"', '\\"')
    new = new[: t.start] + fixed + new[t.end :]
  replaced = {t.value: fixed for t, fixed in changed}
  expected = [
    (k, replaced.get(v, v)) for k, v in zh_leaves(tomllib.loads(source))
  ]
  try:
    if zh_leaves(tomllib.loads(new)) != expected or strip_zh(
      tomllib.loads(new)
    ) != strip_zh(tomllib.loads(source)):
      return (
        f"{path}:1: writing the fixes back would change more than the fixed"
        " texts; file left as it was"
      )
  except tomllib.TOMLDecodeError as e:
    return (
      f"{path}:1: the fixed file would not parse ({e}); file left as it was"
    )
  file.write_text(new, encoding="utf-8")
  return None


def strip_zh(data: object) -> object:
  """Return parsed TOML without its `zh` keys.

  Args:
    data: Parsed TOML, or a part of it.

  Returns:
    The same data without any `zh` key.
  """
  if isinstance(data, dict):
    return {k: strip_zh(v) for k, v in data.items() if k != "zh"}
  if isinstance(data, list):
    return [strip_zh(v) for v in data]
  return data


def read_sources(root: Path) -> tuple[list[Text], list[str]]:
  """Find the `zh` texts of every source file.

  Args:
    root: The checkout.

  Returns:
    The texts, and a problem for each file whose texts cannot be found.
  """
  all_texts, problems = [], []
  for pattern in SOURCES:
    for file in sorted(root.glob(pattern)):
      found, problem = texts(root, file.relative_to(root))
      if problem:
        problems.append(problem)
      else:
        all_texts += found
  return all_texts, problems


def place(stdout: str, names: dict[str, Text]) -> list[str]:
  """Turn limae's findings into findings on the sources.

  Args:
    stdout: limae's output.
    names: The name of each file given to limae -> its text.

  Returns:
    Each finding as `path:line: field: message`.
  """
  findings = []
  for line in stdout.splitlines():
    m = FINDING.match(line)
    if m and m.group(1) in names:
      t = names[m.group(1)]
      findings.append(
        f"{t.path}:{t.line + int(m.group(2)) - 1}: {t.field}: {m.group(3)}"
      )
  return findings


def apply_fixes(root: Path, tmp: str, names: dict[str, Text]) -> list[str]:
  """Write limae's fixes back to the sources and print each file fixed.

  Args:
    root: The checkout.
    tmp: The directory of the files limae fixed.
    names: The name of each file given to limae -> its text.

  Returns:
    A problem for each text or file not written.
  """
  problems, changed = [], {}
  for name, t in names.items():
    fixed = Path(tmp, name).read_text(encoding="utf-8")
    if fixed.count(PLACEHOLDER) != len(CITATION.findall(t.value)):
      problems.append(
        f"{t.path}:{t.line}: {t.field}: limae's fix changed the"
        " citations; not written"
      )
      continue
    fixed = (
      t.unmasked(fixed.rstrip("\n")) + t.value[len(t.value.rstrip("\n")) :]
    )
    if fixed != t.value:
      changed.setdefault(t.path, []).append((t, fixed))
  for path, items in changed.items():
    if problem := write_back(root, path, items):
      problems.append(problem)
    else:
      print(f"{path}: fixed {len(items)} text(s)")
  return problems


def main(argv: list[str] | None = None) -> int:
  """Check, or with --fix fix, the `zh` texts and print the findings.

  Args:
    argv: The command-line arguments, or None for sys.argv.

  Returns:
    limae's exit status, or 2 when a text cannot be checked or fixed.
  """
  parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
  parser.add_argument(
    "--fix", action="store_true", help="write limae's fixes back to the sources"
  )
  parser.add_argument(
    "--root",
    type=Path,
    default=Path(__file__).resolve().parents[4],
    help="the checkout (default: the repo this script is in)",
  )
  args = parser.parse_args(argv)
  limae = Path(sysconfig.get_path("scripts")) / "limae"
  if not limae.is_file():
    print(
      f"limae is not installed in this Python environment ({limae.parent});"
      " run `uv run lint-zh-sources`, which installs the version uv.lock"
      " pins",
      file=sys.stderr,
    )
    return 2

  all_texts, problems = read_sources(args.root)
  if problems:
    print("\n".join(problems), file=sys.stderr)
    return 2

  with tempfile.TemporaryDirectory() as tmp:
    names = {}
    for n, t in enumerate(all_texts):
      name = f"{n:04d}.md"
      names[name] = t
      masked = t.masked()
      Path(tmp, name).write_text(
        masked if masked.endswith("\n") else masked + "\n", encoding="utf-8"
      )
    command = [limae, *(["--fix"] if args.fix else []), *names]
    result = subprocess.run(command, cwd=tmp, capture_output=True, text=True)
    findings = place(result.stdout, names)
    if result.returncode not in (0, 1) or (
      result.returncode == 1 and not findings
    ):
      print(result.stdout + result.stderr, file=sys.stderr)
      print(
        f"limae exited {result.returncode} without a finding this script"
        " can place",
        file=sys.stderr,
      )
      return 2

    if args.fix and (problems := apply_fixes(args.root, tmp, names)):
      print("\n".join(problems), file=sys.stderr)
      return 2

  print(
    "\n".join(findings)
    if findings
    else f"limae: {len(all_texts)} zh texts clean"
  )
  return result.returncode


if __name__ == "__main__":
  sys.exit(main())
