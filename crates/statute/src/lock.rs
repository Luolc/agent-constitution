//! The lock file, `statute.lock`: the constitution, the source and every
//! enabled rule, pinned (`docs/design/rule-system.md`, Rule sets and the
//! lock file). Written by the tool, never by hand; the same rule set and
//! source always give the same bytes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::manifest::{RuleRef, SourceSpec};
use crate::node::Id;
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

/// A rule, named as the rule set names it (`R42`, `team:7`).
#[derive(Serialize)]
struct Rule<'a> {
    rule: String,
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
                rule: project.source.show(node.id),
                revision: node.revision,
                sha256: &node.sha256,
            })
            .collect(),
    };
    let body = toml::to_string(&lock).expect("the lock file holds only strings and integers");
    format!("{HEADER}{body}")
}

/// What a lock file pins, as `upgrade` compares it with a new resolution.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Locked {
    pub edition: String,
    pub revision: String,
    /// Each locked rule's revision and SHA-256, by ID.
    pub rules: BTreeMap<Id, (u64, String)>,
}

#[derive(Deserialize)]
struct RawLock {
    constitution: RawConstitution,
    #[serde(default)]
    rules: Vec<RawRule>,
}

#[derive(Deserialize)]
struct RawConstitution {
    edition: String,
    revision: String,
}

#[derive(Deserialize)]
struct RawRule {
    /// Missing in a lock file of the earlier format, which named a rule
    /// by `source` and `id`.
    rule: Option<String>,
    revision: u64,
    sha256: String,
}

/// Reads the text of a lock file.
pub fn read(text: &str) -> Result<Locked, String> {
    let raw: RawLock = toml::from_str(text).map_err(|e| e.to_string().trim_end().to_owned())?;
    Ok(Locked {
        edition: raw.constitution.edition,
        revision: raw.constitution.revision,
        rules: raw
            .rules
            .into_iter()
            .map(|r| {
                let rule = r.rule.ok_or(
                    "a rule without `rule`, as in a lock file of an earlier format; run `statute lock` to write it anew",
                )?;
                Ok((RuleRef::parse(&rule)?.id, (r.revision, r.sha256)))
            })
            .collect::<Result<_, String>>()?,
    })
}

/// What a lock file pins now, read back from the project that would write
/// it.
pub fn current(project: &Project) -> Locked {
    Locked {
        edition: project.manifest.edition.clone(),
        revision: project.revision.clone(),
        rules: project
            .resolution
            .enabled
            .keys()
            .filter_map(|id| project.source.get(*id))
            .map(|n| (n.id, (n.revision, n.sha256.clone())))
            .collect(),
    }
}
