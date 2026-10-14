//! One node of the parent tree, `rules/<id>.toml`: a rule, a one-of or a
//! domain (`docs/design/rule-system.md`, Rule file format). This module
//! reads one file and checks what can be checked within it; the relations
//! between files are checked in `source`.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::diag::Diagnostics;
use crate::specifier::Specifier;

pub type Id = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Rule,
    OneOf,
    Domain,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Rule => "rule",
            Kind::OneOf => "one-of",
            Kind::Domain => "domain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Draft,
    Trial,
    Active,
    Superseded,
    Repealed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    Project,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    Builtin,
    Command { run: String, tools: Vec<String> },
    Review,
}

/// An entry of `requires` or `conflicts`: a number names a node of the
/// same source, a string `"<source>:<id>"` one of another source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ref {
    Local(Id),
    Foreign { source: String, id: Id },
}

/// A text in each of its languages, keyed by language code.
pub type Text = BTreeMap<String, String>;

/// What only a rule has.
#[derive(Debug, Clone)]
pub struct RuleFields {
    pub requires: Vec<Ref>,
    pub conflicts: Vec<Ref>,
    pub authority: Vec<String>,
    pub constitution: Specifier,
    pub level: Level,
    pub check: Check,
    pub statement: Text,
    pub reason: Text,
    pub verification: Text,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: Id,
    pub slug: Option<String>,
    pub revision: u64,
    pub kind: Kind,
    pub status: Status,
    pub superseded_by: Option<Id>,
    pub lang: String,
    pub parent: Option<Id>,
    pub title: Text,
    pub translations: BTreeMap<String, String>,
    /// Present exactly when `kind` is `rule`.
    pub rule: Option<RuleFields>,
    /// The SHA-256 of the file as read, in lowercase hex.
    pub sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct RawNode {
    id: Id,
    slug: Option<String>,
    revision: u64,
    kind: Kind,
    status: Status,
    superseded_by: Option<Id>,
    lang: String,
    parent: Option<Id>,
    requires: Option<Vec<RawRef>>,
    conflicts: Option<Vec<RawRef>>,
    authority: Option<Vec<String>>,
    constitution: Option<String>,
    level: Option<Level>,
    check: Option<RawCheck>,
    run: Option<String>,
    tools: Option<Vec<String>>,
    title: Text,
    text: Option<RawText>,
    translations: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum RawRef {
    Number(i64),
    Name(String),
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum RawCheck {
    Builtin,
    Command,
    Review,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawText {
    statement: Text,
    reason: Text,
    verification: Text,
}

/// The name a node's file must have: its ID in decimal, with no padding.
pub fn file_name(id: Id) -> String {
    format!("{id}.toml")
}

/// The ID a file name stands for, if it is a well-formed node file name.
pub fn id_from_file_name(name: &str) -> Option<Id> {
    let stem = name.strip_suffix(".toml")?;
    if stem.is_empty() || stem.starts_with('0') || !stem.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    stem.parse().ok()
}

/// The hash a translation records: the original-language values of the
/// title and the three parts, joined with one NUL byte, or of the title
/// alone for a domain or a one-of.
pub fn original_hash(parts: &[&str]) -> String {
    hex(&Sha256::digest(parts.join("\0").as_bytes()))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|w| {
            !w.is_empty()
                && w.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// A language code such as `en`, `zh` or `zh-Hant`.
fn is_lang(s: &str) -> bool {
    let mut parts = s.split('-');
    let first = parts.next().unwrap_or("");
    (2..=3).contains(&first.len())
        && first.bytes().all(|b| b.is_ascii_lowercase())
        && parts.all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric()))
}

fn parse_ref(raw: RawRef) -> Result<Ref, String> {
    match raw {
        RawRef::Number(n) if n >= 1 => Ok(Ref::Local(n as Id)),
        RawRef::Number(n) => Err(format!("{n} is not an ID; IDs start at 1")),
        RawRef::Name(s) => {
            let parsed = s.split_once(':').and_then(|(source, id)| {
                let id = id_from_file_name(&format!("{id}.toml"))?;
                is_slug(source).then(|| Ref::Foreign {
                    source: source.to_owned(),
                    id,
                })
            });
            parsed.ok_or_else(|| {
                format!("`{s}` is not a reference: write a number for a node of this source, or `\"<source>:<id>\"` for one of another")
            })
        }
    }
}

/// Reads one node file. Problems go to `diags`, all of them, not only the
/// first; the node is returned when it could be read at all, so the checks
/// across files can still run on it.
pub fn parse(file: &str, bytes: &[u8], diags: &mut Diagnostics) -> Option<Node> {
    let sha256 = sha256_hex(bytes);
    let Ok(content) = std::str::from_utf8(bytes) else {
        diags.error(file, "not valid UTF-8");
        return None;
    };
    let raw: RawNode = match toml::from_str(content) {
        Ok(raw) => raw,
        Err(e) => {
            diags.error(file, e.to_string().trim_end().to_owned());
            return None;
        }
    };
    let mut err = |message: String| diags.error(file, message);

    if raw.id == 0 {
        err("`id` is 0; IDs start at 1".to_owned());
    }
    match file.rsplit('/').next().and_then(id_from_file_name) {
        Some(from_name) if from_name == raw.id => {}
        _ if raw.id == 0 => {}
        _ => err(format!(
            "the file name must be the ID: `id = {}` belongs in {}",
            raw.id,
            file_name(raw.id)
        )),
    }
    if let Some(slug) = &raw.slug
        && !is_slug(slug)
    {
        err(format!(
            "slug `{slug}` must be lowercase words joined by hyphens"
        ));
    }
    if raw.revision == 0 {
        err("`revision` is 0; revisions start at 1".to_owned());
    }
    if !is_lang(&raw.lang) {
        err(format!(
            "`lang = \"{}\"` is not a language code such as \"en\" or \"zh\"",
            raw.lang
        ));
    }

    let kind = raw.kind;
    match (kind, raw.parent) {
        (Kind::Domain, Some(_)) => {
            err("a domain has no parent; domains do not nest, so a finer grouping is a rule or a one-of under the domain".to_owned())
        }
        (Kind::Rule | Kind::OneOf, None) => err(format!(
            "a {} needs a `parent`: the ID of a rule, one-of or domain",
            kind.name()
        )),
        _ => {}
    }
    if raw.parent == Some(raw.id) {
        err("`parent` names the node itself".to_owned());
    }

    match (raw.status, raw.superseded_by) {
        (Status::Superseded, _) if raw.kind != Kind::Rule => err(format!(
            "only a rule can be superseded; a {} cannot have `status = \"superseded\"`",
            raw.kind.name()
        )),
        (Status::Superseded, None) => err(
            "`status = \"superseded\"` needs `superseded-by`, the ID of the rule that replaces it"
                .to_owned(),
        ),
        (status, Some(_)) if status != Status::Superseded => {
            err("`superseded-by` is allowed only when `status = \"superseded\"`".to_owned())
        }
        _ => {}
    }
    if raw.superseded_by == Some(raw.id) {
        err("`superseded-by` names the rule itself".to_owned());
    }

    let rule = if kind == Kind::Rule {
        parse_rule_fields(&raw, &mut err)
    } else {
        let present: Vec<&str> = [
            ("superseded-by", raw.superseded_by.is_some()),
            ("requires", raw.requires.is_some()),
            ("conflicts", raw.conflicts.is_some()),
            ("authority", raw.authority.is_some()),
            ("constitution", raw.constitution.is_some()),
            ("level", raw.level.is_some()),
            ("check", raw.check.is_some()),
            ("run", raw.run.is_some()),
            ("tools", raw.tools.is_some()),
            ("text", raw.text.is_some()),
        ]
        .into_iter()
        .filter_map(|(name, present)| present.then_some(name))
        .collect();
        if !present.is_empty() {
            err(format!(
                "a {} has only id, slug, revision, kind, status, lang, parent, title and translations; remove {}",
                kind.name(),
                present.join(", ")
            ));
        }
        None
    };

    let translations = raw.translations.clone().unwrap_or_default();
    check_languages(&raw, &translations, &mut err);

    Some(Node {
        id: raw.id,
        slug: raw.slug,
        revision: raw.revision,
        kind,
        status: raw.status,
        superseded_by: raw.superseded_by,
        lang: raw.lang,
        parent: raw.parent,
        title: raw.title,
        translations,
        rule,
        sha256,
    })
}

fn parse_rule_fields(raw: &RawNode, err: &mut impl FnMut(String)) -> Option<RuleFields> {
    let mut missing = Vec::new();
    let mut refs = |name: &str, list: &Option<Vec<RawRef>>| {
        let mut out: Vec<Ref> = Vec::new();
        for raw_ref in list.iter().flatten() {
            match parse_ref(raw_ref.clone()) {
                Ok(r) if r == Ref::Local(raw.id) => err(format!("`{name}` names the rule itself")),
                Ok(r) if out.contains(&r) => err(format!("`{name}` lists {} twice", show_ref(&r))),
                Ok(r) => out.push(r),
                Err(e) => err(format!("`{name}`: {e}")),
            }
        }
        out
    };
    let requires = refs("requires", &raw.requires);
    let conflicts = refs("conflicts", &raw.conflicts);

    let authority = raw.authority.clone().unwrap_or_default();
    if raw.authority.is_none() {
        missing.push("authority");
    } else if authority.is_empty() {
        err(
            "`authority` is empty; name the provisions of the constitution the rule refines"
                .to_owned(),
        );
    }
    for a in &authority {
        if a.is_empty() || a.starts_with("constitution:") || a.contains(char::is_whitespace) {
            err(format!(
                "`authority` entry `{a}` must be a provision ID without a namespace, such as \"simplicity\" or \"must-ask.capital\""
            ));
        }
    }

    let constitution = match &raw.constitution {
        None => {
            missing.push("constitution");
            None
        }
        Some(text) => match Specifier::parse(text) {
            Ok(s) => Some(s),
            Err(e) => {
                err(format!("`constitution`: {e}"));
                None
            }
        },
    };
    if raw.level.is_none() {
        missing.push("level");
    }

    let check = match (raw.check, &raw.run, &raw.tools) {
        (None, _, _) => {
            missing.push("check");
            None
        }
        (Some(RawCheck::Command), Some(run), Some(tools)) => Some(Check::Command {
            run: run.clone(),
            tools: tools.clone(),
        }),
        (Some(RawCheck::Command), run, tools) => {
            err(format!(
                "`check = \"command\"` needs {}",
                [("`run`", run.is_none()), ("`tools`", tools.is_none())]
                    .into_iter()
                    .filter_map(|(n, m)| m.then_some(n))
                    .collect::<Vec<_>>()
                    .join(" and ")
            ));
            None
        }
        (Some(other), run, tools) => {
            if run.is_some() || tools.is_some() {
                err("`run` and `tools` are allowed only with `check = \"command\"`".to_owned());
            }
            Some(match other {
                RawCheck::Builtin => Check::Builtin,
                _ => Check::Review,
            })
        }
    };

    if raw.text.is_none() {
        missing.push("[text.statement], [text.reason] and [text.verification]");
    }
    if !missing.is_empty() {
        err(format!("a rule needs {}", missing.join(", ")));
    }

    let text = raw.text.as_ref()?;
    Some(RuleFields {
        requires,
        conflicts,
        authority,
        constitution: constitution?,
        level: raw.level?,
        check: check?,
        statement: text.statement.clone(),
        reason: text.reason.clone(),
        verification: text.verification.clone(),
    })
}

/// The original language is required in every text; any other language
/// is a translation, which gives every text in that language and records
/// in `[translations]` the hash of the original it follows.
fn check_languages(
    raw: &RawNode,
    translations: &BTreeMap<String, String>,
    err: &mut impl FnMut(String),
) {
    let lang = raw.lang.as_str();
    let mut texts: Vec<(&str, &Text)> = vec![("title", &raw.title)];
    if let Some(text) = &raw.text {
        texts.extend([
            ("text.statement", &text.statement),
            ("text.reason", &text.reason),
            ("text.verification", &text.verification),
        ]);
    }
    for (name, text) in &texts {
        if !text.contains_key(lang) {
            err(format!(
                "[{name}] has no `{lang}`, the original language that `lang` names"
            ));
        }
        for (key, value) in text.iter() {
            if !is_lang(key) {
                err(format!("[{name}] key `{key}` is not a language code"));
            }
            if value.trim().is_empty() {
                err(format!("[{name}] `{key}` is empty"));
            }
        }
    }

    let others: BTreeSet<&str> = texts
        .iter()
        .flat_map(|(_, t)| t.keys().map(String::as_str))
        .filter(|&k| k != lang)
        .collect();
    for &other in &others {
        let lacking: Vec<&str> = texts
            .iter()
            .filter(|(_, t)| !t.contains_key(other))
            .map(|(name, _)| *name)
            .collect();
        if !lacking.is_empty() {
            err(format!(
                "the `{other}` translation must give every text; it is missing in [{}]",
                lacking.join("], [")
            ));
        }
        if !translations.contains_key(other) {
            err(format!(
                "the `{other}` translation needs `{other} = \"<hash>\"` in [translations]"
            ));
        }
    }
    for key in translations.keys() {
        if key == lang {
            err(format!(
                "[translations] has `{key}`, the original language; it lists translations only"
            ));
        } else if !others.contains(key.as_str()) {
            err(format!(
                "[translations] has `{key}`, but no text is given in `{key}`"
            ));
        }
    }

    // The hash can be checked only when every original text is there.
    let original = |t: &Text| t.get(lang).cloned();
    let mut parts = vec![original(&raw.title)];
    match (&raw.text, raw.kind) {
        (Some(t), Kind::Rule) => parts.extend([
            original(&t.statement),
            original(&t.reason),
            original(&t.verification),
        ]),
        (None, Kind::Rule) => return,
        _ => {}
    }
    let Some(parts) = parts.into_iter().collect::<Option<Vec<String>>>() else {
        return;
    };
    let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
    let hash = original_hash(&parts);
    for (key, recorded) in translations {
        if key != lang && others.contains(key.as_str()) && *recorded != hash {
            err(format!(
                "the `{key}` translation follows another original than the current one; update the translation, then set `{key} = \"{hash}\"` in [translations]"
            ));
        }
    }
}

pub fn show_ref(r: &Ref) -> String {
    match r {
        Ref::Local(id) => id.to_string(),
        Ref::Foreign { source, id } => format!("\"{source}:{id}\""),
    }
}
