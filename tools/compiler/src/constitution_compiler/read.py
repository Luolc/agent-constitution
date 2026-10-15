"""Read the constitution and rule sources into Sources, running check 1.

Reads index.toml and the article directories under articles/ in the
edition directory (model.Layout) and rules/<id>.toml, the nodes of the rule
tree (docs/design/source-formats.md, constitution.md, rule-format.md and
checkers.md).
"""

from dataclasses import dataclass, field
import hashlib
from pathlib import Path
from typing import Any

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
  FIRST_PARTY_REF,
  FOREIGN_REF,
  INDEX_STATUS,
  Item,
  KINDS,
  LANG_CODE,
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
  PARAMS_FILE,
  PARTS,
  REVISION_CLAUSE,
  REVISION_MAX,
  RuleFile,
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

  The site's `site_rules._OtherReader` subclasses it to read the rules of
  another source, overriding `parent`, `overrides` and `references`; a
  change to those methods, or to how `node` calls them, changes how the
  site reads another source.

  Attributes:
    root: the checkout.
    layout: where the sources are.
    errors: where errors go.
    languages: the languages of index.toml.
    index_status: the status of index.toml.
    articles: the articles read so far.
    sections: the sections read so far.
    nodes: the nodes of the rule tree read so far.
  """

  root: Path
  layout: Layout
  errors: Errors
  languages: list[str]
  index_status: str
  articles: list[Article] = field(default_factory=list)
  sections: list[Section] = field(default_factory=list)
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
      self.index_status == "active" and allowed == STATUS and status == "draft"
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

  def rule_file(self, path: Path) -> RuleFile | None:
    """Read what the constitution build needs of one node of the rule tree.

    Its status must agree with index.toml's, as a section's does.

    Args:
      path: the file, relative to root.

    Returns:
      The file, or None when it is not named by a number or not TOML.
    """
    if not NODE_ID.fullmatch(path.stem):
      return None
    doc = read_toml(self.root, path, self.errors)
    if not doc:
      return None
    d = doc.data
    kind = string_of(d, "kind") or "node"
    status = string_of(d, "status")
    if status is not None:
      self.status_of(doc, d, f"a {kind}", allowed=NODE_STATUS)
    authority = d.get("authority")
    tables = [
      ("title", d.get("title")),
      ("applicability", d.get("applicability")),
    ]
    text = d.get("text")
    if isinstance(text, dict):
      tables += [(f"text.{p}", text.get(p)) for p in PARTS]
    return RuleFile(
      doc,
      int(path.stem),
      string_of(d, "slug"),
      kind,
      status,
      [a for a in authority if isinstance(a, str)]
      if isinstance(authority, list)
      else [],
      [
        (name, {k: v.strip("\n") for k, v in t.items() if isinstance(v, str)})
        for name, t in tables
        if isinstance(t, dict)
      ],
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
    # statute validates the rule tree; keys it knows and the site does not
    # show are passed over.
    if not check_keys(doc, d, schema, None, self.errors, unknown=False):
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
    self.parent(doc, kind)
    texts = [("title", d["title"])]
    if "applicability" in d:
      texts.append(("applicability", d["applicability"]))
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
        d.get("overrides"),
        successors(d),
        d.get("authority", []),
        d["title"],
        d.get("applicability", {}),
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
    if not LANG_CODE.fullmatch(d["lang"]):
      self.errors.add(
        path,
        doc.line("lang ="),
        1,
        f'`lang = "{d["lang"]}"` is not a language code such as `en`',
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
      for key, text in table.items():
        if not LANG_CODE.fullmatch(key):
          self.errors.add(
            doc.path,
            doc.line(f"[{name}]"),
            1,
            f"`{name}` key `{key}` is not a language code",
          )
        elif key != lang and not text.strip():
          self.errors.add(
            doc.path, doc.line(f"[{name}]"), 1, f"`{name}` `{key}` is empty"
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
    spec = d["constitution"]
    clauses = [REVISION_CLAUSE.fullmatch(c) for c in spec.split(",")]
    if not all(clauses):
      self.errors.add(
        path,
        doc.line("constitution ="),
        1,
        f'`constitution = "{spec}"` is not a specifier of whole revisions:'
        " clauses such as `>=1` joined by commas, each with"
        " >=, <=, >, <, == or !=",
      )
    elif any(int(m[1]) > REVISION_MAX for m in clauses if m):
      self.errors.add(
        path,
        doc.line("constitution ="),
        1,
        f'`constitution = "{spec}"` names a revision above {REVISION_MAX},'
        " the largest there is",
      )
    self.references(doc)
    self.overrides(doc)
    self.superseded_by(doc)
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

  def parent(self, doc: Doc, kind: str) -> None:
    """Check the form of a node's `parent`.

    A domain's or a one-of's is the number of a node of this source; a
    rule's may also name a domain of another source, of which only the
    form is checked (docs/design/rule-sources.md, Subdomains). What the
    number names is statute's to check.

    Args:
      doc: the node's file.
      kind: its kind.
    """
    d, path = doc.data, doc.path
    if "parent" not in d:
      return
    v = d["parent"]
    if kind != "rule":
      if not is_number(v):
        target = "domain" if kind == "domain" else "node"
        self.errors.add(
          path,
          doc.line("parent ="),
          1,
          f"a {kind}'s `parent` `{toml_text(v)}` is not the number of a"
          f" {target} of this source",
        )
    elif first_party(v):
      self.errors.add(
        path,
        doc.line("parent ="),
        1,
        f"`parent` `{v}` names a node of this source; write {v[1:]}",
      )
    elif not (
      is_number(v) or (isinstance(v, str) and FOREIGN_REF.fullmatch(v))
    ):
      self.errors.add(
        path,
        doc.line("parent ="),
        1,
        f"`parent` `{toml_text(v)}` is not a reference: a number from 1 for"
        ' a node of this source, or "<source>:<id>" for a domain of another',
      )

  def overrides(self, doc: Doc) -> None:
    """Check a rule's `overrides`, which takes the place of its `parent`.

    It names one rule of another source; only its form is checked, since
    what it names is in that source (docs/design/rule-sources.md,
    Overrides).

    Args:
      doc: the rule's file.
    """
    d, path = doc.data, doc.path
    if ("parent" in d) == ("overrides" in d):
      self.errors.add(
        path,
        doc.line("overrides =", "parent =", "kind ="),
        1,
        "a rule has `parent` or `overrides`, not both: an overriding rule"
        " stands where the rule it overrides stands"
        if "parent" in d
        else "a rule needs `parent`, or `overrides` in its place",
      )
    if "overrides" not in d:
      return
    v = d["overrides"]
    if is_number(v) or first_party(v):
      problem = (
        f"names {toml_text(v)}, a rule of this source; within one source a"
        " rule is changed in place or superseded, never overridden"
      )
    elif isinstance(v, list):
      problem = (
        "names one rule, as a string; a rule that replaces two is two rules"
      )
    elif not (isinstance(v, str) and FOREIGN_REF.fullmatch(v)):
      problem = (
        f"`{toml_text(v)}` is not a reference to a rule of another source,"
        ' "<source>:<id>"'
      )
    else:
      return
    self.errors.add(path, doc.line("overrides ="), 1, f"`overrides` {problem}")

  def superseded_by(self, doc: Doc) -> None:
    """Check a rule's `superseded-by`: one rule ID or a list of them.

    That each names another rule is statute's to check.

    Args:
      doc: the rule's file.
    """
    d, path = doc.data, doc.path
    if "superseded-by" not in d:
      return
    ids = successors(d)
    problems = [] if ids else ["is empty; name the rule that replaces it"]
    seen = []
    for v in ids:
      # TOML's booleans are Python ints too.
      if not isinstance(v, int) or isinstance(v, bool) or v < 1:
        problems.append(
          f"entry `{toml_text(v)}` is not a rule ID, a number from 1"
        )
      elif v in seen:
        problems.append(f"lists {v} twice")
      else:
        seen.append(v)
    for problem in problems:
      self.errors.add(
        path, doc.line("superseded-by ="), 1, f"`superseded-by` {problem}"
      )

  def references(self, doc: Doc) -> None:
    """Check each entry of a rule's `requires` and `conflicts`.

    Args:
      doc: the rule's file.
    """
    d, path = doc.data, doc.path
    for key in ("requires", "conflicts"):
      seen = []
      for v in d.get(key, []):
        # TOML's booleans are Python ints too.
        if not (
          (isinstance(v, int) and not isinstance(v, bool) and v >= 1)
          or (isinstance(v, str) and FOREIGN_REF.fullmatch(v))
        ):
          problem = (
            f"entry `{toml_text(v)}` is not a reference: a number from 1"
            ' for a node of this source, or "<source>:<id>" for one of'
            " another"
          )
        elif v == d["id"]:
          problem = "names the rule itself"
        elif v in seen:
          problem = f"lists {v} twice"
        else:
          seen.append(v)
          continue
        self.errors.add(path, doc.line(f"{key} ="), 1, f"`{key}` {problem}")


def string_of(d: dict[str, Any], key: str) -> str | None:
  """Return a value of a parsed file when it is a string.

  Args:
    d: the file, as parsed.
    key: the key.

  Returns:
    The value, or None when it is missing or not a string.
  """
  value = d.get(key)
  return value if isinstance(value, str) else None


def is_number(value: object) -> bool:
  """Tell whether a value is a node's number, an integer from 1.

  Args:
    value: a value of a parsed TOML file.

  Returns:
    True when it is.
  """
  # TOML's booleans are Python ints too.
  return isinstance(value, int) and not isinstance(value, bool) and value >= 1


def first_party(value: object) -> bool:
  """Tell whether a value names a first-party rule, as `R12` does.

  The rules of this repo are the first-party ones, so it names a node of
  this source.

  Args:
    value: a value of a parsed TOML file.

  Returns:
    True when it does.
  """
  return isinstance(value, str) and FIRST_PARTY_REF.fullmatch(value) is not None


def successors(d: dict[str, Any]) -> list[Any]:
  """Return a node's `superseded-by` as a list, one ID or many.

  Args:
    d: the node's file, as parsed.

  Returns:
    The entries, as written and not yet checked; empty when there is no
    `superseded-by`.
  """
  value = d.get("superseded-by", [])
  return value if isinstance(value, list) else [value]


def toml_text(value: object) -> object:
  """Return a value as TOML writes it where Python's text differs.

  Args:
    value: a value of a parsed TOML file.

  Returns:
    `true` or `false` for a boolean, else the value itself.
  """
  if isinstance(value, bool):
    return "true" if value else "false"
  return value


def by_number(path: Path) -> tuple[int, str]:
  """Return the sort key of a rule file: its number, else its name.

  Args:
    path: the file.

  Returns:
    The key.
  """
  if NODE_NAME.fullmatch(path.stem):
    return (int(path.stem), "")
  return (0, path.stem)


def node_files(path: Path) -> list[Path]:
  """Return the files of a source's rule tree, by number.

  They are the TOML files of its rules directory but params.toml.

  Args:
    path: the source's directory, holding rules/.

  Returns:
    The files, relative to path.
  """
  return sorted(
    (
      p.relative_to(path)
      for p in (path / RULES).glob("*.toml")
      if p.name != PARAMS_FILE
    ),
    key=by_number,
  )


def load(root: Path, errors: Errors, *, nodes: bool = False) -> Sources | None:
  """Check 1: read every source and check its format.

  The constitution is read whole. Of the rule tree, which statute
  validates, the constitution build reads only what `RuleFile` holds; the
  site reads its nodes.

  Args:
    root: the checkout.
    errors: where errors go.
    nodes: whether to read the nodes of the rule tree, as the site does,
      in place of what the constitution build reads of them.

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
  rules = []
  for path in node_files(root):
    if nodes:
      reader.node(path)
    elif found := reader.rule_file(path):
      rules.append(found)
  if errors:
    return None
  return Sources(
    layout,
    index,
    languages,
    reader.articles,
    reader.sections,
    reader.nodes,
    rules,
  )
