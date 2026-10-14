//! Compiling a locked rule set into `AGENTS.md`
//! (`docs/design/rule-system.md`, Compilation and the site): the
//! constitution's version for agents, then the enabled rules along the
//! parent tree, with the citations in their text rendered.

use std::fmt::Write as _;
use std::path::Path;

use crate::cite::{self, Citation, Labels};
use crate::lock;
use crate::manifest::Part;
use crate::node::{Id, Kind, Node, Ref, Text, file_name};
use crate::project::Project;

pub const FILE: &str = "AGENTS.md";

/// The first line of every file `compile` writes; a file that does not
/// start with it was written by hand, and is never overwritten.
pub const GENERATED: &str =
    "<!-- Written by statute from statute.toml and statute.lock; do not edit by hand. -->";

/// The deepest heading Markdown has; deeper levels of the tree stay at it.
const DEEPEST: usize = 6;

/// The words the output adds around the rules, in one language.
struct Words {
    rules: &'static str,
    reason: &'static str,
    verification: &'static str,
    requires: &'static str,
    /// Follows the title of a cited rule that is not in the file.
    rule: &'static str,
}

fn words(lang: &str) -> Words {
    match lang {
        "zh" => Words {
            rules: "规定",
            reason: "理由：",
            verification: "判定：",
            requires: "依赖：",
            rule: "规定",
        },
        _ => Words {
            rules: "Rules",
            reason: "Reason: ",
            verification: "Verification: ",
            requires: "Requires: ",
            rule: "rule",
        },
    }
}

/// A text in the language compiled to, or in its original.
pub fn pick<'a>(text: &'a Text, node: &Node, lang: &str) -> &'a str {
    text.get(lang)
        .or_else(|| text.get(&node.lang))
        .map_or("", |s| s.trim_matches('\n'))
}

/// Writes `AGENTS.md` for a resolved project. Fails, with a message, when
/// the lock file is missing or not what `statute lock` would write now,
/// the constitution's version for agents in the language cannot be read,
/// or a rule's text cites a rule or provision that is not there.
pub fn compile(project: &Project) -> Result<String, String> {
    let current = lock::render(project);
    match std::fs::read_to_string(project.root.join(lock::FILE)) {
        Ok(text) if text == current => {}
        Ok(_) => {
            return Err(format!(
                "{} is not what statute.toml and the rules give now; run `statute lock` first",
                lock::FILE
            ));
        }
        Err(e) => {
            return Err(format!(
                "cannot read {}: {e}; run `statute lock` first",
                lock::FILE
            ));
        }
    }

    let m = &project.manifest;
    let lang = m.lang.as_str();
    let relative = format!(
        "constitution/{}/generated/constitution.{lang}.agent.md",
        m.edition
    );
    let constitution = std::fs::read_to_string(project.root.join(&m.source.path).join(&relative))
        .map_err(|e| {
        format!(
            "cannot read {relative} of source `{}`: {e}; `uv run constitution-build` writes it",
            m.source.name
        )
    })?;

    let articles = project
        .root
        .join(&m.source.path)
        .join(format!("constitution/{}/articles", m.edition));
    let labels = Labels::load(&articles, lang)?;

    let w = words(lang);
    let mut out = String::new();
    let _ = writeln!(out, "{GENERATED}\n");
    let (edition, revision, name, path) = (
        &m.edition,
        &project.revision,
        &m.source.name,
        &m.source.path,
    );
    let _ = match lang {
        "zh" => writeln!(
            out,
            "依据的宪法：{edition} 版，修订号 {revision}，来自规定源 `{name}` (`{path}`)。\n"
        ),
        _ => writeln!(
            out,
            "Constitution: edition {edition}, revision {revision}, from source `{name}` (`{path}`).\n"
        ),
    };
    out.push_str(constitution.trim_end());
    out.push_str("\n\n");
    let _ = writeln!(out, "# {}", w.rules);

    let source = &project.source;
    let mut domains: Vec<&Node> = source
        .nodes
        .values()
        .filter(|n| n.kind == Kind::Domain)
        .collect();
    domains.sort_by_key(|n| n.id);
    for domain in domains {
        render(project, &w, &labels, domain, 2, &mut out)?;
    }
    Ok(out)
}

/// Whether a node appears: a rule when enabled, a one-of or a domain when
/// something below it does.
fn shown(project: &Project, node: &Node) -> bool {
    match node.kind {
        Kind::Rule => project.resolution.enabled.contains_key(&node.id),
        Kind::OneOf | Kind::Domain => project.source.children(node.id).any(|c| shown(project, c)),
    }
}

/// A rule's text with its citations rendered: a rule as its title, a link
/// when it is in the file, else followed by its ID; a provision as its
/// label, in plain text, as in the constitution's version for agents.
fn cited(
    project: &Project,
    w: &Words,
    labels: &Labels,
    node: &Node,
    part: &str,
    text: &str,
) -> Result<String, String> {
    let lang = project.manifest.lang.as_str();
    let fail = |what: &str| {
        Err(format!(
            "{}: {part}: {what}",
            Path::new("rules").join(file_name(node.id)).display()
        ))
    };
    cite::replace(text, |c| match c {
        Citation::Rule(id) => match id.parse().ok().and_then(|id| project.source.get(id)) {
            Some(n) if shown(project, n) => {
                Ok(format!("[{}](#rule-{})", pick(&n.title, n, lang), n.id))
            }
            Some(n) => Ok(format!("{} ({} {})", pick(&n.title, n, lang), w.rule, n.id)),
            None => fail(&format!(
                "cites `{{@rule:{id}}}`, which is not in source `{}`",
                project.manifest.source.name
            )),
        },
        Citation::Constitution(id) => match labels.get(id) {
            Some(label) => Ok(label.to_string()),
            None => fail(&format!(
                "cites `{{@constitution:{id}}}`, which is not in edition {} of the constitution",
                project.manifest.edition
            )),
        },
        Citation::Bare(id) => fail(&format!(
            "cites `{{@{id}}}` without `rule:` or `constitution:`"
        )),
    })
}

fn render(
    project: &Project,
    w: &Words,
    labels: &Labels,
    node: &Node,
    level: usize,
    out: &mut String,
) -> Result<(), String> {
    if !shown(project, node) {
        return Ok(());
    }
    let lang = project.manifest.lang.as_str();
    let _ = writeln!(out, "\n<a id=\"rule-{}\"></a>", node.id);
    let _ = writeln!(
        out,
        "{} {} ({})",
        "#".repeat(level.min(DEEPEST)),
        pick(&node.title, node, lang),
        node.id
    );
    if let Some(rule) = &node.rule {
        for part in &project.manifest.agents_md {
            let (name, label, text) = match part {
                Part::Statement => ("statement", "", &rule.statement),
                Part::Reason => ("reason", w.reason, &rule.reason),
                Part::Verification => ("verification", w.verification, &rule.verification),
            };
            let text = cited(project, w, labels, node, name, pick(text, node, lang))?;
            if label.is_empty() {
                let _ = writeln!(out, "\n{text}");
            } else {
                let _ = writeln!(out, "\n{label}{text}");
            }
        }
        let crossing: Vec<String> = rule
            .requires
            .iter()
            .filter_map(|r| match r {
                Ref::Local(id) => Some(*id),
                Ref::Foreign { .. } => None,
            })
            .filter(|&id| project.source.root(id) != project.source.root(node.id))
            .filter_map(|id: Id| project.source.get(id))
            .map(|n| format!("[{}](#rule-{})", pick(&n.title, n, lang), n.id))
            .collect();
        if !crossing.is_empty() {
            let _ = writeln!(out, "\n{}{}", w.requires, crossing.join(", "));
        }
    }
    let mut children: Vec<&Node> = project.source.children(node.id).collect();
    children.sort_by_key(|n| n.id);
    for child in children {
        render(project, w, labels, child, level + 1, out)?;
    }
    Ok(())
}
