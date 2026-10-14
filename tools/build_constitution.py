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
Within an edition, and from one draft to the next whatever edition it
names, the revision rises when the constitution text changes and stays
when it does not.
Errors print as `path:line: check N: message`; the exit status is 1 on any
error.
"""

import argparse
from collections.abc import Callable, Iterator
from dataclasses import dataclass, field
from pathlib import Path
import re
import subprocess
import sys
import tomllib

import git_env

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

ROMAN = [
  (1000, "M"),
  (900, "CM"),
  (500, "D"),
  (400, "CD"),
  (100, "C"),
  (90, "XC"),
  (50, "L"),
  (40, "XL"),
  (10, "X"),
  (9, "IX"),
  (5, "V"),
  (4, "IV"),
  (1, "I"),
]
ZH_DIGITS = "零一二三四五六七八九"
EN_MONTHS = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
]


def roman(n: int) -> str:
  """Return n in Roman numerals.

  Args:
    n: the number, from 1.

  Returns:
    The numeral, such as `XIV`.
  """
  out = ""
  for value, symbol in ROMAN:
    count, n = divmod(n, value)
    out += symbol * count
  return out


def zh_number(n: int) -> str:
  """Return n in Chinese numerals, for 1 to 99.

  Args:
    n: the number.

  Returns:
    The numeral, such as 十四.

  Raises:
    ValueError: when n is outside 1 to 99.
  """
  if not 0 < n < 100:
    raise ValueError(f"no Chinese numeral rendering for {n}")
  tens, ones = divmod(n, 10)
  if not tens:
    return ZH_DIGITS[ones]
  return (
    ("" if tens == 1 else ZH_DIGITS[tens])
    + "十"
    + (ZH_DIGITS[ones] if ones else "")
  )


def zh_suffix(suffix: str) -> str:
  """Return the letter suffix of a number in Chinese.

  a is 之一, b is 之二, and so on.

  Args:
    suffix: the letter, or "" for none.

  Returns:
    The Chinese suffix, or "" for none.
  """
  return f"之{zh_number(ord(suffix) - ord('a') + 1)}" if suffix else ""


@dataclass(frozen=True)
class Language:
  """Everything that differs between output languages.

  Adding a language means adding one entry to LANGUAGES and listing it in
  index.toml.

  Attributes:
    header: {revision}.
    legend: follows the header when a provision is marked.
    agent_header: {revision}: the version for agents.
    omitted: in place of a marked provision's text, for agents.
    agent_front: {opening}: in place of the front text, for agents.
    agent_draft: the opening while a draft.
    agent_active: the opening once active.
    draft: in the title's parentheses while a draft without an edition.
    edition: {edition}, in the title's parentheses.
    draft_revision: {revision}, the revision while a draft.
    article_heading: {anchor}, {number}, {title}.
    repealed_article_heading: {anchor}, {number}, {note}.
    article_ref: {number}.
    section_ref: {article}, {section}.
    article_number: number and suffix to the article's number.
    section_number: number and suffix to the section's number.
    item_marker: starts an item's list line, after its anchor.
    item_ref: follows the section's citation.
    connectors: between item citations rendered short.
    repealed_note: year, month.
  """

  header: str
  legend: str
  agent_header: str
  omitted: str
  agent_front: str
  agent_draft: str
  agent_active: str
  draft: str
  edition: str
  draft_revision: str
  article_heading: str
  repealed_article_heading: str
  article_ref: str
  section_ref: str
  article_number: Callable[[int, str], str]
  section_number: Callable[[int, str], str]
  item_marker: Callable[[int, str], str]
  item_ref: Callable[[int, str], str]
  connectors: tuple[str, ...]
  repealed_note: Callable[[int, int], str]


LANGUAGES = {
  "en": Language(
    header=(
      "Generated from `constitution/index.toml` and `constitution/articles/`"
      " by `tools/build_constitution.py`."
      " Do not edit this file; edit the sources and rebuild."
      " Revision: {revision}. Language: English."
    ),
    legend=(
      " \\* Background for people: where this constitution comes from, how it"
      " is organized and how it is amended. The agent version,"
      " `constitution/constitution.en.agent.md`, omits these parts."
    ),
    agent_header=(
      "Generated from `constitution/index.toml` and `constitution/articles/`"
      " by `tools/build_constitution.py`: the version for agents, without the"
      " text of the provisions written for human readers."
      " Do not edit this file; edit the sources and rebuild."
      " Revision: {revision}. Language: English."
    ),
    omitted="(see the full text)",
    agent_front=(
      "{opening}; it leaves out the preamble and the text of the provisions"
      " written for human readers, on how this constitution is organized and"
      " amended, keeping only a pointer to the full version in their place,"
      " so before changing this constitution or the tools that build it,"
      " read the full version, `constitution/constitution.en.md`."
    ),
    agent_draft=(
      "This is the version of this constitution for agents,"
      " and the constitution is a draft not yet in force"
    ),
    agent_active=(
      "This is the version of this constitution for agents,"
      " and every agent must follow it"
    ),
    draft="Draft",
    edition="{edition} edition",
    draft_revision="{revision} (draft)",
    article_heading="## {anchor}Article {number}. {title}",
    repealed_article_heading="## {anchor}Article {number} ({note})",
    article_ref="Art. {number}",
    section_ref="{article} {section}",
    article_number=lambda n, s: roman(n) + s,
    section_number=lambda n, s: f"§{n}{s}",
    item_marker=lambda n, s: f"(**{n}{s}**) ",
    item_ref=lambda n, s: f"({n}{s})",
    connectors=(" or ", " and ", ", ", " to "),
    repealed_note=lambda y, m: f"repealed {EN_MONTHS[m - 1]} {y}",
  ),
  "zh": Language(
    header=(
      "本文件由 `tools/build_constitution.py` 从 `constitution/index.toml` 和"
      " `constitution/articles/` 生成，不要手改；"
      "改源文件后重新生成。修订号：{revision}。语言：中文。"
    ),
    legend=(
      "\\* 写给人看的背景：本宪法的由来、结构和修改方式。agent 版"
      " `constitution/constitution.zh.agent.md` 不收录这些部分。"
    ),
    agent_header=(
      "本文件由 `tools/build_constitution.py` 从 `constitution/index.toml` 和"
      " `constitution/articles/` 生成，是给 agent 的版本，"
      "不收写给人读的条款的正文；"
      "不要手改，改源文件后重新生成。"
      "修订号：{revision}。语言：中文。"
    ),
    omitted="(见完整版)",
    agent_front=(
      "{opening}；不含序言；"
      "写给人读、讲本宪法怎样编排和修改的条款不收正文，"
      "只留一句「见完整版」；"
      "修改本宪法或编译它的工具时，"
      "先读完整版 `constitution/constitution.zh.md`。"
    ),
    agent_draft="这是本宪法给 agent 读的版本，本宪法现为草案，尚未生效",
    agent_active="这是本宪法给 agent 读的版本，每个 agent 都必须遵守它",
    draft="草案",
    edition="{edition} 版",
    draft_revision="{revision} (草案)",
    article_heading="## {anchor}{number} {title}",
    repealed_article_heading="## {anchor}{number} ({note})",
    article_ref="{number}",
    section_ref="{article}{section}",
    article_number=lambda n, s: f"第{zh_number(n)}条" + zh_suffix(s),
    section_number=lambda n, s: f"第{zh_number(n)}款" + zh_suffix(s),
    item_marker=lambda n, s: f"(**{zh_number(n)}{zh_suffix(s)}**) ",
    item_ref=lambda n, s: f"第{zh_number(n)}项" + zh_suffix(s),
    connectors=("或", "和", "、", "至"),
    repealed_note=lambda y, m: f"{y} 年 {m} 月废止",
  ),
}


# Schemas: key -> type; a trailing `?` marks an optional key; LANG is a table
# whose keys are exactly the languages of index.toml and whose values are text.
LANG = "lang"
INDEX_SCHEMA = {
  "status": str,
  "edition?": str,
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


@dataclass
class Doc:
  """One parsed TOML file with its text, for locating errors by line.

  Attributes:
    path: the file, relative to the checkout.
    data: the parsed TOML.
    text: the file's text.
  """

  path: Path
  data: dict
  text: str

  def line(self, *needles: str) -> int:
    """Return the first line holding the first needle found, else 1.

    Args:
      *needles: the strings to look for, in order.

    Returns:
      The line number, from 1.
    """
    for needle in needles:
      for n, line in enumerate(self.text.split("\n"), start=1):
        if needle in line:
          return n
    return 1


@dataclass
class Item:
  """One list item of a section.

  Attributes:
    id: its ID.
    number: its number, `2` or `2a`.
    status: draft, active or repealed.
    repealed: the month it was repealed, if it was.
    content: its text, by language.
    section: the section holding it.
    omitted: whether the version for agents leaves out its text.
  """

  id: str
  number: str
  status: str
  repealed: str | None
  content: dict
  section: Section
  omitted: bool = False

  @property
  def ref(self) -> str:
    """How the constitution cites it: `<section id>.<item id>`."""
    return f"{self.section.id}.{self.id}"


@dataclass
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
  title: dict
  intro: dict | None


@dataclass
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
  content: dict
  items: list[Item] = field(default_factory=list)
  omitted: bool = False


@dataclass
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
  authority: list
  title: dict
  content: dict


@dataclass
class Sources:
  """Everything read from the sources.

  Attributes:
    index: constitution/index.toml.
    languages: the languages of index.toml, in order.
    articles: the articles, in order.
    sections: the sections, in order.
    rules: the general rules.
    retired: rules/retired-ids.txt, in order.
    provisions: constitution citation -> article | section | item.
    repealed: the constitution citations that are repealed.
  """

  index: Doc
  languages: list[str]
  articles: list[Article]
  sections: list[Section]
  rules: list[Rule]
  retired: list[str]
  provisions: dict = field(default_factory=dict)
  repealed: set = field(default_factory=set)

  @property
  def items(self) -> list[Item]:
    """Every list item of every section, in order."""
    return [item for section in self.sections for item in section.items]

  def texts(self) -> Iterator[tuple[Doc, str, dict, bool, bool]]:
    """Yield every multilingual text.

    Yields:
      (doc, field name, lang -> text, source is a rule, source is
      repealed).
    """
    yield self.index, "title", self.index.data["title"], False, False
    yield self.index, "front", self.index.data["front"], False, False
    for a in self.articles:
      yield a.doc, "title", a.title, False, bool(a.repealed)
      if a.intro is not None:
        yield a.doc, "intro", a.intro, False, bool(a.repealed)
    for s in self.sections:
      yield s.doc, "content", s.content, False, s.status == "repealed"
      for i in s.items:
        yield s.doc, f"{i.id}.content", i.content, False, i.status == "repealed"
    for r in self.rules:
      yield r.doc, "title", r.title, True, r.status == "repealed"
      yield r.doc, "content", r.content, True, r.status == "repealed"


def number_key(number: str) -> tuple:
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
        errors.add(doc.path, 1, 1, f"{label}lacks `{name}`")
      continue
    value = table[name]
    line = doc.line(f"[{name}]", f"{name} =")
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
      doc.line(f"[{name}]", f"{name} ="),
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
  doc: Doc, table: dict, status: str | None, errors: Errors, where: str = ""
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
  """
  label = f"`{where}` " if where else ""
  value = table.get("repealed")
  line = doc.line("repealed =")
  if value is not None and not MONTH.fullmatch(value):
    errors.add(
      doc.path, line, 1, f"{label}`repealed` must be `YYYY-MM`, got `{value}`"
    )
  if status == "repealed" and value is None:
    errors.add(
      doc.path,
      doc.line("status ="),
      1,
      f"{label}status is repealed but there is no `repealed` date",
    )
  if status not in (None, "repealed") and value is not None:
    errors.add(
      doc.path, line, 1, f"{label}has `repealed` but status is {status}"
    )


def load(root: Path, errors: Errors) -> Sources | None:  # noqa: C901
  """Check 1: read every source and check its format.

  Args:
    root: the checkout.
    errors: where errors go.

  Returns:
    The sources, or None when any error was found.
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
        f" add it to LANGUAGES in {Path(__file__).name}",
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
  if index_status == "active" and edition is None:
    errors.add(INDEX, 1, 1, "lacks `edition`, which an active constitution has")
  elif edition is not None and not MONTH.fullmatch(edition):
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

  def own_id(doc: Doc, expected: str, name: str) -> None:
    if doc.data["id"] != expected:
      errors.add(
        doc.path,
        doc.line("id ="),
        1,
        f"`id` must be `{expected}`, as in the name `{name}`",
      )

  def status_of(doc: Doc, table: dict, what: str) -> str:
    status = table["status"]
    if status not in STATUS:
      errors.add(
        doc.path,
        doc.line("status ="),
        1,
        f"{what} `status` must be one of {STATUS}",
      )
    elif index_status == "draft" and status != "draft":
      errors.add(
        doc.path,
        doc.line("status ="),
        1,
        f"{what} is {status} while index.toml is draft",
      )
    elif index_status == "active" and what != "rule" and status == "draft":
      errors.add(
        doc.path,
        doc.line("status ="),
        1,
        f"{what} is draft while index.toml is active",
      )
    return status

  def omitted(doc: Doc, table: dict, what: str) -> bool:
    audience = table.get("audience")
    if audience not in (None, MAINTAINERS):
      errors.add(
        doc.path,
        doc.line("audience ="),
        1,
        f"{what} `audience` must be `{MAINTAINERS}`",
      )
    return audience == MAINTAINERS

  def read(path: Path, schema: dict) -> Doc | None:
    doc = read_toml(root, path, errors)
    if not doc or not check_keys(doc, doc.data, schema, languages, errors):
      return None
    strip_texts(doc.data, schema)
    return doc

  def entries(directory: Path) -> list[Path]:
    """Return what the directory holds, skipping hidden names.

    Hidden names are `.DS_Store`, editor swap files and the like.

    Args:
      directory: the directory, relative to root.

    Returns:
      Its entries, relative to root, sorted.
    """
    return sorted(
      p.relative_to(root)
      for p in (root / directory).glob("*")
      if not p.name.startswith(".")
    )

  if (root / OLD_SECTIONS).exists():
    errors.add(
      OLD_SECTIONS,
      1,
      1,
      "sections belong in their article's directory,"
      f" {ARTICLES}/<NN>-<id>/<NN>[a-z]-<id>.toml",
    )
  articles, sections, rules = [], [], []
  for directory in entries(ARTICLES):
    if not (root / directory).is_dir():
      errors.add(
        directory,
        1,
        1,
        f"not an article directory; an article is {ARTICLES}/<NN>-<id>/"
        f" with its {ARTICLE_FILE} and section files",
      )
      continue
    m = ARTICLE_NAME.fullmatch(directory.name)
    if not m:
      errors.add(
        directory, 1, 1, f"article directory name is not {ARTICLE_NAME_SHAPE}"
      )
      continue
    article_id = m.group(2)
    path = directory / ARTICLE_FILE
    if not (root / path).is_file():
      errors.add(directory, 1, 1, f"article directory lacks `{ARTICLE_FILE}`")
      continue
    doc = read(path, ARTICLE_SCHEMA)
    if doc:
      own_id(doc, article_id, directory.name)
      check_repealed(doc, doc.data, None, errors)
      if index_status == "draft" and "repealed" in doc.data:
        errors.add(
          path,
          doc.line("repealed ="),
          1,
          "an article is repealed while index.toml is draft",
        )
      d = doc.data
      articles.append(
        Article(
          doc,
          d["id"],
          name_number(m.group(1)),
          d.get("repealed"),
          d["title"],
          d.get("intro"),
        )
      )
    for path in entries(directory):
      if path.name == ARTICLE_FILE:
        continue
      if path.suffix != ".toml" or not (root / path).is_file():
        errors.add(
          path,
          1,
          1,
          f"not a section file; an article's directory holds only its"
          f" {ARTICLE_FILE} and <NN>[a-z]-<id>.toml section files",
        )
        continue
      m = SECTION_NAME.fullmatch(path.stem)
      if not m:
        errors.add(
          path,
          1,
          1,
          f"section file name is not {SECTION_NAME_SHAPE}, then `.toml`",
        )
        continue
      doc = read(path, SECTION_SCHEMA)
      if not doc:
        continue
      own_id(doc, m.group(3), path.name)
      status = status_of(doc, doc.data, "a section")
      check_repealed(doc, doc.data, status, errors)
      d = doc.data
      section = Section(
        doc,
        d["id"],
        article_id,
        name_number(m.group(1), m.group(2)),
        status,
        d.get("repealed"),
        d["content"],
        omitted=omitted(doc, d, "a section"),
      )
      for n, item in enumerate(d.get("items", [])):
        where = f"items[{n}]"
        if not isinstance(item, dict) or not check_keys(
          doc, item, ITEM_SCHEMA, languages, errors, where
        ):
          continue
        strip_texts(item, ITEM_SCHEMA)
        if not NUMBER.fullmatch(item["number"]):
          errors.add(
            path,
            doc.line(f'number = "{item["number"]}"'),
            1,
            f"`{where}` `number` `{item['number']}` is not {NUMBER_SHAPE}",
          )
        status = status_of(doc, item, f"`{where}`")
        check_repealed(doc, item, status, errors, where)
        section.items.append(
          Item(
            item["id"],
            item["number"],
            status,
            item.get("repealed"),
            item["content"],
            section,
            omitted(doc, item, f"`{where}`"),
          )
        )
      sections.append(section)
  for path in sorted(
    p.relative_to(root) for p in (root / RULES).glob("*.toml")
  ):
    doc = read(path, RULE_SCHEMA)
    if not doc:
      continue
    own_id(doc, path.stem, path.name)
    status = status_of(doc, doc.data, "a rule")
    d = doc.data
    if not d["authority"] or not all(
      isinstance(a, str) for a in d["authority"]
    ):
      errors.add(
        path,
        doc.line("authority ="),
        1,
        "`authority` must be a non-empty list of citations",
      )
    rules.append(
      Rule(doc, d["id"], status, d["authority"], d["title"], d["content"])
    )
  retired = []
  if (root / RETIRED).exists():
    retired = (root / RETIRED).read_text(encoding="utf-8").splitlines()
  if errors:
    return None
  return Sources(index, languages, articles, sections, rules, retired)


# Check 2: IDs.


def check_ids(src: Sources, errors: Errors) -> None:  # noqa: C901
  """Check 2: every ID has the format and is unique where it must be.

  Article and section IDs are unique in the constitution, item IDs in
  their section, rule IDs among the rules and the retired rule IDs.

  Args:
    src: the sources.
    errors: where errors go.
  """

  def unique_ids(declared: list) -> None:
    seen = {}
    for doc, value in declared:
      line = doc.line(f'id = "{value}"')
      if not re.fullmatch(ID, value):
        errors.add(
          doc.path,
          line,
          2,
          f"`{value}` is not an ID: lowercase words joined by hyphens",
        )
      if value in seen:
        errors.add(
          doc.path, line, 2, f"{value} is already defined at {seen[value]}"
        )
      seen.setdefault(value, f"{doc.path}:{line}")

  unique_ids(
    [(a.doc, a.id) for a in src.articles]
    + [(s.doc, s.id) for s in src.sections]
  )
  for s in src.sections:
    unique_ids([(s.doc, i.id) for i in s.items])
  unique_ids([(r.doc, r.id) for r in src.rules])
  for n, value in enumerate(src.retired, start=1):
    if value and not re.fullmatch(ID, value):
      errors.add(
        RETIRED,
        n,
        2,
        f"`{value}` is not an ID: lowercase words joined by hyphens",
      )
  retired = set(src.retired)
  for r in src.rules:
    if r.id in retired:
      errors.add(
        r.doc.path,
        r.doc.line("id ="),
        2,
        f"{r.id} is retired in {RETIRED}; choose another ID",
      )

  for a in src.articles:
    src.provisions[a.id] = "article"
    if a.repealed:
      src.repealed.add(a.id)
  for s in src.sections:
    src.provisions[s.id] = "section"
    if s.status == "repealed":
      src.repealed.add(s.id)
    for i in s.items:
      src.provisions[i.ref] = "item"
      if i.status == "repealed" or s.status == "repealed":
        src.repealed.add(i.ref)


# Check 3: structure.


def check_structure(src: Sources, errors: Errors) -> None:
  """Check 3: numbers are unique in their scope.

  A repealed article has only repealed sections.

  Args:
    src: the sources.
    errors: where errors go.
  """
  unique(errors, [(a.doc, a.number, "article") for a in src.articles])
  for a in src.articles:
    own = [s for s in src.sections if s.article == a.id]
    unique(errors, [(s.doc, s.number, f"section of {a.id}") for s in own])
    if a.repealed:
      for s in own:
        if s.status != "repealed":
          errors.add(
            s.doc.path,
            s.doc.line("status ="),
            3,
            f"{s.id} is {s.status} but its article {a.id} is repealed",
          )
  for s in src.sections:
    unique(errors, [(s.doc, i.number, f"item of {s.id}") for i in s.items])


def unique(errors: Errors, numbered: list) -> None:
  """Check that no number repeats in one scope.

  Args:
    errors: where errors go.
    numbered: (doc, number, what) for each provision of the scope.
  """
  seen = {}
  for doc, number, what in numbered:
    k = number_key(number)
    line = doc.line(f'number = "{number}"')
    if k in seen:
      errors.add(
        doc.path,
        line,
        3,
        f"{what} number {number} is already used at {seen[k]}",
      )
    seen.setdefault(k, f"{doc.path}:{line}")


# Check 4: language alignment.


def check_languages(src: Sources, errors: Errors) -> None:
  """Check 4: each language of a text cites what the first language cites.

  Args:
    src: the sources.
    errors: where errors go.
  """
  first = src.languages[0]
  for doc, name, texts, _, _ in src.texts():
    refs = {lang: set(citations(text)) for lang, text in texts.items()}
    for lang in src.languages[1:]:
      for a, b in ((first, lang), (lang, first)):
        for ref in sorted(refs[a] - refs[b]):
          errors.add(
            doc.path,
            doc.line(f"{{@{ref}}}"),
            4,
            f"{a} `{name}` cites {ref}; {b} does not",
          )


# Check 5: references.


def check_refs(src: Sources, errors: Errors) -> None:  # noqa: C901
  """Check 5: every citation and authority resolves to an unrepealed target.

  The constitution cites itself without a namespace; a rule cites with
  `constitution:` or `rule:`; every citation and authority resolves to a
  provision or rule that is not repealed.

  Args:
    src: the sources.
    errors: where errors go.
  """
  rules = {r.id: r for r in src.rules}

  def problem(ref: str, in_rule: bool) -> str:
    """Return what is wrong with the citation, or "" when nothing is.

    Args:
      ref: the citation as written.
      in_rule: whether a rule cites it.

    Returns:
      The problem, or "".
    """
    namespace, _, target = ref.rpartition(":")
    if not in_rule and namespace:
      return f"{ref}; the constitution cites only itself, without a namespace"
    if in_rule and not namespace:
      return f"{ref} without `constitution:` or `rule:`"
    if namespace == "rule":
      if target not in rules:
        return f"undefined {ref}"
      return f"repealed {ref}" if rules[target].status == "repealed" else ""
    if target not in src.provisions:
      return f"undefined {ref}"
    return f"repealed {ref}" if target in src.repealed else ""

  for doc, _name, texts, in_rule, repealed in src.texts():
    if repealed:
      continue
    for text in texts.values():
      for ref in citations(text):
        if found := problem(ref, in_rule):
          errors.add(doc.path, doc.line(f"{{@{ref}}}"), 5, f"cites {found}")
  for r in src.rules:
    if r.status == "repealed":
      continue
    line = r.doc.line("authority =")
    for ref in r.authority:
      if not ref.startswith("constitution:"):
        errors.add(
          r.doc.path,
          line,
          5,
          f"authority {ref} is not a `constitution:` citation",
        )
      elif found := problem(ref, True):
        errors.add(r.doc.path, line, 5, f"authority cites {found}")


# Rendering.

# List markers padded to the indent, so that a section's or an item's
# further lines start at its content column: CommonMark takes any indent
# up to that column, Python-Markdown wants four spaces.
INDENT = " " * 4
SECTION = "-   "
ITEM = INDENT + SECTION

OMITTED_MARK = '<sup class="omitted">*</sup>'
# A heading of the front text (the preamble's): the version for agents
# leaves the front text out, so in the full version it is marked as well.
FRONT_HEADING = re.compile(r"^#{2,6} .+$", re.M)


def render(lang: str, src: Sources, agent: bool = False) -> str:  # noqa: C901
  """Return the constitution in one language.

  The full version, or with `agent` the version for agents, without
  anchors and links, its citations as plain text, a note of its own in
  place of the front text, and each provision marked for maintainers as
  its number and a note.

  Args:
    lang: the language.
    src: the sources.
    agent: whether to render the version for agents.

  Returns:
    The Markdown text.
  """
  lng = LANGUAGES[lang]
  # short: a section's or an item's citation without its article or
  # section; parent: the article or section that short leaves out.
  labels, numbers, short, parent = {}, {}, {}, {}
  for a in src.articles:
    numbers[a.id] = lng.article_number(*number_key(a.number))
    labels[a.id] = lng.article_ref.format(number=numbers[a.id])
  for s in src.sections:
    numbers[s.id] = short[s.id] = lng.section_number(*number_key(s.number))
    labels[s.id] = lng.section_ref.format(
      article=labels[s.article], section=numbers[s.id]
    )
    parent[s.id] = s.article
    for i in s.items:
      short[i.ref] = lng.item_ref(*number_key(i.number))
      labels[i.ref] = labels[s.id] + short[i.ref]
      parent[i.ref] = s.id

  cited = {}  # citation -> None, in order of first use: the link definitions

  def anchor(ref: str) -> str:
    return "" if agent else f'<a id="{ref}"></a>'

  def note(month: str | None) -> str:
    # Called only on a repealed provision, which check 1 gave a month.
    assert month is not None
    m = MONTH.fullmatch(month)
    assert m is not None
    return lng.repealed_note(int(m.group(1)), int(m.group(2)))

  def refs(text: str) -> str:
    """Return the text with each citation rendered.

    Each citation becomes a link to its provision's anchor, its label as
    the link text, except that a section or item citation joined by a
    connector to the citation of another section of the same article,
    or another item of the same section, just before it renders short:
    `Art. IV §1 and §2`, `Art. IV §1(2) or (4)`. Reference links, not
    inline ones: limae reads the `)` of an inline link followed by a
    Chinese character as a parenthesis that needs a space after it.

    Args:
      text: the source text.

    Returns:
      The rendered text.
    """
    out, end, previous = [], 0, None
    for m in MARK.finditer(text):
      ref, between = m.group(2), text[end : m.start()]
      joined = (
        ref in parent and parent[ref] == previous and between in lng.connectors
      )
      label = short[ref] if joined else labels[ref]
      if not agent:
        cited[ref] = None
        label = f"[{label}][{ref}]"
      out += [between, label]
      previous, end = parent.get(ref), m.end()
    return "".join(out) + text[end:]

  def indented(first: str, text: str, indent: str) -> list[str]:
    """Return a list line `first` + the text's first line, then the rest.

    The other lines are indented under it.

    Args:
      first: the start of the list line.
      text: the source text.
      indent: the indent of the other lines.

    Returns:
      The lines.
    """
    head, *tail = refs(text).split("\n")
    return [first + head, *(indent + t if t else t for t in tail)]

  def section_entries(s: Section) -> list[list[str]]:
    number = f"{SECTION}{anchor(s.id)}**{numbers[s.id]}**"
    if s.omitted and agent:
      return [[f"{number} {lng.omitted}"]]
    mark = OMITTED_MARK if s.omitted else ""
    entries = [indented(f"{number}{mark} ", s.content[lang], INDENT)]
    for i in sorted(s.items, key=lambda i: number_key(i.number)):
      marker = anchor(i.ref) + lng.item_marker(*number_key(i.number))
      if i.status == "repealed":
        entries.append([f"{ITEM}{marker.rstrip()} ({note(i.repealed)})"])
      elif i.omitted and agent:
        entries.append([f"{ITEM}{marker}{lng.omitted}"])
      elif i.omitted:
        entries.append(
          indented(
            f"{ITEM}{marker.rstrip()}{OMITTED_MARK} ",
            i.content[lang],
            2 * INDENT,
          )
        )
      else:
        entries.append(indented(f"{ITEM}{marker}", i.content[lang], 2 * INDENT))
    return entries

  index = src.index.data
  edition = (
    lng.edition.format(edition=index["edition"])
    if "edition" in index
    else lng.draft
  )
  revision = index["revision"]
  if index["status"] == "draft":
    revision = lng.draft_revision.format(revision=revision)
  header = (lng.agent_header if agent else lng.header).format(revision=revision)
  opening = lng.agent_active if index["status"] == "active" else lng.agent_draft
  front = (
    lng.agent_front.format(opening=opening)
    if agent
    else FRONT_HEADING.sub(
      lambda m: m.group() + OMITTED_MARK, refs(index["front"][lang])
    )
  )
  if not agent and (
    OMITTED_MARK in front
    or any(s.omitted or any(i.omitted for i in s.items) for s in src.sections)
  ):
    header += lng.legend
  parts = [header, f"# {refs(index['title'][lang])} ({edition})\n\n{front}"]
  for a in sorted(src.articles, key=lambda a: number_key(a.number)):
    if a.repealed:
      parts.append(
        lng.repealed_article_heading.format(
          anchor=anchor(a.id), number=numbers[a.id], note=note(a.repealed)
        )
      )
      continue
    block = [
      lng.article_heading.format(
        anchor=anchor(a.id), number=numbers[a.id], title=refs(a.title[lang])
      )
    ]
    if a.intro is not None:
      block.append(refs(a.intro[lang]))
    own = sorted(
      (s for s in src.sections if s.article == a.id),
      key=lambda s: number_key(s.number),
    )
    body = []
    for s in own:
      if s.status == "repealed":
        entries = [
          [f"{SECTION}{anchor(s.id)}**{numbers[s.id]}** ({note(s.repealed)})"]
        ]
      else:
        entries = section_entries(s)
      for e in entries:
        # An entry of several paragraphs ends with a blank line, or a
        # list line right after it would join its last paragraph.
        body += e + [""] if "" in e else e
    if body and body[-1] == "":
      body.pop()
    if body:
      block.append("\n".join(body))
    parts.append("\n\n".join(block))
  if cited:
    parts.append("\n".join(f"[{ref}]: #{ref}" for ref in cited))
  return "\n\n".join(parts) + "\n"


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


# Check 7: numbering against a base commit.


def git(root: Path, *args: str) -> str | None:
  """Run git in root.

  Args:
    root: the checkout.
    *args: the git arguments.

  Returns:
    Its output, or None when it fails.
  """
  result = subprocess.run(
    ["git", "-C", str(root), *args],
    capture_output=True,
    text=True,
    env=git_env.clean(),
  )
  return result.stdout if result.returncode == 0 else None


def check_base(  # noqa: C901
  root: Path, ref: str, src: Sources, errors: Errors
) -> str:
  """Check 7: compare the numbering with a base commit.

  Once the base is active: the edition is the base's or a later one.
  While it is the base's: every article,
  section and item of the base still exists, by ID, with the same number,
  a section in the same article; what was repealed is still repealed;
  every rule of the base still exists or is retired, and every retired ID
  stays retired.

  Args:
    root: the checkout.
    ref: the base commit.
    src: the sources.
    errors: where errors go.

  Returns:
    A note when the step is skipped, else "".
  """
  commit = f"{ref}^{{commit}}"
  if git(root, "rev-parse", "--verify", "--quiet", commit) is None:
    errors.add(INDEX, 1, 7, f"base `{ref}` is not a commit")
    return ""
  base_index = git(root, "show", f"{ref}:{INDEX}")
  if base_index is None:
    return f"base {ref} has no {INDEX}; numbering not compared"
  try:
    base = tomllib.loads(base_index)
  except tomllib.TOMLDecodeError as e:
    errors.add(INDEX, 1, 7, f"{INDEX} at base {ref} is not valid TOML: {e}")
    return ""
  if base.get("status") != "active":
    return f"base {ref} is {base.get('status')}; numbering not compared"
  listing = [
    Path(name)
    for name in (
      git(
        root,
        "ls-tree",
        "-r",
        "--name-only",
        ref,
        "--",
        str(ARTICLES),
        str(RULES),
      )
      or ""
    ).split("\n")
  ]
  if not any(
    p.name == ARTICLE_FILE and p.parent.parent == ARTICLES for p in listing
  ):
    return (
      f"base {ref} has no {ARTICLES}/<NN>-<id>/{ARTICLE_FILE};"
      " numbering not compared"
    )
  if src.index.data["status"] != "active":
    errors.add(
      INDEX,
      src.index.line("status ="),
      7,
      f"status went back to draft from active at base {ref}",
    )
  else:
    edition, base_edition = src.index.data["edition"], base.get("edition")
    # Both are `YYYY-MM`, so string order is month order.
    if edition != base_edition:
      if isinstance(base_edition, str) and edition > base_edition:
        return (
          f"base {ref} is edition {base_edition}, this is edition {edition};"
          " numbering not compared"
        )
      errors.add(
        INDEX,
        src.index.line("edition ="),
        7,
        f"edition {edition} is not later than edition {base_edition}"
        f" at base {ref}",
      )
      return ""

  def base_toml(path: Path) -> dict | None:
    try:
      return tomllib.loads(git(root, "show", f"{ref}:{path}") or "")
    except tomllib.TOMLDecodeError as e:
      errors.add(path, 1, 7, f"at base {ref} is not valid TOML: {e}")
      return None

  articles = {a.id: a for a in src.articles}
  sections = {s.id: s for s in src.sections}
  rules = {r.id: r for r in src.rules}
  retired = set(src.retired)

  def gone(path: Path, what: str) -> None:
    errors.add(
      path, 1, 7, f"{what} existed at base {ref} and is gone; repeal it instead"
    )

  def renumbered(doc: Doc, what: str, old: str, new: str) -> None:
    if old != new:
      errors.add(
        doc.path,
        doc.line(f'number = "{new}"'),
        7,
        f"{what} was renumbered from {old} to {new} since base {ref}",
      )

  def still_repealed(doc: Doc, what: str, was: bool, is_now: bool) -> None:
    if was and not is_now:
      errors.add(
        doc.path,
        doc.line("repealed =", "status ="),
        7,
        f"{what} was repealed at base {ref} and is no longer",
      )

  for path in sorted(listing):
    if path.parent == RULES and path.suffix == ".toml":
      old = base_toml(path)
      now = rules.get(path.stem)
      if old is None:
        continue
      if now is None:
        if path.stem not in retired:
          errors.add(
            path,
            1,
            7,
            f"rule {path.stem} existed at base {ref} and is gone"
            f" without being listed in {RETIRED}",
          )
        continue
      still_repealed(
        now.doc,
        f"rule {now.id}",
        old.get("status") == "repealed",
        now.status == "repealed",
      )
      continue
    if path.parent.parent != ARTICLES or path.suffix != ".toml":
      continue
    article = ARTICLE_NAME.fullmatch(path.parent.name)
    section = SECTION_NAME.fullmatch(path.stem)
    if not article or (path.name != ARTICLE_FILE and not section):
      continue
    old = base_toml(path)
    if old is None:
      continue
    if path.name == ARTICLE_FILE:
      aid = article.group(2)
      now = articles.get(aid)
      if now is None:
        gone(path, f"article {aid}")
        continue
      renumbered(
        now.doc, f"article {aid}", name_number(article.group(1)), now.number
      )
      still_repealed(
        now.doc, f"article {aid}", "repealed" in old, bool(now.repealed)
      )
      continue
    assert section is not None
    sid = section.group(3)
    now = sections.get(sid)
    if now is None:
      gone(path, f"section {sid}")
      continue
    if now.article != article.group(2):
      errors.add(
        now.doc.path,
        1,
        7,
        f"section {sid} moved from article {article.group(2)}"
        f" to {now.article} since base {ref}",
      )
    else:
      renumbered(
        now.doc,
        f"section {sid}",
        name_number(section.group(1), section.group(2)),
        now.number,
      )
    still_repealed(
      now.doc,
      f"section {sid}",
      old.get("status") == "repealed",
      now.status == "repealed",
    )
    items = {i.id: i for i in now.items}
    for item in old.get("items", []):
      iid = item.get("id")
      what = f"item {sid}.{iid}"
      if iid not in items:
        errors.add(
          now.doc.path,
          now.doc.line("[[items]]", "[content]"),
          7,
          f"{what} existed at base {ref} and is gone; repeal it instead",
        )
        continue
      renumbered(now.doc, what, item.get("number"), items[iid].number)
      still_repealed(
        now.doc,
        what,
        item.get("status") == "repealed",
        items[iid].status == "repealed",
      )
  if RETIRED in listing:
    for value in (git(root, "show", f"{ref}:{RETIRED}") or "").splitlines():
      if value and value not in retired:
        errors.add(
          RETIRED,
          1,
          7,
          f"{value} was retired at base {ref} and is no longer listed",
        )
  return ""


def revision_key(revision: str) -> tuple[int, int] | None:
  """Return the sort key of a revision.

  A draft `0.<n>` sorts by its number, before every approved integer
  revision.

  Args:
    revision: the revision as written.

  Returns:
    The key, or None for any other string.
  """
  if re.fullmatch(r"0\.[0-9]+", revision):
    return 0, int(revision[2:])
  if re.fullmatch(r"[1-9][0-9]*", revision):
    return int(revision), 0
  return None


def source_text_changed(root: Path, ref: str) -> bool:
  """Return whether the constitution text differs from the base's.

  The text is the article sources and `index.toml` other than its
  `revision` line. The files git tracks are compared as they are in the
  working tree, so the result does not depend on what is committed.

  Args:
    root: the checkout.
    ref: the base commit.

  Returns:
    True when the text changed.
  """
  drop = re.compile(r'^revision = ".*"\n', re.MULTILINE)
  listing = (
    git(root, "ls-tree", "-r", "--name-only", ref, "--", str(ARTICLES)) or ""
  )
  base_paths = {Path(p) for p in listing.split("\n") if p}
  tracked = git(root, "ls-files", "--", str(ARTICLES)) or ""
  paths = {Path(p) for p in tracked.split("\n") if p and (root / p).is_file()}
  if paths != base_paths:
    return True
  if any(
    (root / p).read_text(encoding="utf-8") != git(root, "show", f"{ref}:{p}")
    for p in paths
  ):
    return True
  base_index = git(root, "show", f"{ref}:{INDEX}") or ""
  return drop.sub("", (root / INDEX).read_text(encoding="utf-8")) != drop.sub(
    "", base_index
  )


def check_revision(root: Path, ref: str, src: Sources, errors: Errors) -> None:
  """Check 7: compare the revision with the base's.

  Within one edition the revision is greater than the base's when the
  constitution text changed since it, and equal to it when the text did
  not change. Two drafts count as one edition whatever edition they name,
  so renaming the edition a draft prepares does not restart its
  revisions. Not compared: a different status, or two active editions
  that differ.

  Args:
    root: the checkout.
    ref: the base commit.
    src: the sources.
    errors: where errors go.
  """
  base_index = git(root, "show", f"{ref}:{INDEX}")
  if base_index is None:
    return
  try:
    base = tomllib.loads(base_index)
  except tomllib.TOMLDecodeError:
    return  # check_base reports it
  revision, base_revision = (
    src.index.data["revision"],
    str(base.get("revision")),
  )
  status = src.index.data["status"]
  if status != base.get("status") or (
    status == "active" and src.index.data["edition"] != base.get("edition")
  ):
    return
  line = src.index.line("revision =")
  key, base_key = revision_key(revision), revision_key(base_revision)
  if key is None:
    errors.add(
      INDEX,
      line,
      7,
      f"revision {revision} is neither `0.<n>` nor a whole number from 1",
    )
  elif base_key is None:
    return
  elif source_text_changed(root, ref):
    if key <= base_key:
      errors.add(
        INDEX,
        line,
        7,
        f"the constitution text changed since base {ref}"
        f" but revision {revision} is not greater"
        f" than the base's {base_revision}",
      )
  elif revision != base_revision:
    errors.add(
      INDEX,
      line,
      7,
      f"revision {revision} differs from revision {base_revision}"
      f" at base {ref} but the constitution text did not change",
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
    note = check_base(args.root, args.base, src, errors)
    if note:
      print(note)
    check_revision(args.root, args.base, src, errors)
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
