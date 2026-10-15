//! The sources of a directory as JSON, for programs that read them
//! (`docs/design/statute-command.md`, Export): its constitution's
//! languages and provisions with their labels, and each node of its
//! `rules/` with the fields of its file and whether the source's defaults
//! enable it, after statute has checked them.

use std::path::Path;

use serde_json::{Map, Value, json};

use crate::cite::{self, Labels};
use crate::diag::Diagnostics;
use crate::node::{Kind, Status};
use crate::project::shown;
use crate::source::{Source, by_default};

/// The fields of `index.toml` the export gives.
#[derive(serde::Deserialize)]
struct Index {
    edition: String,
    revision: String,
    status: String,
    languages: Vec<String>,
}

/// Reads the constitution and the rules of `dir`, either of which may be
/// missing, and returns them as JSON:
///
/// - `constitution`: `null` when `dir` has no `constitution/`; else the
///   `edition`, `revision`, `status` and `languages` of its one edition's
///   `index.toml`, and its `provisions` in the order of the text, each an
///   article, a section or an item with its `id` as a citation names it,
///   its `parent` (`null` for an article) and its `labels` by language.
/// - `nodes`: the nodes of `rules/` in order of ID, each with its `file`,
///   the `fields` of the file as it writes them and `on-by-default`:
///   whether the source's defaults enable it, as statute checks them
///   (`docs/design/rule-config.md`, In the rule files), a rule only when
///   it is in force; `null` when that depends on another source.
///
/// The value is `None` when the sources do not pass statute's checks, so
/// every translation a node lists follows its current original.
pub fn export(dir: &Path) -> (Option<Value>, Diagnostics) {
    let mut diags = Diagnostics::default();
    let base = dir.to_string_lossy();
    let constitution = if dir.join("constitution").is_dir() {
        match constitution(dir) {
            Ok(value) => value,
            Err(e) => {
                diags.error(shown(&base, "constitution"), e);
                return (None, diags);
            }
        }
    } else {
        Value::Null
    };
    let rules_dir = dir.join("rules");
    let mut nodes = Vec::new();
    if rules_dir.is_dir() {
        let (source, more) = Source::load(&rules_dir, &shown(&base, "rules"));
        diags.0.extend(more.0);
        if diags.has_errors() {
            return (None, diags);
        }
        for (id, node) in &source.nodes {
            let file = source.file(*id);
            let retired = matches!(node.status, Status::Superseded | Status::Repealed);
            let on = if node.kind == Kind::Rule && retired {
                Some(false)
            } else {
                by_default(&source, *id)
            };
            let fields = std::fs::read_to_string(rules_dir.join(crate::node::file_name(*id)))
                .map_err(|e| format!("cannot read: {e}"))
                .and_then(|text| toml::from_str::<toml::Table>(&text).map_err(|e| e.to_string()));
            match fields {
                Ok(fields) => nodes.push(json!({
                    "file": file,
                    "fields": fields,
                    "on-by-default": on,
                })),
                Err(e) => diags.error(file, e),
            }
        }
        if diags.has_errors() {
            return (None, diags);
        }
    }
    (
        Some(json!({"constitution": constitution, "nodes": nodes})),
        diags,
    )
}

/// The one edition under `dir/constitution/`, as the export gives it.
fn constitution(dir: &Path) -> Result<Value, String> {
    let root = dir.join("constitution");
    let entries =
        std::fs::read_dir(&root).map_err(|e| format!("cannot read the directory: {e}"))?;
    let mut editions = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read the directory: {e}"))?;
        if entry.path().join("index.toml").is_file() {
            editions.push(entry.path());
        }
    }
    let [edition] = editions.as_slice() else {
        return Err(format!(
            "holds {} editions; the export reads a directory with one",
            editions.len()
        ));
    };
    let text = std::fs::read_to_string(edition.join("index.toml"))
        .map_err(|e| format!("cannot read index.toml: {e}"))?;
    let index: Index =
        toml::from_str(&text).map_err(|e| format!("index.toml: {}", e.to_string().trim_end()))?;
    let articles = edition.join("articles");
    let labels = index
        .languages
        .iter()
        .map(|lang| Labels::load(&articles, lang).map(|l| (lang, l)))
        .collect::<Result<Vec<_>, _>>()?;
    let provision = |id: &str, parent: Option<&str>| {
        let labels: Map<String, Value> = labels
            .iter()
            .map(|(lang, l)| (lang.to_string(), json!(l.get(id))))
            .collect();
        json!({"id": id, "parent": parent, "labels": labels})
    };
    let mut provisions = Vec::new();
    for (_, article, article_dir) in cite::named(&articles, false)? {
        provisions.push(provision(&article, None));
        for (_, section, file) in cite::named(&article_dir, true)? {
            provisions.push(provision(&section, Some(&article)));
            for item in items(&file)? {
                provisions.push(provision(&format!("{section}.{item}"), Some(&section)));
            }
        }
    }
    Ok(json!({
        "edition": index.edition,
        "revision": index.revision,
        "status": index.status,
        "languages": index.languages,
        "provisions": provisions,
    }))
}

/// The IDs of a section's items, in the order its file lists them.
fn items(file: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let doc: toml::Table =
        toml::from_str(&text).map_err(|e| format!("cannot parse {}: {e}", file.display()))?;
    Ok(doc
        .get("items")
        .and_then(|i| i.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("id")?.as_str().map(str::to_owned))
        .collect())
}
