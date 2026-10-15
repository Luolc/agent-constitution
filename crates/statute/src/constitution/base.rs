//! Check 7: the comparison with a base commit. Once the base is active,
//! what it numbered still exists, unmoved; within one edition, the
//! revision rises exactly when the text changes.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;
use toml::{Table, Value};

use super::read::parse;
use super::{
    ARTICLE_FILE, ARTICLE_NAME, CONSTITUTION, Doc, Errors, INDEX_FILE, Layout, SECTION_NAME,
    Sources, name_number,
};

/// A git command that failed, or a base with more than one edition
/// directory: check 7 cannot be run.
#[derive(Debug)]
pub struct Unreadable(pub String);

/// Runs git in `root`; its output, or an error with its stderr.
fn git_bytes(root: &Path, args: &[&str]) -> Result<Vec<u8>, Unreadable> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(root).args(args);
    crate::check::without_git_locals(&mut cmd);
    let command = format!("git {}", args.join(" "));
    let out = cmd
        .output()
        .map_err(|e| Unreadable(format!("`{command}` failed: {e}")))?;
    if !out.status.success() {
        return Err(Unreadable(format!(
            "`{command}` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(out.stdout)
}

fn git(root: &Path, args: &[&str]) -> Result<String, Unreadable> {
    git_bytes(root, args).map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Whether `reference` names a commit.
fn is_commit(root: &Path, reference: &str) -> bool {
    git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ],
    )
    .is_ok()
}

/// The file `path` at the base, as text.
fn show(root: &Path, reference: &str, path: &Path) -> Result<String, Unreadable> {
    git(root, &["show", &format!("{reference}:{}", path.display())])
}

/// The files under `paths` in the base commit, relative to the checkout.
fn base_paths(
    root: &Path,
    reference: &str,
    paths: &[PathBuf],
) -> Result<BTreeSet<PathBuf>, Unreadable> {
    let shown: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
    let mut args = vec!["ls-tree", "-r", "--name-only", reference, "--"];
    args.extend(shown.iter().map(String::as_str));
    Ok(git(root, &args)?
        .split('\n')
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .collect())
}

/// The layout of the base commit; `None` when it has no edition directory.
fn base_layout(root: &Path, reference: &str) -> Result<Option<Layout>, Unreadable> {
    let files = base_paths(root, reference, &[PathBuf::from(CONSTITUTION)])?;
    Layout::find(files).map_err(Unreadable)
}

/// A value of the base as Python's `str` writes it: a string as it is,
/// a missing one as `None`.
fn shown(value: Option<&Value>) -> String {
    match value {
        None => "None".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
    }
}

/// Check 7 against the base commit `reference`: the numbering, then the
/// revision. A note for standard output when the numbering is not
/// compared.
pub fn check_base(
    root: &Path,
    reference: &str,
    src: &Sources,
    errors: &mut Errors,
) -> Result<Option<String>, Unreadable> {
    let note = check_numbering(root, reference, src, errors)?;
    check_revision(root, reference, src, errors)?;
    Ok(note)
}

/// The numbering part of check 7. Once the base is active: the edition
/// is the base's or a later one; while it is the base's, every article,
/// section and item of the base still exists, by ID, with the same number,
/// a section in the same article, and what was repealed is still repealed.
pub fn check_numbering(
    root: &Path,
    reference: &str,
    src: &Sources,
    errors: &mut Errors,
) -> Result<Option<String>, Unreadable> {
    if !is_commit(root, reference) {
        errors.add(
            src.layout.index(),
            1,
            7,
            format!("base `{reference}` is not a commit"),
        );
        return Ok(None);
    }
    let Some(layout) = base_layout(root, reference)? else {
        return Ok(Some(format!(
            "base {reference} has no {CONSTITUTION}/<YYYY-MM>/{INDEX_FILE}; numbering not compared"
        )));
    };
    let (index, articles) = (layout.index(), layout.articles());
    let listing = base_paths(root, reference, &[index.clone(), articles.clone()])?;
    let base = match parse(&show(root, reference, &index)?) {
        Ok(base) => base,
        Err(e) => {
            errors.add(
                &index,
                1,
                7,
                format!(
                    "{} at base {reference} is not valid TOML: {e}",
                    index.display()
                ),
            );
            return Ok(None);
        }
    };
    let status = base.get("status");
    if status.and_then(Value::as_str) != Some("active") {
        return Ok(Some(format!(
            "base {reference} is {}; numbering not compared",
            shown(status)
        )));
    }
    let has_article = listing.iter().any(|p| {
        p.file_name().is_some_and(|n| n == ARTICLE_FILE)
            && p.parent().and_then(Path::parent) == Some(articles.as_path())
    });
    if !has_article {
        return Ok(Some(format!(
            "base {reference} has no {}/<NN>-<id>/{ARTICLE_FILE}; numbering not compared",
            articles.display()
        )));
    }
    if let Some(stop) = check_edition(reference, src, &base, errors) {
        return Ok(stop);
    }
    let comparison = Comparison {
        root,
        reference,
        articles: src.articles.iter().map(|a| (a.id.as_str(), a)).collect(),
        sections: src.sections.iter().map(|s| (s.id.as_str(), s)).collect(),
    };
    for path in &listing {
        if path.parent().and_then(Path::parent) != Some(articles.as_path())
            || path.extension().is_none_or(|e| e != "toml")
        {
            continue;
        }
        let dir = name_of(path.parent());
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(article) = ARTICLE_NAME.captures(&dir) else {
            continue;
        };
        let is_article = path.file_name().is_some_and(|n| n == ARTICLE_FILE);
        let section = SECTION_NAME.captures(&stem);
        if !is_article && section.is_none() {
            continue;
        }
        let Some(old) = comparison.toml(path, errors)? else {
            continue;
        };
        match section {
            _ if is_article => comparison.article(path, &article[1], &article[2], &old, errors),
            Some(section) => comparison.section(path, &article[2], &section, &old, errors),
            None => {}
        }
    }
    Ok(None)
}

fn name_of(path: Option<&Path>) -> String {
    path.and_then(Path::file_name)
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Checks the status and the edition against an active base: `None` when
/// the numbering is to be compared, else what [`check_numbering`] returns.
fn check_edition(
    reference: &str,
    src: &Sources,
    base: &Table,
    errors: &mut Errors,
) -> Option<Option<String>> {
    if src.status != "active" {
        errors.add(
            src.layout.index(),
            src.index.line(&["status ="]),
            7,
            format!("status went back to draft from active at base {reference}"),
        );
        return None;
    }
    let base_edition = base.get("edition");
    if base_edition.and_then(Value::as_str) == Some(src.edition.as_str()) {
        return None;
    }
    // Both are `YYYY-MM`, so string order is month order.
    if let Some(b) = base_edition.and_then(Value::as_str)
        && src.edition.as_str() > b
    {
        return Some(Some(format!(
            "base {reference} is edition {b}, this is edition {}; numbering not compared",
            src.edition
        )));
    }
    errors.add(
        src.layout.index(),
        src.index.line(&["edition ="]),
        7,
        format!(
            "edition {} is not later than edition {} at base {reference}",
            src.edition,
            shown(base_edition)
        ),
    );
    Some(None)
}

/// The comparison of the sources with the files of a base commit.
struct Comparison<'a> {
    root: &'a Path,
    reference: &'a str,
    articles: BTreeMap<&'a str, &'a super::Article>,
    sections: BTreeMap<&'a str, &'a super::Section>,
}

impl Comparison<'_> {
    /// A file of the base, parsed; `None` when it is not valid TOML.
    fn toml(&self, path: &Path, errors: &mut Errors) -> Result<Option<Table>, Unreadable> {
        match parse(&show(self.root, self.reference, path)?) {
            Ok(t) => Ok(Some(t)),
            Err(e) => {
                errors.add(
                    path,
                    1,
                    7,
                    format!("at base {} is not valid TOML: {e}", self.reference),
                );
                Ok(None)
            }
        }
    }

    fn gone(&self, path: &Path, what: &str, errors: &mut Errors) {
        errors.add(
            path,
            1,
            7,
            format!(
                "{what} existed at base {} and is gone; repeal it instead",
                self.reference
            ),
        );
    }

    fn renumbered(&self, doc: &Doc, what: &str, old: &str, new: &str, errors: &mut Errors) {
        if old != new {
            errors.add(
                &doc.path,
                doc.line(&[format!("number = \"{new}\"")]),
                7,
                format!(
                    "{what} was renumbered from {old} to {new} since base {}",
                    self.reference
                ),
            );
        }
    }

    fn still_repealed(&self, doc: &Doc, what: &str, was: bool, is_now: bool, errors: &mut Errors) {
        if was && !is_now {
            errors.add(
                &doc.path,
                doc.line(&["repealed =", "status ="]),
                7,
                format!(
                    "{what} was repealed at base {} and is no longer",
                    self.reference
                ),
            );
        }
    }

    fn article(&self, path: &Path, digits: &str, id: &str, old: &Table, errors: &mut Errors) {
        let what = format!("article {id}");
        let Some(now) = self.articles.get(id) else {
            return self.gone(path, &what, errors);
        };
        self.renumbered(
            &now.doc,
            &what,
            &name_number(digits, ""),
            &now.number,
            errors,
        );
        let was = old.contains_key("repealed");
        self.still_repealed(&now.doc, &what, was, now.repealed.is_some(), errors);
    }

    fn section(
        &self,
        path: &Path,
        article: &str,
        section: &regex::Captures,
        old: &Table,
        errors: &mut Errors,
    ) {
        let id = &section[3];
        let what = format!("section {id}");
        let Some(now) = self.sections.get(id) else {
            return self.gone(path, &what, errors);
        };
        if now.article != article {
            errors.add(
                &now.doc.path,
                1,
                7,
                format!(
                    "section {id} moved from article {article} to {} since base {}",
                    now.article, self.reference
                ),
            );
        } else {
            let number = name_number(&section[1], &section[2]);
            self.renumbered(&now.doc, &what, &number, &now.number, errors);
        }
        let repealed = |t: &Table| t.get("status").and_then(Value::as_str) == Some("repealed");
        self.still_repealed(
            &now.doc,
            &what,
            repealed(old),
            now.status == "repealed",
            errors,
        );
        let items: BTreeMap<&str, &super::Item> =
            now.items.iter().map(|i| (i.id.as_str(), i)).collect();
        let old_items = old.get("items").and_then(Value::as_array);
        for item in old_items.into_iter().flatten().filter_map(Value::as_table) {
            let item_id = shown(item.get("id"));
            let what = format!("item {id}.{item_id}");
            let Some(now_item) = items.get(item_id.as_str()) else {
                errors.add(
                    &now.doc.path,
                    now.doc.line(&["[[items]]", "[content]"]),
                    7,
                    format!(
                        "{what} existed at base {} and is gone; repeal it instead",
                        self.reference
                    ),
                );
                continue;
            };
            let number = shown(item.get("number"));
            self.renumbered(&now.doc, &what, &number, &now_item.number, errors);
            let now_repealed = now_item.status == "repealed";
            self.still_repealed(&now.doc, &what, repealed(item), now_repealed, errors);
        }
    }
}

/// The sort key of a revision: a draft `0.<n>` sorts by its number,
/// before every approved revision, a whole number from 1; `None` for any
/// other string.
pub fn revision_key(revision: &str) -> Option<(u128, u128)> {
    static DRAFT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0\.[0-9]+$").unwrap());
    static WHOLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[1-9][0-9]*$").unwrap());
    if DRAFT.is_match(revision) {
        return revision[2..].parse().ok().map(|n| (0, n));
    }
    if WHOLE.is_match(revision) {
        return revision.parse().ok().map(|n| (n, 0));
    }
    None
}

/// Whether the constitution text differs from the base's: the article
/// sources and `index.toml` but for its `revision` line. The files git
/// tracks are compared as they are in the working tree, so the result
/// does not depend on what is committed. Each side's files are found by
/// its own layout and compared by their paths within it, so moving the
/// sources to another directory alone changes nothing.
fn text_changed(
    root: &Path,
    reference: &str,
    layout: &Layout,
    base: &Layout,
) -> Result<bool, Unreadable> {
    static REVISION: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?m)^revision = ".*"\n"#).unwrap());
    let articles = layout.articles();
    let tracked = git(root, &["ls-files", "--", &articles.display().to_string()])?;
    let paths: BTreeSet<PathBuf> = tracked
        .split('\n')
        .filter(|p| !p.is_empty() && root.join(p).is_file())
        .filter_map(|p| {
            Path::new(p)
                .strip_prefix(&articles)
                .ok()
                .map(Path::to_path_buf)
        })
        .collect();
    let base_articles = base.articles();
    let base_files: BTreeSet<PathBuf> =
        base_paths(root, reference, std::slice::from_ref(&base_articles))?
            .iter()
            .filter_map(|p| p.strip_prefix(&base_articles).ok().map(Path::to_path_buf))
            .collect();
    if paths != base_files {
        return Ok(true);
    }
    for p in &paths {
        let now = std::fs::read(root.join(&articles).join(p))
            .map_err(|e| Unreadable(format!("cannot read {}: {e}", articles.join(p).display())))?;
        let then = git_bytes(
            root,
            &[
                "show",
                &format!("{reference}:{}", base_articles.join(p).display()),
            ],
        )?;
        if now != then {
            return Ok(true);
        }
    }
    let base_index = show(root, reference, &base.index())?;
    let index = std::fs::read_to_string(root.join(layout.index()))
        .map_err(|e| Unreadable(format!("cannot read {}: {e}", layout.index().display())))?;
    Ok(REVISION.replace_all(&index, "") != REVISION.replace_all(&base_index, ""))
}

/// The revision part of check 7: within one edition the revision is
/// greater than the base's when the constitution text changed since it,
/// and equal to it when the text did not. A draft base counts as the same
/// edition whatever edition each names, so renaming the edition a draft
/// prepares does not restart its revisions, and the approved text's
/// revision is greater than the draft's. An active base of another
/// edition is not compared.
pub fn check_revision(
    root: &Path,
    reference: &str,
    src: &Sources,
    errors: &mut Errors,
) -> Result<(), Unreadable> {
    if !is_commit(root, reference) {
        return Ok(()); // check_numbering reports it
    }
    let Some(layout) = base_layout(root, reference)? else {
        return Ok(());
    };
    let Ok(base) = parse(&show(root, reference, &layout.index())?) else {
        return Ok(()); // check_numbering reports it
    };
    let base_revision = shown(base.get("revision"));
    let active = base.get("status").and_then(Value::as_str) == Some("active");
    if active && base.get("edition").and_then(Value::as_str) != Some(src.edition.as_str()) {
        return Ok(());
    }
    let revision = &src.revision;
    let line = src.index.line(&["revision ="]);
    let Some(key) = revision_key(revision) else {
        errors.add(
            src.layout.index(),
            line,
            7,
            format!("revision {revision} is neither `0.<n>` nor a whole number from 1"),
        );
        return Ok(());
    };
    let Some(base_key) = revision_key(&base_revision) else {
        return Ok(());
    };
    if text_changed(root, reference, &src.layout, &layout)? {
        if key <= base_key {
            errors.add(
                src.layout.index(),
                line,
                7,
                format!(
                    "the constitution text changed since base {reference} but revision {revision} is not greater than the base's {base_revision}"
                ),
            );
        }
    } else if *revision != base_revision {
        errors.add(
            src.layout.index(),
            line,
            7,
            format!(
                "revision {revision} differs from revision {base_revision} at base {reference} but the constitution text did not change"
            ),
        );
    }
    Ok(())
}
