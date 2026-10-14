//! Resolving a rule set: which rules it enables, and why each
//! (`docs/design/rule-system.md`, Dependencies, one-ofs and packs, and
//! Tracing rules to the constitution).

use std::collections::{BTreeMap, BTreeSet};

use crate::diag::Diagnostics;
use crate::manifest::{self, Manifest};
use crate::node::{Id, Kind, Node, Ref, Status};
use crate::source::Source;

/// Why a rule is enabled. A rule may have several reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// Listed in `rules`.
    Direct,
    /// Inside the pack with this root.
    Pack(Id),
    /// The parent, or the rule above a one-of, of this enabled rule.
    ParentOf(Id),
    /// Listed under `requires` of this enabled rule.
    RequiredBy(Id),
}

#[derive(Debug, Default)]
pub struct Resolution {
    /// Every enabled rule, by ID, with the reasons it is enabled.
    pub enabled: BTreeMap<Id, BTreeSet<Reason>>,
}

/// The rule a node's parent requirement lands on: the parent when it is a
/// rule, through any one-of to the rule above it, and nothing when the
/// chain ends at a domain.
pub fn parent_rule(source: &Source, node: &Node) -> Option<Id> {
    let mut parent = source.get(node.parent?)?;
    let mut steps = 0;
    while parent.kind == Kind::OneOf && steps <= source.nodes.len() {
        parent = source.get(parent.parent?)?;
        steps += 1;
    }
    (parent.kind == Kind::Rule).then_some(parent.id)
}

fn title(node: &Node) -> &str {
    node.title.get(&node.lang).map_or("", String::as_str)
}

/// `R42 (Sync from the lock file first in CI)`.
pub fn label(source: &Source, id: Id) -> String {
    match source.get(id) {
        Some(n) => format!("{} ({})", source.show(id), title(n)),
        None => source.show(id),
    }
}

/// The labels of `ids`, joined by commas.
pub fn labels(source: &Source, ids: &[Id]) -> String {
    ids.iter()
        .map(|id| label(source, *id))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `ids` as references of `source`, joined by commas.
pub fn refs(source: &Source, ids: &[Id]) -> String {
    ids.iter()
        .map(|id| source.show(*id))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Resolves `manifest` against `source`, which has passed its own checks,
/// for the constitution `revision`. Problems go to `diags`, against
/// `statute.toml`.
pub fn resolve(
    source: &Source,
    manifest: &Manifest,
    revision: u64,
    diags: &mut Diagnostics,
) -> Resolution {
    let file = manifest::FILE;
    let mut res = Resolution::default();
    let name = &manifest.source.name;
    let show = |id: Id| source.show(id);

    for &id in &manifest.rules {
        let Some(node) = source.get(id) else {
            diags.error(
                file,
                format!(
                    "`rules` names {}, which does not exist in source `{name}`",
                    show(id)
                ),
            );
            continue;
        };
        match node.kind {
            Kind::Rule => {
                res.enabled.entry(id).or_default().insert(Reason::Direct);
            }
            Kind::Domain => diags.error(
                file,
                format!("`rules` names domain {}, which cannot be enabled on its own; to enable every rule under it, move it to `packs`", label(source, id)),
            ),
            Kind::OneOf => diags.error(
                file,
                format!(
                    "`rules` names one-of {}; a one-of is enabled by choosing one of its children: {}",
                    label(source, id),
                    children_list(source, id)
                ),
            ),
        }
    }

    for &root in &manifest.packs {
        if source.get(root).is_none() {
            diags.error(
                file,
                format!(
                    "`packs` names {}, which does not exist in source `{name}`",
                    show(root)
                ),
            );
            continue;
        }
        if let Some(node) = source.get(root)
            && matches!(node.status, Status::Superseded | Status::Repealed)
        {
            let what = match node.superseded_by.as_slice() {
                [] => "repealed; take it out of `packs`".to_owned(),
                [next] => format!(
                    "superseded by {}; replace {} with {} in `packs` (`statute upgrade --replace-superseded` does it)",
                    label(source, *next),
                    show(root),
                    show(*next)
                ),
                many => format!(
                    "superseded by {}; replace {} in `packs` with those of {} that apply",
                    labels(source, many),
                    show(root),
                    refs(source, many)
                ),
            };
            diags.warning(
                file,
                format!(
                    "pack {} enables nothing: its root is {what}",
                    label(source, root)
                ),
            );
            continue;
        }
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            let Some(node) = source.get(id) else { continue };
            if matches!(node.status, Status::Superseded | Status::Repealed) {
                continue;
            }
            if node.kind == Kind::Rule {
                res.enabled
                    .entry(id)
                    .or_default()
                    .insert(Reason::Pack(root));
            }
            // Into a one-of, only through the child the rule set chose.
            stack.extend(
                source
                    .children(id)
                    .filter(|c| node.kind != Kind::OneOf || manifest.rules.contains(&c.id))
                    .map(|c| c.id),
            );
        }
    }

    // Add what the enabled rules require, until nothing more is added.
    let mut one_of_requirements: Vec<(Id, Id)> = Vec::new();
    let mut queue: Vec<Id> = res.enabled.keys().copied().collect();
    let mut done = BTreeSet::new();
    while let Some(id) = queue.pop() {
        if !done.insert(id) {
            continue;
        }
        let Some(node) = source.get(id) else { continue };
        let mut add = |target: Id, reason: Reason, queue: &mut Vec<Id>| {
            res.enabled.entry(target).or_default().insert(reason);
            queue.push(target);
        };
        if let Some(parent) = parent_rule(source, node) {
            add(parent, Reason::ParentOf(id), &mut queue);
        }
        let Some(rule) = &node.rule else { continue };
        for r in &rule.requires {
            let Ref::Local(target) = *r else { continue };
            match source.get(target).map(|n| n.kind) {
                Some(Kind::Rule) => add(target, Reason::RequiredBy(id), &mut queue),
                Some(Kind::OneOf) => one_of_requirements.push((id, target)),
                _ => {}
            }
        }
    }

    for (rule, one_of) in one_of_requirements {
        if !is_enabled(source, &res, one_of) {
            diags.error(
                file,
                format!(
                    "rule {} requires one-of {}, and the rule set enables none of its children; add one of them to `rules`: {}",
                    label(source, rule),
                    label(source, one_of),
                    children_list(source, one_of)
                ),
            );
        }
    }

    for one_of in source.nodes.values().filter(|n| n.kind == Kind::OneOf) {
        let chosen: Vec<Id> = source
            .children(one_of.id)
            .filter(|c| is_enabled(source, &res, c.id))
            .map(|c| c.id)
            .collect();
        if chosen.len() > 1 {
            let list: Vec<String> = chosen
                .iter()
                .map(|&c| format!("{} ({})", label(source, c), how(source, &res, c)))
                .collect();
            diags.error(
                file,
                format!(
                    "at most one child of one-of {} may be enabled, and the rule set enables {}: {}; keep one",
                    label(source, one_of.id),
                    chosen.len(),
                    list.join(", ")
                ),
            );
        }
    }

    let mut conflicting = BTreeSet::new();
    for &id in res.enabled.keys() {
        let Some(rule) = source.get(id).and_then(|n| n.rule.as_ref()) else {
            continue;
        };
        for r in &rule.conflicts {
            let Ref::Local(other) = *r else { continue };
            if res.enabled.contains_key(&other)
                && conflicting.insert((id.min(other), id.max(other)))
            {
                // Two rules of one pack cannot be taken out one at a time.
                let shared_pack = res.enabled[&id].iter().find_map(|r| match r {
                    Reason::Pack(p) if res.enabled[&other].contains(r) => Some(*p),
                    _ => None,
                });
                let advice = match shared_pack {
                    Some(p) => format!(
                        "both are in pack {}, so in place of the pack, list in `rules` the rules of it you want",
                        show(p)
                    ),
                    None => "take one of them out of the rule set, or the rule that brings it in"
                        .to_owned(),
                };
                diags.error(
                    file,
                    format!(
                        "rules {} ({}) and {} ({}) conflict ({} says so); {advice}",
                        label(source, id),
                        how(source, &res, id),
                        label(source, other),
                        how(source, &res, other),
                        show(id)
                    ),
                );
            }
        }
        if !rule.constitution.contains(revision) {
            diags.error(
                file,
                format!(
                    "rule {} is compatible with constitution revisions `{}`, and the rule set rests on revision {revision}; take the rule out or move to a revision it accepts",
                    label(source, id),
                    rule.constitution
                ),
            );
        }
    }

    for (&id, reasons) in &res.enabled {
        let Some(node) = source.get(id) else { continue };
        let what = match node.status {
            Status::Superseded => format!("superseded by {}", labels(source, &node.superseded_by)),
            Status::Repealed => "repealed, with nothing in its place".to_owned(),
            _ => continue,
        };
        // A rule listed directly is changed in `rules`; one brought in
        // goes with the rule that brings it in.
        let advice = match (
            reasons.contains(&Reason::Direct),
            node.superseded_by.as_slice(),
        ) {
            (true, []) => format!("take {} out of `rules`", show(id)),
            (true, [next]) => format!(
                "replace {} with {} in `rules` (`statute upgrade --replace-superseded` does it)",
                show(id),
                show(*next)
            ),
            (true, many) => format!(
                "replace {} in `rules` with those of {} that apply",
                show(id),
                refs(source, many)
            ),
            (false, _) => {
                let by = reasons.iter().find_map(|r| match r {
                    Reason::ParentOf(b) | Reason::RequiredBy(b) => Some(*b),
                    _ => None,
                });
                match by {
                    Some(b) => format!(
                        "it stays as long as {} is enabled, which brings it in",
                        label(source, b)
                    ),
                    None => "it stays as long as what brings it in is enabled".to_owned(),
                }
            }
        };
        diags.warning(
            file,
            format!(
                "rule {} ({}) is {what}; {advice}",
                label(source, id),
                how(source, &res, id)
            ),
        );
    }

    res
}

/// A rule is enabled when it is in the resolution; a one-of when one of
/// its children is.
fn is_enabled(source: &Source, res: &Resolution, id: Id) -> bool {
    match source.get(id).map(|n| n.kind) {
        Some(Kind::Rule) => res.enabled.contains_key(&id),
        Some(Kind::OneOf) => source.children(id).any(|c| is_enabled(source, res, c.id)),
        _ => false,
    }
}

fn children_list(source: &Source, id: Id) -> String {
    let children: Vec<String> = source
        .children(id)
        .filter(|c| !matches!(c.status, Status::Superseded | Status::Repealed))
        .map(|c| format!("\"{}\" ({})", source.show(c.id), title(c)))
        .collect();
    if children.is_empty() {
        "it has none yet".to_owned()
    } else {
        children.join(", ")
    }
}

/// How a rule came in, in a few words: `listed in rules`, `required by
/// 12`, and so on.
pub fn how(source: &Source, res: &Resolution, id: Id) -> String {
    let Some(reasons) = res.enabled.get(&id) else {
        // A one-of child that is itself a one-of.
        return "through a child".to_owned();
    };
    reasons
        .iter()
        .map(|r| match r {
            Reason::Direct => "listed in `rules`".to_owned(),
            Reason::Pack(root) => format!("in pack {}", source.show(*root)),
            Reason::ParentOf(child) => format!("parent of {}", source.show(*child)),
            Reason::RequiredBy(by) => format!("required by {}", source.show(*by)),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
