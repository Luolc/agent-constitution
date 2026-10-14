"""Checks 2 to 5 of the constitution and rule sources.

IDs, structure, language alignment and references (docs/design/checkers.md).
"""

import re

from constitution_compiler.model import (
  citations,
  Doc,
  Errors,
  ID,
  Node,
  number_key,
  Sources,
)

# Check 2: IDs.


def check_ids(src: Sources, errors: Errors) -> None:
  """Check 2: every ID has the format and is unique where it must be.

  Article and section IDs are unique in the constitution, item IDs in
  their section, the rule tree's slugs among the slugs.

  Args:
    src: the sources.
    errors: where errors go.
  """
  unique_ids(
    errors,
    [(a.doc, a.id) for a in src.articles]
    + [(s.doc, s.id) for s in src.sections],
  )
  for s in src.sections:
    unique_ids(errors, [(s.doc, i.id) for i in s.items])
  unique_ids(
    errors,
    [(n.doc, n.slug) for n in src.nodes if n.slug is not None],
  )


def unique_ids(errors: Errors, declared: list[tuple[Doc, str]]) -> None:
  """Check that each ID has the format and none repeats in one scope.

  Args:
    errors: where errors go.
    declared: (doc, ID) for each ID of the scope.
  """
  seen = {}
  for doc, value in declared:
    line = doc.line(f'id = "{value}"', f'slug = "{value}"')
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
  check_tree(src.nodes, errors)


def check_tree(nodes: list[Node], errors: Errors) -> None:
  """Check the links between the nodes of the rule tree.

  Each `parent` of this source and each entry of `superseded-by` names a
  node of the tree, another rule for `superseded-by`; a domain's parent
  is a domain with no parent, so domains nest two levels deep at most
  (docs/design/rule-sources.md, Subdomains); and following the parents
  never comes back to a node. A parent in another source is not checked
  here.

  Args:
    nodes: the nodes.
    errors: where errors go.
  """
  by_id = {n.id: n for n in nodes}
  for n in nodes:
    parent = by_id.get(n.parent) if isinstance(n.parent, int) else None
    if isinstance(n.parent, int) and parent is None:
      errors.add(
        n.doc.path, n.doc.line("parent ="), 3, f"parent {n.parent} is no node"
      )
    if (
      n.kind == "domain"
      and parent is not None
      and (problem := domain_parent(n, parent))
    ):
      errors.add(
        n.doc.path,
        n.doc.line("parent ="),
        3,
        f"a domain's parent is a domain with no parent; {problem}",
      )
    for t in n.superseded_by:
      target = by_id.get(t)
      if target is None or target.kind != "rule" or target is n:
        errors.add(
          n.doc.path,
          n.doc.line("superseded-by ="),
          3,
          f"superseded-by {t} is no other rule",
        )
  check_cycles(by_id, errors)


def check_cycles(by_id: dict[int, Node], errors: Errors) -> None:
  """Check that following the parents never comes back to a node.

  Args:
    by_id: the nodes, by ID.
    errors: where errors go.
  """
  for n in by_id.values():
    seen, at = {n.id}, n.parent
    while isinstance(at, int) and at in by_id:
      if at == n.id:
        errors.add(
          n.doc.path,
          n.doc.line("parent ="),
          3,
          f"following the parents from {n.id} comes back to it",
        )
        break
      if at in seen:
        break  # a cycle above this node, reported at its own nodes
      seen.add(at)
      at = by_id[at].parent


def domain_parent(node: Node, parent: Node) -> str:
  """Return what is wrong with a domain's parent, or "" when nothing is.

  Args:
    node: the domain.
    parent: the node its `parent` names.

  Returns:
    The problem, or "".
  """
  if parent is node:
    return f"{node.id} is the domain itself"
  if parent.kind != "domain":
    return f"{parent.id} is a {parent.kind}"
  if parent.parent is not None:
    return f"{parent.id} is a subdomain, under {parent.parent}"
  return ""


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

  The first language is that of index.toml, or a node's original. A
  node's translations are also each complete and made from its current
  original.

  Args:
    src: the sources.
    errors: where errors go.
  """
  for t in src.texts():
    first = t.lang or src.languages[0]
    refs = {lang: set(citations(text)) for lang, text in t.texts.items()}
    others = src.languages if t.lang is None else t.texts
    for lang in [lang for lang in others if lang != first]:
      for a, b in ((first, lang), (lang, first)):
        for ref in sorted(refs[a] - refs[b]):
          errors.add(
            t.doc.path,
            t.doc.line(f"{{@{ref}}}"),
            4,
            f"{a} `{t.field}` cites {ref}; {b} does not",
          )
  for n in src.nodes:
    check_translations(n, errors)


def check_translations(node: Node, errors: Errors) -> None:
  """Check that the node's texts have just the languages it translates to.

  Each translation follows the current original: its hash is the one the
  original has now.

  Args:
    node: the node.
    errors: where errors go.
  """
  doc = node.doc
  texts = (
    {"title": node.title}
    | ({"applicability": node.applicability} if node.applicability else {})
    | {f"text.{p}": t for p, t in node.text.items()}
  )
  for field, table in texts.items():
    for lang in sorted(set(table) - {node.lang} ^ set(node.translations)):
      errors.add(
        doc.path,
        doc.line(f"[{field}]"),
        4,
        f"`{field}` has a text in `{lang}`, `translations` has no hash for it"
        if lang in table
        else f"`{field}` lacks its translation into `{lang}`",
      )
  start = doc.line("[translations]")
  for lang, value in sorted(node.translations.items()):
    if value != node.original:
      errors.add(
        doc.path,
        doc.line(f"{lang} =", start=start),
        4,
        f"the `{lang}` translation was made from another original;"
        f" once it follows the original, its hash is {node.original}",
      )


# Check 5: references.


def check_refs(src: Sources, errors: Errors) -> None:
  """Check 5: every citation and authority resolves to an unrepealed target.

  The constitution cites itself without a namespace; a rule cites with
  `constitution:` or `rule:`; every citation and authority resolves to a
  provision or rule that is not repealed.

  Args:
    src: the sources.
    errors: where errors go.
  """
  rules = {str(n.id): n.status for n in src.nodes}
  for t in src.texts():
    if t.repealed:
      continue
    for text in t.texts.values():
      for ref in citations(text):
        if found := problem(src, rules, ref, t.in_rule):
          errors.add(t.doc.path, t.doc.line(f"{{@{ref}}}"), 5, f"cites {found}")
  for n in src.nodes:
    if n.status != "repealed":
      check_node_authority(src, rules, n, errors)


def problem(
  src: Sources, rules: dict[str, str], ref: str, in_rule: bool
) -> str:
  """Return what is wrong with the citation, or "" when nothing is.

  Args:
    src: the sources.
    rules: the status of each node, by ID.
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
    return f"repealed {ref}" if rules[target] == "repealed" else ""
  if target not in src.provisions:
    return f"undefined {ref}"
  return f"repealed {ref}" if target in src.repealed else ""


def check_node_authority(
  src: Sources, rules: dict[str, str], node: Node, errors: Errors
) -> None:
  """Check that each authority of a node names a provision, no namespace.

  Args:
    src: the sources.
    rules: the status of each node, by ID.
    node: the node.
    errors: where errors go.
  """
  for ref in node.authority:
    if found := problem(src, rules, ref, False):
      errors.add(
        node.doc.path,
        node.doc.line("authority ="),
        5,
        f"authority cites {found}",
      )


def authority_overlaps(src: Sources) -> list[str]:
  """Warn of each node whose `authority` names a provision and one above it.

  The narrower provision says where the node belongs; the wider one adds
  nothing. Not an error: the build goes on. A superseded or repealed
  node is not in force and is left alone.

  Args:
    src: the sources.

  Returns:
    The warnings, `path:line: warning: message`, one per pair.
  """
  warnings = []
  for n in src.nodes:
    if n.status in ("superseded", "repealed"):
      continue
    for ref in n.authority:
      for above in src.ancestors.get(ref, ()):
        if above in n.authority:
          line = n.doc.line("authority =")
          warnings.append(
            f"{n.doc.path}:{line}: warning: rule {n.id}: authority names"
            f" both {above} and {ref}, which is under it"
          )
  return warnings
