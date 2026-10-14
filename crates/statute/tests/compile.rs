//! `statute compile`, run as a user runs it, in repositories written under
//! Cargo's test directory; and on this repository's own rules.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, one_of, rule};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";
const CONSTITUTION_EN: &str = "# Constitution\n\nAgents follow it.\n";
const CONSTITUTION_ZH: &str = "# 宪法\n\nagent 遵守它。\n";

/// Domain 1: rule 2 and its child 3, one-of 4 with children 5 and 6.
/// Domain 9: rule 10, which requires the one-of. Domain 11: rule 12.
fn nodes() -> Vec<(u64, String)> {
    vec![
        domain(1),
        rule(2, 1, ""),
        rule(3, 2, ""),
        one_of(4, 1),
        rule(5, 4, ""),
        rule(6, 4, ""),
        domain(9),
        rule(10, 9, "requires = [4]"),
        domain(11),
        rule(12, 11, ""),
    ]
}

fn manifest(compile: &str) -> String {
    format!(
        "constitution = {{ source = \"ac\", edition = \"2026-10\" }}\nrules = [\"ac:3\", \"ac:5\", \"ac:10\"]\n\n[sources]\nac = {{ path = \".\" }}\n{compile}"
    )
}

fn repo(name: &str, manifest: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("compile")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("rules")).unwrap();
    let generated = dir.join("constitution/2026-10/generated");
    std::fs::create_dir_all(&generated).unwrap();
    std::fs::write(dir.join("constitution/2026-10/index.toml"), INDEX).unwrap();
    std::fs::write(generated.join("constitution.en.agent.md"), CONSTITUTION_EN).unwrap();
    std::fs::write(generated.join("constitution.zh.agent.md"), CONSTITUTION_ZH).unwrap();
    std::fs::write(dir.join("statute.toml"), manifest).unwrap();
    for (id, text) in nodes() {
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

#[track_caller]
fn ok(run: &Run) {
    assert_eq!(
        run.code, 0,
        "stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
}

/// Locks and compiles into `out.md`, and returns it.
fn compiled(dir: &Path) -> String {
    ok(&statute(dir, &["lock"]));
    ok(&statute(dir, &["compile", "--output", "out.md"]));
    std::fs::read_to_string(dir.join("out.md")).unwrap()
}

const RULES_EN: &str = r#"# Rules

<a id="rule-1"></a>
## Domain 1 (1)

<a id="rule-2"></a>
### Rule 2 (2)

Do the thing 2.

<a id="rule-3"></a>
#### Rule 3 (3)

Do the thing 3.

<a id="rule-4"></a>
### One-of 4 (4)

<a id="rule-5"></a>
#### Rule 5 (5)

Do the thing 5.

<a id="rule-9"></a>
## Domain 9 (9)

<a id="rule-10"></a>
### Rule 10 (10)

Do the thing 10.

Requires: [One-of 4](#rule-4)
"#;

#[test]
fn compiles_the_rules_along_the_tree() {
    let dir = repo("tree", &manifest(""));
    let expected = format!(
        "<!-- Written by statute from statute.toml and statute.lock; do not edit by hand. -->\n\nConstitution: edition 2026-10, revision 1, from source `ac` (`.`).\n\n# Constitution\n\nAgents follow it.\n\n{RULES_EN}"
    );
    assert_eq!(compiled(&dir), expected);
    // AGENTS.md is written only without --output.
    assert!(!dir.join("AGENTS.md").exists());
}

#[test]
fn the_same_input_compiles_to_the_same_bytes() {
    let dir = repo("same", &manifest(""));
    let first = compiled(&dir);
    assert_eq!(compiled(&dir), first);
}

#[test]
fn the_rule_set_adds_the_reason_and_the_verification() {
    let dir = repo(
        "parts",
        &manifest("\n[compile]\nagents-md = [\"statement\", \"reason\", \"verification\"]\n"),
    );
    let out = compiled(&dir);
    assert!(
        out.contains("### Rule 2 (2)\n\nDo the thing 2.\n\nReason: Because.\n\nVerification: Violated when the thing is not done.\n"),
        "{out}"
    );
    let plain = compiled(&repo("parts-plain", &manifest("")));
    assert!(!plain.contains("Reason:"), "{plain}");
}

#[test]
fn compiles_in_the_language_of_the_rule_set() {
    let dir = repo(
        "zh",
        &manifest("\n[compile]\nlang = \"zh\"\nagents-md = [\"statement\", \"reason\"]\n"),
    );
    let out = compiled(&dir);
    assert!(
        out.contains("依据的宪法：2026-10 版，修订号 1，来自规定源 `ac` (`.`)。\n\n# 宪法\n"),
        "{out}"
    );
    assert!(out.contains("\n# 规定\n"), "{out}");
    // No translation: the original.
    assert!(out.contains("Do the thing 2.\n\n理由：Because.\n"), "{out}");
    assert!(out.contains("依赖：[One-of 4](#rule-4)"), "{out}");
}

#[test]
fn compile_needs_a_current_lock_file() {
    let dir = repo("no-lock", &manifest(""));
    let run = statute(&dir, &["compile", "--output", "out.md"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains("cannot read statute.lock"),
        "{}",
        run.stderr
    );
    ok(&statute(&dir, &["lock"]));
    let rule = dir.join("rules/3.toml");
    let text = std::fs::read_to_string(&rule).unwrap();
    std::fs::write(
        &rule,
        text.replace("Do the thing 3.", "Do another thing 3."),
    )
    .unwrap();
    let run = statute(&dir, &["compile", "--output", "out.md"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains(
            "statute.lock is not what statute.toml and the rules give now; run `statute lock` first"
        ),
        "{}",
        run.stderr
    );
    assert!(!dir.join("out.md").exists());
}

#[test]
fn compile_says_which_constitution_file_is_missing() {
    let dir = repo("no-constitution", &manifest("\n[compile]\nlang = \"fr\"\n"));
    ok(&statute(&dir, &["lock"]));
    let run = statute(&dir, &["compile", "--output", "out.md"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains(
            "cannot read constitution/2026-10/generated/constitution.fr.agent.md of source `ac`"
        ) && run.stderr.contains("`uv run constitution-build` writes it"),
        "{}",
        run.stderr
    );
}

#[test]
fn this_repo_compiles_the_same_twice() {
    // This repository's constitution and rules, every domain as a pack,
    // compiled into a directory under Cargo's test directory.
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let (source, _) = statute::source::Source::load(&repo_root.join("rules"), "rules");
    let domains: Vec<String> = source
        .nodes
        .values()
        .filter(|n| n.kind == statute::node::Kind::Domain)
        .map(|n| format!("\"ac:{}\"", n.id))
        .collect();
    assert!(!domains.is_empty());
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("compile")
        .join("this-repo");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("statute.toml"),
        format!(
            "constitution = {{ source = \"ac\", edition = \"2026-10\" }}\npacks = [{}]\n\n[sources]\nac = {{ path = {:?} }}\n",
            domains.join(", "),
            repo_root.to_str().unwrap()
        ),
    )
    .unwrap();
    let first = compiled(&dir);
    assert_eq!(compiled(&dir), first);
    let constitution = std::fs::read_to_string(
        repo_root.join("constitution/2026-10/generated/constitution.en.agent.md"),
    )
    .unwrap();
    assert!(first.contains(constitution.trim_end()));
    for domain in &domains {
        let id = domain.trim_matches('"').trim_start_matches("ac:");
        assert!(
            first.contains(&format!("<a id=\"rule-{id}\"></a>\n## ")),
            "domain {id}"
        );
    }
}

#[test]
fn headings_stop_at_level_six() {
    let dir = repo(
        "deep",
        &manifest("").replace("\"ac:3\", \"ac:5\", \"ac:10\"", "\"ac:107\""),
    );
    // A chain under domain 1 (level 2): 102 to 107, each under the one
    // before, 105 at level 6.
    for (id, parent) in [
        (102, 1),
        (103, 102),
        (104, 103),
        (105, 104),
        (106, 105),
        (107, 106),
    ] {
        std::fs::write(dir.join(format!("rules/{id}.toml")), rule(id, parent, "").1).unwrap();
    }
    let out = compiled(&dir);
    assert!(out.contains("\n##### Rule 104 (104)\n"), "{out}");
    assert!(out.contains("\n###### Rule 105 (105)\n"), "{out}");
    assert!(out.contains("\n###### Rule 106 (106)\n"), "{out}");
    assert!(out.contains("\n###### Rule 107 (107)\n"), "{out}");
}
