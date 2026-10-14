//! The commands, run as a user runs them, in repositories written under
//! Cargo's test directory.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, one_of, rule};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

fn manifest(rules: &str) -> String {
    format!(
        "# The rules of this repository.\nconstitution = {{ source = \"ac\", edition = \"2026-10\" }}\nrules = [{rules}]  # chosen directly\n\n[sources]\nac = {{ path = \".\" }}\n"
    )
}

/// Domain 1; rule 2 and its child 3; one-of 4 with children 5 and 6;
/// rule 7 requiring the one-of; rule 8 in conflict with 2.
fn nodes() -> Vec<(u64, String)> {
    vec![
        domain(1),
        rule(2, 1, ""),
        rule(3, 2, ""),
        one_of(4, 1),
        rule(5, 4, ""),
        rule(6, 4, ""),
        rule(7, 1, "requires = [4]"),
        rule(8, 1, "conflicts = [2]"),
    ]
}

fn repo(name: &str, manifest: &str, nodes: &[(u64, String)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("cli")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("rules")).unwrap();
    std::fs::create_dir_all(dir.join("constitution/2026-10")).unwrap();
    std::fs::write(dir.join("statute.toml"), manifest).unwrap();
    std::fs::write(dir.join("constitution/2026-10/index.toml"), INDEX).unwrap();
    for (id, text) in nodes {
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

fn read(dir: &Path, file: &str) -> String {
    std::fs::read_to_string(dir.join(file)).unwrap()
}

fn sha256(dir: &Path, file: &str) -> String {
    statute::node::sha256_hex(&std::fs::read(dir.join(file)).unwrap())
}

#[track_caller]
fn ok(run: &Run) {
    assert_eq!(
        run.code, 0,
        "stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
}

#[test]
fn lock_writes_the_lock_file() {
    let dir = repo("lock", &manifest("\"ac:3\""), &nodes());
    let run = statute(&dir, &["lock"]);
    ok(&run);
    assert_eq!(run.stdout, "wrote statute.lock: 2 rules enabled\n");
    let expected = format!(
        r#"# Written by statute from statute.toml; do not edit by hand.

[constitution]
path = "."
edition = "2026-10"
revision = "1"

[[sources]]
name = "ac"
path = "."

[[rules]]
source = "ac"
id = 2
revision = 1
sha256 = "{}"

[[rules]]
source = "ac"
id = 3
revision = 1
sha256 = "{}"
"#,
        sha256(&dir, "rules/2.toml"),
        sha256(&dir, "rules/3.toml")
    );
    assert_eq!(read(&dir, "statute.lock"), expected);
}

#[test]
fn the_lock_file_is_the_same_for_the_same_input() {
    let dir = repo("same", &manifest("\"ac:7\", \"ac:5\""), &nodes());
    ok(&statute(&dir, &["lock"]));
    let first = std::fs::read(dir.join("statute.lock")).unwrap();
    ok(&statute(&dir, &["lock"]));
    assert_eq!(std::fs::read(dir.join("statute.lock")).unwrap(), first);
    // Another order in the rule set gives the same bytes.
    let other = repo("same-reordered", &manifest("\"ac:5\", \"ac:7\""), &nodes());
    ok(&statute(&other, &["lock"]));
    assert_eq!(std::fs::read(other.join("statute.lock")).unwrap(), first);
    // A changed rule file changes its hash.
    let mut changed = nodes();
    changed[6] = edit(
        changed[6].clone(),
        "Do the thing 7.",
        "Do the other thing 7.",
    );
    let changed = repo("same-changed", &manifest("\"ac:7\", \"ac:5\""), &changed);
    ok(&statute(&changed, &["lock"]));
    assert_ne!(std::fs::read(changed.join("statute.lock")).unwrap(), first);
}

#[test]
fn lock_fails_on_an_error_and_writes_nothing() {
    let dir = repo("lock-fails", &manifest("\"ac:7\""), &nodes());
    let run = statute(&dir, &["lock"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr
            .contains("statute.toml: error: rule 7 (Rule 7) requires one-of 4 (One-of 4)"),
        "{}",
        run.stderr
    );
    assert!(!dir.join("statute.lock").exists());
}

#[test]
fn explain_follows_the_chain() {
    let dir = repo("explain", &manifest("\"ac:3\""), &nodes());
    let run = statute(&dir, &["explain", "ac:2"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "rule 2 (Rule 2) is enabled:\n- the parent of 3 (Rule 3), which is enabled:\n  - listed in `rules`\n"
    );
    let run = statute(&dir, &["explain", "3"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "rule 3 (Rule 3) is enabled:\n- listed in `rules`\n"
    );
}

#[test]
fn explain_a_rule_brought_in_by_a_requirement() {
    let mut nodes = nodes();
    nodes.push(rule(9, 1, "requires = [8]"));
    let dir = repo("explain-requires", &manifest("\"ac:9\""), &nodes);
    let run = statute(&dir, &["explain", "8"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "rule 8 (Rule 8) is enabled:\n- required by 9 (Rule 9), which is enabled:\n  - listed in `rules`\n"
    );
}

#[test]
fn explain_a_rule_not_enabled() {
    let dir = repo("explain-not", &manifest("\"ac:3\""), &nodes());
    let run = statute(&dir, &["explain", "ac:8"]);
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stdout,
        "rule 8 (Rule 8) is not enabled by the rule set\n"
    );
    let run = statute(&dir, &["explain", "team:8"]);
    assert_eq!(run.code, 1);
    assert!(run.stderr.contains("names source `team`"), "{}", run.stderr);
}

#[test]
fn add_keeps_the_rest_of_the_rule_set() {
    let dir = repo("add", &manifest("\"ac:3\""), &nodes());
    let run = statute(&dir, &["add", "ac:5"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "added ac:5 to `rules`\nwrote statute.lock: 3 rules enabled\n"
    );
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:3\", \"ac:5\""));
    assert!(read(&dir, "statute.lock").contains("id = 5\n"));
    // Twice changes nothing.
    let run = statute(&dir, &["add", "5"]);
    ok(&run);
    assert_eq!(run.stdout, "ac:5 is already in `rules`\n");
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:3\", \"ac:5\""));
}

#[test]
fn add_that_would_break_the_rule_set_changes_nothing() {
    let dir = repo("add-conflict", &manifest("\"ac:3\""), &nodes());
    ok(&statute(&dir, &["lock"]));
    let lock = read(&dir, "statute.lock");
    let run = statute(&dir, &["add", "ac:8"]);
    assert_eq!(run.code, 1);
    assert!(run.stderr.contains("conflict"), "{}", run.stderr);
    assert!(
        run.stderr
            .contains("statute.toml and statute.lock are left unchanged"),
        "{}",
        run.stderr
    );
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:3\""));
    assert_eq!(read(&dir, "statute.lock"), lock);
}

#[test]
fn add_a_pack() {
    let dir = repo("add-pack", &manifest(""), &nodes());
    ok(&statute(&dir, &["add", "--pack", "ac:2"]));
    assert!(
        read(&dir, "statute.toml").contains("packs = [\"ac:2\"]"),
        "{}",
        read(&dir, "statute.toml")
    );
    let lock = read(&dir, "statute.lock");
    assert!(
        lock.contains("id = 2\n") && lock.contains("id = 3\n"),
        "{lock}"
    );
}

#[test]
fn remove_keeps_the_rest_of_the_rule_set() {
    let dir = repo("remove", &manifest("\"ac:3\", \"ac:5\""), &nodes());
    let run = statute(&dir, &["remove", "ac:5"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "removed ac:5 from `rules`\nwrote statute.lock: 2 rules enabled\n"
    );
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:3\""));
    assert!(!read(&dir, "statute.lock").contains("id = 5\n"));
}

#[test]
fn remove_says_when_the_rule_stays_enabled() {
    let dir = repo("remove-stays", &manifest("\"ac:2\", \"ac:3\""), &nodes());
    let run = statute(&dir, &["remove", "2"]);
    ok(&run);
    assert!(
        run.stdout
            .contains("ac:2 is still enabled; `statute explain ac:2` says why"),
        "{}",
        run.stdout
    );
}

#[test]
fn remove_that_would_break_the_rule_set_changes_nothing() {
    let dir = repo("remove-breaks", &manifest("\"ac:7\", \"ac:5\""), &nodes());
    let run = statute(&dir, &["remove", "ac:5"]);
    assert_eq!(run.code, 1);
    assert!(run.stderr.contains("requires one-of 4"), "{}", run.stderr);
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:7\", \"ac:5\""));
    let run = statute(&dir, &["remove", "ac:6"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains("`rules` does not list ac:6"),
        "{}",
        run.stderr
    );
}

#[test]
fn commands_not_written_yet_exit_2() {
    let dir = repo("later", &manifest("\"ac:3\""), &nodes());
    for command in ["init", "compile", "check", "upgrade"] {
        let run = statute(&dir, &[command]);
        assert_eq!(run.code, 2, "{command}");
        assert_eq!(run.stderr, "statute: this command is not implemented yet\n");
    }
}

#[test]
fn add_leaves_both_files_as_they_were_when_the_lock_file_cannot_be_written() {
    let dir = repo("add-lock-fails", &manifest("\"ac:3\""), &nodes());
    // A directory in the way: the rule set is written, the lock file not.
    std::fs::create_dir_all(dir.join("statute.lock/in-the-way")).unwrap();
    let run = statute(&dir, &["add", "ac:5"]);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(
        run.stderr.contains("statute.lock: error: cannot write"),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr
            .contains("statute.toml and statute.lock are left unchanged"),
        "{}",
        run.stderr
    );
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:3\""));
    assert!(!dir.join("statute.toml.tmp").exists());
    assert!(!dir.join("statute.lock.tmp").exists());
    // With the way clear, the same command goes through.
    std::fs::remove_dir_all(dir.join("statute.lock")).unwrap();
    ok(&statute(&dir, &["add", "ac:5"]));
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:3\", \"ac:5\""));
}
