"""Tests for the nodes of the rule tree, `rules/<id>.toml`.

The format of docs/design/rule-format.md (Rule file format). Each check
fails on a broken node and passes on its control, the tree as set up.
"""

import contextlib
import io
from pathlib import Path
import tomllib

import test_constitution_base as tcb
from test_constitution_fixture import (
  DOMAIN,
  original,
  RULE,
  STATEMENT,
  Tree,
)

from constitution_compiler import build as bc
from constitution_compiler import checks as cc

# Shared with statute's tests (crates/statute/tests/superseded_by.rs).
CASES = Path(__file__).parent / "superseded_by_cases.toml"
# Shared with statute's tests (crates/statute/tests/compile.rs).
APPLICABILITY_CASES = Path(__file__).parent / "applicability_cases.toml"
SOURCE_CASES = Path(__file__).parent / "source_cases.toml"


class FormatTest(Tree):
  def test_controls_pass_sorted_by_number(self):
    errors, src, _ = bc.build(self.root)
    self.assertEqual(errors, [])
    assert src is not None
    self.assertEqual([n.id for n in src.nodes], [2, 3, 10])

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

  def test_domain_has_a_domain_or_no_parent_and_no_text(self):
    self.edit("rules/2.toml", 'lang = "en"', 'lang = "en"\nparent = 10')
    self.assertError(
      3,
      "rules/2.toml",
      "a domain's parent is a domain with no parent, or a one-of in one;"
      " 10 is a rule",
    )
    self.edit("rules/2.toml", "parent = 10\n", "")
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
    self.move("rules/010.toml", "rules/new-rule.toml")
    self.assertError(
      1,
      "rules/new-rule.toml",
      "a rule's file name is its ID, without leading zeros",
    )

  def test_values(self):
    for old, new, message in (
      ('kind = "rule"', 'kind = "law"', "`kind` must be one of"),
      ('status = "draft"', 'status = "old"', "a rule `status` must be one of"),
      ('level = "project"', 'level = "team"', "`level` must be one of"),
      ('check = "review"', 'check = "manual"', "`check` must be one of"),
      ("revision = 1", "revision = 0", "`revision` must be 1 or more"),
    ):
      with self.subTest(new), self.edited("rules/10.toml", old, new):
        errors = self.errors()
        self.assertTrue(
          any(e.startswith("rules/10.toml:") and message in e for e in errors),
          errors,
        )

  def test_list_fields(self):
    self.edit(
      "rules/10.toml",
      'check = "review"',
      'check = "command"\nrun = "x"\ntools = ["uv"]\nrequires = [3, "team:7"]'
      "\nconflicts = [2]",
    )
    self.assertEqual(self.errors(), [])
    for old, new, message in (
      (
        '["alpha", "alpha-one.money"]',
        "[]",
        "`authority` must be a non-empty list of provision IDs",
      ),
      (
        '["alpha", "alpha-one.money"]',
        '["alpha", 1]',
        "`authority` must be a non-empty list of provision IDs",
      ),
      ('tools = ["uv"]', "tools = [1]", "`tools` must be a list of names"),
    ):
      with self.subTest(new), self.edited("rules/10.toml", old, new):
        self.assertError(1, "rules/10.toml", message)

  def test_references(self):
    self.edit(
      "rules/10.toml",
      'check = "review"',
      'check = "review"\nrequires = [3, "team:7"]\nconflicts = [2, "team:8"]',
    )
    self.assertEqual(self.errors(), [])
    bad = " is not a reference: a number from 1 for a node of this source,"
    bad += ' or "<source>:<id>" for one of another'
    for key, old in (
      ("requires", '[3, "team:7"]'),
      ("conflicts", '[2, "team:8"]'),
    ):
      for new, message in (
        ('["seven"]', f"`{key}` entry `seven`{bad}"),
        ("[0]", f"`{key}` entry `0`{bad}"),
        ("[-3]", f"`{key}` entry `-3`{bad}"),
        ("[true]", f"`{key}` entry `true`{bad}"),
        ("[2.5]", f"`{key}` entry `2.5`{bad}"),
        ('["team:07"]', f"`{key}` entry `team:07`{bad}"),
        ('["Team:7"]', f"`{key}` entry `Team:7`{bad}"),
        ("[10]", f"`{key}` names the rule itself"),
        ("[3, 3]", f"`{key}` lists 3 twice"),
        ('["team:7", "team:7"]', f"`{key}` lists team:7 twice"),
      ):
        with (
          self.subTest(key=key, new=new),
          self.edited("rules/10.toml", f"{key} = {old}", f"{key} = {new}"),
        ):
          self.assertError(1, "rules/10.toml", message)

  def test_constitution_compares_whole_revisions(self):
    old = 'constitution = ">=1"'
    for good in (
      ">=1",
      "==1",
      ">= 1, <4",
      " > 0 ,!=3",
      "<=2",
      "<=18446744073709551615",
    ):
      with (
        self.subTest(good),
        self.edited("rules/10.toml", old, f'constitution = "{good}"'),
      ):
        self.assertEqual(self.errors(), [])
    for bad in ("~=1", "1", "any", ">=1.0", "==1.*", "===1", "", ">=1,"):
      with (
        self.subTest(bad),
        self.edited("rules/10.toml", old, f'constitution = "{bad}"'),
      ):
        self.assertError(
          1,
          "rules/10.toml",
          f'`constitution = "{bad}"` is not a specifier of whole revisions:'
          " clauses such as `>=1` joined by commas, each with"
          " >=, <=, >, <, == or !=",
        )

  def test_constitution_revisions_fit_in_64_bits(self):
    old = 'constitution = ">=1"'
    for bad in (">=18446744073709551616", ">=1, <018446744073709551616"):
      with (
        self.subTest(bad),
        self.edited("rules/10.toml", old, f'constitution = "{bad}"'),
      ):
        self.assertError(
          1,
          "rules/10.toml",
          f'`constitution = "{bad}"` names a revision above'
          " 18446744073709551615, the largest there is",
        )

  def test_language_codes(self):
    self.write("rules/2.toml", DOMAIN.replace("zh =", "zh-Hant ="))
    self.assertEqual(self.errors(), [])
    self.edit("rules/2.toml", 'lang = "en"', 'lang = "English"')
    self.assertError(
      1,
      "rules/2.toml",
      '`lang = "English"` is not a language code such as `en`',
    )
    self.write("rules/2.toml", DOMAIN.replace("zh =", "Chinese =", 1))
    self.assertError(
      1, "rules/2.toml", "`title` key `Chinese` is not a language code"
    )

  def test_translation_is_not_empty(self):
    self.edit("rules/2.toml", 'zh = "流程"', 'zh = " "')
    self.assertError(1, "rules/2.toml", "`title` `zh` is empty")
    self.edit("rules/10.toml", "因为。\n", "\n")
    self.assertError(1, "rules/10.toml", "`text.reason` `zh` is empty")

  def test_only_a_rule_is_superseded(self):
    self.activate()
    self.edit("rules/2.toml", 'status = "active"', 'status = "superseded"')
    errors = self.errors()
    self.assertEqual(len(errors), 1, errors)
    self.assertIn("a domain `status` must be one of", errors[0])
    self.assertNotIn("superseded'", errors[0])

  def test_status_under_a_draft_index(self):
    self.edit("rules/2.toml", 'status = "draft"', 'status = "trial"')
    self.assertError(
      1, "rules/2.toml", "a domain is trial while index.toml is draft"
    )

  def test_trial_under_an_active_index(self):
    self.activate()
    self.edit("rules/2.toml", 'status = "active"', 'status = "trial"')
    self.assertEqual(self.errors(), [])

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


class SupersededTest(Tree):
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
      with (
        self.subTest(target),
        self.edited(
          "rules/11.toml", "superseded-by = 10", f"superseded-by = {target}"
        ),
      ):
        self.assertError(
          3, "rules/11.toml", f"superseded-by {target} is no other rule"
        )

  def test_superseded_by_shows_a_boolean_as_toml_writes_it(self):
    with self.edited(
      "rules/11.toml", "superseded-by = 10", "superseded-by = [true]"
    ):
      self.assertError(
        1,
        "rules/11.toml",
        "`superseded-by` entry `true` is not a rule ID, a number from 1",
      )

  def test_superseded_by_cases(self):
    """The cases statute reads too, with the same outcome."""
    self.write(
      "rules/12.toml",
      self.SUPERSEDED.replace("id = 11", "id = 12")
      .replace('slug = "old-rule"', 'slug = "other-rule"')
      .replace('status = "draft"', 'status = "active"'),
    )
    cases = tomllib.loads(CASES.read_text(encoding="utf-8"))["cases"]
    self.assertGreater(len(cases), 0)
    for case in cases:
      with (
        self.subTest(case["value"]),
        self.edited(
          "rules/11.toml",
          "superseded-by = 10\n",
          f"superseded-by = {case['value']}\n",
        ),
      ):
        errors = self.errors()
        self.assertEqual(not errors, case["passes"], errors)
        self.assertTrue(
          all(e.startswith("rules/11.toml:") for e in errors), errors
        )


class ApplicabilityTest(Tree):
  def test_applicability_cases(self):
    """The cases statute reads too, with the same outcome."""
    cases = tomllib.loads(APPLICABILITY_CASES.read_text(encoding="utf-8"))
    self.assertGreater(len(cases["cases"]), 0)
    for case in cases["cases"]:
      with self.subTest(case["name"]):
        self.write("rules/20.toml", case["node"])
        errors = self.errors()
        self.assertEqual(not errors, case["passes"], errors)
        self.assertTrue(
          all(e.startswith("rules/20.toml:") for e in errors), errors
        )
        if not case["passes"]:
          self.assertTrue(any(case["compiler"] in e for e in errors), errors)

  def test_only_a_domain_has_applicability(self):
    applicability = (
      '\n[applicability]\nen = "For every repository."\nzh = "所有仓库。"\n'
    )
    self.write("rules/2.toml", DOMAIN + applicability)
    self.rehash("rules/2.toml")
    self.assertEqual(self.errors(), [])
    self.edit(
      "rules/3.toml", 'zh = "二选一"\n', 'zh = "二选一"\n' + applicability
    )
    self.assertError(1, "rules/3.toml", "unknown key `applicability`")


def node(id_: int, kind: str, parent: int) -> str:
  """Return a domain or a one-of with only the fields it needs.

  Args:
    id_: its number.
    kind: domain or one-of.
    parent: its parent's number.

  Returns:
    The file's text.
  """
  return (
    f'id = {id_}\nrevision = 1\nkind = "{kind}"\nstatus = "draft"\n'
    f'lang = "en"\nparent = {parent}\n\n[title]\nen = "Node {id_}"\n'
  )


class ParentTest(Tree):
  """No rule under a rule (docs/design/rule-config.md)."""

  def test_a_rules_parent_is_a_domain_or_a_one_of(self):
    self.edit("rules/10.toml", "parent = 2", "parent = 3")
    self.assertEqual(self.errors(), [])
    under_a_rule = (
      RULE.replace("id = 10", "id = 11")
      .replace('slug = "new-rule"\n', "")
      .replace("parent = 2", "parent = 10")
    )
    self.write("rules/11.toml", under_a_rule)
    self.assertError(
      3,
      "rules/11.toml",
      "a rule's parent is a domain or a one-of; 10 is a rule",
    )

  def test_the_options_of_a_one_of_are_all_rules_or_all_domains(self):
    self.write("rules/20.toml", node(20, "domain", 3))
    self.assertEqual(self.errors(), [])
    self.edit("rules/10.toml", "parent = 2", "parent = 3")
    self.assertError(
      3,
      "rules/20.toml",
      "parent 3 is a one-of whose options are rules (10);"
      " the options of a one-of are all rules or all domains",
    )

  def test_a_domain_option_counts_as_a_level(self):
    self.write("rules/4.toml", node(4, "domain", 2))
    self.write("rules/5.toml", node(5, "one-of", 4))
    self.assertEqual(self.errors(), [])
    self.write("rules/20.toml", node(20, "domain", 5))
    self.assertError(
      3,
      "rules/20.toml",
      "a domain's parent is a domain with no parent, or a one-of in one;"
      " 5 is a one-of in 4, a subdomain, under 2",
    )


class SourceTest(Tree):
  def test_source_cases(self):
    """Overrides and subdomains, as far as one source can tell."""
    cases = tomllib.loads(SOURCE_CASES.read_text(encoding="utf-8"))
    self.write("rules/4.toml", cases["subdomain"])
    self.assertEqual(self.errors(), [])
    self.assertGreater(len(cases["cases"]), 0)
    for case in cases["cases"]:
      with self.subTest(case["name"]):
        self.write("rules/20.toml", case["node"])
        errors = self.errors()
        self.assertEqual(not errors, case["passes"], errors)
        self.assertTrue(
          all(e.startswith("rules/20.toml:") for e in errors), errors
        )
        if not case["passes"]:
          self.assertTrue(any(case["compiler"] in e for e in errors), errors)

  def test_first_party_reference_is_this_source(self):
    """This repo's rules are the first-party ones: `R2` is a node here."""
    self.edit("rules/10.toml", "parent = 2", 'parent = "R2"')
    self.assertError(
      1, "rules/10.toml", "`parent` `R2` names a node of this source; write 2"
    )
    self.edit("rules/10.toml", 'parent = "R2"', 'overrides = "R3"')
    self.assertError(
      1,
      "rules/10.toml",
      "`overrides` names R3, a rule of this source; within one source a"
      " rule is changed in place or superseded, never overridden",
    )
    self.edit("rules/10.toml", 'overrides = "R3"', 'overrides = "team:3"')
    self.assertEqual(self.errors(), [])


class TreeTest(Tree):
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


class TranslationTest(Tree):
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
    self.edit("rules/10.toml", "与{@rule:3}", "")
    self.assertError(
      4, "rules/10.toml", "en `text.statement` cites rule:3; zh does not"
    )


class ReferenceTest(Tree):
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


class AuthorityOverlapTest(Tree):
  """The warning on an `authority` that names a provision and one above it.

  Attributes:
    WARNING: the warning on the tree as set up, whose rule 10 names
      `alpha` and its item `alpha-one.money`.
  """

  WARNING = (
    "rules/10.toml:8: warning: rule 10: authority names both alpha and"
    " alpha-one.money, which is under it"
  )

  def run_main(self):
    err = io.StringIO()
    with contextlib.redirect_stderr(err):
      status = bc.main(["--root", str(self.root)])
    return status, err.getvalue().splitlines()

  def test_item_under_its_article_warns_without_failing(self):
    self.assertEqual(self.run_main(), (0, [self.WARNING]))

  def test_item_under_its_section_warns(self):
    self.edit("rules/10.toml", '"alpha",', '"alpha-one",')
    _, src, _ = bc.build(self.root)
    assert src is not None
    self.assertEqual(
      cc.authority_overlaps(src),
      [
        "rules/10.toml:8: warning: rule 10: authority names both alpha-one"
        " and alpha-one.money, which is under it"
      ],
    )

  def test_superseded_rule_does_not_warn(self):
    """Rule 11 names the same pair as rule 10 but is superseded."""
    self.write("rules/11.toml", SupersededTest.SUPERSEDED)
    self.activate()
    self.edit(
      "rules/11.toml",
      'status = "draft"',
      'status = "superseded"\nsuperseded-by = 10',
    )
    self.assertEqual(self.run_main(), (0, [self.WARNING]))

  def test_provisions_apart_do_not_warn(self):
    self.edit("rules/10.toml", '"alpha",', '"beta",')
    self.assertEqual(self.run_main(), (0, []))


class NodeBaseTest(Tree):
  """Check 7 on the nodes of the rule tree.

  Attributes:
    git: runs git in the tree, as in BaseTest.
    commit_base: commits the tree as the base, as in BaseTest.
    base_errors: runs check 7 against the base, as in BaseTest.
  """

  git = tcb.BaseTest.git
  commit_base = tcb.BaseTest.commit_base
  base_errors = tcb.BaseTest.base_errors

  def test_older_rule_lives_on_under_its_slug(self):
    """A base from before the rule tree, with a rule named by a word."""
    self.commit_base()
    self.write("rules/rule.toml", 'id = "rule"\nstatus = "active"\n')
    self.git("add", "-A")
    self.git("commit", "-q", "-m", "a rule of the older format")
    (self.root / "rules/rule.toml").unlink()
    errors, _ = self.base_errors()
    self.assertEqual(
      errors,
      [
        "rules/rule.toml:1: check 7: rule rule existed at base HEAD and no"
        " node has it as its slug"
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
