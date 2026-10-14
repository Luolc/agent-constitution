"""The model of the constitution and rule sources.

Where the sources are, the shapes of their names and IDs, and the records
they are read into (docs/design.md, Source formats).
"""

from collections.abc import Iterator
from dataclasses import dataclass
from functools import cached_property
from pathlib import Path
import re
from typing import Any, NamedTuple

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
