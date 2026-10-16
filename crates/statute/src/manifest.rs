//! The rule set, `statute.toml`: the constitution it rests on, its
//! sources, the rules and packs it enables, and what the compiled output
//! carries (`docs/design/rule-sets.md`, Rule sets and the lock file).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::node::{self, Id, id_from_file_name};

pub const FILE: &str = "statute.toml";

/// The key of `[sources]` that names the first-party source, the
/// repository that publishes the constitution, and the prefix of its
/// rules: `R42`.
pub const FIRST_PARTY: &str = "R";

/// The prefix of a local rule, one of the repository's own `rules/`:
/// `L12`.
pub const LOCAL: &str = "L";
/// The directory of the repository's own rules when the rule set names no
/// other in `local-rules` (`docs/design/rule-config.md`, The directory of
/// local rules).
pub const LOCAL_RULES: &str = ".statute/rules";

/// A source's rules directory, below its top, when `rules-dir` names no
/// other.
pub const RULES_DIR: &str = "rules";

/// Which source a reference in a rule set names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// `R42`.
    FirstParty,
    /// `12`, `L12` or `local:12`: the rule set's own repository.
    Local,
    /// `team:7` or `P7`, a source `[sources]` names.
    Named(String),
}

/// A reference to a rule, as a rule set and the command line write it
/// (`docs/design/rule-sets.md`, Sources and references).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleRef {
    pub origin: Origin,
    pub id: Id,
}

/// How rule `id` of the source named `name` in `[sources]` is shown:
/// `R42` for the first-party source, `L12` for the repository's own
/// rules, `P7` for another whose name is capital letters, `team:7` for
/// one whose name is a word, `42` when the source has no name, as when
/// the rules of a directory are checked alone.
pub fn show(name: &str, id: Id) -> String {
    match name {
        "" => id.to_string(),
        _ if is_letters(name) => format!("{name}{id}"),
        _ => format!("{name}:{id}"),
    }
}

/// Whether a source's name is one or two capital letters, as `R` and
/// `P` are, which its references write before the ID with no colon.
pub fn is_letters(name: &str) -> bool {
    (1..=2).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_uppercase())
}

/// A node of a rule set: the name of its source, `L` for the
/// repository's own rules, and its ID. Ordered as the compiled file
/// shows the sources: the first-party source, the others by name, then
/// the repository's own rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub source: String,
    pub id: Id,
}

impl Key {
    pub fn new(source: &str, id: Id) -> Key {
        Key {
            source: source.to_owned(),
            id,
        }
    }

    fn rank(&self) -> u8 {
        match self.source.as_str() {
            FIRST_PARTY => 0,
            LOCAL => 2,
            _ => 1,
        }
    }

    /// The key a reference names when its source is the one its form
    /// says: `R42`, `L12`, `P7` and `team:7` as written.
    pub fn of(r: &RuleRef) -> Key {
        let source = match &r.origin {
            Origin::FirstParty => FIRST_PARTY,
            Origin::Local => LOCAL,
            Origin::Named(name) => name,
        };
        Key::new(source, r.id)
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.rank(), &self.source, self.id).cmp(&(other.rank(), &other.source, other.id))
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&show(&self.source, self.id))
    }
}

/// The `[sources]` keys no source may have, with why: `L` and `local`
/// stand for the repository's own rules, and `ac` is the first-party
/// source's former name.
fn reserved(name: &str) -> Option<String> {
    match name {
        LOCAL | "local" => Some(format!(
            "`{name}` stands for this repository's own rules, so no source may be named so"
        )),
        "ac" => Some(format!(
            "`ac` is the former name of the first-party source; name it `{FIRST_PARTY}`, and write its rules as `{FIRST_PARTY}42`"
        )),
        _ => None,
    }
}

fn number(text: &str) -> Option<Id> {
    id_from_file_name(&format!("{text}.toml"))
}

impl RuleRef {
    pub fn parse(text: &str) -> Result<RuleRef, String> {
        let at = |origin| move |id| RuleRef { origin, id };
        let parsed = match text.split_once(':') {
            None => {
                let (name, id) =
                    text.split_at(text.find(|c: char| c.is_ascii_digit()).unwrap_or(0));
                match (name, number(id)) {
                    ("" | LOCAL, Some(id)) => Some(at(Origin::Local)(id)),
                    (FIRST_PARTY, Some(id)) => Some(at(Origin::FirstParty)(id)),
                    (_, Some(id)) if is_letters(name) => {
                        Some(at(Origin::Named(name.to_owned()))(id))
                    }
                    _ => None,
                }
            }
            Some((name, id)) => {
                let id = number(id);
                match (name, id) {
                    ("local", Some(id)) => Some(at(Origin::Local)(id)),
                    (FIRST_PARTY | "ac", Some(id)) => {
                        return Err(format!(
                            "`{text}`: the first-party source's rules are written `{FIRST_PARTY}{id}`"
                        ));
                    }
                    (_, Some(id)) if is_letters(name) => {
                        return Err(format!(
                            "`{text}`: the rules of a source named in capital letters are written with no colon, `{name}{id}`"
                        ));
                    }
                    (_, Some(id)) if node::is_slug(name) => {
                        Some(at(Origin::Named(name.to_owned()))(id))
                    }
                    _ => None,
                }
            }
        };
        parsed.ok_or_else(|| {
            format!(
                "`{text}` is not a reference; write `{FIRST_PARTY}42` for a first-party rule, `42` for one of this repository's own rules, `P42` for one of a source named in one or two capital letters, or `<source>:42` for one of a source named in lowercase words"
            )
        })
    }
}

impl std::fmt::Display for RuleRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.origin {
            Origin::FirstParty => write!(f, "{FIRST_PARTY}{}", self.id),
            Origin::Local => write!(f, "{LOCAL}{}", self.id),
            Origin::Named(name) => f.write_str(&show(name, self.id)),
        }
    }
}

/// What a git source follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Follow {
    Branch(String),
    Tag(String),
}

impl std::fmt::Display for Follow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Follow::Branch(b) => write!(f, "branch `{b}`"),
            Follow::Tag(t) => write!(f, "tag `{t}`"),
        }
    }
}

/// A git source (`docs/design/global-configuration.md`, Git sources).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Git {
    /// The address `git clone` takes, as written.
    pub address: String,
    pub follow: Follow,
    /// The user's own clone, read in place of statute's.
    pub clone: Option<String>,
    /// The commit read, once chosen.
    pub commit: Option<String>,
}

/// Where a source's entry comes from (`docs/design/global-configuration.md`,
/// Which source a name means).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Declared {
    /// The rule set's own `[sources]`.
    RuleSet,
    /// The global configuration.
    Global,
    /// The lock file, for a source neither of the above names.
    Lock,
    /// statute's own default, `R`.
    BuiltIn,
}

/// A source of rules: a directory on disk, its `path` relative to the
/// directory of `statute.toml`, or a git repository read at one commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpec {
    pub name: String,
    /// The directory read: as written for a source on disk; for a git
    /// source, the files of its commit once chosen
    /// (`crate::global::materialize`), empty before.
    pub path: String,
    pub git: Option<Git>,
    pub declared: Declared,
    /// Whether anyone may read the source, as the rule set declares it.
    pub public: bool,
    /// `scope = "local"`: the rules are this repository's, as those of
    /// its own `rules/` are, and rank with them for overrides.
    pub local: bool,
    /// `defaults = false` in the rule set's entry takes none of the
    /// source's defaults (`docs/design/rule-config.md`, Whose defaults).
    pub defaults: bool,
    /// The directory of the source's rules, below its top: `rules-dir`,
    /// `rules` by default.
    pub rules_dir: String,
}

impl SourceSpec {
    /// Whether the source is the rule set's own directory, whose `rules/`
    /// are then the source's rules rather than local ones.
    pub fn is_here(&self) -> bool {
        self.git.is_none() && matches!(self.path.trim_end_matches('/'), "." | "")
    }

    /// Whether the source's rules directory is that of the repository's
    /// own rules, `local_rules`, so that those rules are the source's
    /// (`docs/design/rule-config.md`, The directory of local rules).
    pub fn holds(&self, local_rules: &str) -> bool {
        self.git.is_none()
            && normal(&Path::new(&self.path).join(&self.rules_dir))
                == normal(Path::new(local_rules))
    }

    /// How a path below the source is named in messages: `rules` for the
    /// source `.`, `../team/rules` for `../team`, `R@1a2b3c4d:rules` for a
    /// git source at a commit.
    pub fn shown(&self, below: &str) -> String {
        match &self.git {
            Some(git) => {
                let commit = git.commit.as_deref().unwrap_or("");
                format!("{}@{}:{below}", self.name, &commit[..commit.len().min(8)])
            }
            None => crate::project::shown(&self.path, below),
        }
    }

    /// Where the source is, for the compiled `AGENTS.md`: its path, or its
    /// address at the commit.
    pub fn place(&self) -> String {
        match &self.git {
            Some(git) => format!("{} at {}", git.address, git.commit.as_deref().unwrap_or("")),
            None => self.path.clone(),
        }
    }
}

/// The source entries a rule set draws on besides its own
/// (`docs/design/global-configuration.md`, Which source a name means):
/// those of the global configuration, and those of the lock file, which
/// count only where nothing else names a source.
#[derive(Debug, Clone, Default)]
pub struct Layers {
    pub global: Vec<SourceSpec>,
    /// The parameter values of the global configuration, by name.
    pub values: BTreeMap<String, String>,
    pub locked: Vec<SourceSpec>,
    /// Whether `R` is the built-in default when nothing names it.
    pub built_in: bool,
}

/// `R` as statute knows it by itself.
pub fn built_in_first_party() -> SourceSpec {
    SourceSpec {
        name: FIRST_PARTY.to_owned(),
        path: String::new(),
        git: Some(Git {
            address: crate::global::FIRST_PARTY_ADDRESS.to_owned(),
            follow: Follow::Branch(crate::global::FIRST_PARTY_BRANCH.to_owned()),
            clone: None,
            commit: None,
        }),
        declared: Declared::BuiltIn,
        public: true,
        local: false,
        defaults: true,
        rules_dir: RULES_DIR.to_owned(),
    }
}

#[derive(Debug, Clone)]
pub struct Manifest {
    /// The name of the source the constitution comes from.
    pub constitution_source: String,
    pub edition: String,
    /// Whether the repository that holds the rule set is public; the
    /// visibility of its own rules.
    pub public: bool,
    /// The sources of `[sources]`, with those the global configuration,
    /// the lock file and the built-in default add, in order of name.
    pub sources: Vec<SourceSpec>,
    /// The private sources that the global configuration names, held for
    /// the private layer of a public rule set, in order of name.
    pub held: Vec<SourceSpec>,
    /// The rules chosen directly, in the order written.
    pub rules: Vec<Key>,
    /// The roots of the packs enabled, in the order written.
    pub packs: Vec<Key>,
    /// The domains turned on, in the order written.
    pub domains: Vec<Key>,
    /// The domains, one-ofs and rules the defaults enable that the rule
    /// set turns off, in the order written.
    pub exclude: Vec<Key>,
    /// What the private layer turns on again of what `statute.toml`
    /// excludes: listed in `rules`, `packs` or `domains` of
    /// `private.toml`, though the defaults may enable it.
    pub reopened: Vec<Key>,
    /// The directory of the repository's own rules, relative to that of
    /// `statute.toml`.
    pub local_rules: String,
    /// The language `compile` writes in.
    pub lang: String,
    /// The parameter values `statute.toml` sets, by name
    /// (`docs/design/rule-config.md`, Setting a value).
    pub values: BTreeMap<String, String>,
    /// Those of the global configuration.
    pub global_values: BTreeMap<String, String>,
    /// Those of the private rule set, in the private layer.
    pub private_values: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct RawManifest {
    constitution: RawConstitution,
    public: Option<bool>,
    local_rules: Option<String>,
    #[serde(default)]
    rules: Vec<String>,
    #[serde(default)]
    packs: Vec<String>,
    #[serde(default)]
    domains: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    sources: BTreeMap<String, toml::Value>,
    compile: Option<RawCompile>,
    /// The tables of parameter values, and any key unknown.
    #[serde(flatten)]
    rest: BTreeMap<String, toml::Value>,
}

/// The parameter values of a file's top-level tables `rest`, by name
/// (`[language] docs = "zh"` as `language.docs`); a key that is not a
/// table is unknown, and a value that is not a string fails.
fn values(
    rest: BTreeMap<String, toml::Value>,
    errors: &mut Vec<String>,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (table, value) in rest {
        let toml::Value::Table(t) = value else {
            errors.push(format!("unknown field `{table}`"));
            continue;
        };
        for (key, value) in t {
            match value {
                toml::Value::String(v) => {
                    out.insert(format!("{table}.{key}"), v);
                }
                other => errors.push(format!(
                    "[{table}] `{key} = {other}`: a parameter's value is a string, as `{key} = \"zh\"`"
                )),
            }
        }
    }
    out
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConstitution {
    source: String,
    edition: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSource {
    path: Option<String>,
    git: Option<String>,
    tag: Option<String>,
    branch: Option<String>,
    clone: Option<String>,
    public: Option<bool>,
    scope: Option<String>,
    defaults: Option<bool>,
    #[serde(rename = "rules-dir")]
    rules_dir: Option<String>,
}

/// An entry of a rule set's `[sources]`: a source, or `false`, which takes
/// a source of the global configuration out.
enum RawEntry {
    Off(bool),
    On(RawSource),
}

impl RawEntry {
    /// Reads the entry `name` by its shape, so that a problem in a source's
    /// table names the key and the entry.
    fn read(name: &str, value: toml::Value) -> Result<RawEntry, String> {
        match value {
            toml::Value::Boolean(b) => Ok(RawEntry::Off(b)),
            toml::Value::Table(_) => value.try_into().map(RawEntry::On).map_err(|e| {
                format!(
                    "[sources] `{name}`: {}",
                    e.to_string().trim_end().trim_end_matches('.')
                )
            }),
            other => Err(format!(
                "[sources] `{name} = {other}`: name the source, as in `{name} = {{ path = \"<dir>\" }}`, or write `{name} = false` to take a source of the global configuration out"
            )),
        }
    }
}

/// The keys of a rule set's top level that are not tables, those of
/// `RawManifest` and `RawPrivate`. Written after the `[sources]` header,
/// such a key falls in that table.
const TOP_LEVEL: &[&str] = &[
    "public",
    "local-rules",
    "rules",
    "packs",
    "domains",
    "exclude",
];

/// Takes out of a `[sources]` table the keys of `TOP_LEVEL` that fell in
/// it, each with an error, and returns their names. `false` stays: it
/// takes out a source of the global configuration of that name.
fn misplaced(sources: &mut BTreeMap<String, toml::Value>, errors: &mut Vec<String>) -> Vec<String> {
    let mut names = Vec::new();
    sources.retain(|name, value| {
        if value.is_table()
            || *value == toml::Value::Boolean(false)
            || !TOP_LEVEL.contains(&name.as_str())
        {
            return true;
        }
        errors.push(format!(
            "[sources] `{name}`: written after the `[sources]` header, it falls in that table; move it above the first table header, to the top level of the file"
        ));
        names.push(name.clone());
        false
    });
    names
}

/// The global configuration.
#[derive(Deserialize)]
struct RawConfig {
    #[serde(default)]
    sources: BTreeMap<String, RawSource>,
    #[serde(flatten)]
    rest: BTreeMap<String, toml::Value>,
}

/// The parameter values of the global configuration, from its text, by
/// name; every problem found is returned.
pub fn config_values(text: &str) -> Result<BTreeMap<String, String>, Vec<String>> {
    let raw: RawConfig =
        toml::from_str(text).map_err(|e| vec![e.to_string().trim_end().to_owned()])?;
    let mut errors = Vec::new();
    let out = values(raw.rest, &mut errors);
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

/// The sources of the global configuration, from its text; every problem
/// found is returned.
pub fn read_config(text: &str) -> Result<Vec<SourceSpec>, Vec<String>> {
    let raw: RawConfig =
        toml::from_str(text).map_err(|e| vec![e.to_string().trim_end().to_owned()])?;
    let mut errors = Vec::new();
    values(raw.rest, &mut errors);
    for (name, s) in &raw.sources {
        if s.scope.is_some() {
            errors.push(format!(
                "[sources] `{name}`: a source of the global configuration is global; remove `scope`"
            ));
        }
        if s.defaults.is_some() {
            errors.push(format!(
                "[sources] `{name}`: the global configuration enables no rule, so its sources bring no defaults; remove `defaults`, and write `defaults = false` in a rule set's entry for the source instead"
            ));
        }
        if s.git.is_none() {
            errors.push(format!(
                "[sources] `{name}`: a source of the global configuration is a git source, `{name} = {{ git = \"<address>\", branch = \"main\" }}`; a directory read as it is belongs in a rule set, with `path`"
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut sources = read_sources(&raw.sources, None, Declared::Global, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }
    sources.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(sources)
}

/// `private.toml`.
#[derive(Deserialize)]
struct RawPrivate {
    #[serde(default)]
    rules: Vec<String>,
    #[serde(default)]
    packs: Vec<String>,
    #[serde(default)]
    domains: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    sources: BTreeMap<String, toml::Value>,
    #[serde(flatten)]
    rest: BTreeMap<String, toml::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct RawCompile {
    /// Read only to say that `AGENTS.md` carries the statements alone.
    agents_md: Option<toml::Value>,
    lang: Option<String>,
}

/// The sources of a `[sources]` table, each problem in `errors`.
/// `public_rule_set` is the `public` of `statute.toml`, which names no
/// private source when it is true; `None` for the table of
/// `private.toml`, which may.
fn read_sources(
    raw: &BTreeMap<String, RawSource>,
    public_rule_set: Option<bool>,
    declared: Declared,
    errors: &mut Vec<String>,
) -> Vec<SourceSpec> {
    let mut sources = Vec::new();
    for (name, s) in raw {
        if let Some(why) = reserved(name) {
            errors.push(format!("[sources] {why}"));
            continue;
        }
        if !is_letters(name) && !node::is_slug(name) {
            errors.push(format!(
                "[sources] `{name}`: a source's short name is one or two capital letters, as in `{FIRST_PARTY}` for the first-party source or `P`, or lowercase words joined by hyphens, as in `team`"
            ));
            continue;
        }
        let source_public = match (name.as_str(), s.public) {
            (FIRST_PARTY, Some(false)) => {
                errors.push(format!(
                    "[sources] `{FIRST_PARTY}`: the first-party source is always public; remove `public = false`"
                ));
                continue;
            }
            (FIRST_PARTY, _) => true,
            (_, Some(p)) => p,
            (_, None) => {
                errors.push(format!(
                    "[sources] `{name}`: say whether the source is public, `public = true` or `public = false`"
                ));
                continue;
            }
        };
        let local = match (name.as_str(), s.scope.as_deref()) {
            (_, None | Some("global")) => false,
            (FIRST_PARTY, Some("local")) => {
                errors.push(format!(
                    "[sources] `{FIRST_PARTY}`: the first-party source is global; remove `scope = \"local\"`"
                ));
                continue;
            }
            (_, Some("local")) => true,
            (_, Some(other)) => {
                errors.push(format!(
                    "[sources] `{name}`: `scope = \"{other}\"` is neither \"global\" nor \"local\""
                ));
                continue;
            }
        };
        let rules_dir = match s.rules_dir.as_deref().map(rules_dir) {
            None => RULES_DIR.to_owned(),
            Some(Ok(dir)) => dir,
            Some(Err(why)) => {
                errors.push(format!("[sources] `{name}`: {why}"));
                continue;
            }
        };
        if public_rule_set == Some(true) && !source_public {
            errors.push(format!(
                "[sources] `{name}` is private, and this rule set is of a public repository, where anyone can read it and the files compiled from it; name private sources in the private rule set, {}, as `statute source add` does",
                crate::private::MANIFEST
            ));
            continue;
        }
        let spec = |path: &str, git| SourceSpec {
            name: name.clone(),
            path: path.to_owned(),
            git,
            declared,
            public: source_public,
            local,
            defaults: s.defaults.unwrap_or(true),
            rules_dir: rules_dir.clone(),
        };
        match (&s.path, &s.git) {
            (Some(path), None) if s.tag.is_none() && s.branch.is_none() && s.clone.is_none() => {
                sources.push(spec(path, None))
            }
            (Some(_), _) => errors.push(format!(
                "[sources] `{name}`: a source is either `path` or `git` with `tag` or `branch`, not both"
            )),
            (None, Some(address)) => {
                let follow = match (&s.branch, &s.tag) {
                    (Some(b), None) => Follow::Branch(b.clone()),
                    (None, Some(t)) => Follow::Tag(t.clone()),
                    _ => {
                        errors.push(format!(
                            "[sources] `{name}`: a git source follows one `branch` or one `tag`"
                        ));
                        continue;
                    }
                };
                if s.clone.is_some() && declared != Declared::Global {
                    errors.push(format!(
                        "[sources] `{name}`: `clone` is for the global configuration; in a rule set, name a directory with `path`"
                    ));
                    continue;
                }
                sources.push(spec(
                    "",
                    Some(Git {
                        address: address.clone(),
                        follow,
                        clone: s.clone.clone(),
                        commit: None,
                    }),
                ))
            }
            (None, None) => errors.push(format!(
                "[sources] `{name}`: give the source's `path`, as in `{name} = {{ path = \".\" }}`, or its `git` address and `branch`"
            )),
        }
    }
    sources
}

/// A relative path without its `.` parts and with its `..` parts applied
/// where they can be, to compare two ways of writing one directory.
fn normal(path: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir if out.last().is_some_and(|p| p != "..") => {
                out.pop();
            }
            other => out.push(other.as_os_str().to_string_lossy().into_owned()),
        }
    }
    out
}

/// A `rules-dir` as written, when it names a directory within the
/// source, outside `.statute/`; else what is wrong.
pub fn rules_dir(dir: &str) -> Result<String, String> {
    use std::path::Component;
    let path = Path::new(dir);
    let fail = |why: &str| Err(format!("`rules-dir = \"{dir}\"` {why}"));
    let first = path.components().find(|c| *c != Component::CurDir);
    if path.is_absolute() {
        fail("is absolute; name a directory below the source's top")
    } else if path.components().any(|c| c == Component::ParentDir) {
        fail("leads out of the source with `..`; a source is read only within itself")
    } else if first.is_some_and(|c| c.as_os_str() == ".statute") {
        fail("leads into `.statute/`, which holds a repository's rules for itself, never another's")
    } else {
        Ok(dir.to_owned())
    }
}

/// The key a reference of a rule set names among `sources`; else what is
/// wrong, to follow the reference as written.
fn key_in(sources: &[SourceSpec], local_rules: &str, r: &RuleRef) -> Result<Key, String> {
    let named = |name: &str| sources.iter().any(|s| s.name == name);
    match &r.origin {
        Origin::Local => match sources.iter().find(|s| s.holds(local_rules)) {
            Some(here) => Err(format!(
                "is a rule of this repository's `{}/`, which is source `{}` here; write \"{}\"",
                local_rules.trim_end_matches('/'),
                here.name,
                show(&here.name, r.id)
            )),
            None => Ok(Key::of(r)),
        },
        Origin::FirstParty if !named(FIRST_PARTY) => Err(format!(
            "is a first-party rule, and [sources] names no `{FIRST_PARTY}`"
        )),
        Origin::Named(n) if !named(n) => Err(format!(
            "names source `{n}`, which [sources] does not define"
        )),
        _ => Ok(Key::of(r)),
    }
}

/// Why `entry`, a reference to a rule of a private source the public rule
/// set holds for its private layer, cannot be enabled there; `None` for
/// any other reference.
fn held_private(held: &[SourceSpec], r: &RuleRef, entry: &str) -> Option<String> {
    match &r.origin {
        Origin::Named(n) if held.iter().any(|s| &s.name == n) => Some(format!(
            "`{entry}` is a rule of the private source `{n}`, and this rule set is of a public repository; enable it in {}, `statute add --private {entry}`, or leave it out",
            crate::private::MANIFEST
        )),
        _ => None,
    }
}

impl Manifest {
    /// Reads `statute.toml` from its text. Every problem found is returned,
    /// each a message about the file.
    pub fn parse(text: &str) -> Result<Manifest, Vec<String>> {
        Manifest::parse_with(text, &Layers::default())
    }

    /// As `parse`, with the sources of `layers` added where the rule set
    /// does not name them (`docs/design/global-configuration.md`, Which
    /// source a name means).
    pub fn parse_with(text: &str, layers: &Layers) -> Result<Manifest, Vec<String>> {
        let mut raw: RawManifest =
            toml::from_str(text).map_err(|e| vec![e.to_string().trim_end().to_owned()])?;
        let mut errors = Vec::new();
        misplaced(&mut raw.sources, &mut errors);
        let public = raw.public.unwrap_or_else(|| {
            errors.push(
                "`public` is missing; say whether the repository that holds this rule set is public, `public = true` or `public = false`"
                    .to_owned(),
            );
            // Read as private, so that no error about a private source
            // follows from the guess.
            false
        });

        let mut on = BTreeMap::new();
        let mut off = Vec::new();
        // The entries that could not be read, so that no error follows from
        // their absence.
        let mut unread = Vec::new();
        for (name, value) in raw.sources {
            let entry = match RawEntry::read(&name, value) {
                Ok(entry) => entry,
                Err(e) => {
                    errors.push(e);
                    unread.push(name);
                    continue;
                }
            };
            match entry {
                RawEntry::On(s) => {
                    on.insert(name, s);
                }
                RawEntry::Off(false) if name == FIRST_PARTY => errors.push(format!(
                    "[sources] `{FIRST_PARTY} = false`: the first-party source is replaced by naming another `{FIRST_PARTY}`, never taken out"
                )),
                RawEntry::Off(false) => off.push(name),
                RawEntry::Off(true) => errors.push(format!(
                    "[sources] `{name} = true`: name the source, as in `{name} = {{ path = \"<dir>\" }}`, or write `{name} = false` to take a source of the global configuration out"
                )),
            }
        }
        let mut sources = read_sources(&on, Some(public), Declared::RuleSet, &mut errors);
        let sources_ok = sources.len() == on.len() && unread.is_empty();
        let mut held = Vec::new();
        let given = |sources: &[SourceSpec], held: &[SourceSpec], name: &str| {
            off.iter().any(|o| o == name)
                || sources.iter().any(|s| s.name == name)
                || held.iter().any(|s| s.name == name)
        };
        for spec in layers.global.iter().chain(&layers.locked) {
            if given(&sources, &held, &spec.name) {
                continue;
            }
            if public && !spec.public {
                held.push(spec.clone());
            } else {
                sources.push(spec.clone());
            }
        }
        let uses_first_party = raw.constitution.source == FIRST_PARTY
            || raw
                .rules
                .iter()
                .chain(&raw.packs)
                .any(|e| RuleRef::parse(e).is_ok_and(|r| r.origin == Origin::FirstParty));
        if layers.built_in && uses_first_party && !given(&sources, &held, FIRST_PARTY) {
            sources.push(built_in_first_party());
        }
        sources.sort_by(|a, b| a.name.cmp(&b.name));
        held.sort_by(|a, b| a.name.cmp(&b.name));
        if sources.is_empty() && unread.is_empty() {
            errors.push(
                "[sources] is empty; name the source of the rules, as in `R = { path = \".\" }`"
                    .to_owned(),
            );
        }
        if sources_ok && !sources.iter().any(|s| s.name == raw.constitution.source) {
            errors.push(format!(
                "`constitution.source = \"{}\"` is not a source named in [sources]",
                raw.constitution.source
            ));
        }
        let here: Vec<&str> = sources
            .iter()
            .filter(|s| s.is_here())
            .map(|s| s.name.as_str())
            .collect();
        if here.len() > 1 {
            errors.push(format!(
                "[sources] {} are all at `.`; one source at most is this repository",
                here.iter()
                    .map(|n| format!("`{n}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        let values = values(raw.rest, &mut errors);
        let local_rules = raw.local_rules.unwrap_or_else(|| LOCAL_RULES.to_owned());
        let mut refs = |field: &str, list: &[String]| -> Vec<Key> {
            let mut out = Vec::new();
            for entry in list {
                let key = RuleRef::parse(entry).and_then(|r| {
                    if let Some(e) = held_private(&held, &r, entry) {
                        Err(e)
                    } else if sources_ok {
                        key_in(&sources, &local_rules, &r).map_err(|e| format!("`{entry}` {e}"))
                    } else {
                        // The sources are wrong already; that error is enough.
                        Ok(Key::of(&r))
                    }
                });
                match key {
                    Err(e) => errors.push(format!("`{field}`: {e}")),
                    Ok(key) if out.contains(&key) => {
                        errors.push(format!("`{field}` lists `{entry}` twice"))
                    }
                    Ok(key) => out.push(key),
                }
            }
            out
        };
        let rules = refs("rules", &raw.rules);
        let packs = refs("packs", &raw.packs);
        let domains = refs("domains", &raw.domains);
        let exclude = refs("exclude", &raw.exclude);
        if local_rules.trim().is_empty() || Path::new(&local_rules).is_absolute() {
            errors.push(format!(
                "`local-rules = \"{local_rules}\"` must name a directory relative to that of statute.toml, as in \"rules\""
            ));
        }

        let (agents_md, lang) = raw.compile.map_or((None, None), |c| (c.agents_md, c.lang));
        if agents_md.is_some() {
            errors.push(
                "[compile] `agents-md` is not a setting: AGENTS.md is for agents and carries each rule's statement alone, while the title, reason and verification are for the site; delete the key".to_owned(),
            );
        }
        let lang = lang.unwrap_or_else(|| "en".to_owned());
        if !crate::node::is_lang(&lang) {
            errors.push(format!(
                "[compile] `lang = \"{lang}\"` is not a language code such as \"en\" or \"zh\""
            ));
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(Manifest {
            constitution_source: raw.constitution.source,
            edition: raw.constitution.edition,
            public,
            sources,
            held,
            rules,
            packs,
            domains,
            exclude,
            reopened: Vec::new(),
            local_rules,
            lang,
            values,
            global_values: layers.values.clone(),
            private_values: BTreeMap::new(),
        })
    }

    /// The rule set with the private layer of `private.toml`, from
    /// its text, added: its sources and its rules after those of
    /// `statute.toml` (`docs/design/rule-sources.md`, Private rules in a
    /// public repository). Every problem found is returned.
    pub fn with_private(&self, text: &str) -> Result<Manifest, Vec<String>> {
        self.with_private_from(text, &[])
    }

    /// As `with_private`, with `locked`, the sources of the private lock
    /// file, for names neither layer gives.
    pub fn with_private_from(
        &self,
        text: &str,
        locked: &[SourceSpec],
    ) -> Result<Manifest, Vec<String>> {
        let mut raw: RawPrivate =
            toml::from_str(text).map_err(|e| vec![e.to_string().trim_end().to_owned()])?;
        let mut errors = Vec::new();
        misplaced(&mut raw.sources, &mut errors);
        if !self.public {
            errors.push(format!(
                "{} is for a public repository; in a private one (`public = false`), statute.toml names every source and rule",
                crate::private::MANIFEST
            ));
        }
        let mut sources = self.sources.clone();
        let mut on = BTreeMap::new();
        for (name, value) in raw.sources {
            match value.try_into() {
                Ok(s) => {
                    on.insert(name, s);
                }
                Err(e) => errors.push(format!(
                    "[sources] `{name}`: {}",
                    e.to_string().trim_end().trim_end_matches('.')
                )),
            }
        }
        let mut own = read_sources(&on, None, Declared::RuleSet, &mut errors);
        // The file serves every worktree of a clone, from which a relative
        // path would lead to different places.
        for spec in own.iter_mut().filter(|s| s.git.is_none()) {
            if spec.path.starts_with("~/") {
                spec.path = crate::global::expand(&spec.path)
                    .to_string_lossy()
                    .into_owned();
            } else if !Path::new(&spec.path).is_absolute() {
                errors.push(format!(
                    "[sources] `{}`: `path = \"{}\"` is relative; the private rule set serves every worktree, so a path in it is absolute or starts with `~/`",
                    spec.name, spec.path
                ));
            }
        }
        for spec in &own {
            match self.spec(&spec.name) {
                Some(have) if have.declared == Declared::RuleSet => errors.push(format!(
                    "[sources] `{}` is named in {FILE} already",
                    spec.name
                )),
                Some(_) => errors.push(format!(
                    "[sources] `{}` is a source of the public layer already, from the global configuration, {} or statute's default; the private layer only adds sources",
                    spec.name,
                    crate::lock::FILE
                )),
                None => sources.push(spec.clone()),
            }
        }
        for spec in self.held.iter().chain(locked) {
            if !sources.iter().any(|s| s.name == spec.name) {
                sources.push(spec.clone());
            }
        }
        sources.sort_by(|a, b| a.name.cmp(&b.name));
        if sources.iter().filter(|s| s.is_here()).count() > 1 {
            errors.push(
                "[sources] a source at `.` is this repository, which statute.toml names already"
                    .to_owned(),
            );
        }
        // The private layer's entries come after those of statute.toml,
        // and repeat none of them.
        let mut own = |field: &str, given: &[Key], list: &[String]| {
            let mut out: Vec<Key> = Vec::new();
            for entry in list {
                match RuleRef::parse(entry).and_then(|r| {
                    key_in(&sources, &self.local_rules, &r).map_err(|e| format!("`{entry}` {e}"))
                }) {
                    Err(e) => errors.push(format!("`{field}`: {e}")),
                    Ok(key) if given.contains(&key) => errors.push(format!(
                        "`{field}` lists `{entry}`, which {FILE} lists already"
                    )),
                    Ok(key) if out.contains(&key) => {
                        errors.push(format!("`{field}` lists `{entry}` twice"))
                    }
                    Ok(key) => out.push(key),
                }
            }
            out
        };
        let rules = own("rules", &self.rules, &raw.rules);
        let packs = own("packs", &self.packs, &raw.packs);
        let domains = own("domains", &self.domains, &raw.domains);
        let exclude = own("exclude", &self.exclude, &raw.exclude);
        let private_values = values(raw.rest, &mut errors);
        if !errors.is_empty() {
            return Err(errors);
        }
        // What the private layer turns off leaves the lists of
        // statute.toml, and what it turns on leaves its `exclude`.
        let reopened: Vec<Key> = self
            .exclude
            .iter()
            .filter(|k| rules.contains(k) || packs.contains(k) || domains.contains(k))
            .cloned()
            .collect();
        let merge = |given: &[Key], off: &[Key], added: Vec<Key>| -> Vec<Key> {
            given
                .iter()
                .filter(|k| !off.contains(k))
                .cloned()
                .chain(added)
                .collect()
        };
        Ok(Manifest {
            sources,
            held: Vec::new(),
            rules: merge(&self.rules, &exclude, rules),
            packs: merge(&self.packs, &exclude, packs),
            domains: merge(&self.domains, &exclude, domains),
            exclude: merge(&self.exclude, &reopened, exclude),
            reopened,
            private_values,
            ..self.clone()
        })
    }

    /// The key a reference names in this rule set, written as in
    /// `statute.toml`; else what is wrong.
    pub fn key(&self, text: &str) -> Result<Key, String> {
        RuleRef::parse(text).and_then(|r| match held_private(&self.held, &r, text) {
            Some(e) => Err(e),
            None => {
                key_in(&self.sources, &self.local_rules, &r).map_err(|e| format!("`{text}` {e}"))
            }
        })
    }

    /// The source named `name` in `[sources]`.
    pub fn spec(&self, name: &str) -> Option<&SourceSpec> {
        self.sources.iter().find(|s| s.name == name)
    }

    /// The source the constitution comes from.
    pub fn constitution_spec(&self) -> &SourceSpec {
        self.spec(&self.constitution_source)
            .expect("Manifest::parse checks that the constitution's source is named")
    }

    /// How an entry for `key` is written in `rules` or `packs`: `R42`,
    /// `team:7`, or the ID alone for one of the repository's own rules.
    pub fn entry(key: &Key) -> String {
        match key.source.as_str() {
            LOCAL => key.id.to_string(),
            _ => key.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
constitution = { source = "R", edition = "2026-10" }
public = false
rules = ["R42", "R7"]
packs = ["R3"]

[sources]
R = { path = "." }

[compile]
lang = "en"
"#;

    /// GOOD with its one source at `../constitution` in place of `.`, so
    /// the rules of this repository's own `rules/` are another source.
    fn elsewhere() -> String {
        GOOD.replace("R = { path = \".\" }", "R = { path = \"../constitution\" }")
    }

    #[test]
    fn reads_a_rule_set() {
        let m = Manifest::parse(GOOD).unwrap();
        assert_eq!(m.edition, "2026-10");
        assert!(!m.public);
        assert_eq!(
            m.sources,
            [SourceSpec {
                name: "R".into(),
                path: ".".into(),
                git: None,
                declared: Declared::RuleSet,
                public: true,
                local: false,
                defaults: true,
                rules_dir: "rules".into(),
            }]
        );
        assert_eq!(m.rules, [Key::new("R", 42), Key::new("R", 7)]);
        assert_eq!(m.packs, [Key::new("R", 3)]);
        assert_eq!(m.lang, "en");
    }

    #[test]
    fn agents_md_is_no_setting() {
        fails_with(
            &format!("{GOOD}agents-md = [\"statement\"]\n"),
            "[compile] `agents-md` is not a setting: AGENTS.md is for agents and carries each rule's statement alone",
        );
    }

    #[track_caller]
    fn fails_with(text: &str, needle: &str) {
        let errors = Manifest::parse(text).unwrap_err();
        assert!(
            errors.iter().any(|e| e.contains(needle)),
            "expected an error containing {needle:?}, got {errors:#?}"
        );
    }

    #[track_caller]
    fn parses(text: &str, origin: Origin, id: Id) {
        assert_eq!(RuleRef::parse(text), Ok(RuleRef { origin, id }), "{text}");
    }

    #[test]
    fn references_name_their_source() {
        parses("R3", Origin::FirstParty, 3);
        parses("12", Origin::Local, 12);
        parses("L12", Origin::Local, 12);
        parses("local:12", Origin::Local, 12);
        parses("team:7", Origin::Named("team".into()), 7);
        parses("P17", Origin::Named("P".into()), 17);
        // A two-letter name that starts with `R` or `L` is a source of
        // its own, not the first-party or the local rules.
        parses("RL12", Origin::Named("RL".into()), 12);
        parses("LR12", Origin::Named("LR".into()), 12);
        for (text, needle) in [
            ("ac:3", "written `R3`"),
            ("R:3", "written `R3`"),
            ("R03", "is not a reference"),
            ("r3", "is not a reference"),
            ("L", "is not a reference"),
            ("Team:7", "is not a reference"),
            ("P:17", "written with no colon, `P17`"),
            ("RL:12", "written with no colon, `RL12`"),
            ("ABC1", "is not a reference"),
            ("team7", "is not a reference"),
        ] {
            let e = RuleRef::parse(text).unwrap_err();
            assert!(e.contains(needle), "{text}: {e}");
        }
    }

    #[test]
    fn references_show_with_their_prefix() {
        for text in ["R3", "L12", "team:7", "P17", "RL12"] {
            assert_eq!(RuleRef::parse(text).unwrap().to_string(), text);
            assert_eq!(Key::of(&RuleRef::parse(text).unwrap()).to_string(), text);
        }
        assert_eq!(RuleRef::parse("local:12").unwrap().to_string(), "L12");
        assert_eq!(show("R", 3), "R3");
        assert_eq!(show("team", 7), "team:7");
        assert_eq!(show("L", 12), "L12");
        assert_eq!(show("P", 17), "P17");
    }

    #[test]
    fn keys_order_the_first_party_source_first_and_the_local_rules_last() {
        let mut keys = [
            Key::new("L", 1),
            Key::new("team", 2),
            Key::new("R", 9),
            Key::new("me", 5),
            Key::new("P", 4),
            Key::new("R", 3),
        ];
        keys.sort();
        let shown: Vec<String> = keys.iter().map(Key::to_string).collect();
        assert_eq!(shown, ["R3", "R9", "P4", "me:5", "team:2", "L1"]);
    }

    #[test]
    fn local_rules_are_the_first_party_ones_here() {
        // The local rules are the `rules/` of the source at `.`, so its
        // rules are written `R12`.
        let here = GOOD.replace("public = false", "public = false\nlocal-rules = \"rules\"");
        for local in ["12", "L12", "local:12"] {
            fails_with(
                &here.replace("\"R7\"", &format!("\"{local}\"")),
                &format!(
                    "`{local}` is a rule of this repository's `rules/`, which is source `R` here; write \"R12\""
                ),
            );
        }
        // In the default `.statute/rules`, or with the source elsewhere,
        // they are the repository's own rules, `L`.
        for local in ["12", "L12", "local:12"] {
            let m = Manifest::parse(&GOOD.replace("\"R7\"", &format!("\"{local}\""))).unwrap();
            assert_eq!(m.rules, [Key::new("R", 42), Key::new("L", 12)]);
            let m =
                Manifest::parse(&elsewhere().replace("\"R7\"", &format!("\"{local}\""))).unwrap();
            assert_eq!(m.rules, [Key::new("R", 42), Key::new("L", 12)]);
        }
    }

    #[test]
    fn the_former_name_is_an_error() {
        fails_with(
            &GOOD.replace("\"R7\"", "\"ac:7\""),
            "`ac:7`: the first-party source's rules are written `R7`",
        );
        fails_with(
            &GOOD
                .replace("R = {", "ac = {")
                .replace("source = \"R\"", "source = \"ac\""),
            "`ac` is the former name of the first-party source; name it `R`",
        );
    }

    #[test]
    fn reserved_and_malformed_source_names() {
        for name in ["L", "local"] {
            fails_with(
                &GOOD.replace("R = {", &format!("{name} = {{")),
                &format!("[sources] `{name}` stands for this repository's own rules"),
            );
        }
        for name in ["Team", "ABC"] {
            fails_with(
                &GOOD.replace("R = {", &format!("{name} = {{")),
                &format!("[sources] `{name}`: a source's short name is one or two capital letters"),
            );
        }
        // A third-party source with a valid name is read, its rules
        // written with it.
        let m = Manifest::parse(
            &GOOD
                .replace("R = {", "team = {")
                .replace("source = \"R\"", "source = \"team\"")
                .replace("\"R", "\"team:")
                .replace("path = \".\" }", "path = \".\", public = false }"),
        )
        .unwrap();
        assert_eq!(m.rules, [Key::new("team", 42), Key::new("team", 7)]);
        // So is one named in capital letters, its rules written with no
        // colon.
        let m = Manifest::parse(
            &GOOD
                .replace("R = {", "P = {")
                .replace("source = \"R\"", "source = \"P\"")
                .replace("\"R", "\"P")
                .replace("path = \".\" }", "path = \".\", public = false }"),
        )
        .unwrap();
        assert_eq!(m.rules, [Key::new("P", 42), Key::new("P", 7)]);
    }

    #[test]
    fn git_sources() {
        let with = |entry: &str| GOOD.replace("R = { path = \".\" }", entry);
        let m = Manifest::parse(&with(
            "R = { git = \"https://example.test/rules.git\", tag = \"v1\" }",
        ))
        .unwrap();
        assert_eq!(
            m.sources[0].git,
            Some(Git {
                address: "https://example.test/rules.git".into(),
                follow: Follow::Tag("v1".into()),
                clone: None,
                commit: None,
            })
        );
        fails_with(
            &with("R = { git = \"https://example.test/rules.git\" }"),
            "follows one `branch` or one `tag`",
        );
        fails_with(
            &with(
                "R = { git = \"https://example.test/rules.git\", branch = \"main\", clone = \"~/rules\" }",
            ),
            "`clone` is for the global configuration",
        );
    }

    #[test]
    fn several_sources() {
        let m = Manifest::parse(&GOOD.replace(
            "R = { path = \".\" }",
            "R = { path = \".\" }\nteam = { path = \"../team\", public = true }\nme = { path = \"../me\", public = false }",
        ).replace("\"R7\"", "\"team:7\", \"me:4\""))
        .unwrap();
        let names: Vec<(&str, bool)> = m
            .sources
            .iter()
            .map(|s| (s.name.as_str(), s.public))
            .collect();
        assert_eq!(names, [("R", true), ("me", false), ("team", true)]);
        assert_eq!(
            m.rules,
            [Key::new("R", 42), Key::new("team", 7), Key::new("me", 4)]
        );
        // Two sources cannot both be this repository.
        fails_with(
            &GOOD.replace(
                "R = { path = \".\" }",
                "R = { path = \".\" }\nteam = { path = \"./\", public = true }",
            ),
            "[sources] `R`, `team` are all at `.`",
        );
    }

    #[test]
    fn public_is_declared() {
        fails_with(
            &GOOD.replace("public = false\n", ""),
            "`public` is missing; say whether the repository that holds this rule set is public",
        );
        assert!(
            Manifest::parse(&GOOD.replace("public = false", "public = true"))
                .unwrap()
                .public
        );
        // On every source but `R`, which is always public.
        let with = |team: &str| {
            GOOD.replace(
                "R = { path = \".\" }",
                &format!("R = {{ path = \".\" }}\nteam = {{ path = \"../team\"{team} }}"),
            )
        };
        fails_with(
            &with(""),
            "[sources] `team`: say whether the source is public, `public = true` or `public = false`",
        );
        assert!(Manifest::parse(&with(", public = true")).is_ok());
        fails_with(
            &GOOD.replace(
                "R = { path = \".\" }",
                "R = { path = \".\", public = false }",
            ),
            "[sources] `R`: the first-party source is always public; remove `public = false`",
        );
        assert!(
            Manifest::parse(&GOOD.replace(
                "R = { path = \".\" }",
                "R = { path = \".\", public = true }"
            ))
            .is_ok()
        );
    }

    #[test]
    fn scope_is_global_or_local() {
        let with = |scope: &str| {
            GOOD.replace(
                "R = { path = \".\" }",
                &format!(
                    "R = {{ path = \".\" }}\nhere = {{ path = \"../here\", public = false{scope} }}"
                ),
            )
        };
        let local = |text: &str| Manifest::parse(text).unwrap().spec("here").unwrap().local;
        assert!(!local(&with("")));
        assert!(!local(&with(", scope = \"global\"")));
        assert!(local(&with(", scope = \"local\"")));
        fails_with(
            &with(", scope = \"repo\""),
            "[sources] `here`: `scope = \"repo\"` is neither \"global\" nor \"local\"",
        );
        fails_with(
            &GOOD.replace(
                "R = { path = \".\" }",
                "R = { path = \".\", scope = \"local\" }",
            ),
            "[sources] `R`: the first-party source is global",
        );
    }

    #[test]
    fn a_public_rule_set_names_no_private_source() {
        let text = GOOD.replace(
            "R = { path = \".\" }",
            "R = { path = \".\" }\nme = { path = \"../me\", public = false }",
        );
        // A private repository may name it.
        assert!(Manifest::parse(&text).is_ok());
        fails_with(
            &text.replace("public = false\n", "public = true\n"),
            "[sources] `me` is private, and this rule set is of a public repository",
        );
    }

    #[test]
    fn references_name_a_defined_source() {
        fails_with(
            &GOOD.replace("\"R7\"", "\"team:7\""),
            "`team:7` names source `team`, which [sources] does not define",
        );
        fails_with(
            &GOOD.replace("\"R7\"", "\"R7x\""),
            "`R7x` is not a reference",
        );
        fails_with(&GOOD.replace("\"R7\"", "\"R42\""), "lists `R42` twice");
        fails_with(
            &GOOD.replace("source = \"R\"", "source = \"core\""),
            "`constitution.source = \"core\"` is not a source named in [sources]",
        );
    }

    #[test]
    fn unknown_fields_are_errors() {
        fails_with(
            &format!("{GOOD}\nextends = \"base\""),
            "unknown field `extends`",
        );
    }

    /// A top-level key written after the `[sources]` header, which TOML
    /// reads into that table, is named with the table it fell in.
    #[test]
    fn a_top_level_key_under_sources_is_named() {
        fails_with(
            &GOOD.replace("[compile]", "exclude = [\"R7\"]\n\n[compile]"),
            "[sources] `exclude`: written after the `[sources]` header",
        );
        // `false` takes out a source of the global configuration of that
        // name.
        let global = read_config(
            "[sources]\nrules = { git = \"https://example.test/rules.git\", branch = \"main\", public = true }\n",
        )
        .unwrap();
        let layers = Layers {
            global,
            ..Layers::default()
        };
        let m = Manifest::parse_with(
            &GOOD.replace("[compile]", "rules = false\n\n[compile]"),
            &layers,
        )
        .unwrap();
        assert!(m.spec("rules").is_none());
        let base = Manifest::parse(&GOOD.replace("public = false", "public = true")).unwrap();
        let errors = base
            .with_private("[sources]\nteam = { path = \"/srv/team\" }\nexclude = [\"R7\"]\n")
            .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.starts_with("[sources] `exclude`: written after the `[sources]` header")),
            "{errors:#?}"
        );
    }
}
