"""The constitution and rule sources: their data model and check 1.

Reads constitution/index.toml, the article directories under
constitution/articles/, rules/<id>.toml and rules/retired-ids.txt, and
checks their format (docs/design.md, Source formats, Checkers).
"""

from collections.abc import Iterator
from dataclasses import dataclass, field
from functools import cached_property
from pathlib import Path
import re
import tomllib
from typing import Any, NamedTuple

from constitution_languages import (
  LANGUAGES,
)

INDEX = Path("constitution/index.toml")
ARTICLES = Path("constitution/articles")
ARTICLE_FILE = "article.toml"
OLD_SECTIONS = Path("constitution/sections")
RULES = Path("rules")
RETIRED = RULES / "retired-ids.txt"

ID = r"[a-z0-9]+(?:-[a-z0-9]+)*"
# A citation: `{@id}`, `{@section.item}`, each optionally after a namespace.
MARK = re.compile(
  r"\{@(?:(constitution|rule):)?(" + ID + r"(?:\." + ID + r")?)\}"
)
ARTICLE_NAME = re.compile(r"(0[1-9]|[1-9][0-9])-(.+)")
SECTION_NAME = re.compile(r"(0[1-9]|[1-9][0-9])([a-z]?)-(.+)")
ARTICLE_NAME_SHAPE = "two digits from 01, a hyphen and the ID (04-<id>)"
SECTION_NAME_SHAPE = (
  "two digits from 01, an optional lowercase letter, a hyphen and the ID"
  " (02-<id>, 02a-<id>)"
)
NUMBER = re.compile(r"([1-9][0-9]*)([a-z]?)")
NUMBER_SHAPE = "an integer with an optional letter suffix (2, 2a)"
MONTH = re.compile(r"([0-9]{4})-(0[1-9]|1[0-2])")

# Schemas: key -> type; a trailing `?` marks an optional key; LANG is a table
# whose keys are exactly the languages of index.toml and whose values are text.
LANG = "lang"
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
RULE_SCHEMA = {
  "id": str,
  "status": str,
  "scope": str,
  "authority": list,
  "title": LANG,
  "content": LANG,
}
INDEX_STATUS = ("draft", "active")
STATUS = ("draft", "active", "repealed")
# The one value of `audience`: a provision on how this text itself is made
# and amended, whose text the version for agents leaves out.
MAINTAINERS = "maintainers"


# A text in every language of index.toml: language -> text.
Texts = dict[str, str]


class Errors(list):
  """The errors found, each a line `path:line: check N: message`."""

  def add(self, path: Path, line: int, check: int, message: str) -> None:
    """Record one error.

    Args:
      path: the file.
      line: its line, from 1.
      check: the number of the check that failed.
      message: what is wrong.
    """
    self.append(f"{path}:{line}: check {check}: {message}")


@dataclass(frozen=True, slots=True)
class Doc:
  """One parsed TOML file with its text, for locating errors by line.

  Attributes:
    path: the file, relative to the checkout.
    data: the parsed TOML.
    text: the file's text.
  """

  path: Path
  data: dict[str, Any]
  text: str

  def line(self, *needles: str, start: int = 1) -> int:
    """Return the first line holding the first needle found, else start.

    Args:
      *needles: the strings to look for, in order.
      start: the line to search from, from 1.

    Returns:
      The line number, from 1.
    """
    lines = self.text.split("\n")
    for needle in needles:
      for n, line in enumerate(lines[start - 1 :], start=start):
        if needle in line:
          return n
    return start

  def item_line(self, n: int) -> int:
    """Return the line of the n-th `[[items]]` header, else 1.

    Args:
      n: the item's index, from 0.

    Returns:
      The line number, from 1.
    """
    headers = [
      k
      for k, line in enumerate(self.text.split("\n"), start=1)
      if line.strip() == "[[items]]"
    ]
    return headers[n] if n < len(headers) else 1


@dataclass(frozen=True, slots=True)
class Item:
  """One list item of a section.

  Attributes:
    id: its ID.
    number: its number, `2` or `2a`.
    status: draft, active or repealed.
    repealed: the month it was repealed, if it was.
    content: its text, by language.
    section: the ID of the section holding it.
    omitted: whether the version for agents leaves out its text.
  """

  id: str
  number: str
  status: str
  repealed: str | None
  content: Texts
  section: str
  omitted: bool = False

  @property
  def ref(self) -> str:
    """How the constitution cites it: `<section id>.<item id>`."""
    return f"{self.section}.{self.id}"


@dataclass(frozen=True, slots=True)
class Article:
  """One article.

  Attributes:
    doc: its article.toml.
    id: its ID.
    number: its number, from the directory name.
    repealed: the month it was repealed, if it was.
    title: its title, by language.
    intro: its intro, by language, if it has one.
  """

  doc: Doc
  id: str
  number: str
  repealed: str | None
  title: Texts
  intro: Texts | None


@dataclass(frozen=True, slots=True)
class Section:
  """One section.

  Attributes:
    doc: its file.
    id: its ID.
    article: the number of its article.
    number: its number, from the file name.
    status: draft, active or repealed.
    repealed: the month it was repealed, if it was.
    content: its text, by language.
    items: its list items.
    omitted: whether the version for agents leaves out its text.
  """

  doc: Doc
  id: str
  article: str
  number: str
  status: str
  repealed: str | None
  content: Texts
  items: tuple[Item, ...] = ()
  omitted: bool = False


@dataclass(frozen=True, slots=True)
class Rule:
  """One general rule.

  Attributes:
    doc: its file.
    id: its ID.
    status: draft, active or repealed.
    authority: the constitution provisions it rests on.
    title: its title, by language.
    content: its text, by language.
  """

  doc: Doc
  id: str
  status: str
  authority: list[str]
  title: Texts
  content: Texts


class Text(NamedTuple):
  """One multilingual text of the sources.

  Attributes:
    doc: the file holding it.
    field: its field, as in error messages.
    texts: the text, by language.
    in_rule: whether a rule holds it.
    repealed: whether what holds it is repealed.
  """

  doc: Doc
  field: str
  texts: Texts
  in_rule: bool
  repealed: bool


@dataclass(frozen=True)
class Sources:
  """Everything read from the sources.

  Attributes:
    index: constitution/index.toml.
    languages: the languages of index.toml, in order.
    articles: the articles, in order.
    sections: the sections, in order.
    rules: the general rules.
    retired: rules/retired-ids.txt, in order.
  """

  index: Doc
  languages: list[str]
  articles: list[Article]
  sections: list[Section]
  rules: list[Rule]
  retired: list[str]

  @property
  def items(self) -> list[Item]:
    """Every list item of every section, in order."""
    return [item for section in self.sections for item in section.items]

  @cached_property
  def provisions(self) -> set[str]:
    """Return every constitution citation.

    Returns:
      The IDs of the articles and sections and the refs of the items.
    """
    return (
      {a.id for a in self.articles}
      | {s.id for s in self.sections}
      | {i.ref for i in self.items}
    )

  @cached_property
  def repealed(self) -> set[str]:
    """Return the constitution citations that are repealed.

    An item of a repealed section is repealed with it.

    Returns:
      The repealed citations, as in `provisions`.
    """
    return (
      {a.id for a in self.articles if a.repealed}
      | {s.id for s in self.sections if s.status == "repealed"}
      | {
        i.ref
        for s in self.sections
        for i in s.items
        if i.status == "repealed" or s.status == "repealed"
      }
    )

  def texts(self) -> Iterator[Text]:
    """Yield every multilingual text.

    Yields:
      Each text, with where it is.
    """
    index = self.index.data
    yield Text(self.index, "title", index["title"], False, False)
    yield Text(self.index, "front", index["front"], False, False)
    for a in self.articles:
      yield Text(a.doc, "title", a.title, False, bool(a.repealed))
      if a.intro is not None:
        yield Text(a.doc, "intro", a.intro, False, bool(a.repealed))
    for s in self.sections:
      yield Text(s.doc, "content", s.content, False, s.status == "repealed")
      for i in s.items:
        yield Text(
          s.doc, f"{i.id}.content", i.content, False, i.status == "repealed"
        )
    for r in self.rules:
      yield Text(r.doc, "title", r.title, True, r.status == "repealed")
      yield Text(r.doc, "content", r.content, True, r.status == "repealed")


def number_key(number: str) -> tuple[int, str]:
  """Return the sort key of a number, `2a` as (2, "a").

  Args:
    number: a number checked against NUMBER.

  Returns:
    The key.
  """
  m = NUMBER.fullmatch(number)
  assert m is not None
  return (int(m.group(1)), m.group(2))


def name_number(digits: str, suffix: str = "") -> str:
  """Return the number a name prefix stands for: `02a` is 2a.

  Args:
    digits: the two digits.
    suffix: the letter, if any.

  Returns:
    The number.
  """
  return f"{int(digits)}{suffix}"


def citations(text: str) -> list[str]:
  """Return the citations in the text as written.

  With their namespace if any: `leaks`, `must-ask.capital`,
  `constitution:leaks`, `rule:x`.

  Args:
    text: the source text.

  Returns:
    The citations, in order.
  """
  return [
    (f"{m.group(1)}:" if m.group(1) else "") + m.group(2)
    for m in MARK.finditer(text)
  ]


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
    if kind == LANG:
      if not isinstance(value, dict) or not all(
        isinstance(v, str) for v in value.values()
      ):
        errors.add(
          doc.path,
          line,
          1,
          f"{label}`{name}` must be a table of language to text",
        )
      elif languages is not None and set(value) != set(languages):
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
    elif not isinstance(value, kind):
      errors.add(
        doc.path, line, 1, f"{label}`{name}` must be a {kind.__name__}"
      )
  for name in table.keys() - {k.rstrip("?") for k in schema}:
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
    if kind == LANG and name in table:
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


def load_index(root: Path, errors: Errors) -> Doc | None:
  """Read index.toml and check its format.

  Args:
    root: the checkout.
    errors: where errors go.

  Returns:
    The index, or None when any error was found.
  """
  index = read_toml(root, INDEX, errors)
  if not index or not check_keys(index, index.data, INDEX_SCHEMA, None, errors):
    return None
  languages = index.data["languages"]
  if (
    not languages
    or not all(isinstance(lang, str) for lang in languages)
    or len(set(languages)) != len(languages)
  ):
    errors.add(
      INDEX,
      index.line("languages ="),
      1,
      "`languages` must list each language once",
    )
    return None
  for lang in languages:
    if lang not in LANGUAGES:
      errors.add(
        INDEX,
        index.line("languages ="),
        1,
        f"no rendering for language `{lang}`;"
        " add it to LANGUAGES in constitution_languages.py",
      )
  index_status = index.data["status"]
  if index_status not in INDEX_STATUS:
    errors.add(
      INDEX,
      index.line("status ="),
      1,
      f"`status` must be one of {INDEX_STATUS}",
    )
  edition = index.data.get("edition")
  if isinstance(edition, str) and not MONTH.fullmatch(edition):
    errors.add(
      INDEX,
      index.line("edition ="),
      1,
      f"`edition` must be `YYYY-MM`, got `{edition}`",
    )
  check_keys(index, index.data, INDEX_SCHEMA, languages, errors)
  if errors:
    return None
  strip_texts(index.data, INDEX_SCHEMA)
  return index


@dataclass
class _Reader:
  """What reading the article and rule sources needs, once index.toml is read.

  Attributes:
    root: the checkout.
    errors: where errors go.
    languages: the languages of index.toml.
    index_status: the status of index.toml.
    articles: the articles read so far.
    sections: the sections read so far.
    rules: the rules read so far.
  """

  root: Path
  errors: Errors
  languages: list[str]
  index_status: str
  articles: list[Article] = field(default_factory=list)
  sections: list[Section] = field(default_factory=list)
  rules: list[Rule] = field(default_factory=list)

  def own_id(self, doc: Doc, expected: str, name: str) -> None:
    """Check that the ID is the one the file or directory name gives.

    Args:
      doc: the file.
      expected: the ID in the name.
      name: the file or directory name.
    """
    if doc.data["id"] != expected:
      self.errors.add(
        doc.path,
        doc.line("id ="),
        1,
        f"`id` must be `{expected}`, as in the name `{name}`",
      )

  def status_of(self, doc: Doc, table: dict, what: str, start: int = 1) -> str:
    """Check a status against the allowed ones and index.toml's.

    Args:
      doc: the file holding the table.
      table: the section, item or rule.
      what: what the table is, in messages.
      start: the table's first line.

    Returns:
      The status.
    """
    status = table["status"]
    line = doc.line("status =", start=start)
    if status not in STATUS:
      self.errors.add(
        doc.path,
        line,
        1,
        f"{what} `status` must be one of {STATUS}",
      )
    elif self.index_status == "draft" and status != "draft":
      self.errors.add(
        doc.path,
        line,
        1,
        f"{what} is {status} while index.toml is draft",
      )
    elif self.index_status == "active" and what != "rule" and status == "draft":
      self.errors.add(
        doc.path,
        line,
        1,
        f"{what} is draft while index.toml is active",
      )
    return status

  def omitted(self, doc: Doc, table: dict, what: str, start: int = 1) -> bool:
    """Check `audience` and say whether the version for agents omits the text.

    Args:
      doc: the file holding the table.
      table: the section or item.
      what: what the table is, in messages.
      start: the table's first line.

    Returns:
      Whether the table is for maintainers.
    """
    audience = table.get("audience")
    if audience not in (None, MAINTAINERS):
      self.errors.add(
        doc.path,
        doc.line("audience =", start=start),
        1,
        f"{what} `audience` must be `{MAINTAINERS}`",
      )
    return audience == MAINTAINERS

  def read(self, path: Path, schema: dict) -> Doc | None:
    """Read one source and check its keys.

    Args:
      path: the file, relative to root.
      schema: its schema.

    Returns:
      The file, or None when it cannot be read or its keys are wrong.
    """
    doc = read_toml(self.root, path, self.errors)
    if not doc or not check_keys(
      doc, doc.data, schema, self.languages, self.errors
    ):
      return None
    strip_texts(doc.data, schema)
    return doc

  def entries(self, directory: Path) -> list[Path]:
    """Return what the directory holds, skipping hidden names.

    Hidden names are `.DS_Store`, editor swap files and the like.

    Args:
      directory: the directory, relative to root.

    Returns:
      Its entries, relative to root, sorted.
    """
    return sorted(
      p.relative_to(self.root)
      for p in (self.root / directory).glob("*")
      if not p.name.startswith(".")
    )

  def article_directory(self, directory: Path) -> None:
    """Read one article directory: its article.toml, then its sections.

    Args:
      directory: the directory, relative to root.
    """
    if not (self.root / directory).is_dir():
      self.errors.add(
        directory,
        1,
        1,
        f"not an article directory; an article is {ARTICLES}/<NN>-<id>/"
        f" with its {ARTICLE_FILE} and section files",
      )
      return
    m = ARTICLE_NAME.fullmatch(directory.name)
    if not m:
      self.errors.add(
        directory, 1, 1, f"article directory name is not {ARTICLE_NAME_SHAPE}"
      )
      return
    article_id = m.group(2)
    path = directory / ARTICLE_FILE
    if not (self.root / path).is_file():
      self.errors.add(
        directory, 1, 1, f"article directory lacks `{ARTICLE_FILE}`"
      )
      return
    self.article(path, article_id, m.group(1))
    for path in self.entries(directory):
      if path.name != ARTICLE_FILE:
        self.section(path, article_id)

  def article(self, path: Path, article_id: str, digits: str) -> None:
    """Read one article.toml.

    Args:
      path: the file, relative to root.
      article_id: the ID in the directory name.
      digits: the number prefix of the directory name.
    """
    doc = self.read(path, ARTICLE_SCHEMA)
    if not doc:
      return
    self.own_id(doc, article_id, path.parent.name)
    check_repealed(doc, doc.data, None, self.errors)
    if self.index_status == "draft" and "repealed" in doc.data:
      self.errors.add(
        path,
        doc.line("repealed ="),
        1,
        "an article is repealed while index.toml is draft",
      )
    d = doc.data
    self.articles.append(
      Article(
        doc,
        d["id"],
        name_number(digits),
        d.get("repealed"),
        d["title"],
        d.get("intro"),
      )
    )

  def section(self, path: Path, article_id: str) -> None:
    """Read one section file with its items.

    Args:
      path: the file, relative to root.
      article_id: the ID of its article.
    """
    if path.suffix != ".toml" or not (self.root / path).is_file():
      self.errors.add(
        path,
        1,
        1,
        f"not a section file; an article's directory holds only its"
        f" {ARTICLE_FILE} and <NN>[a-z]-<id>.toml section files",
      )
      return
    m = SECTION_NAME.fullmatch(path.stem)
    if not m:
      self.errors.add(
        path,
        1,
        1,
        f"section file name is not {SECTION_NAME_SHAPE}, then `.toml`",
      )
      return
    doc = self.read(path, SECTION_SCHEMA)
    if not doc:
      return
    self.own_id(doc, m.group(3), path.name)
    status = self.status_of(doc, doc.data, "a section")
    check_repealed(doc, doc.data, status, self.errors)
    d = doc.data
    section_omitted = self.omitted(doc, d, "a section")
    items = []
    for n, item in enumerate(d.get("items", [])):
      if found := self.item(doc, n, item):
        items.append(found)
    self.sections.append(
      Section(
        doc,
        d["id"],
        article_id,
        name_number(m.group(1), m.group(2)),
        status,
        d.get("repealed"),
        d["content"],
        tuple(items),
        section_omitted,
      )
    )

  def item(self, doc: Doc, n: int, item: object) -> Item | None:
    """Check one list item of a section.

    Args:
      doc: the section's file.
      n: the item's index, from 0.
      item: the item as parsed.

    Returns:
      The item, or None when its keys are wrong.
    """
    where = f"items[{n}]"
    start = doc.item_line(n)
    if not isinstance(item, dict) or not check_keys(
      doc, item, ITEM_SCHEMA, self.languages, self.errors, where, start
    ):
      return None
    strip_texts(item, ITEM_SCHEMA)
    if not NUMBER.fullmatch(item["number"]):
      self.errors.add(
        doc.path,
        doc.line("number =", start=start),
        1,
        f"`{where}` `number` `{item['number']}` is not {NUMBER_SHAPE}",
      )
    status = self.status_of(doc, item, f"`{where}`", start)
    check_repealed(doc, item, status, self.errors, where, start)
    return Item(
      item["id"],
      item["number"],
      status,
      item.get("repealed"),
      item["content"],
      doc.data["id"],
      self.omitted(doc, item, f"`{where}`", start),
    )

  def rule(self, path: Path) -> None:
    """Read one rule.

    Args:
      path: the file, relative to root.
    """
    doc = self.read(path, RULE_SCHEMA)
    if not doc:
      return
    self.own_id(doc, path.stem, path.name)
    status = self.status_of(doc, doc.data, "a rule")
    d = doc.data
    if not d["authority"] or not all(
      isinstance(a, str) for a in d["authority"]
    ):
      self.errors.add(
        path,
        doc.line("authority ="),
        1,
        "`authority` must be a non-empty list of citations",
      )
    self.rules.append(
      Rule(doc, d["id"], status, d["authority"], d["title"], d["content"])
    )


def read_retired(root: Path, errors: Errors) -> list[str]:
  """Read rules/retired-ids.txt.

  Args:
    root: the checkout.
    errors: where errors go.

  Returns:
    Its lines, or none when it is missing or cannot be read.
  """
  if not (root / RETIRED).exists():
    return []
  try:
    return (root / RETIRED).read_text(encoding="utf-8").splitlines()
  except OSError as e:
    errors.add(RETIRED, 1, 1, f"cannot read: {e.strerror}")
  except UnicodeDecodeError:
    errors.add(RETIRED, 1, 1, "not UTF-8")
  return []


def load(root: Path, errors: Errors) -> Sources | None:
  """Check 1: read every source and check its format.

  Args:
    root: the checkout.
    errors: where errors go.

  Returns:
    The sources, or None when any error was found.
  """
  index = load_index(root, errors)
  if not index:
    return None
  languages = index.data["languages"]
  reader = _Reader(root, errors, languages, index.data["status"])
  if (root / OLD_SECTIONS).exists():
    errors.add(
      OLD_SECTIONS,
      1,
      1,
      "sections belong in their article's directory,"
      f" {ARTICLES}/<NN>-<id>/<NN>[a-z]-<id>.toml",
    )
  for directory in reader.entries(ARTICLES):
    reader.article_directory(directory)
  for path in sorted(
    p.relative_to(root) for p in (root / RULES).glob("*.toml")
  ):
    reader.rule(path)
  retired = read_retired(root, errors)
  if errors:
    return None
  return Sources(
    index, languages, reader.articles, reader.sections, reader.rules, retired
  )
