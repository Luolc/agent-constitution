//! Resolving a rule set against a source. Each test resolves a rule set
//! that passes, then one change of it that must fail or warn.

use std::collections::BTreeSet;

use statute::diag::{Diagnostics, Severity};
use statute::manifest::Key;
use statute::manifest::Manifest;
use statute::resolve::{Reason, Resolution, resolve};
use statute::source::Sources;

mod common;
use common::{domain, edit, one_of, rule, source};

/// The rule set enabling `rules` and `packs` of source `R`.
fn manifest(rules: &[u64], packs: &[u64]) -> Manifest {
    let list = |ids: &[u64]| {
        ids.iter()
            .map(|id| format!("\"R{id}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    Manifest::parse(&format!(
        "constitution = {{ source = \"R\", edition = \"2026-10\" }}\npublic = false\nrules = [{}]\npacks = [{}]\n\n[sources]\nR = {{ path = \".\" }}\n",
        list(rules),
        list(packs)
    ))
    .unwrap()
}

/// Rule `id` of source `R`.
fn r(id: u64) -> Key {
    Key::new("R", id)
}

fn run(source: &Sources, rules: &[u64], packs: &[u64], revision: u64) -> (Resolution, Diagnostics) {
    let mut diags = Diagnostics::default();
    let res = resolve(source, &manifest(rules, packs), revision, &mut diags);
    (res, diags)
}

#[track_caller]
fn enabled(source: &Sources, rules: &[u64], packs: &[u64]) -> Resolution {
    let (res, diags) = run(source, rules, packs, 1);
    assert!(
        diags.0.is_empty(),
        "expected no findings, got {:#?}",
        diags.0
    );
    res
}

#[track_caller]
fn ids(res: &Resolution) -> Vec<u64> {
    res.enabled.keys().map(|k| k.id).collect()
}

#[track_caller]
fn fails(source: &Sources, rules: &[u64], packs: &[u64], revision: u64, needle: &str) {
    let (_, diags) = run(source, rules, packs, revision);
    let errors: Vec<String> = diags
        .0
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(ToString::to_string)
        .collect();
    assert!(
        errors.len() == 1
            && errors[0].starts_with("statute.toml: error: ")
            && errors[0].contains(needle),
        "expected one error containing {needle:?}, got {errors:#?}"
    );
}

#[track_caller]
fn warns(source: &Sources, rules: &[u64], needle: &str) {
    let (_, diags) = run(source, rules, &[], 1);
    assert!(!diags.has_errors(), "unexpected errors: {:#?}", diags.0);
    let warnings: Vec<String> = diags.warnings().map(ToString::to_string).collect();
    assert!(
        warnings.len() == 1 && warnings[0].contains(needle),
        "expected one warning containing {needle:?}, got {warnings:#?}"
    );
}

fn reasons(res: &Resolution, id: u64) -> BTreeSet<Reason> {
    res.enabled[&r(id)].clone()
}

// Parents.

#[test]
fn a_parent_rule_is_brought_in() {
    let s = source(&[domain(1), rule(2, 1, ""), rule(3, 2, "")]);
    let res = enabled(&s, &[3], &[]);
    assert_eq!(ids(&res), [2, 3]);
    assert_eq!(reasons(&res, 2), BTreeSet::from([Reason::ParentOf(r(3))]));
    assert_eq!(reasons(&res, 3), BTreeSet::from([Reason::Direct]));
}

#[test]
fn a_domain_parent_brings_in_nothing() {
    let s = source(&[domain(1), rule(2, 1, "")]);
    assert_eq!(ids(&enabled(&s, &[2], &[])), [2]);
}

#[test]
fn the_parent_requirement_passes_through_a_one_of() {
    let s = source(&[
        domain(1),
        rule(2, 1, ""),
        one_of(3, 2),
        rule(4, 3, ""),
        rule(5, 3, ""),
    ]);
    let res = enabled(&s, &[4], &[]);
    assert_eq!(ids(&res), [2, 4]);
    assert_eq!(reasons(&res, 2), BTreeSet::from([Reason::ParentOf(r(4))]));
}

// Requirements.

#[test]
fn requirements_are_added_transitively() {
    let s = source(&[
        domain(1),
        rule(2, 1, ""),
        rule(3, 1, "requires = [2]"),
        rule(4, 1, "requires = [3]"),
    ]);
    let res = enabled(&s, &[4], &[]);
    assert_eq!(ids(&res), [2, 3, 4]);
    assert_eq!(reasons(&res, 2), BTreeSet::from([Reason::RequiredBy(r(3))]));
    assert_eq!(reasons(&res, 3), BTreeSet::from([Reason::RequiredBy(r(4))]));
    // Not brought in when nothing requires it.
    assert_eq!(ids(&enabled(&s, &[3], &[])), [2, 3]);
}

#[test]
fn a_required_one_of_needs_a_chosen_child() {
    let s = source(&[
        domain(1),
        one_of(2, 1),
        rule(3, 2, ""),
        rule(4, 2, ""),
        rule(5, 1, "requires = [2]"),
    ]);
    assert_eq!(ids(&enabled(&s, &[5, 4], &[])), [4, 5]);
    fails(
        &s,
        &[5],
        &[],
        1,
        "rule R5 (Rule 5) requires one-of R2 (One-of 2), and the rule set enables none of its children; add one of them to `rules`: \"R3\" (Rule 3), \"R4\" (Rule 4)",
    );
}

#[test]
fn a_required_one_of_is_met_by_a_child_brought_in_otherwise() {
    // 6 requires the one-of; 7 requires its child 3.
    let s = source(&[
        domain(1),
        one_of(2, 1),
        rule(3, 2, ""),
        rule(4, 2, ""),
        rule(6, 1, "requires = [2]"),
        rule(7, 1, "requires = [3]"),
    ]);
    assert_eq!(ids(&enabled(&s, &[6, 7], &[])), [3, 6, 7]);
}

// One-ofs.

#[test]
fn at_most_one_child_of_a_one_of() {
    let s = source(&[
        domain(1),
        one_of(2, 1),
        rule(3, 2, ""),
        rule(4, 2, ""),
        rule(5, 1, "requires = [4]"),
    ]);
    assert_eq!(ids(&enabled(&s, &[3], &[])), [3]);
    fails(
        &s,
        &[3, 4],
        &[],
        1,
        "at most one child of one-of R2 (One-of 2) may be enabled, and the rule set enables 2: R3 (Rule 3) (listed in `rules`), R4 (Rule 4) (listed in `rules`); keep one",
    );
    fails(&s, &[3, 5], &[], 1, "R4 (Rule 4) (required by R5)");
}

// Conflicts.

#[test]
fn conflicting_rules_are_not_enabled_together() {
    let s = source(&[
        domain(1),
        rule(2, 1, ""),
        rule(3, 1, "conflicts = [2]"),
        rule(4, 1, "requires = [2]"),
    ]);
    assert_eq!(ids(&enabled(&s, &[3], &[])), [3]);
    fails(
        &s,
        &[2, 3],
        &[],
        1,
        "rules R3 (Rule 3) (listed in `rules`) and R2 (Rule 2) (listed in `rules`) conflict (R3 says so)",
    );
    // Declared on 3 only; the conflict is found from either side, here
    // with 2 brought in by 4.
    fails(&s, &[4, 3], &[], 1, "R2 (Rule 2) (required by R4)");
}

// The constitution's revision.

#[test]
fn every_rule_accepts_the_revision() {
    let s = source(&[domain(1), edit(rule(2, 1, ""), ">=1", ">=2,<4")]);
    assert_eq!(
        ids(&{
            let (res, diags) = run(&s, &[2], &[], 3);
            assert!(diags.0.is_empty(), "{:#?}", diags.0);
            res
        }),
        [2]
    );
    fails(
        &s,
        &[2],
        &[],
        1,
        "rule R2 (Rule 2) is compatible with constitution revisions `>=2,<4`, and the rule set rests on revision 1",
    );
    fails(&s, &[2], &[], 4, "rests on revision 4");
}

#[test]
fn a_rule_brought_in_must_accept_the_revision_too() {
    let s = source(&[
        domain(1),
        edit(rule(2, 1, ""), ">=1", ">=2"),
        rule(3, 2, ""),
    ]);
    fails(&s, &[3], &[], 1, "rule R2 (Rule 2) is compatible");
}

// Packs.

fn pack_source() -> Sources {
    // Domain 1: rule 2, its child 3; one-of 4 with children 5 and 6, and 7
    // under 5; rule 8 superseded by 2.
    source(&[
        domain(1),
        rule(2, 1, ""),
        rule(3, 2, ""),
        one_of(4, 1),
        rule(5, 4, ""),
        rule(6, 4, ""),
        rule(7, 5, ""),
        edit(
            rule(8, 1, "superseded-by = 2"),
            "status = \"draft\"",
            "status = \"superseded\"",
        ),
        domain(9),
        rule(10, 9, ""),
    ])
}

#[test]
fn a_pack_enables_its_subtree_but_no_child_of_a_one_of_unchosen() {
    let s = pack_source();
    let res = enabled(&s, &[], &[1]);
    assert_eq!(ids(&res), [2, 3]);
    assert_eq!(reasons(&res, 3), BTreeSet::from([Reason::Pack(r(1))]));
}

#[test]
fn a_pack_goes_through_the_chosen_child_of_a_one_of() {
    let s = pack_source();
    let res = enabled(&s, &[5], &[1]);
    assert_eq!(ids(&res), [2, 3, 5, 7]);
    assert_eq!(
        reasons(&res, 5),
        BTreeSet::from([Reason::Direct, Reason::Pack(r(1)), Reason::ParentOf(r(7))])
    );
}

#[test]
fn a_pack_rooted_at_a_rule() {
    let s = pack_source();
    assert_eq!(ids(&enabled(&s, &[], &[2])), [2, 3]);
    assert_eq!(ids(&enabled(&s, &[], &[9])), [10]);
}

// What the rule set may name.

#[test]
fn rules_name_rules_that_exist() {
    let s = pack_source();
    fails(
        &s,
        &[42],
        &[],
        1,
        "`rules` names R42, which does not exist in source `R`",
    );
    fails(&s, &[], &[42], 1, "`packs` names R42, which does not exist");
    fails(
        &s,
        &[1],
        &[],
        1,
        "`rules` names domain R1 (Domain 1), which cannot be enabled on its own; to enable every rule under it, move it to `packs`",
    );
    fails(
        &s,
        &[4],
        &[],
        1,
        "`rules` names one-of R4 (One-of 4); a one-of is enabled by choosing one of its children: \"R5\" (Rule 5), \"R6\" (Rule 6)",
    );
}

#[test]
fn a_superseded_rule_is_a_warning_naming_its_successor() {
    let s = pack_source();
    warns(
        &s,
        &[8],
        "rule R8 (Rule 8) (listed in `rules`) is superseded by R2 (Rule 2); replace R8 with R2 in `rules` (`statute upgrade --replace-superseded` does it)",
    );
}

#[test]
fn a_rule_split_in_two_is_a_warning_naming_both() {
    let s = source(&[
        domain(1),
        rule(2, 1, ""),
        rule(3, 1, ""),
        edit(
            rule(8, 1, "superseded-by = [2, 3]"),
            "status = \"draft\"",
            "status = \"superseded\"",
        ),
    ]);
    warns(
        &s,
        &[8],
        "rule R8 (Rule 8) (listed in `rules`) is superseded by R2 (Rule 2), R3 (Rule 3); replace R8 in `rules` with those of R2, R3 that apply",
    );
    let (_, diags) = run(&s, &[], &[8], 1);
    let warnings: Vec<String> = diags.warnings().map(ToString::to_string).collect();
    assert_eq!(
        warnings,
        [
            "statute.toml: warning: pack R8 (Rule 8) enables nothing: its root is superseded by R2 (Rule 2), R3 (Rule 3); replace R8 in `packs` with those of R2, R3 that apply"
        ]
    );
}

#[test]
fn a_repealed_rule_is_a_warning() {
    let s = source(&[
        domain(1),
        edit(
            rule(2, 1, ""),
            "status = \"draft\"",
            "status = \"repealed\"",
        ),
    ]);
    warns(&s, &[2], "rule R2 (Rule 2) (listed in `rules`) is repealed");
    // Left out of a pack.
    assert!(ids(&enabled(&s, &[], &[1])).is_empty());
}

#[test]
fn a_superseded_rule_brought_in_names_what_brings_it_in() {
    let mut nodes = vec![
        domain(1),
        rule(2, 1, ""),
        edit(
            rule(8, 1, "superseded-by = 2"),
            "status = \"draft\"",
            "status = \"superseded\"",
        ),
    ];
    // Under the superseded rule: a requirement of it would draw a warning
    // of the source already.
    nodes.push(rule(9, 8, ""));
    let s = source(&nodes);
    warns(
        &s,
        &[9],
        "rule R8 (Rule 8) (parent of R9) is superseded by R2 (Rule 2); it stays as long as R9 (Rule 9) is enabled, which brings it in",
    );
    let (_, diags) = run(&s, &[9], &[], 1);
    assert!(
        !diags.0.iter().any(|d| d.message.contains("replace")),
        "{:#?}",
        diags.0
    );
}

#[test]
fn a_pack_whose_root_is_superseded_warns() {
    let s = pack_source();
    let (res, diags) = run(&s, &[], &[8], 1);
    assert!(res.enabled.is_empty());
    let warnings: Vec<String> = diags.warnings().map(ToString::to_string).collect();
    assert_eq!(
        warnings,
        [
            "statute.toml: warning: pack R8 (Rule 8) enables nothing: its root is superseded by R2 (Rule 2); replace R8 with R2 in `packs` (`statute upgrade --replace-superseded` does it)"
        ]
    );
}

#[test]
fn conflicting_rules_of_one_pack_are_listed_one_by_one() {
    let s = source(&[domain(1), rule(2, 1, ""), rule(3, 1, "conflicts = [2]")]);
    fails(
        &s,
        &[],
        &[1],
        1,
        "both are in pack R1, so in place of the pack, list in `rules` the rules of it you want",
    );
    // With one of them listed directly as well, the advice stays the same.
    fails(&s, &[2], &[1], 1, "both are in pack R1");
}
