"""The model of the constitution and rule sources.

Where the sources are, the shapes of their names and IDs, and the records
they are read into (docs/design/source-formats.md,
constitution.md and rule-format.md).
"""

from collections.abc import Iterable, Iterator
from dataclasses import dataclass
from functools import cached_property
from pathlib import Path
import re
from typing import Any, NamedTuple

CONSTITUTION = Path("constitution")
INDEX_FILE = "index.toml"
ARTICLE_FILE = "article.toml"
RULES = Path("rules")

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
# A node of the rule tree: a rule, a one-of or a domain in the format of
# docs/design/rule-format.md (Rule file format), named `rules/<id>.toml` by
# its number. NODE_NAME is any name of digits, as in a base commit's listing.
NODE_NAME = re.compile(r"[0-9]+")
NODE_ID = re.compile(r"[1-9][0-9]*")
# An entry of `requires` or `conflicts` naming a node of another source.
FOREIGN_REF = re.compile(rf"{ID}:[1-9][0-9]*")
# A first-party rule as a rule set names it, `R12`: in this repo's rules,
# a node of the same source.
FIRST_PARTY_REF = re.compile(r"R[1-9][0-9]*")
# A language code such as `en`, `zh` or `zh-Hant`.
LANG_CODE = re.compile(r"[a-z]{2,3}(?:-[A-Za-z0-9]+)*")
# One clause of `constitution`: revisions are whole numbers, so only the
# comparisons of whole numbers (docs/design/rule-format.md, Tracing rules
# to the constitution).
REVISION_CLAUSE = re.compile(r"\s*(?:[<>]=?|[=!]=)\s*([0-9]+)\s*")
# The largest revision a clause may name: statute reads revisions as
# unsigned 64-bit integers.
REVISION_MAX = 2**64 - 1
KINDS = ("rule", "one-of", "domain")
NODE_STATUS = ("draft", "trial", "active", "superseded", "repealed")
LEVELS = ("project", "user")
CHECKS = ("builtin", "command", "review")
PARTS = ("statement", "reason", "verification")
# The one value of `audience`: a provision on how this text itself is made
# and amended, whose text the version for agents leaves out.
MAINTAINERS = "maintainers"


class LayoutError(Exception):
  """The checkout or commit has no edition directory, or more than one."""


@dataclass(frozen=True)
class Layout:
  """Where the constitution sources and the generated files are.

  The sources are in an edition directory, `constitution/<YYYY-MM>/`, and
  the generated files in its `generated/`.

  Attributes:
    dir: the directory holding index.toml and articles/.
  """

  dir: Path

  @property
  def index(self) -> Path:
    """index.toml."""
    return self.dir / INDEX_FILE

  @property
  def articles(self) -> Path:
    """The directory of the article directories."""
    return self.dir / "articles"

  @property
  def old_sections(self) -> Path:
    """Where sections were before they moved into their articles."""
    return self.dir / "sections"

  def output(self, name: str) -> Path:
    """Return where a generated file goes.

    Args:
      name: the language, with `.agent` after it for the version for
        agents.

    Returns:
      The path, relative to the checkout.
    """
    return self.dir / "generated" / f"constitution.{name}.md"

  @classmethod
  def find(cls, files: Iterable[Path]) -> Layout | None:
    """Return the layout of a checkout or a commit from its files.

    The edition directory is the one directory under `constitution/`
    that holds an index.toml.

    Args:
      files: paths relative to the checkout.

    Returns:
      The layout, or None when no directory holds an index.toml.

    Raises:
      LayoutError: when more than one directory holds an index.toml.
    """
    dirs = sorted(
      {
        p.parent
        for p in files
        if p.name == INDEX_FILE and p.parent.parent == CONSTITUTION
      }
    )
    if len(dirs) > 1:
      raise LayoutError(
        "more than one edition directory: " + ", ".join(map(str, dirs))
      )
    return cls(dirs[0]) if dirs else None

  @classmethod
  def of(cls, root: Path) -> Layout:
    """Return the layout of the working tree.

    Args:
      root: the checkout.

    Returns:
      The layout.

    Raises:
      LayoutError: when not exactly one directory holds an index.toml.
    """
    layout = cls.find(
      p.relative_to(root) for p in root.glob(f"{CONSTITUTION}/*/{INDEX_FILE}")
    )
    if layout is None:
      raise LayoutError(
        f"no edition directory, {CONSTITUTION}/<YYYY-MM>/ with its {INDEX_FILE}"
      )
    return layout


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
    article: the ID of its article.
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
class Node:
  """One node of the rule tree: a rule, a one-of or a domain.

  Attributes:
    doc: its file.
    id: its number.
    slug: its readable name, if it has one.
    kind: rule, one-of or domain.
    status: draft, trial, active, superseded or repealed.
    lang: the language of its original.
    parent: the number of its parent, or for a rule a reference
      `<source>:<id>` to a domain of another source; None for a domain
      that is not a subdomain, and for a rule that overrides another.
    overrides: the rule of another source it overrides, `<source>:<id>`;
      None for a rule that overrides none, as for a domain or a one-of.
    superseded_by: the numbers of the rules that supersede it; empty
      unless it is superseded.
    authority: the constitution provisions it refines, without a
      namespace; empty for a one-of or a domain.
    title: its title, by language.
    applicability: a domain's applicability, by language; empty when it
      has none, as a rule or a one-of always.
    text: its three parts, part -> text by language; empty for a one-of
      or a domain.
    translations: language -> the hash of the original it follows.
    original: the hash of its original text, as `translations` records it.
  """

  doc: Doc
  id: int
  slug: str | None
  kind: str
  status: str
  lang: str
  parent: int | str | None
  overrides: str | None
  superseded_by: list[int]
  authority: list[str]
  title: Texts
  applicability: Texts
  text: dict[str, Texts]
  translations: dict[str, str]
  original: str


class Text(NamedTuple):
  """One multilingual text of the sources.

  Attributes:
    doc: the file holding it.
    field: its field, as in error messages.
    texts: the text, by language.
    in_rule: whether a rule holds it.
    repealed: whether what holds it is repealed.
    lang: the language of its original, None for one in every language of
      index.toml.
  """

  doc: Doc
  field: str
  texts: Texts
  in_rule: bool
  repealed: bool
  lang: str | None = None


@dataclass(frozen=True)
class Sources:
  """Everything read from the sources.

  Attributes:
    layout: where the sources are.
    index: index.toml.
    languages: the languages of index.toml, in order.
    articles: the articles, in order.
    sections: the sections, in order.
    nodes: the rules, one-ofs and domains, by number.
  """

  layout: Layout
  index: Doc
  languages: list[str]
  articles: list[Article]
  sections: list[Section]
  nodes: list[Node]

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
  def ancestors(self) -> dict[str, tuple[str, ...]]:
    """Return the provisions above each provision.

    Returns:
      For each citation of a section, its article; for each item, its
      section and its article; nearest first.
    """
    above: dict[str, tuple[str, ...]] = {}
    for s in self.sections:
      above[s.id] = (s.article,)
      for i in s.items:
        above[i.ref] = (s.id, s.article)
    return above

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
    for n in self.nodes:
      repealed = n.status == "repealed"
      yield Text(n.doc, "title", n.title, True, repealed, n.lang)
      if n.applicability:
        yield Text(
          n.doc, "applicability", n.applicability, True, repealed, n.lang
        )
      for part, texts in n.text.items():
        yield Text(n.doc, f"text.{part}", texts, True, repealed, n.lang)


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
