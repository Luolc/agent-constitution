"""Check 1, per table: the keys, types and dates of a source file."""

from pathlib import Path
import tomllib

from constitution_compiler.model import (
  Doc,
  Errors,
  MONTH,
)

# Schemas: key -> type; a trailing `?` marks an optional key; LANG is a table
# whose keys are exactly the languages of index.toml and whose values are text;
# TEXT is a table of language to text in any languages.
LANG = "lang"
TEXT = "text"
INDEX_SCHEMA = {
  "status": str,
  "edition": str,
  "revision": str,
  "languages": list,
  "title": LANG,
  "front": LANG,
}
ARTICLE_SCHEMA = {"id": str, "repealed?": str, "title": LANG, "intro?": LANG}
SECTION_SCHEMA = {
  "id": str,
  "status": str,
  "repealed?": str,
  "audience?": str,
  "content": LANG,
  "items?": list,
}
ITEM_SCHEMA = {
  "id": str,
  "number": str,
  "status": str,
  "repealed?": str,
  "audience?": str,
  "content": LANG,
}
# The rule tree's nodes, by kind (docs/design/rule-format.md, Rule file
# format). The keys every kind has, then those of each kind.
NODE_SCHEMA = {
  "id": int,
  "slug?": str,
  "revision": int,
  "kind": str,
  "status": str,
  "lang": str,
  "title": TEXT,
  "translations?": dict,
}
NODE_SCHEMAS = {
  "domain": NODE_SCHEMA | {"applicability?": TEXT},
  "one-of": NODE_SCHEMA | {"parent": int},
  "rule": NODE_SCHEMA
  | {
    "parent": int,
    # One rule ID or a list of them; `_Reader.superseded_by` checks it.
    "superseded-by?": object,
    "requires?": list,
    "conflicts?": list,
    "authority": list,
    "constitution": str,
    "level": str,
    "check": str,
    "run?": str,
    "tools?": list,
    "text": dict,
  },
}
PARTS_SCHEMA = {"statement": TEXT, "reason": TEXT, "verification": TEXT}

# Check 1: format.


def read_toml(root: Path, path: Path, errors: Errors) -> Doc | None:
  """Read and parse one TOML file; on failure, record check 1.

  Args:
    root: the checkout.
    path: the file, relative to root.
    errors: where errors go.

  Returns:
    The parsed file, or None when it cannot be read or parsed.
  """
  try:
    text = (root / path).read_text(encoding="utf-8")
    data = tomllib.loads(text)
  except OSError as e:
    errors.add(path, 1, 1, f"cannot read: {e.strerror}")
    return None
  except UnicodeDecodeError:
    errors.add(path, 1, 1, "not UTF-8")
    return None
  except tomllib.TOMLDecodeError as e:
    errors.add(path, 1, 1, f"not valid TOML: {e}")
    return None
  return Doc(path, data, text)


def check_keys(
  doc: Doc,
  table: dict,
  schema: dict,
  languages: list[str] | None,
  errors: Errors,
  where: str = "",
  start: int = 1,
) -> bool:
  """Check that the table has the schema's keys, no others, of their types.

  Language tables have exactly the languages. The text of the language
  tables is normalized later by `strip_texts`.

  Args:
    doc: the file holding the table.
    table: the table.
    schema: key -> type, as the schemas above.
    languages: the languages of index.toml, or None before they are known.
    errors: where errors go.
    where: the table's name in messages, "" for the file's top level.
    start: the table's first line, where the search for its keys begins.

  Returns:
    True when clean.
  """
  before = len(errors)
  label = f"`{where}` " if where else ""
  for key, kind in schema.items():
    optional = key.endswith("?")
    name = key.rstrip("?")
    if name not in table:
      if not optional:
        errors.add(doc.path, start, 1, f"{label}lacks `{name}`")
      continue
    value = table[name]
    line = doc.line(f"[{name}]", f"{name} =", start=start)
    if kind in (LANG, TEXT):
      if not isinstance(value, dict) or not all(
        isinstance(v, str) for v in value.values()
      ):
        errors.add(
          doc.path,
          line,
          1,
          f"{label}`{name}` must be a table of language to text",
        )
      elif (
        kind == LANG and languages is not None and set(value) != set(languages)
      ):
        errors.add(
          doc.path,
          line,
          1,
          f"{label}`{name}` has languages {list(value)},"
          f" index.toml lists {languages}",
        )
      elif name == "title" and any(
        "\n" in text.strip("\n") for text in value.values()
      ):
        errors.add(doc.path, line, 1, f"{label}`title` must be one line")
    # TOML's booleans are Python ints too.
    elif not isinstance(value, kind) or (
      kind is int and isinstance(value, bool)
    ):
      errors.add(
        doc.path,
        line,
        1,
        f"{label}`{name}` must be "
        + ("an integer" if kind is int else f"a {kind.__name__}"),
      )
  for name in sorted(table.keys() - {k.rstrip("?") for k in schema}):
    errors.add(
      doc.path,
      doc.line(f"[{name}]", f"{name} =", start=start),
      1,
      f"{label}unknown key `{name}`",
    )
  return len(errors) == before


def strip_texts(table: dict, schema: dict) -> None:
  """Drop the newlines that open and close each text of the language tables.

  That is, the newline after the opening and before the closing quotes.

  Args:
    table: the table, changed in place.
    schema: its schema.
  """
  for key, kind in schema.items():
    name = key.rstrip("?")
    if kind in (LANG, TEXT) and name in table:
      for lang, text in table[name].items():
        table[name][lang] = text.strip("\n")


def check_repealed(
  doc: Doc,
  table: dict,
  status: str | None,
  errors: Errors,
  where: str = "",
  start: int = 1,
) -> None:
  """Check that `repealed` is a year-month, there when the status says so.

  It is present exactly when the status says repealed (for articles:
  present only when repealed).

  Args:
    doc: the file holding the table.
    table: the article, section or item.
    status: its status, None for an article.
    errors: where errors go.
    where: the table's name in messages, "" for the file's top level.
    start: the table's first line, where the search for its keys begins.
  """
  label = f"`{where}` " if where else ""
  value = table.get("repealed")
  line = doc.line("repealed =", start=start)
  if value is not None and not MONTH.fullmatch(value):
    errors.add(
      doc.path, line, 1, f"{label}`repealed` must be `YYYY-MM`, got `{value}`"
    )
  if status == "repealed" and value is None:
    errors.add(
      doc.path,
      doc.line("status =", start=start),
      1,
      f"{label}status is repealed but there is no `repealed` date",
    )
  if status not in (None, "repealed") and value is not None:
    errors.add(
      doc.path, line, 1, f"{label}has `repealed` but status is {status}"
    )
