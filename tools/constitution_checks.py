"""Checks 2 to 5 of the constitution and rule sources.

IDs, structure, language alignment and references (docs/design.md,
Checkers).
"""

import re

from constitution_sources import (
  citations,
  Errors,
  ID,
  number_key,
  RETIRED,
  Sources,
)

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

  for t in src.texts():
    if t.repealed:
      continue
    for text in t.texts.values():
      for ref in citations(text):
        if found := problem(ref, t.in_rule):
          errors.add(t.doc.path, t.doc.line(f"{{@{ref}}}"), 5, f"cites {found}")
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
