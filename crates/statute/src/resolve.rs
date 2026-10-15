//! Resolving a rule set: which rules it enables, and why each
//! (`docs/design/rule-sets.md`, Dependencies, one-ofs and packs, and
//! `docs/design/rule-format.md`, Tracing rules to the constitution).

use std::collections::{BTreeMap, BTreeSet};

use crate::diag::Diagnostics;
use crate::manifest::{self, Declared, FIRST_PARTY, Key, LOCAL, Manifest};
use crate::node::{Kind, Node, Status, Switch};
use crate::source::{Sources, describe};

/// Why a rule is enabled. A rule may have several reasons.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// Listed in `rules`.
    Direct,
    /// Inside the pack with this root.
    Pack(Key),
    /// On by default, in a domain that is on.
    Default,
    /// The default child of this one-of, which needs a child and has no
    /// other enabled.
    DefaultOf(Key),
    /// Listed under `requires` of this enabled rule.
    RequiredBy(Key),
    /// Overridden by this enabled rule, which stands in its place.
    OverriddenBy(Key),
}

#[derive(Debug, Default)]
pub struct Resolution {
    /// Every enabled rule, by key, with the reasons it is enabled. An
    /// overridden rule counts as enabled.
    pub enabled: BTreeMap<Key, BTreeSet<Reason>>,
    /// Each overridden rule and the enabled rule that overrides it.
    pub overridden: BTreeMap<Key, Key>,
    /// The domains and one-ofs that are on.
    pub on: BTreeSet<Key>,
}

impl Resolution {
    /// The enabled rules the rule set chose: those `rules` or `packs`
    /// names, and what a chosen rule requires or overrides, down the
    /// chain. Every other enabled rule came in by the defaults alone.
    pub fn chosen(&self) -> BTreeSet<Key> {
        let mut chosen: BTreeSet<Key> = BTreeSet::new();
        loop {
            let before = chosen.len();
            for (key, reasons) in &self.enabled {
                if reasons.iter().any(|r| match r {
                    Reason::Direct | Reason::Pack(_) => true,
                    Reason::RequiredBy(by) | Reason::OverriddenBy(by) => chosen.contains(by),
                    Reason::Default | Reason::DefaultOf(_) => false,
                }) {
                    chosen.insert(key.clone());
                }
            }
            if chosen.len() == before {
                return chosen;
            }
        }
    }

    /// The rule that stands in the place of `key`, following a chain of
    /// overrides: `key` itself when nothing overrides it.
    pub fn standing(&self, key: &Key) -> Key {
        let mut at = key.clone();
        for _ in 0..3 {
            match self.overridden.get(&at) {
                Some(by) => at = by.clone(),
                None => break,
            }
        }
        at
    }

    /// The rules a rule overrides, down a chain: for `L3` that overrides
    /// `team:7`, which overrides `R200`, those two.
    pub fn chain(&self, key: &Key) -> Vec<Key> {
        let mut out = Vec::new();
        let mut at = key.clone();
        while let Some((target, _)) = self.overridden.iter().find(|(_, by)| **by == at) {
            if out.contains(target) {
                break;
            }
            out.push(target.clone());
            at = target.clone();
        }
        out
    }

    /// Whether a rule is enabled and not overridden: its text is compiled
    /// and its check runs.
    pub fn in_force(&self, key: &Key) -> bool {
        self.enabled.contains_key(key) && !self.overridden.contains_key(key)
    }
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

/// What takes the place of `key` now: nothing when it is not superseded,
/// else the rules that are neither superseded nor repealed which
/// `superseded-by` leads to from it, through any number of superseded
/// rules; nothing when all it leads to are repealed, and its own
/// successors when it leads to no rule that is not superseded (a cycle).
pub fn current_successors(sources: &Sources, key: &Key) -> Vec<Key> {
    fn walk(sources: &Sources, at: &Key, seen: &mut Vec<Key>, out: &mut Vec<Key>) {
        let Some(node) = sources.get(at) else {
            return;
        };
        for next in successors(at, node) {
            if seen.contains(&next) {
                continue;
            }
            seen.push(next.clone());
            if sources
                .get(&next)
                .is_some_and(|n| n.status == Status::Superseded)
            {
                walk(sources, &next, seen, out);
            } else {
                out.push(next);
            }
        }
    }
    let Some(node) = sources.get(key) else {
        return Vec::new();
    };
    if node.status != Status::Superseded {
        return Vec::new();
    }
    let mut out = Vec::new();
    walk(sources, key, &mut vec![key.clone()], &mut out);
    if out.is_empty() {
        return successors(key, node);
    }
    out.retain(|k| sources.get(k).is_none_or(|n| n.status != Status::Repealed));
    out
}

/// The default switches as a rule set sees them
/// (`docs/design/rule-config.md`, Default switches): which domains and
/// one-ofs are on, and what `exclude` turns off.
pub struct Switches<'a> {
    sources: &'a Sources,
    manifest: &'a Manifest,
    /// The domains and one-ofs that are on.
    pub active: BTreeSet<Key>,
}

impl<'a> Switches<'a> {
    pub fn new(sources: &'a Sources, manifest: &'a Manifest) -> Switches<'a> {
        let mut sw = Switches {
            sources,
            manifest,
            active: BTreeSet::new(),
        };
        let groups: Vec<Key> = sources
            .nodes()
            .filter(|(_, n)| matches!(n.kind, Kind::Domain | Kind::OneOf))
            .map(|(k, _)| k)
            .collect();
        sw.active = groups.into_iter().filter(|k| sw.is_on(k, 0)).collect();
        sw
    }

    /// Whether the rule set takes the defaults of `source`: those of `R`
    /// and of this repository's own rules, and of each source its own
    /// `[sources]` names, unless the entry says `defaults = false`.
    pub fn takes_defaults(&self, source: &str) -> bool {
        match self.manifest.spec(source) {
            _ if source == LOCAL => true,
            Some(spec) if source == FIRST_PARTY => spec.defaults,
            Some(spec) => spec.declared == Declared::RuleSet && spec.defaults,
            None => false,
        }
    }

    /// The node and those above it.
    pub fn above(&self, key: &Key) -> Vec<Key> {
        let mut out = vec![key.clone()];
        let mut at = key.clone();
        while let Some(parent) = self.sources.parent(&at) {
            if out.contains(&parent) || self.sources.get(&parent).is_none() {
                break;
            }
            out.push(parent.clone());
            at = parent;
        }
        out
    }

    /// Whether `exclude` turns off the node, itself or a node above it.
    pub fn excluded(&self, key: &Key) -> bool {
        self.above(key)
            .iter()
            .any(|k| self.manifest.exclude.contains(k))
    }

    /// Whether a node's own `default` counts: its source's defaults are
    /// taken, or `domains` lists it or a domain above it.
    fn counts(&self, key: &Key) -> bool {
        self.takes_defaults(&key.source)
            || self
                .above(key)
                .iter()
                .any(|k| self.manifest.domains.contains(k))
    }

    /// Whether a domain or a rule is on by its own `default`.
    pub fn switched_on(&self, key: &Key) -> bool {
        self.sources
            .get(key)
            .is_some_and(|n| n.default == Some(Switch::On))
    }

    /// The child a one-of has chosen: the one the rule set lists in
    /// `rules` or `domains`, or the one a rule it lists overrides; else
    /// its default.
    pub fn chosen(&self, one_of: &Key) -> Option<Key> {
        let m = self.manifest;
        let children = self.sources.children(one_of);
        children
            .iter()
            .find(|c| {
                m.domains.contains(c)
                    || m.rules
                        .iter()
                        .any(|r| r == *c || self.sources.place(r) == **c)
            })
            .cloned()
            .or_else(|| self.default_child(one_of))
    }

    /// A one-of's default child, when it names one that is a child.
    pub fn default_child(&self, one_of: &Key) -> Option<Key> {
        let id = self.sources.get(one_of)?.default_child?;
        let child = Key::new(&one_of.source, id);
        self.sources
            .children(one_of)
            .contains(&child)
            .then_some(child)
    }

    /// A one-of's default child when it is a rule in force.
    fn default_rule_child(&self, one_of: &Key) -> Option<Key> {
        let child = self.default_child(one_of)?;
        self.sources
            .get(&child)
            .is_some_and(|n| {
                n.kind == Kind::Rule && !matches!(n.status, Status::Superseded | Status::Repealed)
            })
            .then_some(child)
    }

    fn is_on(&self, key: &Key, depth: usize) -> bool {
        let Some(node) = self.sources.get(key) else {
            return false;
        };
        if depth > 8 || self.excluded(key) {
            return false;
        }
        let parent = self.sources.parent(key);
        let parent_kind = parent
            .as_ref()
            .and_then(|p| self.sources.get(p))
            .map(|n| n.kind);
        match node.kind {
            Kind::Domain if self.manifest.domains.contains(key) => true,
            // An option of a one-of has no `default` of its own: it is on
            // when its one-of is and has chosen it.
            Kind::Domain => match (&parent, parent_kind) {
                (Some(p), Some(Kind::OneOf)) => {
                    self.is_on(p, depth + 1) && self.chosen(p).as_ref() == Some(key)
                }
                (None, _) => self.switched_on(key) && self.counts(key),
                (Some(p), Some(Kind::Domain)) => {
                    self.is_on(p, depth + 1) && self.switched_on(key) && self.counts(key)
                }
                _ => false,
            },
            Kind::OneOf => parent.is_some_and(|p| self.is_on(&p, depth + 1)),
            Kind::Rule => false,
        }
    }

    /// Whether the defaults enable a rule that is no child of a one-of:
    /// it is on, in a domain that is on, and not turned off.
    pub fn default_rule(&self, key: &Key) -> bool {
        let Some(node) = self.sources.get(key) else {
            return false;
        };
        node.kind == Kind::Rule
            && !matches!(node.status, Status::Superseded | Status::Repealed)
            && node.overrides.is_none()
            && self.sources.parent(key).is_some_and(|p| {
                self.sources.get(&p).is_some_and(|n| n.kind == Kind::Domain)
                    && self.active.contains(&p)
            })
            && self.switched_on(key)
            && self.counts(key)
            && !self.excluded(key)
    }

    /// Whether the defaults alone enable a rule, as a rule that is on in a
    /// domain that is on, or as the default child of a one-of that is on.
    pub fn enables(&self, key: &Key) -> bool {
        self.default_rule(key)
            || self.sources.parent(key).is_some_and(|p| {
                self.active.contains(&p)
                    && self.sources.get(&p).is_some_and(|n| n.kind == Kind::OneOf)
                    && self.default_rule_child(&p).as_ref() == Some(key)
            })
    }

    /// Whether a domain is on without `domains` listing it.
    pub fn on_by_default(&self, key: &Key) -> bool {
        let without = Manifest {
            domains: self
                .manifest
                .domains
                .iter()
                .filter(|k| *k != key)
                .cloned()
                .collect(),
            ..self.manifest.clone()
        };
        Switches::new(self.sources, &without).active.contains(key)
    }
}

/// Resolves `manifest` against `sources`, which have passed their own
/// checks, for the constitution `revision`. Problems go to `diags`,
/// against `statute.toml`. An overridden rule counts as enabled, and the
/// rule that overrides it brings in what it would have
/// (`docs/design/rule-sources.md`, Overrides).
pub fn resolve(
    sources: &Sources,
    manifest: &Manifest,
    revision: u64,
    diags: &mut Diagnostics,
) -> Resolution {
    let file = manifest::FILE;
    let mut res = Resolution::default();
    let sw = Switches::new(sources, manifest);

    for (field, list) in [
        ("domains", &manifest.domains),
        ("exclude", &manifest.exclude),
    ] {
        for key in list {
            match sources.get(key) {
                None => diags.error(
                    file,
                    format!(
                        "`{field}` names {key}, which does not exist in {}",
                        describe(&key.source)
                    ),
                ),
                Some(n) if field == "domains" && n.kind != Kind::Domain => diags.error(
                    file,
                    format!(
                        "`domains` names {} {}; it lists domains, and a rule is turned on in `rules`",
                        n.kind.name(),
                        label(sources, key)
                    ),
                ),
                Some(_) if field == "domains" && sw.on_by_default(key) => diags.warning(
                    file,
                    format!(
                        "`domains` lists {}, which is on by default; take it out",
                        label(sources, key)
                    ),
                ),
                // An option is chosen, not turned off: another option, or
                // none, is what the rule set means.
                Some(_) if field == "exclude" => {
                    if let Some(one_of) = sources
                        .parent(key)
                        .filter(|p| sources.get(p).is_some_and(|n| n.kind == Kind::OneOf))
                    {
                        diags.error(
                            file,
                            format!(
                                "`exclude` names {key}, an option of the one-of {one_of}; list another option in `rules`, or exclude {one_of}"
                            ),
                        );
                    }
                }
                Some(_) => {}
            }
        }
    }

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
                if manifest.exclude.contains(key) {
                    diags.error(
                        file,
                        format!(
                            "`rules` lists {} and `exclude` turns it off; keep one",
                            label(sources, key)
                        ),
                    );
                } else if sw.enables(key) {
                    diags.warning(
                        file,
                        format!(
                            "`rules` lists {}, which the defaults enable; take it out",
                            label(sources, key)
                        ),
                    );
                }
            }
            Kind::Domain => diags.error(
                file,
                format!("`rules` names domain {}; to turn it on, move it to `domains`, or to enable every rule under it, to `packs`", label(sources, key)),
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

    for (key, _) in sources.nodes() {
        if sw.default_rule(&key) {
            res.enabled.entry(key).or_default().insert(Reason::Default);
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
        if manifest.exclude.contains(root) {
            diags.error(
                file,
                format!(
                    "`packs` lists {} and `exclude` turns it off; keep one",
                    label(sources, root)
                ),
            );
            continue;
        }
        if matches!(node.status, Status::Superseded | Status::Repealed) {
            let what = match current_successors(sources, root).as_slice() {
                [] if node.status == Status::Superseded => format!(
                    "superseded by {}, and nothing takes its place: what it leads to is repealed; take it out of `packs`",
                    labels(sources, &successors(root, node)),
                ),
                [] => "repealed; take it out of `packs`".to_owned(),
                [next] => format!(
                    "superseded by {}; replace {root} with {next} in `packs` (`statute upgrade --replace-superseded` does it)",
                    labels(sources, &successors(root, node)),
                ),
                many => format!(
                    "superseded by {}; replace {root} in `packs` with {}, which together take its place, leaving out any that does not apply",
                    labels(sources, &successors(root, node)),
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
            // Within the pack's source only, and not into what `exclude`
            // turns off; into a one-of, only through the child the rule set
            // chose, or the child a rule it chose overrides, or else its
            // default.
            let chosen = (node.kind == Kind::OneOf)
                .then(|| sw.chosen(&key))
                .flatten();
            stack.extend(sources.children(&key).into_iter().filter(|c| {
                c.source == root.source
                    && !sw.excluded(c)
                    && (node.kind != Kind::OneOf || chosen.as_ref() == Some(c))
            }));
        }
    }

    // Add what the enabled rules require, until nothing more is added;
    // then the default child of each one-of that needs a child and has
    // none, and again what that brings in.
    let mut chosen = res.enabled.clone();
    let mut one_of_requirements: Vec<(Key, Key)>;
    let mut unsettled;
    loop {
        let closed = close(sources, &chosen);
        res.enabled = closed.enabled;
        res.overridden = closed.overridden;
        one_of_requirements = closed.one_of_requirements;
        unsettled = closed.unsettled;
        let needing = sw
            .active
            .iter()
            .filter(|k| sources.get(k).is_some_and(|n| n.kind == Kind::OneOf))
            .cloned()
            .chain(one_of_requirements.iter().map(|(_, o)| o.clone()))
            .collect::<BTreeSet<Key>>();
        let mut added = false;
        for one_of in needing {
            if sw.excluded(&one_of) || is_enabled(sources, &res, &one_of) {
                continue;
            }
            if let Some(child) = sw.default_rule_child(&one_of) {
                chosen
                    .entry(child)
                    .or_default()
                    .insert(Reason::DefaultOf(one_of));
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    res.on = sw.active.clone();
    if let Some(unsettled) = unsettled {
        diags.error(
            file,
            format!(
                "the overrides do not settle: {} come in only through rules that are overridden once they are in; list them in `rules`",
                unsettled
                    .iter()
                    .map(|k| label(sources, k))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }

    for (target, reasons) in &res.enabled {
        let by: Vec<&Key> = reasons
            .iter()
            .filter_map(|r| match r {
                Reason::OverriddenBy(by) => Some(by),
                _ => None,
            })
            .collect();
        match by.as_slice() {
            [] => {}
            [by] if reasons.len() == 1 => diags.warning(
                file,
                format!(
                    "rule {} overrides {}, which nothing else in the rule set enables; {by} is compiled where {target} would stand",
                    label(sources, by),
                    label(sources, target)
                ),
            ),
            [_] => {}
            many => diags.error(
                file,
                format!(
                    "rule {} is overridden by {}; one rule at most may override it, so keep one",
                    label(sources, target),
                    many.iter()
                        .map(|k| format!("{} ({})", label(sources, k), how(&res, k)))
                        .collect::<Vec<_>>()
                        .join(" and ")
                ),
            ),
        }
    }

    for (rule, one_of) in &one_of_requirements {
        if sw.excluded(one_of) && !is_enabled(sources, &res, one_of) {
            diags.error(
                file,
                format!(
                    "one-of {} cannot be turned off: rule {} requires it; take it out of `exclude`",
                    label(sources, one_of),
                    label(sources, rule)
                ),
            );
        }
    }
    for (key, reasons) in &res.enabled {
        if !sw.excluded(key) || reasons.contains(&Reason::Direct) {
            continue;
        }
        for by in reasons.iter().filter_map(|r| match r {
            Reason::RequiredBy(by) => Some(by),
            _ => None,
        }) {
            diags.error(
                file,
                format!(
                    "rule {} cannot be turned off: rule {} requires it; take it out of `exclude`, or turn off {by} too",
                    label(sources, key),
                    label(sources, by)
                ),
            );
        }
    }

    for (rule, one_of) in one_of_requirements {
        if !is_enabled(sources, &res, &one_of) && !sw.excluded(&one_of) {
            diags.error(
                file,
                format!(
                    "rule {} requires one-of {}, whose default brings in no rule in force, and the rule set enables none of its children; add one of them to `rules`: {}",
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

    // An overridden rule's conflicts, and those on it, do not apply, and
    // its compatibility does not matter: it is not compiled.
    let mut conflicting = BTreeSet::new();
    for key in res.enabled.keys().filter(|k| res.in_force(k)) {
        let Some(rule) = sources.get(key).and_then(|n| n.rule.as_ref()) else {
            continue;
        };
        for other in sources.conflicts(key) {
            if res.in_force(&other) {
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
        let next = current_successors(sources, key);
        let what = match node.status {
            Status::Superseded if next.is_empty() => format!(
                "superseded by {}, and nothing takes its place: what it leads to is repealed",
                labels(sources, &successors(key, node))
            ),
            Status::Superseded => {
                format!("superseded by {}", labels(sources, &successors(key, node)))
            }
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
                "replace {key} in `rules` with {}, which together take its place, leaving out any that does not apply",
                refs(many)
            ),
            (false, _) => {
                let by = reasons.iter().find_map(|r| match r {
                    Reason::RequiredBy(b) | Reason::OverriddenBy(b) => Some(b),
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

/// What [`close`] gives: the rules enabled, the overridden ones, each
/// requirement of a one-of, and the rules that keep the overrides from
/// settling, if they do not.
struct Closed {
    enabled: BTreeMap<Key, BTreeSet<Reason>>,
    overridden: BTreeMap<Key, Key>,
    one_of_requirements: Vec<(Key, Key)>,
    unsettled: Option<BTreeSet<Key>>,
}

/// Adds to `chosen` what its rules require, until nothing more is added.
/// An overridden rule brings in nothing but the rule it overrides in its
/// turn; which rules are overridden is known only once the rules that
/// override them are in, so the closure runs again until that settles.
fn close(sources: &Sources, chosen: &BTreeMap<Key, BTreeSet<Reason>>) -> Closed {
    let mut enabled: BTreeMap<Key, BTreeSet<Reason>>;
    let mut one_of_requirements: Vec<(Key, Key)>;
    let mut overridden: BTreeMap<Key, Key> = BTreeMap::new();
    // The sets of overridden rules of the rounds before, to tell a closure
    // that goes round from one still settling.
    let mut seen: Vec<BTreeMap<Key, Key>> = Vec::new();
    loop {
        enabled = chosen.clone();
        one_of_requirements = Vec::new();
        let mut queue: Vec<Key> = enabled.keys().cloned().collect();
        let mut done = BTreeSet::new();
        while let Some(key) = queue.pop() {
            if !done.insert(key.clone()) {
                continue;
            }
            let mut add = |target: Key, reason: Reason, queue: &mut Vec<Key>| {
                enabled.entry(target.clone()).or_default().insert(reason);
                queue.push(target);
            };
            if let Some(target) = sources.overrides(&key)
                && sources.get(&target).is_some_and(|n| n.kind == Kind::Rule)
            {
                add(target, Reason::OverriddenBy(key.clone()), &mut queue);
            }
            if overridden.contains_key(&key) {
                continue;
            }
            for target in sources.requires(&key) {
                match sources.get(&target).map(|n| n.kind) {
                    Some(Kind::Rule) => add(target, Reason::RequiredBy(key.clone()), &mut queue),
                    Some(Kind::OneOf) => one_of_requirements.push((key.clone(), target)),
                    _ => {}
                }
            }
        }
        let now: BTreeMap<Key, Key> = enabled
            .iter()
            .flat_map(|(key, reasons)| {
                reasons.iter().filter_map(move |r| match r {
                    Reason::OverriddenBy(by) => Some((key.clone(), by.clone())),
                    _ => None,
                })
            })
            .collect();
        if now == overridden {
            return Closed {
                enabled,
                overridden,
                one_of_requirements,
                unsettled: None,
            };
        }
        if seen.contains(&now) {
            // The closure goes round: a rule that overrides is brought in
            // only through a rule that is overridden while it is in.
            let unsettled: BTreeSet<Key> = now
                .values()
                .chain(overridden.values())
                .filter(|by| {
                    now.values().any(|b| b == *by) != overridden.values().any(|b| b == *by)
                })
                .cloned()
                .collect();
            return Closed {
                enabled,
                overridden,
                one_of_requirements,
                unsettled: Some(unsettled),
            };
        }
        seen.push(overridden);
        overridden = now;
    }
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
            Reason::Default => "on by default".to_owned(),
            Reason::DefaultOf(one_of) => format!("the default of one-of {one_of}"),
            Reason::RequiredBy(by) => format!("required by {by}"),
            Reason::OverriddenBy(by) => format!("overridden by {by}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
