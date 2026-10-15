//! The checks on rule files and on the relations between them. Each test
//! states a source that passes, then the same source with one change that
//! must fail (or warn), so a check that never fires shows up red.

use statute::diag::{Diagnostics, Severity};
use statute::node::original_hash;
use statute::source::Source;

mod common;
use common::{domain, edit, empty_one_of, one_of, option, rule, set_default};

fn load(nodes: &[(u64, String)]) -> Diagnostics {
    let files = nodes
        .iter()
        .map(|(id, text)| (format!("rules/{id}.toml"), text.clone().into_bytes()))
        .collect();
    Source::from_files(files).1
}

fn load_named(files: &[(&str, &str)]) -> Diagnostics {
    let files = files
        .iter()
        .map(|(name, text)| (format!("rules/{name}"), text.as_bytes().to_vec()))
        .collect();
    Source::from_files(files).1
}

fn lines(diags: &Diagnostics, severity: Severity) -> Vec<String> {
    diags
        .0
        .iter()
        .filter(|d| d.severity == severity)
        .map(ToString::to_string)
        .collect()
}

#[track_caller]
fn assert_clean(diags: &Diagnostics) {
    assert!(
        diags.0.is_empty(),
        "expected no findings, got {:#?}",
        diags.0
    );
}

/// Asserts exactly one error, in `file`, whose message contains `needle`.
#[track_caller]
fn assert_error(diags: &Diagnostics, file: &str, needle: &str) {
    let errors = lines(diags, Severity::Error);
    assert!(
        errors.len() == 1
            && errors[0].starts_with(&format!("rules/{file}: error: "))
            && errors[0].contains(needle),
        "expected one error in rules/{file} containing {needle:?}, got {errors:#?}"
    );
}

#[track_caller]
fn assert_warning(diags: &Diagnostics, file: &str, needle: &str) {
    assert!(!diags.has_errors(), "unexpected errors: {:#?}", diags.0);
    let warnings = lines(diags, Severity::Warning);
    assert!(
        warnings.len() == 1
            && warnings[0].starts_with(&format!("rules/{file}: warning: "))
            && warnings[0].contains(needle),
        "expected one warning in rules/{file} containing {needle:?}, got {warnings:#?}"
    );
}

// Fields and types of one file.

#[test]
fn each_kind_reads_cleanly() {
    assert_clean(&load(&[
        domain(1),
        one_of(2, 1, 3),
        option(3, 2, ""),
        option(4, 2, ""),
    ]));
}

#[test]
fn file_name_must_be_the_id() {
    let (_, text) = rule(5, 1, "");
    assert_clean(&load_named(&[("1.toml", &domain(1).1), ("5.toml", &text)]));
    assert_error(
        &load_named(&[("1.toml", &domain(1).1), ("6.toml", &text)]),
        "6.toml",
        "belongs in 5.toml",
    );
    assert_error(
        &load_named(&[("1.toml", &domain(1).1), ("05.toml", &text)]),
        "05.toml",
        "belongs in 5.toml",
    );
}

#[test]
fn ids_are_unique() {
    let (_, text) = rule(5, 1, "");
    let diags = load_named(&[
        ("1.toml", &domain(1).1),
        ("5.toml", &text),
        ("7.toml", &text),
    ]);
    let errors = lines(&diags, Severity::Error);
    assert_eq!(errors.len(), 2, "{errors:#?}");
    assert!(
        errors[1].contains("ID 5 is already used by rules/5.toml"),
        "{errors:#?}"
    );
}

#[test]
fn id_starts_at_one() {
    assert_error(
        &load_named(&[("0.toml", &domain(0).1)]),
        "0.toml",
        "`id` is 0",
    );
}

#[test]
fn slugs_are_well_formed_and_unique() {
    let a = rule(2, 1, "slug = \"run-checks\"");
    let b = rule(3, 1, "slug = \"run-tests\"");
    assert_clean(&load(&[domain(1), a.clone(), b.clone()]));
    assert_error(
        &load(&[domain(1), a.clone(), edit(b, "run-tests", "run-checks")]),
        "3.toml",
        "slug `run-checks` is already used by 2.toml",
    );
    assert_error(
        &load(&[domain(1), edit(a, "run-checks", "Run_Checks")]),
        "2.toml",
        "lowercase words joined by hyphens",
    );
}

#[test]
fn unknown_fields_and_wrong_types_are_errors() {
    assert_error(
        &load(&[domain(1), rule(2, 1, "scope = \"all\"")]),
        "2.toml",
        "unknown field `scope`",
    );
    assert_error(
        &load(&[
            domain(1),
            edit(rule(2, 1, ""), "revision = 1", "revision = \"1\""),
        ]),
        "2.toml",
        "revision",
    );
    assert_error(
        &load(&[
            domain(1),
            edit(rule(2, 1, ""), "status = \"draft\"", "status = \"done\""),
        ]),
        "2.toml",
        "unknown variant `done`",
    );
}

#[test]
fn domains_nest_at_any_depth_and_rules_and_one_ofs_need_a_parent() {
    let under = |id, parent: &str| {
        edit(
            domain(id),
            "lang = \"en\"",
            &format!("lang = \"en\"\nparent = {parent}"),
        )
    };
    // A subdomain, with a rule and a one-of below it.
    assert_clean(&load(&[
        domain(1),
        under(2, "1"),
        rule(3, 2, ""),
        one_of(4, 2, 5),
        option(5, 4, ""),
    ]));
    // A subdomain under a subdomain, and one below that.
    assert_clean(&load(&[
        domain(1),
        under(2, "1"),
        under(3, "2"),
        under(4, "3"),
        rule(5, 4, ""),
    ]));
    assert_error(
        &load(&[domain(1), rule(2, 1, ""), under(3, "2")]),
        "3.toml",
        "parent 2 is a rule; a domain's parent is a domain, or a one-of of which it is an option",
    );
    // A domain as the option of a one-of, in a domain or in a subdomain.
    assert_clean(&load(&[
        domain(1),
        one_of(2, 1, 3),
        edit(under(3, "2"), "default = \"off\"\n", ""),
        rule(4, 3, ""),
    ]));
    assert_clean(&load(&[
        domain(1),
        under(2, "1"),
        one_of(3, 2, 4),
        edit(under(4, "3"), "default = \"off\"\n", ""),
        rule(5, 4, ""),
    ]));
    // A rule's or a domain's parent may be in another source, a one-of's
    // not; which domains may have one is the rule set's to check, with the
    // other source.
    assert_clean(&load(&[
        domain(1),
        edit(rule(2, 1, ""), "parent = 1", "parent = \"team:7\""),
    ]));
    assert_clean(&load(&[domain(1), under(2, "\"team:7\"")]));
    assert_error(
        &load(&[
            domain(1),
            edit(one_of(2, 1, 3), "parent = 1", "parent = \"team:7\""),
            option(3, 2, ""),
        ]),
        "2.toml",
        "a one-of's `parent` is a node of the same source, written as its number; only a rule's or a domain's may name a domain of another source",
    );
    assert_error(
        &load(&[domain(1), edit(rule(2, 1, ""), "parent = 1\n", "")]),
        "2.toml",
        "a rule needs a `parent`",
    );
    assert_error(
        &load(&[
            domain(1),
            edit(one_of(2, 1, 3), "parent = 1\n", ""),
            option(3, 2, ""),
        ]),
        "2.toml",
        "a one-of needs a `parent`",
    );
}

#[test]
fn domains_and_one_ofs_have_no_rule_fields() {
    assert_error(
        &load(&[
            domain(1),
            rule(2, 1, ""),
            edit(one_of(3, 1, 4), "parent = 1", "parent = 1\nrequires = [2]"),
            option(4, 3, ""),
        ]),
        "3.toml",
        "remove requires",
    );
    assert_error(
        &load(&[edit(
            domain(1),
            "lang = \"en\"",
            "lang = \"en\"\ncheck = \"none\"",
        )]),
        "1.toml",
        "remove check",
    );
}

#[test]
fn a_rule_has_its_required_fields() {
    let r = rule(2, 1, "");
    for field in [
        "authority = [\"simplicity\"]\n",
        "constitution = \">=1\"\n",
        "level = \"project\"\n",
        "check = \"none\"\n",
    ] {
        let name = field.split(' ').next().unwrap();
        assert_error(
            &load(&[domain(1), edit(r.clone(), field, "")]),
            "2.toml",
            &format!("a rule needs {name}"),
        );
    }
    let without_text = (2, r.1.split("[text.statement]").next().unwrap().to_owned());
    assert_error(
        &load(&[domain(1), without_text]),
        "2.toml",
        "a rule needs [text.statement]",
    );
    assert_error(
        &load(&[
            domain(1),
            edit(
                r,
                "authority = [\"simplicity\"]",
                "authority = [\"constitution:simplicity\"]",
            ),
        ]),
        "2.toml",
        "without a namespace",
    );
}

#[test]
fn command_checks_need_run_and_tools() {
    let command = "check = \"command\"\nrun = \"uv run check\"\ntools = [\"uv\"]";
    let r = edit(rule(2, 1, ""), "check = \"none\"", command);
    assert_clean(&load(&[domain(1), r.clone()]));
    assert_error(
        &load(&[domain(1), edit(r.clone(), "tools = [\"uv\"]", "")]),
        "2.toml",
        "needs `tools`",
    );
    assert_error(
        &load(&[
            domain(1),
            edit(r, "check = \"command\"", "check = \"builtin\""),
        ]),
        "2.toml",
        "only with `check = \"command\"`",
    );
}

#[test]
fn constitution_is_a_version_specifier() {
    let r = rule(2, 1, "");
    assert_clean(&load(&[domain(1), edit(r.clone(), ">=1", ">=1,<4")]));
    assert_error(
        &load(&[domain(1), edit(r, ">=1", "1 or later")]),
        "2.toml",
        "`constitution`",
    );
}

#[test]
fn superseded_names_its_successor() {
    let old = rule(2, 1, "superseded-by = 3");
    let old = edit(old, "status = \"draft\"", "status = \"superseded\"");
    assert_clean(&load(&[domain(1), old.clone(), rule(3, 1, "")]));
    assert_error(
        &load(&[
            domain(1),
            edit(old.clone(), "superseded-by = 3", ""),
            rule(3, 1, ""),
        ]),
        "2.toml",
        "needs `superseded-by`",
    );
    assert_error(
        &load(&[
            domain(1),
            edit(
                old.clone(),
                "status = \"superseded\"",
                "status = \"active\"",
            ),
            rule(3, 1, ""),
        ]),
        "2.toml",
        "only when `status = \"superseded\"`",
    );
    assert_error(
        &load(&[domain(1), old.clone()]),
        "2.toml",
        "superseded-by 3 does not exist",
    );
    assert_error(
        &load(&[domain(1), old, one_of(3, 1, 4), option(4, 3, "")]),
        "2.toml",
        "superseded-by 3 is a one-of",
    );
}

#[test]
fn superseded_by_a_list() {
    let split = |by: &str| {
        edit(
            rule(2, 1, &format!("superseded-by = {by}")),
            "status = \"draft\"",
            "status = \"superseded\"",
        )
    };
    let with = |by: &str| load(&[domain(1), split(by), rule(3, 1, ""), rule(4, 1, "")]);
    assert_clean(&with("[3, 4]"));
    assert_clean(&with("[3]"));
    assert_error(&with("[]"), "2.toml", "`superseded-by` is empty");
    assert_error(&with("[3, 3]"), "2.toml", "`superseded-by` lists 3 twice");
    assert_error(
        &with("[3, 2]"),
        "2.toml",
        "`superseded-by` names the rule itself",
    );
    assert_error(&with("[3, 5]"), "2.toml", "superseded-by 5 does not exist");
    // Requiring the split rule names both successors.
    let mut nodes = vec![domain(1), split("[3, 4]"), rule(3, 1, ""), rule(4, 1, "")];
    nodes.push(rule(5, 1, "requires = [2]"));
    assert_warning(
        &load(&nodes),
        "5.toml",
        "`requires` names rule 2, which is superseded by 3, 4; require those of them that apply instead",
    );
}

// Languages and translations.

fn translated(hash: &str) -> (u64, String) {
    let r = rule(2, 1, "");
    let r = edit(r, "en = \"Rule 2\"", "en = \"Rule 2\"\nzh = \"规定 2\"");
    let r = edit(
        r,
        "en = \"Do the thing 2.\"",
        "en = \"Do the thing 2.\"\nzh = \"做事 2。\"",
    );
    let r = edit(r, "Because.\n'''", "Because.\n'''\nzh = \"因为。\"");
    let r = edit(
        r,
        "en = \"Violated when the thing is not done.\"",
        "en = \"Violated when the thing is not done.\"\nzh = \"没做即为违反。\"",
    );
    (r.0, format!("{}\n[translations]\nzh = \"{hash}\"\n", r.1))
}

fn current_hash() -> String {
    // The values as the TOML parser returns them: the multi-line reason
    // keeps its trailing newline.
    original_hash(&[
        "Rule 2",
        "Do the thing 2.",
        "Because.\n",
        "Violated when the thing is not done.",
    ])
}

#[test]
fn a_translation_records_the_hash_of_its_original() {
    assert_clean(&load(&[domain(1), translated(&current_hash())]));
    let stale = "0".repeat(64);
    assert_error(
        &load(&[domain(1), translated(&stale)]),
        "2.toml",
        &format!("set `zh = \"{}\"`", current_hash()),
    );
}

#[test]
fn a_translation_gives_every_text() {
    let partial = edit(translated(&current_hash()), "zh = \"因为。\"\n", "");
    assert_error(
        &load(&[domain(1), partial]),
        "2.toml",
        "missing in [text.reason]",
    );
    let unrecorded = edit(translated(&current_hash()), "[translations]\n", "[other]\n");
    let diags = load(&[domain(1), unrecorded]);
    assert!(diags.has_errors());
    let no_hash = (
        2,
        translated(&current_hash())
            .1
            .split("\n[translations]")
            .next()
            .unwrap()
            .to_owned(),
    );
    assert_error(
        &load(&[domain(1), no_hash]),
        "2.toml",
        "needs `zh = \"<hash>\"` in [translations]",
    );
}

#[test]
fn the_original_language_is_required() {
    let r = edit(rule(2, 1, ""), "lang = \"en\"", "lang = \"zh\"");
    let diags = load(&[domain(1), r]);
    assert!(
        lines(&diags, Severity::Error)
            .iter()
            .any(|e| e.contains("[title] has no `zh`")),
        "{:#?}",
        diags.0
    );
    let only_original = edit(
        translated(&current_hash()),
        "[translations]\nzh",
        "[translations]\nen",
    );
    let diags = load(&[domain(1), only_original]);
    assert!(
        lines(&diags, Severity::Error)
            .iter()
            .any(|e| e.contains("has `en`, the original language")),
        "{:#?}",
        diags.0
    );
}

#[test]
fn a_domain_translation_hashes_the_title_alone() {
    let hash = original_hash(&["Domain 1"]);
    let d = edit(
        domain(1),
        "en = \"Domain 1\"",
        "en = \"Domain 1\"\nzh = \"领域 1\"",
    );
    let d = (1, format!("{}\n[translations]\nzh = \"{hash}\"\n", d.1));
    assert_clean(&load(std::slice::from_ref(&d)));
    assert_error(
        &load(&[edit(d, "Domain 1\"\nzh", "Domain one\"\nzh")]),
        "1.toml",
        "follows another original",
    );
}

// References.

#[test]
fn references_must_exist_and_have_the_right_kind() {
    assert_clean(&load(&[
        domain(1),
        one_of(2, 1, 5),
        rule(3, 1, "requires = [2]"),
        rule(4, 1, "conflicts = [3]"),
        option(5, 2, ""),
    ]));
    assert_error(
        &load(&[domain(1), rule(3, 9, "")]),
        "3.toml",
        "parent 9 does not exist",
    );
    assert_error(
        &load(&[domain(1), rule(3, 1, "requires = [9]")]),
        "3.toml",
        "names 9, which does not exist",
    );
    assert_error(
        &load(&[domain(1), domain(2), rule(3, 1, "requires = [2]")]),
        "3.toml",
        "a domain cannot be required",
    );
    assert_error(
        &load(&[
            domain(1),
            one_of(2, 1, 4),
            rule(3, 1, "conflicts = [2]"),
            option(4, 2, ""),
        ]),
        "3.toml",
        "can conflict only with a rule",
    );
}

#[test]
fn references_to_other_sources_are_left_to_the_rule_set() {
    // A source read alone cannot tell whether `team:7` exists; the rule
    // set that names `team` does (`Sources::check`).
    for r in ["\"team:7\"", "\"R7\""] {
        let diags = load(&[domain(1), rule(3, 1, &format!("requires = [{r}]"))]);
        assert!(diags.0.is_empty(), "{r}: {:#?}", diags.0);
    }
    assert_error(
        &load(&[domain(1), rule(3, 1, "requires = [\"team-7\"]")]),
        "3.toml",
        "is not a reference",
    );
    // A first-party rule is `R7`; the former `ac:7` is no reference.
    assert_error(
        &load(&[domain(1), rule(3, 1, "requires = [\"ac:7\"]")]),
        "3.toml",
        "`\"R<id>\"` for a first-party rule",
    );
}

#[test]
fn a_rule_does_not_name_itself_or_repeat_a_reference() {
    assert_error(
        &load(&[domain(1), rule(3, 1, "requires = [3]")]),
        "3.toml",
        "`requires` names the rule itself",
    );
    assert_error(
        &load(&[domain(1), rule(2, 1, ""), rule(3, 1, "conflicts = [2, 2]")]),
        "3.toml",
        "`conflicts` lists 2 twice",
    );
}

// Cycles.

#[test]
fn the_parents_form_no_cycle() {
    let under = |id, parent: u64| {
        edit(
            domain(id),
            "lang = \"en\"",
            &format!("lang = \"en\"\nparent = {parent}"),
        )
    };
    assert_clean(&load(&[domain(1), under(2, 1)]));
    // Each of the two is a subdomain under a subdomain as well.
    let errors = lines(&load(&[under(2, 3), under(3, 2)]), Severity::Error);
    assert!(
        errors.contains(&"rules/2.toml: error: the parents form a cycle: 2 -> 3 -> 2".to_owned()),
        "{errors:#?}"
    );
}

#[test]
fn the_requirements_form_no_cycle() {
    assert_clean(&load(&[
        domain(1),
        rule(2, 1, "requires = [3]"),
        rule(3, 1, ""),
        rule(4, 1, "requires = [2]"),
    ]));
    assert_error(
        &load(&[
            domain(1),
            rule(2, 1, "requires = [3]"),
            rule(3, 1, "requires = [4]"),
            rule(4, 1, "requires = [2]"),
        ]),
        "2.toml",
        "the requirements form a cycle",
    );
}

// Redundant requirements.

#[test]
fn a_requirement_that_follows_from_another_is_not_listed() {
    let base = [domain(1), rule(2, 1, ""), rule(3, 1, "requires = [2]")];
    let mut ok = base.to_vec();
    ok.push(rule(4, 1, "requires = [3]"));
    assert_clean(&load(&ok));
    let mut redundant = base.to_vec();
    redundant.push(rule(4, 1, "requires = [3, 2]"));
    assert_error(
        &load(&redundant),
        "4.toml",
        "lists 2 needlessly: it already follows from rule 3",
    );
}

// Conflicts.

#[test]
fn siblings_may_conflict() {
    assert_clean(&load(&[
        domain(1),
        rule(2, 1, ""),
        rule(3, 1, "conflicts = [2]"),
    ]));
}

#[test]
fn a_rule_does_not_conflict_with_what_it_brings_in() {
    assert_error(
        &load(&[
            domain(1),
            rule(2, 1, ""),
            rule(3, 1, "requires = [2]\nconflicts = [2]"),
        ]),
        "3.toml",
        "conflicts with 2, which enabling 3 brings in",
    );
    // Through two requirements: 4 requires 3, 3 requires 2.
    assert_error(
        &load(&[
            domain(1),
            rule(2, 1, ""),
            rule(3, 1, "requires = [2]"),
            rule(4, 1, "requires = [3]\nconflicts = [2]"),
        ]),
        "4.toml",
        "conflicts with 2, which enabling 4 brings in",
    );
    // The other way: 3 requires 2, and 2 declares the conflict.
    assert_error(
        &load(&[
            domain(1),
            rule(2, 1, "conflicts = [3]"),
            rule(3, 1, "requires = [2]"),
        ]),
        "2.toml",
        "conflicts with 3, which brings in 2",
    );
}

#[test]
fn a_conflict_written_on_both_sides_is_a_warning() {
    assert_warning(
        &load(&[
            domain(1),
            rule(2, 1, "conflicts = [3]"),
            rule(3, 1, "conflicts = [2]"),
        ]),
        "2.toml",
        "written on both sides",
    );
}

// One-ofs and requirements across subtrees.

#[test]
fn a_rule_that_needs_two_children_of_a_one_of_is_an_error() {
    let base = [
        domain(1),
        one_of(2, 1, 3),
        option(3, 2, ""),
        option(4, 2, ""),
    ];
    let mut ok = base.to_vec();
    ok.push(rule(5, 1, "requires = [3]"));
    assert_clean(&load(&ok));
    let mut both = base.to_vec();
    both.push(rule(5, 1, "requires = [3, 4]"));
    assert_error(
        &load(&both),
        "5.toml",
        "can never be enabled: it brings in 3 and 4, children of one-of 2",
    );
    // A child that requires its sibling.
    let sibling = [
        domain(1),
        one_of(2, 1, 3),
        option(3, 2, "requires = [4]"),
        option(4, 2, ""),
    ];
    assert_error(
        &load(&sibling),
        "3.toml",
        "brings in 3 and 4, children of one-of 2",
    );
}

#[test]
fn a_requirement_across_domains_should_name_a_one_of() {
    let base = [domain(1), domain(2), one_of(3, 2, 4), option(4, 3, "")];
    let mut ok = base.to_vec();
    ok.push(rule(5, 1, "requires = [3]"));
    assert_clean(&load(&ok));
    let mut crossing = base.to_vec();
    crossing.push(rule(5, 1, "requires = [4]"));
    assert_warning(
        &load(&crossing),
        "5.toml",
        "requires rule 4, under another domain (2)",
    );
}

#[test]
fn files_other_than_toml_are_ignored_when_loading_a_directory() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("source-load");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("1.toml"), domain(1).1).unwrap();
    std::fs::write(dir.join("2.toml"), rule(2, 1, "").1).unwrap();
    std::fs::write(dir.join("retired-ids.txt"), "old-id\n").unwrap();
    let (source, diags) = Source::load(&dir, "rules");
    assert_clean(&diags);
    assert_eq!(source.nodes.keys().copied().collect::<Vec<_>>(), [1, 2]);
}

#[test]
fn a_required_one_of_has_children() {
    assert_clean(&load(&[
        domain(1),
        one_of(2, 1, 3),
        option(3, 2, ""),
        rule(4, 1, "requires = [2]"),
    ]));
    assert_error(
        &load(&[domain(1), empty_one_of(2, 1), rule(4, 1, "requires = [2]")]),
        "4.toml",
        "names one-of 2, which has no children yet",
    );
}

#[test]
fn only_a_rule_is_superseded() {
    let superseded = edit(
        one_of(2, 1, 3),
        "status = \"active\"",
        "status = \"superseded\"",
    );
    assert_error(
        &load(&[domain(1), superseded, option(3, 2, "")]),
        "2.toml",
        "only a rule can be superseded; a one-of cannot",
    );
}

#[test]
fn a_conflict_on_both_sides_with_a_rule_brought_in_is_reported_once() {
    let diags = load(&[
        domain(1),
        rule(11, 1, "conflicts = [12]"),
        rule(12, 11, "conflicts = [11]"),
    ]);
    assert_eq!(lines(&diags, Severity::Error).len(), 1, "{:#?}", diags.0);
}

#[test]
fn a_one_of_clash_is_reported_on_the_rule_that_makes_it() {
    // 5 brings in both children; 6 requires 5 and inherits the clash.
    let diags = load(&[
        domain(1),
        one_of(2, 1, 3),
        option(3, 2, ""),
        option(4, 2, ""),
        rule(5, 1, "requires = [3, 4]"),
        rule(6, 1, "requires = [5]"),
    ]);
    assert_error(&diags, "5.toml", "rule 5 can never be enabled");
}

#[test]
fn crlf_line_ends_hash_as_lf() {
    let lf = translated(&current_hash());
    assert_clean(&load(&[domain(1), lf.clone()]));
    let crlf = (lf.0, lf.1.replace('\n', "\r\n"));
    assert!(crlf.1.contains("Because.\r\n'''"));
    assert_clean(&load(&[domain(1), crlf]));
}

#[test]
fn an_escaped_crlf_is_not_a_line_end() {
    // `\r\n` written as an escape in a basic string keeps its CR, as
    // Python's tomllib does: `tomllib.loads('a = "Violated\\r\\nwhen."')`
    // gives 'Violated\r\nwhen.', and the SHA-256 of the four values as
    // tomllib returns them, joined with NUL, is the hex below.
    assert_eq!(
        original_hash(&[
            "Rule 2",
            "Do the thing 2.",
            "Because.\n",
            "Violated\r\nwhen."
        ]),
        "bb4d66ca9cc3adecba159bd59b6d4eda2621966805e93ce677509adc87c4c0ec"
    );
    let escaped = edit(
        translated(&original_hash(&[
            "Rule 2",
            "Do the thing 2.",
            "Because.\n",
            "Violated\r\nwhen.",
        ])),
        "en = \"Violated when the thing is not done.\"",
        "en = \"Violated\\r\\nwhen.\"",
    );
    assert_clean(&load(&[domain(1), escaped.clone()]));
    let as_lf = edit(
        escaped,
        &original_hash(&[
            "Rule 2",
            "Do the thing 2.",
            "Because.\n",
            "Violated\r\nwhen.",
        ]),
        &original_hash(&["Rule 2", "Do the thing 2.", "Because.\n", "Violated\nwhen."]),
    );
    assert_error(
        &load(&[domain(1), as_lf]),
        "2.toml",
        "follows another original",
    );
}

#[test]
fn a_title_is_one_line() {
    // A multi-line string whose one line ends in a newline is still one
    // line.
    let r = edit(rule(2, 1, ""), "en = \"Rule 2\"", "en = '''\nRule 2\n'''");
    assert_clean(&load(&[domain(1), r]));
    let r = edit(
        rule(2, 1, ""),
        "en = \"Rule 2\"",
        "en = \"Rule 2\\nand more\"",
    );
    assert_error(
        &load(&[domain(1), r]),
        "2.toml",
        "[title] `en` must be one line",
    );
}

#[test]
fn requiring_a_superseded_or_repealed_rule_is_a_warning() {
    let superseded = edit(
        rule(2, 1, "superseded-by = 3"),
        "status = \"draft\"",
        "status = \"superseded\"",
    );
    let repealed = edit(
        rule(4, 1, ""),
        "status = \"draft\"",
        "status = \"repealed\"",
    );
    let base = [domain(1), superseded, rule(3, 1, ""), repealed];
    let mut ok = base.to_vec();
    ok.push(rule(5, 1, "requires = [3]"));
    assert_clean(&load(&ok));
    let mut to_superseded = base.to_vec();
    to_superseded.push(rule(5, 1, "requires = [2]"));
    assert_warning(
        &load(&to_superseded),
        "5.toml",
        "`requires` names rule 2, which is superseded by 3; require 3 instead",
    );
    let mut to_repealed = base.to_vec();
    to_repealed.push(rule(5, 1, "requires = [4]"));
    assert_warning(
        &load(&to_repealed),
        "5.toml",
        "`requires` names rule 4, which is repealed, with nothing in its place",
    );
}

#[test]
fn a_rule_no_longer_in_force_may_require_one_no_longer_in_force() {
    let retire = |id: u64, status: &str, extra: &str| {
        edit(
            rule(id, 1, extra),
            "status = \"draft\"",
            &format!("status = \"{status}\""),
        )
    };
    // Rule 2 is superseded by 3; rule 5, superseded by 6 or repealed, still
    // requires 2.
    let base = [
        domain(1),
        retire(2, "superseded", "superseded-by = 3"),
        rule(3, 1, ""),
        rule(6, 1, ""),
    ];
    for retired in [
        retire(5, "superseded", "superseded-by = 6\nrequires = [2]"),
        retire(5, "repealed", "requires = [2]"),
    ] {
        let mut nodes = base.to_vec();
        nodes.push(retired);
        assert_clean(&load(&nodes));
    }
}

// The parent tree without a rule under a rule.

#[test]
fn a_rules_parent_is_a_domain_or_a_one_of() {
    assert_clean(&load(&[
        domain(1),
        rule(2, 1, ""),
        one_of(3, 1, 4),
        option(4, 3, ""),
    ]));
    assert_error(
        &load(&[domain(1), rule(2, 1, ""), rule(3, 2, "")]),
        "3.toml",
        "parent 2 is a rule; a rule's parent is a domain or a one-of, and the rule it refines is written in `requires`",
    );
    assert_error(
        &load(&[domain(1), rule(2, 1, ""), one_of(3, 2, 4), option(4, 3, "")]),
        "3.toml",
        "parent 2 is a rule; a one-of's parent is a domain",
    );
    assert_error(
        &load(&[
            domain(1),
            one_of(2, 1, 3),
            option(3, 2, ""),
            one_of(4, 2, 5),
            option(5, 4, ""),
        ]),
        "4.toml",
        "parent 2 is a one-of; a one-of's parent is a domain",
    );
}

#[test]
fn a_one_ofs_options_are_all_rules_or_all_domains() {
    let domain_option = edit(domain(4), "default = \"off\"", "parent = 2");
    assert_clean(&load(&[
        domain(1),
        one_of(2, 1, 3),
        option(3, 2, ""),
        option(5, 2, ""),
    ]));
    assert_error(
        &load(&[domain(1), one_of(2, 1, 3), option(3, 2, ""), domain_option]),
        "4.toml",
        "parent 2 is a one-of whose options are rules (3); the options of a one-of are all rules or all domains",
    );
}

#[test]
fn a_one_ofs_default_names_one_of_its_children() {
    let with = |default: &str| set_default(one_of(2, 1, 3), default);
    let nodes = |one: (u64, String)| {
        [
            domain(1),
            one,
            option(3, 2, ""),
            option(4, 2, ""),
            rule(5, 1, ""),
        ]
    };
    assert_clean(&load(&nodes(with("4"))));
    assert_error(
        &load(&nodes(with("5"))),
        "2.toml",
        "`default = 5` names rule 5, which is not a child of this one-of",
    );
    assert_error(
        &load(&nodes(with("\"on\""))),
        "2.toml",
        "a one-of's `default` is the ID of one of its children",
    );
    // A child has no `default` of its own.
    let mut child = nodes(with("4"));
    child[2] = option(3, 2, "default = \"on\"");
    assert_error(
        &load(&child),
        "3.toml",
        "a child of a one-of has no `default`: one-of 2 names its default child",
    );
}

// Default switches and uses.

#[test]
fn a_domains_and_a_rules_default_is_on_or_off() {
    assert_clean(&load(&[domain(1), set_default(rule(2, 1, ""), "\"on\"")]));
    assert_error(
        &load(&[domain(1), set_default(rule(2, 1, ""), "\"yes\"")]),
        "2.toml",
        "`default = \"yes\"` must be \"on\" or \"off\"",
    );
    assert_error(
        &load(&[domain(1), set_default(rule(2, 1, ""), "3")]),
        "2.toml",
        "a rule's `default` is \"on\" or \"off\"",
    );
}

#[test]
fn every_domain_and_rule_says_whether_it_is_on_by_default() {
    let without = |node| edit(node, "default = \"off\"\n", "");
    assert_error(
        &load(&[without(domain(1))]),
        "1.toml",
        "a domain says whether it is on by default: `default = \"on\"` or `default = \"off\"`",
    );
    assert_error(
        &load(&[domain(1), without(rule(2, 1, ""))]),
        "2.toml",
        "a rule says whether it is on by default: `default = \"on\"` or `default = \"off\"`",
    );
    // A superseded rule says so too.
    let old = edit(
        rule(2, 1, "superseded-by = 3"),
        "status = \"draft\"",
        "status = \"superseded\"",
    );
    assert_error(
        &load(&[domain(1), without(old), rule(3, 1, "")]),
        "2.toml",
        "a rule says whether it is on by default",
    );
    // A rule that overrides another stands in its place and has none.
    let over = edit(rule(2, 1, "overrides = \"team:8\""), "parent = 1\n", "");
    assert_clean(&load(&[domain(1), without(over.clone())]));
    assert_error(
        &load(&[domain(1), over]),
        "2.toml",
        "a rule with `overrides` has no `default`: it stands in the place of the rule it overrides; remove `default`",
    );
}

#[test]
fn a_one_of_with_children_names_its_default() {
    assert_error(
        &load(&[
            domain(1),
            edit(one_of(2, 1, 3), "default = 3\n", ""),
            option(3, 2, ""),
        ]),
        "2.toml",
        "a one-of names its default child, `default = <the ID of one of its children>`",
    );
    assert_clean(&load(&[domain(1), empty_one_of(2, 1)]));
}

#[test]
fn uses_lists_instructions_review_or_both() {
    for uses in [
        "[\"instructions\"]",
        "[\"review\"]",
        "[\"review\", \"instructions\"]",
    ] {
        assert_clean(&load(&[domain(1), rule(2, 1, &format!("uses = {uses}"))]));
    }
    for (uses, needle) in [
        ("[]", "`uses` is empty"),
        ("[\"agents\"]", "`uses` lists `agents`"),
        ("[\"review\", \"review\"]", "`uses` lists `review` twice"),
    ] {
        assert_error(
            &load(&[domain(1), rule(2, 1, &format!("uses = {uses}"))]),
            "2.toml",
            needle,
        );
    }
    assert_error(
        &load(&[
            edit(
                domain(1),
                "lang = \"en\"",
                "lang = \"en\"\nuses = [\"review\"]",
            ),
            rule(2, 1, ""),
        ]),
        "1.toml",
        "remove uses",
    );
}

#[test]
fn check_none_and_not_its_older_name_review() {
    assert_clean(&load(&[domain(1), rule(2, 1, "")]));
    assert_error(
        &load(&[
            domain(1),
            edit(rule(2, 1, ""), "check = \"none\"", "check = \"review\""),
        ]),
        "2.toml",
        "`check = \"review\"` is the older name of `check = \"none\"`; write that",
    );
}

#[test]
fn the_defaults_of_a_source_resolve_on_their_own() {
    let switched = |id, default: &str| set_default(domain(id), &format!("\"{default}\""));
    let on = |id, parent, extra: &str| set_default(rule(id, parent, extra), "\"on\"");
    let off_rule = |id, parent| rule(id, parent, "");
    // A rule the defaults enable requiring one they enable; one in a
    // domain that is off is not checked.
    assert_clean(&load(&[
        switched(1, "on"),
        on(2, 1, ""),
        on(3, 1, "requires = [2]"),
        switched(4, "off"),
        off_rule(5, 4),
        on(7, 4, "requires = [5]"),
    ]));
    assert_error(
        &load(&[
            switched(1, "on"),
            off_rule(2, 1),
            on(3, 1, "requires = [2]"),
        ]),
        "3.toml",
        "rule 3 is enabled by default and requires rule 2, which is not; turn 2 on by default, or this rule off",
    );
    // A rule in a domain that is off is not enabled by default.
    assert_error(
        &load(&[
            switched(1, "on"),
            switched(4, "off"),
            on(5, 4, ""),
            on(3, 1, "requires = [5]"),
        ]),
        "3.toml",
        "rule 3 is enabled by default and requires rule 5, which is not",
    );
    // A one-of's default child is enabled by default when its domain is.
    let one_of_default = |child: u64| one_of(8, 1, child);
    assert_error(
        &load(&[
            switched(1, "on"),
            off_rule(2, 1),
            one_of_default(9),
            option(9, 8, "requires = [2]"),
            option(10, 8, ""),
        ]),
        "9.toml",
        "rule 9 is enabled by default and requires rule 2, which is not",
    );
    // In a domain that is off, nothing is checked; a required one-of
    // counts as met, and only its default child brings its requirements.
    assert_clean(&load(&[
        domain(1),
        off_rule(2, 1),
        on(3, 1, "requires = [2]"),
    ]));
    assert_clean(&load(&[
        switched(1, "on"),
        one_of(11, 1, 14),
        off_rule(12, 1),
        option(13, 11, "requires = [12]"),
        option(14, 11, ""),
        on(3, 1, "requires = [11]"),
    ]));
}

// Citations.

#[test]
fn every_language_cites_what_the_original_cites() {
    let hash = original_hash(&[
        "Rule 3",
        "Do the thing 3, after {@rule:2}.",
        "Because.\n",
        "Violated when the thing is not done.",
    ]);
    let translated = |zh: &str| {
        let r = edit(
            rule(3, 1, ""),
            "Do the thing 3.",
            "Do the thing 3, after {@rule:2}.",
        );
        let r = edit(r, "en = \"Rule 3\"", "en = \"Rule 3\"\nzh = \"规定 3\"");
        let r = edit(
            r,
            "en = \"Do the thing 3, after {@rule:2}.\"",
            &format!("en = \"Do the thing 3, after {{@rule:2}}.\"\nzh = \"{zh}\""),
        );
        let r = edit(r, "Because.\n'''", "Because.\n'''\nzh = \"因为。\"");
        let r = edit(
            r,
            "en = \"Violated when the thing is not done.\"",
            "en = \"Violated when the thing is not done.\"\nzh = \"没做即为违反。\"",
        );
        (3, format!("{}\n[translations]\nzh = \"{hash}\"\n", r.1))
    };
    assert_clean(&load(&[
        domain(1),
        rule(2, 1, ""),
        translated("在{@rule:2}之后做事 3。"),
    ]));
    assert_error(
        &load(&[domain(1), rule(2, 1, ""), translated("做事 3。")]),
        "3.toml",
        "[text.statement] `en` cites `{@rule:2}`, and `zh` does not",
    );
    // A citation only the translation has is checked as well.
    let extra = load(&[
        domain(1),
        rule(2, 1, ""),
        translated("在{@rule:2}和{@rule:9}之后做事 3。"),
    ]);
    assert_eq!(
        lines(&extra, Severity::Error),
        [
            "rules/3.toml: error: [text.statement] `zh` cites `{@rule:9}`, and `en` does not",
            "rules/3.toml: error: [text.statement] cites `{@rule:9}`, which does not exist in this source",
        ]
    );
}

#[test]
fn a_citation_names_a_rule_in_force_with_its_namespace() {
    let citing = |what: &str| {
        edit(
            rule(3, 1, ""),
            "Do the thing 3.",
            &format!("Do the thing 3, after {what}."),
        )
    };
    assert_clean(&load(&[domain(1), rule(2, 1, ""), citing("{@rule:2}")]));
    assert_error(
        &load(&[domain(1), rule(2, 1, ""), citing("{@rule:9}")]),
        "3.toml",
        "[text.statement] cites `{@rule:9}`, which does not exist in this source",
    );
    let repealed = edit(
        rule(2, 1, ""),
        "status = \"draft\"",
        "status = \"repealed\"",
    );
    assert_error(
        &load(&[domain(1), repealed, citing("{@rule:2}")]),
        "3.toml",
        "[text.statement] cites `{@rule:2}`, which is repealed",
    );
    assert_error(
        &load(&[domain(1), rule(2, 1, ""), citing("{@simplicity}")]),
        "3.toml",
        "[text.statement] cites `{@simplicity}` without `rule:` or `constitution:`",
    );
    assert_error(
        &load(&[domain(1), rule(2, 1, ""), citing("{@rule:domain-1}")]),
        "3.toml",
        "[text.statement] cites `{@rule:domain-1}`, which names no rule",
    );
}

#[test]
fn a_repealed_rule_may_cite_what_is_gone() {
    let repeal =
        |(id, text): (u64, String)| edit((id, text), "status = \"draft\"", "status = \"repealed\"");
    let citing = |what: &str| {
        repeal(edit(
            rule(3, 1, ""),
            "Do the thing 3.",
            &format!("Do the thing 3, after {what}."),
        ))
    };
    assert_clean(&load(&[
        domain(1),
        repeal(rule(2, 1, "")),
        citing("{@rule:2} and {@rule:9}"),
    ]));
}

#[test]
fn a_domain_under_an_option_is_a_subdomain() {
    let under = |id, parent: u64| {
        edit(
            domain(id),
            "lang = \"en\"",
            &format!("lang = \"en\"\nparent = {parent}"),
        )
    };
    assert_clean(&load(&[
        domain(1),
        one_of(2, 1, 3),
        edit(under(3, 2), "default = \"off\"\n", ""),
        under(4, 3),
        rule(5, 4, ""),
    ]));
}
