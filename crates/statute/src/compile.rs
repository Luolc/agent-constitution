//! Compiling a locked rule set (`docs/design/statute-command.md`, Compilation
//! and the site) into `AGENTS.md`, the constitution's version for agents
//! and then the enabled rules along the parent tree, with the citations in
//! their text rendered; and, next to it, the skills the enabled rules
//! need and the checks to install.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::cite::{self, Citation, Labels};
use crate::lock;
use crate::manifest::{Key, Part};
use crate::node::{self, Check, Kind, Node, Text};
use crate::param;
use crate::private;
use crate::project::Project;
use crate::skill;
use crate::source::describe;

pub const FILE: &str = "AGENTS.md";

/// The first line of the `AGENTS.md` `compile` writes; a file that does
/// not start with it was written by hand, and is never overwritten.
pub const GENERATED: &str =
    "<!-- Written by statute from statute.toml and statute.lock; do not edit by hand. -->";

/// The first line of the other file `compile` writes, as a TOML
/// comment, and the content of the marker in a skill directory.
pub const GENERATED_HASH: &str =
    "# Written by statute from statute.toml and statute.lock; do not edit by hand.";

/// The check binary's version and the external commands of the enabled
/// rules, next to `AGENTS.md`.
pub const CHECKS: &str = "statute-checks.toml";

/// Where the skills go, below the directory of `AGENTS.md`.
pub const SKILLS: &str = ".agents/skills";

/// Where the links to the skills go, below the directory of `AGENTS.md`.
pub const LINKS: &str = ".claude/skills";

/// The file in a skill directory that says statute wrote it.
pub const MARKER: &str = ".statute-generated";

/// Everything `compile` writes, before it is written.
pub struct Output {
    pub agents_md: String,
    pub checks: String,
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
    rule: String,
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
    /// Before the rule an overriding rule overrides, and between the
    /// rules of a chain of overrides.
    overrides: &'static str,
    which_overrides: &'static str,
    /// Before the title of the one-of whose option stands in its place.
    one_of: &'static str,
    end: &'static str,
}

fn words(lang: &str) -> Words {
    match lang {
        "zh" => Words {
            rules: "规定",
            reason: "理由：",
            verification: "判定：",
            requires: "依赖：",
            rule: "规定",
            overrides: "覆盖 ",
            which_overrides: "，后者覆盖 ",
            one_of: "择一：",
            end: "。",
        },
        _ => Words {
            rules: "Rules",
            reason: "Reason: ",
            verification: "Verification: ",
            requires: "Requires: ",
            rule: "rule",
            overrides: "Overrides ",
            which_overrides: ", which overrides ",
            one_of: "One of: ",
            end: ".",
        },
    }
}

/// A text in the language compiled to, or in its original.
pub fn pick<'a>(text: &'a Text, node: &Node, lang: &str) -> &'a str {
    text.get(lang)
        .or_else(|| text.get(&node.lang))
        .map_or("", |s| s.trim_matches('\n'))
}

/// `text`, of node `key`, with each placeholder replaced by the value of
/// its parameter, a language as its name in the language compiled to:
/// the committed value, or with `private` the person's own
/// (`docs/design/rule-config.md`, Values in the lock file and the
/// outputs). A rule that is not enabled, whose parameters have no
/// setting, gets each default.
pub fn filled(project: &Project, key: &Key, text: &str, private: bool) -> String {
    let lang = project.manifest.lang.as_str();
    param::fill(text, |name| match project.values.get(name) {
        Some(s) => s
            .param
            .shown(if private { s.private_value() } else { &s.value }, lang),
        None => match project.sources.param(&key.source, name) {
            Ok((_, p)) => p.shown(&p.default, lang),
            Err(_) => format!("{{${name}}}"),
        },
    })
}

/// A node's title in the language compiled to, its placeholders filled.
fn title(project: &Project, key: &Key, node: &Node, private: bool) -> String {
    filled(
        project,
        key,
        pick(&node.title, node, &project.manifest.lang),
        private,
    )
}

/// Compiles a resolved project. Fails, with a message, when the lock file
/// is missing or not what `statute lock` would write now, the
/// constitution's version for agents in the language cannot be read, a
/// rule's text cites a rule or provision that is not there, or a skill's
/// declaration cannot be read or names a rule that is not there.
pub fn compile(project: &Project) -> Result<Output, String> {
    check_lock(
        &project.root.join(lock::FILE),
        lock::FILE,
        &lock::render(project),
        crate::manifest::FILE,
    )?;
    let agents_md = agents_md(project, true)?;
    let mut skills: Vec<(String, PathBuf)> = Vec::new();
    let mut from: Vec<&str> = Vec::new();
    // The `skills/` at the top belong to the source there, if any; the
    // repository's own rules then have none.
    let here = project.manifest.sources.iter().any(|s| s.is_here());
    for source in project.sources.iter() {
        let spec = project.manifest.spec(&source.name);
        if spec.is_none() && here {
            continue;
        }
        let label = spec.map_or_else(|| source.path.clone(), |spec| spec.shown(""));
        for s in skill::load(&project.root.join(&source.path), &label)? {
            let below = format!("skills/{}/SKILL.md", s.name);
            let file = spec.map_or_else(
                || crate::project::shown(&source.path, &below),
                |spec| spec.shown(&below),
            );
            if let Some(id) = s.rules.iter().find(|&&id| source.get(id).is_none()) {
                return Err(format!(
                    "{file}: `metadata.{}` names rule {id}, which is not in {}",
                    skill::KEY,
                    source.described()
                ));
            }
            if !s.rules.iter().any(|&id| {
                project
                    .resolution
                    .enabled
                    .contains_key(&Key::new(&source.name, id))
            }) {
                continue;
            }
            if let Some(i) = skills.iter().position(|(name, _)| *name == s.name) {
                return Err(format!(
                    "{file}: skill `{}` of {} has the name of one of {}, and both are needed; one of the sources has to rename its skill",
                    s.name,
                    source.described(),
                    crate::source::describe(from[i])
                ));
            }
            skills.push((s.name, s.dir));
            from.push(&source.name);
        }
    }
    skills.sort();
    let commands = project
        .resolution
        .enabled
        .keys()
        .filter_map(
            |key| match &project.sources.get(key)?.rule.as_ref()?.check {
                Check::Command { run, tools } => Some(Command {
                    rule: key.to_string(),
                    run,
                    tools,
                }),
                _ => None,
            },
        )
        .collect();
    let checks = toml::to_string(&Checks {
        statute: env!("CARGO_PKG_VERSION"),
        commands,
    })
    .map_err(|e| format!("cannot write {CHECKS}: {e}"))?;
    Ok(Output {
        agents_md,
        checks: format!("{GENERATED_HASH}\n\n{checks}"),
        skills,
    })
}

/// Writes the output: `AGENTS.md` to `agents_md`, the rest next to it,
/// each skill as a copy in `.agents/skills/<name>` with a link to it,
/// `.claude/skills/<name>`, since Claude Code reads only that directory.
/// Checks every place first, and writes nothing when one holds something
/// statute did not write, or when `.agents/skills` or `.claude/skills` is
/// not a plain directory while a skill is to be written. A skill directory
/// statute wrote that is no longer needed is removed, with its link. An
/// error midway can leave part of the output; the next run finishes it.
/// Links need a Unix system; elsewhere a rule set with a skill fails.
/// Returns what was written.
pub fn write(output: &Output, agents_md: &Path) -> Result<Vec<PathBuf>, String> {
    let dir = agents_md.parent().unwrap_or(Path::new(""));
    let files = [
        (agents_md.to_path_buf(), &output.agents_md, GENERATED),
        (dir.join(CHECKS), &output.checks, GENERATED_HASH),
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
    // A link in place of either directory, such as `.claude/skills` to
    // `../.agents/skills`, would put the copies and the links somewhere
    // else, or make each skill's link its own copy.
    if !output.skills.is_empty() {
        for d in [&skills, &links] {
            if d.symlink_metadata().is_ok_and(|m| !m.is_dir()) {
                return Err(format!(
                    "{}: not a directory but a link or a file, so nothing is written; statute writes one link per skill in {LINKS}, so make it a plain directory, or move it away first",
                    d.display()
                ));
            }
        }
    }
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
    // In an order that a run stopped by an error can be repeated from:
    // each skill's marker before its files, a link removed before its
    // directory, and the `AGENTS.md` last.
    //
    // Every skill directory statute wrote goes, the ones still needed
    // included, so that a file taken out of a skill goes too.
    if skills.symlink_metadata().is_ok_and(|m| m.is_dir())
        && let Ok(entries) = std::fs::read_dir(&skills)
    {
        let mut stale: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| generated_skill(p))
            .collect();
        stale.sort();
        for path in stale {
            if let Some(name) = path.file_name() {
                let link = links.join(name);
                if links_to(&link, &name.to_string_lossy()) {
                    std::fs::remove_file(&link).map_err(|e| fail(&link, e))?;
                }
            }
            std::fs::remove_dir_all(&path).map_err(|e| fail(&path, e))?;
        }
    }
    for (name, from, relative) in sources {
        let target = skills.join(name);
        std::fs::create_dir_all(&target).map_err(|e| fail(&target, e))?;
        let marker = target.join(MARKER);
        std::fs::write(&marker, format!("{GENERATED_HASH}\n")).map_err(|e| fail(&marker, e))?;
        for file in relative {
            let to = target.join(&file);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|e| fail(parent, e))?;
            }
            std::fs::copy(from.join(&file), &to).map_err(|e| fail(&to, e))?;
        }
        written.push(target);
        let link = links.join(name);
        if link.symlink_metadata().is_err() {
            std::fs::create_dir_all(&links).map_err(|e| fail(&links, e))?;
            symlink(&link_target(name), &link).map_err(|e| fail(&link, e))?;
        }
        written.push(link);
    }
    for (path, text, _) in files.into_iter().rev() {
        std::fs::write(&path, text).map_err(|e| fail(&path, e))?;
        written.push(path);
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

/// The full text of the private layer of a public repository: the
/// `AGENTS.md` of `both`, which holds both layers over `public`, without
/// the line that points to it, after checking `statute.lock` against
/// `public` and the private lock file in `dir` against `both`
/// (`docs/design/rule-sources.md`, Private rules in a public repository).
/// Without a private rule set, `both` is `public` and `dir` is `None`:
/// the text then differs only in the person's own values.
/// Fails as `compile` does, and when it is larger than Codex reads.
pub fn full_text(public: &Project, both: &Project, dir: Option<&Path>) -> Result<String, String> {
    check_lock(
        &public.root.join(lock::FILE),
        lock::FILE,
        &lock::render(public),
        crate::manifest::FILE,
    )?;
    if let Some(dir) = dir {
        let path = dir.join(private::LOCK);
        check_lock(
            &path,
            &path.display().to_string(),
            &lock::render_private(public, both),
            private::MANIFEST,
        )?;
    }
    let text = agents_md(both, false)?;
    if text.len() > private::MAX_BYTES {
        return Err(format!(
            "{}: {} bytes, more than the {} that Codex reads of a project's instruction files; enable fewer rules",
            private::full().display(),
            text.len(),
            private::MAX_BYTES
        ));
    }
    Ok(text)
}

/// Whether a person's own value of a `personal` parameter differs from
/// the committed one, which makes the private output worth writing in a
/// public repository with no private rule set.
pub fn personal_differs(project: &Project) -> bool {
    project
        .values
        .values()
        .any(|s| s.personal.as_ref().is_some_and(|(v, _)| *v != s.value))
}

/// Fails unless the lock file at `path`, shown as `shown`, holds
/// `current`, what the rule set `set` and the rules give now.
fn check_lock(path: &Path, shown: &str, current: &str, set: &str) -> Result<(), String> {
    match std::fs::read_to_string(path) {
        Ok(text) if text == current => Ok(()),
        Ok(_) => Err(format!(
            "{shown} is not what {set} and the rules give now; run `statute lock` first"
        )),
        Err(e) => Err(format!(
            "cannot read {shown}: {e}; run `statute lock` first"
        )),
    }
}

/// The text of `AGENTS.md`; with `fallback`, in a public repository,
/// ending with the line that points to the private layer's full text.
fn agents_md(project: &Project, fallback: bool) -> Result<String, String> {
    let m = &project.manifest;
    let spec = m.constitution_spec();
    let lang = m.lang.as_str();
    let relative = format!(
        "constitution/{}/generated/constitution.{lang}.agent.md",
        m.edition
    );
    let constitution = std::fs::read_to_string(project.root.join(&spec.path).join(&relative))
        .map_err(|e| {
            format!(
                "cannot read {relative} of source `{}`: {e}; `uv run constitution-build` writes it",
                spec.name
            )
        })?;

    let labels = labels(project)?;

    let w = words(lang);
    let mut out = String::new();
    let _ = writeln!(out, "{GENERATED}\n");
    let (edition, revision, name, path) =
        (&m.edition, &project.revision, &spec.name, &spec.place());
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

    // The domains of the first-party source, then those of the other
    // sources by name, then those of the repository's own rules; each
    // with its subdomains below it.
    let domains: Vec<Key> = project
        .sources
        .nodes()
        .filter(|(_, n)| n.kind == Kind::Domain && n.parent.is_none() && n.foreign_parent.is_none())
        .map(|(key, _)| key)
        .collect();
    for domain in &domains {
        render(project, &w, &labels, domain, 2, !fallback, &mut out)?;
    }
    if fallback && m.public {
        let _ = writeln!(out, "\n---\n\n{}", private::fallback(lang));
    }
    Ok(out)
}

/// The anchor of a node's heading, its reference with any colon made a
/// hyphen: `rule-R42`, `rule-L12`, `rule-team-7`.
fn anchor(key: &Key) -> String {
    format!("rule-{}", key.to_string().replace(':', "-"))
}

/// Whether a node appears: a rule when enabled and the rule in its place
/// goes to the agents (its `uses` has `instructions`), a one-of or a
/// domain when something below it does.
fn shown(project: &Project, key: &Key) -> bool {
    match project.sources.get(key).map(|n| n.kind) {
        Some(Kind::Rule) => {
            project.resolution.enabled.contains_key(key)
                && project
                    .sources
                    .get(&project.resolution.standing(key))
                    .and_then(|n| n.rule.as_ref())
                    .is_some_and(|r| r.uses.instructions)
        }
        Some(Kind::OneOf | Kind::Domain) => project
            .sources
            .children(key)
            .iter()
            .any(|c| shown(project, c)),
        None => false,
    }
}

/// The labels of the provisions of the project's constitution, in the
/// language compiled to.
pub fn labels(project: &Project) -> Result<Labels, String> {
    let m = &project.manifest;
    let articles = project
        .root
        .join(&m.constitution_spec().path)
        .join(format!("constitution/{}/articles", m.edition));
    Labels::load(&articles, &m.lang)
}

/// The text of node `key` with its citations rendered: a rule as its
/// title, with `links` a link when it is in the file, else followed by
/// its reference; a provision as its label, in plain text, as in the
/// constitution's version for agents.
pub fn cited(
    project: &Project,
    labels: &Labels,
    key: &Key,
    part: &str,
    text: &str,
    links: bool,
    private: bool,
) -> Result<String, String> {
    let lang = project.manifest.lang.as_str();
    let w = words(lang);
    let fail = |what: &str| Err(format!("{}: {part}: {what}", project.sources.file(key)));
    let text = filled(project, key, text, private);
    cite::replace(&text, |c| match c {
        Citation::Rule(text) => {
            let Some(cited) = node::cited_ref(text).map(|r| r.key(&key.source)) else {
                return fail(&format!(
                    "cites `{{@rule:{text}}}`, which is not a reference"
                ));
            };
            if project.sources.get(&cited).is_none() {
                return fail(&format!(
                    "cites `{{@rule:{text}}}`, which is not in {}",
                    describe(&cited.source)
                ));
            }
            // An overridden rule is shown as the rule in its place.
            let cited = project.resolution.standing(&cited);
            match project.sources.get(&cited) {
                Some(n) if links && shown(project, &cited) => Ok(format!(
                    "[{}](#{})",
                    title(project, &cited, n, private),
                    anchor(&cited)
                )),
                Some(n) => Ok(format!(
                    "{} ({} {cited})",
                    title(project, &cited, n, private),
                    w.rule
                )),
                None => fail(&format!(
                    "cites `{{@rule:{text}}}`, which is not in {}",
                    describe(&cited.source)
                )),
            }
        }
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

/// Writes node `key` and what is below it at heading `level`. A one-of
/// has no heading of its own: its enabled option stands in its place,
/// with a line that names the one-of (`docs/design/rule-config.md`, No
/// rule under a rule).
fn render(
    project: &Project,
    w: &Words,
    labels: &Labels,
    key: &Key,
    level: usize,
    private: bool,
    out: &mut String,
) -> Result<(), String> {
    render_in(project, w, labels, key, level, private, None, out)
}

/// As `render`, for an option of the one-of `of`.
#[allow(clippy::too_many_arguments)]
fn render_in(
    project: &Project,
    w: &Words,
    labels: &Labels,
    key: &Key,
    level: usize,
    private: bool,
    of: Option<&Key>,
    out: &mut String,
) -> Result<(), String> {
    let Some(at) = project.sources.get(key) else {
        return Ok(());
    };
    if !shown(project, key) {
        return Ok(());
    }
    if at.kind == Kind::OneOf {
        let mut options = project.sources.children(key);
        options.sort();
        for option in options {
            render_in(project, w, labels, &option, level, private, Some(key), out)?;
        }
        return Ok(());
    }
    // An overridden rule's place holds the rule that stands in it, with
    // the anchors of every rule of the chain, so links to them land here.
    let res = &project.resolution;
    let standing = res.standing(key);
    let mut chain = res.chain(&standing);
    let Some(node) = project.sources.get(&standing) else {
        return Ok(());
    };
    let lang = project.manifest.lang.as_str();
    out.push('\n');
    // A link to the one-of lands on the option in its place.
    for k in of.into_iter().chain(chain.iter().rev()).chain([&standing]) {
        let _ = writeln!(out, "<a id=\"{}\"></a>", anchor(k));
    }
    let _ = writeln!(
        out,
        "{} {} ({standing})",
        "#".repeat(level.min(DEEPEST)),
        title(project, &standing, node, private),
    );
    if !chain.is_empty() {
        let names: Vec<String> = chain.iter().map(Key::to_string).collect();
        let _ = writeln!(
            out,
            "\n{}{}{}",
            w.overrides,
            names.join(w.which_overrides),
            w.end
        );
    }
    if let Some(one_of) = of.and_then(|k| Some((k, project.sources.get(k)?))) {
        let _ = writeln!(
            out,
            "\n{}{}{}",
            w.one_of,
            title(project, one_of.0, one_of.1, private),
            w.end
        );
    }
    if let Some(applicability) = &node.applicability {
        let text = pick(applicability, node, lang);
        let text = cited(
            project,
            labels,
            &standing,
            "applicability",
            text,
            true,
            private,
        )?;
        let _ = writeln!(out, "\n{text}");
    }
    if let Some(rule) = &node.rule {
        for part in &project.manifest.agents_md {
            let (name, label, text) = match part {
                Part::Statement => ("statement", "", &rule.statement),
                Part::Reason => ("reason", w.reason, &rule.reason),
                Part::Verification => ("verification", w.verification, &rule.verification),
            };
            let text = cited(
                project,
                labels,
                &standing,
                name,
                pick(text, node, lang),
                true,
                private,
            )?;
            if label.is_empty() {
                let _ = writeln!(out, "\n{text}");
            } else {
                let _ = writeln!(out, "\n{label}{text}");
            }
        }
        // A requirement that leads out of the nearest domain, a subdomain
        // included, is a link.
        let domain = project.sources.domain(&standing);
        let crossing: Vec<String> = project
            .sources
            .requires(&standing)
            .into_iter()
            .filter(|target| project.sources.domain(target) != domain)
            .filter_map(|target| {
                // A rule not in the file, one that only the reviewer
                // reads, is named as a citation of it is.
                let in_file = shown(project, &target);
                let target = res.standing(&target);
                let n = project.sources.get(&target)?;
                let title = title(project, &target, n, private);
                Some(if in_file {
                    format!("[{title}](#{})", anchor(&target))
                } else {
                    format!("{title} ({} {target})", w.rule)
                })
            })
            .collect();
        if !crossing.is_empty() {
            let _ = writeln!(out, "\n{}{}", w.requires, crossing.join(", "));
        }
    }
    // The children of every rule of the chain stay, under this heading.
    chain.push(standing);
    let mut children: Vec<Key> = chain
        .iter()
        .flat_map(|k| project.sources.children(k))
        .collect();
    children.sort();
    for child in children {
        render(project, w, labels, &child, level + 1, private, out)?;
    }
    Ok(())
}
