//! A source: the nodes of one `rules/` directory, and the checks on how
//! they relate (`docs/design/rule-system.md`, Dependencies, one-ofs and
//! packs).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use crate::diag::Diagnostics;
use crate::node::{self, Id, Kind, Node, Ref, Status, show_ref};

#[derive(Debug, Default)]
pub struct Source {
    pub nodes: BTreeMap<Id, Node>,
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
        let (source, more) = Source::from_files(files);
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
        Checker {
            source: &source,
            dir,
            diags: &mut diags,
        }
        .run();
        (source, diags)
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
                for r in list {
                    let id = match r {
                        Ref::Foreign { .. } => {
                            self.diags.error(
                                &file,
                                format!(
                                    "`{field}` names {}, a rule of another source; references to other sources are not supported yet",
                                    show_ref(r)
                                ),
                            );
                            continue;
                        }
                        Ref::Local(id) => *id,
                    };
                    match (field, source.get(id).map(|n| n.kind)) {
                        (_, None) => self.diags.error(
                            &file,
                            format!("`{field}` names {id}, which does not exist in this source"),
                        ),
                        ("requires", Some(Kind::OneOf)) if source.children(id).next().is_none() => {
                            self.diags.error(
                                &file,
                                format!("`requires` names one-of {id}, which has no children yet, so the rule can never be enabled; require a rule, or add a child to the one-of first"),
                            )
                        }
                        ("requires", Some(Kind::Rule))
                            if matches!(
                                source.get(id).map(|n| n.status),
                                Some(Status::Superseded | Status::Repealed)
                            ) =>
                        {
                            let target = &source.nodes[&id];
                            let what = match target.superseded_by.as_slice() {
                                [] => "is repealed, with nothing in its place".to_owned(),
                                [next] => format!("is superseded by {next}; require {next} instead"),
                                many => {
                                    let many: Vec<String> = many.iter().map(Id::to_string).collect();
                                    format!(
                                        "is superseded by {}; require those of them that apply instead",
                                        many.join(", ")
                                    )
                                }
                            };
                            self.diags.warning(&file, format!("`requires` names rule {id}, which {what}"));
                        }
                        ("requires", Some(Kind::Domain)) => self.diags.error(
                            &file,
                            format!("`requires` names domain {id}; a domain cannot be required, require a rule or a one-of"),
                        ),
                        ("conflicts", Some(kind)) if kind != Kind::Rule => self.diags.error(
                            &file,
                            format!("`conflicts` names {} {id}; a rule can conflict only with a rule", kind.name()),
                        ),
                        _ => {}
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
        for node in source.nodes.values() {
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

fn show_cycle(cycle: &[Id], back_to: Id) -> String {
    cycle
        .iter()
        .chain([&back_to])
        .map(Id::to_string)
        .collect::<Vec<_>>()
        .join(" -> ")
}
