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
    /// of its name, an item's from its `number`.
    pub fn load(articles: &Path, lang: &str) -> Result<Labels, String> {
        let mut labels = BTreeMap::new();
        for (article_number, article_id, dir) in named(articles, false)? {
            let article = label(lang, Level::Article, &article_number)
                .map_err(|e| format!("{}: {e}", dir.display()))?;
            for (section_number, section_id, file) in named(&dir, true)? {
                let section = article.clone()
                    + &label(lang, Level::Section, &section_number)
                        .map_err(|e| format!("{}: {e}", file.display()))?;
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
                    let (Some(id), Some(number)) = (field("id"), field("number")) else {
                        return Err(format!(
                            "{}: an item without a string `id` and `number`",
                            file.display()
                        ));
                    };
                    let short = label(lang, Level::Item, number)
                        .map_err(|e| format!("{}: {e}", file.display()))?;
                    labels.insert(format!("{section_id}.{id}"), section.clone() + &short);
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
fn named(dir: &Path, sections: bool) -> Result<Vec<(String, String, PathBuf)>, String> {
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

#[derive(Clone, Copy)]
enum Level {
    Article,
    Section,
    Item,
}

/// The label of one level of a provision's number, `2a` as `§2a`: the
/// whole label of an article, the part a section adds to its article's,
/// and the part an item adds to its section's. Languages other than
/// Chinese are labeled in English.
fn label(lang: &str, level: Level, number: &str) -> Result<String, String> {
    let split = number
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(number.len());
    let (digits, suffix) = number.split_at(split);
    let n: u32 = match digits.parse() {
        Ok(n) if n > 0 && suffix.len() <= 1 && suffix.bytes().all(|b| b.is_ascii_lowercase()) => n,
        _ => return Err(format!("`{number}` is not a number such as 2 or 2a")),
    };
    Ok(match (lang, level) {
        ("zh", level) => {
            let suffix = match suffix.bytes().next() {
                Some(b) => format!("之{}", zh_number(u32::from(b - b'a') + 1)?),
                None => String::new(),
            };
            let unit = match level {
                Level::Article => "条",
                Level::Section => "款",
                Level::Item => "项",
            };
            format!("第{}{unit}{suffix}", zh_number(n)?)
        }
        (_, Level::Article) => format!("Art. {}{suffix}", roman(n)),
        (_, Level::Section) => format!(" §{n}{suffix}"),
        (_, Level::Item) => format!("({n}{suffix})"),
    })
}

fn roman(mut n: u32) -> String {
    const ROMAN: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (value, symbol) in ROMAN {
        while n >= value {
            out.push_str(symbol);
            n -= value;
        }
    }
    out
}

/// n in Chinese numerals, for 1 to 99: 十四, 二十.
fn zh_number(n: u32) -> Result<String, String> {
    const DIGITS: [&str; 10] = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
    if !(1..100).contains(&n) {
        return Err(format!("no Chinese numeral for {n}"));
    }
    let (tens, ones) = ((n / 10) as usize, (n % 10) as usize);
    if tens == 0 {
        return Ok(DIGITS[ones].to_string());
    }
    Ok(format!(
        "{}十{}",
        if tens == 1 { "" } else { DIGITS[tens] },
        if ones == 0 { "" } else { DIGITS[ones] }
    ))
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
/// hyphens, at most one dot.
fn parse(inner: &str) -> Option<Citation<'_>> {
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
