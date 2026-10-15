//! The constitution's build: reads the sources of one edition, checks
//! them and renders the full version and the version for agents in each
//! language (`docs/design/source-formats.md`, `constitution.md` and
//! `checkers.md`).
//!
//! The sources are in the edition directory `constitution/<YYYY-MM>/`, the
//! one directory under `constitution/` that holds an `index.toml`:
//! `index.toml` and one directory per article, `articles/<NN>-<id>/`,
//! holding its `article.toml` and one `<NN>[a-z]-<id>.toml` per section.
//! The outputs go to its `generated/`. The rules are not read here.
//!
//! Errors are `path:line: check N: message`: check 1 is the format
//! ([`load`]), 2 to 5 the IDs, the numbers, the languages and the
//! citations ([`check`]), 6 the committed outputs and 7 the comparison
//! with a base commit ([`base`]).

pub mod base;
mod checks;
pub mod cli;
pub(crate) mod lang;
mod read;
mod render;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

pub use checks::check;
pub use read::load;
pub use render::{labels, render};

/// The command the generated files name as the one that wrote them.
pub const BUILD_COMMAND: &str = "cargo run -p statute -- constitution build";

pub const CONSTITUTION: &str = "constitution";
pub const INDEX_FILE: &str = "index.toml";
pub const ARTICLE_FILE: &str = "article.toml";
/// After the language in the name of the version for agents.
pub const AGENT: &str = ".agent";

const ID: &str = r"[a-z0-9]+(?:-[a-z0-9]+)*";

/// A citation: `{@id}` or `{@section.item}`, each optionally after a
/// namespace.
static MARK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"\{{@(?:(constitution|rule):)?({ID}(?:\.{ID})?)\}}"
    ))
    .unwrap()
});
static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([1-9][0-9]*)([a-z]?)$").unwrap());
static MONTH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9]{4})-(0[1-9]|1[0-2])$").unwrap());
static ARTICLE_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(0[1-9]|[1-9][0-9])-(.+)$").unwrap());
static SECTION_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(0[1-9]|[1-9][0-9])([a-z]?)-(.+)$").unwrap());

/// Where the sources and the generated files of an edition are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// The edition directory, relative to the checkout.
    pub dir: PathBuf,
}

impl Layout {
    pub fn index(&self) -> PathBuf {
        self.dir.join(INDEX_FILE)
    }

    pub fn articles(&self) -> PathBuf {
        self.dir.join("articles")
    }

    /// Where sections were before they moved into their articles.
    pub fn old_sections(&self) -> PathBuf {
        self.dir.join("sections")
    }

    /// Where a generated file goes: `name` is the language, with
    /// [`AGENT`] after it for the version for agents.
    pub fn output(&self, name: &str) -> PathBuf {
        self.dir
            .join("generated")
            .join(format!("constitution.{name}.md"))
    }

    /// The layout of a checkout or a commit from its files, relative to
    /// it; `None` when no directory holds an `index.toml`, an error when
    /// more than one does.
    pub fn find(files: impl IntoIterator<Item = PathBuf>) -> Result<Option<Layout>, String> {
        let dirs: BTreeSet<PathBuf> = files
            .into_iter()
            .filter(|p| {
                p.file_name().is_some_and(|n| n == INDEX_FILE)
                    && p.parent().and_then(Path::parent) == Some(Path::new(CONSTITUTION))
            })
            .filter_map(|p| p.parent().map(Path::to_path_buf))
            .collect();
        if dirs.len() > 1 {
            let names: Vec<String> = dirs.iter().map(|d| d.display().to_string()).collect();
            return Err(format!(
                "more than one edition directory: {}",
                names.join(", ")
            ));
        }
        Ok(dirs.into_iter().next().map(|dir| Layout { dir }))
    }

    /// The layout of the working tree at `root`.
    pub fn of(root: &Path) -> Result<Layout, String> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(root.join(CONSTITUTION)) {
            for entry in entries.flatten() {
                let index = entry.path().join(INDEX_FILE);
                if index.exists() {
                    files.push(
                        Path::new(CONSTITUTION)
                            .join(entry.file_name())
                            .join(INDEX_FILE),
                    );
                }
            }
        }
        Layout::find(files)?.ok_or_else(|| {
            format!("no edition directory, {CONSTITUTION}/<YYYY-MM>/ with its {INDEX_FILE}")
        })
    }
}

/// The errors found, each a line `path:line: check N: message`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Errors(pub Vec<String>);

impl Errors {
    pub fn add(
        &mut self,
        path: impl AsRef<Path>,
        line: usize,
        check: u8,
        message: impl fmt::Display,
    ) {
        self.0.push(format!(
            "{}:{line}: check {check}: {message}",
            path.as_ref().display()
        ));
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// One parsed TOML file with its text, for locating errors by line.
#[derive(Debug, Clone)]
pub struct Doc {
    /// The file, relative to the checkout.
    pub path: PathBuf,
    pub data: toml::Table,
    pub text: String,
}

impl Doc {
    /// The first line, from 1, holding the first of `needles` found, else 1.
    pub fn line<S: AsRef<str>>(&self, needles: &[S]) -> usize {
        self.line_from(needles, 1)
    }

    /// The first line from `start` on holding the first of `needles`
    /// found, else `start`.
    pub fn line_from<S: AsRef<str>>(&self, needles: &[S], start: usize) -> usize {
        for needle in needles {
            for (n, line) in self.text.split('\n').enumerate().skip(start - 1) {
                if line.contains(needle.as_ref()) {
                    return n + 1;
                }
            }
        }
        start
    }

    /// The line of the `n`-th `[[items]]` header, from 0, else 1.
    pub fn item_line(&self, n: usize) -> usize {
        self.text
            .split('\n')
            .enumerate()
            .filter(|(_, l)| l.trim() == "[[items]]")
            .nth(n)
            .map_or(1, |(k, _)| k + 1)
    }
}

/// A text in every language of `index.toml`: language -> text.
pub type Texts = BTreeMap<String, String>;

#[derive(Debug, Clone)]
pub struct Article {
    /// Its `article.toml`.
    pub doc: Doc,
    pub id: String,
    /// From the directory name.
    pub number: String,
    /// The month it was repealed, `YYYY-MM`, if it was.
    pub repealed: Option<String>,
    pub title: Texts,
    pub intro: Option<Texts>,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub doc: Doc,
    pub id: String,
    /// The ID of its article.
    pub article: String,
    /// From the file name: `2` or `2a`.
    pub number: String,
    /// draft, active or repealed.
    pub status: String,
    pub repealed: Option<String>,
    pub content: Texts,
    pub items: Vec<Item>,
    /// Whether the version for agents leaves out its text.
    pub omitted: bool,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub number: String,
    pub status: String,
    pub repealed: Option<String>,
    pub content: Texts,
    /// The ID of the section holding it.
    pub section: String,
    pub omitted: bool,
}

impl Item {
    /// How the constitution cites it: `<section id>.<item id>`.
    pub fn cite(&self) -> String {
        format!("{}.{}", self.section, self.id)
    }
}

/// Everything read from the sources of one edition.
#[derive(Debug, Clone)]
pub struct Sources {
    pub layout: Layout,
    /// `index.toml`.
    pub index: Doc,
    /// The languages of `index.toml`, in order.
    pub languages: Vec<String>,
    /// draft or active.
    pub status: String,
    pub edition: String,
    pub revision: String,
    pub title: Texts,
    pub front: Texts,
    /// In the order of their directories.
    pub articles: Vec<Article>,
    /// In the order of their articles and files.
    pub sections: Vec<Section>,
}

/// One multilingual text of the sources, with where it is.
pub(crate) struct Text<'a> {
    pub doc: &'a Doc,
    /// As in error messages.
    pub field: String,
    pub texts: &'a Texts,
    /// Whether what holds it is repealed.
    pub repealed: bool,
}

impl Sources {
    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.sections.iter().flat_map(|s| &s.items)
    }

    /// Every citation of a provision: the IDs of the articles and the
    /// sections and the `<section>.<item>` of the items.
    pub fn provisions(&self) -> BTreeSet<String> {
        let mut out: BTreeSet<String> = self.articles.iter().map(|a| a.id.clone()).collect();
        out.extend(self.sections.iter().map(|s| s.id.clone()));
        out.extend(self.items().map(Item::cite));
        out
    }

    /// The citations that are repealed; an item of a repealed section is
    /// repealed with it.
    pub fn repealed(&self) -> BTreeSet<String> {
        let mut out: BTreeSet<String> = self
            .articles
            .iter()
            .filter(|a| a.repealed.is_some())
            .map(|a| a.id.clone())
            .collect();
        for s in &self.sections {
            if s.status == "repealed" {
                out.insert(s.id.clone());
            }
            for i in &s.items {
                if i.status == "repealed" || s.status == "repealed" {
                    out.insert(i.cite());
                }
            }
        }
        out
    }

    /// The provisions above each section and item, nearest first: a
    /// section's article; an item's section and article.
    pub fn ancestors(&self) -> BTreeMap<String, Vec<String>> {
        let mut out = BTreeMap::new();
        for s in &self.sections {
            out.insert(s.id.clone(), vec![s.article.clone()]);
            for i in &s.items {
                out.insert(i.cite(), vec![s.id.clone(), s.article.clone()]);
            }
        }
        out
    }

    /// Every multilingual text, in order.
    pub(crate) fn texts(&self) -> Vec<Text<'_>> {
        let text = |doc, field: &str, texts, repealed| Text {
            doc,
            field: field.to_string(),
            texts,
            repealed,
        };
        let mut out = vec![
            text(&self.index, "title", &self.title, false),
            text(&self.index, "front", &self.front, false),
        ];
        for a in &self.articles {
            out.push(text(&a.doc, "title", &a.title, a.repealed.is_some()));
            if let Some(intro) = &a.intro {
                out.push(text(&a.doc, "intro", intro, a.repealed.is_some()));
            }
        }
        for s in &self.sections {
            out.push(text(&s.doc, "content", &s.content, s.status == "repealed"));
            for i in &s.items {
                out.push(text(
                    &s.doc,
                    &format!("{}.content", i.id),
                    &i.content,
                    i.status == "repealed",
                ));
            }
        }
        out
    }
}

/// The sort key of a number, `2a` as (2, "a"); a number that is not one
/// sorts first.
pub(crate) fn number_key(number: &str) -> (u64, String) {
    NUMBER
        .captures(number)
        .map(|c| (c[1].parse().unwrap_or(u64::MAX), c[2].to_string()))
        .unwrap_or_default()
}

/// The number a name prefix stands for: `02a` is 2a.
pub(crate) fn name_number(digits: &str, suffix: &str) -> String {
    format!("{}{suffix}", digits.parse::<u64>().unwrap_or(0))
}

/// The citations in the text as written, with their namespace if any:
/// `leaks`, `must-ask.capital`, `constitution:leaks`, `rule:x`.
pub(crate) fn citations(text: &str) -> Vec<String> {
    MARK.captures_iter(text)
        .map(|c| match c.get(1) {
            Some(ns) => format!("{}:{}", ns.as_str(), &c[2]),
            None => c[2].to_string(),
        })
        .collect()
}
