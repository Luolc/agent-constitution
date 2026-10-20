//! The lock file, `statute.lock`: the version of statute that wrote it,
//! the constitution, the sources and every enabled rule, pinned, and the
//! commands of the enabled rules' checks (`docs/design/rule-sets.md`, Rule
//! sets and the lock file). Written by statute, never by hand; the same
//! rule set and sources always give the same bytes with the same statute.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::manifest::{
    self, Declared, FIRST_PARTY, Follow, Git, Key, RULES_DIR, RuleRef, SourceSpec,
};
use crate::node::Check;
use crate::project::Project;

pub const FILE: &str = "statute.lock";

const HEADER: &str = "# Written by statute from statute.toml; do not edit by hand.\n\n";

/// The version of statute that writes the lock file.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
struct Lock<'a> {
    /// First, on a line of its own, so that CI can read the version to
    /// install with one `sed` or `grep`.
    statute: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    constitution: Option<Constitution<'a>>,
    sources: Vec<Source<'a>>,
    rules: Vec<Rule<'a>>,
    /// The value the committed `AGENTS.md` gets of each parameter an
    /// enabled rule uses (`docs/design/rule-config.md`, Values in the lock
    /// file and the outputs).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parameters: Vec<Parameter<'a>>,
    /// The external commands of the enabled rules' checks, which CI runs
    /// beside statute's own.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    commands: Vec<Command<'a>>,
}

#[derive(Serialize)]
struct Command<'a> {
    rule: String,
    run: &'a str,
    tools: &'a [String],
}

#[derive(Serialize)]
struct Parameter<'a> {
    name: &'a str,
    value: &'a str,
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
        statute: VERSION,
        constitution: m.constitution_spec().zip(project.revision.as_deref()).map(
            |((c, spec), revision)| Constitution {
                at: pin(spec, false),
                edition: &c.edition,
                revision,
            },
        ),
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
        parameters: project
            .values
            .iter()
            .map(|(name, s)| Parameter {
                name,
                value: &s.value,
            })
            .collect(),
        commands: project
            .resolution
            .enabled
            .keys()
            .filter_map(
                |key| match &project.sources.get(key)?.rule.as_ref()?.check {
                    Check::Command { run, tools } => Some(Command {
                        rule: key.to_string(),
                        run,
                        tools,
                    }),
                    _ => None,
                },
            )
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
    /// The version of statute that wrote it; missing in a lock file
    /// written before it was recorded.
    pub statute: Option<String>,
    /// The constitution's edition and revision; `None` without one.
    pub edition: Option<String>,
    pub revision: Option<String>,
    pub rules: BTreeMap<Key, LockedRule>,
    /// The value of each parameter, by name.
    pub parameters: BTreeMap<String, String>,
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

/// The version of statute that wrote the lock file `text`, read
/// whatever else the file holds, so that a lock file of a later format
/// still gives it; `None` in a lock file written before it was recorded.
pub fn version(text: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Raw {
        statute: Option<String>,
    }
    toml::from_str::<Raw>(text).ok()?.statute
}

/// Whether statute `version` is older than this one, both read as
/// major.minor.patch; false when `version` does not read so.
pub fn older(version: &str) -> bool {
    fn numbers(v: &str) -> Option<[u64; 3]> {
        let mut parts = v.split('.').map(|p| p.parse().ok());
        let n = [parts.next()??, parts.next()??, parts.next()??];
        parts.next().is_none().then_some(n)
    }
    numbers(version)
        .zip(numbers(VERSION))
        .is_some_and(|(it, this)| it < this)
}

/// The lock file `text` that statute `other` wrote, with this version in
/// place of `other`: what this statute writes from the same rules,
/// unless the format changed between the two.
pub fn as_this(text: &str, other: &str) -> String {
    text.replacen(
        &format!("statute = \"{other}\"\n"),
        &format!("statute = \"{VERSION}\"\n"),
        1,
    )
}

#[derive(Deserialize)]
struct RawLock {
    statute: Option<String>,
    constitution: Option<RawConstitution>,
    #[serde(default)]
    rules: Vec<RawRule>,
    #[serde(default)]
    parameters: Vec<RawParameter>,
}

#[derive(Deserialize)]
struct RawParameter {
    name: String,
    value: String,
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
        statute: raw.statute,
        edition: raw.constitution.as_ref().map(|c| c.edition.clone()),
        revision: raw.constitution.map(|c| c.revision),
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
        parameters: raw
            .parameters
            .into_iter()
            .map(|p| (p.name, p.value))
            .collect(),
    })
}

/// What a lock file pins now, read back from the project that would write
/// it.
pub fn current(project: &Project) -> Locked {
    Locked {
        statute: Some(VERSION.to_owned()),
        edition: project
            .manifest
            .constitution
            .as_ref()
            .map(|c| c.edition.clone()),
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
        parameters: project
            .values
            .iter()
            .map(|(name, s)| (name.clone(), s.value.clone()))
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
/// (`docs/design/global-configuration.md`, Which source a name means).
/// A source without `public`, as in a lock file written before it was
/// recorded, counts as private, the side on which a public rule cannot
/// cite it. Text that is not a lock file, or a git source without its
/// commit or without exactly one of branch and tag, is an error rather
/// than no record, so that a broken lock file never moves a source.
pub fn sources(text: &str) -> Result<Vec<SourceSpec>, String> {
    let raw: RawPins = toml::from_str(text).map_err(|e| e.to_string().trim_end().to_owned())?;
    let mut specs = Vec::new();
    for p in raw.sources {
        let Some(address) = p.git else {
            continue;
        };
        let fail = |what: &str| format!("source `{}` {what}", p.name);
        let follow = match (p.branch, p.tag) {
            (Some(b), None) => Follow::Branch(b),
            (None, Some(t)) => Follow::Tag(t),
            _ => return Err(fail("records neither or both of `branch` and `tag`")),
        };
        let Some(commit) = p.commit else {
            return Err(fail("records no `commit`"));
        };
        let rules_dir = match p.rules_dir {
            Some(dir) => manifest::rules_dir(&dir).map_err(|e| fail(&e))?,
            None => RULES_DIR.to_owned(),
        };
        let first_party = p.name == FIRST_PARTY;
        specs.push(SourceSpec {
            git: Some(Git {
                address,
                follow,
                clone: None,
                commit: Some(commit),
            }),
            path: p.path.unwrap_or_default(),
            public: first_party || p.public == Some(true),
            name: p.name,
            declared: Declared::Lock,
            local: p.scope.as_deref() == Some("local"),
            defaults: true,
            rules_dir,
        });
    }
    Ok(specs)
}
