"""Tests for the nodes of the rule tree, `rules/<id>.toml`.

statute validates the rule tree. Here: check 1 as the site reads the
nodes (`load` with `nodes=True`), in the format of
docs/design/rule-format.md (Rule file format), and what the constitution
build reads of them: the status against index.toml's, what they cite of
the constitution, and check 7. Each check fails on a broken node and
passes on its control, the tree as set up.
"""

import contextlib
import io

import test_constitution_base as tcb
from test_constitution_fixture import (
  DOMAIN,
  RULE,
  Tree,
)

from constitution_compiler import build as bc
from constitution_compiler import checks as cc
from constitution_compiler.model import Errors
from constitution_compiler.read import load


class Nodes(Tree):
  """The tree as the site reads it, the nodes of the rule tree whole."""

  def load(self):
    errors = Errors()
    return errors, load(self.root, errors, nodes=True)

  def errors(self):
    return self.load()[0]


class FormatTest(Nodes):
  def test_controls_pass_sorted_by_number(self):
    errors, src = self.load()
    self.assertEqual(errors, [])
    assert src is not None
    self.assertEqual([n.id for n in src.nodes], [2, 3, 10])

  def test_texts_lose_the_newlines_by_the_quotes(self):
    _, src = self.load()
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

  def test_keys_statute_knows_are_passed_over(self):
    """The site reads what it shows; statute validates the rest."""
    self.edit(
      "rules/10.toml",
      'check = "review"',
      'check = "none"\ndefault = "on"\nuses = ["review"]',
    )
    self.edit("rules/3.toml", "parent = 2", "parent = 2\ndefault = 10")
    self.assertEqual(self.errors(), [])

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


class SupersededTest(Nodes):
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

  def test_superseded_by_shows_a_boolean_as_toml_writes_it(self):
    with self.edited(
      "rules/11.toml", "superseded-by = 10", "superseded-by = [true]"
    ):
      self.assertError(
        1,
        "rules/11.toml",
        "`superseded-by` entry `true` is not a rule ID, a number from 1",
      )


class ApplicabilityTest(Nodes):
  def test_a_domain_has_applicability(self):
    applicability = (
      '\n[applicability]\nen = "For every repository."\nzh = "所有仓库。"\n'
    )
    self.write("rules/2.toml", DOMAIN + applicability)
    self.rehash("rules/2.toml")
    self.assertEqual(self.errors(), [])


class SourceTest(Nodes):
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


class TranslationTest(Nodes):
  def test_no_translation_into_the_original(self):
    self.edit(
      "rules/2.toml", "\n[translations]\n", '\n[translations]\nen = "x"\n'
    )
    self.assertError(
      1,
      "rules/2.toml",
      "`translations` maps each language but the original's to a hash",
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


class BuildTest(Tree):
  """What the constitution build reads of the rule tree, and no more."""

  def test_what_statute_validates_is_passed_over(self):
    """Keys and values the build does not read are statute's to check."""
    self.edit(
      "rules/10.toml",
      'check = "review"',
      'check = "none"\ndefault = "on"\nuses = ["review"]\nrequires = [9999]',
    )
    self.edit("rules/3.toml", "parent = 2", "parent = 2\ndefault = 10")
    errors, _, _ = bc.build(self.root)
    self.assertEqual(errors, [])

  def test_status_agrees_with_the_index(self):
    """The build reads a node's status and checks it as a section's."""
    self.edit("rules/2.toml", 'status = "draft"', 'status = "trial"')
    errors, _, _ = bc.build(self.root)
    self.assertEqual(
      [e for e in errors if e.startswith("rules/2.toml")],
      ["rules/2.toml:5: check 1: a domain is trial while index.toml is draft"],
    )


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

  def test_params_toml_is_no_node(self):
    """The parameters beside the nodes, in the base and now."""
    self.write("rules/params.toml", '[[parameter]]\nname = "language.docs"\n')
    self.commit_base()
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
