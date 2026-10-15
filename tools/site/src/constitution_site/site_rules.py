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
to its page. With `--enabled`, each rule says whether the rule set of a
statute.lock enables it. Without either, the pages are this repo's alone
and say neither.

Errors print as `path:line: check 1: message`, as constitution-build
prints them; the exit status is 1 when the sources cannot be read.
"""

import argparse
from collections import defaultdict
import json
from pathlib import Path
import re
import sys
import tomllib
from typing import Any, NamedTuple

from constitution_compiler import render
from constitution_compiler.model import (
  Errors,
  ID,
  Layout,
  MARK,
  Node,
  PARTS,
  RULES,
  Sources,
)
from constitution_compiler.read import _Reader, by_number, first_party, load

ROOT = Path(__file__).resolve().parents[4]
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
# A citation of a rule of another source, as `requires` names it:
# `{@rule:R12}`, `{@rule:team:7}`.
OTHER_RULE = re.compile(r"\{@rule:(R[1-9][0-9]*|" + ID + r":[1-9][0-9]*)\}")


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


def specific(authority: list[str], src: Sources) -> list[str]:
  """Return an `authority` without the provisions it names something under.

  Args:
    authority: the provisions, as the node names them.
    src: the sources.

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

  def __init__(self, lang: str, src: Sources, sources: list[Source]) -> None:
    """Index the nodes and the constitution's labels.

    Args:
      lang: the language of the pages.
      src: the sources.
      sources: the rules of each source, this repo's first.
    """
    self.lang, self.src = lang, src
    self.nodes = {(s.name, n.id): n for s in sources for n in s.nodes}
    self.many = len(sources) > 1
    self.labels = {code: render.labels(code, src) for code in src.languages}

  def shown(self, node: Node) -> str:
    """Return the language the node is shown in on this language's pages.

    The page's own when the node is written in it, or translated into it
    from its current original; its original's otherwise, also when the
    translation follows an older original.

    Args:
      node: the node.

    Returns:
      The language code.
    """
    current = node.translations.get(self.lang) == node.original
    return self.lang if node.lang == self.lang or current else node.lang

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
    data = node.doc.data
    name = source.name
    shown = self.shown(node)
    groups, ancestor = [], node.parent
    while isinstance(ancestor, int):
      parent = self.nodes[(name, ancestor)]
      if parent.kind != "rule":
        groups.insert(0, self.ref(parent.id, name) | {"kind": parent.kind})
      ancestor = parent.parent
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
        "translations": sorted(
          lang
          for lang, sha in node.translations.items()
          if sha == node.original
        ),
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
  for s in sources:
    for n in s.nodes:
      for target in n.doc.data.get("conflicts", []):
        conflicts[(s.name, n.id)].append(target)
        if isinstance(target, int):
          conflicts[(s.name, target)].append(n.id)
  return conflicts


def site_data(
  src: Sources,
  others: list[Source] | None = None,
  enabled: set[str] | None = None,
) -> dict[str, Any]:
  """Return the data of every language of the site.

  Args:
    src: the sources.
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
  sources = [Source(FIRST_PARTY, QUADRANTS[0], src.nodes), *(others or [])]
  sources.sort(key=lambda s: QUADRANTS.index(s.quadrant))
  rank = {s.name: i for i, s in enumerate(sources)}
  order = [(s, n, depth) for s in sources for n, depth in tree_order(s.nodes)]
  conflicts = both_sides(sources)
  articles = {a.id for a in src.articles}
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


class _OtherReader(_Reader):
  """Reads the rule tree of another source.

  There `R12` names a rule of this repo, as `requires` and the like name
  it; this repo's own reader takes that for a node of the same source.
  """

  def parent(self, doc: Any, kind: str) -> None:
    """Check a node's `parent`, which may name a domain of this repo.

    Args:
      doc: the node's file.
      kind: its kind.
    """
    if not (kind == "rule" and first_party(doc.data.get("parent"))):
      super().parent(doc, kind)

  def overrides(self, doc: Any) -> None:
    """Check a rule's `overrides`, which may name a rule of this repo.

    Args:
      doc: the rule's file.
    """
    if not first_party(doc.data.get("overrides")):
      super().overrides(doc)

  def references(self, doc: Any) -> None:
    """Check `requires` and `conflicts`, which may name rules of this repo.

    Args:
      doc: the rule's file.
    """
    d = doc.data
    kept = {k: d[k] for k in ("requires", "conflicts") if k in d}
    for k, values in kept.items():
      d[k] = [v for v in values if not first_party(v)]
    super().references(doc)
    d.update(kept)


def load_source(
  path: Path, name: str, quadrant: str, src: Sources, errors: Errors
) -> Source:
  """Read the rules of another source, as this repo's are read.

  Args:
    path: the source's directory, holding rules/.
    name: its name.
    quadrant: its quadrant.
    src: this repo's sources, for the languages and the status.
    errors: where errors go.

  Returns:
    The source.
  """
  reader = _OtherReader(
    path, Layout(Path()), errors, src.languages, src.index.data["status"]
  )
  for file in sorted(
    (p.relative_to(path) for p in (path / RULES).glob("*.toml")), key=by_number
  ):
    reader.node(file)
  return Source(name, quadrant, reader.nodes)


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
  errors = Errors()
  src = load(args.root, errors)
  others = (
    []
    if src is None
    else [
      load_source(path, name, quadrant, src, errors)
      for name, quadrant, path in args.source
    ]
  )
  if src is None or errors:
    print("\n".join(errors), file=sys.stderr)
    return 1
  enabled = enabled_rules(args.enabled) if args.enabled else None
  args.out.parent.mkdir(parents=True, exist_ok=True)
  args.out.write_text(
    json.dumps(site_data(src, others, enabled), ensure_ascii=False, indent=2)
    + "\n",
    encoding="utf-8",
  )
  return 0


if __name__ == "__main__":
  sys.exit(main())
