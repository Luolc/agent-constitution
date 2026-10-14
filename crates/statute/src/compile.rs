//! Compiling a locked rule set (`docs/design/rule-system.md`, Compilation
//! and the site) into `AGENTS.md`, the constitution's version for agents
//! and then the enabled rules along the parent tree, with the citations in
//! their text rendered; and, next to it, the skills the enabled rules
//! need, the checks to install and the hook configuration that runs them.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::cite::{self, Citation, Labels};
use crate::lock;
use crate::manifest::Part;
use crate::node::{Check, Id, Kind, Node, Ref, Text, file_name};
use crate::project::Project;
use crate::skill;

pub const FILE: &str = "AGENTS.md";

/// The first line of the `AGENTS.md` `compile` writes; a file that does
/// not start with it was written by hand, and is never overwritten.
pub const GENERATED: &str =
    "<!-- Written by statute from statute.toml and statute.lock; do not edit by hand. -->";

/// The first line of the other files `compile` writes, as a comment of
/// TOML and YAML, and the content of the marker in a skill directory.
pub const GENERATED_HASH: &str =
    "# Written by statute from statute.toml and statute.lock; do not edit by hand.";

/// The check binary's version and the external commands of the enabled
/// rules, next to `AGENTS.md`.
pub const CHECKS: &str = "statute-checks.toml";

/// A pre-commit configuration that runs `statute check`, next to
/// `AGENTS.md`.
pub const HOOKS: &str = "statute-hooks.yaml";

/// Where the skills go, below the directory of `AGENTS.md`.
pub const SKILLS: &str = ".agents/skills";

/// Where the links to the skills go, below the directory of `AGENTS.md`.
pub const LINKS: &str = ".claude/skills";

/// The file in a skill directory that says statute wrote it.
pub const MARKER: &str = ".statute-generated";

const HOOKS_BODY: &str = "repos:
  - repo: local
    hooks:
      - id: statute-check
        name: statute check
        entry: statute check
        language: system
        pass_filenames: false
        always_run: true
";

/// Everything `compile` writes, before it is written.
pub struct Output {
    pub agents_md: String,
    pub checks: String,
    pub hooks: String,
    /// The skills to install, sorted by name: the name and the skill's
    /// directory in the source.
    pub skills: Vec<(String, PathBuf)>,
}

#[derive(Serialize)]
struct Checks<'a> {
    statute: &'a str,
    commands: Vec<Command<'a>>,
}

#[derive(Serialize)]
struct Command<'a> {
    rule: Id,
    run: &'a str,
    tools: &'a [String],
}

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

/// Compiles a resolved project. Fails, with a message, when the lock file
/// is missing or not what `statute lock` would write now, the
/// constitution's version for agents in the language cannot be read, a
/// rule's text cites a rule or provision that is not there, or a skill's
/// declaration cannot be read or names a rule that is not there.
pub fn compile(project: &Project) -> Result<Output, String> {
    let agents_md = agents_md(project)?;
    let m = &project.manifest;
    let mut skills = Vec::new();
    for s in skill::load(&project.root.join(&m.source.path), &m.source.path)? {
        if let Some(id) = s.rules.iter().find(|&&id| project.source.get(id).is_none()) {
            return Err(format!(
                "{}: `metadata.{}` names rule {id}, which is not in source `{}`",
                crate::project::shown(&m.source.path, &format!("skills/{}/SKILL.md", s.name)),
                skill::KEY,
                m.source.name
            ));
        }
        if s.rules
            .iter()
            .any(|id| project.resolution.enabled.contains_key(id))
        {
            skills.push((s.name, s.dir));
        }
    }
    let commands = project
        .resolution
        .enabled
        .keys()
        .filter_map(|&id| match &project.source.get(id)?.rule.as_ref()?.check {
            Check::Command { run, tools } => Some(Command {
                rule: id,
                run,
                tools,
            }),
            _ => None,
        })
        .collect();
    let checks = toml::to_string(&Checks {
        statute: env!("CARGO_PKG_VERSION"),
        commands,
    })
    .map_err(|e| format!("cannot write {CHECKS}: {e}"))?;
    Ok(Output {
        agents_md,
        checks: format!("{GENERATED_HASH}\n\n{checks}"),
        hooks: format!("{GENERATED_HASH}\n\n{HOOKS_BODY}"),
        skills,
    })
}

/// Writes the output: `AGENTS.md` to `agents_md`, the rest next to it,
/// each skill as a copy in `.agents/skills/<name>` with a link to it,
/// `.claude/skills/<name>`, since Claude Code reads only that directory.
/// Checks every place first, and writes nothing when one holds something
/// statute did not write. A skill directory statute wrote that is no
/// longer needed is removed, with its link. Links need a Unix system;
/// elsewhere a rule set with a skill fails. Returns what was written.
pub fn write(output: &Output, agents_md: &Path) -> Result<Vec<PathBuf>, String> {
    let dir = agents_md.parent().unwrap_or(Path::new(""));
    let files = [
        (agents_md.to_path_buf(), &output.agents_md, GENERATED),
        (dir.join(CHECKS), &output.checks, GENERATED_HASH),
        (dir.join(HOOKS), &output.hooks, GENERATED_HASH),
    ];
    let refuse = |path: &Path| {
        Err(format!(
            "{}: not written by statute, so it is left as it is and nothing is written; write elsewhere with --output, or move it away first",
            path.display()
        ))
    };
    for (path, _, marker) in &files {
        let Ok(meta) = path.symlink_metadata() else {
            continue;
        };
        let generated = meta.is_file()
            && std::fs::read(path).is_ok_and(|t| t.starts_with(format!("{marker}\n").as_bytes()));
        if !generated {
            return refuse(path);
        }
    }
    let (skills, links) = (dir.join(SKILLS), dir.join(LINKS));
    for (name, _) in &output.skills {
        let target = skills.join(name);
        if target.symlink_metadata().is_ok() && !generated_skill(&target) {
            return refuse(&target);
        }
        let link = links.join(name);
        if link.symlink_metadata().is_ok() && !links_to(&link, name) {
            return refuse(&link);
        }
    }
    if !cfg!(unix) && !output.skills.is_empty() {
        return Err(format!(
            "the skills need a link in {LINKS}, which statute makes only on a Unix system"
        ));
    }
    let mut sources = Vec::new();
    for (name, from) in &output.skills {
        sources.push((name, from, files_below(from)?));
    }

    let fail = |path: &Path, e: std::io::Error| format!("{}: cannot write: {e}", path.display());
    let mut written = Vec::new();
    for (path, text, _) in files {
        std::fs::write(&path, text).map_err(|e| fail(&path, e))?;
        written.push(path);
    }
    // Every skill directory statute wrote goes, the ones still needed
    // included, so that a file taken out of a skill goes too.
    if let Ok(entries) = std::fs::read_dir(&skills) {
        let mut stale: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| generated_skill(p))
            .collect();
        stale.sort();
        for path in stale {
            std::fs::remove_dir_all(&path).map_err(|e| fail(&path, e))?;
            if let Some(name) = path.file_name() {
                let link = links.join(name);
                if links_to(&link, &name.to_string_lossy()) {
                    std::fs::remove_file(&link).map_err(|e| fail(&link, e))?;
                }
            }
        }
    }
    for (name, from, relative) in sources {
        let target = skills.join(name);
        for file in relative {
            let to = target.join(&file);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|e| fail(parent, e))?;
            }
            std::fs::copy(from.join(&file), &to).map_err(|e| fail(&to, e))?;
        }
        let marker = target.join(MARKER);
        std::fs::write(&marker, format!("{GENERATED_HASH}\n")).map_err(|e| fail(&marker, e))?;
        written.push(target);
        let link = links.join(name);
        if link.symlink_metadata().is_err() {
            std::fs::create_dir_all(&links).map_err(|e| fail(&links, e))?;
            symlink(&link_target(name), &link).map_err(|e| fail(&link, e))?;
        }
        written.push(link);
    }
    Ok(written)
}

#[cfg(unix)]
fn symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn symlink(_: &Path, _: &Path) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "links are made only on a Unix system",
    ))
}

/// What `.claude/skills/<name>` points to: the skill's copy, relatively.
fn link_target(name: &str) -> PathBuf {
    Path::new("../..").join(SKILLS).join(name)
}

/// Whether `link` is the link statute makes for the skill `name`.
fn links_to(link: &Path, name: &str) -> bool {
    std::fs::read_link(link).is_ok_and(|t| t == link_target(name))
}

/// Whether `path` is a skill directory statute wrote: a directory, not a
/// link to one, with the marker in it.
fn generated_skill(path: &Path) -> bool {
    path.symlink_metadata().is_ok_and(|m| m.is_dir()) && path.join(MARKER).is_file()
}

/// The regular files below `dir`, relative to it and sorted. Fails on
/// anything else, such as a symbolic link, which a copy would not carry.
fn files_below(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut todo = vec![PathBuf::new()];
    while let Some(relative) = todo.pop() {
        let entries = std::fs::read_dir(dir.join(&relative))
            .map_err(|e| format!("{}: cannot read: {e}", dir.join(&relative).display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: cannot read: {e}", dir.display()))?;
            let path = relative.join(entry.file_name());
            let kind = entry
                .file_type()
                .map_err(|e| format!("{}: cannot read: {e}", dir.join(&path).display()))?;
            if kind.is_dir() {
                todo.push(path);
            } else if kind.is_file() {
                files.push(path);
            } else {
                return Err(format!(
                    "{}: only directories and regular files are copied into a skill",
                    dir.join(&path).display()
                ));
            }
        }
    }
    files.sort();
    Ok(files)
}

/// The text of `AGENTS.md`.
fn agents_md(project: &Project) -> Result<String, String> {
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
