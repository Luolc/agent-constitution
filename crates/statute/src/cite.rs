//! Citations in a rule's text, `{@rule:<id>}` and `{@constitution:<id>}`
//! (`docs/design/source-formats.md`, IDs), and the labels of the
//! constitution's provisions they render as.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The label of every article, section and item of one edition, by
/// citation, as the constitution's version for agents writes them:
/// `Art. IV §1(2)` for `principals.competent`.
pub struct Labels(BTreeMap<String, String>);

impl Labels {
    /// Reads the numbers from the edition's sources, `articles` its
    /// `articles` directory: an article's and a section's from the prefix
    /// of its name, an item's from its `number`. The labels are those of
    /// the constitution's build (`constitution::labels`), which has no
    /// language table for `lang` labels in English.
    pub fn load(articles: &Path, lang: &str) -> Result<Labels, String> {
        let lng = crate::constitution::lang::language_or_default(lang);
        let mut labels = BTreeMap::new();
        for (article_number, article_id, dir) in named(articles, false)? {
            let (n, s) = number(&article_number).map_err(|e| format!("{}: {e}", dir.display()))?;
            let article = lng.article_label(n, &s);
            for (section_number, section_id, file) in named(&dir, true)? {
                let (n, s) =
                    number(&section_number).map_err(|e| format!("{}: {e}", file.display()))?;
                let section = lng.section_label(&article, n, &s);
                let text = std::fs::read_to_string(&file)
                    .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
                let doc: toml::Table = toml::from_str(&text)
                    .map_err(|e| format!("cannot parse {}: {e}", file.display()))?;
                for item in doc
                    .get("items")
                    .and_then(|i| i.as_array())
                    .into_iter()
                    .flatten()
                {
                    let field = |k: &str| item.get(k).and_then(|v| v.as_str());
                    let (Some(id), Some(item_number)) = (field("id"), field("number")) else {
                        return Err(format!(
                            "{}: an item without a string `id` and `number`",
                            file.display()
                        ));
                    };
                    let (n, s) =
                        number(item_number).map_err(|e| format!("{}: {e}", file.display()))?;
                    labels.insert(
                        format!("{section_id}.{id}"),
                        lng.item_label(&section, n, &s),
                    );
                }
                labels.insert(section_id, section);
            }
            labels.insert(article_id, article);
        }
        Ok(Labels(labels))
    }

    pub fn get(&self, id: &str) -> Option<&str> {
        self.0.get(id).map(String::as_str)
    }

    /// Every citation with its label, in order of citation.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(id, label)| (id.as_str(), label.as_str()))
    }
}

/// The entries of a directory named `<number>-<id>`: articles'
/// directories, or with `sections` sections' `.toml` files, as (number,
/// ID, path) in order, the number as the name writes it (`02a`).
pub(crate) fn named(dir: &Path, sections: bool) -> Result<Vec<(String, String, PathBuf)>, String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let stem = match (sections, name.strip_suffix(".toml")) {
            (false, _) => name.as_str(),
            (true, Some(stem)) => stem,
            (true, None) => continue,
        };
        if let Some((number, id)) = stem.split_once('-') {
            out.push((number.to_string(), id.to_string(), entry.path()));
        }
    }
    out.sort();
    Ok(out)
}

/// A number as a name prefix or an item's `number` writes it, `02a` or
/// `2a`, as its value and its letter suffix.
fn number(number: &str) -> Result<(u64, String), String> {
    let split = number
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(number.len());
    let (digits, suffix) = number.split_at(split);
    match digits.parse() {
        Ok(n) if n > 0 && suffix.len() <= 1 && suffix.bytes().all(|b| b.is_ascii_lowercase()) => {
            Ok((n, suffix.to_string()))
        }
        _ => Err(format!("`{number}` is not a number such as 2 or 2a")),
    }
}

/// One citation in a text.
pub enum Citation<'a> {
    Rule(&'a str),
    Constitution(&'a str),
    /// Without a namespace, which a rule's text must give.
    Bare(&'a str),
}

/// The text with each citation replaced by what `cite` returns for it;
/// anything else, braces included, stays as it is. The first error of
/// `cite` is returned.
pub fn replace(
    text: &str,
    mut cite: impl FnMut(Citation) -> Result<String, String>,
) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{@") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after
            .find('}')
            .and_then(|end| parse(&after[..end]).map(|c| (c, end)))
        {
            Some((citation, end)) => {
                out.push_str(&cite(citation)?);
                rest = &after[end + 1..];
            }
            None => {
                out.push('{');
                rest = &rest[start + 1..];
            }
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// `rule:7`, `constitution:leaks` or `must-ask.capital`, as the
/// constitution's build reads a citation: IDs of lowercase words joined by
/// hyphens, at most one dot. A rule of another source is cited as
/// `requires` names it: `rule:R12`, `rule:P7`, `rule:team:7`.
fn parse(inner: &str) -> Option<Citation<'_>> {
    if let Some(rule) = inner.strip_prefix("rule:")
        && crate::node::cited_ref(rule).is_some()
    {
        return Some(Citation::Rule(rule));
    }
    let (namespace, id) = match inner.split_once(':') {
        Some((ns, id)) => (Some(ns), id),
        None => (None, inner),
    };
    let word = |w: &str| {
        !w.is_empty()
            && w.split('-').all(|p| {
                !p.is_empty()
                    && p.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            })
    };
    let mut parts = id.split('.');
    let valid =
        parts.next().is_some_and(word) && parts.next().is_none_or(word) && parts.next().is_none();
    if !valid {
        return None;
    }
    match namespace {
        Some("rule") => Some(Citation::Rule(id)),
        Some("constitution") => Some(Citation::Constitution(id)),
        Some(_) => None,
        None => Some(Citation::Bare(id)),
    }
}
