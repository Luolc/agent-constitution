"""Write the rules of rules/ as data for the static site.

Writes web/data/rules.json, which Hugo makes into the pages under rules/
and the lists under the provisions of the constitution
(docs/design/static-site.md, Rules). For each language of index.toml it
holds every node of the rule tree in the order of the parent tree, each
with its texts in that language where it has a translation that follows
its original, else in the original, the domains above it, which head
it (a one-of above it is a box, not a heading); the citations in the
texts as Markdown links, relative to the rule's page; the other side of
each conflict; and, for each provision of the constitution, the rules
whose `authority` names it, and for each article, the rules that name
it or a provision under it.
An `authority` that names a provision and one above it counts only the
narrower one. A superseded or repealed rule is not listed under the
provisions.

With `--source`, the rules of other sources join them, each source with
its name and quadrant: their nodes follow this repo's, source by source
in the order of the quadrants, each node with its source and quadrant,
and a node of another source cited, required or named as a parent links
to its page; a conflict across sources shows on both sides. Each node
also names its nearest domain, of another source when its parent is.
With `--enabled`, each rule says whether the rule set of a statute.lock
enables it. Without either, the pages are this repo's alone
and say neither.

Each node says whether its source's defaults enable it, as statute
reckons it; a domain or a rule gives its own `default`, a one-of its
default option, an option whether it is that one, and a rule its `uses`
(docs/design/rule-config.md, Default switches and Where a rule is used).

Each source with parameters has a page of them, each parameter with its
title, name, type, scope, default (and values) and the rules that use
it. In a rule's texts a placeholder, `{$<name>}`, becomes the default,
as statute writes it in the text's language, marked as a value: a link
to its entry on that page, the name on hover; in a title, where no link
can go, the value alone (docs/design/rule-config.md, Parameters).

statute reads and checks the sources (`statute export`, built from this
checkout with cargo) and prints what is wrong with them; the exit status
is 1 when they cannot be read.
"""

import argparse
from collections import defaultdict
from dataclasses import dataclass
import html
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib
from typing import Any, NamedTuple

ROOT = Path(__file__).resolve().parents[4]
# statute, built from this checkout, as cargo runs it.
STATUTE = [
  "cargo",
  "run",
  "--quiet",
  "--locked",
  "--manifest-path",
  str(ROOT / "Cargo.toml"),
  "--package",
  "statute",
  "--",
]
OUT = ROOT / "web/data/rules.json"
# A rule's page is rules/<id>/, the full version of the constitution the
# home page of the same language: from a rule's page, these links lead to
# a provision and to another rule's page.
PROVISION_LINK = "../../#{ref}"
RULE_LINK = "../{id}/"
# The text of a link to a rule, `{@rule:7}`, in the language of the text.
RULE_LABEL = {"en": "rule {id}", "zh": "规定 {id}"}
# Each node keeps these fields of its file as they are, when it has them.
FIELDS = ("revision", "level", "constitution", "check", "run", "tools")
# The name of this repo's source, as another source names one of its
# rules (`R12`), and its quadrant; the quadrants in the order the pages
# list the sources (docs/design/rule-sources.md).
FIRST_PARTY = "R"
QUADRANTS = ("global-public", "global-private", "local-public", "local-private")
# The parts of a rule's text.
PARTS = ("statement", "reason", "verification")
# A rule's `uses` when its file leaves it out.
USES = ["instructions"]
# The name of a source, or a part of a provision's ID.
ID = r"[a-z0-9]+(?:-[a-z0-9]+)*"
# A citation: `{@id}`, `{@section.item}`, each optionally after a namespace.
MARK = re.compile(
  r"\{@(?:(constitution|rule):)?(" + ID + r"(?:\." + ID + r")?)\}"
)
# A citation of a rule of another source, as `requires` names it:
# `{@rule:R12}`, `{@rule:team:7}`.
OTHER_RULE = re.compile(r"\{@rule:(R[1-9][0-9]*|" + ID + r":[1-9][0-9]*)\}")
FIRST_PARTY_REF = re.compile(r"R[1-9][0-9]*")
# A placeholder, `{$language.docs}`, and a Markdown code span, in which
# one is no Markdown but stays literal.
PLACEHOLDER = re.compile(r"\{\$(" + ID + r"\." + ID + r")\}")
CODE_SPAN = re.compile(r"(`+)(.+?)\1", re.S)


@dataclass(frozen=True)
class Node:
  """One node of a rule tree, as `statute export` gives it.

  Attributes:
    id: its ID.
    slug: its slug, if it has one.
    kind: rule, one-of or domain.
    status: its status.
    lang: the language of its original.
    parent: its parent, an ID or a reference to another source's node.
    overrides: the rule of another source it takes the place of.
    superseded_by: the rules that supersede it.
    authority: the provisions it names.
    title: its title by language.
    applicability: a domain's applicability by language, if it has one.
    text: a rule's parts, each by language; empty for a domain or one-of.
    translations: the languages it is translated into, each following its
      current original, as statute checks.
    on_by_default: whether its source's defaults enable it, None when
      that depends on another source.
    data: the fields of its file.
  """

  id: int
  slug: str | None
  kind: str
  status: str
  lang: str
  parent: int | str | None
  overrides: str | None
  superseded_by: list[int]
  authority: list[str]
  title: dict[str, str]
  applicability: dict[str, str] | None
  text: dict[str, dict[str, str]]
  translations: list[str]
  on_by_default: bool | None
  data: dict[str, Any]


def texts(table: dict[str, str]) -> dict[str, str]:
  """Return a text by language without the newlines that open and close it.

  That is, the newline after the opening and before the closing quotes of
  a multi-line string.

  Args:
    table: the text by language, as the file writes it.

  Returns:
    The text by language.
  """
  return {lang: text.strip("\n") for lang, text in table.items()}


def node(entry: dict[str, Any]) -> Node:
  """Return a node of the rule tree as `statute export` gives it.

  Args:
    entry: the node: the fields of its file and whether its source's
      defaults enable it.

  Returns:
    The node.
  """
  fields = entry["fields"]
  successors = fields.get("superseded-by", [])
  applicability = fields.get("applicability")
  return Node(
    id=fields["id"],
    slug=fields.get("slug"),
    kind=fields["kind"],
    status=fields["status"],
    lang=fields["lang"],
    parent=fields.get("parent"),
    overrides=fields.get("overrides"),
    superseded_by=successors if isinstance(successors, list) else [successors],
    authority=fields.get("authority", []),
    title=texts(fields["title"]),
    applicability=None if applicability is None else texts(applicability),
    text={part: texts(t) for part, t in fields.get("text", {}).items()},
    translations=sorted(fields.get("translations", {})),
    on_by_default=entry["on-by-default"],
    data=fields,
  )


class Constitution(NamedTuple):
  """What the pages take from the constitution.

  Attributes:
    languages: the languages of index.toml, in order.
    parents: the IDs of the provisions that have one under them.
    ancestors: for each section, its article; for each item, its section
      and its article; nearest first.
    labels: for each language, the label of each provision by citation:
      `Art. IV §1(2)` for `principals.competent`.
  """

  languages: list[str]
  parents: set[str]
  ancestors: dict[str, tuple[str, ...]]
  labels: dict[str, dict[str, str]]


def constitution(data: dict[str, Any]) -> Constitution:
  """Return the constitution `statute export` gives.

  Args:
    data: its `constitution`.

  Returns:
    The constitution.
  """
  provisions = data["provisions"]
  parent = {p["id"]: p["parent"] for p in provisions}
  ancestors: dict[str, tuple[str, ...]] = {}
  for p in provisions:
    above, ref = [], p["parent"]
    while ref is not None:
      above.append(ref)
      ref = parent[ref]
    if above:
      ancestors[p["id"]] = tuple(above)
  return Constitution(
    languages=data["languages"],
    parents={p["parent"] for p in provisions if p["parent"] is not None},
    ancestors=ancestors,
    labels={
      lang: {p["id"]: p["labels"][lang] for p in provisions}
      for lang in data["languages"]
    },
  )


def export(path: Path) -> dict[str, Any] | None:
  """Return the sources of a directory, as `statute export` prints them.

  Args:
    path: the directory, holding constitution/ or rules/.

  Returns:
    Its constitution and its nodes, or None when cargo cannot run or
    statute cannot read or check them; either has then printed why.
  """
  try:
    run = subprocess.run(
      [*STATUTE, "export", str(path)],
      stdout=subprocess.PIPE,
      text=True,
      encoding="utf-8",
      check=False,
    )
  except OSError as e:
    print(f"site-rules: error: cannot run cargo: {e}", file=sys.stderr)
    return None
  return json.loads(run.stdout) if run.returncode == 0 else None


def first_party(value: object) -> bool:
  """Tell whether a value names a first-party rule, as `R12` does.

  Args:
    value: a value of a node's file.

  Returns:
    True when it does.
  """
  return isinstance(value, str) and FIRST_PARTY_REF.fullmatch(value) is not None


@dataclass(frozen=True)
class Param:
  """One parameter of a source, as `statute export` gives it.

  Attributes:
    name: its name, `language.docs`.
    type: language, string or choice.
    values: a choice's values; None for the other types.
    scope: repo or personal.
    default: its default, as the rule set writes a value.
    title: its title by language.
    shown: its default as statute writes it in each language.
  """

  name: str
  type: str
  values: list[str] | None
  scope: str
  default: str
  title: dict[str, str]
  shown: dict[str, str]


class Source(NamedTuple):
  """The rules of one source.

  Attributes:
    name: its name, as the rules of the other sources name it.
    quadrant: one of QUADRANTS.
    nodes: its rule tree.
    params: the parameters its params.toml declares.
  """

  name: str
  quadrant: str
  nodes: list[Node]
  params: tuple[Param, ...] = ()


def params_key(source: str = FIRST_PARTY) -> str:
  """Return the path of a source's page of parameters under rules/.

  Args:
    source: the name of the source.

  Returns:
    `params` for this repo's, `team-params` for another source's.
  """
  return "params" if source == FIRST_PARTY else f"{source}-params"


def rule_label(node_id: int, source: str = FIRST_PARTY) -> str:
  """Return the label the site gives a node of the rule tree.

  Args:
    node_id: the node's ID.
    source: the name of its source.

  Returns:
    The label: `R12` for this repo's, `team:12` for another source's.
  """
  return f"R{node_id}" if source == FIRST_PARTY else f"{source}:{node_id}"


def page_key(node_id: int, source: str = FIRST_PARTY) -> str:
  """Return the path of a node's page under rules/, which also names it.

  Args:
    node_id: the node's ID.
    source: the name of its source.

  Returns:
    `12` for this repo's node 12, `team-12` for another source's.
  """
  return str(node_id) if source == FIRST_PARTY else f"{source}-{node_id}"


def other_ref(ref: str) -> tuple[str, int]:
  """Return the source and ID a reference to a node of another source names.

  Args:
    ref: `R12` or `team:12`.

  Returns:
    The source's name and the node's ID.
  """
  if first_party(ref):
    return FIRST_PARTY, int(ref[1:])
  name, node_id = ref.split(":")
  return name, int(node_id)


def specific(authority: list[str], src: Constitution) -> list[str]:
  """Return an `authority` without the provisions it names something under.

  Args:
    authority: the provisions, as the node names them.
    src: the constitution.

  Returns:
    The provisions no other of them is under, in their order.
  """
  above = {a for ref in authority for a in src.ancestors.get(ref, ())}
  return [ref for ref in authority if ref not in above]


def listed(node: Node) -> bool:
  """Tell whether the node is listed under the provisions it cites.

  A rule in force is, neither superseded nor repealed; the list of the
  rules shows every rule, and its filter chooses the statuses.

  Args:
    node: the node.

  Returns:
    True when it is listed under them.
  """
  return node.kind == "rule" and node.status not in ("superseded", "repealed")


def sibling_key(node: Node, parent_kind: str | None) -> tuple[bool, int]:
  """Return the key that orders a node among the children of its parent.

  By ID, except that a domain's subdomains come after its rules and
  one-ofs, as statute orders them (`docs/design/statute-command.md`,
  Compilation and the site).

  Args:
    node: the node.
    parent_kind: the kind of its parent, or None when it has none here.

  Returns:
    Whether it is a subdomain, then its ID.
  """
  return (node.kind == "domain" and parent_kind == "domain", node.id)


def tree_order(nodes: list[Node]) -> list[tuple[Node, int]]:
  """Return the nodes along the parent tree, each with its depth.

  Each node comes before its children, which follow in the order of
  `sibling_key`; a node with no parent in this source starts at depth 0.

  Args:
    nodes: the nodes, a valid tree.

  Returns:
    Each node and its depth, 0 for a domain that is not a subdomain.
  """
  kinds = {n.id: n.kind for n in nodes}
  children: dict[int | None, list[Node]] = defaultdict(list)
  for n in nodes:
    # A node under a domain of another source, or in place of a rule of
    # another source, heads a tree of its own here.
    children[n.parent if isinstance(n.parent, int) else None].append(n)
  for siblings in children.values():
    siblings.sort(
      key=lambda n: sibling_key(
        n, kinds.get(n.parent) if isinstance(n.parent, int) else None
      )
    )
  order: list[tuple[Node, int]] = []

  def visit(parent: int | None, depth: int) -> None:
    for n in children[parent]:
      order.append((n, depth))
      visit(n.id, depth + 1)

  visit(None, 0)
  return order


class _Language:
  """The data of one language of the site."""

  def __init__(
    self, lang: str, src: Constitution, sources: list[Source]
  ) -> None:
    """Index the nodes and the constitution's labels.

    Args:
      lang: the language of the pages.
      src: the constitution.
      sources: the rules of each source, this repo's first.
    """
    self.lang, self.src = lang, src
    self.nodes = {(s.name, n.id): n for s in sources for n in s.nodes}
    self.params = {(s.name, p.name): p for s in sources for p in s.params}
    self.many = len(sources) > 1
    self.labels = src.labels

  def shown(self, node: Node) -> str:
    """Return the language the node is shown in on this language's pages.

    The page's own when the node is written in it or translated into it;
    its original's otherwise.

    Args:
      node: the node.

    Returns:
      The language code.
    """
    translated = self.lang in node.translations
    return self.lang if node.lang == self.lang or translated else node.lang

  def title(self, key: tuple[str, int]) -> str:
    """Return a node's title as this language's pages show it.

    Each placeholder is its parameter's default.

    Args:
      key: its source's name and its ID.

    Returns:
      The title.
    """
    node = self.nodes[key]
    shown = self.shown(node)
    return self.values(node.title[shown], shown, key[0], mark="none")

  def title_html(self, key: tuple[str, int]) -> str:
    """Return a node's title as HTML, for a template that does not render it.

    The text is escaped, each code span is code and each placeholder
    its parameter's default, marked as a span.

    Args:
      key: its source's name and its ID.

    Returns:
      The title.
    """
    node = self.nodes[key]
    shown = self.shown(node)
    # Escaping leaves the backticks and the placeholders as they are.
    text = CODE_SPAN.sub(
      lambda m: f"<code>{m.group(2)}</code>",
      html.escape(node.title[shown], quote=False),
    )
    return self.values(text, shown, key[0], "span")

  def param(self, name: str, source: str) -> tuple[str, Param] | None:
    """Return the parameter a placeholder of a source's rule names.

    It is the source's own, or else the first-party source's.

    Args:
      name: the parameter's name.
      source: the name of the source of the rule.

    Returns:
      The name of the parameter's source and the parameter, or None when
      neither declares it.
    """
    for where in (source, FIRST_PARTY):
      if (where, name) in self.params:
        return where, self.params[(where, name)]
    return None

  def values(
    self, text: str, text_lang: str, source: str, mark: str = "link"
  ) -> str:
    """Return a text with each placeholder as its parameter's default.

    With `mark` "link", a default is a link to the parameter's entry on
    its page of parameters, with the name as its title; with "span", for
    a text that stands inside a link, a span with the name as its title;
    either escaped as HTML, and in a code span as well, which becomes
    HTML for it. With "none", the default alone, as it is, for a text
    that the template escapes.

    Args:
      text: the text, its citations already links.
      text_lang: its language.
      source: the name of the source of the node that holds it.
      mark: "link", "span" or "none".

    Returns:
      The text; a placeholder of no parameter stays as it is.
    """

    def value(m: re.Match[str]) -> str:
      found = self.param(m.group(1), source)
      if found is None:
        return m.group(0)
      where, p = found
      shown = p.shown.get(text_lang, p.default)
      if mark == "none":
        return shown
      if mark == "span":
        return (
          f'<span class="param-value" title="{p.name}">'
          f"{html.escape(shown)}</span>"
        )
      return (
        f'<a class="param-value" href="../{params_key(where)}/#{p.name}"'
        f' title="{p.name}">{html.escape(shown)}</a>'
      )

    def code(m: re.Match[str]) -> str:
      if not PLACEHOLDER.search(m.group(2)) or mark == "none":
        return m.group(0)
      # Escaping leaves a placeholder as it is.
      inner = html.escape(m.group(2), quote=False)
      return f"<code>{PLACEHOLDER.sub(value, inner)}</code>"

    return PLACEHOLDER.sub(value, CODE_SPAN.sub(code, text))

  def ref(
    self, target: int | str, source: str = FIRST_PARTY
  ) -> dict[str, object]:
    """Return a node named by `requires`, `conflicts` or a field like them.

    Args:
      target: its ID, or a reference to a node of another source.
      source: the source of the node that names it.

    Returns:
      Its ID, label, title, title as HTML and page, or the reference
      alone when it names a node of a source the site does not have.
    """
    key = (source, target) if isinstance(target, int) else other_ref(target)
    if key not in self.nodes:
      return {"ref": target}
    return {
      "id": key[1],
      "label": rule_label(key[1], key[0]),
      "title": self.title(key),
      "titleHtml": self.title_html(key),
      "key": page_key(key[1], key[0]),
    }

  def links(self, text: str, text_lang: str, source: str = FIRST_PARTY) -> str:
    """Return a text with each citation as a Markdown link.

    A provision's label is in the language of the text, or of the page
    when the constitution has no text in it. A rule of this repo cited in
    its own text is named by its number, any other by its label.

    Args:
      text: the text, as the sources hold it.
      text_lang: its language.
      source: the source of the node that holds the text.

    Returns:
      The Markdown text.
    """
    labels = self.labels.get(text_lang) or self.labels[self.lang]
    rule = RULE_LABEL.get(text_lang) or RULE_LABEL[self.lang]

    def rule_link(target: str) -> str:
      ref = self.ref(int(target) if target.isdigit() else target, source)
      if "key" not in ref:
        return rule.format(id=target)
      name = target if source == FIRST_PARTY else ref["label"]
      return f"[{rule.format(id=name)}]({RULE_LINK.format(id=ref['key'])})"

    def link(m: re.Match[str]) -> str:
      namespace, target = m.group(1), m.group(2)
      if namespace == "rule":
        return rule_link(target)
      label = labels.get(target, target)
      return f"[{label}]({PROVISION_LINK.format(ref=target)})"

    return OTHER_RULE.sub(lambda m: rule_link(m.group(1)), MARK.sub(link, text))

  def params_page(
    self, source: Source, used: dict[tuple[str, str], list[tuple[str, int]]]
  ) -> dict[str, object]:
    """Return what a source's page of parameters shows.

    Args:
      source: the source.
      used: for each parameter, by its source's name and its name, the
        rules whose texts name it, by their source's name and ID.

    Returns:
      The page's key, the source's name and quadrant, and each parameter
      with its title in this language where it has one, else in the
      other, its default as written and as shown, and the rules that use
      it.
    """
    params = []
    for p in source.params:
      lang = self.lang if self.lang in p.title else next(iter(p.title))
      params.append(
        {
          "name": p.name,
          "type": p.type,
          "values": p.values,
          "scope": p.scope,
          "default": p.default,
          "shown": p.shown.get(self.lang, p.default),
          "title": p.title[lang],
          "lang": lang,
          "rules": [
            self.ref(i, name) for name, i in used[(source.name, p.name)]
          ],
        }
      )
    return {
      "key": params_key(source.name),
      "source": source.name,
      "quadrant": source.quadrant,
      "params": params,
    }

  def switches(self, node: Node, source: str) -> dict[str, object]:
    """Return a node's `default` and, for a rule, its `uses`.

    Args:
      node: the node.
      source: the name of its source.

    Returns:
      A domain's or a rule's `default`, when it has one, or a one-of's
      default option (`defaultOption`), its options in order of ID
      (`options`) and the values of `uses` its options that are rules
      have, both when none is (`optionUses`); for an option of a one-of,
      whether it is the default (`optionDefault`); and a rule's `uses`.
    """
    data: dict[str, object] = {}
    default = node.data.get("default")
    if node.kind == "one-of":
      if default is not None:
        data["defaultOption"] = self.ref(default, source)
      options = [
        o
        for (name, _), o in sorted(self.nodes.items(), key=lambda kv: kv[0][1])
        if name == source and o.parent == node.id
      ]
      data["options"] = [self.ref(o.id, source) for o in options]
      uses = {
        u for o in options if o.kind == "rule" for u in o.data.get("uses", USES)
      }
      data["optionUses"] = sorted(uses or {"instructions", "review"})
    elif default is not None:
      data["default"] = default
    if isinstance(node.parent, int):
      parent = self.nodes[(source, node.parent)]
      if parent.kind == "one-of":
        data["optionDefault"] = parent.data.get("default") == node.id
    if node.kind == "rule":
      data["uses"] = node.data.get("uses", USES)
    return data

  def node(
    self,
    node: Node,
    source: Source,
    depth: int,
    conflicts: list[int | str],
    enabled: set[str] | None,
  ) -> dict[str, object]:
    """Return what the pages show of one node.

    Args:
      node: the node.
      source: its source.
      depth: its depth in the tree, 0 for a domain; the pages count no
        one-of above it.
      conflicts: the rules it conflicts with, on either side.
      enabled: the labels of the rules the rule set enables, or None.

    Returns:
      The node's data.
    """
    data = node.data
    name = source.name
    shown = self.shown(node)
    # The domains above it head it; a one-of is a box in its domain's
    # list, not a heading, and counts for no level.
    groups, ancestor, boxes = [], node.parent, 0
    while isinstance(ancestor, int):
      parent = self.nodes[(name, ancestor)]
      if parent.kind == "domain":
        groups.insert(0, self.ref(parent.id, name) | {"kind": parent.kind})
      elif parent.kind == "one-of":
        boxes += 1
      ancestor = parent.parent
    # The nearest domain, which the filter goes by, may be another
    # source's: a node under it heads a tree of its own here, with no
    # group, but is shown with that domain.
    domain = next(
      (g["key"] for g in reversed(groups) if g["kind"] == "domain"), ""
    )
    if isinstance(ancestor, str):
      above = self.ref(ancestor, name)
      if "key" in above and self.nodes[other_ref(ancestor)].kind == "domain":
        domain = above["key"]
    if self.many:
      groups.insert(
        0,
        {"key": f"source-{name}", "kind": "source", "source": name}
        | {"quadrant": source.quadrant},
      )
    label = rule_label(node.id, name)
    extra: dict[str, object] = {}
    if enabled is not None and node.kind == "rule":
      extra["enabled"] = label in enabled
    extra |= self.switches(node, name)
    return (
      {
        "id": node.id,
        "label": label,
        "slug": node.slug,
        "kind": node.kind,
        "status": node.status,
        "lang": node.lang,
        "shown": shown,
        "translations": node.translations,
        "depth": depth - boxes,
        "groups": groups,
        "title": self.title((name, node.id)),
        "titleMarked": self.values(node.title[shown], shown, name),
        "titleInLink": self.values(node.title[shown], shown, name, "span"),
        "applicability": self.links(node.applicability[shown], shown, name)
        if node.applicability
        else None,
        "text": {
          part: self.values(
            self.links(node.text[part][shown], shown, name), shown, name
          )
          for part in PARTS
          if node.text
        },
        "parent": self.ref(node.parent, name) if node.parent else None,
        "overrides": self.ref(node.overrides, name) if node.overrides else None,
        "supersededBy": [self.ref(t, name) for t in node.superseded_by],
        "children": [
          self.ref(n.id, name)
          for n in sorted(
            (n for n in source.nodes if n.parent == node.id),
            key=lambda n: sibling_key(n, node.kind),
          )
        ],
        "requires": [self.ref(t, name) for t in data.get("requires", [])],
        "conflicts": [self.ref(t, name) for t in conflicts],
        "authority": [
          {"ref": ref, "label": self.labels[self.lang].get(ref, ref)}
          for ref in specific(node.authority, self.src)
        ],
        "key": page_key(node.id, name),
        "domain": domain,
        "source": name,
        "quadrant": source.quadrant,
        "onByDefault": node.on_by_default,
        # A rule that is an option of a one-of is listed in the one-of's
        # box, not on its own.
        "boxed": node.kind == "rule" and "optionDefault" in extra,
      }
      | {field: data[field] for field in FIELDS if field in data}
      | extra
    )


def both_sides(sources: list[Source]) -> dict[tuple[str, int], list[int | str]]:
  """Return each rule's conflicts: a conflict is written on one side.

  Args:
    sources: the rules of each source.

  Returns:
    For each rule, by its source's name and its ID, the rules it
    conflicts with, as either side writes them.
  """
  conflicts: dict[tuple[str, int], list[int | str]] = defaultdict(list)
  names = {s.name for s in sources}
  for s in sources:
    for n in s.nodes:
      for target in n.data.get("conflicts", []):
        conflicts[(s.name, n.id)].append(target)
        if isinstance(target, int):
          conflicts[(s.name, target)].append(n.id)
        elif (other := other_ref(target))[0] in names:
          # On the other side, a reference to this source, as its label.
          conflicts[other].append(rule_label(n.id, s.name))
  return conflicts


def site_data(
  src: Constitution,
  nodes: list[Node],
  others: list[Source] | None = None,
  enabled: set[str] | None = None,
  params: tuple[Param, ...] = (),
) -> dict[str, Any]:
  """Return the data of every language of the site.

  Args:
    src: the constitution.
    nodes: this repo's rule tree.
    others: the rules of other sources, if any.
    enabled: the labels of the rules a rule set enables, or None.
    params: this repo's parameters.

  Returns:
    For each language, its sources, each with its name and quadrant
    (`sources`); its nodes in tree order, source by source; the page of
    parameters of each source that has some (`params`); for each
    provision a listed rule names in `authority`, those rules
    (`citedBy`); and for each provision with one under it that it or
    one under it names, those rules (`citedWithin`); each list in order
    of source and ID, without repeats. With other sources, each node's
    groups start with its source.
  """
  sources = [
    Source(FIRST_PARTY, QUADRANTS[0], nodes, params),
    *(others or []),
  ]
  sources.sort(key=lambda s: QUADRANTS.index(s.quadrant))
  rank = {s.name: i for i, s in enumerate(sources)}
  order = [(s, n, depth) for s in sources for n, depth in tree_order(s.nodes)]
  conflicts = both_sides(sources)
  cited: dict[str, list[tuple[str, int]]] = defaultdict(list)
  within: dict[str, list[tuple[str, int]]] = defaultdict(list)
  for s, n, _ in order:
    if listed(n):
      refs = specific(n.authority, src)
      for ref in refs:
        cited[ref].append((s.name, n.id))
      for parent in sorted(
        {r for ref in refs for r in (ref, *src.ancestors.get(ref, ()))}
        & src.parents
      ):
        within[parent].append((s.name, n.id))

  def ordered(keys: list[tuple[str, int]]) -> list[tuple[str, int]]:
    return sorted(keys, key=lambda k: (rank[k[0]], k[1]))

  data: dict[str, Any] = {}
  for lang in src.languages:
    language = _Language(lang, src, sources)
    # A translation names the parameters its original names, so the
    # original tells which rules use which.
    used: dict[tuple[str, str], list[tuple[str, int]]] = defaultdict(list)
    for s, n, _ in order:
      texts = [n.title[n.lang], *(t[n.lang] for t in n.text.values())]
      for name in dict.fromkeys(
        m.group(1) for t in texts for m in PLACEHOLDER.finditer(t)
      ):
        if (found := language.param(name, s.name)) is not None:
          used[(found[0], name)].append((s.name, n.id))
    data[lang] = {
      "sources": [{"name": s.name, "quadrant": s.quadrant} for s in sources],
      "nodes": [
        language.node(
          n, s, depth, sorted(conflicts[(s.name, n.id)], key=str), enabled
        )
        for s, n, depth in order
      ],
      "citedBy": {
        ref: [language.ref(i, name) for name, i in ordered(keys)]
        for ref, keys in cited.items()
      },
      "citedWithin": {
        ref: [language.ref(i, name) for name, i in ordered(keys)]
        for ref, keys in within.items()
      },
      "params": [language.params_page(s, used) for s in sources if s.params],
    }
  return data


def load(
  path: Path,
) -> tuple[Constitution | None, list[Node], tuple[Param, ...]] | None:
  """Read the constitution and the rules of a directory through statute.

  Args:
    path: the directory.

  Returns:
    Its constitution, None when it has none, its rule tree and its
    parameters; or None when statute cannot read or check them.
  """
  data = export(path)
  if data is None:
    return None
  src = data["constitution"]
  return (
    None if src is None else constitution(src),
    [node(n) for n in data["nodes"]],
    tuple(Param(**p) for p in data["params"]),
  )


def enabled_rules(locks: list[Path]) -> set[str]:
  """Return the rules that statute.lock files enable.

  Args:
    locks: the files.

  Returns:
    The rules, as the files name them: `R12`, `team:7`.
  """
  return {
    entry["rule"]
    for lock in locks
    for entry in tomllib.loads(lock.read_text(encoding="utf-8")).get(
      "rules", []
    )
  }


def source_arg(text: str) -> tuple[str, str, Path]:
  """Parse `--source <name>:<quadrant>:<directory>`.

  Args:
    text: the argument.

  Returns:
    The name, the quadrant and the directory.

  Raises:
    argparse.ArgumentTypeError: when it is not of that form.
  """
  name, _, rest = text.partition(":")
  quadrant, _, path = rest.partition(":")
  if not (re.fullmatch(ID, name) and quadrant in QUADRANTS[1:] and path):
    raise argparse.ArgumentTypeError(
      f"`{text}` is not <name>:<quadrant>:<directory>, the name lowercase"
      f" words, the quadrant one of {', '.join(QUADRANTS[1:])}"
    )
  return name, quadrant, Path(path)


def main(argv: list[str] | None = None) -> int:
  """Read the sources and write the JSON file.

  Args:
    argv: The command-line arguments, or None for sys.argv.

  Returns:
    The exit status: 0 on success, 1 when the sources cannot be read.
  """
  parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
  parser.add_argument(
    "--root",
    type=Path,
    default=ROOT,
    help="the checkout to read (default: the one this script is in)",
  )
  parser.add_argument(
    "--out",
    type=Path,
    default=OUT,
    help="the JSON file (default: web/data/rules.json)",
  )
  parser.add_argument(
    "--source",
    type=source_arg,
    action="append",
    default=[],
    metavar="NAME:QUADRANT:DIR",
    help="another source's rules, in DIR/rules/ (repeatable)",
  )
  parser.add_argument(
    "--enabled",
    type=Path,
    action="append",
    metavar="LOCK",
    help="a statute.lock whose rules the pages mark as enabled (repeatable)",
  )
  args = parser.parse_args(argv)
  read = load(args.root)
  if read is None:
    return 1
  src, nodes, params = read
  if src is None:
    print(f"{args.root}: error: no constitution/", file=sys.stderr)
    return 1
  others = []
  for name, quadrant, path in args.source:
    other = load(path)
    if other is None:
      return 1
    others.append(Source(name, quadrant, other[1], other[2]))
  enabled = enabled_rules(args.enabled) if args.enabled else None
  data = site_data(src, nodes, others, enabled, params)
  args.out.parent.mkdir(parents=True, exist_ok=True)
  args.out.write_text(
    json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
  )
  return 0


if __name__ == "__main__":
  sys.exit(main())
