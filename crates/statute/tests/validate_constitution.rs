//! `statute validate` with the constitution beside the rules: what the
//! constitution decides of them, and with `--base`, what a base commit
//! had of them (`docs/design/checkers.md`). Each check fails on a broken
//! tree and passes on its control, the tree as set up.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, rule};

const INDEX: &str = r#"status = "draft"
edition = "2026-10"
revision = "0.1"
languages = ["en"]

[title]
en = "Test Constitution"

[front]
en = "The preamble."
"#;

const ALPHA: &str = r#"id = "alpha"

[title]
en = "First"
"#;

const ALPHA_ONE: &str = r#"id = "alpha-one"
status = "draft"

[content]
en = "One."

[[items]]
id = "money"
number = "1"
status = "draft"

[items.content]
en = "Money."
"#;

const BETA: &str = r#"id = "beta"

[title]
en = "Second"
"#;

const BETA_ONE: &str = r#"id = "beta-one"
status = "draft"

[content]
en = "Four."
"#;

const IDX: &str = "constitution/2026-10/index.toml";
const A: &str = "constitution/2026-10/articles/01-alpha";
const B: &str = "constitution/2026-10/articles/02-beta";
const SECTIONS: [&str; 2] = [
    "constitution/2026-10/articles/01-alpha/01-alpha-one.toml",
    "constitution/2026-10/articles/02-beta/01-beta-one.toml",
];

/// The warning on the tree as set up: rule 10 names `alpha` and its item
/// `alpha-one.money`.
const WARNING: &str = "rules/10.toml:8: warning: rule 10: authority names both alpha and alpha-one.money, which is under it";

struct Run {
    code: i32,
    stdout: String,
    stderr: Vec<String>,
}

/// A constitution of two articles and a rule tree beside it: domain 2 and
/// rule 10, which cites `beta-one` and names `alpha` and `alpha-one.money`
/// in its `authority`; all draft.
struct Tree {
    root: PathBuf,
}

impl Tree {
    fn new(name: &str) -> Tree {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("validate-constitution")
            .join(name);
        let _ = std::fs::remove_dir_all(&root);
        let tree = Tree { root };
        tree.write(IDX, INDEX);
        tree.write(&format!("{A}/article.toml"), ALPHA);
        tree.write(SECTIONS[0], ALPHA_ONE);
        tree.write(&format!("{B}/article.toml"), BETA);
        tree.write(SECTIONS[1], BETA_ONE);
        tree.node(edit(domain(2), "status = \"active\"", "status = \"draft\""));
        let ten = edit(
            rule(10, 2, ""),
            "authority = [\"simplicity\"]",
            "authority = [\"alpha\", \"alpha-one.money\"]",
        );
        tree.node(edit(
            ten,
            "Do the thing 10.",
            "Do the thing {@constitution:beta-one}.",
        ));
        tree
    }

    fn write(&self, name: &str, text: &str) {
        let path = self.root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn node(&self, (id, text): (u64, String)) {
        self.write(&format!("rules/{id}.toml"), &text);
    }

    fn remove(&self, name: &str) {
        std::fs::remove_file(self.root.join(name)).unwrap();
    }

    /// Replaces the first `old` with `new` in the file, or each with `all`.
    fn edit_all(&self, name: &str, old: &str, new: &str, all: bool) {
        let text = std::fs::read_to_string(self.root.join(name)).unwrap();
        assert!(text.contains(old), "{name} lacks {old:?}");
        let text = if all {
            text.replace(old, new)
        } else {
            text.replacen(old, new, 1)
        };
        self.write(name, &text);
    }

    fn edit(&self, name: &str, old: &str, new: &str) {
        self.edit_all(name, old, new, false);
    }

    /// Makes the constitution and the rules active.
    fn activate(&self) {
        self.edit(IDX, "status = \"draft\"", "status = \"active\"");
        self.edit(IDX, "revision = \"0.1\"", "revision = \"1\"");
        for name in SECTIONS {
            self.edit_all(name, "status = \"draft\"", "status = \"active\"", true);
        }
        for id in [2, 10] {
            self.edit(
                &format!("rules/{id}.toml"),
                "status = \"draft\"",
                "status = \"active\"",
            );
        }
    }

    /// A rule 11 with rule 10's `authority`, superseded by rule 10.
    fn superseded(&self) {
        let eleven = edit(
            rule(11, 2, "superseded-by = 10"),
            "status = \"draft\"",
            "status = \"superseded\"",
        );
        self.node(edit(
            eleven,
            "authority = [\"simplicity\"]",
            "authority = [\"alpha\", \"alpha-one.money\"]",
        ));
    }

    fn validate(&self, args: &[&str]) -> Run {
        let out = Command::new(env!("CARGO_BIN_EXE_statute"))
            .current_dir(&self.root)
            .args(["validate", "rules"])
            .args(args)
            .output()
            .unwrap();
        Run {
            code: out.status.code().unwrap(),
            stdout: String::from_utf8(out.stdout).unwrap(),
            stderr: String::from_utf8(out.stderr)
                .unwrap()
                .lines()
                .map(str::to_owned)
                .collect(),
        }
    }

    /// Asserts that `validate` fails with exactly these lines besides the
    /// warning of the tree as set up.
    fn assert_fails(&self, args: &[&str], want: &[&str]) {
        let run = self.validate(args);
        assert_eq!(run.code, 1, "{:?}", run.stderr);
        let got: Vec<&str> = run
            .stderr
            .iter()
            .map(String::as_str)
            .filter(|l| *l != WARNING)
            .collect();
        assert_eq!(got, want);
    }

    fn assert_passes(&self, args: &[&str]) -> Run {
        let run = self.validate(args);
        assert_eq!(run.code, 0, "{:?}", run.stderr);
        run
    }

    fn git(&self, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.test",
                "-c",
                "maintenance.auto=false",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_WORK_TREE")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Commits the tree, a repository from the first call on.
    fn commit(&self, message: &str) {
        if !self.root.join(".git").exists() {
            self.git(&["init", "-q"]);
        }
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message]);
    }

    /// Commits the tree, made active first, as the base `HEAD`.
    fn commit_base(&self) {
        self.activate();
        self.commit("base");
    }
}

#[test]
fn controls_pass_with_the_overlap_warning() {
    let tree = Tree::new("controls");
    let run = tree.assert_passes(&[]);
    assert_eq!(run.stderr, [WARNING]);
    assert_eq!(run.stdout, "rules: 2 nodes, no errors\n");
}

#[test]
fn item_under_its_section_warns() {
    let tree = Tree::new("overlap-section");
    tree.edit("rules/10.toml", "\"alpha\",", "\"alpha-one\",");
    let run = tree.assert_passes(&[]);
    assert_eq!(
        run.stderr,
        [
            "rules/10.toml:8: warning: rule 10: authority names both alpha-one and alpha-one.money, which is under it"
        ]
    );
}

#[test]
fn provisions_apart_do_not_warn() {
    let tree = Tree::new("overlap-apart");
    tree.edit("rules/10.toml", "\"alpha\",", "\"beta\",");
    assert!(tree.assert_passes(&[]).stderr.is_empty());
}

#[test]
fn superseded_rule_does_not_warn() {
    let tree = Tree::new("overlap-superseded");
    tree.activate();
    tree.superseded();
    assert_eq!(tree.assert_passes(&[]).stderr, [WARNING]);
}

#[test]
fn authority_resolves() {
    let tree = Tree::new("authority-undefined");
    tree.edit("rules/10.toml", "\"alpha-one.money\"", "\"alpha-one.cash\"");
    tree.assert_fails(
        &[],
        &["rules/10.toml:8: error: authority cites undefined alpha-one.cash"],
    );
    // A namespace is the node format's error, reported once.
    tree.edit(
        "rules/10.toml",
        "\"alpha-one.cash\"",
        "\"constitution:alpha\"",
    );
    tree.assert_fails(
        &[],
        &["rules/10.toml: error: `authority` entry `constitution:alpha` must be a provision ID without a namespace, such as \"simplicity\" or \"must-ask.capital\""],
    );
}

#[test]
fn citation_resolves() {
    let tree = Tree::new("citation-undefined");
    tree.edit(
        "rules/10.toml",
        "{@constitution:beta-one}",
        "{@constitution:beta-two}",
    );
    tree.assert_fails(
        &[],
        &["rules/10.toml:18: error: cites undefined constitution:beta-two"],
    );
}

#[test]
fn repealed_provision_fails_where_a_rule_names_it() {
    let tree = Tree::new("repealed-provision");
    tree.activate();
    tree.edit(
        SECTIONS[1],
        "status = \"active\"",
        "status = \"repealed\"\nrepealed = \"2026-10\"",
    );
    tree.assert_fails(
        &[],
        &["rules/10.toml:18: error: cites repealed constitution:beta-one"],
    );
    tree.edit(
        "rules/10.toml",
        "{@constitution:beta-one}",
        "{@constitution:alpha}",
    );
    tree.edit("rules/10.toml", "\"alpha\",", "\"beta-one\",");
    tree.assert_fails(
        &[],
        &["rules/10.toml:8: error: authority cites repealed beta-one"],
    );
}

#[test]
fn repealed_rule_is_not_checked() {
    let tree = Tree::new("repealed-rule");
    tree.activate();
    tree.edit("rules/10.toml", "\"alpha-one.money\"", "\"alpha-one.cash\"");
    tree.edit(
        "rules/10.toml",
        "status = \"active\"",
        "status = \"repealed\"",
    );
    let run = tree.assert_passes(&[]);
    assert!(run.stderr.is_empty(), "{:?}", run.stderr);
}

#[test]
fn status_agrees_with_the_index() {
    let tree = Tree::new("status");
    tree.edit("rules/2.toml", "status = \"draft\"", "status = \"trial\"");
    tree.assert_fails(
        &[],
        &["rules/2.toml:5: error: a domain is trial while index.toml is draft"],
    );
}

#[test]
fn draft_rule_under_an_active_index_passes() {
    let tree = Tree::new("draft-under-active");
    tree.activate();
    tree.edit("rules/10.toml", "status = \"active\"", "status = \"draft\"");
    tree.assert_passes(&[]);
}

#[test]
fn a_constitution_that_does_not_load_is_one_error() {
    let tree = Tree::new("no-load");
    tree.edit(SECTIONS[1], "status = \"draft\"", "status = \"active\"");
    tree.assert_fails(
        &[],
        &["constitution: error: does not load, so the rules are not checked against it; `statute constitution build --check` lists its errors"],
    );
}

#[test]
fn base_needs_the_constitution() {
    let tree = Tree::new("base-no-constitution-beside");
    std::fs::remove_dir_all(tree.root.join("constitution")).unwrap();
    assert_eq!(tree.validate(&[]).code, 0);
    tree.assert_fails(
        &["--base", "HEAD"],
        &["rules: error: --base compares with the constitution, and there is no constitution/ beside it"],
    );
}

#[test]
fn node_is_never_deleted() {
    let tree = Tree::new("base-deleted");
    tree.superseded();
    tree.commit_base();
    tree.assert_passes(&["--base", "HEAD"]);
    tree.remove("rules/11.toml");
    tree.assert_fails(
        &["--base", "HEAD"],
        &["rules/11.toml:1: error: rule 11 existed at base HEAD and is gone; repeal it instead"],
    );
}

#[test]
fn superseded_stays_superseded() {
    let tree = Tree::new("base-superseded");
    tree.superseded();
    tree.commit_base();
    tree.edit(
        "rules/11.toml",
        "status = \"superseded\"",
        "status = \"active\"",
    );
    tree.edit("rules/11.toml", "superseded-by = 10\n", "");
    tree.assert_fails(
        &["--base", "HEAD"],
        &[
            "rules/11.toml:8: warning: rule 11: authority names both alpha and alpha-one.money, which is under it",
            "rules/11.toml:4: error: rule 11 was superseded at base HEAD and is no longer",
        ],
    );
}

#[test]
fn repealed_stays_repealed() {
    let tree = Tree::new("base-repealed");
    tree.activate();
    tree.edit(
        "rules/10.toml",
        "status = \"active\"",
        "status = \"repealed\"",
    );
    tree.commit("base");
    tree.edit(
        "rules/10.toml",
        "status = \"repealed\"",
        "status = \"active\"",
    );
    tree.assert_fails(
        &["--base", "HEAD"],
        &["rules/10.toml:4: error: rule 10 was repealed at base HEAD and is no longer"],
    );
}

#[test]
fn older_rule_lives_on_under_its_slug() {
    let tree = Tree::new("base-older");
    tree.commit_base();
    tree.write("rules/rule.toml", "id = \"rule\"\nstatus = \"active\"\n");
    tree.commit("a rule of the older format");
    tree.remove("rules/rule.toml");
    tree.assert_fails(
        &["--base", "HEAD"],
        &["rules/rule.toml:1: error: rule rule existed at base HEAD and no node has it as its slug"],
    );
    tree.edit(
        "rules/10.toml",
        "revision = 1",
        "slug = \"rule\"\nrevision = 1",
    );
    tree.assert_passes(&["--base", "HEAD"]);
}

#[test]
fn params_toml_is_no_node() {
    let tree = Tree::new("base-params");
    tree.write(
        "rules/params.toml",
        "[[parameter]]\nname = \"language.docs\"\ntype = \"language\"\nscope = \"repo\"\ndefault = \"en\"\n\n[parameter.title]\nen = \"Language of the docs\"\n",
    );
    tree.commit_base();
    tree.assert_passes(&["--base", "HEAD"]);
}

#[test]
fn base_must_be_a_commit() {
    let tree = Tree::new("base-not-a-commit");
    tree.commit_base();
    tree.assert_fails(
        &["--base", "nope"],
        &["constitution/2026-10/index.toml:1: error: base `nope` is not a commit"],
    );
}

#[test]
fn draft_base_is_not_compared() {
    let tree = Tree::new("base-draft");
    tree.superseded();
    tree.edit(
        "rules/11.toml",
        "status = \"superseded\"",
        "status = \"draft\"",
    );
    tree.edit("rules/11.toml", "superseded-by = 10\n", "");
    tree.commit("draft base");
    tree.remove("rules/11.toml");
    let run = tree.assert_passes(&["--base", "HEAD"]);
    assert_eq!(
        run.stdout,
        "base HEAD is draft; numbering not compared\nrules: 2 nodes, no errors\n"
    );
}

#[test]
fn dot_inside_the_rules_directory_finds_the_constitution() {
    let tree = Tree::new("dot");
    tree.edit("rules/10.toml", "\"alpha-one.money\"", "\"alpha-one.cash\"");
    let out = Command::new(env!("CARGO_BIN_EXE_statute"))
        .current_dir(tree.root.join("rules"))
        .args(["validate", "."])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "./10.toml:8: error: authority cites undefined alpha-one.cash\n"
    );
}
