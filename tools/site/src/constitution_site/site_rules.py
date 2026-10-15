"""Write the rules of rules/ as data for the static site.

Writes web/data/rules.json, which Hugo makes into the pages under rules/
and the lists under the provisions of the constitution
(docs/design/static-site.md, Rules). For each language of index.toml it
holds every node of the rule tree in the order of the parent tree, each
with its texts in that language where it has a translation that follows
its original, else in the original, the domains and one-ofs above it
with their kinds; the citations in the texts as Markdown
links, relative to the rule's page; the other side of each conflict; and,
for each provision of the constitution, the rules whose `authority` names
it, and for each article, the rules that name it or a provision under it.
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

statute reads and checks the sources (`statute export`, built from this
checkout with cargo) and prints what is wrong with them; the exit status
is 1 when they cannot be read.
"""

import argparse
from collections import defaultdict
from dataclasses import dataclass
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


def node(fields: dict[str, Any]) -> Node:
  """Return a node of the rule tree from the fields of its file.

  Args:
    fields: the fields, as `statute export` gives them.

  Returns:
    The node.
  """
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
    data=fields,
  )


class Constitution(NamedTuple):
  """What the pages take from the constitution.

  Attributes:
    languages: the languages of index.toml, in order.
    articles: the IDs of the articles.
    ancestors: for each section, its article; for each item, its section
      and its article; nearest first.
    labels: for each language, the label of each provision by citation:
      `Art. IV §1(2)` for `principals.competent`.
  """

  languages: list[str]
  articles: set[str]
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
    articles={p["id"] for p in provisions if p["parent"] is None},
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
    Its constitution and its nodes, or None when statute cannot read or
    check them; statute has then printed why.
  """
  run = subprocess.run(
    [*STATUTE, "export", str(path)],
    stdout=subprocess.PIPE,
    text=True,
    encoding="utf-8",
    check=False,
  )
  return json.loads(run.stdout) if run.returncode == 0 else None


def first_party(value: object) -> bool:
  """Tell whether a value names a first-party rule, as `R12` does.

  Args:
    value: a value of a node's file.

  Returns:
    True when it does.
  """
  return isinstance(value, str) and FIRST_PARTY_REF.fullmatch(value) is not None


class Source(NamedTuple):
  """The rules of one source.

  Attributes:
    name: its name, as the rules of the other sources name it.
    quadrant: one of QUADRANTS.
    nodes: its rule tree.
  """

  name: str
  quadrant: str
  nodes: list[Node]


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


def tree_order(nodes: list[Node]) -> list[tuple[Node, int]]:
  """Return the nodes along the parent tree, each with its depth.

  Each node comes before its children, which follow in order of ID, as
  the domains do; a node with no parent in this source starts at depth 0.

  Args:
    nodes: the nodes, a valid tree.

  Returns:
    Each node and its depth, 0 for a domain that is not a subdomain.
  """
  children: dict[int | None, list[Node]] = defaultdict(list)
  for n in sorted(nodes, key=lambda n: n.id):
    # A node under a domain of another source, or in place of a rule of
    # another source, heads a tree of its own here.
    children[n.parent if isinstance(n.parent, int) else None].append(n)
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

    Args:
      key: its source's name and its ID.

    Returns:
      The title.
    """
    node = self.nodes[key]
    return node.title[self.shown(node)]

  def ref(
    self, target: int | str, source: str = FIRST_PARTY
  ) -> dict[str, object]:
    """Return a node named by `requires`, `conflicts` or a field like them.

    Args:
      target: its ID, or a reference to a node of another source.
      source: the source of the node that names it.

    Returns:
      Its ID, label, title and page, or the reference alone when it names
      a node of a source the site does not have.
    """
    key = (source, target) if isinstance(target, int) else other_ref(target)
    if key not in self.nodes:
      return {"ref": target}
    return {
      "id": key[1],
      "label": rule_label(key[1], key[0]),
      "title": self.title(key),
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
      depth: its depth in the tree, 0 for a domain.
      conflicts: the rules it conflicts with, on either side.
      enabled: the labels of the rules the rule set enables, or None.

    Returns:
      The node's data.
    """
    data = node.data
    name = source.name
    shown = self.shown(node)
    groups, ancestor = [], node.parent
    while isinstance(ancestor, int):
      parent = self.nodes[(name, ancestor)]
      if parent.kind != "rule":
        groups.insert(0, self.ref(parent.id, name) | {"kind": parent.kind})
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
        "depth": depth,
        "groups": groups,
        "title": node.title[shown],
        "applicability": self.links(node.applicability[shown], shown, name)
        if node.applicability
        else None,
        "text": {
          part: self.links(node.text[part][shown], shown, name)
          for part in PARTS
          if node.text
        },
        "parent": self.ref(node.parent, name) if node.parent else None,
        "overrides": self.ref(node.overrides, name) if node.overrides else None,
        "supersededBy": [self.ref(t, name) for t in node.superseded_by],
        "children": [
          self.ref(n.id, name) for n in source.nodes if n.parent == node.id
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
) -> dict[str, Any]:
  """Return the data of every language of the site.

  Args:
    src: the constitution.
    nodes: this repo's rule tree.
    others: the rules of other sources, if any.
    enabled: the labels of the rules a rule set enables, or None.

  Returns:
    For each language, its sources, each with its name and quadrant
    (`sources`); its nodes in tree order, source by source; for each
    provision a listed rule names in `authority`, those rules
    (`citedBy`); and for each article that or a provision under it
    names, those rules (`citedWithin`); each list in order of source and
    ID, without repeats. With other sources, each node's groups start
    with its source.
  """
  sources = [Source(FIRST_PARTY, QUADRANTS[0], nodes), *(others or [])]
  sources.sort(key=lambda s: QUADRANTS.index(s.quadrant))
  rank = {s.name: i for i, s in enumerate(sources)}
  order = [(s, n, depth) for s in sources for n, depth in tree_order(s.nodes)]
  conflicts = both_sides(sources)
  articles = src.articles
  cited: dict[str, list[tuple[str, int]]] = defaultdict(list)
  within: dict[str, list[tuple[str, int]]] = defaultdict(list)
  for s, n, _ in order:
    if listed(n):
      refs = specific(n.authority, src)
      for ref in refs:
        cited[ref].append((s.name, n.id))
      for article in sorted(
        {r for ref in refs for r in (ref, *src.ancestors.get(ref, ()))}
        & articles
      ):
        within[article].append((s.name, n.id))

  def ordered(keys: list[tuple[str, int]]) -> list[tuple[str, int]]:
    return sorted(keys, key=lambda k: (rank[k[0]], k[1]))

  data: dict[str, Any] = {}
  for lang in src.languages:
    language = _Language(lang, src, sources)
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
    }
  return data


def load(path: Path) -> tuple[Constitution | None, list[Node]] | None:
  """Read the constitution and the rules of a directory through statute.

  Args:
    path: the directory.

  Returns:
    Its constitution, None when it has none, and its rule tree; or None
    when statute cannot read or check them.
  """
  data = export(path)
  if data is None:
    return None
  src = data["constitution"]
  return (
    None if src is None else constitution(src),
    [node(n["fields"]) for n in data["nodes"]],
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
  src, nodes = read
  if src is None:
    print(f"{args.root}: error: no constitution/", file=sys.stderr)
    return 1
  others = []
  for name, quadrant, path in args.source:
    other = load(path)
    if other is None:
      return 1
    others.append(Source(name, quadrant, other[1]))
  enabled = enabled_rules(args.enabled) if args.enabled else None
  data = site_data(src, nodes, others, enabled)
  args.out.parent.mkdir(parents=True, exist_ok=True)
  args.out.write_text(
    json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
  )
  return 0


if __name__ == "__main__":
  sys.exit(main())
