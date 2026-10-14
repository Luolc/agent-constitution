//! A source: the nodes of one `rules/` directory, and the checks on how
//! they relate (`docs/design/rule-sets.md`, Dependencies, one-ofs and
//! packs).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use crate::cite::{self, Citation};
use crate::diag::Diagnostics;
use crate::manifest::{FIRST_PARTY, Key, LOCAL};
use crate::node::{self, Id, Kind, Node, Ref, Status, show_ref};

#[derive(Debug, Default)]
pub struct Source {
    pub nodes: BTreeMap<Id, Node>,
    /// The source's name in the rule set, which its rules are shown with
    /// (`manifest::show`); empty when the directory is read alone.
    pub name: String,
    /// How its `rules/` directory is named in messages: `rules`,
    /// `../team/rules`.
    pub dir: String,
    /// The directory that holds its `rules/` and `skills/`, relative to
    /// that of `statute.toml`: `.`, `../team`.
    pub path: String,
    /// Whether anyone may read it, as the rule set declares it.
    pub public: bool,
    /// Whether its rules are the repository's own: `L`, or a source with
    /// `scope = "local"`.
    pub local: bool,
}

impl Source {
    /// Reads every `*.toml` file of `dir` and checks them. `label` is how
    /// the directory is named in messages (`rules`). The source is returned
    /// even when there are errors; callers look at `Diagnostics`.
    pub fn load(dir: &Path, label: &str) -> (Source, Diagnostics) {
        let mut files = Vec::new();
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                let mut diags = Diagnostics::default();
                diags.error(label, format!("cannot read the directory: {e}"));
                return (Source::default(), diags);
            }
        };
        let mut diags = Diagnostics::default();
        for entry in entries {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(e) => {
                    diags.error(label, format!("cannot read the directory: {e}"));
                    continue;
                }
            };
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.ends_with(".toml") {
                continue;
            }
            let file = format!("{label}/{name}");
            match std::fs::read(&path) {
                Ok(bytes) => files.push((file, bytes)),
                Err(e) => diags.error(file, format!("cannot read: {e}")),
            }
        }
        let (mut source, more) = Source::from_files(files);
        source.dir = label.to_owned();
        diags.0.extend(more.0);
        (source, diags)
    }

    /// Checks the given files, each a name such as `rules/42.toml` and its
    /// bytes.
    pub fn from_files(mut files: Vec<(String, Vec<u8>)>) -> (Source, Diagnostics) {
        // By number, not by name; a misnamed file sorts after the rest.
        files.sort_by_key(|(file, _)| {
            let name = file.rsplit('/').next().unwrap_or(file);
            (
                node::id_from_file_name(name).unwrap_or(Id::MAX),
                file.clone(),
            )
        });
        let mut diags = Diagnostics::default();
        let mut source = Source::default();
        let mut files_by_id: HashMap<Id, String> = HashMap::new();
        let mut slugs: HashMap<String, Id> = HashMap::new();
        for (file, bytes) in &files {
            let Some(node) = node::parse(file, bytes, &mut diags) else {
                continue;
            };
            if let Some(first) = files_by_id.get(&node.id) {
                diags.error(file, format!("ID {} is already used by {first}", node.id));
                continue;
            }
            if let Some(slug) = &node.slug {
                if let Some(first) = slugs.get(slug) {
                    diags.error(
                        file,
                        format!(
                            "slug `{slug}` is already used by {}",
                            node::file_name(*first)
                        ),
                    );
                } else {
                    slugs.insert(slug.clone(), node.id);
                }
            }
            files_by_id.insert(node.id, file.clone());
            source.nodes.insert(node.id, node);
        }
        let dir = files
            .first()
            .and_then(|(f, _)| f.rsplit_once('/'))
            .map_or(String::new(), |(d, _)| format!("{d}/"));
        source.dir = dir.trim_end_matches('/').to_owned();
        Checker {
            source: &source,
            dir,
            diags: &mut diags,
        }
        .run();
        (source, diags)
    }

    /// How rule `id` of this source is shown: `R42`, `team:7`, or `42`.
    pub fn show(&self, id: Id) -> String {
        crate::manifest::show(&self.name, id)
    }

    /// The file of node `id`, as messages name it: `rules/42.toml`.
    pub fn file(&self, id: Id) -> String {
        match self.dir.as_str() {
            "" => node::file_name(id),
            dir => format!("{dir}/{}", node::file_name(id)),
        }
    }

    /// The source in a sentence: `source `team``, or this repository's
    /// own rules.
    pub fn described(&self) -> String {
        describe(&self.name)
    }

    pub fn get(&self, id: Id) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// The nodes a node leads to directly: its parent, and for a rule the
    /// nodes of this source it requires. Missing nodes are left out.
    pub fn edges(&self, id: Id) -> Vec<Id> {
        let Some(node) = self.get(id) else {
            return Vec::new();
        };
        let mut out: Vec<Id> = node.parent.into_iter().collect();
        if let Some(rule) = &node.rule {
            out.extend(rule.requires.iter().filter_map(|r| match r {
                Ref::Local(id) => Some(*id),
                Ref::Foreign { .. } => None,
            }));
        }
        out.retain(|id| self.nodes.contains_key(id));
        out
    }

    /// Every node reachable from `starts` along `edges`, the starts
    /// included. A one-of in the result means a child of it must be
    /// enabled; a domain only gives a heading.
    pub fn reach(&self, starts: impl IntoIterator<Item = Id>) -> BTreeSet<Id> {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<Id> = starts.into_iter().collect();
        while let Some(id) = stack.pop() {
            if seen.insert(id) {
                stack.extend(self.edges(id));
            }
        }
        seen
    }

    /// The top of the tree above a node: its domain, or the highest node
    /// that has no parent in this source.
    pub fn root(&self, mut id: Id) -> Id {
        let mut steps = 0;
        while let Some(parent) = self.get(id).and_then(|n| n.parent) {
            if !self.nodes.contains_key(&parent) || steps > self.nodes.len() {
                break;
            }
            id = parent;
            steps += 1;
        }
        id
    }

    pub fn children(&self, id: Id) -> impl Iterator<Item = &Node> {
        self.nodes.values().filter(move |n| n.parent == Some(id))
    }
}

/// A source named `name` in a sentence: `source `team``, or this
/// repository's own rules for `L`.
pub fn describe(name: &str) -> String {
    match name {
        LOCAL => format!("this repository's own rules (`{LOCAL}`)"),
        _ => format!("source `{name}`"),
    }
}

/// The sources of a rule set, read together: a node is found by its
/// source's name and its ID (`Key`). Kept in the order of their keys.
#[derive(Debug, Default)]
pub struct Sources(Vec<Source>);

impl Sources {
    pub fn new(mut list: Vec<Source>) -> Sources {
        list.sort_by_key(|s| Key::new(&s.name, 0));
        Sources(list)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Source> {
        self.0.iter()
    }

    pub fn source(&self, name: &str) -> Option<&Source> {
        self.0.iter().find(|s| s.name == name)
    }

    pub fn get(&self, key: &Key) -> Option<&Node> {
        self.source(&key.source)?.get(key.id)
    }

    /// Every node, in order of key.
    pub fn nodes(&self) -> impl Iterator<Item = (Key, &Node)> {
        self.0
            .iter()
            .flat_map(|s| s.nodes.values().map(|n| (Key::new(&s.name, n.id), n)))
    }

    /// The file of a node, as messages name it.
    pub fn file(&self, key: &Key) -> String {
        match self.source(&key.source) {
            Some(s) => s.file(key.id),
            None => key.to_string(),
        }
    }

    pub fn parent(&self, key: &Key) -> Option<Key> {
        let parent = self.get(key)?.parent?;
        Some(Key::new(&key.source, parent))
    }

    /// The children of a node, in order of key.
    pub fn children(&self, key: &Key) -> Vec<Key> {
        self.source(&key.source)
            .into_iter()
            .flat_map(|s| s.children(key.id).map(|c| Key::new(&s.name, c.id)))
            .collect()
    }

    /// The nodes a rule lists in `requires`.
    pub fn requires(&self, key: &Key) -> Vec<Key> {
        self.refs(key, |r| &r.requires)
    }

    /// The nodes a rule lists in `conflicts`.
    pub fn conflicts(&self, key: &Key) -> Vec<Key> {
        self.refs(key, |r| &r.conflicts)
    }

    fn refs(&self, key: &Key, list: impl Fn(&node::RuleFields) -> &Vec<Ref>) -> Vec<Key> {
        self.get(key)
            .and_then(|n| n.rule.as_ref())
            .map(|r| list(r).iter().map(|r| r.key(&key.source)).collect())
            .unwrap_or_default()
    }

    /// The top of the tree above a node: its domain, or the highest node
    /// above it that its source has. A rule that overrides another stands
    /// in that one's place.
    pub fn root(&self, key: &Key) -> Key {
        let key = self.place(key);
        match self.source(&key.source) {
            Some(s) => Key::new(&s.name, s.root(key.id)),
            None => key,
        }
    }

    /// The rule a rule overrides.
    pub fn overrides(&self, key: &Key) -> Option<Key> {
        Some(self.get(key)?.overrides.as_ref()?.key(&key.source))
    }

    /// Where a node stands in the tree: its own place, or for a rule that
    /// overrides another, through any chain of overrides, the place of
    /// the last rule overridden.
    pub fn place(&self, key: &Key) -> Key {
        let mut at = key.clone();
        // Ranks only go down along a chain, so it has at most three links.
        for _ in 0..3 {
            match self.overrides(&at) {
                Some(next) if self.get(&next).is_some() => at = next,
                _ => break,
            }
        }
        at
    }

    /// The rank of a source for overrides: the first-party source 1, the
    /// other global sources 2, the repository's own rules 3
    /// (`docs/design/rule-sources.md`, Overrides).
    pub fn rank(&self, name: &str) -> u8 {
        if name == FIRST_PARTY {
            1
        } else if name == LOCAL || self.source(name).is_some_and(|s| s.local) {
            3
        } else {
            2
        }
    }

    /// Checks what each source could not check alone: the references to
    /// other sources in `requires`, `conflicts` and `{@rule:...}`, which
    /// must name a node the rule set has, and from a public source a
    /// node of a public one (`docs/design/rule-sources.md`, Which rules
    /// may cite which).
    pub fn check(&self, diags: &mut Diagnostics) {
        for source in &self.0 {
            for node in source.nodes.values() {
                let file = source.file(node.id);
                if let Some(rule) = &node.rule {
                    for (field, list) in
                        [("requires", &rule.requires), ("conflicts", &rule.conflicts)]
                    {
                        for r in list {
                            self.check_ref(
                                source,
                                r,
                                &format!("`{field}` names {}", show_ref(r)),
                                &file,
                                diags,
                                |target, file, diags| {
                                    judge(target.0, field, target.1, true, file, diags)
                                },
                            );
                        }
                    }
                }
                if let Some(r) = &node.overrides {
                    let what = format!("`overrides` names {}", show_ref(r));
                    self.check_ref(
                        source,
                        r,
                        &what,
                        &file,
                        diags,
                        |(target_source, target), file, diags| {
                            self.check_override(source, &what, target_source, target, file, diags)
                        },
                    );
                }
                let mut texts: Vec<&node::Text> = vec![&node.title];
                texts.extend(&node.applicability);
                if let Some(rule) = &node.rule {
                    texts.extend([&rule.statement, &rule.reason, &rule.verification]);
                }
                let mut cited = BTreeSet::new();
                for text in texts.iter().flat_map(|t| t.values()) {
                    let _ = cite::replace(text, |c| {
                        if let Citation::Rule(r) = c
                            && let Some(r) = node::cited_ref(r)
                        {
                            cited.insert(r);
                        }
                        Ok(String::new())
                    });
                }
                for r in &cited {
                    let what = format!("cites `{{@rule:{}}}`", show_ref(r).trim_matches('"'));
                    self.check_ref(source, r, &what, &file, diags, |_, _, _| {});
                }
            }
        }
        self.check_cycles(diags);
        self.check_overrides_brought_in(diags);
    }

    /// The nodes a node leads to: its parent, the parent of its place
    /// for a rule that overrides another, and what it requires.
    fn edges(&self, key: &Key) -> Vec<Key> {
        self.parent(&self.place(key))
            .into_iter()
            .chain(self.requires(key))
            .filter(|k| self.get(k).is_some())
            .collect()
    }

    /// Fails a rule that overrides a rule which brings it in, along
    /// `parent` and `requires`: it would stand in the place of the very
    /// rule that enables it.
    fn check_overrides_brought_in(&self, diags: &mut Diagnostics) {
        for (key, _) in self.nodes() {
            let Some(target) = self.overrides(&key) else {
                continue;
            };
            let mut seen = BTreeSet::new();
            let mut stack = vec![target.clone()];
            while let Some(at) = stack.pop() {
                if seen.insert(at.clone()) {
                    stack.extend(self.edges(&at));
                }
            }
            if seen.contains(&key) {
                diags.error(
                    self.file(&key),
                    format!(
                        "`overrides` names \"{target}\", which brings in {key} along `parent` or `requires`; a rule cannot take the place of a rule it is enabled through"
                    ),
                );
            }
        }
    }

    /// Fails a cycle of requirements that goes through more than one
    /// source, a parent counting as a requirement; each source has found
    /// those within it already.
    fn check_cycles(&self, diags: &mut Diagnostics) {
        let edges = |key: &Key| self.edges(key);
        // 0: not visited, 1: on the path, 2: done.
        let mut state: HashMap<Key, u8> = HashMap::new();
        let mut reported = BTreeSet::new();
        for (start, _) in self.nodes() {
            if state.contains_key(&start) {
                continue;
            }
            let mut stack: Vec<Vec<Key>> = vec![edges(&start)];
            let mut path = vec![start.clone()];
            state.insert(start, 1);
            while let Some(next) = stack.last_mut() {
                let Some(key) = next.pop() else {
                    stack.pop();
                    if let Some(done) = path.pop() {
                        state.insert(done, 2);
                    }
                    continue;
                };
                match state.get(&key) {
                    Some(1) => {
                        let pos = path.iter().position(|p| *p == key).unwrap_or(0);
                        let cycle = &path[pos..];
                        let first = cycle.iter().min().unwrap_or(&key);
                        if cycle.iter().any(|k| k.source != key.source)
                            && reported.insert(first.clone())
                        {
                            let shown: Vec<String> =
                                cycle.iter().chain([&key]).map(Key::to_string).collect();
                            diags.error(
                                self.file(first),
                                format!(
                                    "the requirements form a cycle across sources (a parent counts as a requirement): {}",
                                    shown.join(" -> ")
                                ),
                            );
                        }
                    }
                    Some(_) => {}
                    None => {
                        state.insert(key.clone(), 1);
                        stack.push(edges(&key));
                        path.push(key);
                    }
                }
            }
        }
    }

    /// Checks that a rule of `source` may override `target`, a node of
    /// `target_source`: a rule, of a lower rank.
    fn check_override(
        &self,
        source: &Source,
        what: &str,
        target_source: &Source,
        target: &Node,
        file: &str,
        diags: &mut Diagnostics,
    ) {
        if target.kind != Kind::Rule {
            diags.error(
                file,
                format!(
                    "{what}, a {}; only a rule can be overridden: in place of a one-of's child, choose another child, and propose to its source one it lacks",
                    target.kind.name()
                ),
            );
            return;
        }
        let (from, to) = (self.rank(&source.name), self.rank(&target_source.name));
        if from <= to {
            diags.error(
                file,
                format!(
                    "{what}, a rule of {}, which {} {}; a rule may override only a rule of a lower rank: the first-party source, then the other global sources, then this repository's own rules and the sources with `scope = \"local\"`",
                    target_source.described(),
                    if from == to { "ranks with" } else { "ranks above" },
                    source.described()
                ),
            );
        }
    }

    /// Checks one reference `r` of a node of `source` to another source;
    /// `what` says where it stands, and `more` checks the node it names.
    fn check_ref(
        &self,
        source: &Source,
        r: &Ref,
        what: &str,
        file: &str,
        diags: &mut Diagnostics,
        more: impl FnOnce((&Source, &Node), &str, &mut Diagnostics),
    ) {
        let Ref::Foreign { source: name, id } = r else {
            return;
        };
        if source.name == FIRST_PARTY && name == FIRST_PARTY {
            diags.error(
                file,
                format!(
                    "{what}; in a rule of the first-party source itself, write the number, {id}"
                ),
            );
            return;
        }
        let Some(target_source) = self.source(name) else {
            diags.error(
                file,
                format!("{what}, a rule of source `{name}`, which the rule set does not name in [sources]"),
            );
            return;
        };
        let Some(target) = target_source.get(*id) else {
            diags.error(
                file,
                format!(
                    "{what}, which does not exist in {}",
                    target_source.described()
                ),
            );
            return;
        };
        if source.public && !target_source.public {
            diags.error(
                file,
                format!("{what}, a rule of the private {}; a public rule may name only rules of public sources", target_source.described()),
            );
        }
        more((target_source, target), file, diags);
    }
}

struct Checker<'a> {
    source: &'a Source,
    dir: String,
    diags: &'a mut Diagnostics,
}

impl Checker<'_> {
    fn file(&self, id: Id) -> String {
        format!("{}{}", self.dir, node::file_name(id))
    }

    fn describe(&self, id: Id) -> String {
        match self.source.get(id) {
            Some(n) => format!("{} {id}", n.kind.name()),
            None => id.to_string(),
        }
    }

    fn run(&mut self) {
        self.check_references();
        let tree_ok = self.check_tree_cycles();
        if tree_ok && self.check_requirement_cycles() {
            self.check_redundant_requirements();
            self.check_conflicts();
            self.check_one_ofs();
            self.check_crossing_requirements();
        }
    }

    fn check_references(&mut self) {
        let source = self.source;
        for node in source.nodes.values() {
            let file = self.file(node.id);
            if let Some(parent) = node.parent
                && parent != node.id
                && source.get(parent).is_none()
            {
                self.diags.error(
                    &file,
                    format!("parent {parent} does not exist in this source"),
                );
            }
            for &next in &node.superseded_by {
                match source.get(next) {
                    None => self.diags.error(
                        &file,
                        format!("superseded-by {next} does not exist in this source"),
                    ),
                    Some(n) if n.kind != Kind::Rule => self.diags.error(
                        &file,
                        format!(
                            "superseded-by {next} is a {}; only a rule can supersede a rule",
                            n.kind.name()
                        ),
                    ),
                    _ => {}
                }
            }
            let Some(rule) = &node.rule else { continue };
            for (field, list) in [("requires", &rule.requires), ("conflicts", &rule.conflicts)] {
                // A reference to another source is checked with the rule
                // set that names it (`Sources::check`).
                for &id in list.iter().filter_map(|r| match r {
                    Ref::Local(id) => Some(id),
                    Ref::Foreign { .. } => None,
                }) {
                    match source.get(id) {
                        None => self.diags.error(
                            &file,
                            format!("`{field}` names {id}, which does not exist in this source"),
                        ),
                        Some(target) => judge(source, field, target, false, &file, self.diags),
                    }
                }
            }
        }
    }

    /// Returns whether the parent tree is free of cycles.
    fn check_tree_cycles(&mut self) -> bool {
        let source = self.source;
        let mut reported = BTreeSet::new();
        for &start in source.nodes.keys() {
            let mut path = vec![start];
            let mut id = start;
            while let Some(parent) = source.get(id).and_then(|n| n.parent) {
                if source.get(parent).is_none() {
                    break;
                }
                if let Some(pos) = path.iter().position(|&p| p == parent) {
                    let cycle = &path[pos..];
                    let first = *cycle.iter().min().unwrap_or(&parent);
                    if reported.insert(first) {
                        self.diags.error(
                            self.file(first),
                            format!("the parents form a cycle: {}", show_cycle(cycle, parent)),
                        );
                    }
                    break;
                }
                path.push(parent);
                id = parent;
            }
        }
        reported.is_empty()
    }

    /// Run after the tree is known to be free of cycles, so any cycle found
    /// here goes through a requirement. Returns whether there is none.
    fn check_requirement_cycles(&mut self) -> bool {
        let source = self.source;
        // 0: not visited, 1: on the stack, 2: done.
        let mut state: HashMap<Id, u8> = HashMap::new();
        let mut reported = BTreeSet::new();
        for &start in source.nodes.keys() {
            if state.contains_key(&start) {
                continue;
            }
            let mut stack: Vec<(Id, Vec<Id>)> = vec![(start, source.edges(start))];
            let mut path = vec![start];
            state.insert(start, 1);
            while let Some((_, next)) = stack.last_mut() {
                let Some(id) = next.pop() else {
                    let (done, _) = stack.pop().unwrap_or_default();
                    state.insert(done, 2);
                    path.pop();
                    continue;
                };
                match state.get(&id) {
                    Some(1) => {
                        let pos = path.iter().position(|&p| p == id).unwrap_or(0);
                        let cycle = &path[pos..];
                        let first = *cycle.iter().min().unwrap_or(&id);
                        if reported.insert(first) {
                            self.diags.error(
                                self.file(first),
                                format!(
                                    "the requirements form a cycle (a parent counts as a requirement): {}",
                                    show_cycle(cycle, id)
                                ),
                            );
                        }
                    }
                    Some(_) => {}
                    None => {
                        state.insert(id, 1);
                        path.push(id);
                        stack.push((id, source.edges(id)));
                    }
                }
            }
        }
        reported.is_empty()
    }

    fn check_redundant_requirements(&mut self) {
        let source = self.source;
        for node in source.nodes.values() {
            let Some(rule) = &node.rule else { continue };
            let required: Vec<Id> = rule
                .requires
                .iter()
                .filter_map(|r| match r {
                    Ref::Local(id) if source.get(*id).is_some() => Some(*id),
                    _ => None,
                })
                .collect();
            for &target in &required {
                let others = node
                    .parent
                    .filter(|p| source.get(*p).is_some())
                    .map(|p| (p, true))
                    .into_iter()
                    .chain(
                        required
                            .iter()
                            .filter(|&&r| r != target)
                            .map(|&r| (r, false)),
                    );
                for (via, is_parent) in others {
                    if source.reach([via]).contains(&target) {
                        let how = match (is_parent, via == target) {
                            (true, true) => {
                                "it is the parent, which already counts as a requirement".to_owned()
                            }
                            (true, false) => format!(
                                "it already follows from the parent, {}",
                                self.describe(via)
                            ),
                            (false, _) => format!("it already follows from {}", self.describe(via)),
                        };
                        self.diags.error(
                            self.file(node.id),
                            format!("`requires` lists {target} needlessly: {how}; remove {target} from `requires`"),
                        );
                        break;
                    }
                }
            }
        }
    }

    fn check_conflicts(&mut self) {
        let source = self.source;
        let mut both_sides = BTreeSet::new();
        let mut reported = BTreeSet::new();
        for node in source.nodes.values() {
            let Some(rule) = &node.rule else { continue };
            for r in &rule.conflicts {
                let Ref::Local(other) = *r else { continue };
                let Some(other_node) = source.get(other) else {
                    continue;
                };
                if other_node.kind != Kind::Rule {
                    continue;
                }
                let file = self.file(node.id);
                let pair = (node.id.min(other), node.id.max(other));
                if reported.contains(&pair) {
                    // Written on both sides and already reported from the other.
                } else if source.reach([node.id]).contains(&other) {
                    reported.insert(pair);
                    self.diags.error(
                        &file,
                        format!("conflicts with {other}, which enabling {} brings in along `parent` or `requires`; the two can never be used apart, so remove the conflict or the requirement", node.id),
                    );
                } else if source.reach([other]).contains(&node.id) {
                    reported.insert(pair);
                    self.diags.error(
                        &file,
                        format!("conflicts with {other}, which brings in {} along `parent` or `requires`; the two can never be used apart, so remove the conflict or the requirement", node.id),
                    );
                }
                let declared_back = other_node
                    .rule
                    .as_ref()
                    .is_some_and(|o| o.conflicts.contains(&Ref::Local(node.id)));
                if declared_back && both_sides.insert((node.id.min(other), node.id.max(other))) {
                    let later = node.id.max(other);
                    let earlier = node.id.min(other);
                    self.diags.warning(
                        self.file(earlier),
                        format!("the conflict between {earlier} and {later} is written on both sides; write it only on the rule that came later, and remove it here"),
                    );
                }
            }
        }
    }

    /// A rule that brings in two children of one one-of can never be
    /// enabled, since at most one child of a one-of is.
    fn check_one_ofs(&mut self) {
        let source = self.source;
        // The children of each one-of that a set of nodes holds, for the
        // one-ofs of which it holds more than one.
        let clashes = |held: &BTreeSet<Id>| -> BTreeMap<Id, Vec<Id>> {
            let mut by_one_of: BTreeMap<Id, Vec<Id>> = BTreeMap::new();
            for &id in held {
                if let Some(parent) = source.get(id).and_then(|n| n.parent)
                    && source.get(parent).is_some_and(|p| p.kind == Kind::OneOf)
                {
                    by_one_of.entry(parent).or_default().push(id);
                }
            }
            by_one_of.retain(|_, children| children.len() > 1);
            by_one_of
        };
        for node in source.nodes.values().filter(|n| n.kind == Kind::Rule) {
            // Reported only on the rule that brings the two together, not
            // again on every rule that brings that one in.
            let inherited: BTreeSet<Id> = source
                .edges(node.id)
                .into_iter()
                .flat_map(|next| clashes(&source.reach([next])).into_keys())
                .collect();
            for (one_of, children) in clashes(&source.reach([node.id])) {
                if inherited.contains(&one_of) {
                    continue;
                }
                let list: Vec<String> = children.iter().map(Id::to_string).collect();
                self.diags.error(
                    self.file(node.id),
                    format!(
                        "rule {} can never be enabled: it brings in {}, children of one-of {one_of}, of which at most one may be enabled",
                        node.id,
                        list.join(" and ")
                    ),
                );
            }
        }
    }

    fn check_crossing_requirements(&mut self) {
        let source = self.source;
        // A rule that overrides another stands in another source's tree,
        // which this source cannot see.
        for node in source.nodes.values().filter(|n| n.overrides.is_none()) {
            let Some(rule) = &node.rule else { continue };
            for r in &rule.requires {
                let Ref::Local(target) = *r else { continue };
                let Some(target_node) = source.get(target) else {
                    continue;
                };
                if target_node.kind == Kind::Rule && source.root(target) != source.root(node.id) {
                    self.diags.warning(
                        self.file(node.id),
                        format!(
                            "requires rule {target}, under another domain ({}); a requirement across subtrees should name a one-of, so that any of its children satisfies it",
                            source.root(target)
                        ),
                    );
                }
            }
        }
    }
}

/// The findings on `field` of a rule naming `target`, a node of `source`,
/// against `file`: what can be named there, and a requirement of a rule
/// that is no longer in force. A node of another source than the rule's
/// (`foreign`) is shown as the rule writes it, `"team:7"`.
fn judge(
    source: &Source,
    field: &str,
    target: &Node,
    foreign: bool,
    file: &str,
    diags: &mut Diagnostics,
) {
    let show = |id: Id| {
        if foreign {
            format!("\"{}\"", source.show(id))
        } else {
            id.to_string()
        }
    };
    let shown = show(target.id);
    match (field, target.kind) {
        ("requires", Kind::OneOf) if source.children(target.id).next().is_none() => diags.error(
            file,
            format!("`requires` names one-of {shown}, which has no children yet, so the rule can never be enabled; require a rule, or add a child to the one-of first"),
        ),
        ("requires", Kind::Rule)
            if matches!(target.status, Status::Superseded | Status::Repealed) =>
        {
            let what = match target.superseded_by.as_slice() {
                [] => "is repealed, with nothing in its place".to_owned(),
                [next] => {
                    let next = show(*next);
                    format!("is superseded by {next}; require {next} instead")
                }
                many => {
                    let many: Vec<String> = many.iter().map(|id| show(*id)).collect();
                    format!(
                        "is superseded by {}; require those of them that apply instead",
                        many.join(", ")
                    )
                }
            };
            diags.warning(file, format!("`requires` names rule {shown}, which {what}"));
        }
        ("requires", Kind::Domain) => diags.error(
            file,
            format!("`requires` names domain {shown}; a domain cannot be required, require a rule or a one-of"),
        ),
        ("conflicts", kind) if kind != Kind::Rule => diags.error(
            file,
            format!("`conflicts` names {} {shown}; a rule can conflict only with a rule", kind.name()),
        ),
        _ => {}
    }
}

fn show_cycle(cycle: &[Id], back_to: Id) -> String {
    cycle
        .iter()
        .chain([&back_to])
        .map(Id::to_string)
        .collect::<Vec<_>>()
        .join(" -> ")
}
