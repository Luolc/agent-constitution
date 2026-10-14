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

/// Writes a rule file (or the index, for `id` 0) over the one in `dir`.
fn put(dir: &Path, (id, text): &(u64, String)) {
    let file = if *id == 0 {
        "constitution/2026-10/index.toml".to_owned()
    } else {
        format!("rules/{id}.toml")
    };
    std::fs::write(dir.join(file), text).unwrap();
}

/// `nodes()` and rule 9, which rule 7 also requires.
fn upgrade_nodes() -> Vec<(u64, String)> {
    let mut nodes = nodes();
    nodes[6] = rule(7, 1, "requires = [4, 9]");
    nodes.push(rule(9, 1, ""));
    nodes
}

#[test]
fn upgrade_without_a_lock_file_fails() {
    let dir = repo("upgrade-no-lock", &manifest("\"ac:3\""), &nodes());
    let run = statute(&dir, &["upgrade"]);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(
        run.stderr.contains("statute.lock: error: cannot read: ")
            && run.stderr.contains("`statute lock` writes it"),
        "{}",
        run.stderr
    );
    assert!(!dir.join("statute.lock").exists());
}

#[test]
fn upgrade_reports_what_changed() {
    let dir = repo("upgrade", &manifest("\"ac:7\", \"ac:5\""), &upgrade_nodes());
    ok(&statute(&dir, &["lock"]));
    let run = statute(&dir, &["upgrade"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "statute.lock is up to date\nwrote statute.lock: 3 rules enabled\n"
    );
    // Rule 7 moves to revision 2 and requires 2 in place of 9; rule 5
    // changes without a new revision; the constitution moves to revision 2.
    put(
        &dir,
        &edit(
            rule(7, 1, "requires = [4, 2]"),
            "revision = 1",
            "revision = 2",
        ),
    );
    put(&dir, &edit(rule(5, 4, ""), "Do the thing 5.", "Do it 5."));
    put(
        &dir,
        &(0, INDEX.replace("revision = \"1\"", "revision = \"2\"")),
    );
    let run = statute(&dir, &["upgrade"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "constitution: revision 1 -> 2\n\
         brought in: rule 2 (Rule 2), required by 7 (Rule 7)\n\
         changed: rule 5 (Rule 5): content changed, still revision 1; an edited rule's revision goes up by one\n\
         changed: rule 7 (Rule 7): revision 1 -> 2\n\
         dropped: rule 9 (Rule 9)\n\
         wrote statute.lock: 3 rules enabled\n"
    );
    // The lock file is the one `lock` writes.
    let upgraded = read(&dir, "statute.lock");
    ok(&statute(&dir, &["lock"]));
    assert_eq!(read(&dir, "statute.lock"), upgraded);
    assert!(upgraded.contains("revision = \"2\"\n"), "{upgraded}");
    let run = statute(&dir, &["upgrade"]);
    assert_eq!(
        run.stdout,
        "statute.lock is up to date\nwrote statute.lock: 3 rules enabled\n"
    );
}

#[test]
fn lock_warns_of_a_rule_changed_without_a_new_revision() {
    let dir = repo(
        "lock-revision",
        &manifest("\"ac:7\", \"ac:5\""),
        &upgrade_nodes(),
    );
    let run = statute(&dir, &["lock"]);
    ok(&run);
    assert!(!run.stderr.contains("changed since"), "{}", run.stderr);
    put(&dir, &edit(rule(5, 4, ""), "Do the thing 5.", "Do it 5."));
    put(
        &dir,
        &edit(
            edit(
                rule(7, 1, "requires = [4, 9]"),
                "revision = 1",
                "revision = 2",
            ),
            "Do the thing 7.",
            "Do it 7.",
        ),
    );
    let run = statute(&dir, &["lock"]);
    ok(&run);
    assert_eq!(
        run.stderr,
        "rules/5.toml: warning: rule ac:5 (Rule 5) changed since statute.lock and is still at revision 1; an edited rule's revision goes up by one\n"
    );
    // Locked again, the change is pinned and the warning gone.
    let run = statute(&dir, &["lock"]);
    ok(&run);
    assert_eq!(run.stderr, "");
    // Without a lock file there is nothing to compare with.
    put(
        &dir,
        &edit(rule(5, 4, ""), "Do the thing 5.", "Do it again 5."),
    );
    std::fs::remove_file(dir.join("statute.lock")).unwrap();
    let run = statute(&dir, &["lock"]);
    ok(&run);
    assert_eq!(run.stderr, "");
}

/// `nodes()` and rule 10, superseded by 11.
fn superseded_nodes(eleven: &str) -> Vec<(u64, String)> {
    let mut nodes = nodes();
    nodes.push(edit(
        rule(10, 1, "superseded-by = 11"),
        "status = \"draft\"",
        "status = \"superseded\"",
    ));
    nodes.push(rule(11, 1, eleven));
    nodes
}

#[test]
fn upgrade_replaces_a_superseded_rule() {
    let dir = repo(
        "upgrade-replace",
        &manifest("\"ac:10\", \"ac:3\""),
        &superseded_nodes(""),
    );
    ok(&statute(&dir, &["lock"]));
    // Without the flag, the rule set stays and the warning says what to do.
    let run = statute(&dir, &["upgrade"]);
    ok(&run);
    assert!(
        run.stderr.contains(
            "replace ac:10 with ac:11 in `rules` (`statute upgrade --replace-superseded` does it)"
        ),
        "{}",
        run.stderr
    );
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:10\", \"ac:3\""));
    let run = statute(&dir, &["upgrade", "--replace-superseded"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "replaced ac:10 with ac:11 in `rules`\n\
         brought in: rule 11 (Rule 11), listed in `rules`\n\
         dropped: rule 10 (Rule 10)\n\
         wrote statute.lock: 3 rules enabled\n"
    );
    assert_eq!(run.stderr, "");
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:11\", \"ac:3\""));
    let run = statute(&dir, &["upgrade", "--replace-superseded"]);
    assert_eq!(
        run.stdout,
        "statute.lock is up to date\nwrote statute.lock: 3 rules enabled\n"
    );
}

#[test]
fn upgrade_takes_out_a_superseded_rule_whose_successor_is_listed() {
    let dir = repo(
        "upgrade-replace-listed",
        &manifest("\"ac:3\", \"ac:10\", \"ac:11\""),
        &superseded_nodes(""),
    );
    ok(&statute(&dir, &["lock"]));
    let run = statute(&dir, &["upgrade", "--replace-superseded"]);
    ok(&run);
    assert!(
        run.stdout.starts_with(
            "took ac:10 out of `rules`: superseded by ac:11, which it lists already\n"
        ),
        "{}",
        run.stdout
    );
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:3\", \"ac:11\""));
}

#[test]
fn upgrade_replaces_two_rules_with_one_successor_once() {
    let mut nodes = superseded_nodes("");
    nodes.push(edit(
        rule(12, 1, "superseded-by = 11"),
        "status = \"draft\"",
        "status = \"superseded\"",
    ));
    let dir = repo(
        "upgrade-replace-two",
        &manifest("\"ac:10\", \"ac:3\", \"ac:12\""),
        &nodes,
    );
    ok(&statute(&dir, &["lock"]));
    let run = statute(&dir, &["upgrade", "--replace-superseded"]);
    ok(&run);
    assert!(
        run.stdout.starts_with(
            "took ac:10 out of `rules`: superseded by ac:11, which it lists already\n\
             replaced ac:12 with ac:11 in `rules`\n"
        ),
        "{}",
        run.stdout
    );
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:3\", \"ac:11\""));
}

#[test]
fn upgrade_leaves_a_rule_with_two_successors_to_the_user() {
    let mut nodes = superseded_nodes("");
    nodes[8] = edit(
        rule(10, 1, "superseded-by = [11, 12]"),
        "status = \"draft\"",
        "status = \"superseded\"",
    );
    nodes.push(rule(12, 1, ""));
    let dir = repo(
        "upgrade-replace-split",
        &manifest("\"ac:10\", \"ac:3\""),
        &nodes,
    );
    ok(&statute(&dir, &["lock"]));
    let run = statute(&dir, &["upgrade", "--replace-superseded"]);
    ok(&run);
    assert!(
        run.stdout.starts_with(
            "left ac:10 in `rules`: superseded by ac:11, ac:12; choose among them with `statute add` and `statute remove`\n"
        ),
        "{}",
        run.stdout
    );
    assert!(
        run.stderr
            .contains("replace ac:10 in `rules` with those of ac:11, ac:12 that apply"),
        "{}",
        run.stderr
    );
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:10\", \"ac:3\""));
}

#[test]
fn upgrade_replaces_a_superseded_pack_root() {
    let text = manifest("").replace("\n\n[sources]", "\npacks = [\"ac:10\"]\n\n[sources]");
    let mut nodes = superseded_nodes("");
    nodes.push(rule(12, 11, ""));
    let dir = repo("upgrade-replace-pack", &text, &nodes);
    ok(&statute(&dir, &["lock"]));
    let run = statute(&dir, &["upgrade", "--replace-superseded"]);
    ok(&run);
    assert!(
        run.stdout
            .starts_with("replaced ac:10 with ac:11 in `packs`\n"),
        "{}",
        run.stdout
    );
    assert_eq!(
        read(&dir, "statute.toml"),
        text.replace("\"ac:10\"", "\"ac:11\"")
    );
    let lock = read(&dir, "statute.lock");
    assert!(
        lock.contains("id = 11\n") && lock.contains("id = 12\n"),
        "{lock}"
    );
}

#[test]
fn upgrade_that_would_break_the_rule_set_changes_nothing() {
    let dir = repo(
        "upgrade-replace-breaks",
        &manifest("\"ac:10\", \"ac:3\""),
        &superseded_nodes("conflicts = [3]"),
    );
    ok(&statute(&dir, &["lock"]));
    let lock = read(&dir, "statute.lock");
    let run = statute(&dir, &["upgrade", "--replace-superseded"]);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(run.stderr.contains("conflict"), "{}", run.stderr);
    assert!(
        run.stderr
            .contains("statute.toml and statute.lock are left unchanged"),
        "{}",
        run.stderr
    );
    assert_eq!(read(&dir, "statute.toml"), manifest("\"ac:10\", \"ac:3\""));
    assert_eq!(read(&dir, "statute.lock"), lock);
}

/// A repository with a source but no rule set yet.
fn bare_repo(name: &str) -> PathBuf {
    let dir = repo(name, "", &nodes());
    std::fs::remove_file(dir.join("statute.toml")).unwrap();
    dir
}

#[test]
fn init_writes_a_rule_set_that_locks() {
    let dir = bare_repo("init");
    let run = statute(&dir, &["init"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "wrote statute.toml: edition 2026-10 of the constitution, source `ac` at .\n`statute add ac:<id>` enables a rule\n"
    );
    assert_eq!(
        read(&dir, "statute.toml"),
        "# Written by `statute init`; `statute add` and `statute remove` change the rules.\nconstitution = { source = \"ac\", edition = \"2026-10\" }\nrules = []\n\n[sources]\nac = { path = \".\" }\n"
    );
    let run = statute(&dir, &["lock"]);
    ok(&run);
    assert_eq!(run.stdout, "wrote statute.lock: 0 rules enabled\n");
    ok(&statute(&dir, &["add", "ac:3"]));
    assert!(read(&dir, "statute.lock").contains("id = 3\n"));
}

#[test]
fn init_leaves_an_existing_rule_set_as_it_is() {
    for (name, text) in [
        ("init-exists", manifest("\"ac:3\"")),
        ("init-exists-broken", "not a rule set\n".to_owned()),
    ] {
        let dir = repo(name, &text, &nodes());
        let before = std::fs::read(dir.join("statute.toml")).unwrap();
        let run = statute(&dir, &["init"]);
        assert_eq!(run.code, 1, "{}", run.stdout);
        assert!(
            run.stderr
                .contains("statute.toml: error: already exists, so it is left as it is"),
            "{}",
            run.stderr
        );
        assert_eq!(std::fs::read(dir.join("statute.toml")).unwrap(), before);
        assert!(!dir.join("statute.lock").exists());
    }
}

#[test]
fn init_with_a_source_elsewhere() {
    let source = bare_repo("init-source");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("cli")
        .join("init-elsewhere");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    ok(&statute(&dir, &["init", "--source", "../init-source"]));
    assert!(
        read(&dir, "statute.toml").contains("ac = { path = \"../init-source\" }\n"),
        "{}",
        read(&dir, "statute.toml")
    );
    ok(&statute(&dir, &["add", "ac:3"]));
    assert!(!source.join("statute.lock").exists());
    assert!(dir.join("statute.lock").exists());
}

#[test]
fn init_picks_the_edition() {
    let dir = bare_repo("init-editions");
    std::fs::create_dir_all(dir.join("constitution/2027-04")).unwrap();
    std::fs::write(
        dir.join("constitution/2027-04/index.toml"),
        "edition = \"2027-04\"\nrevision = \"1\"\n",
    )
    .unwrap();
    // A directory without an index.toml is not an edition.
    std::fs::create_dir_all(dir.join("constitution/notes")).unwrap();
    let run = statute(&dir, &["init"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains(
            "constitution: error: holds editions 2026-10, 2027-04; choose one with --edition"
        ),
        "{}",
        run.stderr
    );
    assert!(!dir.join("statute.toml").exists());
    ok(&statute(&dir, &["init", "--edition", "2027-04"]));
    assert!(read(&dir, "statute.toml").contains("edition = \"2027-04\""));
}

#[cfg(unix)]
#[test]
fn init_reports_an_edition_it_cannot_read() {
    use std::os::unix::fs::PermissionsExt;
    let dir = bare_repo("init-unreadable");
    let locked = dir.join("constitution/2027-04");
    std::fs::create_dir_all(&locked).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let blocked = std::fs::metadata(locked.join("index.toml"))
        .is_err_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied);
    let run = statute(&dir, &["init"]);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    // Run as root, the permission does not block, and there is nothing to
    // report.
    if !blocked {
        return;
    }
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(
        run.stderr
            .contains("constitution/2027-04/index.toml: error: cannot read: "),
        "{}",
        run.stderr
    );
    assert!(!dir.join("statute.toml").exists());
}

#[test]
fn init_writes_nothing_when_the_rule_set_would_not_resolve() {
    let dir = bare_repo("init-unknown-edition");
    let run = statute(&dir, &["init", "--edition", "2030-01"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr
            .contains("cannot read the constitution of edition `2030-01`"),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("statute: statute.toml is not written"));
    assert!(!dir.join("statute.toml").exists());
    std::fs::remove_dir_all(dir.join("constitution/2026-10")).unwrap();
    let run = statute(&dir, &["init"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains("holds no edition of the constitution"),
        "{}",
        run.stderr
    );
    assert!(!dir.join("statute.toml").exists());
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
