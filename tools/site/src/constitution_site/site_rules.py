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

Errors print as `path:line: check 1: message`, as constitution-build
prints them; the exit status is 1 when the sources cannot be read.
"""

import argparse
from collections import defaultdict
import json
from pathlib import Path
import re
import sys
from typing import Any

from constitution_compiler import render
from constitution_compiler.model import Errors, MARK, Node, PARTS, Sources
from constitution_compiler.read import load

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


def rule_label(node_id: int) -> str:
  """Return the label the site gives a node of the rule tree: `R12`.

  Args:
    node_id: the node's ID.

  Returns:
    The label.
  """
  return f"R{node_id}"


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
  the domains do.

  Args:
    nodes: the nodes, a valid tree.

  Returns:
    Each node and its depth, 0 for a domain.
  """
  children: dict[int | None, list[Node]] = defaultdict(list)
  for n in sorted(nodes, key=lambda n: n.id):
    children[n.parent].append(n)
  order: list[tuple[Node, int]] = []

  def visit(parent: int | None, depth: int) -> None:
    for n in children[parent]:
      order.append((n, depth))
      visit(n.id, depth + 1)

  visit(None, 0)
  return order


class _Language:
  """The data of one language of the site."""

  def __init__(self, lang: str, src: Sources) -> None:
    """Index the nodes and the constitution's labels.

    Args:
      lang: the language of the pages.
      src: the sources.
    """
    self.lang, self.src = lang, src
    self.nodes = {n.id: n for n in src.nodes}
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

  def title(self, node_id: int) -> str:
    """Return a node's title as this language's pages show it.

    Args:
      node_id: the node's ID.

    Returns:
      The title.
    """
    node = self.nodes[node_id]
    return node.title[self.shown(node)]

  def ref(self, target: int | str) -> dict[str, object]:
    """Return a node named by `requires`, `conflicts` or a field like them.

    Args:
      target: its ID, or a reference to a node of another source.

    Returns:
      Its ID, label and title, or the reference alone.
    """
    if isinstance(target, str):
      return {"ref": target}
    return {
      "id": target,
      "label": rule_label(target),
      "title": self.title(target),
    }

  def links(self, text: str, text_lang: str) -> str:
    """Return a text with each citation as a Markdown link.

    A provision's label is in the language of the text, or of the page
    when the constitution has no text in it.

    Args:
      text: the text, as the sources hold it.
      text_lang: its language.

    Returns:
      The Markdown text.
    """
    labels = self.labels.get(text_lang) or self.labels[self.lang]
    rule = RULE_LABEL.get(text_lang) or RULE_LABEL[self.lang]

    def link(m: re.Match[str]) -> str:
      namespace, target = m.group(1), m.group(2)
      if namespace == "rule":
        return f"[{rule.format(id=target)}]({RULE_LINK.format(id=target)})"
      label = labels.get(target, target)
      return f"[{label}]({PROVISION_LINK.format(ref=target)})"

    return MARK.sub(link, text)

  def node(
    self, node: Node, depth: int, conflicts: list[int | str]
  ) -> dict[str, object]:
    """Return what the pages show of one node.

    Args:
      node: the node.
      depth: its depth in the tree, 0 for a domain.
      conflicts: the rules it conflicts with, on either side.

    Returns:
      The node's data.
    """
    data = node.doc.data
    shown = self.shown(node)
    groups, ancestor = [], node.parent
    while ancestor is not None:
      parent = self.nodes[ancestor]
      if parent.kind != "rule":
        groups.insert(0, self.ref(parent.id) | {"kind": parent.kind})
      ancestor = parent.parent
    return {
      "id": node.id,
      "label": rule_label(node.id),
      "slug": node.slug,
      "kind": node.kind,
      "status": node.status,
      "lang": node.lang,
      "shown": shown,
      "translations": sorted(
        lang for lang, sha in node.translations.items() if sha == node.original
      ),
      "depth": depth,
      "groups": groups,
      "title": node.title[shown],
      "text": {
        part: self.links(node.text[part][shown], shown)
        for part in PARTS
        if node.text
      },
      "parent": self.ref(node.parent) if node.parent else None,
      "supersededBy": [self.ref(t) for t in node.superseded_by],
      "children": [
        self.ref(n.id) for n in self.src.nodes if n.parent == node.id
      ],
      "requires": [self.ref(t) for t in data.get("requires", [])],
      "conflicts": [self.ref(t) for t in conflicts],
      "authority": [
        {"ref": ref, "label": self.labels[self.lang].get(ref, ref)}
        for ref in specific(node.authority, self.src)
      ],
    } | {field: data[field] for field in FIELDS if field in data}


def site_data(src: Sources) -> dict[str, Any]:
  """Return the data of every language of the site.

  Args:
    src: the sources.

  Returns:
    For each language, its nodes in tree order; for each provision a
    listed rule names in `authority`, those rules (`citedBy`); and for
    each article that or a provision under it names, those rules
    (`citedWithin`); each list in order of ID, without repeats.
  """
  order = tree_order(src.nodes)
  # A conflict is written on one side; the pages show it on both.
  conflicts: dict[int, list[int | str]] = defaultdict(list)
  for n in src.nodes:
    for target in n.doc.data.get("conflicts", []):
      conflicts[n.id].append(target)
      if isinstance(target, int):
        conflicts[target].append(n.id)
  articles = {a.id for a in src.articles}
  cited: dict[str, list[int]] = defaultdict(list)
  within: dict[str, list[int]] = defaultdict(list)
  for n, _ in order:
    if listed(n):
      refs = specific(n.authority, src)
      for ref in refs:
        cited[ref].append(n.id)
      for article in sorted(
        {r for ref in refs for r in (ref, *src.ancestors.get(ref, ()))}
        & articles
      ):
        within[article].append(n.id)
  data: dict[str, Any] = {}
  for lang in src.languages:
    language = _Language(lang, src)
    data[lang] = {
      "nodes": [
        language.node(n, depth, sorted(conflicts[n.id], key=str))
        for n, depth in order
      ],
      "citedBy": {
        ref: [language.ref(i) for i in sorted(ids)]
        for ref, ids in cited.items()
      },
      "citedWithin": {
        ref: [language.ref(i) for i in sorted(ids)]
        for ref, ids in within.items()
      },
    }
  return data


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
  args = parser.parse_args(argv)
  errors = Errors()
  src = load(args.root, errors)
  if src is None:
    print("\n".join(errors), file=sys.stderr)
    return 1
  args.out.parent.mkdir(parents=True, exist_ok=True)
  args.out.write_text(
    json.dumps(site_data(src), ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
  )
  return 0


if __name__ == "__main__":
  sys.exit(main())
