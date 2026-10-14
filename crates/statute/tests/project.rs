//! Reading a repository from disk: its rule set, the source and the
//! constitution's revision.

use std::path::{Path, PathBuf};

use statute::project::Project;

mod common;
use common::{domain, edit, rule};

const MANIFEST: &str = r#"constitution = { source = "R", edition = "2026-10" }
rules = ["R3"]

[sources]
R = { path = "." }
"#;

const INDEX: &str = "status = \"active\"\nedition = \"2026-10\"\nrevision = \"2\"\n";

/// A fresh directory for one test, holding a repository with the given
/// rule set, rules and constitution index.
fn repo(name: &str, manifest: &str, rules: &[(u64, String)], index: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("project")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("rules")).unwrap();
    std::fs::create_dir_all(dir.join("constitution/2026-10")).unwrap();
    std::fs::write(dir.join("statute.toml"), manifest).unwrap();
    std::fs::write(dir.join("constitution/2026-10/index.toml"), index).unwrap();
    for (id, text) in rules {
        std::fs::write(dir.join(format!("rules/{id}.toml")), text).unwrap();
    }
    dir
}

fn rules() -> Vec<(u64, String)> {
    vec![domain(1), rule(2, 1, ""), rule(3, 2, "")]
}

#[track_caller]
fn fails(dir: &Path, needle: &str) {
    let (project, diags) = Project::load(dir);
    assert!(project.is_none());
    let errors: Vec<String> = diags.errors().map(ToString::to_string).collect();
    assert!(
        errors.len() == 1 && errors[0].contains(needle),
        "expected one error containing {needle:?}, got {errors:#?}"
    );
}

#[test]
fn loads_and_resolves() {
    let dir = repo("loads", MANIFEST, &rules(), INDEX);
    let (project, diags) = Project::load(&dir);
    assert!(diags.0.is_empty(), "{:#?}", diags.0);
    let project = project.unwrap();
    assert_eq!(project.revision, "2");
    assert_eq!(
        project
            .resolution
            .enabled
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        [2, 3]
    );
}

#[test]
fn the_revision_is_checked_against_each_rule() {
    let mut nodes = rules();
    nodes[1] = edit(nodes[1].clone(), ">=1", ">=3");
    fails(
        &repo("revision-range", MANIFEST, &nodes, INDEX),
        "statute.toml: error: rule R2 (Rule 2) is compatible with constitution revisions `>=3`, and the rule set rests on revision 2",
    );
}

#[test]
fn the_constitution_must_be_the_edition_named() {
    fails(
        &repo(
            "edition",
            MANIFEST,
            &rules(),
            &INDEX.replace("2026-10", "2027-01"),
        ),
        "constitution/2026-10/index.toml: error: the index is of edition `2027-01`, and the rule set names `2026-10`",
    );
    fails(
        &repo(
            "revision-text",
            MANIFEST,
            &rules(),
            &INDEX.replace("\"2\"", "\"two\""),
        ),
        "revision `two` is not a whole number",
    );
    fails(
        &repo(
            "no-edition",
            &MANIFEST.replace("2026-10", "2030-01"),
            &rules(),
            INDEX,
        ),
        "cannot read the constitution of edition `2030-01`",
    );
}

#[test]
fn errors_in_the_source_stop_the_resolution() {
    let mut nodes = rules();
    nodes[2] = edit(nodes[2].clone(), "parent = 2", "parent = 9");
    fails(
        &repo("bad-source", MANIFEST, &nodes, INDEX),
        "rules/3.toml: error: parent 9 does not exist",
    );
}

#[test]
fn a_source_elsewhere_is_named_by_its_path() {
    let dir = repo("elsewhere", MANIFEST, &[], INDEX);
    let src = dir.join("their-rules");
    std::fs::create_dir_all(src.join("rules")).unwrap();
    std::fs::create_dir_all(src.join("constitution/2026-10")).unwrap();
    std::fs::write(src.join("constitution/2026-10/index.toml"), INDEX).unwrap();
    let mut nodes = rules();
    nodes[2] = edit(nodes[2].clone(), "parent = 2", "parent = 9");
    for (id, text) in &nodes {
        std::fs::write(src.join(format!("rules/{id}.toml")), text).unwrap();
    }
    std::fs::write(
        dir.join("statute.toml"),
        MANIFEST.replace("path = \".\"", "path = \"their-rules\""),
    )
    .unwrap();
    fails(
        &dir,
        "their-rules/rules/3.toml: error: parent 9 does not exist",
    );
}
