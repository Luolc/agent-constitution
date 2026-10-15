//! `statute validate`: a directory of rule files checked as the
//! first-party source, with no rule set.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, rule};

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
fn the_parameters_file_is_not_a_rule() {
    let dir = rules("params", &[domain(1), rule(2, 1, "")]);
    std::fs::write(dir.join("params.toml"), "[[parameter]]\n").unwrap();
    let run = validate(&dir);
    assert_eq!(run.code, 0, "{}", run.stderr);
}
