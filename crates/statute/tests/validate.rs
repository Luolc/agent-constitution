//! `statute validate`: a directory of rule files checked as the
//! first-party source, with no rule set.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, one_of, option, param, rule};

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn validate(dir: &Path) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_statute"))
        .args(["validate", dir.to_str().unwrap()])
        .output()
        .unwrap();
    Run {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

/// A fresh `rules/` under Cargo's test directory with the nodes.
fn rules(name: &str, nodes: &[(u64, String)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("validate")
        .join(name)
        .join("rules");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (id, text) in nodes {
        std::fs::write(dir.join(format!("{id}.toml")), text).unwrap();
    }
    dir
}

#[test]
fn the_rules_of_this_repo_pass() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rules");
    let run = validate(&dir);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(
        run.stdout.ends_with(" nodes, no errors\n"),
        "{}",
        run.stdout
    );
}

/// Each reference to a node that is not there fails, and the same
/// directory passes again once the reference is put back.
#[test]
fn a_reference_to_no_node_fails() {
    let good = || vec![domain(1), rule(2, 1, ""), rule(3, 1, "requires = [2]")];
    let superseded = edit(
        rule(3, 1, "superseded-by = 99"),
        "status = \"draft\"",
        "status = \"superseded\"",
    );
    for (name, node, needle) in [
        (
            "requires",
            rule(3, 1, "requires = [9999]"),
            "rules/3.toml: error: `requires` names 9999, which does not exist in this source",
        ),
        (
            "conflicts",
            rule(3, 1, "conflicts = [9999]"),
            "rules/3.toml: error: `conflicts` names 9999, which does not exist in this source",
        ),
        (
            "overrides",
            rule(3, 1, "overrides = 9999"),
            "rules/3.toml: error: `overrides` names 9999, a node of this source; a rule overrides a rule of another source",
        ),
        (
            "parent",
            rule(3, 99, ""),
            "rules/3.toml: error: parent 99 does not exist in this source",
        ),
        (
            "superseded-by",
            superseded,
            "rules/3.toml: error: superseded-by 99 does not exist in this source",
        ),
        (
            "citation",
            edit(
                rule(3, 1, ""),
                "Do the thing 3.",
                "Do the thing 3, after {@rule:99}.",
            ),
            "rules/3.toml: error: [text.statement] cites `{@rule:99}`, which does not exist in this source",
        ),
        (
            "slug",
            edit(
                rule(3, 1, ""),
                "Do the thing 3.",
                "Do the thing 3, after {@rule:rule-2}.",
            ),
            "rules/3.toml: error: [text.statement] cites `{@rule:rule-2}`, which names no rule",
        ),
        (
            "first-party",
            rule(3, 1, "requires = [\"R2\"]"),
            "rules/3.toml: error: `requires` names \"R2\"; in a rule of the first-party source itself, write the number, 2",
        ),
    ] {
        let mut nodes = good();
        nodes[2] = node;
        let dir = rules(name, &nodes);
        let run = validate(&dir);
        assert_eq!(run.code, 1, "{name}: {}", run.stdout);
        assert!(run.stderr.contains(needle), "{name}: {}", run.stderr);
        std::fs::write(dir.join("3.toml"), &good()[2].1).unwrap();
        let run = validate(&dir);
        assert_eq!(run.code, 0, "{name}: {}", run.stderr);
        assert!(
            run.stdout.ends_with("rules: 3 nodes, no errors\n"),
            "{name}: {}",
            run.stdout
        );
    }
}

#[test]
fn the_parameters_file_declares_what_the_rules_name() {
    let uses = edit(
        rule(2, 1, ""),
        "Do the thing 2.",
        "Write in {$language.docs}.",
    );
    let dir = rules("params", &[domain(1), uses.clone()]);
    let declared = param("language.docs", "type = \"language\"", "repo", "en");
    std::fs::write(dir.join("params.toml"), &declared).unwrap();
    let run = validate(&dir);
    assert_eq!(run.code, 0, "{}", run.stderr);
    // A placeholder of no parameter, and a file that is no declaration.
    let dir = rules("params-missing", &[domain(1), uses]);
    std::fs::write(dir.join("params.toml"), "[[parameter]]\n").unwrap();
    let run = validate(&dir);
    assert_eq!(run.code, 1);
    for line in [
        "rules/params.toml: error: parameter 1: `name` is missing",
        "rules/2.toml: error: names `{$language.docs}`, a parameter that source `R` does not declare\n",
    ] {
        assert!(run.stderr.contains(line), "{line}: {}", run.stderr);
    }
}

/// A repealed rule keeps its texts as they were: what it cites may be
/// gone, or repealed with it.
#[test]
fn a_repealed_rule_may_cite_what_is_gone() {
    let repeal =
        |(id, text): (u64, String)| edit((id, text), "status = \"draft\"", "status = \"repealed\"");
    let citing = repeal(edit(
        rule(3, 1, ""),
        "Do the thing 3.",
        "Do the thing 3, after {@rule:2} and {@rule:99}.",
    ));
    let dir = rules("repealed", &[domain(1), repeal(rule(2, 1, "")), citing]);
    let run = validate(&dir);
    assert_eq!(run.code, 0, "{}", run.stderr);
}

/// A node in force that names a superseded or repealed rule fails, with
/// what supersedes it; naming the successor passes, and so does a
/// superseded rule that still names it.
#[test]
fn a_node_in_force_names_no_retired_rule() {
    let superseded = edit(
        rule(2, 1, "superseded-by = 4"),
        "status = \"draft\"",
        "status = \"superseded\"",
    );
    let repealed = edit(
        rule(5, 1, ""),
        "status = \"draft\"",
        "status = \"repealed\"",
    );
    let retired_domain = edit(domain(6), "status = \"active\"", "status = \"repealed\"");
    let cites = |id: u64| {
        edit(
            rule(3, 1, ""),
            "Do the thing 3.",
            &format!("Do the thing 3, after {{@rule:{id}}}."),
        )
    };
    for (name, node, needle) in [
        (
            "citation",
            cites(2),
            "rules/3.toml: error: [text.statement] cites rule 2, which is superseded by 4; a node in force names only nodes in force",
        ),
        (
            "requires",
            rule(3, 1, "requires = [2]"),
            "rules/3.toml: error: `requires` names rule 2, which is superseded by 4;",
        ),
        (
            "conflicts",
            rule(3, 1, "conflicts = [5]"),
            "rules/3.toml: error: `conflicts` names rule 5, which is repealed;",
        ),
        (
            "parent",
            rule(3, 6, ""),
            "rules/3.toml: error: `parent` names domain 6, which is repealed;",
        ),
    ] {
        let nodes = |three| {
            vec![
                domain(1),
                superseded.clone(),
                three,
                rule(4, 1, ""),
                repealed.clone(),
                retired_domain.clone(),
            ]
        };
        let dir = rules(&format!("retired-{name}"), &nodes(node.clone()));
        let run = validate(&dir);
        assert_eq!(run.code, 1, "{name}: {}", run.stdout);
        assert!(run.stderr.contains(needle), "{name}: {}", run.stderr);
        // Superseded itself, rule 3 names what it named before.
        let frozen = edit(
            edit(node, "status = \"draft\"", "status = \"superseded\""),
            "revision = 1",
            "revision = 1\nsuperseded-by = 4",
        );
        let dir = rules(&format!("retired-{name}-frozen"), &nodes(frozen));
        let run = validate(&dir);
        assert_eq!(run.code, 0, "{name}: {}", run.stderr);
    }
    let dir = rules(
        "retired-successor",
        &[domain(1), superseded, cites(4), rule(4, 1, "")],
    );
    let run = validate(&dir);
    assert_eq!(run.code, 0, "{}", run.stderr);
    // A citation of a repealed rule is the per-source checks' error alone.
    let dir = rules(
        "retired-repealed-citation",
        &[domain(1), cites(5), repealed.clone()],
    );
    let run = validate(&dir);
    assert_eq!(run.code, 1, "{}", run.stdout);
    let errors: Vec<&str> = run
        .stderr
        .lines()
        .filter(|l| l.contains(": error: "))
        .collect();
    assert_eq!(errors.len(), 1, "{}", run.stderr);
    assert!(
        errors[0].ends_with(
            "rules/3.toml: error: [text.statement] cites `{@rule:5}`, which is repealed"
        ),
        "{}",
        run.stderr
    );
    // A one-of whose `default` is a superseded option, then its successor.
    let options = || {
        vec![
            domain(1),
            edit(
                option(8, 7, "superseded-by = 9"),
                "status = \"draft\"",
                "status = \"superseded\"",
            ),
            option(9, 7, ""),
        ]
    };
    let dir = rules(
        "retired-default",
        &[options(), vec![one_of(7, 1, 8)]].concat(),
    );
    let run = validate(&dir);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(
        run.stderr.contains(
            "rules/7.toml: error: `default` names rule 8, which is superseded by 9; a node in force names only nodes in force"
        ),
        "{}",
        run.stderr
    );
    let dir = rules(
        "retired-default-successor",
        &[options(), vec![one_of(7, 1, 9)]].concat(),
    );
    let run = validate(&dir);
    assert_eq!(run.code, 0, "{}", run.stderr);
}
