//! The lock file, `statute.lock`: the constitution, the source and every
//! enabled rule, pinned (`docs/design/rule-system.md`, Rule sets and the
//! lock file). Written by the tool, never by hand; the same rule set and
//! source always give the same bytes.

use serde::Serialize;

use crate::manifest::SourceSpec;
use crate::project::Project;

pub const FILE: &str = "statute.lock";

const HEADER: &str = "# Written by statute from statute.toml; do not edit by hand.\n\n";

#[derive(Serialize)]
struct Lock<'a> {
    constitution: Constitution<'a>,
    sources: Vec<Source<'a>>,
    rules: Vec<Rule<'a>>,
}

#[derive(Serialize)]
struct Constitution<'a> {
    #[serde(flatten)]
    at: Pin<'a>,
    edition: &'a str,
    revision: &'a str,
}

#[derive(Serialize)]
struct Source<'a> {
    name: &'a str,
    #[serde(flatten)]
    at: Pin<'a>,
}

/// Where a source is. A source on disk is pinned by its path alone: its
/// commit would change with the very commit that records the lock file,
/// and each rule's `sha256` already pins its content. A git source, once
/// supported, adds its address and commit here.
#[derive(Serialize)]
struct Pin<'a> {
    path: &'a str,
}

fn pin(spec: &SourceSpec) -> Pin<'_> {
    Pin { path: &spec.path }
}

#[derive(Serialize)]
struct Rule<'a> {
    source: &'a str,
    id: u64,
    revision: u64,
    sha256: &'a str,
}

/// The lock file of a resolved project, rules in order of ID.
pub fn render(project: &Project) -> String {
    let spec = &project.manifest.source;
    let lock = Lock {
        constitution: Constitution {
            at: pin(spec),
            edition: &project.manifest.edition,
            revision: &project.revision,
        },
        sources: vec![Source {
            name: &spec.name,
            at: pin(spec),
        }],
        rules: project
            .resolution
            .enabled
            .keys()
            .filter_map(|id| project.source.get(*id))
            .map(|node| Rule {
                source: &spec.name,
                id: node.id,
                revision: node.revision,
                sha256: &node.sha256,
            })
            .collect(),
    };
    let body = toml::to_string(&lock).expect("the lock file holds only strings and integers");
    format!("{HEADER}{body}")
}
