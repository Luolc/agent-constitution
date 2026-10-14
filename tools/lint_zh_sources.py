#!/usr/bin/env python3
"""Run limae on the Chinese text of the constitution and rule sources.

Every `zh` text in constitution/index.toml, constitution/articles/ and
rules/ goes to limae (Chinese Markdown typography) as a Markdown file of
its own, with its default configuration. Each citation `{@...}` is first
replaced by PLACEHOLDER, Chinese text shaped like the rendering of a
constitution citation (`第十条`), so limae judges the spacing around it as
it will read once built, and is put back afterwards. A rule citation gets
the same placeholder until its rendering is set.

Without --fix nothing is written. With --fix limae fixes what it can and
each changed text is written back in place; no other byte of the file
changes, which is checked by parsing the file again. Findings print as
`path:line: field: <limae's message>`; the exit status is limae's (0
clean, 1 with errors left), or 2 when a text cannot be checked or fixed.
"""

import argparse
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path

SOURCES = [
  "constitution/index.toml",
  "constitution/articles/*/*.toml",
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


@dataclass
class Text:
  path: Path
  start: int  # offset of the text in the file
  end: int
  line: int  # line of the text's first line in the file
  field: str  # `title`, `content`, `items.<id>.content`, ...
  basic: bool
  value: str  # as TOML parses it

  def masked(self) -> str:
    return CITATION.sub(PLACEHOLDER, self.value)

  def unmasked(self, text: str) -> str:
    citations = iter(CITATION.findall(self.value))
    return re.sub(PLACEHOLDER, lambda _: next(citations), text)


def field_at(source: str, offset: int) -> str:
  """The dotted name of the table holding the `zh` key at `offset`, with
  an item's ID in place of `items`."""
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


def zh_leaves(data, keys=()) -> list:
  """Every (key path, value) under a `zh` key, in TOML's own order."""
  if isinstance(data, dict):
    return [leaf for k, v in data.items() for leaf in zh_leaves(v, keys + (k,))]
  if isinstance(data, list):
    return [
      leaf for i, v in enumerate(data) for leaf in zh_leaves(v, keys + (i,))
    ]
  return [(keys, data)] if keys and keys[-1] == "zh" else []


def texts(root: Path, path: Path) -> tuple[list[Text], str | None]:
  """The `zh` texts of one file, or an error when one of them is in a
  form this script cannot write back."""
  source = (root / path).read_text(encoding="utf-8")
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
  parsed = sorted(value for _, value in zh_leaves(tomllib.loads(source)))
  if parsed != sorted(t.value for t in found):
    return (
      found,
      f"{path}:1: a `zh` text is not `zh = \"...\"` on one line or `zh = '''` on its own line",
    )
  for t in found:
    if PLACEHOLDER in t.value:
      return (
        found,
        f"{path}:{t.line}: {t.field}: contains `{PLACEHOLDER}`, the citation placeholder",
      )
  return found, None


def write_back(
  root: Path, path: Path, changed: list[tuple[Text, str]]
) -> str | None:
  """Put each fixed text in place of its original, then check that the
  file parses to the same data apart from those texts."""
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
      return f"{path}:1: writing the fixes back would change more than the fixed texts; file left as it was"
  except tomllib.TOMLDecodeError as e:
    return (
      f"{path}:1: the fixed file would not parse ({e}); file left as it was"
    )
  file.write_text(new, encoding="utf-8")
  return None


def strip_zh(data):
  if isinstance(data, dict):
    return {k: strip_zh(v) for k, v in data.items() if k != "zh"}
  if isinstance(data, list):
    return [strip_zh(v) for v in data]
  return data


def main(argv: list[str] | None = None) -> int:
  parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
  parser.add_argument(
    "--fix", action="store_true", help="write limae's fixes back to the sources"
  )
  parser.add_argument(
    "--root",
    type=Path,
    default=Path("."),
    help="the checkout (default: the current directory)",
  )
  args = parser.parse_args(argv)
  limae = shutil.which("limae")
  if limae is None:
    print(
      "limae is not on PATH; run this script with `uv run`, which installs it",
      file=sys.stderr,
    )
    return 2

  all_texts, problems = [], []
  for pattern in SOURCES:
    for file in sorted(args.root.glob(pattern)):
      found, problem = texts(args.root, file.relative_to(args.root))
      if problem:
        problems.append(problem)
      else:
        all_texts += found
  if problems:
    print("\n".join(problems))
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
    findings = []
    for line in result.stdout.splitlines():
      m = FINDING.match(line)
      if m and m.group(1) in names:
        t = names[m.group(1)]
        findings.append(
          f"{t.path}:{t.line + int(m.group(2)) - 1}: {t.field}: {m.group(3)}"
        )
    if result.returncode not in (0, 1) or (
      result.returncode == 1 and not findings
    ):
      print(result.stdout + result.stderr, file=sys.stderr)
      print(
        f"limae exited {result.returncode} without a finding this script can place",
        file=sys.stderr,
      )
      return 2

    if args.fix:
      changed = {}
      for name, t in names.items():
        fixed = Path(tmp, name).read_text(encoding="utf-8")
        if fixed.count(PLACEHOLDER) != len(CITATION.findall(t.value)):
          problems.append(
            f"{t.path}:{t.line}: {t.field}: limae's fix changed the citations; not written"
          )
          continue
        fixed = (
          t.unmasked(fixed.rstrip("\n")) + t.value[len(t.value.rstrip("\n")) :]
        )
        if fixed != t.value:
          changed.setdefault(t.path, []).append((t, fixed))
      for path, items in changed.items():
        if problem := write_back(args.root, path, items):
          problems.append(problem)
        else:
          print(f"{path}: fixed {len(items)} text(s)")
      if problems:
        print("\n".join(problems))
        return 2

  print(
    "\n".join(findings)
    if findings
    else f"limae: {len(all_texts)} zh texts clean"
  )
  return result.returncode


if __name__ == "__main__":
  sys.exit(main())
