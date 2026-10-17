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
  node_id: int,
  kind: str,
  parent: int | None,
  cites: str = "3",
  title: str | None = None,
  zh: str | None = None,
) -> str:
  """Return the file of a domain or a one-of, translated.

  A domain has an applicability that cites a rule and is off by
  default; the one-of's default is its option, rule 3.

  Args:
    node_id: its ID.
    kind: domain or one-of.
    parent: its parent, None for a domain.
    cites: the rule a domain's applicability cites, as `{@rule:...}`
      names it.
    title: its English title, `Group <id>` when None.
    zh: its Chinese title, `组 <id>` when None.

  Returns:
    The TOML text.
  """
  title = title or f"Group {node_id}"
  zh = zh or f"组 {node_id}"
  texts = [title]
  text = (
    f'id = {node_id}\nslug = "group-{node_id}"\nrevision = 1\n'
    f'kind = "{kind}"\nstatus = "active"\nlang = "en"\n'
    + (f"parent = {parent}\n" if parent else "")
    + ('default = "off"\n' if kind == "domain" else "default = 3\n")
    + f'\n[title]\nen = "{title}"\nzh = "{zh}"\n'
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
  text += f'lang = "en"\nparent = {parent}\ndefault = "off"\n{extra}'
  text += 'constitution = ">=1"\nlevel = "project"\ncheck = "none"\n'
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
  # An option of the one-of, which names the default, has none.
  "rules/3.toml": rule(3, 2, 'authority = ["one", "one-a"]\n').replace(
    'default = "off"\n', "", 1
  ),
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


def long_domain(root: Path) -> None:
  """Give the domain 1 a title as long as the longest of this repo's.

  Args:
    root: the checkout.
  """
  title = "Agent instructions, skills and the settings of their tools"
  (root / "rules/1.toml").write_text(
    group(1, "domain", None, title=title), encoding="utf-8"
  )


def repealed(root: Path) -> None:
  """Repeal rule 4 instead of superseding it.

  Args:
    root: the checkout.
  """
  (root / "rules/4.toml").write_text(
    rule(4, 1, 'status = "repealed"\nauthority = ["one"]\n').replace(
      'status = "active"\n', "", 1
    ),
    encoding="utf-8",
  )


def long_code(root: Path) -> None:
  """Put a path in code, longer than a phone's row, in two statements.

  Rule 5's English one, which the English list shows, and rule 3's
  Chinese one, which the Chinese list shows; the English text the
  translation follows is unchanged.

  Args:
    root: the checkout.
  """
  code = "`.claude/skills/<name>/references/alongfilename.md`"
  for name, old, new in (
    ("5", 'en = "New."', f'en = "New, in {code}."'),
    ("3", "和{@rule:6}。", f"和{{@rule:6}}，见 {code}。"),
  ):
    rule = root / f"rules/{name}.toml"
    text = rule.read_text(encoding="utf-8")
    assert old in text
    rule.write_text(text.replace(old, new, 1), encoding="utf-8")


def rule_on(root: Path) -> None:
  """Turn rule 6 on by default, in the domain 1, which stays off.

  Args:
    root: the checkout.
  """
  rule = root / "rules/6.toml"
  text = rule.read_text(encoding="utf-8")
  assert 'default = "off"\n' in text
  rule.write_text(
    text.replace('default = "off"\n', 'default = "on"\n', 1), encoding="utf-8"
  )


def subdomain(root: Path) -> None:
  """Make the one-of 2 a domain under the domain 1.

  Its rule 3, no longer an option, says whether it is on by default.

  Args:
    root: the checkout.
  """
  (root / "rules/2.toml").write_text(group(2, "domain", 1), encoding="utf-8")
  rule = root / "rules/3.toml"
  text = rule.read_text(encoding="utf-8")
  rule.write_text(
    text.replace("parent = 2\n", 'parent = 2\ndefault = "off"\n', 1),
    encoding="utf-8",
  )


def shared_titles(root: Path) -> None:
  """Give two domains under two parents one title, Tiers / 层级.

  Domain 7, under the domain 1, and domain 9, under a new domain 8,
  which holds rule 4.

  Args:
    root: the checkout.
  """
  files = {
    7: group(7, "domain", 1, title="Tiers", zh="层级"),
    8: group(8, "domain", None),
    9: group(9, "domain", 8, title="Tiers", zh="层级"),
  }
  for node_id, text in files.items():
    (root / f"rules/{node_id}.toml").write_text(text, encoding="utf-8")
  rule = root / "rules/4.toml"
  text = rule.read_text(encoding="utf-8")
  rule.write_text(
    text.replace("parent = 1\n", "parent = 9\n", 1), encoding="utf-8"
  )


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
# applicability cites a first-party rule 9 that no source here has; 2, a
# rule under it, untranslated, that requires and cites rule 3 of this repo
# and cites itself; 4, the same rule under this repo's domain 1, in
# conflict with this repo's rule 5.
OTHER_RULE = """\
id = 2
revision = 1
kind = "rule"
status = "active"
lang = "en"
parent = 1
default = "off"
requires = ["R3"]
authority = ["one-a"]
constitution = ">=1"
level = "user"
check = "none"

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


PARAMS = """\
[[parameter]]
name = "language.docs"
type = "language"
scope = "repo"
default = "zh"

[parameter.title]
en = "Language of documents"

[[parameter]]
name = "work-items.label-done"
type = "string"
scope = "repo"
default = "done & dusted"

[parameter.title]
en = "Label of a done item"
zh = "完成项的标签"

[parameter.translations]
zh = "{sha}"
""".replace("{sha}", original("Label of a done item"))


def params(root: Path) -> None:
  """Declare two parameters and name them in rule 6's statement.

  `language.docs`, a language, and `work-items.label-done`, a string
  with a character HTML escapes, which the statement names in a code
  span.

  Args:
    root: the checkout.
  """
  (root / "rules/params.toml").write_text(PARAMS, encoding="utf-8")
  rule = root / "rules/6.toml"
  text = rule.read_text(encoding="utf-8")
  old = 'en = "Rule six, see {@constitution:one-a.x}."'
  assert old in text
  rule.write_text(
    text.replace(
      old,
      'en = "Write in {$language.docs}; label `x {$work-items.label-done}`."',
    ),
    encoding="utf-8",
  )


def long_value(root: Path) -> None:
  """Name in rule 3's statement, in a code span, a path wider than a phone.

  `dirs.socket`, a string whose default starts with `~`, as the tip's
  line of the default does too. The Chinese title joins paths in code by
  full-width commas, which no line breaks next to: one word wider than a
  phone.

  Args:
    root: the checkout.
  """
  path = "~/an-example-service/with-a-long-name/client.sock"
  (root / "rules/params.toml").write_text(
    '[[parameter]]\nname = "dirs.socket"\ntype = "string"\nscope = "repo"\n'
    f'default = "{path}"\n\n[parameter.title]\nen = "Socket"\n',
    encoding="utf-8",
  )
  rule = root / "rules/3.toml"
  text = rule.read_text(encoding="utf-8")
  en, zh = STATEMENTS[3]
  new_en = "Run `serve -socket {$dirs.socket}`."
  new_zh = "运行 `serve -socket {$dirs.socket}`。"
  zh_title = "`.example/one/`，`.example/two/`，`.example/three/`"
  for old, new in (
    (en, new_en),
    (zh, new_zh),
    ('zh = "规定 3"', f'zh = "规定 3 {zh_title}"'),
    (
      original("Rule 3", en, "Why.", "Violated when not."),
      original("Rule 3", new_en, "Why.", "Violated when not."),
    ),
  ):
    assert old in text
    text = text.replace(old, new, 1)
  rule.write_text(text, encoding="utf-8")


def domain_option(root: Path) -> None:
  """Make the one-of 2's option a domain, 7, which holds rule 3.

  The one-of names the domain as its default; the domain, an option,
  has no `default`, and rule 3, no longer an option, has its own.

  Args:
    root: the checkout.
  """
  domain = group(7, "domain", 2).replace('default = "off"\n', "", 1)
  (root / "rules/7.toml").write_text(domain, encoding="utf-8")
  one_of = root / "rules/2.toml"
  text = one_of.read_text(encoding="utf-8")
  one_of.write_text(
    text.replace("default = 3\n", "default = 7\n", 1), encoding="utf-8"
  )
  rule = root / "rules/3.toml"
  text = rule.read_text(encoding="utf-8")
  rule.write_text(
    text.replace("parent = 2\n", 'parent = 7\ndefault = "off"\n', 1),
    encoding="utf-8",
  )
