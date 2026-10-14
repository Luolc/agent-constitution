"""Tests for the nodes of the rule tree, `rules/<id>.toml`.

The format of docs/design/rule-system.md (Rule file format), read beside
the rules of the older format. Each check fails on a broken node and
passes on its control, the tree as set up.
"""

import hashlib
import re
import tomllib

import test_constitution_base as tcb
from test_constitution_fixture import Tree

from constitution_compiler import build as bc


def original(*texts: str) -> str:
  """Return the hash a translation records, as the design defines it.

  Args:
    *texts: the original's title and parts, as TOML parses them.

  Returns:
    The SHA-256 of the texts joined by NUL, in hex.
  """
  return hashlib.sha256(b"\0".join(t.encode() for t in texts)).hexdigest()


DOMAIN = f"""\
id = 2
slug = "process"
revision = 1
kind = "domain"
status = "draft"
lang = "en"

[title]
en = "Process"
zh = "流程"

[translations]
zh = "{original("Process")}"
"""

ONE_OF = """\
id = 3
revision = 1
kind = "one-of"
status = "draft"
lang = "zh"
parent = 2

[title]
zh = "二选一"
"""

STATEMENT = "Cite {@constitution:beta-one}, {@rule:3} and {@rule:rule}.\n"
RULE = f"""\
id = 10
slug = "new-rule"
revision = 1
kind = "rule"
status = "draft"
lang = "en"
parent = 2
authority = ["alpha", "alpha-one.money"]
constitution = ">=1"
level = "project"
check = "review"

[title]
en = "A new rule"
zh = "一条新规定"

[text.statement]
en = '''
{STATEMENT}'''
zh = '''
引用{{@constitution:beta-one}}、{{@rule:3}}与{{@rule:rule}}。
'''

[text.reason]
en = '''
Because.
'''
zh = '''
因为。
'''

[text.verification]
en = '''
Violated when not.
'''
zh = '''
没有做到即为违反。
'''

[translations]
zh = "{original("A new rule", STATEMENT, "Because.\n", "Violated when not.\n")}"
"""

NODES = {"rules/2.toml": DOMAIN, "rules/3.toml": ONE_OF, "rules/10.toml": RULE}


class NodeTree(Tree):
  """The test tree with a domain, a one-of and a rule of the rule tree."""

  def setUp(self):
    super().setUp()
    for name, text in NODES.items():
      self.write(name, text)

  def activate(self):
    super().activate()
    for name in NODES:
      self.edit(name, 'status = "draft"', 'status = "active"')

  def rehash(self, name: str) -> None:
    """Record the current original's hash for each translation of a node.

    Args:
      name: the node's file.
    """
    path = self.root / name
    text = path.read_text(encoding="utf-8")
    data = tomllib.loads(text)
    lang = data["lang"]
    texts = [data["title"]] + [
      data["text"][p] for p in ("statement", "reason", "verification")
    ] * ("text" in data)
    new = original(*(t[lang] for t in texts))
    path.write_text(
      re.sub(r'(?m)^(\w+) = "[0-9a-f]{64}"$', rf'\1 = "{new}"', text),
      encoding="utf-8",
    )

  def assertError(self, check, path, message):  # noqa: N802
    errors = self.errors()
    pattern = re.compile(rf"{re.escape(path)}:\d+: check {check}: ")
    self.assertTrue(
      any(pattern.match(e) and e.endswith(message) for e in errors), errors
    )


class FormatTest(NodeTree):
  def test_controls_pass_sorted_by_number(self):
    errors, src, _ = bc.build(self.root)
    self.assertEqual(errors, [])
    assert src is not None
    self.assertEqual([n.id for n in src.nodes], [2, 3, 10])
    self.assertEqual([r.id for r in src.rules], ["rule"])

  def test_texts_lose_the_newlines_by_the_quotes(self):
    _, src, _ = bc.build(self.root)
    assert src is not None
    self.assertEqual(
      src.nodes[2].text["reason"], {"en": "Because.", "zh": "因为。"}
    )

  def test_rule_lacks_a_field(self):
    self.edit("rules/10.toml", 'check = "review"\n', "")
    self.assertError(1, "rules/10.toml", "lacks `check`")

  def test_rule_lacks_a_part(self):
    self.edit("rules/10.toml", "[text.reason]", "[text.reasons]")
    self.assertError(1, "rules/10.toml", "`text` lacks `reason`")

  def test_domain_has_no_parent_or_text(self):
    self.edit("rules/2.toml", 'lang = "en"', 'lang = "en"\nparent = 3')
    self.assertError(1, "rules/2.toml", "unknown key `parent`")
    self.edit("rules/2.toml", "parent = 3\n", "")
    self.write(
      "rules/2.toml", DOMAIN + "\n[text.statement]\nen = '''\nDo.\n'''\n"
    )
    self.assertError(1, "rules/2.toml", "unknown key `text`")

  def test_one_of_lacks_parent(self):
    self.edit("rules/3.toml", "parent = 2\n", "")
    self.assertError(1, "rules/3.toml", "lacks `parent`")

  def test_id_is_the_file_name(self):
    self.edit("rules/10.toml", "id = 10", "id = 11")
    self.assertError(
      1, "rules/10.toml", "`id` must be 10, as in the name `10.toml`"
    )
    self.edit("rules/10.toml", "id = 11", 'id = "10"')
    self.assertError(1, "rules/10.toml", "`id` must be an integer")
    self.edit("rules/10.toml", 'id = "10"', "id = 10")
    self.move("rules/10.toml", "rules/010.toml")
    self.assertError(
      1, "rules/010.toml", "a rule's file name is its ID, without leading zeros"
    )

  def test_values(self):
    for old, new, message in (
      ('kind = "rule"', 'kind = "law"', "`kind` must be one of"),
      ('status = "draft"', 'status = "old"', "a rule `status` must be one of"),
      ('level = "project"', 'level = "team"', "`level` must be one of"),
      ('check = "review"', 'check = "manual"', "`check` must be one of"),
      ("revision = 1", "revision = 0", "`revision` must be 1 or more"),
    ):
      with self.subTest(new):
        self.edit("rules/10.toml", old, new)
        errors = self.errors()
        self.assertTrue(
          any(e.startswith("rules/10.toml:") and message in e for e in errors),
          errors,
        )
        self.edit("rules/10.toml", new, old)

  def test_status_under_a_draft_index(self):
    self.edit("rules/2.toml", 'status = "draft"', 'status = "trial"')
    self.assertError(
      1, "rules/2.toml", "a domain is trial while index.toml is draft"
    )

  def test_command_check(self):
    self.edit(
      "rules/10.toml",
      'check = "review"',
      'check = "command"\nrun = "uv run x"\ntools = ["uv"]',
    )
    self.assertEqual(self.errors(), [])
    self.edit("rules/10.toml", 'tools = ["uv"]\n', "")
    self.assertError(
      1, "rules/10.toml", "`tools` is there exactly when `check` is command"
    )
    self.edit("rules/10.toml", 'check = "command"', 'check = "review"')
    self.assertError(
      1, "rules/10.toml", "`run` is there exactly when `check` is command"
    )

  def test_original_is_required(self):
    self.edit("rules/10.toml", "Because.\n", "\n")
    self.assertError(
      1,
      "rules/10.toml",
      "`text.reason` lacks its text in `en`, the original's language",
    )
    self.edit("rules/3.toml", 'zh = "二选一"', 'en = "One of two"')
    self.assertError(
      1,
      "rules/3.toml",
      "`title` lacks its text in `zh`, the original's language",
    )

  def test_title_is_one_line(self):
    self.edit("rules/2.toml", 'en = "Process"', "en = '''\nPro\ncess\n'''")
    self.assertError(1, "rules/2.toml", "`title` must be one line")


class SupersededTest(NodeTree):
  SUPERSEDED = RULE.replace("id = 10", "id = 11").replace(
    'slug = "new-rule"', 'slug = "old-rule"'
  )

  def setUp(self):
    super().setUp()
    self.write("rules/11.toml", self.SUPERSEDED)
    self.activate()
    self.edit(
      "rules/11.toml",
      'status = "draft"',
      'status = "superseded"\nsuperseded-by = 10',
    )

  def test_superseded_by_a_rule_passes(self):
    self.assertEqual(self.errors(), [])

  def test_superseded_by_needs_superseded(self):
    self.edit("rules/11.toml", 'status = "superseded"', 'status = "active"')
    self.assertError(
      1,
      "rules/11.toml",
      "`superseded-by` is there exactly when `status` is superseded",
    )
    self.edit("rules/11.toml", 'status = "active"', 'status = "superseded"')
    self.edit("rules/11.toml", "superseded-by = 10\n", "")
    self.assertError(
      1,
      "rules/11.toml",
      "`superseded-by` is there exactly when `status` is superseded",
    )

  def test_superseded_by_another_rule(self):
    for target in ("2", "11", "99"):
      with self.subTest(target):
        self.edit(
          "rules/11.toml", "superseded-by = 10", f"superseded-by = {target}"
        )
        self.assertError(
          3, "rules/11.toml", f"superseded-by {target} is no other rule"
        )
        self.edit(
          "rules/11.toml", f"superseded-by = {target}", "superseded-by = 10"
        )


class TreeTest(NodeTree):
  def test_parent_exists(self):
    self.edit("rules/10.toml", "parent = 2", "parent = 9")
    self.assertError(3, "rules/10.toml", "parent 9 is no node")

  def test_parent_chain_has_no_cycle(self):
    self.edit("rules/10.toml", "parent = 2", "parent = 3")
    self.assertEqual(self.errors(), [])
    self.edit("rules/3.toml", "parent = 2", "parent = 10")
    self.assertError(
      3, "rules/3.toml", "following the parents from 3 comes back to it"
    )
    self.assertError(
      3, "rules/10.toml", "following the parents from 10 comes back to it"
    )

  def test_slug(self):
    self.edit("rules/10.toml", 'slug = "new-rule"', 'slug = "New rule"')
    self.assertError(
      2,
      "rules/10.toml",
      "`New rule` is not an ID: lowercase words joined by hyphens",
    )
    self.edit("rules/10.toml", 'slug = "New rule"', 'slug = "process"')
    self.assertError(
      2, "rules/10.toml", "process is already defined at rules/2.toml:2"
    )
    self.edit("rules/10.toml", 'slug = "process"', 'slug = "rule"')
    self.assertError(
      2, "rules/10.toml", "rule is already defined at rules/rule.toml:1"
    )


class TranslationTest(NodeTree):
  def test_translation_needs_every_part(self):
    self.edit("rules/10.toml", "zh = '''\n因为。\n'''\n", "")
    self.assertError(
      4, "rules/10.toml", "`text.reason` lacks its translation into `zh`"
    )

  def test_translation_needs_a_hash(self):
    self.edit("rules/2.toml", "\n[translations]\n", "\n[other]\n")
    self.assertError(1, "rules/2.toml", "unknown key `other`")
    self.write("rules/2.toml", DOMAIN.split("\n[translations]")[0] + "\n")
    self.assertError(
      4,
      "rules/2.toml",
      "`title` has a text in `zh`, `translations` has no hash for it",
    )

  def test_no_translation_into_the_original(self):
    self.edit(
      "rules/2.toml", "\n[translations]\n", '\n[translations]\nen = "x"\n'
    )
    self.assertError(
      1,
      "rules/2.toml",
      "`translations` maps each language but the original's to a hash",
    )

  def test_stale_translation_prints_the_new_hash(self):
    self.edit("rules/10.toml", "Because.", "Because so.")
    new = original(
      "A new rule", STATEMENT, "Because so.\n", "Violated when not.\n"
    )
    self.assertError(
      4,
      "rules/10.toml",
      "the `zh` translation was made from another original; once it follows"
      f" the original, its hash is {new}",
    )
    self.edit("rules/2.toml", 'en = "Process"', 'en = "Processes"')
    self.assertError(4, "rules/2.toml", f"its hash is {original('Processes')}")

  def test_translation_cites_what_the_original_cites(self):
    self.edit("rules/10.toml", "、{@rule:3}", "")
    self.assertError(
      4, "rules/10.toml", "en `text.statement` cites rule:3; zh does not"
    )


class ReferenceTest(NodeTree):
  def test_authority_has_no_namespace(self):
    self.edit("rules/10.toml", '"alpha",', '"constitution:alpha",')
    self.assertError(
      5,
      "rules/10.toml",
      "authority cites constitution:alpha; the constitution cites only"
      " itself, without a namespace",
    )

  def test_authority_resolves(self):
    self.edit("rules/10.toml", '"alpha-one.money"', '"alpha-one.cash"')
    self.assertError(
      5, "rules/10.toml", "authority cites undefined alpha-one.cash"
    )

  def test_rule_citation_resolves(self):
    self.edit("rules/10.toml", "{@rule:3}", "{@rule:9}", 2)
    self.rehash("rules/10.toml")
    self.assertError(5, "rules/10.toml", "cites undefined rule:9")

  def test_rule_citation_not_repealed(self):
    self.activate()
    self.edit("rules/3.toml", 'status = "active"', 'status = "repealed"')
    self.assertError(5, "rules/10.toml", "cites repealed rule:3")

  def test_old_rule_cites_a_node(self):
    self.edit("rules/rule.toml", "{@rule:rule}", "{@rule:10}", 2)
    self.assertEqual(self.errors(), [])


class NodeBaseTest(NodeTree):
  """Check 7 across the move from the older format to the rule tree.

  Attributes:
    git: runs git in the tree, as in BaseTest.
    commit_base: commits the tree as the base, as in BaseTest.
    base_errors: runs check 7 against the base, as in BaseTest.
  """

  git = tcb.BaseTest.git
  commit_base = tcb.BaseTest.commit_base
  base_errors = tcb.BaseTest.base_errors

  def test_old_rule_lives_on_under_its_slug(self):
    self.commit_base()
    (self.root / "rules/rule.toml").unlink()
    self.edit("rules/10.toml", "{@rule:rule}", "{@rule:10}", 2)
    self.rehash("rules/10.toml")
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        "rules/rule.toml:1: check 7: rule rule existed at base HEAD and is gone"
        " without being listed in rules/retired-ids.txt"
      ],
    )
    self.edit("rules/10.toml", 'slug = "new-rule"', 'slug = "rule"')
    self.assertEqual(self.base_errors(), ([], ""))

  def test_node_is_never_deleted(self):
    self.commit_base()
    (self.root / "rules/3.toml").unlink()
    self.edit("rules/10.toml", "{@rule:3}", "{@rule:2}", 2)
    self.rehash("rules/10.toml")
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        "rules/3.toml:1: check 7: one-of 3 existed at base HEAD and is gone;"
        " repeal it instead"
      ],
    )

  def test_superseded_stays_superseded(self):
    self.write("rules/11.toml", SupersededTest.SUPERSEDED)
    self.activate()
    self.edit(
      "rules/11.toml",
      'status = "draft"',
      'status = "superseded"\nsuperseded-by = 10',
    )
    self.commit_base(active=False)
    self.edit(
      "rules/11.toml",
      'status = "superseded"\nsuperseded-by = 10',
      'status = "active"',
    )
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        "rules/11.toml:5: check 7: rule 11 was superseded at base HEAD and is"
        " no longer"
      ],
    )
