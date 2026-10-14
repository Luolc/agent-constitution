//! The rule set, `statute.toml`: the constitution it rests on, its
//! sources, the rules and packs it enables, and what the compiled output
//! carries (`docs/design/rule-sets.md`, Rule sets and the lock file).

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::node::{self, Id, id_from_file_name};

pub const FILE: &str = "statute.toml";

/// A part of a rule's text that the compiled `AGENTS.md` may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Part {
    Statement,
    Reason,
    Verification,
}

/// The key of `[sources]` that names the first-party source, the
/// repository that publishes the constitution, and the prefix of its
/// rules: `R42`.
pub const FIRST_PARTY: &str = "R";

/// The prefix of a local rule, one of the repository's own `rules/`:
/// `L12`.
pub const LOCAL: &str = "L";

/// Which source a reference in a rule set names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// `R42`.
    FirstParty,
    /// `12`, `L12` or `local:12`: the rule set's own repository.
    Local,
    /// `team:7`, a source `[sources]` names.
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
/// rules, `team:7` for another, `42` when the source has no name, as when
/// the rules of a directory are checked alone.
pub fn show(name: &str, id: Id) -> String {
    match name {
        FIRST_PARTY | LOCAL => format!("{name}{id}"),
        "" => id.to_string(),
        _ => format!("{name}:{id}"),
    }
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
    /// says: `R42`, `L12` and `team:7` as written.
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
            None => number(text)
                .map(at(Origin::Local))
                .or_else(|| number(text.strip_prefix(FIRST_PARTY)?).map(at(Origin::FirstParty)))
                .or_else(|| number(text.strip_prefix(LOCAL)?).map(at(Origin::Local))),
            Some((name, id)) => {
                let id = number(id);
                match (name, id) {
                    ("local", Some(id)) => Some(at(Origin::Local)(id)),
                    (FIRST_PARTY | "ac", Some(id)) => {
                        return Err(format!(
                            "`{text}`: the first-party source's rules are written `{FIRST_PARTY}{id}`"
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
                "`{text}` is not a reference; write `{FIRST_PARTY}42` for a first-party rule, `42` for one of this repository's own rules, or `<source>:42`"
            )
        })
    }
}

impl std::fmt::Display for RuleRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.origin {
            Origin::FirstParty => write!(f, "{FIRST_PARTY}{}", self.id),
            Origin::Local => write!(f, "{LOCAL}{}", self.id),
            Origin::Named(name) => write!(f, "{name}:{}", self.id),
        }
    }
}

/// A source of rules. Only a source on the local disk is supported so far;
/// its `path` is relative to the directory of `statute.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpec {
    pub name: String,
    pub path: String,
    /// Whether anyone may read the source, as the rule set declares it.
    pub public: bool,
}

impl SourceSpec {
    /// Whether the source is the rule set's own directory, whose `rules/`
    /// are then the source's rules rather than local ones.
    pub fn is_here(&self) -> bool {
        matches!(self.path.trim_end_matches('/'), "." | "")
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
    /// The sources of `[sources]`, in order of name.
    pub sources: Vec<SourceSpec>,
    /// The rules chosen directly, in the order written.
    pub rules: Vec<Key>,
    /// The roots of the packs enabled, in the order written.
    pub packs: Vec<Key>,
    pub agents_md: Vec<Part>,
    /// The language `compile` writes in.
    pub lang: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct RawManifest {
    constitution: RawConstitution,
    public: Option<bool>,
    #[serde(default)]
    rules: Vec<String>,
    #[serde(default)]
    packs: Vec<String>,
    sources: BTreeMap<String, RawSource>,
    compile: Option<RawCompile>,
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
    public: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct RawCompile {
    agents_md: Option<Vec<Part>>,
    lang: Option<String>,
}

/// The key a reference of a rule set names among `sources`; else what is
/// wrong, to follow the reference as written.
fn key_in(sources: &[SourceSpec], r: &RuleRef) -> Result<Key, String> {
    let named = |name: &str| sources.iter().any(|s| s.name == name);
    match &r.origin {
        Origin::Local => match sources.iter().find(|s| s.is_here()) {
            Some(here) => Err(format!(
                "is a rule of this repository's `rules/`, which is source `{}` here; write \"{}\"",
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

impl Manifest {
    /// Reads `statute.toml` from its text. Every problem found is returned,
    /// each a message about the file.
    pub fn parse(text: &str) -> Result<Manifest, Vec<String>> {
        let raw: RawManifest =
            toml::from_str(text).map_err(|e| vec![e.to_string().trim_end().to_owned()])?;
        let mut errors = Vec::new();
        let public = raw.public.unwrap_or_else(|| {
            errors.push(
                "`public` is missing; say whether the repository that holds this rule set is public, `public = true` or `public = false`"
                    .to_owned(),
            );
            // Read as private, so that no error about a private source
            // follows from the guess.
            false
        });

        let mut sources = Vec::new();
        for (name, s) in &raw.sources {
            if let Some(why) = reserved(name) {
                errors.push(format!("[sources] {why}"));
                continue;
            }
            if name != FIRST_PARTY && !node::is_slug(name) {
                errors.push(format!(
                    "[sources] `{name}`: a source's short name is `{FIRST_PARTY}`, for the first-party source, or lowercase words joined by hyphens, as in `team`"
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
            if public && !source_public {
                errors.push(format!(
                    "[sources] `{name}` is private, and this rule set is of a public repository, where anyone can read it and the files compiled from it; name private sources in statute.private.toml, next to statute.toml, which this version of statute does not read yet"
                ));
                continue;
            }
            match (&s.path, &s.git) {
                (Some(path), None) if s.tag.is_none() && s.branch.is_none() => {
                    sources.push(SourceSpec {
                        name: name.clone(),
                        path: path.clone(),
                        public: source_public,
                    })
                }
                (Some(_), _) => errors.push(format!(
                    "[sources] `{name}`: a source is either `path` or `git` with `tag` or `branch`, not both"
                )),
                (None, Some(_)) => errors.push(format!(
                    "[sources] `{name}`: git sources are not supported yet; use a source on disk, `{name} = {{ path = \"<dir>\" }}`"
                )),
                (None, None) => errors.push(format!(
                    "[sources] `{name}`: give the source's `path`, as in `{name} = {{ path = \".\" }}`"
                )),
            }
        }
        if raw.sources.is_empty() {
            errors.push(
                "[sources] is empty; name the source of the rules, as in `R = { path = \".\" }`"
                    .to_owned(),
            );
        }
        let sources_ok = sources.len() == raw.sources.len();
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

        let mut refs = |field: &str, list: &[String]| -> Vec<Key> {
            let mut out = Vec::new();
            for entry in list {
                let key = RuleRef::parse(entry).and_then(|r| {
                    if sources_ok {
                        key_in(&sources, &r).map_err(|e| format!("`{entry}` {e}"))
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

        let (agents_md, lang) = raw.compile.map_or((None, None), |c| (c.agents_md, c.lang));
        let agents_md = agents_md.unwrap_or_else(|| vec![Part::Statement]);
        let lang = lang.unwrap_or_else(|| "en".to_owned());
        if !crate::node::is_lang(&lang) {
            errors.push(format!(
                "[compile] `lang = \"{lang}\"` is not a language code such as \"en\" or \"zh\""
            ));
        }
        let mut seen = Vec::new();
        for part in &agents_md {
            if seen.contains(part) {
                errors.push(format!("[compile] `agents-md` lists {part:?} twice").to_lowercase());
            }
            seen.push(*part);
        }

        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(Manifest {
            constitution_source: raw.constitution.source,
            edition: raw.constitution.edition,
            public,
            sources,
            rules,
            packs,
            agents_md,
            lang,
        })
    }

    /// The key a reference names in this rule set, written as in
    /// `statute.toml`; else what is wrong.
    pub fn key(&self, text: &str) -> Result<Key, String> {
        RuleRef::parse(text)
            .and_then(|r| key_in(&self.sources, &r).map_err(|e| format!("`{text}` {e}")))
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
agents-md = ["statement", "reason"]
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
                public: true,
            }]
        );
        assert_eq!(m.rules, [Key::new("R", 42), Key::new("R", 7)]);
        assert_eq!(m.packs, [Key::new("R", 3)]);
        assert_eq!(m.agents_md, [Part::Statement, Part::Reason]);
    }

    #[test]
    fn agents_md_defaults_to_the_statement() {
        let m =
            Manifest::parse(&GOOD.replace("agents-md = [\"statement\", \"reason\"]", "")).unwrap();
        assert_eq!(m.agents_md, [Part::Statement]);
        assert_eq!(m.lang, "en");
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
        for (text, needle) in [
            ("ac:3", "written `R3`"),
            ("R:3", "written `R3`"),
            ("R03", "is not a reference"),
            ("r3", "is not a reference"),
            ("L", "is not a reference"),
            ("Team:7", "is not a reference"),
        ] {
            let e = RuleRef::parse(text).unwrap_err();
            assert!(e.contains(needle), "{text}: {e}");
        }
    }

    #[test]
    fn references_show_with_their_prefix() {
        for text in ["R3", "L12", "team:7"] {
            assert_eq!(RuleRef::parse(text).unwrap().to_string(), text);
            assert_eq!(Key::of(&RuleRef::parse(text).unwrap()).to_string(), text);
        }
        assert_eq!(RuleRef::parse("local:12").unwrap().to_string(), "L12");
        assert_eq!(show("R", 3), "R3");
        assert_eq!(show("team", 7), "team:7");
        assert_eq!(show("L", 12), "L12");
    }

    #[test]
    fn keys_order_the_first_party_source_first_and_the_local_rules_last() {
        let mut keys = [
            Key::new("L", 1),
            Key::new("team", 2),
            Key::new("R", 9),
            Key::new("me", 5),
            Key::new("R", 3),
        ];
        keys.sort();
        let shown: Vec<String> = keys.iter().map(Key::to_string).collect();
        assert_eq!(shown, ["R3", "R9", "me:5", "team:2", "L1"]);
    }

    #[test]
    fn local_rules_are_the_first_party_ones_here() {
        // The source is this repository, so its rules are written `R12`.
        for local in ["12", "L12", "local:12"] {
            fails_with(
                &GOOD.replace("\"R7\"", &format!("\"{local}\"")),
                &format!(
                    "`{local}` is a rule of this repository's `rules/`, which is source `R` here; write \"R12\""
                ),
            );
        }
        // Elsewhere, they are the repository's own rules, `L`.
        for local in ["12", "L12", "local:12"] {
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
        fails_with(
            &GOOD.replace("R = {", "Team = {"),
            "[sources] `Team`: a source's short name is `R`",
        );
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
    }

    #[test]
    fn git_sources_are_not_supported_yet() {
        fails_with(
            &GOOD.replace(
                "R = { path = \".\" }",
                "R = { git = \"example.test/rules\", tag = \"v1\" }",
            ),
            "git sources are not supported yet",
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
    fn unknown_fields_and_parts_are_errors() {
        fails_with(
            &format!("{GOOD}\nextends = \"base\""),
            "unknown field `extends`",
        );
        fails_with(
            &GOOD.replace("\"reason\"]", "\"examples\"]"),
            "unknown variant `examples`",
        );
    }
}
