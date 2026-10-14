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
  RETIRED,
  Rule,
  Sources,
)

# Check 2: IDs.


def check_ids(src: Sources, errors: Errors) -> None:
  """Check 2: every ID has the format and is unique where it must be.

  Article and section IDs are unique in the constitution, item IDs in
  their section, rule IDs among the rules and the retired rule IDs. The
  rule tree's slugs share the scope of the rule IDs of the older format.

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
    [(r.doc, r.id) for r in src.rules]
    + [(n.doc, n.slug) for n in src.nodes if n.slug is not None],
  )
  check_retired(src, errors)


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


def check_retired(src: Sources, errors: Errors) -> None:
  """Check the retired rule IDs and that no rule uses one.

  Args:
    src: the sources.
    errors: where errors go.
  """
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

  Each `parent` and `superseded-by` names a node of the tree, a rule for
  `superseded-by`, and following the parents never comes back to a node.

  Args:
    nodes: the nodes.
    errors: where errors go.
  """
  by_id = {n.id: n for n in nodes}
  for n in nodes:
    if n.parent is not None and n.parent not in by_id:
      errors.add(
        n.doc.path, n.doc.line("parent ="), 3, f"parent {n.parent} is no node"
      )
    if n.superseded_by is not None:
      target = by_id.get(n.superseded_by)
      if target is None or target.kind != "rule" or target is n:
        errors.add(
          n.doc.path,
          n.doc.line("superseded-by ="),
          3,
          f"superseded-by {n.superseded_by} is no other rule",
        )
  for n in nodes:
    seen, at = {n.id}, n.parent
    while at in by_id:
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
  texts = {"title": node.title} | {f"text.{p}": t for p, t in node.text.items()}
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
  rules = {r.id: r.status for r in src.rules} | {
    str(n.id): n.status for n in src.nodes
  }
  for t in src.texts():
    if t.repealed:
      continue
    for text in t.texts.values():
      for ref in citations(text):
        if found := problem(src, rules, ref, t.in_rule):
          errors.add(t.doc.path, t.doc.line(f"{{@{ref}}}"), 5, f"cites {found}")
  for r in src.rules:
    if r.status != "repealed":
      check_authority(src, rules, r, errors)
  for n in src.nodes:
    if n.status != "repealed":
      check_node_authority(src, rules, n, errors)


def problem(
  src: Sources, rules: dict[str, str], ref: str, in_rule: bool
) -> str:
  """Return what is wrong with the citation, or "" when nothing is.

  Args:
    src: the sources.
    rules: the status of each rule and node, by ID.
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


def check_authority(
  src: Sources, rules: dict[str, str], rule: Rule, errors: Errors
) -> None:
  """Check that each authority of a rule is a resolving constitution citation.

  Args:
    src: the sources.
    rules: the status of each rule and node, by ID.
    rule: the rule.
    errors: where errors go.
  """
  line = rule.doc.line("authority =")
  for ref in rule.authority:
    if not ref.startswith("constitution:"):
      errors.add(
        rule.doc.path,
        line,
        5,
        f"authority {ref} is not a `constitution:` citation",
      )
    elif found := problem(src, rules, ref, True):
      errors.add(rule.doc.path, line, 5, f"authority cites {found}")


def check_node_authority(
  src: Sources, rules: dict[str, str], node: Node, errors: Errors
) -> None:
  """Check that each authority of a node names a provision, no namespace.

  Args:
    src: the sources.
    rules: the status of each rule and node, by ID.
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
