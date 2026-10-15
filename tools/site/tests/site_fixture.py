"""A made-up checkout with rules, shared by the tests of the rules' pages.

Also a second source, `demo`, at the end.

The constitution is active, so that a rule may be superseded: one article,
`one`, with one section, `one-a`, holding one item, `one-a.x`. The rule
tree:

- 1, a domain, translated;
  - 2, a one-of, translated;
    - 3, a rule, translated, citing `one` and `one-a`;
  - 4, a rule superseded by 5 and 6, untranslated;
  - 5, a rule, untranslated, requiring 6 and in conflict with 3;
  - 6, a rule, untranslated, citing `one-a.x`.
"""

import hashlib
from pathlib import Path

INDEX = """\
status = "active"
edition = "2026-10"
revision = "1"
languages = ["en", "zh"]

[title]
en = "Title"
zh = "标题"

[front]
en = "Front."
zh = "序。"
"""
ARTICLE = """\
id = "one"

[title]
en = "One"
zh = "一"
"""
SECTION = """\
id = "one-a"
status = "active"

[content]
en = "Section."
zh = "款。"

[[items]]
id = "x"
number = "1"
status = "active"

[items.content]
en = "Item."
zh = "项。"
"""
STATEMENTS = {
  3: (
    "See {@constitution:one-a} and {@rule:6}.",
    "见{@constitution:one-a}和{@rule:6}。",
  ),
  4: ("Old.", None),
  5: ("New.", None),
  6: ("Rule six, see {@constitution:one-a.x}.", None),
}


APPLICABILITY = "For every repository; see {{@rule:{rule}}}."


def original(*texts: str) -> str:
  """Return the hash a translation records.

  Args:
    *texts: the original's title and parts.

  Returns:
    The SHA-256 of the texts joined by NUL, in hex.
  """
  return hashlib.sha256("\0".join(texts).encode()).hexdigest()


def group(
  node_id: int, kind: str, parent: int | None, cites: str = "3"
) -> str:
  """Return the file of a domain or a one-of, translated.

  A domain has an applicability that cites a rule.

  Args:
    node_id: its ID.
    kind: domain or one-of.
    parent: its parent, None for a domain.
    cites: the rule a domain's applicability cites, as `{@rule:...}`
      names it.

  Returns:
    The TOML text.
  """
  title = f"Group {node_id}"
  texts = [title]
  text = (
    f'id = {node_id}\nslug = "group-{node_id}"\nrevision = 1\n'
    f'kind = "{kind}"\nstatus = "active"\nlang = "en"\n'
    + (f"parent = {parent}\n" if parent else "")
    + f'\n[title]\nen = "{title}"\nzh = "组 {node_id}"\n'
  )
  if kind == "domain":
    applicability = APPLICABILITY.format(rule=cites)
    texts.append(applicability)
    text += f'\n[applicability]\nen = "{applicability}"\n'
    text += f'zh = "所有仓库，见{{@rule:{cites}}}。"\n'
  return text + f'\n[translations]\nzh = "{original(*texts)}"\n'


def rule(node_id: int, parent: int, extra: str = "") -> str:
  """Return the file of a rule.

  Args:
    node_id: its ID.
    parent: its parent.
    extra: more fields, one a line.

  Returns:
    The TOML text.
  """
  en, zh = STATEMENTS[node_id]
  title = f"Rule {node_id}"
  texts = [("statement", en, zh), ("reason", "Why.", "为何。")]
  texts.append(("verification", "Violated when not.", "否则违反。"))
  text = f'id = {node_id}\nrevision = 2\nkind = "rule"\nstatus = "active"\n'
  text += f'lang = "en"\nparent = {parent}\n{extra}'
  text += 'constitution = ">=1"\nlevel = "project"\ncheck = "review"\n'
  text += f'\n[title]\nen = "{title}"\n'
  if zh:
    text += f'zh = "规定 {node_id}"\n'
  for part, en_text, zh_text in texts:
    text += f'\n[text.{part}]\nen = "{en_text}"\n'
    if zh:
      text += f'zh = "{zh_text}"\n'
  if zh:
    sha = original(title, *(t[1] for t in texts))
    text += f'\n[translations]\nzh = "{sha}"\n'
  return text


FILES = {
  "constitution/2026-10/index.toml": INDEX,
  "constitution/2026-10/articles/01-one/article.toml": ARTICLE,
  "constitution/2026-10/articles/01-one/01-one-a.toml": SECTION,
  "rules/1.toml": group(1, "domain", None),
  "rules/2.toml": group(2, "one-of", 1),
  "rules/3.toml": rule(3, 2, 'authority = ["one", "one-a"]\n'),
  "rules/4.toml": rule(
    4,
    1,
    'status = "superseded"\nsuperseded-by = [5, 6]\nauthority = ["one"]\n',
  ).replace('status = "active"\n', "", 1),
  "rules/5.toml": rule(
    5, 1, 'requires = [6]\nconflicts = [3, "team:7"]\nauthority = ["one"]\n'
  ),
  "rules/6.toml": rule(6, 1, 'authority = ["one-a.x"]\n'),
}


def write(root: Path) -> None:
  """Write the checkout.

  Args:
    root: the directory to write it in.
  """
  for name, text in FILES.items():
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


# A made-up second source, `demo`: 1, a domain, translated, whose
# applicability cites a first-party rule 9 that no source here has; 2, a rule under
# it, untranslated, that requires and cites rule 3 of this repo and cites
# itself; 4, the same rule under this repo's domain 1, in conflict with
# this repo's rule 5.
OTHER_RULE = """\
id = 2
revision = 1
kind = "rule"
status = "active"
lang = "en"
parent = 1
requires = ["R3"]
authority = ["one-a"]
constitution = ">=1"
level = "user"
check = "review"

[title]
en = "Demo rule"

[text.statement]
en = "After {@rule:R3}, see {@rule:2}."

[text.reason]
en = "Why."

[text.verification]
en = "Violated when not."
"""
OTHER_FILES = {
  "rules/1.toml": group(1, "domain", None, "R9"),
  "rules/2.toml": OTHER_RULE,
  "rules/4.toml": OTHER_RULE.replace("id = 2", "id = 4")
  .replace("parent = 1", 'parent = "R1"')
  .replace('requires = ["R3"]', 'conflicts = ["R5"]'),
}


def write_other(root: Path) -> None:
  """Write the second source.

  Args:
    root: the directory to write it in.
  """
  for name, text in OTHER_FILES.items():
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
