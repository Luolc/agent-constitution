//! The lock file, `statute.lock`: the constitution, the sources and every
//! enabled rule, pinned (`docs/design/rule-sets.md`, Rule sets and the
//! lock file). Written by statute, never by hand; the same rule set and
//! sources always give the same bytes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::manifest::{
    self, Declared, FIRST_PARTY, Follow, Git, Key, RULES_DIR, RuleRef, SourceSpec,
};
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
    /// Every source's but `R`'s, so that a source read from the lock file
    /// alone is checked for which rules may cite which.
    #[serde(skip_serializing_if = "Option::is_none")]
    public: Option<bool>,
    /// A `rules-dir` other than `rules`, for the same reason.
    #[serde(rename = "rules-dir", skip_serializing_if = "Option::is_none")]
    rules_dir: Option<&'a str>,
}

/// Where a source is. A source on disk is pinned by its path alone: its
/// commit would change with the very commit that records the lock file,
/// and each rule's `sha256` already pins its content. A git source is
/// pinned by its address and commit, with the branch or tag it follows
/// (`docs/design/global-configuration.md`, Git sources).
#[derive(Serialize)]
#[serde(untagged)]
enum Pin<'a> {
    Path {
        path: &'a str,
    },
    Git {
        git: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tag: Option<&'a str>,
        commit: &'a str,
    },
}

/// `follow` is false for the constitution, whose source's entry says what
/// it follows.
fn pin(spec: &SourceSpec, follow: bool) -> Pin<'_> {
    match &spec.git {
        None => Pin::Path { path: &spec.path },
        Some(git) => {
            let (branch, tag) = match (&git.follow, follow) {
                (Follow::Branch(b), true) => (Some(b.as_str()), None),
                (Follow::Tag(t), true) => (None, Some(t.as_str())),
                _ => (None, None),
            };
            Pin::Git {
                git: &git.address,
                branch,
                tag,
                commit: git.commit.as_deref().unwrap_or(""),
            }
        }
    }
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
    /// `"default"` for a rule the defaults alone enable
    /// (`docs/design/rule-config.md`, Defaults in the lock file).
    #[serde(skip_serializing_if = "Option::is_none")]
    by: Option<&'static str>,
}

/// The lock file of a resolved project: the sources in order of name,
/// the rules in order of key.
pub fn render(project: &Project) -> String {
    let m = &project.manifest;
    let chosen = project.resolution.chosen();
    let lock = Lock {
        constitution: Constitution {
            at: pin(m.constitution_spec(), false),
            edition: &m.edition,
            revision: &project.revision,
        },
        sources: m
            .sources
            .iter()
            .map(|spec| Source {
                name: &spec.name,
                at: pin(spec, true),
                public: (spec.name != FIRST_PARTY).then_some(spec.public),
                rules_dir: (spec.rules_dir != RULES_DIR).then_some(spec.rules_dir.as_str()),
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
                by: (!chosen.contains(key)).then_some("default"),
            })
            .collect(),
    };
    let body = toml::to_string(&lock).expect("the lock file holds only strings and integers");
    format!("{HEADER}{body}")
}

const PRIVATE_HEADER: &str = "# Written by statute from private.toml; do not edit by hand.\n\n";

#[derive(Serialize)]
struct PrivateLock<'a> {
    sources: Vec<PrivateSource<'a>>,
}

#[derive(Serialize)]
struct PrivateSource<'a> {
    name: &'a str,
    #[serde(flatten)]
    at: Pin<'a>,
    public: bool,
    scope: &'a str,
    #[serde(rename = "rules-dir", skip_serializing_if = "Option::is_none")]
    rules_dir: Option<&'a str>,
}

/// The private lock file of `both`, the project with both layers over
/// `public`: the private layer's own sources, those `public` does not
/// have, and nothing else, so that worktrees on different branches share
/// it (`docs/design/rule-sources.md`, Private rules in a public
/// repository).
pub fn render_private(public: &Project, both: &Project) -> String {
    let lock = PrivateLock {
        sources: both
            .manifest
            .sources
            .iter()
            .filter(|s| public.manifest.spec(&s.name).is_none())
            .map(|spec| PrivateSource {
                name: &spec.name,
                at: pin(spec, true),
                public: spec.public,
                scope: if spec.local { "local" } else { "global" },
                rules_dir: (spec.rules_dir != RULES_DIR).then_some(spec.rules_dir.as_str()),
            })
            .collect(),
    };
    let body = toml::to_string(&lock).expect("the lock file holds only strings and booleans");
    format!("{PRIVATE_HEADER}{body}")
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

#[derive(Deserialize)]
struct RawPins {
    #[serde(default)]
    sources: Vec<RawPin>,
}

#[derive(Deserialize)]
struct RawPin {
    name: String,
    path: Option<String>,
    git: Option<String>,
    branch: Option<String>,
    tag: Option<String>,
    commit: Option<String>,
    public: Option<bool>,
    scope: Option<String>,
    #[serde(rename = "rules-dir")]
    rules_dir: Option<String>,
}

/// The git sources a lock file's text records, each with its commit
/// (`docs/design/global-configuration.md`, Which source a name means);
/// none when the text cannot be read. A source without `public`, as in
/// a lock file written before it was recorded, counts as private, the
/// side on which a public rule cannot cite it.
pub fn sources(text: &str) -> Vec<SourceSpec> {
    let Ok(raw) = toml::from_str::<RawPins>(text) else {
        return Vec::new();
    };
    raw.sources
        .into_iter()
        .filter_map(|p| {
            let follow = match (p.branch, p.tag) {
                (Some(b), None) => Follow::Branch(b),
                (None, Some(t)) => Follow::Tag(t),
                _ => return None,
            };
            let rules_dir = match p.rules_dir {
                Some(dir) => manifest::rules_dir(&dir).ok()?,
                None => RULES_DIR.to_owned(),
            };
            let first_party = p.name == FIRST_PARTY;
            Some(SourceSpec {
                git: Some(Git {
                    address: p.git?,
                    follow,
                    clone: None,
                    commit: Some(p.commit?),
                }),
                path: p.path.unwrap_or_default(),
                public: first_party || p.public == Some(true),
                name: p.name,
                declared: Declared::Lock,
                local: p.scope.as_deref() == Some("local"),
                defaults: true,
                rules_dir,
            })
        })
        .collect()
}
