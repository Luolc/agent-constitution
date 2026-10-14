//! The checks on rule files and on the relations between them. Each test
//! states a source that passes, then the same source with one change that
//! must fail (or warn), so a check that never fires shows up red.

use statute::diag::{Diagnostics, Severity};
use statute::node::original_hash;
use statute::source::Source;

mod common;
use common::{domain, edit, one_of, rule};

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
        one_of(2, 1),
        rule(3, 2, ""),
        rule(4, 3, ""),
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
fn only_rules_and_one_ofs_have_a_parent() {
    assert_error(
        &load(&[
            domain(1),
            edit(domain(2), "lang = \"en\"", "lang = \"en\"\nparent = 1"),
        ]),
        "2.toml",
        "a domain has no parent",
    );
    assert_error(
        &load(&[domain(1), edit(rule(2, 1, ""), "parent = 1\n", "")]),
        "2.toml",
        "a rule needs a `parent`",
    );
    assert_error(
        &load(&[domain(1), edit(one_of(2, 1), "parent = 1\n", "")]),
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
            edit(one_of(3, 1), "parent = 1", "parent = 1\nrequires = [2]"),
        ]),
        "3.toml",
        "remove requires",
    );
    assert_error(
        &load(&[edit(
            domain(1),
            "lang = \"en\"",
            "lang = \"en\"\ncheck = \"review\"",
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
        "check = \"review\"\n",
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
    let r = edit(rule(2, 1, ""), "check = \"review\"", command);
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
        &load(&[domain(1), old, one_of(3, 1)]),
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
        one_of(2, 1),
        rule(3, 1, "requires = [2]"),
        rule(4, 1, "conflicts = [3]"),
        rule(5, 2, ""),
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
        &load(&[domain(1), one_of(2, 1), rule(3, 1, "conflicts = [2]")]),
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
    assert_clean(&load(&[domain(1), rule(2, 1, ""), rule(3, 2, "")]));
    assert_error(
        &load(&[domain(1), rule(2, 3, ""), rule(3, 2, "")]),
        "2.toml",
        "the parents form a cycle: 2 -> 3 -> 2",
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
    // Through a parent: 3 sits under 2 and 2 requires 3.
    assert_error(
        &load(&[domain(1), rule(2, 1, "requires = [3]"), rule(3, 2, "")]),
        "2.toml",
        "the requirements form a cycle (a parent counts as a requirement): 2 -> 3 -> 2",
    );
}

// Redundant requirements.

#[test]
fn the_parent_is_not_required_again() {
    assert_clean(&load(&[domain(1), rule(2, 1, ""), rule(3, 2, "")]));
    assert_error(
        &load(&[domain(1), rule(2, 1, ""), rule(3, 2, "requires = [2]")]),
        "3.toml",
        "it is the parent, which already counts as a requirement",
    );
}

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

#[test]
fn a_requirement_that_follows_from_the_parent_is_not_listed() {
    // Rule 4 sits under a one-of whose parent is rule 2; the requirement
    // passes through the one-of to rule 2.
    let base = [domain(1), rule(2, 1, ""), one_of(3, 2)];
    let mut ok = base.to_vec();
    ok.push(rule(4, 3, ""));
    assert_clean(&load(&ok));
    let mut redundant = base.to_vec();
    redundant.push(rule(4, 3, "requires = [2]"));
    assert_error(
        &load(&redundant),
        "4.toml",
        "lists 2 needlessly: it already follows from the parent, one-of 3",
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
    // Through a parent and a requirement: 4 sits under 3, 3 requires 2.
    assert_error(
        &load(&[
            domain(1),
            rule(2, 1, ""),
            rule(3, 1, "requires = [2]"),
            rule(4, 3, "conflicts = [2]"),
        ]),
        "4.toml",
        "conflicts with 2, which enabling 4 brings in",
    );
    // The other way: 3 is under 2, and 2 declares the conflict.
    assert_error(
        &load(&[domain(1), rule(2, 1, "conflicts = [3]"), rule(3, 2, "")]),
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
    let base = [domain(1), one_of(2, 1), rule(3, 2, ""), rule(4, 2, "")];
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
    // A rule under one child that requires its sibling.
    let mut sibling = base.to_vec();
    sibling.push(rule(5, 3, "requires = [4]"));
    assert_error(
        &load(&sibling),
        "5.toml",
        "brings in 3 and 4, children of one-of 2",
    );
}

#[test]
fn a_requirement_across_domains_should_name_a_one_of() {
    let base = [domain(1), domain(2), one_of(3, 2), rule(4, 3, "")];
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
        one_of(2, 1),
        rule(3, 2, ""),
        rule(4, 1, "requires = [2]"),
    ]));
    assert_error(
        &load(&[domain(1), one_of(2, 1), rule(4, 1, "requires = [2]")]),
        "4.toml",
        "names one-of 2, which has no children yet",
    );
}

#[test]
fn only_a_rule_is_superseded() {
    let superseded = edit(
        one_of(2, 1),
        "status = \"active\"",
        "status = \"superseded\"",
    );
    assert_error(
        &load(&[domain(1), superseded]),
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
        one_of(2, 1),
        rule(3, 2, ""),
        rule(4, 2, ""),
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
