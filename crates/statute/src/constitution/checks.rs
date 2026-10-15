//! Checks 2 to 5 of the constitution: IDs, numbers, language alignment
//! and citations (`docs/design/checkers.md`).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use regex::Regex;

use super::{Doc, Errors, ID, Sources, citations, number_key};

static ID_SHAPE: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!("^{ID}$")).unwrap());

/// Checks 2 to 5 in order, stopping after the first that fails; the
/// errors, empty when all pass.
pub fn check(src: &Sources) -> Errors {
    let mut errors = Errors::default();
    for check in [check_ids, check_structure, check_languages, check_refs] {
        check(src, &mut errors);
        if !errors.is_empty() {
            break;
        }
    }
    errors
}

/// Check 2: every ID has the format, article and section IDs are unique
/// in the constitution and item IDs in their section.
fn check_ids(src: &Sources, errors: &mut Errors) {
    let declared = src
        .articles
        .iter()
        .map(|a| (&a.doc, a.id.as_str()))
        .chain(src.sections.iter().map(|s| (&s.doc, s.id.as_str())));
    unique_ids(errors, declared);
    for s in &src.sections {
        unique_ids(errors, s.items.iter().map(|i| (&s.doc, i.id.as_str())));
    }
}

fn unique_ids<'a>(errors: &mut Errors, declared: impl Iterator<Item = (&'a Doc, &'a str)>) {
    let mut seen: BTreeMap<&str, String> = BTreeMap::new();
    for (doc, value) in declared {
        let line = doc.line(&[format!("id = \"{value}\""), format!("slug = \"{value}\"")]);
        if !ID_SHAPE.is_match(value) {
            errors.add(
                &doc.path,
                line,
                2,
                format!("`{value}` is not an ID: lowercase words joined by hyphens"),
            );
        }
        if let Some(at) = seen.get(value) {
            errors.add(
                &doc.path,
                line,
                2,
                format!("{value} is already defined at {at}"),
            );
        } else {
            seen.insert(value, format!("{}:{line}", doc.path.display()));
        }
    }
}

/// Check 3: numbers are unique in their scope, and a repealed article has
/// only repealed sections.
fn check_structure(src: &Sources, errors: &mut Errors) {
    unique(
        errors,
        src.articles
            .iter()
            .map(|a| (&a.doc, a.number.as_str(), "article".to_string())),
    );
    for a in &src.articles {
        let own: Vec<_> = src.sections.iter().filter(|s| s.article == a.id).collect();
        unique(
            errors,
            own.iter()
                .map(|s| (&s.doc, s.number.as_str(), format!("section of {}", a.id))),
        );
        if a.repealed.is_some() {
            for s in own.iter().filter(|s| s.status != "repealed") {
                errors.add(
                    &s.doc.path,
                    s.doc.line(&["status ="]),
                    3,
                    format!(
                        "{} is {} but its article {} is repealed",
                        s.id, s.status, a.id
                    ),
                );
            }
        }
    }
    for s in &src.sections {
        unique(
            errors,
            s.items
                .iter()
                .map(|i| (&s.doc, i.number.as_str(), format!("item of {}", s.id))),
        );
    }
}

/// Checks that no number repeats among `numbered`, (doc, number, what)
/// for each provision of one scope.
fn unique<'a>(errors: &mut Errors, numbered: impl Iterator<Item = (&'a Doc, &'a str, String)>) {
    let mut seen = BTreeMap::new();
    for (doc, number, what) in numbered {
        let line = doc.line(&[format!("number = \"{number}\"")]);
        let key = number_key(number);
        if let Some(at) = seen.get(&key) {
            errors.add(
                &doc.path,
                line,
                3,
                format!("{what} number {number} is already used at {at}"),
            );
        } else {
            seen.insert(key, format!("{}:{line}", doc.path.display()));
        }
    }
}

/// Check 4: each language of a text cites what the first language of
/// `index.toml` cites.
fn check_languages(src: &Sources, errors: &mut Errors) {
    let Some(first) = src.languages.first() else {
        return;
    };
    for t in src.texts() {
        let refs: BTreeMap<&str, BTreeSet<String>> = t
            .texts
            .iter()
            .map(|(lang, text)| (lang.as_str(), citations(text).into_iter().collect()))
            .collect();
        let empty = BTreeSet::new();
        let of = |lang: &str| refs.get(lang).unwrap_or(&empty);
        for lang in src.languages.iter().filter(|l| *l != first) {
            for (a, b) in [(first, lang), (lang, first)] {
                for cite in of(a).difference(of(b)) {
                    errors.add(
                        &t.doc.path,
                        t.doc.line(&[format!("{{@{cite}}}")]),
                        4,
                        format!("{a} `{}` cites {cite}; {b} does not", t.field),
                    );
                }
            }
        }
    }
}

/// Check 5: every citation is of a provision that exists and is not
/// repealed, without a namespace; a repealed text is not checked.
fn check_refs(src: &Sources, errors: &mut Errors) {
    let (provisions, repealed) = (src.provisions(), src.repealed());
    for t in src.texts().into_iter().filter(|t| !t.repealed) {
        for lang in &src.languages {
            let Some(text) = t.texts.get(lang) else {
                continue;
            };
            for cite in citations(text) {
                let problem = if cite.contains(':') {
                    format!("{cite}; the constitution cites only itself, without a namespace")
                } else if !provisions.contains(&cite) {
                    format!("undefined {cite}")
                } else if repealed.contains(&cite) {
                    format!("repealed {cite}")
                } else {
                    continue;
                };
                errors.add(
                    &t.doc.path,
                    t.doc.line(&[format!("{{@{cite}}}")]),
                    5,
                    format!("cites {problem}"),
                );
            }
        }
    }
}
