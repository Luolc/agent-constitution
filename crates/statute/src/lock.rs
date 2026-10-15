//! The lock file, `statute.lock`: the constitution, the sources and every
//! enabled rule, pinned (`docs/design/rule-sets.md`, Rule sets and the
//! lock file). Written by statute, never by hand; the same rule set and
//! sources always give the same bytes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::manifest::{Key, RuleRef, SourceSpec};
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

/// A rule, named as the rule set names it (`R42`, `team:7`), and the
/// rule that overrides it, if one does.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct Rule<'a> {
    rule: String,
    revision: u64,
    sha256: &'a str,
    original_sha256: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    overridden_by: Option<String>,
}

/// The lock file of a resolved project: the sources in order of name,
/// the rules in order of key.
pub fn render(project: &Project) -> String {
    let m = &project.manifest;
    let lock = Lock {
        constitution: Constitution {
            at: pin(m.constitution_spec()),
            edition: &m.edition,
            revision: &project.revision,
        },
        sources: m
            .sources
            .iter()
            .map(|spec| Source {
                name: &spec.name,
                at: pin(spec),
            })
            .collect(),
        rules: project
            .resolution
            .enabled
            .keys()
            .filter_map(|key| Some((key, project.sources.get(key)?)))
            .map(|(key, node)| Rule {
                rule: key.to_string(),
                revision: node.revision,
                sha256: &node.sha256,
                original_sha256: &node.original_sha256,
                overridden_by: project.resolution.overridden.get(key).map(Key::to_string),
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
    pub rules: BTreeMap<Key, LockedRule>,
}

/// A locked rule's revision and its two hashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedRule {
    pub revision: u64,
    /// The SHA-256 of the file.
    pub sha256: String,
    /// The SHA-256 of the rule without its translations; missing in a lock
    /// file written before it was recorded.
    pub original_sha256: Option<String>,
}

impl LockedRule {
    /// Whether the rule changed between `self` and `now` beyond its
    /// translations; without the hash of the original, any change counts.
    pub fn original_changed(&self, now: &LockedRule) -> bool {
        match (&self.original_sha256, &now.original_sha256) {
            (Some(was), Some(is)) => was != is,
            _ => self.sha256 != now.sha256,
        }
    }
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
    #[serde(rename = "original-sha256")]
    original_sha256: Option<String>,
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
                let locked = LockedRule {
                    revision: r.revision,
                    sha256: r.sha256,
                    original_sha256: r.original_sha256,
                };
                Ok((Key::of(&RuleRef::parse(&rule)?), locked))
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
            .filter_map(|key| Some((key, project.sources.get(key)?)))
            .map(|(key, n)| {
                let locked = LockedRule {
                    revision: n.revision,
                    sha256: n.sha256.clone(),
                    original_sha256: Some(n.original_sha256.clone()),
                };
                (key.clone(), locked)
            })
            .collect(),
    }
}
