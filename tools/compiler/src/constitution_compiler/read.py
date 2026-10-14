"""Read the constitution and rule sources into Sources, running check 1.

Reads index.toml and the article directories under articles/ in the
edition directory (model.Layout), rules/<id>.toml and rules/retired-ids.txt
(docs/design/source-formats.md, constitution.md, rules.md and
checkers.md). A rule file named by a number is a node of the rule tree in
the format of docs/design/rule-system.md (Rule file format); one named by a
word is in the older format of rules.md.
"""

from dataclasses import dataclass, field
import hashlib
from pathlib import Path

from constitution_compiler.languages import (
  LANGUAGES,
)
from constitution_compiler.model import (
  Article,
  ARTICLE_FILE,
  ARTICLE_NAME,
  ARTICLE_NAME_SHAPE,
  CHECKS,
  CONSTITUTION,
  Doc,
  Errors,
  INDEX_STATUS,
  Item,
  KINDS,
  Layout,
  LayoutError,
  LEVELS,
  MAINTAINERS,
  MONTH,
  name_number,
  Node,
  NODE_ID,
  NODE_NAME,
  NODE_STATUS,
  NUMBER,
  NUMBER_SHAPE,
  PARTS,
  RETIRED,
  Rule,
  RULES,
  Section,
  SECTION_NAME,
  SECTION_NAME_SHAPE,
  Sources,
  STATUS,
)
from constitution_compiler.schema import (
  ARTICLE_SCHEMA,
  check_keys,
  check_repealed,
  INDEX_SCHEMA,
  ITEM_SCHEMA,
  NODE_SCHEMAS,
  PARTS_SCHEMA,
  read_toml,
  RULE_SCHEMA,
  SECTION_SCHEMA,
  strip_texts,
)


def load_index(root: Path, layout: Layout, errors: Errors) -> Doc | None:
  """Read index.toml and check its format.

  Args:
    root: the checkout.
    layout: where the sources are.
    errors: where errors go.

  Returns:
    The index, or None when any error was found.
  """
  index = read_toml(root, layout.index, errors)
  if not index or not check_keys(index, index.data, INDEX_SCHEMA, None, errors):
    return None
  languages = index.data["languages"]
  if (
    not languages
    or not all(isinstance(lang, str) for lang in languages)
    or len(set(languages)) != len(languages)
  ):
    errors.add(
      layout.index,
      index.line("languages ="),
      1,
      "`languages` must list each language once",
    )
    return None
  for lang in languages:
    if lang not in LANGUAGES:
      errors.add(
        layout.index,
        index.line("languages ="),
        1,
        f"no rendering for language `{lang}`;"
        " add it to LANGUAGES in constitution_compiler/languages.py",
      )
  index_status = index.data["status"]
  if index_status not in INDEX_STATUS:
    errors.add(
      layout.index,
      index.line("status ="),
      1,
      f"`status` must be one of {INDEX_STATUS}",
    )
  edition = index.data.get("edition")
  if isinstance(edition, str) and not MONTH.fullmatch(edition):
    errors.add(
      layout.index,
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
    layout: where the sources are.
    errors: where errors go.
    languages: the languages of index.toml.
    index_status: the status of index.toml.
    articles: the articles read so far.
    sections: the sections read so far.
    rules: the rules of the older format read so far.
    nodes: the nodes of the rule tree read so far.
  """

  root: Path
  layout: Layout
  errors: Errors
  languages: list[str]
  index_status: str
  articles: list[Article] = field(default_factory=list)
  sections: list[Section] = field(default_factory=list)
  rules: list[Rule] = field(default_factory=list)
  nodes: list[Node] = field(default_factory=list)

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

  def status_of(
    self,
    doc: Doc,
    table: dict,
    what: str,
    start: int = 1,
    allowed: tuple[str, ...] = STATUS,
  ) -> str:
    """Check a status against the allowed ones and index.toml's.

    Args:
      doc: the file holding the table.
      table: the section, item or rule.
      what: what the table is, in messages.
      start: the table's first line.
      allowed: the statuses it may have.

    Returns:
      The status.
    """
    status = table["status"]
    line = doc.line("status =", start=start)
    if status not in allowed:
      self.errors.add(
        doc.path,
        line,
        1,
        f"{what} `status` must be one of {allowed}",
      )
    elif self.index_status == "draft" and status != "draft":
      self.errors.add(
        doc.path,
        line,
        1,
        f"{what} is {status} while index.toml is draft",
      )
    elif (
      self.index_status == "active"
      and allowed == STATUS
      and what != "a rule"
      and status == "draft"
    ):
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
        f"not an article directory; an article is {self.layout.articles}"
        "/<NN>-<id>/"
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

  def node(self, path: Path) -> None:
    """Read one node of the rule tree, `rules/<id>.toml`.

    Args:
      path: the file, relative to root.
    """
    if not NODE_ID.fullmatch(path.stem):
      self.errors.add(
        path, 1, 1, "a rule's file name is its ID, without leading zeros"
      )
      return
    doc = read_toml(self.root, path, self.errors)
    if not doc:
      return
    d = doc.data
    kind = d.get("kind")
    if kind not in KINDS:
      self.errors.add(
        path, doc.line("kind ="), 1, f"`kind` must be one of {KINDS}"
      )
      return
    schema = NODE_SCHEMAS[kind]
    if not check_keys(doc, d, schema, None, self.errors):
      return
    if kind == "rule" and not check_keys(
      doc,
      d["text"],
      PARTS_SCHEMA,
      None,
      self.errors,
      "text",
      doc.line("[text"),
    ):
      return
    before = len(self.errors)
    status = self.node_fields(doc, kind, path)
    texts = [("title", d["title"])]
    if kind == "rule":
      self.rule_fields(doc)
      texts += [(f"text.{p}", d["text"][p]) for p in PARTS]
    self.originals(doc, texts)
    if len(self.errors) > before:
      return
    original = hashlib.sha256(
      "\0".join(table[d["lang"]] for _, table in texts).encode()
    ).hexdigest()
    strip_texts(d, schema)
    if kind == "rule":
      strip_texts(d["text"], PARTS_SCHEMA)
    self.nodes.append(
      Node(
        doc,
        d["id"],
        d.get("slug"),
        kind,
        status,
        d["lang"],
        d.get("parent"),
        d.get("superseded-by"),
        d.get("authority", []),
        d["title"],
        {p: d["text"][p] for p in PARTS} if kind == "rule" else {},
        d.get("translations", {}),
        original,
      )
    )

  def node_fields(self, doc: Doc, kind: str, path: Path) -> str:
    """Check the fields every node has.

    Args:
      doc: the node's file.
      kind: its kind.
      path: the file, relative to root.

    Returns:
      Its status.
    """
    d = doc.data
    if d["id"] != int(path.stem):
      self.errors.add(
        path,
        doc.line("id ="),
        1,
        f"`id` must be {int(path.stem)}, as in the name `{path.name}`",
      )
    if d["revision"] < 1:
      self.errors.add(
        path, doc.line("revision ="), 1, "`revision` must be 1 or more"
      )
    # Only a rule is superseded, by another rule.
    allowed = tuple(
      s for s in NODE_STATUS if kind == "rule" or s != "superseded"
    )
    status = self.status_of(doc, d, f"a {kind}", allowed=allowed)
    if kind == "rule" and (status == "superseded") != ("superseded-by" in d):
      self.errors.add(
        path,
        doc.line("superseded-by =", "status ="),
        1,
        "`superseded-by` is there exactly when `status` is superseded",
      )
    return status

  def originals(self, doc: Doc, texts: list[tuple[str, dict]]) -> None:
    """Check that each text has its original and `translations` its shape.

    Args:
      doc: the node's file.
      texts: (field, text by language) for each text of the node.
    """
    lang = doc.data["lang"]
    for name, table in texts:
      if not table.get(lang, "").strip():
        self.errors.add(
          doc.path,
          doc.line(f"[{name}]"),
          1,
          f"`{name}` lacks its text in `{lang}`, the original's language",
        )
    translations = doc.data.get("translations", {})
    if lang in translations or not all(
      isinstance(v, str) for v in translations.values()
    ):
      self.errors.add(
        doc.path,
        doc.line("[translations]"),
        1,
        "`translations` maps each language but the original's to a hash",
      )

  def rule_fields(self, doc: Doc) -> None:
    """Check the fields only a rule node has.

    Args:
      doc: the rule's file.
    """
    d, path = doc.data, doc.path
    for key, allowed in (("level", LEVELS), ("check", CHECKS)):
      if d[key] not in allowed:
        self.errors.add(
          path, doc.line(f"{key} ="), 1, f"`{key}` must be one of {allowed}"
        )
    command = d["check"] == "command"
    for key in ("run", "tools"):
      if (key in d) != command:
        self.errors.add(
          path,
          doc.line(f"{key} =", "check ="),
          1,
          f"`{key}` is there exactly when `check` is command",
        )
    for key in ("requires", "conflicts"):
      if not all(
        isinstance(v, str) or (isinstance(v, int) and not isinstance(v, bool))
        for v in d.get(key, [])
      ):
        self.errors.add(
          path,
          doc.line(f"{key} ="),
          1,
          f"`{key}` must list IDs, a number or `<source>:<id>`",
        )
    if not all(isinstance(t, str) for t in d.get("tools", [])):
      self.errors.add(
        path, doc.line("tools ="), 1, "`tools` must be a list of names"
      )
    if not d["authority"] or not all(
      isinstance(a, str) for a in d["authority"]
    ):
      self.errors.add(
        path,
        doc.line("authority ="),
        1,
        "`authority` must be a non-empty list of provision IDs",
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
  try:
    layout = Layout.of(root)
  except LayoutError as e:
    errors.add(CONSTITUTION, 1, 1, str(e))
    return None
  index = load_index(root, layout, errors)
  if not index:
    return None
  languages = index.data["languages"]
  reader = _Reader(root, layout, errors, languages, index.data["status"])
  if (root / layout.old_sections).exists():
    errors.add(
      layout.old_sections,
      1,
      1,
      "sections belong in their article's directory,"
      f" {layout.articles}/<NN>-<id>/<NN>[a-z]-<id>.toml",
    )
  for directory in reader.entries(layout.articles):
    reader.article_directory(directory)
  rules = [p.relative_to(root) for p in (root / RULES).glob("*.toml")]
  for path in sorted(rules):
    if not NODE_NAME.fullmatch(path.stem):
      reader.rule(path)
  for path in sorted(
    (p for p in rules if NODE_NAME.fullmatch(p.stem)), key=lambda p: int(p.stem)
  ):
    reader.node(path)
  retired = read_retired(root, errors)
  if errors:
    return None
  return Sources(
    layout,
    index,
    languages,
    reader.articles,
    reader.sections,
    reader.rules,
    retired,
    reader.nodes,
  )
