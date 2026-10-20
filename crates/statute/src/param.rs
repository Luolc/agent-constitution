//! Parameters: the values a rule's text leaves to the rule set, declared
//! in a source's `params.toml` and named in a rule's text as `{$<name>}`
//! (`docs/design/rule-config.md`, Parameters).

use std::collections::{BTreeMap, BTreeSet};

use crate::diag::Diagnostics;
use crate::manifest::{Key, Manifest};
use crate::node::{Text, is_lang, original_hash};
use crate::resolve::Resolution;
use crate::source::Sources;

/// The keys of a rule set that a parameter's first word may not be.
const RESERVED: [&str; 10] = [
    "constitution",
    "public",
    "rules",
    "packs",
    "sources",
    "compile",
    "domains",
    "exclude",
    "local-rules",
    "base",
];

/// The languages statute has a name for.
const LANGUAGES: [&str; 2] = ["en", "zh"];

/// One parameter of a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub kind: Type,
    pub scope: Scope,
    pub default: String,
    pub title: Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Language,
    String,
    /// One of the values listed.
    Choice(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Set in `statute.toml` alone, the same for everyone.
    Repo,
    /// The person's own, from the global configuration or the private
    /// rule set, in the private output.
    Personal,
}

impl Type {
    pub fn name(&self) -> &'static str {
        match self {
            Type::Language => "language",
            Type::String => "string",
            Type::Choice(_) => "choice",
        }
    }
}

impl Param {
    /// Why `value` is no value of this parameter, if it is not.
    pub fn refuse(&self, value: &str) -> Option<String> {
        match &self.kind {
            Type::Language if !LANGUAGES.contains(&value) => Some(format!(
                "`{value}` is not a language statute has a name for: {}",
                quoted(&LANGUAGES)
            )),
            Type::String => string_refused(value),
            Type::Choice(values) if !values.iter().any(|v| v == value) => Some(format!(
                "`{value}` is not one of its values: {}",
                quoted(values)
            )),
            _ => None,
        }
    }

    /// `value` as the output writes it in `lang`: a language as its name,
    /// anything else as it is.
    pub fn shown(&self, value: &str, lang: &str) -> String {
        match self.kind {
            Type::Language => language_name(value, lang).to_owned(),
            _ => value.to_owned(),
        }
    }
}

/// The name of the language `code` in the language `lang`.
pub fn language_name<'a>(code: &'a str, lang: &str) -> &'a str {
    match (code, lang) {
        ("en", "zh") => "英文",
        ("zh", "zh") => "中文",
        ("en", _) => "English",
        ("zh", _) => "Chinese",
        _ => code,
    }
}

fn quoted<S: AsRef<str>>(values: &[S]) -> String {
    values
        .iter()
        .map(|v| format!("`{}`", v.as_ref()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Why `value` is no `string`: empty, more than one line, a space at
/// either end, or a backtick.
fn string_refused(value: &str) -> Option<String> {
    if value.is_empty() {
        Some("an empty string is no value".to_owned())
    } else if value.contains(['\n', '\r']) {
        Some(format!("`{value}` is more than one line"))
    } else if value.trim() != value {
        Some(format!("`{value}` has a space at one end"))
    } else if value.contains('`') {
        Some(format!("{value:?} has a backtick"))
    } else {
        None
    }
}

/// Whether `name` has the shape of a parameter's name: two words joined
/// by a dot, each lowercase letters and digits joined by hyphens.
pub fn is_name(name: &str) -> bool {
    let word = |w: &str| {
        !w.is_empty()
            && w.split('-').all(|p| {
                !p.is_empty()
                    && p.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            })
    };
    name.split_once('.')
        .is_some_and(|(a, b)| word(a) && word(b))
}

/// One placeholder in a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placeholder<'a> {
    /// `{$<name>}` with a name of the right shape.
    Name(&'a str),
    /// What follows `{$` when it is no name, up to the brace if there is
    /// one.
    Malformed(&'a str),
}

/// The placeholders of `text`, in order.
pub fn placeholders(text: &str) -> Vec<Placeholder<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{$") {
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) if is_name(&after[..end]) => {
                out.push(Placeholder::Name(&after[..end]));
                rest = &after[end + 1..];
            }
            Some(end) => {
                out.push(Placeholder::Malformed(&after[..end]));
                rest = &after[end + 1..];
            }
            None => {
                out.push(Placeholder::Malformed(after));
                rest = "";
            }
        }
    }
    out
}

/// The names `text` holds, each with how many times.
pub fn counted(text: &str) -> BTreeMap<&str, usize> {
    let mut out = BTreeMap::new();
    for p in placeholders(text) {
        if let Placeholder::Name(n) = p {
            *out.entry(n).or_default() += 1;
        }
    }
    out
}

/// `text` with each placeholder of a well-formed name replaced by what
/// `value` returns for it; the rest stays as it is.
pub fn fill(text: &str, mut value: impl FnMut(&str) -> String) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{$") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}').filter(|&end| is_name(&after[..end])) {
            Some(end) => {
                out.push_str(&value(&after[..end]));
                rest = &after[end + 1..];
            }
            None => {
                out.push_str("{$");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Reads `params.toml`, shown as `file`: its parameters, each problem in
/// `diags`. A parameter with a problem is left out.
pub fn parse(file: &str, bytes: &[u8], diags: &mut Diagnostics) -> Vec<Param> {
    let table: toml::Table = match std::str::from_utf8(bytes)
        .map_err(|e| e.to_string())
        .and_then(|t| toml::from_str(t).map_err(|e| e.to_string()))
    {
        Ok(t) => t,
        Err(e) => {
            diags.error(file, e.trim_end().to_owned());
            return Vec::new();
        }
    };
    let mut out: Vec<Param> = Vec::new();
    for key in table.keys().filter(|k| *k != "parameter") {
        diags.error(
            file,
            format!("`{key}`: the file holds only `[[parameter]]` tables; remove `{key}`"),
        );
    }
    let Some(list) = table.get("parameter") else {
        return out;
    };
    let Some(list) = list.as_array() else {
        diags.error(file, "`parameter` is an array of tables, `[[parameter]]`");
        return out;
    };
    for (i, item) in list.iter().enumerate() {
        let mut err = |m: String| diags.error(file, m);
        let Some(item) = item.as_table() else {
            err(format!("parameter {}: not a table", i + 1));
            continue;
        };
        let at = item
            .get("name")
            .and_then(toml::Value::as_str)
            .map_or_else(|| format!("parameter {}", i + 1), |n| format!("`{n}`"));
        if let Some(p) = one(item, &at, &mut err) {
            if out.iter().any(|q| q.name == p.name) {
                err(format!("{at}: another parameter of the file has this name"));
            } else {
                out.push(p);
            }
        }
    }
    out
}

/// One `[[parameter]]` table, named `at` in messages.
fn one(item: &toml::Table, at: &str, err: &mut impl FnMut(String)) -> Option<Param> {
    const KEYS: [&str; 7] = [
        "name",
        "type",
        "values",
        "scope",
        "default",
        "title",
        "translations",
    ];
    let mut ok = true;
    let mut fail = |m: String| {
        ok = false;
        err(format!("{at}: {m}"));
    };
    for key in item.keys().filter(|k| !KEYS.contains(&k.as_str())) {
        fail(format!(
            "`{key}` is no key of a parameter, which has name, type, values, scope, default, title and translations"
        ));
    }
    let string = |key: &str, fail: &mut dyn FnMut(String)| -> Option<String> {
        match item.get(key) {
            None => {
                fail(format!("`{key}` is missing"));
                None
            }
            Some(toml::Value::String(s)) => Some(s.clone()),
            Some(other) => {
                fail(format!("`{key} = {other}` is not a string"));
                None
            }
        }
    };
    let name = string("name", &mut fail);
    if let Some(name) = &name {
        if !is_name(name) {
            fail(
                "a name is two words joined by a dot, each lowercase letters and digits joined by hyphens, as `language.commits`"
                    .to_owned(),
            );
        } else if let Some(word) = name.split('.').next().filter(|w| RESERVED.contains(w)) {
            fail(format!(
                "`{word}` is a key of the rule set already, so no parameter's name begins with it"
            ));
        }
    }
    let kind = match string("type", &mut fail).as_deref() {
        Some("language") => Some(Type::Language),
        Some("string") => Some(Type::String),
        Some("choice") => match item.get("values").and_then(toml::Value::as_array) {
            Some(values) => {
                let mut list = Vec::new();
                for v in values {
                    match v.as_str() {
                        Some(s) => {
                            if let Some(why) = string_refused(s) {
                                fail(format!("`values`: {why}"));
                            } else if list.iter().any(|l| l == s) {
                                fail(format!("`values` lists `{s}` twice"));
                            } else {
                                list.push(s.to_owned());
                            }
                        }
                        None => fail(format!("`values`: `{v}` is not a string")),
                    }
                }
                if values.is_empty() {
                    fail("`values` is empty".to_owned());
                }
                Some(Type::Choice(list))
            }
            None => {
                fail("a `choice` lists its `values`, an array of strings".to_owned());
                None
            }
        },
        Some(other) => {
            fail(format!(
                "`type = \"{other}\"` must be \"language\", \"string\" or \"choice\""
            ));
            None
        }
        None => None,
    };
    if item.contains_key("values") && !matches!(kind, Some(Type::Choice(_)) | None) {
        fail("`values` belongs to a `choice` only".to_owned());
    }
    let scope = match string("scope", &mut fail).as_deref() {
        Some("repo") => Some(Scope::Repo),
        Some("personal") => Some(Scope::Personal),
        Some(other) => {
            fail(format!(
                "`scope = \"{other}\"` must be \"repo\" or \"personal\""
            ));
            None
        }
        None => None,
    };
    let default = string("default", &mut fail);
    let title = text(item, "title", &mut fail);
    let translations = text(item, "translations", &mut fail).unwrap_or_default();
    if let Some(title) = &title {
        check_title(title, &translations, &mut fail);
    }
    let (Some(name), Some(kind), Some(scope), Some(default), Some(title)) =
        (name, kind, scope, default, title)
    else {
        return None;
    };
    let param = Param {
        name,
        kind,
        scope,
        default,
        title,
    };
    if let Some(why) = param.refuse(&param.default) {
        fail(format!("`default`: {why}"));
    }
    ok.then_some(param)
}

/// The table `key` of strings by language; `None` when it is missing
/// (only `title` must be there) or not such a table.
fn text(item: &toml::Table, key: &str, fail: &mut impl FnMut(String)) -> Option<Text> {
    match item.get(key) {
        None if key == "title" => {
            fail("`[title]` is missing".to_owned());
            None
        }
        None => None,
        Some(toml::Value::Table(t)) => {
            let mut out = Text::new();
            for (k, v) in t {
                match v.as_str() {
                    Some(s) => {
                        out.insert(k.clone(), s.to_owned());
                    }
                    None => fail(format!("[{key}] `{k}` is not a string")),
                }
            }
            Some(out)
        }
        Some(_) => {
            fail(format!("`{key}` is a table of strings by language"));
            None
        }
    }
}

/// A title's original is its one language that `[translations]` does not
/// name, and each translation records the SHA-256 of it.
fn check_title(title: &Text, translations: &Text, fail: &mut impl FnMut(String)) {
    for (lang, value) in title {
        if !is_lang(lang) {
            fail(format!("[title] key `{lang}` is not a language code"));
        }
        if value.trim().is_empty() {
            fail(format!("[title] `{lang}` is empty"));
        } else if value.trim_matches(['\r', '\n']).contains('\n') {
            fail(format!("[title] `{lang}` must be one line"));
        }
        if !placeholders(value).is_empty() {
            fail(format!(
                "[title] `{lang}` holds a placeholder; only a rule's title and text may"
            ));
        }
    }
    for lang in translations.keys().filter(|l| !title.contains_key(*l)) {
        fail(format!(
            "[translations] has `{lang}`, but [title] gives no `{lang}`"
        ));
    }
    let originals: BTreeSet<&String> = title
        .keys()
        .filter(|l| !translations.contains_key(*l))
        .collect();
    let [original] = originals.into_iter().collect::<Vec<_>>()[..] else {
        fail(
            "[title] has one language that [translations] does not name, the original; each other language is a translation with the SHA-256 of the original in [translations]"
                .to_owned(),
        );
        return;
    };
    let hash = original_hash(&[&title[original]]);
    for (lang, recorded) in translations {
        if title.contains_key(lang) && *recorded != hash {
            fail(format!(
                "the `{lang}` translation follows another original than the current one; update the translation, then set `{lang} = \"{hash}\"` in [translations]"
            ));
        }
    }
}

/// Where a value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// `statute.toml`.
    RuleSet,
    /// The parameter's `default`.
    Default,
    /// The private rule set, `private.toml`.
    Private,
    /// The global configuration.
    Global,
}

impl Origin {
    /// The place, as `explain` names it.
    pub fn place(self) -> String {
        match self {
            Origin::RuleSet => crate::manifest::FILE.to_owned(),
            Origin::Default => "the default".to_owned(),
            Origin::Private => crate::private::MANIFEST.to_owned(),
            Origin::Global => format!(
                "the global configuration ({})",
                crate::global::shown_config()
            ),
        }
    }
}

/// The value of a parameter an enabled rule uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setting {
    /// The source that declares it.
    pub source: String,
    pub param: Param,
    /// The value the committed `AGENTS.md` and the lock file get.
    pub value: String,
    pub from: Origin,
    /// The person's own value of a `personal` parameter, when one is set.
    pub personal: Option<(String, Origin)>,
    /// The enabled rules that use it.
    pub rules: Vec<Key>,
}

impl Setting {
    /// The value the private output gets.
    pub fn private_value(&self) -> &str {
        self.personal.as_ref().map_or(&self.value, |(v, _)| v)
    }
}

/// The value of each parameter the enabled rules of `resolution` use, by
/// name (`docs/design/rule-config.md`, Setting a value). Problems with
/// the values the rule set, the private rule set and the global
/// configuration set go to `diags`.
pub fn settle(
    sources: &Sources,
    manifest: &Manifest,
    resolution: &Resolution,
    diags: &mut Diagnostics,
) -> BTreeMap<String, Setting> {
    let mut out: BTreeMap<String, Setting> = BTreeMap::new();
    for key in resolution.enabled.keys() {
        // An overridden rule's text is not used; the rule in its place is.
        if resolution.overridden.contains_key(key) {
            continue;
        }
        for (name, (source, param)) in sources.params_of(key) {
            out.entry(name)
                .or_insert_with(|| Setting {
                    source,
                    value: param.default.clone(),
                    param,
                    from: Origin::Default,
                    personal: None,
                    rules: Vec::new(),
                })
                .rules
                .push(key.clone());
        }
    }
    let declared = |name: &str| -> Vec<&Param> {
        sources
            .iter()
            .flat_map(|s| s.params.iter())
            .filter(|p| p.name == name)
            .collect()
    };
    let shown = |name: &str| match name.split_once('.') {
        Some((table, key)) => format!("[{table}] `{key}`"),
        None => format!("`{name}`"),
    };
    let file = crate::manifest::FILE;
    for (name, value) in &manifest.values {
        let params = declared(name);
        if params.is_empty() {
            diags.error(
                file,
                format!(
                    "{} names no parameter of the rule set's sources",
                    shown(name)
                ),
            );
            continue;
        }
        if let Some(why) = params.iter().find_map(|p| p.refuse(value)) {
            diags.error(file, format!("{}: {why}", shown(name)));
            continue;
        }
        match out.get_mut(name) {
            Some(setting) => {
                setting.value.clone_from(value);
                setting.from = Origin::RuleSet;
            }
            None => diags.warning(
                file,
                format!(
                    "{} is set, and no enabled rule uses `{{${name}}}`",
                    shown(name)
                ),
            ),
        }
    }
    // The person's own values: the private rule set's before the global
    // configuration's. A name no source declares is an error in the
    // private rule set; the global configuration serves every repository,
    // and one there is left alone.
    let private_file = crate::private::MANIFEST;
    let global_file = crate::global::shown_config();
    for (values, origin, place) in [
        (
            &manifest.private_values,
            Origin::Private,
            private_file.to_owned(),
        ),
        (&manifest.global_values, Origin::Global, global_file),
    ] {
        for (name, value) in values {
            let params = declared(name);
            if params.is_empty() {
                if origin == Origin::Private {
                    diags.error(
                        &place,
                        format!(
                            "{} names no parameter of the rule set's sources",
                            shown(name)
                        ),
                    );
                }
                continue;
            }
            if params.iter().any(|p| p.scope == Scope::Repo) {
                diags.warning(
                    &place,
                    format!(
                        "{} is a parameter of scope `repo`, which only {file} sets; the value here is ignored",
                        shown(name)
                    ),
                );
                continue;
            }
            if let Some(why) = params.iter().find_map(|p| p.refuse(value)) {
                diags.error(&place, format!("{}: {why}", shown(name)));
                continue;
            }
            if let Some(setting) = out.get_mut(name)
                && setting.personal.is_none()
            {
                setting.personal = Some((value.clone(), origin));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_and_fill() {
        let text = "Use {$language.docs}, {$Bad.name} and {$x}; {$work-items.done}";
        assert_eq!(
            placeholders(text),
            [
                Placeholder::Name("language.docs"),
                Placeholder::Malformed("Bad.name"),
                Placeholder::Malformed("x"),
                Placeholder::Name("work-items.done"),
            ]
        );
        assert_eq!(
            fill(text, |n| n.to_uppercase()),
            "Use LANGUAGE.DOCS, {$Bad.name} and {$x}; WORK-ITEMS.DONE"
        );
        assert_eq!(placeholders("open {$a.b"), [Placeholder::Malformed("a.b")]);
    }
}
