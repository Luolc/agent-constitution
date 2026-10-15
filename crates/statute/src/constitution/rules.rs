//! What the constitution decides of the rules (`docs/design/checkers.md`,
//! the rules after check 7): a node's status agrees with `index.toml`'s;
//! each `{@constitution:...}` citation and each `authority` entry names a
//! provision that is not repealed, with a warning on an `authority` that
//! names a provision and one above it; and against a base commit, every
//! node of the base still exists and stays repealed or superseded.
//!
//! `statute validate <dir>` runs these when `constitution/` is beside
//! `<dir>`. Each finding names its file and line, `rules/7.toml:12`, with
//! the message the constitution's checks give it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use toml::{Table, Value};

use super::base::{self, Gate, Unreadable};
use super::read::parse;
use super::{CONSTITUTION, Doc, Errors, Sources, citations, load};
use crate::diag::Diagnostics;
use crate::node::{self, Id, Node, Status};
use crate::source::{PARAMS, Source};

/// Checks the rules of `source`, read from `dir`, against the
/// constitution beside `dir`, and with `base` against that commit too.
/// Without a `constitution/` beside `dir` it checks nothing, and fails
/// when `base` asks for the comparison. The notes of a comparison
/// skipped, for standard output.
pub fn validate(
    dir: &Path,
    source: &Source,
    base: Option<&str>,
    diags: &mut Diagnostics,
) -> Vec<String> {
    let root = dir
        .canonicalize()
        .ok()
        .and_then(|d| d.parent().map(Path::to_path_buf));
    let Some(root) = root.filter(|r| r.join(CONSTITUTION).is_dir()) else {
        if base.is_some() {
            diags.error(
                &source.dir,
                format!("--base compares with the constitution, and there is no {CONSTITUTION}/ beside it"),
            );
        }
        return Vec::new();
    };
    // The constitution's own errors are `statute constitution build`'s to
    // list; the rules cannot be checked against it until it loads.
    let Ok(src) = load(&root) else {
        diags.error(
            CONSTITUTION,
            "does not load, so the rules are not checked against it; `statute constitution build --check` lists its errors",
        );
        return Vec::new();
    };
    let rules = Rules::new(dir, source);
    rules.check(&src, diags);
    let Some(reference) = base else {
        return Vec::new();
    };
    let mut errors = Errors::default();
    let note = match rules.check_base(&root, reference, &src, &mut errors, diags) {
        Ok(note) => note,
        Err(Unreadable(e)) => {
            errors.add(src.layout.index(), 1, 7, e);
            None
        }
    };
    report(errors, diags);
    note.into_iter().collect()
}

/// Moves the constitution's errors, `path:line: check N: message`, into
/// `diags` as `path:line` and the message.
fn report(errors: Errors, diags: &mut Diagnostics) {
    for e in errors.0 {
        match e
            .split_once(": check ")
            .and_then(|(at, rest)| Some((at, rest.split_once(": ")?.1)))
        {
            Some((at, message)) => diags.error(at, message),
            None => diags.error("", e),
        }
    }
}

/// The nodes of one directory, with their files' text for the lines of
/// the findings.
struct Rules<'a> {
    dir: &'a Path,
    source: &'a Source,
    docs: BTreeMap<Id, Doc>,
}

impl<'a> Rules<'a> {
    fn new(dir: &'a Path, source: &'a Source) -> Rules<'a> {
        let docs = source
            .nodes
            .keys()
            .map(|&id| {
                let text =
                    std::fs::read_to_string(dir.join(node::file_name(id))).unwrap_or_default();
                let doc = Doc {
                    path: PathBuf::from(source.file(id)),
                    data: Table::new(),
                    text,
                };
                (id, doc)
            })
            .collect();
        Rules { dir, source, docs }
    }

    /// `rules/7.toml:12`: the node's file and the first line holding the
    /// first of `needles` found.
    fn at(&self, id: Id, needles: &[&str]) -> String {
        let doc = &self.docs[&id];
        format!("{}:{}", doc.path.display(), doc.line(needles))
    }

    /// The status against `index.toml`'s, the citations of the
    /// constitution and `authority`, and the warning on an `authority`
    /// naming a provision and one above it.
    fn check(&self, src: &Sources, diags: &mut Diagnostics) {
        let provisions = src.provisions();
        let repealed = src.repealed();
        let ancestors = src.ancestors();
        let problem = |target: &str, written: &str| {
            if !provisions.contains(target) {
                Some(format!("undefined {written}"))
            } else if repealed.contains(target) {
                Some(format!("repealed {written}"))
            } else {
                None
            }
        };
        for (&id, n) in &self.source.nodes {
            // A rule may stay draft once the constitution is active; a
            // section may not, and that is the constitution's to check.
            if src.status == "draft" && n.status != Status::Draft {
                diags.error(
                    self.at(id, &["status ="]),
                    format!(
                        "a {} is {} while index.toml is draft",
                        n.kind.name(),
                        status_name(n.status)
                    ),
                );
            }
            if n.status == Status::Repealed {
                continue;
            }
            for text in texts(n) {
                for cited in citations(text) {
                    if let Some(target) = cited.strip_prefix("constitution:")
                        && let Some(found) = problem(target, &cited)
                    {
                        diags.error(
                            self.at(id, &[&format!("{{@{cited}}}")]),
                            format!("cites {found}"),
                        );
                    }
                }
            }
            let authority = n.rule.as_ref().map_or(&[][..], |r| &r.authority[..]);
            // An entry with a namespace fails the node's own format check.
            for a in authority.iter().filter(|a| !a.contains(':')) {
                if let Some(found) = problem(a, a) {
                    diags.error(
                        self.at(id, &["authority ="]),
                        format!("authority cites {found}"),
                    );
                }
            }
            if n.status == Status::Superseded {
                continue;
            }
            for a in authority {
                for above in ancestors.get(a).into_iter().flatten() {
                    if authority.contains(above) {
                        diags.warning(
                            self.at(id, &["authority ="]),
                            format!(
                                "rule {id}: authority names both {above} and {a}, which is under it"
                            ),
                        );
                    }
                }
            }
        }
    }

    /// The rules part of check 7, when the numbering is compared: every
    /// node of the base still exists, a rule of the older format, named
    /// by a word, as the node whose slug is that word; what was repealed
    /// or superseded still is. The note when it is skipped.
    fn check_base(
        &self,
        root: &Path,
        reference: &str,
        src: &Sources,
        errors: &mut Errors,
        diags: &mut Diagnostics,
    ) -> Result<Option<String>, Unreadable> {
        if let Gate::Stop(note) = base::gate(root, reference, src, errors)? {
            return Ok(note);
        }
        // Paths relative to the rules directory, wherever it is in the
        // repository: `ls-tree` lists them from the current directory and
        // `./` makes `show` read them from there.
        let listing = base::git(
            self.dir,
            &["ls-tree", "-r", "--name-only", reference, "--", "."],
        )?;
        let names: BTreeSet<&str> = listing
            .split('\n')
            .filter(|p| !p.contains('/') && p.ends_with(".toml") && *p != PARAMS)
            .collect();
        let slugs: BTreeMap<&str, &Node> = self
            .source
            .nodes
            .values()
            .filter_map(|n| Some((n.slug.as_deref()?, n)))
            .collect();
        for name in names {
            let shown = format!("{}/{name}", self.source.dir);
            let text = base::show(self.dir, reference, Path::new(&format!("./{name}")))?;
            let old = match parse(&text) {
                Ok(old) => old,
                Err(e) => {
                    diags.error(
                        format!("{shown}:1"),
                        format!("at base {reference} is not valid TOML: {e}"),
                    );
                    continue;
                }
            };
            let stem = name.trim_end_matches(".toml");
            let old_status = old.get("status").and_then(Value::as_str);
            if stem.bytes().all(|b| b.is_ascii_digit()) {
                let Some(now) = stem.parse().ok().and_then(|id| self.source.get(id)) else {
                    let kind = old.get("kind").and_then(Value::as_str).unwrap_or("rule");
                    diags.error(
                        format!("{shown}:1"),
                        format!("{kind} {stem} existed at base {reference} and is gone; repeal it instead"),
                    );
                    continue;
                };
                for status in [Status::Repealed, Status::Superseded] {
                    if old_status == Some(status_name(status)) && now.status != status {
                        diags.error(
                            self.at(now.id, &["status ="]),
                            format!(
                                "rule {} was {} at base {reference} and is no longer",
                                now.id,
                                status_name(status)
                            ),
                        );
                    }
                }
                continue;
            }
            let Some(now) = slugs.get(stem) else {
                diags.error(
                    format!("{shown}:1"),
                    format!(
                        "rule {stem} existed at base {reference} and no node has it as its slug"
                    ),
                );
                continue;
            };
            if old_status == Some("repealed") && now.status != Status::Repealed {
                diags.error(
                    self.at(now.id, &["repealed =", "status ="]),
                    format!("rule {stem} was repealed at base {reference} and is no longer"),
                );
            }
        }
        Ok(None)
    }
}

/// The texts of a node, in the order of its file: the title, a domain's
/// applicability and a rule's parts, each in every language.
fn texts(n: &Node) -> impl Iterator<Item = &String> {
    let parts = n
        .rule
        .iter()
        .flat_map(|r| [&r.statement, &r.reason, &r.verification]);
    std::iter::once(&n.title)
        .chain(&n.applicability)
        .chain(parts)
        .flat_map(|t| t.values())
}

fn status_name(status: Status) -> &'static str {
    match status {
        Status::Draft => "draft",
        Status::Trial => "trial",
        Status::Active => "active",
        Status::Superseded => "superseded",
        Status::Repealed => "repealed",
    }
}
