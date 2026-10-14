//! The rule set, `statute.toml`: the constitution it rests on, its
//! sources, the rules and packs it enables, and what the compiled output
//! carries (`docs/design/rule-system.md`, Rule sets and the lock file).

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::node::{Id, id_from_file_name};

pub const FILE: &str = "statute.toml";

/// A part of a rule's text that the compiled `AGENTS.md` may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Part {
    Statement,
    Reason,
    Verification,
}

/// A source of rules. Only a source on the local disk is supported so far;
/// its `path` is relative to the directory of `statute.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpec {
    pub name: String,
    pub path: String,
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
                    "[sources] is empty; name the source of the rules, as in `ac = { path = \".\" }`"
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
                let parsed = entry
                    .split_once(':')
                    .and_then(|(name, id)| Some((name, id_from_file_name(&format!("{id}.toml"))?)));
                match parsed {
                    None => errors.push(format!(
                        "`{field}`: `{entry}` is not a reference; write the source's name, a colon and the ID, as in \"ac:42\""
                    )),
                    Some((name, _)) if !raw.sources.contains_key(name) => errors.push(format!(
                        "`{field}`: `{entry}` names source `{name}`, which [sources] does not define"
                    )),
                    Some((_, id)) if out.contains(&id) => {
                        errors.push(format!("`{field}` lists `{entry}` twice"))
                    }
                    Some((_, id)) => out.push(id),
                }
            }
            out
        };
        let rules = refs("rules", &raw.rules);
        let packs = refs("packs", &raw.packs);

        let agents_md = raw
            .compile
            .and_then(|c| c.agents_md)
            .unwrap_or_else(|| vec![Part::Statement]);
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
            }),
            _ => Err(errors),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
constitution = { source = "ac", edition = "2026-10" }
rules = ["ac:42", "ac:7"]
packs = ["ac:3"]

[sources]
ac = { path = "." }

[compile]
agents-md = ["statement", "reason"]
"#;

    #[test]
    fn reads_a_rule_set() {
        let m = Manifest::parse(GOOD).unwrap();
        assert_eq!(m.edition, "2026-10");
        assert_eq!(
            m.source,
            SourceSpec {
                name: "ac".into(),
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
    }

    #[track_caller]
    fn fails_with(text: &str, needle: &str) {
        let errors = Manifest::parse(text).unwrap_err();
        assert!(
            errors.iter().any(|e| e.contains(needle)),
            "expected an error containing {needle:?}, got {errors:#?}"
        );
    }

    #[test]
    fn git_sources_are_not_supported_yet() {
        fails_with(
            &GOOD.replace(
                "ac = { path = \".\" }",
                "ac = { git = \"example.test/rules\", tag = \"v1\" }",
            ),
            "git sources are not supported yet",
        );
    }

    #[test]
    fn one_source_only() {
        fails_with(
            &GOOD.replace(
                "ac = { path = \".\" }",
                "ac = { path = \".\" }\nteam = { path = \"../team\" }",
            ),
            "more than one source is not supported yet",
        );
    }

    #[test]
    fn references_name_a_defined_source() {
        fails_with(
            &GOOD.replace("\"ac:7\"", "\"team:7\""),
            "names source `team`, which [sources] does not define",
        );
        fails_with(&GOOD.replace("\"ac:7\"", "\"7\""), "`7` is not a reference");
        fails_with(
            &GOOD.replace("\"ac:7\"", "\"ac:42\""),
            "lists `ac:42` twice",
        );
        fails_with(
            &GOOD.replace("source = \"ac\"", "source = \"core\""),
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
