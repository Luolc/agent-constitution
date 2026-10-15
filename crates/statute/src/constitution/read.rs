//! Check 1: read the sources of the edition and check their format.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

use super::{
    ARTICLE_FILE, ARTICLE_NAME, Article, CONSTITUTION, Doc, Errors, Item, Layout, MONTH, NUMBER,
    SECTION_NAME, Section, Sources, Texts, lang, name_number,
};

const ARTICLE_NAME_SHAPE: &str = "two digits from 01, a hyphen and the ID (04-<id>)";
const SECTION_NAME_SHAPE: &str =
    "two digits from 01, an optional lowercase letter, a hyphen and the ID (02-<id>, 02a-<id>)";
const NUMBER_SHAPE: &str = "an integer with an optional letter suffix (2, 2a)";
const INDEX_STATUS: &[&str] = &["draft", "active"];
const STATUS: &[&str] = &["draft", "active", "repealed"];
/// The one value of `audience`: a provision on how this text itself is
/// made and amended, whose text the version for agents leaves out.
const MAINTAINERS: &str = "maintainers";

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Str,
    List,
    /// A table whose keys are the languages of `index.toml` and whose
    /// values are text.
    Lang,
}

/// Key -> type; a trailing `?` marks an optional key.
type Schema = &'static [(&'static str, Kind)];

const INDEX_SCHEMA: Schema = &[
    ("status", Kind::Str),
    ("edition", Kind::Str),
    ("revision", Kind::Str),
    ("languages", Kind::List),
    ("title", Kind::Lang),
    ("front", Kind::Lang),
];
const ARTICLE_SCHEMA: Schema = &[
    ("id", Kind::Str),
    ("repealed?", Kind::Str),
    ("title", Kind::Lang),
    ("intro?", Kind::Lang),
];
const SECTION_SCHEMA: Schema = &[
    ("id", Kind::Str),
    ("status", Kind::Str),
    ("repealed?", Kind::Str),
    ("audience?", Kind::Str),
    ("content", Kind::Lang),
    ("items?", Kind::List),
];
const ITEM_SCHEMA: Schema = &[
    ("id", Kind::Str),
    ("number", Kind::Str),
    ("status", Kind::Str),
    ("repealed?", Kind::Str),
    ("audience?", Kind::Str),
    ("content", Kind::Lang),
];

/// Check 1: reads every source of the edition at `root` and checks its
/// format; the errors when any is found.
pub fn load(root: &Path) -> Result<Sources, Errors> {
    let mut errors = Errors::default();
    let layout = match Layout::of(root) {
        Ok(layout) => layout,
        Err(e) => {
            errors.add(CONSTITUTION, 1, 1, e);
            return Err(errors);
        }
    };
    let Some(index) = load_index(root, &layout, &mut errors) else {
        return Err(errors);
    };
    let languages: Vec<String> = index.data["languages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    let field = |k: &str| index.data[k].as_str().unwrap_or_default().to_string();
    let status = field("status");
    let mut reader = Reader {
        root,
        layout: &layout,
        errors: &mut errors,
        languages: &languages,
        index_status: &status,
        articles: Vec::new(),
        sections: Vec::new(),
    };
    if root.join(layout.old_sections()).exists() {
        reader.errors.add(
            layout.old_sections(),
            1,
            1,
            format!(
                "sections belong in their article's directory, {}/<NN>-<id>/<NN>[a-z]-<id>.toml",
                layout.articles().display()
            ),
        );
    }
    for dir in reader.entries(&layout.articles()) {
        reader.article_directory(&dir);
    }
    let (articles, sections) = (reader.articles, reader.sections);
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Sources {
        edition: field("edition"),
        revision: field("revision"),
        title: texts(&index.data["title"]),
        front: texts(&index.data["front"]),
        layout,
        index,
        languages,
        status,
        articles,
        sections,
    })
}

/// Reads `index.toml` and checks its format; `None` on any error.
fn load_index(root: &Path, layout: &Layout, errors: &mut Errors) -> Option<Doc> {
    let path = layout.index();
    let index = read_toml(root, &path, errors)?;
    if !check_keys(&index, &index.data, INDEX_SCHEMA, None, errors, "", 1) {
        return None;
    }
    let line = index.line(&["languages ="]);
    let listed = index.data["languages"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let languages: Vec<String> = listed
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    if listed.is_empty()
        || languages.len() != listed.len()
        || languages.iter().collect::<BTreeSet<_>>().len() != languages.len()
    {
        errors.add(&path, line, 1, "`languages` must list each language once");
        return None;
    }
    for code in &languages {
        if lang::language(code).is_none() {
            errors.add(
                &path,
                line,
                1,
                format!(
                    "no rendering for language `{code}`; add it to LANGUAGES in crates/statute/src/constitution/lang.rs"
                ),
            );
        }
    }
    let status = index.data["status"].as_str().unwrap_or_default();
    if !INDEX_STATUS.contains(&status) {
        errors.add(
            &path,
            index.line(&["status ="]),
            1,
            format!("`status` must be one of {}", tuple(INDEX_STATUS)),
        );
    }
    let edition = index.data["edition"].as_str().unwrap_or_default();
    if !MONTH.is_match(edition) {
        errors.add(
            &path,
            index.line(&["edition ="]),
            1,
            format!("`edition` must be `YYYY-MM`, got `{edition}`"),
        );
    }
    check_keys(
        &index,
        &index.data,
        INDEX_SCHEMA,
        Some(&languages),
        errors,
        "",
        1,
    );
    errors.is_empty().then_some(index)
}

/// Reads and parses one TOML file, `path` relative to `root`; on failure
/// records check 1.
pub(crate) fn read_toml(root: &Path, path: &Path, errors: &mut Errors) -> Option<Doc> {
    let bytes = match std::fs::read(root.join(path)) {
        Ok(bytes) => bytes,
        Err(e) => {
            errors.add(path, 1, 1, format!("cannot read: {}", strerror(&e)));
            return None;
        }
    };
    let Ok(text) = String::from_utf8(bytes) else {
        errors.add(path, 1, 1, "not UTF-8");
        return None;
    };
    match parse(&text) {
        Ok(data) => Some(Doc {
            path: path.to_path_buf(),
            data,
            text,
        }),
        Err(e) => {
            errors.add(path, 1, 1, format!("not valid TOML: {e}"));
            None
        }
    }
}

/// Parses a TOML text; the error is one line, with where it is.
pub(crate) fn parse(text: &str) -> Result<Table, String> {
    text.parse::<Table>().map_err(|e| {
        let message = e.message().trim_end().replace('\n', " ");
        match e.span() {
            Some(span) => {
                let before = &text[..span.start.min(text.len())];
                let line = before.matches('\n').count() + 1;
                let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
                format!("{message} (at line {line}, column {column})")
            }
            None => message,
        }
    })
}

/// An I/O error without Rust's ` (os error N)`.
fn strerror(e: &std::io::Error) -> String {
    let text = e.to_string();
    match text.find(" (os error") {
        Some(at) => text[..at].to_string(),
        None => text,
    }
}

/// A list of words as Python writes a tuple of strings: `('a', 'b')`.
fn tuple(words: &[&str]) -> String {
    let quoted: Vec<String> = words.iter().map(|w| format!("'{w}'")).collect();
    format!("({})", quoted.join(", "))
}

/// A list of words as Python writes a list of strings: `['a', 'b']`.
fn list<'a>(words: impl IntoIterator<Item = &'a String>) -> String {
    let quoted: Vec<String> = words.into_iter().map(|w| format!("'{w}'")).collect();
    format!("[{}]", quoted.join(", "))
}

/// A language table's texts, without the newlines that open and close
/// each: the one after the opening quotes and the one before the closing.
fn texts(value: &Value) -> Texts {
    value
        .as_table()
        .into_iter()
        .flatten()
        .map(|(k, v)| {
            let text = v.as_str().unwrap_or_default().trim_matches('\n');
            (k.clone(), text.to_string())
        })
        .collect()
}

/// Checks that `table` has the schema's keys, no others, of their types;
/// a language table has exactly `languages`, when they are known. `label`
/// names the table in messages, "" for the file's top level, and `start`
/// is its first line, where the search for its keys begins. True when
/// clean.
fn check_keys(
    doc: &Doc,
    table: &Table,
    schema: Schema,
    languages: Option<&[String]>,
    errors: &mut Errors,
    label: &str,
    start: usize,
) -> bool {
    let before = errors.len();
    let label = if label.is_empty() {
        String::new()
    } else {
        format!("`{label}` ")
    };
    let path = &doc.path;
    for &(key, kind) in schema {
        let name = key.trim_end_matches('?');
        let Some(value) = table.get(name) else {
            if !key.ends_with('?') {
                errors.add(path, start, 1, format!("{label}lacks `{name}`"));
            }
            continue;
        };
        let line = doc.line_from(&[format!("[{name}]"), format!("{name} =")], start);
        let problem = match kind {
            Kind::Lang => match value.as_table() {
                Some(t) if t.values().all(Value::is_str) => {
                    let keys: BTreeSet<&String> = t.keys().collect();
                    match languages {
                        Some(langs) if keys != langs.iter().collect() => Some(format!(
                            "`{name}` has languages {}, index.toml lists {}",
                            list(t.keys()),
                            list(langs)
                        )),
                        _ if name == "title"
                            && t.values()
                                .filter_map(Value::as_str)
                                .any(|s| s.trim_matches('\n').contains('\n')) =>
                        {
                            Some("`title` must be one line".to_string())
                        }
                        _ => None,
                    }
                }
                _ => Some(format!("`{name}` must be a table of language to text")),
            },
            Kind::Str => (!value.is_str()).then(|| format!("`{name}` must be a str")),
            Kind::List => (!value.is_array()).then(|| format!("`{name}` must be a list")),
        };
        if let Some(problem) = problem {
            errors.add(path, line, 1, format!("{label}{problem}"));
        }
    }
    let known: BTreeSet<&str> = schema
        .iter()
        .map(|(k, _)| k.trim_end_matches('?'))
        .collect();
    for name in table.keys().filter(|k| !known.contains(k.as_str())) {
        errors.add(
            path,
            doc.line_from(&[format!("[{name}]"), format!("{name} =")], start),
            1,
            format!("{label}unknown key `{name}`"),
        );
    }
    errors.len() == before
}

/// Checks that `repealed` is a year-month and is there exactly when the
/// status says repealed; `status` is `None` for an article, which has
/// none.
fn check_repealed(
    doc: &Doc,
    table: &Table,
    status: Option<&str>,
    errors: &mut Errors,
    label: &str,
    start: usize,
) {
    let label = if label.is_empty() {
        String::new()
    } else {
        format!("`{label}` ")
    };
    let value = table.get("repealed").and_then(Value::as_str);
    let line = doc.line_from(&["repealed ="], start);
    if let Some(v) = value
        && !MONTH.is_match(v)
    {
        errors.add(
            &doc.path,
            line,
            1,
            format!("{label}`repealed` must be `YYYY-MM`, got `{v}`"),
        );
    }
    match (status, value) {
        (Some("repealed"), None) => errors.add(
            &doc.path,
            doc.line_from(&["status ="], start),
            1,
            format!("{label}status is repealed but there is no `repealed` date"),
        ),
        (Some(s), Some(_)) if s != "repealed" => errors.add(
            &doc.path,
            line,
            1,
            format!("{label}has `repealed` but status is {s}"),
        ),
        _ => {}
    }
}

/// What reading the articles needs, once `index.toml` is read.
struct Reader<'a> {
    root: &'a Path,
    layout: &'a Layout,
    errors: &'a mut Errors,
    languages: &'a [String],
    index_status: &'a str,
    articles: Vec<Article>,
    sections: Vec<Section>,
}

impl Reader<'_> {
    /// What the directory holds, relative to the root, sorted, without
    /// hidden names (`.DS_Store`, editor swap files and the like).
    fn entries(&self, dir: &Path) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = std::fs::read_dir(self.root.join(dir))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.file_name())
            .filter(|n| !n.to_string_lossy().starts_with('.'))
            .map(|n| dir.join(n))
            .collect();
        out.sort();
        out
    }

    /// Reads one source and checks its keys.
    fn read(&mut self, path: &Path, schema: Schema) -> Option<Doc> {
        let doc = read_toml(self.root, path, self.errors)?;
        check_keys(
            &doc,
            &doc.data,
            schema,
            Some(self.languages),
            self.errors,
            "",
            1,
        )
        .then_some(doc)
    }

    /// Checks that the ID is the one the file or directory name gives.
    fn own_id(&mut self, doc: &Doc, expected: &str, name: &str) {
        if doc.data["id"].as_str() != Some(expected) {
            self.errors.add(
                &doc.path,
                doc.line(&["id ="]),
                1,
                format!("`id` must be `{expected}`, as in the name `{name}`"),
            );
        }
    }

    /// Checks a status against the allowed ones and `index.toml`'s.
    fn status_of(&mut self, doc: &Doc, table: &Table, what: &str, start: usize) -> String {
        let status = table["status"].as_str().unwrap_or_default().to_string();
        let line = doc.line_from(&["status ="], start);
        if !STATUS.contains(&status.as_str()) {
            self.errors.add(
                &doc.path,
                line,
                1,
                format!("{what} `status` must be one of {}", tuple(STATUS)),
            );
        } else if self.index_status == "draft" && status != "draft" {
            self.errors.add(
                &doc.path,
                line,
                1,
                format!("{what} is {status} while index.toml is draft"),
            );
        } else if self.index_status == "active" && status == "draft" {
            self.errors.add(
                &doc.path,
                line,
                1,
                format!("{what} is draft while index.toml is active"),
            );
        }
        status
    }

    /// Checks `audience`; whether the version for agents omits the text.
    fn omitted(&mut self, doc: &Doc, table: &Table, what: &str, start: usize) -> bool {
        let audience = table.get("audience").and_then(Value::as_str);
        if audience.is_some_and(|a| a != MAINTAINERS) {
            self.errors.add(
                &doc.path,
                doc.line_from(&["audience ="], start),
                1,
                format!("{what} `audience` must be `{MAINTAINERS}`"),
            );
        }
        audience == Some(MAINTAINERS)
    }

    /// Reads one article directory: its `article.toml`, then its sections.
    fn article_directory(&mut self, dir: &Path) {
        if !self.root.join(dir).is_dir() {
            self.errors.add(
                dir,
                1,
                1,
                format!(
                    "not an article directory; an article is {}/<NN>-<id>/ with its {ARTICLE_FILE} and section files",
                    self.layout.articles().display()
                ),
            );
            return;
        }
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(m) = ARTICLE_NAME.captures(&name) else {
            self.errors.add(
                dir,
                1,
                1,
                format!("article directory name is not {ARTICLE_NAME_SHAPE}"),
            );
            return;
        };
        let (digits, article_id) = (m[1].to_string(), m[2].to_string());
        let path = dir.join(ARTICLE_FILE);
        if !self.root.join(&path).is_file() {
            self.errors.add(
                dir,
                1,
                1,
                format!("article directory lacks `{ARTICLE_FILE}`"),
            );
            return;
        }
        self.article(&path, &article_id, &digits, &name);
        for path in self.entries(dir) {
            if path.file_name().is_some_and(|n| n != ARTICLE_FILE) {
                self.section(&path, &article_id);
            }
        }
    }

    fn article(&mut self, path: &Path, article_id: &str, digits: &str, dir_name: &str) {
        let Some(doc) = self.read(path, ARTICLE_SCHEMA) else {
            return;
        };
        self.own_id(&doc, article_id, dir_name);
        check_repealed(&doc, &doc.data, None, self.errors, "", 1);
        if self.index_status == "draft" && doc.data.contains_key("repealed") {
            self.errors.add(
                path,
                doc.line(&["repealed ="]),
                1,
                "an article is repealed while index.toml is draft",
            );
        }
        let d = &doc.data;
        self.articles.push(Article {
            id: d["id"].as_str().unwrap_or_default().to_string(),
            number: name_number(digits, ""),
            repealed: d
                .get("repealed")
                .and_then(Value::as_str)
                .map(str::to_string),
            title: texts(&d["title"]),
            intro: d.get("intro").map(texts),
            doc,
        });
    }

    /// Reads one section file with its items.
    fn section(&mut self, path: &Path, article_id: &str) {
        let is_toml = path.extension().is_some_and(|e| e == "toml");
        if !is_toml || !self.root.join(path).is_file() {
            self.errors.add(
                path,
                1,
                1,
                format!(
                    "not a section file; an article's directory holds only its {ARTICLE_FILE} and <NN>[a-z]-<id>.toml section files"
                ),
            );
            return;
        }
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(m) = SECTION_NAME.captures(&stem) else {
            self.errors.add(
                path,
                1,
                1,
                format!("section file name is not {SECTION_NAME_SHAPE}, then `.toml`"),
            );
            return;
        };
        let Some(doc) = self.read(path, SECTION_SCHEMA) else {
            return;
        };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.own_id(&doc, &m[3], &name);
        let status = self.status_of(&doc, &doc.data, "a section", 1);
        check_repealed(&doc, &doc.data, Some(&status), self.errors, "", 1);
        let omitted = self.omitted(&doc, &doc.data, "a section", 1);
        let listed = doc.data.get("items").and_then(Value::as_array);
        let items = listed
            .into_iter()
            .flatten()
            .enumerate()
            .filter_map(|(n, item)| self.item(&doc, n, item))
            .collect();
        let d = &doc.data;
        self.sections.push(Section {
            id: d["id"].as_str().unwrap_or_default().to_string(),
            article: article_id.to_string(),
            number: name_number(&m[1], &m[2]),
            repealed: d
                .get("repealed")
                .and_then(Value::as_str)
                .map(str::to_string),
            content: texts(&d["content"]),
            status,
            items,
            omitted,
            doc,
        });
    }

    /// Checks one list item of a section; `None` when its keys are wrong
    /// or it is not a table.
    fn item(&mut self, doc: &Doc, n: usize, item: &Value) -> Option<Item> {
        let label = format!("items[{n}]");
        let start = doc.item_line(n);
        let item = item.as_table()?;
        if !check_keys(
            doc,
            item,
            ITEM_SCHEMA,
            Some(self.languages),
            self.errors,
            &label,
            start,
        ) {
            return None;
        }
        let number = item["number"].as_str().unwrap_or_default();
        if !NUMBER.is_match(number) {
            self.errors.add(
                &doc.path,
                doc.line_from(&["number ="], start),
                1,
                format!("`{label}` `number` `{number}` is not {NUMBER_SHAPE}"),
            );
        }
        let what = format!("`{label}`");
        let status = self.status_of(doc, item, &what, start);
        check_repealed(doc, item, Some(&status), self.errors, &label, start);
        let omitted = self.omitted(doc, item, &what, start);
        Some(Item {
            id: item["id"].as_str().unwrap_or_default().to_string(),
            number: number.to_string(),
            status,
            repealed: item
                .get("repealed")
                .and_then(Value::as_str)
                .map(str::to_string),
            content: texts(&item["content"]),
            section: doc.data["id"].as_str().unwrap_or_default().to_string(),
            omitted,
        })
    }
}
