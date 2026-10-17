//! One node of the parent tree, `rules/<id>.toml`: a rule, a one-of or a
//! domain (`docs/design/rule-format.md`, Rule file format). This module
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
    Command {
        run: String,
        tools: Vec<String>,
    },
    /// No program checks the rule: `check = "none"`.
    None,
}

/// A domain's or a rule's `default`: whether a rule set gets it without
/// listing it (`docs/design/rule-config.md`, Default switches).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    On,
    Off,
}

/// Where a rule goes, its `uses` (`docs/design/rule-config.md`, Where a
/// rule is used): into the compiled `AGENTS.md`, to the reviewer, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Uses {
    pub instructions: bool,
    pub review: bool,
}

impl Default for Uses {
    fn default() -> Self {
        Uses {
            instructions: true,
            review: false,
        }
    }
}

/// An entry of `requires` or `conflicts`: a number names a node of the
/// same source, a string `"R<id>"`, `"P<id>"` or `"<source>:<id>"` one of
/// another source.
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
    /// Empty, and `constitution` `None`, only in a rule of a rule set
    /// that rests on no constitution, which may leave both out.
    pub authority: Vec<String>,
    pub constitution: Option<Specifier>,
    pub level: Level,
    pub check: Check,
    pub uses: Uses,
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
    /// The rules that supersede this one; empty unless it is superseded.
    pub superseded_by: Vec<Id>,
    pub lang: String,
    pub parent: Option<Id>,
    /// A rule's parent in another source, a domain or a subdomain there
    /// (`docs/design/rule-sources.md`, Subdomains); `parent` is then empty.
    pub foreign_parent: Option<Ref>,
    /// The rule of another source this rule takes the place of
    /// (`docs/design/rule-sources.md`, Overrides); only a rule has it, and
    /// then no `parent`.
    pub overrides: Option<Ref>,
    pub title: Text,
    /// A domain's or a rule's `default`, when the file gives one.
    pub default: Option<Switch>,
    /// A one-of's `default`, the child it enables when a rule set picks
    /// none, when the file gives one.
    pub default_child: Option<Id>,
    /// Whether the file writes `default`, well formed or not, so that a
    /// malformed one is not also reported missing.
    pub default_written: bool,
    /// Whether the file writes `overrides`, well formed or not.
    pub overrides_written: bool,
    /// Which repositories a domain is for; only a domain has it.
    pub applicability: Option<Text>,
    pub translations: BTreeMap<String, String>,
    /// Present exactly when `kind` is `rule`.
    pub rule: Option<RuleFields>,
    /// The SHA-256 of the file as read, in lowercase hex.
    pub sha256: String,
    /// The SHA-256 of the node without its translations
    /// ([`original_sha256`]), in lowercase hex.
    pub original_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct RawNode {
    id: Id,
    slug: Option<String>,
    revision: u64,
    kind: Kind,
    status: Status,
    superseded_by: Option<RawSuccessors>,
    lang: String,
    parent: Option<RawRef>,
    overrides: Option<RawRef>,
    requires: Option<Vec<RawRef>>,
    conflicts: Option<Vec<RawRef>>,
    authority: Option<Vec<String>>,
    constitution: Option<String>,
    level: Option<Level>,
    check: Option<RawCheck>,
    default: Option<RawDefault>,
    uses: Option<Vec<String>>,
    run: Option<String>,
    tools: Option<Vec<String>>,
    title: Text,
    applicability: Option<Text>,
    text: Option<RawText>,
    translations: Option<BTreeMap<String, String>>,
}

/// `superseded-by`: one ID, or a list of them when a rule is split.
#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum RawSuccessors {
    One(Id),
    Many(Vec<Id>),
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
    None,
    Review,
}

/// `default`: `"on"` or `"off"` on a domain or a rule, a child's ID on a
/// one-of.
#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum RawDefault {
    Child(i64),
    Switch(String),
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
/// title and the three parts, joined with one NUL byte; for a domain or
/// a one-of, of the title alone, or of the title and the applicability
/// for a domain that has one.
pub fn original_hash(parts: &[&str]) -> String {
    hex(&Sha256::digest(parts.join("\0").as_bytes()))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|w| {
            !w.is_empty()
                && w.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// A language code such as `en`, `zh` or `zh-Hant`.
pub fn is_lang(s: &str) -> bool {
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
            use crate::manifest::{FIRST_PARTY, Origin, RuleRef};
            match RuleRef::parse(&s) {
                Ok(RuleRef {
                    origin: Origin::FirstParty,
                    id,
                }) => Ok(Ref::Foreign {
                    source: FIRST_PARTY.to_owned(),
                    id,
                }),
                Ok(RuleRef {
                    origin: Origin::Named(source),
                    id,
                }) => Ok(Ref::Foreign { source, id }),
                _ => Err(format!(
                    "`{s}` is not a reference: write a number for a node of this source, `\"{FIRST_PARTY}<id>\"` for a first-party rule, `\"P<id>\"` for one of a source named in one or two capital letters, or `\"<source>:<id>\"` for one of a source named in lowercase words"
                )),
            }
        }
    }
}

/// Reads one node file. Problems go to `diags`, all of them, not only the
/// first; the node is returned when it could be read at all, so the checks
/// across files can still run on it.
pub fn parse(file: &str, bytes: &[u8], diags: &mut Diagnostics) -> Option<Node> {
    read(file, bytes, true, diags)
}

/// As `parse`; without `constitution`, for a rule set that rests on no
/// constitution, a rule may leave out `authority` and `constitution`
/// (`docs/design/rule-format.md`, Rule file format).
pub(crate) fn read(
    file: &str,
    bytes: &[u8],
    constitution: bool,
    diags: &mut Diagnostics,
) -> Option<Node> {
    let sha256 = sha256_hex(bytes);
    let Ok(content) = std::str::from_utf8(bytes) else {
        diags.error(file, "not valid UTF-8");
        return None;
    };
    // A file's line ends are read as LF whatever it uses, as the TOML
    // parsers of other tools return them, so a checkout with CRLF line ends
    // reads, and hashes, the same. A `\r\n` written as an escape is not a
    // line end and stays.
    let content = content.replace("\r\n", "\n");
    let raw: RawNode = match toml::from_str(&content) {
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
    let overrides = match raw.overrides.clone().map(parse_ref) {
        None => None,
        Some(Ok(Ref::Local(id))) => {
            err(format!(
                "`overrides` names {id}, a node of this source; a rule overrides a rule of another source, written \"R200\" or \"team:7\", and within one source a rule is changed in place or superseded"
            ));
            None
        }
        Some(Ok(r)) => Some(r),
        Some(Err(e)) => {
            err(format!("`overrides`: {e}"));
            None
        }
    };
    let (parent, foreign_parent) = match raw.parent.clone().map(parse_ref) {
        None => (None, None),
        Some(Ok(Ref::Local(id))) => (Some(id), None),
        Some(Ok(r)) => (None, Some(r)),
        Some(Err(e)) => {
            err(format!("`parent`: {e}"));
            (None, None)
        }
    };
    if foreign_parent.is_some() && kind == Kind::OneOf {
        err(
            "a one-of's `parent` is a node of the same source, written as its number; only a rule's or a domain's may name a domain of another source"
                .to_owned(),
        );
    }
    let has_parent = raw.parent.is_some();
    match (kind, has_parent, raw.overrides.is_some()) {
        (Kind::Rule, true, true) => err(
            "a rule with `overrides` has no `parent`: it stands where the rule it overrides stands"
                .to_owned(),
        ),
        (Kind::Rule, false, false) => err(
            "a rule needs a `parent`, the ID of a domain or a one-of, or `overrides`, the rule of another source it takes the place of"
                .to_owned(),
        ),
        (Kind::OneOf, false, _) => err(
            "a one-of needs a `parent`: the ID of a domain".to_owned(),
        ),
        _ => {}
    }
    if parent == Some(raw.id) {
        err("`parent` names the node itself".to_owned());
    }

    let superseded_by = match &raw.superseded_by {
        None => Vec::new(),
        Some(RawSuccessors::One(id)) => vec![*id],
        Some(RawSuccessors::Many(ids)) => ids.clone(),
    };
    match (raw.status, &raw.superseded_by) {
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
    if matches!(&raw.superseded_by, Some(RawSuccessors::Many(ids)) if ids.is_empty()) {
        err("`superseded-by` is empty; name the rule that replaces it".to_owned());
    }
    if superseded_by.contains(&raw.id) {
        err("`superseded-by` names the rule itself".to_owned());
    }
    for (i, id) in superseded_by.iter().enumerate() {
        if superseded_by[..i].contains(id) {
            err(format!("`superseded-by` lists {id} twice"));
        }
    }

    if raw.applicability.is_some() && kind != Kind::Domain {
        err(format!(
            "only a domain has [applicability]; remove it from this {}",
            kind.name()
        ));
    }

    let (default, default_child) = match (&raw.default, kind) {
        (None, _) => (None, None),
        (Some(RawDefault::Switch(s)), Kind::Domain | Kind::Rule) => match s.as_str() {
            "on" => (Some(Switch::On), None),
            "off" => (Some(Switch::Off), None),
            _ => {
                err(format!("`default = \"{s}\"` must be \"on\" or \"off\""));
                (None, None)
            }
        },
        (Some(RawDefault::Child(n)), Kind::OneOf) if *n >= 1 => (None, Some(*n as Id)),
        (Some(RawDefault::Child(n)), Kind::OneOf) => {
            err(format!("`default = {n}` is not an ID; IDs start at 1"));
            (None, None)
        }
        (Some(_), Kind::OneOf) => {
            err(
                "a one-of's `default` is the ID of one of its children, written as a number"
                    .to_owned(),
            );
            (None, None)
        }
        (Some(_), _) => {
            err(format!(
                "a {}'s `default` is \"on\" or \"off\"",
                kind.name()
            ));
            (None, None)
        }
    };

    let rule = if kind == Kind::Rule {
        parse_rule_fields(&raw, constitution, &mut err)
    } else {
        let present: Vec<&str> = [
            ("superseded-by", raw.superseded_by.is_some()),
            ("overrides", raw.overrides.is_some()),
            ("requires", raw.requires.is_some()),
            ("conflicts", raw.conflicts.is_some()),
            ("authority", raw.authority.is_some()),
            ("constitution", raw.constitution.is_some()),
            ("level", raw.level.is_some()),
            ("check", raw.check.is_some()),
            ("uses", raw.uses.is_some()),
            ("run", raw.run.is_some()),
            ("tools", raw.tools.is_some()),
            ("text", raw.text.is_some()),
        ]
        .into_iter()
        .filter_map(|(name, present)| present.then_some(name))
        .collect();
        if !present.is_empty() {
            err(format!(
                "a {} has only id, slug, revision, kind, status, lang, parent, default, {} and translations; remove {}",
                kind.name(),
                if kind == Kind::Domain {
                    "title, applicability"
                } else {
                    "title"
                },
                present.join(", ")
            ));
        }
        None
    };

    let translations = raw.translations.clone().unwrap_or_default();
    check_languages(&raw, &translations, &mut err);
    let original_sha256 = original_sha256(&content, &raw.lang);

    Some(Node {
        id: raw.id,
        slug: raw.slug,
        revision: raw.revision,
        kind,
        status: raw.status,
        superseded_by,
        lang: raw.lang,
        parent,
        foreign_parent,
        overrides,
        title: raw.title,
        default,
        default_child,
        default_written: raw.default.is_some(),
        overrides_written: raw.overrides.is_some(),
        applicability: raw.applicability,
        translations,
        rule,
        sha256,
        original_sha256,
    })
}

/// The SHA-256 of a node read as TOML without `[translations]` and
/// without the text of every language but `lang`, written as JSON with its
/// keys in order. A changed translation leaves it as it was; a changed
/// original or any other field moves it (`docs/design/rule-format.md`,
/// IDs and revisions).
fn original_sha256(content: &str, lang: &str) -> String {
    let mut table: toml::Table =
        toml::from_str(content).expect("the file was read as a node above");
    table.remove("translations");
    let only_original = |value: &mut toml::Value| {
        if let Some(t) = value.as_table_mut() {
            t.retain(|key, _| key == lang);
        }
    };
    for (key, value) in table.iter_mut() {
        match (key.as_str(), value.as_table_mut()) {
            ("title" | "applicability", _) => only_original(value),
            ("text", Some(parts)) => parts.iter_mut().for_each(|(_, part)| only_original(part)),
            _ => {}
        }
    }
    sha256_hex(
        serde_json::to_string(&table)
            .expect("a TOML table is valid JSON")
            .as_bytes(),
    )
}

fn parse_rule_fields(
    raw: &RawNode,
    constitution: bool,
    err: &mut impl FnMut(String),
) -> Option<RuleFields> {
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
    if raw.authority.is_none() && constitution {
        missing.push("authority");
    } else if raw.authority.is_some() && authority.is_empty() {
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

    let revisions = match &raw.constitution {
        None if constitution => {
            missing.push("constitution");
            Err(())
        }
        None => Ok(None),
        Some(text) => Specifier::parse(text).map(Some).map_err(|e| {
            err(format!("`constitution`: {e}"));
        }),
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
        (Some(RawCheck::Review), _, _) => {
            err(
                "`check = \"review\"` is the older name of `check = \"none\"`; write that, and `uses` says whether the rule is listed for review"
                    .to_owned(),
            );
            None
        }
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
                _ => Check::None,
            })
        }
    };

    let uses = match &raw.uses {
        None => Uses::default(),
        Some(list) => {
            let mut uses = Uses {
                instructions: false,
                review: false,
            };
            if list.is_empty() {
                err("`uses` is empty; write `[\"instructions\"]`, `[\"review\"]` or both, or leave it out for `[\"instructions\"]`".to_owned());
            }
            for value in list {
                let seen = match value.as_str() {
                    "instructions" => &mut uses.instructions,
                    "review" => &mut uses.review,
                    _ => {
                        err(format!(
                            "`uses` lists `{value}`; it holds \"instructions\", \"review\" or both"
                        ));
                        continue;
                    }
                };
                if *seen {
                    err(format!("`uses` lists `{value}` twice"));
                }
                *seen = true;
            }
            uses
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
        constitution: revisions.ok()?,
        level: raw.level?,
        check: check?,
        uses,
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
    if let Some(applicability) = &raw.applicability {
        texts.push(("applicability", applicability));
    }
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
            if *name == "title" && value.trim_matches(['\r', '\n']).contains('\n') {
                err(format!("[title] `{key}` must be one line"));
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
    parts.extend(raw.applicability.as_ref().map(original));
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

/// The reference a citation `{@rule:<text>}` makes: `12` a node of the
/// same source, `R12`, `P7` or `team:7` one of another, as `requires`
/// writes it.
pub fn cited_ref(text: &str) -> Option<Ref> {
    if text.bytes().all(|b| b.is_ascii_digit()) {
        return id_from_file_name(&format!("{text}.toml")).map(Ref::Local);
    }
    parse_ref(RawRef::Name(text.to_owned())).ok()
}

impl Ref {
    /// The node it names, written in a node of source `from`.
    pub fn key(&self, from: &str) -> crate::manifest::Key {
        match self {
            Ref::Local(id) => crate::manifest::Key::new(from, *id),
            Ref::Foreign { source, id } => crate::manifest::Key::new(source, *id),
        }
    }
}

pub fn show_ref(r: &Ref) -> String {
    match r {
        Ref::Local(id) => id.to_string(),
        Ref::Foreign { source, id } => format!("\"{}\"", crate::manifest::show(source, *id)),
    }
}

#[cfg(test)]
mod tests {
    use super::original_sha256;

    const BASE: &str = r#"id = 7
slug = "seven"
requires = [2]
lang = "en"

[title]
en = "Seven"
zh = "七"

[text.statement]
en = "Do seven."
zh = "做七。"

[translations]
zh = "abc"
"#;

    fn hash(content: &str) -> String {
        original_sha256(content, "en")
    }

    #[test]
    fn the_hash_follows_the_original_and_the_fields() {
        let base = hash(BASE);
        for (from, to) in [
            ("requires = [2]", "requires = [3]"),
            ("en = \"Seven\"", "en = \"Seven rules\""),
            ("en = \"Do seven.\"", "en = \"Do eight.\""),
        ] {
            let changed = BASE.replace(from, to);
            assert_ne!(changed, BASE, "{from} is in the sample");
            assert_ne!(hash(&changed), base, "{from} -> {to}");
        }
    }

    #[test]
    fn the_hash_ignores_translations_and_layout() {
        let base = hash(BASE);
        for (from, to) in [
            ("zh = \"七\"", "zh = \"第七\""),
            ("zh = \"做七。\"", "zh = \"做第七条。\""),
            ("zh = \"abc\"", "zh = \"def\""),
            // The same table, its keys in another order with a comment.
            (
                "id = 7\nslug = \"seven\"",
                "# A comment.\nslug = \"seven\"\nid = 7",
            ),
            ("en = \"Seven\"", "en = 'Seven'"),
        ] {
            let changed = BASE.replace(from, to);
            assert_ne!(changed, BASE, "{from} is in the sample");
            assert_eq!(hash(&changed), base, "{from} -> {to}");
        }
    }
}
