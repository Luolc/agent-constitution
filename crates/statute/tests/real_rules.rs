//! The rules and the constitution of this repository pass statute's
//! checks, and a rule set enabling every domain as a pack resolves and
//! locks.

use std::path::Path;

use statute::node::{Kind, Status};
use statute::project::Project;
use statute::source::Source;

fn repo() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_rules_of_this_repo_pass() {
    let (source, diags) = Source::load(&repo().join("rules"), "rules");
    // No error and no warning either.
    assert!(diags.0.is_empty(), "{:#?}", diags.0);
    // Read at all: the directory holds some nodes.
    assert!(!source.nodes.is_empty());
}

#[test]
fn every_domain_of_this_repo_as_a_pack_resolves() {
    let (source, _) = Source::load(&repo().join("rules"), "rules");
    let domains: Vec<String> = source
        .nodes
        .values()
        .filter(|n| n.kind == Kind::Domain)
        .map(|n| format!("\"ac:{}\"", n.id))
        .collect();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("real-rules");
    std::fs::create_dir_all(&dir).unwrap();
    let path = repo().canonicalize().unwrap();
    let manifest = format!(
        "constitution = {{ source = \"ac\", edition = \"2026-10\" }}\npacks = [{}]\n\n[sources]\nac = {{ path = {:?} }}\n",
        domains.join(", "),
        path.to_str().unwrap()
    );
    let (project, diags) = Project::with_manifest(&dir, &manifest);
    assert!(diags.0.is_empty(), "{:#?}", diags.0);
    // Every rule in force is enabled; the revision is the constitution's.
    let in_force = source
        .nodes
        .values()
        .filter(|n| n.kind == Kind::Rule)
        .filter(|n| !matches!(n.status, Status::Superseded | Status::Repealed))
        .count();
    assert!(in_force > 0, "no rule in force to check");
    let project = project.unwrap();
    assert_eq!(project.resolution.enabled.len(), in_force);
    assert!(project.revision.parse::<u64>().is_ok());
    let lock = statute::lock::render(&project);
    assert_eq!(lock.matches("[[rules]]").count(), in_force, "{lock}");
}
