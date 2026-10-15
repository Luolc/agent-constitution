//! The rules and the constitution of this repository pass statute's
//! checks, and a rule set enabling every domain as a pack, and naming
//! one child of each one-of, resolves, locks and makes the local site.

use std::collections::BTreeMap;
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
        .map(|n| format!("\"R{}\"", n.id))
        .collect();
    let in_force = |n: &&statute::node::Node| {
        n.kind == Kind::Rule && !matches!(n.status, Status::Superseded | Status::Repealed)
    };
    // A pack enters a one-of only through a child the rule set names: the
    // lowest ID among its children in force.
    let mut chosen: BTreeMap<u64, u64> = BTreeMap::new();
    for n in source.nodes.values().filter(in_force) {
        if let Some(p) = n.parent
            && source.nodes[&p].kind == Kind::OneOf
        {
            let c = chosen.entry(p).or_insert(n.id);
            *c = (*c).min(n.id);
        }
    }
    let named: Vec<String> = chosen.values().map(|id| format!("\"R{id}\"")).collect();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("real-rules");
    std::fs::create_dir_all(&dir).unwrap();
    let path = repo().canonicalize().unwrap();
    let manifest = format!(
        "constitution = {{ source = \"R\", edition = \"2026-10\" }}\npublic = false\npacks = [{}]\nrules = [{}]\n\n[sources]\nR = {{ path = {:?} }}\n",
        domains.join(", "),
        named.join(", "),
        path.to_str().unwrap()
    );
    let (project, diags) = Project::with_manifest(&dir, &manifest, &Default::default());
    assert!(diags.0.is_empty(), "{:#?}", diags.0);
    // Every rule in force is enabled, except the children of a one-of that
    // the rule set does not name and the rules below them; the revision is
    // the constitution's.
    let in_force = source
        .nodes
        .values()
        .filter(in_force)
        .filter(|n| {
            let mut id = n.id;
            while let Some(p) = source.nodes[&id].parent {
                if source.nodes[&p].kind == Kind::OneOf && chosen.get(&p) != Some(&id) {
                    return false;
                }
                id = p;
            }
            true
        })
        .count();
    assert!(in_force > 0, "no rule in force to check");
    let project = project.unwrap();
    assert_eq!(project.resolution.enabled.len(), in_force);
    assert!(project.revision.parse::<u64>().is_ok());
    let lock = statute::lock::render(&project);
    assert_eq!(lock.matches("[[rules]]").count(), in_force, "{lock}");
    // The local site has a page for every node, and the index.
    let pages = statute::site::pages(&project).unwrap();
    assert_eq!(pages.len(), source.nodes.len() + 1);
}
