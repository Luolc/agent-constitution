//! Default switches (`docs/design/rule-config.md`, Default switches): the
//! rules a rule set gets without listing them, and how `rules`,
//! `domains`, `exclude` and `packs` depart from them.

use statute::diag::{Diagnostics, Severity};
use statute::manifest::{Key, Manifest};
use statute::resolve::{Reason, Resolution, resolve};
use statute::source::Sources;

mod common;
use common::{domain, edit, one_of, option, rule, source};

use common::set_default as default;

/// Domain 1, on: rule 2 on, rule 3 off, one-of 4 whose default is 5 of
/// its children 5 and 6, and rule 9 off, which requires 2. Domain 10,
/// off: rule 11 on, rule 12 off, rule 13 on, which requires 11, and one-of 14 whose default is 15 of its
/// children 15 and 16. Rule 7, off in domain 1, requires one-of 14.
fn tree() -> Sources {
    source(&tree_files())
}

fn tree_files() -> Vec<(u64, String)> {
    vec![
        default(domain(1), "\"on\""),
        default(rule(2, 1, ""), "\"on\""),
        default(rule(3, 1, ""), "\"off\""),
        default(one_of(4, 1, 5), "5"),
        option(5, 4, ""),
        option(6, 4, ""),
        default(rule(7, 1, "requires = [14]"), "\"off\""),
        default(rule(9, 1, "requires = [2]"), "\"off\""),
        default(domain(10), "\"off\""),
        default(rule(11, 10, ""), "\"on\""),
        default(rule(12, 10, ""), "\"off\""),
        default(rule(13, 10, "requires = [11]"), "\"on\""),
        default(one_of(14, 10, 15), "15"),
        option(15, 14, ""),
        option(16, 14, ""),
    ]
}

/// A rule set of source `R` with the lines `extra` at its top level and
/// `entry` as the source's entry.
fn manifest_with(extra: &str, entry: &str) -> Manifest {
    Manifest::parse(&format!(
        "constitution = {{ source = \"R\", edition = \"2026-10\" }}\npublic = false\n{extra}\n\n[sources]\nR = {entry}\n"
    ))
    .unwrap()
}

fn run(extra: &str) -> (Resolution, Diagnostics) {
    let mut diags = Diagnostics::default();
    let res = resolve(
        &tree(),
        &manifest_with(extra, "{ path = \".\" }"),
        1,
        &mut diags,
    );
    (res, diags)
}

fn r(id: u64) -> Key {
    Key::new("R", id)
}

/// The rules enabled with no finding.
#[track_caller]
fn enabled(extra: &str) -> Vec<u64> {
    let (res, diags) = run(extra);
    assert!(
        diags.0.is_empty(),
        "expected no findings, got {:#?}",
        diags.0
    );
    res.enabled.keys().map(|k| k.id).collect()
}

#[track_caller]
fn finds(extra: &str, severity: Severity, needle: &str) {
    let (_, diags) = run(extra);
    let found: Vec<String> = diags
        .0
        .iter()
        .filter(|d| d.severity == severity)
        .map(ToString::to_string)
        .collect();
    assert!(
        found.len() == 1 && found[0].contains(needle),
        "expected one finding containing {needle:?}, got {:#?}",
        diags.0
    );
}

#[test]
fn the_defaults_alone_enable_the_rules_on_in_domains_on() {
    assert_eq!(enabled(""), [2, 5]);
    let (res, _) = run("");
    assert!(res.enabled[&r(2)].contains(&Reason::Default));
    assert!(res.enabled[&r(5)].contains(&Reason::DefaultOf(r(4))));
}

#[test]
fn defaults_false_takes_none_of_a_sources_defaults() {
    let mut diags = Diagnostics::default();
    let m = manifest_with("rules = [\"R3\"]", "{ path = \".\", defaults = false }");
    let res = resolve(&tree(), &m, 1, &mut diags);
    assert!(diags.0.is_empty(), "{:#?}", diags.0);
    assert_eq!(res.enabled.keys().map(|k| k.id).collect::<Vec<_>>(), [3]);
}

#[test]
fn domains_turns_on_a_domain_that_is_off() {
    assert_eq!(enabled("domains = [\"R10\"]"), [2, 5, 11, 13, 15]);
    finds(
        "domains = [\"R1\"]",
        Severity::Warning,
        "`domains` lists R1",
    );
    finds(
        "domains = [\"R2\"]",
        Severity::Error,
        "`domains` names rule",
    );
}

#[test]
fn domains_picks_a_domain_that_is_the_option_of_a_one_of() {
    // Domain 20, on: one-of 21 whose default is domain 22 of its options
    // 22 and 23, each with a rule on.
    let domain_option = |id| edit(domain(id), "default = \"off\"", "parent = 21");
    let s = source(&[
        default(domain(20), "\"on\""),
        one_of(21, 20, 22),
        domain_option(22),
        domain_option(23),
        default(rule(24, 22, ""), "\"on\""),
        default(rule(25, 23, ""), "\"on\""),
    ]);
    let enabled = |extra: &str| {
        let mut diags = Diagnostics::default();
        let m = manifest_with(extra, "{ path = \".\" }");
        let res = resolve(&s, &m, 1, &mut diags);
        assert!(diags.0.is_empty(), "{:#?}", diags.0);
        res.enabled.keys().map(|k| k.id).collect::<Vec<_>>()
    };
    assert_eq!(enabled(""), [24]);
    assert_eq!(enabled("domains = [\"R23\"]"), [25]);
}

#[test]
fn rules_turns_on_a_rule_and_picks_a_child() {
    assert_eq!(enabled("rules = [\"R3\"]"), [2, 3, 5]);
    assert_eq!(enabled("rules = [\"R6\"]"), [2, 6]);
    assert_eq!(enabled("rules = [\"R12\"]"), [2, 5, 12]);
    finds(
        "rules = [\"R2\"]",
        Severity::Warning,
        "which the defaults enable; take it out",
    );
}

#[test]
fn exclude_turns_off_a_node_and_its_subtree() {
    assert_eq!(enabled("exclude = [\"R2\"]"), [5]);
    assert_eq!(enabled("exclude = [\"R4\"]"), [2]);
    assert_eq!(enabled("exclude = [\"R1\"]"), Vec::<u64>::new());
    finds(
        "rules = [\"R3\"]\nexclude = [\"R3\"]",
        Severity::Error,
        "and `exclude` turns it off; keep one",
    );
    finds(
        "packs = [\"R10\"]\nexclude = [\"R10\"]",
        Severity::Error,
        "`packs` lists R10 (Domain 10) and `exclude` turns it off; keep one",
    );
}

/// An option of a one-of, its default or another, is chosen in `rules`
/// and never turned off alone; the one-of is.
#[test]
fn exclude_names_no_option_of_a_one_of() {
    for option in ["R5", "R6"] {
        finds(
            &format!("exclude = [\"{option}\"]"),
            Severity::Error,
            &format!(
                "`exclude` names {option}, an option of the one-of R4; list another option in `rules`, or exclude R4"
            ),
        );
    }
    assert_eq!(enabled("rules = [\"R6\"]"), [2, 6]);
    assert_eq!(enabled("exclude = [\"R4\"]"), [2]);
}

#[test]
fn exclude_cannot_turn_off_a_rule_an_enabled_rule_requires() {
    assert_eq!(enabled("rules = [\"R9\"]"), [2, 5, 9]);
    finds(
        "rules = [\"R9\"]\nexclude = [\"R2\"]",
        Severity::Error,
        "cannot be turned off: rule R9",
    );
}

#[test]
fn a_pack_takes_the_default_child_of_a_one_of() {
    assert_eq!(enabled("packs = [\"R10\"]"), [2, 5, 11, 12, 13, 15]);
    assert_eq!(
        enabled("packs = [\"R10\"]\nrules = [\"R16\"]"),
        [2, 5, 11, 12, 13, 16]
    );
    assert_eq!(
        enabled("packs = [\"R10\"]\nexclude = [\"R12\"]"),
        [2, 5, 11, 13, 15]
    );
}

#[test]
fn a_required_one_of_with_no_child_brings_in_its_default() {
    assert_eq!(enabled("rules = [\"R7\"]"), [2, 5, 7, 15]);
    let (res, _) = run("rules = [\"R7\"]");
    assert!(res.enabled[&r(15)].contains(&Reason::DefaultOf(r(14))));
    assert_eq!(enabled("rules = [\"R7\", \"R16\"]"), [2, 5, 7, 16]);
}

// The commands.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A repository under Cargo's test directory with the nodes of [`tree`]
/// and a rule set of source `R` with the lines `extra`.
fn repo(name: &str, extra: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("defaults")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("rules")).unwrap();
    std::fs::create_dir_all(dir.join("constitution/2026-10")).unwrap();
    std::fs::write(
        dir.join("statute.toml"),
        format!(
            "constitution = {{ source = \"R\", edition = \"2026-10\" }}\npublic = false\n{extra}\n\n[sources]\nR = {{ path = \".\" }}\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("constitution/2026-10/index.toml"),
        "edition = \"2026-10\"\nrevision = \"1\"\n",
    )
    .unwrap();
    for (id, text) in tree_files() {
        std::fs::write(dir.join(format!("rules/{id}.toml")), text).unwrap();
    }
    dir
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn statute(dir: &Path, args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_statute"))
        .env("HOME", concat!(env!("CARGO_TARGET_TMPDIR"), "/home"))
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_CACHE_HOME")
        // git looks for a repository no further up than the tests' directory,
        // which lies inside this repository's work tree.
        .env("GIT_CEILING_DIRECTORIES", env!("CARGO_TARGET_TMPDIR"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    Run {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

#[track_caller]
fn ok(run: &Run) {
    assert_eq!(
        run.code, 0,
        "stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
}

fn read(dir: &Path, file: &str) -> String {
    std::fs::read_to_string(dir.join(file)).unwrap()
}

/// The lock file marks a rule that came in by the defaults alone, though
/// another such rule requires it, and no rule the rule set chose or that
/// a chosen rule requires.
#[test]
fn the_lock_file_marks_what_the_defaults_alone_enable() {
    let marked = |extra: &str| {
        let dir = repo("lock", extra);
        ok(&statute(&dir, &["lock"]));
        let lock: toml::Value = toml::from_str(&read(&dir, "statute.lock")).unwrap();
        lock["rules"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r.get("by").and_then(|b| b.as_str()) == Some("default"))
            .map(|r| r["rule"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(marked("rules = [\"R3\"]"), ["R2", "R5"]);
    assert_eq!(
        marked("domains = [\"R10\"]"),
        ["R2", "R5", "R11", "R13", "R15"]
    );
    assert_eq!(marked("rules = [\"R9\"]"), ["R5"]);
}

#[test]
fn add_writes_a_domain_to_domains() {
    let dir = repo("add-domain", "");
    let run = statute(&dir, &["add", "R10"]);
    ok(&run);
    assert!(
        run.stdout.contains("added R10 to `domains`"),
        "{}",
        run.stdout
    );
    assert!(read(&dir, "statute.toml").contains("domains = [\"R10\"]"));
    assert!(read(&dir, "statute.lock").contains("rule = \"R11\""));
}

/// `remove` takes a listed entry out, and writes what the defaults
/// still enable to `exclude`, a rule or a domain.
#[test]
fn remove_writes_what_the_defaults_enable_to_exclude() {
    let dir = repo("remove", "rules = [\"R3\"]");
    let run = statute(&dir, &["remove", "R3"]);
    ok(&run);
    assert!(
        run.stdout.contains("removed R3 from `rules`"),
        "{}",
        run.stdout
    );
    let run = statute(&dir, &["remove", "R2"]);
    ok(&run);
    assert!(
        run.stdout
            .contains("added R2 to `exclude`, as it is on by default"),
        "{}",
        run.stdout
    );
    let run = statute(&dir, &["remove", "R1"]);
    ok(&run);
    assert!(read(&dir, "statute.toml").contains("exclude = [\"R2\", \"R1\"]"));
    assert!(!read(&dir, "statute.lock").contains("[[rules]]"));
    let run = statute(&dir, &["remove", "R12"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr
            .contains("statute.toml: error: `rules` does not list R12"),
        "{}",
        run.stderr
    );
}

#[test]
fn explain_says_whether_a_node_is_on_by_default() {
    let dir = repo("explain", "rules = [\"R3\"]\nexclude = [\"R4\"]");
    ok(&statute(&dir, &["lock"]));
    for (what, line) in [
        ("R2", "- on by default, and its domain R1 (Domain 1) is on"),
        (
            "R12",
            "- R12 is off by default; `statute add R12` turns it on",
        ),
        (
            "R11",
            "- R11 is on by default, and its domain R10 is off; `statute add R10` turns that on",
        ),
        ("R4", "one-of R4 (One-of 4): its default is R5 (Rule 5)"),
        ("R1", "- R1 is on by default"),
    ] {
        let run = statute(&dir, &["explain", what]);
        assert!(
            run.stdout.contains(line),
            "{what}: {}{}",
            run.stdout,
            run.stderr
        );
    }
    let run = statute(&dir, &["explain", "--defaults"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "On by default, and kept:\n- R2 (Rule 2)\n\nOn by default, and turned off:\n- R5 (Rule 5)\n\nOff by default, and turned on:\n- R3 (Rule 3)\n"
    );
}

/// An unknown key in a source's entry is named with the entry, and no
/// error follows from the entry being left out.
#[test]
fn an_unknown_key_of_a_source_entry_is_named() {
    let errors = Manifest::parse(
        "constitution = { source = \"R\", edition = \"2026-10\" }\npublic = false\nrules = [\"R3\"]\n\n[sources]\nR = { path = \".\", brnch = \"main\" }\n",
    )
    .err()
    .unwrap();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert!(
        errors[0].starts_with("[sources] `R`: unknown field `brnch`"),
        "{errors:#?}"
    );
}

/// The private layer may turn on a domain, and turns off nothing.
#[test]
fn the_private_layer_has_domains_and_no_exclude() {
    let base = Manifest::parse(
        "constitution = { source = \"R\", edition = \"2026-10\" }\npublic = true\n\n[sources]\nR = { path = \".\" }\n",
    )
    .unwrap();
    let private = base.with_private("domains = [\"R10\"]\n").unwrap();
    assert_eq!(private.domains, [r(10)]);
    let errors = base.with_private("exclude = [\"R2\"]\n").err().unwrap();
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`exclude` is for statute.toml")),
        "{errors:#?}"
    );
}

/// `upgrade` lists a rule that a changed `default` brings in.
#[test]
fn upgrade_lists_a_rule_a_changed_default_brings_in() {
    let dir = repo("upgrade", "");
    ok(&statute(&dir, &["lock"]));
    let on = default(rule(3, 1, ""), "\"on\"")
        .1
        .replace("revision = 1", "revision = 2");
    std::fs::write(dir.join("rules/3.toml"), on).unwrap();
    let run = statute(&dir, &["upgrade"]);
    ok(&run);
    assert!(
        run.stdout
            .contains("brought in: rule R3 (Rule 3), on by default"),
        "{}",
        run.stdout
    );
}

/// A rule set that excludes an option neither locks nor explains it as
/// enabled.
#[test]
fn lock_and_explain_fail_on_an_excluded_option() {
    let dir = repo("exclude-option", "exclude = [\"R5\"]");
    let needle = "statute.toml: error: `exclude` names R5, an option of the one-of R4";
    for args in [&["lock"][..], &["explain", "R5"]] {
        let run = statute(&dir, args);
        assert_eq!(run.code, 1, "{args:?}: {}", run.stdout);
        assert!(run.stderr.contains(needle), "{args:?}: {}", run.stderr);
        assert!(
            !run.stdout.contains("on by default"),
            "{args:?}: {}",
            run.stdout
        );
    }
    assert!(!dir.join("statute.lock").exists());
}
