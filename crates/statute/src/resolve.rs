//! Resolving a rule set: which rules it enables, and why each
//! (`docs/design/rule-sets.md`, Dependencies, one-ofs and packs, and
//! `docs/design/rule-format.md`, Tracing rules to the constitution).

use std::collections::{BTreeMap, BTreeSet};

use crate::diag::Diagnostics;
use crate::manifest::{self, Key, Manifest};
use crate::node::{Kind, Node, Status};
use crate::source::{Sources, describe};

/// Why a rule is enabled. A rule may have several reasons.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// Listed in `rules`.
    Direct,
    /// Inside the pack with this root.
    Pack(Key),
    /// The parent, or the rule above a one-of, of this enabled rule.
    ParentOf(Key),
    /// Listed under `requires` of this enabled rule.
    RequiredBy(Key),
}

#[derive(Debug, Default)]
pub struct Resolution {
    /// Every enabled rule, by key, with the reasons it is enabled.
    pub enabled: BTreeMap<Key, BTreeSet<Reason>>,
}

/// The rule a node's parent requirement lands on: the parent when it is a
/// rule, through any one-of to the rule above it, and nothing when the
/// chain ends at a domain.
pub fn parent_rule(sources: &Sources, key: &Key) -> Option<Key> {
    let mut parent = sources.parent(key)?;
    let mut steps = 0;
    while sources.get(&parent)?.kind == Kind::OneOf && steps <= sources.nodes().count() {
        parent = sources.parent(&parent)?;
        steps += 1;
    }
    (sources.get(&parent)?.kind == Kind::Rule).then_some(parent)
}

fn title(node: &Node) -> &str {
    node.title.get(&node.lang).map_or("", String::as_str)
}

/// `R42 (Sync from the lock file first in CI)`.
pub fn label(sources: &Sources, key: &Key) -> String {
    match sources.get(key) {
        Some(n) => format!("{key} ({})", title(n)),
        None => key.to_string(),
    }
}

/// The labels of `keys`, joined by commas.
pub fn labels(sources: &Sources, keys: &[Key]) -> String {
    keys.iter()
        .map(|k| label(sources, k))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `keys` as references, joined by commas.
pub fn refs(keys: &[Key]) -> String {
    keys.iter()
        .map(Key::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The rules a superseded or repealed node of `key`'s source names in
/// `superseded-by`, as keys.
pub fn successors(key: &Key, node: &Node) -> Vec<Key> {
    node.superseded_by
        .iter()
        .map(|id| Key::new(&key.source, *id))
        .collect()
}

/// Resolves `manifest` against `sources`, which have passed their own
/// checks, for the constitution `revision`. Problems go to `diags`,
/// against `statute.toml`.
pub fn resolve(
    sources: &Sources,
    manifest: &Manifest,
    revision: u64,
    diags: &mut Diagnostics,
) -> Resolution {
    let file = manifest::FILE;
    let mut res = Resolution::default();

    for key in &manifest.rules {
        let Some(node) = sources.get(key) else {
            diags.error(
                file,
                format!(
                    "`rules` names {key}, which does not exist in {}",
                    describe(&key.source)
                ),
            );
            continue;
        };
        match node.kind {
            Kind::Rule => {
                res.enabled.entry(key.clone()).or_default().insert(Reason::Direct);
            }
            Kind::Domain => diags.error(
                file,
                format!("`rules` names domain {}, which cannot be enabled on its own; to enable every rule under it, move it to `packs`", label(sources, key)),
            ),
            Kind::OneOf => diags.error(
                file,
                format!(
                    "`rules` names one-of {}; a one-of is enabled by choosing one of its children: {}",
                    label(sources, key),
                    children_list(sources, key)
                ),
            ),
        }
    }

    for root in &manifest.packs {
        let Some(node) = sources.get(root) else {
            diags.error(
                file,
                format!(
                    "`packs` names {root}, which does not exist in {}",
                    describe(&root.source)
                ),
            );
            continue;
        };
        if matches!(node.status, Status::Superseded | Status::Repealed) {
            let next = successors(root, node);
            let what = match next.as_slice() {
                [] => "repealed; take it out of `packs`".to_owned(),
                [next] => format!(
                    "superseded by {}; replace {root} with {next} in `packs` (`statute upgrade --replace-superseded` does it)",
                    label(sources, next),
                ),
                many => format!(
                    "superseded by {}; replace {root} in `packs` with those of {} that apply",
                    labels(sources, many),
                    refs(many)
                ),
            };
            diags.warning(
                file,
                format!(
                    "pack {} enables nothing: its root is {what}",
                    label(sources, root)
                ),
            );
            continue;
        }
        let mut stack = vec![root.clone()];
        while let Some(key) = stack.pop() {
            let Some(node) = sources.get(&key) else {
                continue;
            };
            if matches!(node.status, Status::Superseded | Status::Repealed) {
                continue;
            }
            if node.kind == Kind::Rule {
                res.enabled
                    .entry(key.clone())
                    .or_default()
                    .insert(Reason::Pack(root.clone()));
            }
            // Into a one-of, only through the child the rule set chose.
            stack.extend(
                sources
                    .children(&key)
                    .into_iter()
                    .filter(|c| node.kind != Kind::OneOf || manifest.rules.contains(c)),
            );
        }
    }

    // Add what the enabled rules require, until nothing more is added.
    let mut one_of_requirements: Vec<(Key, Key)> = Vec::new();
    let mut queue: Vec<Key> = res.enabled.keys().cloned().collect();
    let mut done = BTreeSet::new();
    while let Some(key) = queue.pop() {
        if !done.insert(key.clone()) {
            continue;
        }
        let mut add = |target: Key, reason: Reason, queue: &mut Vec<Key>| {
            res.enabled
                .entry(target.clone())
                .or_default()
                .insert(reason);
            queue.push(target);
        };
        if let Some(parent) = parent_rule(sources, &key) {
            add(parent, Reason::ParentOf(key.clone()), &mut queue);
        }
        for target in sources.requires(&key) {
            match sources.get(&target).map(|n| n.kind) {
                Some(Kind::Rule) => add(target, Reason::RequiredBy(key.clone()), &mut queue),
                Some(Kind::OneOf) => one_of_requirements.push((key.clone(), target)),
                _ => {}
            }
        }
    }

    for (rule, one_of) in one_of_requirements {
        if !is_enabled(sources, &res, &one_of) {
            diags.error(
                file,
                format!(
                    "rule {} requires one-of {}, and the rule set enables none of its children; add one of them to `rules`: {}",
                    label(sources, &rule),
                    label(sources, &one_of),
                    children_list(sources, &one_of)
                ),
            );
        }
    }

    for (one_of, _) in sources.nodes().filter(|(_, n)| n.kind == Kind::OneOf) {
        let chosen: Vec<Key> = sources
            .children(&one_of)
            .into_iter()
            .filter(|c| is_enabled(sources, &res, c))
            .collect();
        if chosen.len() > 1 {
            let list: Vec<String> = chosen
                .iter()
                .map(|c| format!("{} ({})", label(sources, c), how(&res, c)))
                .collect();
            diags.error(
                file,
                format!(
                    "at most one child of one-of {} may be enabled, and the rule set enables {}: {}; keep one",
                    label(sources, &one_of),
                    chosen.len(),
                    list.join(", ")
                ),
            );
        }
    }

    let mut conflicting = BTreeSet::new();
    for key in res.enabled.keys() {
        let Some(rule) = sources.get(key).and_then(|n| n.rule.as_ref()) else {
            continue;
        };
        for other in sources.conflicts(key) {
            if res.enabled.contains_key(&other) {
                let pair = if *key < other {
                    (key.clone(), other.clone())
                } else {
                    (other.clone(), key.clone())
                };
                if !conflicting.insert(pair) {
                    continue;
                }
                // Two rules of one pack cannot be taken out one at a time.
                let shared_pack = res.enabled[key].iter().find_map(|r| match r {
                    Reason::Pack(p) if res.enabled[&other].contains(r) => Some(p.clone()),
                    _ => None,
                });
                let advice = match shared_pack {
                    Some(p) => format!(
                        "both are in pack {p}, so in place of the pack, list in `rules` the rules of it you want"
                    ),
                    None => "take one of them out of the rule set, or the rule that brings it in"
                        .to_owned(),
                };
                diags.error(
                    file,
                    format!(
                        "rules {} ({}) and {} ({}) conflict ({key} says so); {advice}",
                        label(sources, key),
                        how(&res, key),
                        label(sources, &other),
                        how(&res, &other),
                    ),
                );
            }
        }
        if !rule.constitution.contains(revision) {
            diags.error(
                file,
                format!(
                    "rule {} is compatible with constitution revisions `{}`, and the rule set rests on revision {revision}; take the rule out or move to a revision it accepts",
                    label(sources, key),
                    rule.constitution
                ),
            );
        }
    }

    for (key, reasons) in &res.enabled {
        let Some(node) = sources.get(key) else {
            continue;
        };
        let next = successors(key, node);
        let what = match node.status {
            Status::Superseded => format!("superseded by {}", labels(sources, &next)),
            Status::Repealed => "repealed, with nothing in its place".to_owned(),
            _ => continue,
        };
        // A rule listed directly is changed in `rules`; one brought in
        // goes with the rule that brings it in.
        let advice = match (reasons.contains(&Reason::Direct), next.as_slice()) {
            (true, []) => format!("take {key} out of `rules`"),
            (true, [next]) => format!(
                "replace {key} with {next} in `rules` (`statute upgrade --replace-superseded` does it)"
            ),
            (true, many) => format!(
                "replace {key} in `rules` with those of {} that apply",
                refs(many)
            ),
            (false, _) => {
                let by = reasons.iter().find_map(|r| match r {
                    Reason::ParentOf(b) | Reason::RequiredBy(b) => Some(b),
                    _ => None,
                });
                match by {
                    Some(b) => format!(
                        "it stays as long as {} is enabled, which brings it in",
                        label(sources, b)
                    ),
                    None => "it stays as long as what brings it in is enabled".to_owned(),
                }
            }
        };
        diags.warning(
            file,
            format!(
                "rule {} ({}) is {what}; {advice}",
                label(sources, key),
                how(&res, key)
            ),
        );
    }

    res
}

/// A rule is enabled when it is in the resolution; a one-of when one of
/// its children is.
fn is_enabled(sources: &Sources, res: &Resolution, key: &Key) -> bool {
    match sources.get(key).map(|n| n.kind) {
        Some(Kind::Rule) => res.enabled.contains_key(key),
        Some(Kind::OneOf) => sources
            .children(key)
            .iter()
            .any(|c| is_enabled(sources, res, c)),
        _ => false,
    }
}

fn children_list(sources: &Sources, key: &Key) -> String {
    let children: Vec<String> = sources
        .children(key)
        .into_iter()
        .filter_map(|c| Some((sources.get(&c)?, c)))
        .filter(|(n, _)| !matches!(n.status, Status::Superseded | Status::Repealed))
        .map(|(n, c)| format!("\"{}\" ({})", Manifest::entry(&c), title(n)))
        .collect();
    if children.is_empty() {
        "it has none yet".to_owned()
    } else {
        children.join(", ")
    }
}

/// How a rule came in, in a few words: `listed in rules`, `required by
/// R12`, and so on.
pub fn how(res: &Resolution, key: &Key) -> String {
    let Some(reasons) = res.enabled.get(key) else {
        // A one-of child that is itself a one-of.
        return "through a child".to_owned();
    };
    reasons
        .iter()
        .map(|r| match r {
            Reason::Direct => "listed in `rules`".to_owned(),
            Reason::Pack(root) => format!("in pack {root}"),
            Reason::ParentOf(child) => format!("parent of {child}"),
            Reason::RequiredBy(by) => format!("required by {by}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
