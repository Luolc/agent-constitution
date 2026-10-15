"""Checks 2 to 5 of the constitution sources.

IDs, structure, language alignment and references (docs/design/checkers.md).
statute validates the rule tree (`statute validate rules`); of the rules,
these checks read only the provisions they cite and name in their
`authority`.
"""

import re

from constitution_compiler.model import (
  citations,
  Doc,
  Errors,
  ID,
  number_key,
  RuleFile,
  Sources,
)

# Check 2: IDs.


def check_ids(src: Sources, errors: Errors) -> None:
  """Check 2: every ID has the format and is unique where it must be.

  Article and section IDs are unique in the constitution, item IDs in
  their section.

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


def unique_ids(errors: Errors, declared: list[tuple[Doc, str]]) -> None:
  """Check that each ID has the format and none repeats in one scope.

  Args:
    errors: where errors go.
    declared: (doc, ID) for each ID of the scope.
  """
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

  The first language is the first of index.toml.

  Args:
    src: the sources.
    errors: where errors go.
  """
  first = src.languages[0]
  for t in src.texts():
    refs = {lang: set(citations(text)) for lang, text in t.texts.items()}
    for lang in src.languages[1:]:
      for a, b in ((first, lang), (lang, first)):
        for ref in sorted(refs[a] - refs[b]):
          errors.add(
            t.doc.path,
            t.doc.line(f"{{@{ref}}}"),
            4,
            f"{a} `{t.field}` cites {ref}; {b} does not",
          )


# Check 5: references.


def check_refs(src: Sources, errors: Errors) -> None:
  """Check 5: every citation of the constitution resolves.

  The constitution cites itself without a namespace, and a rule cites it
  with `constitution:`; each citation and each provision of a rule's
  `authority` names a provision that is not repealed. A rule's other
  citations are statute's to check.

  Args:
    src: the sources.
    errors: where errors go.
  """
  for t in src.texts():
    if t.repealed:
      continue
    for text in t.texts.values():
      for ref in citations(text):
        if ":" in ref:
          found = (
            f"{ref}; the constitution cites only itself, without a namespace"
          )
        else:
          found = problem(src, ref)
        if found:
          errors.add(t.doc.path, t.doc.line(f"{{@{ref}}}"), 5, f"cites {found}")
  for r in src.rules:
    if r.status != "repealed":
      check_rule_refs(src, r, errors)


def problem(src: Sources, ref: str, written: str = "") -> str:
  """Return what is wrong with a citation of a provision, or "" when nothing is.

  Args:
    src: the sources.
    ref: the provision, without a namespace.
    written: the citation as written, when it differs from ref.

  Returns:
    The problem, or "".
  """
  if ref not in src.provisions:
    return f"undefined {written or ref}"
  return f"repealed {written or ref}" if ref in src.repealed else ""


def check_rule_refs(src: Sources, rule: RuleFile, errors: Errors) -> None:
  """Check what a rule cites of the constitution and its `authority`.

  Args:
    src: the sources.
    rule: the rule, or a domain or one-of, which may cite too.
    errors: where errors go.
  """
  doc = rule.doc
  for _, table in rule.texts:
    for text in table.values():
      for ref in citations(text):
        namespace, _, target = ref.rpartition(":")
        if namespace == "constitution" and (found := problem(src, target, ref)):
          errors.add(doc.path, doc.line(f"{{@{ref}}}"), 5, f"cites {found}")
  for ref in rule.authority:
    found = (
      f"{ref}; the constitution cites only itself, without a namespace"
      if ":" in ref
      else problem(src, ref)
    )
    if found:
      errors.add(
        doc.path, doc.line("authority ="), 5, f"authority cites {found}"
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
  for n in src.rules:
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
