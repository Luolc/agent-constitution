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
/// `R42` for the first-party source, `team:7` for another, `42` when the
/// source has no name, as when the rules of a directory are checked alone.
pub fn show(name: &str, id: Id) -> String {
    match name {
        FIRST_PARTY => format!("{FIRST_PARTY}{id}"),
        "" => id.to_string(),
        _ => format!("{name}:{id}"),
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
}

impl SourceSpec {
    /// The ID `r` names in this source, the rule set's one; else what is
    /// wrong, to follow the reference as written.
    pub fn id_of(&self, r: &RuleRef) -> Result<Id, String> {
        let name = &self.name;
        match &r.origin {
            Origin::FirstParty if name == FIRST_PARTY => Ok(r.id),
            Origin::Named(n) if n == name => Ok(r.id),
            Origin::Local if matches!(self.path.trim_end_matches('/'), "." | "") => Err(format!(
                "is a rule of this repository's `rules/`, which is source `{name}` here; write \"{}\"",
                show(name, r.id)
            )),
            Origin::Local => Err(format!(
                "is one of this repository's own rules, and reading them beside source `{name}` is not supported yet"
            )),
            Origin::FirstParty => Err(format!(
                "is a first-party rule, and [sources] names no `{FIRST_PARTY}`"
            )),
            Origin::Named(n) => Err(format!(
                "names source `{n}`, which [sources] does not define"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Manifest {
    /// The name of the source the constitution comes from.
    pub constitution_source: String,
    pub edition: String,
    pub source: SourceSpec,
    /// The rules chosen directly, in the order written.
    pub rules: Vec<Id>,
    /// The roots of the packs enabled, in the order written.
    pub packs: Vec<Id>,
    pub agents_md: Vec<Part>,
    /// The language `compile` writes in.
    pub lang: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct RawManifest {
    constitution: RawConstitution,
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
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct RawCompile {
    agents_md: Option<Vec<Part>>,
    lang: Option<String>,
}

impl Manifest {
    /// Reads `statute.toml` from its text. Every problem found is returned,
    /// each a message about the file.
    pub fn parse(text: &str) -> Result<Manifest, Vec<String>> {
        let raw: RawManifest =
            toml::from_str(text).map_err(|e| vec![e.to_string().trim_end().to_owned()])?;
        let mut errors = Vec::new();

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
            match (&s.path, &s.git) {
                (Some(path), None) if s.tag.is_none() && s.branch.is_none() => {
                    sources.push(SourceSpec {
                        name: name.clone(),
                        path: path.clone(),
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
        let source = match (sources.len(), raw.sources.len()) {
            (1, 1) => sources.pop(),
            (_, 0) => {
                errors.push(
                    "[sources] is empty; name the source of the rules, as in `R = { path = \".\" }`"
                        .to_owned(),
                );
                None
            }
            (_, n) if n > 1 => {
                errors.push(format!(
                    "[sources] names {n} sources; more than one source is not supported yet"
                ));
                None
            }
            _ => None,
        };

        if let Some(source) = &source
            && raw.constitution.source != source.name
        {
            errors.push(format!(
                "`constitution.source = \"{}\"` is not a source named in [sources]",
                raw.constitution.source
            ));
        }

        let mut refs = |field: &str, list: &[String]| -> Vec<Id> {
            let mut out = Vec::new();
            for entry in list {
                let id = RuleRef::parse(entry).and_then(|r| match &source {
                    Some(source) => source.id_of(&r).map_err(|e| format!("`{entry}` {e}")),
                    // The sources are wrong already; that error is enough.
                    None => Ok(r.id),
                });
                match id {
                    Err(e) => errors.push(format!("`{field}`: {e}")),
                    Ok(id) if out.contains(&id) => {
                        errors.push(format!("`{field}` lists `{entry}` twice"))
                    }
                    Ok(id) => out.push(id),
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

        match source {
            Some(source) if errors.is_empty() => Ok(Manifest {
                constitution_source: raw.constitution.source,
                edition: raw.constitution.edition,
                source,
                rules,
                packs,
                agents_md,
                lang,
            }),
            _ => Err(errors),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
constitution = { source = "R", edition = "2026-10" }
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
        assert_eq!(
            m.source,
            SourceSpec {
                name: "R".into(),
                path: ".".into()
            }
        );
        assert_eq!(m.rules, [42, 7]);
        assert_eq!(m.packs, [3]);
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
        }
        assert_eq!(RuleRef::parse("local:12").unwrap().to_string(), "L12");
        assert_eq!(show("R", 3), "R3");
        assert_eq!(show("team", 7), "team:7");
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
        // Elsewhere, they are a second source.
        fails_with(
            &elsewhere().replace("\"R7\"", "\"12\""),
            "`12` is one of this repository's own rules, and reading them beside source `R` is not supported yet",
        );
        assert_eq!(Manifest::parse(&elsewhere()).unwrap().rules, [42, 7]);
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
                .replace("\"R", "\"team:"),
        )
        .unwrap();
        assert_eq!(m.rules, [42, 7]);
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
    fn one_source_only() {
        fails_with(
            &GOOD.replace(
                "R = { path = \".\" }",
                "R = { path = \".\" }\nteam = { path = \"../team\" }",
            ),
            "more than one source is not supported yet",
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
