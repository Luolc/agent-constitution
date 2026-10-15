//! The local site of a rule set: every node of its sources, the private
//! ones included, as static HTML pages in `.statute/site/`, which git
//! ignores and which a browser opens from disk. Nothing is published.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cite::{self, Citation, Labels};
use crate::compile::{labels, pick};
use crate::manifest::Key;
use crate::node::{self, Kind, Node, Status};
use crate::private;
use crate::project::Project;
use crate::source::Source;

/// The site's directory, below the private output's.
pub fn dir() -> PathBuf {
    Path::new(private::DIR).join("site")
}

/// The `.gitignore` the site's directory holds: git ignores every file
/// there, that one included. It also marks the directory as statute's.
pub const GITIGNORE: &str = "*\n";

/// Where a source's rules reach, and who may read them
/// (`docs/design/rule-sources.md`, Four kinds of source), in the order
/// the index shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quadrant {
    GlobalPublic,
    GlobalPrivate,
    LocalPublic,
    LocalPrivate,
}

pub const QUADRANTS: [Quadrant; 4] = [
    Quadrant::GlobalPublic,
    Quadrant::GlobalPrivate,
    Quadrant::LocalPublic,
    Quadrant::LocalPrivate,
];

impl Quadrant {
    /// The repository's own rules, `L`, and a source of `scope = "local"`
    /// are local; `public` is the source's, and the rule set's for `L`.
    pub fn of(source: &Source) -> Quadrant {
        match (source.local, source.public) {
            (false, true) => Quadrant::GlobalPublic,
            (false, false) => Quadrant::GlobalPrivate,
            (true, true) => Quadrant::LocalPublic,
            (true, false) => Quadrant::LocalPrivate,
        }
    }

    /// The class of its section and the ID of its filter.
    pub fn class(self) -> &'static str {
        match self {
            Quadrant::GlobalPublic => "global-public",
            Quadrant::GlobalPrivate => "global-private",
            Quadrant::LocalPublic => "local-public",
            Quadrant::LocalPrivate => "local-private",
        }
    }

    fn name(self, w: &Words) -> &'static str {
        let [global_public, global_private, local_public, local_private] = w.quadrants;
        match self {
            Quadrant::GlobalPublic => global_public,
            Quadrant::GlobalPrivate => global_private,
            Quadrant::LocalPublic => local_public,
            Quadrant::LocalPrivate => local_private,
        }
    }
}

/// The words the site adds around the rules, in one language.
struct Words {
    title: &'static str,
    quadrants: [&'static str; 4],
    show_old: &'static str,
    show_off: &'static str,
    empty: &'static str,
    /// Before the name of the source at `.`, in the local section of
    /// its visibility.
    here: &'static str,
    enabled: &'static str,
    overridden_by: &'static str,
    not_enabled: &'static str,
    statement: &'static str,
    reason: &'static str,
    verification: &'static str,
    requires: &'static str,
    conflicts: &'static str,
    overrides: &'static str,
    superseded_by: &'static str,
    children: &'static str,
    index: &'static str,
    /// A one-of and a domain; a status is shown as the rule file
    /// writes it.
    kinds: [&'static str; 2],
}

fn words(lang: &str) -> Words {
    match lang {
        "zh" => Words {
            title: "本仓规定",
            quadrants: ["全局公开", "全局私有", "本仓公开", "本仓私有"],
            show_old: "显示已被取代与已废止的",
            show_off: "显示未启用的",
            empty: "没有规定源。",
            here: "本仓自己的规定就是规定源 ",
            enabled: "已启用",
            overridden_by: "被覆盖，取代它的是 ",
            not_enabled: "未启用",
            statement: "规定",
            reason: "理由",
            verification: "判定",
            requires: "依赖",
            conflicts: "冲突",
            overrides: "覆盖",
            superseded_by: "取代它的是",
            children: "下级",
            index: "全部规定",
            kinds: ["one-of", "领域"],
        },
        _ => Words {
            title: "Rules of this repository",
            quadrants: [
                "Global public",
                "Global private",
                "Local public",
                "Local private",
            ],
            show_old: "Show superseded and repealed",
            show_off: "Show rules not enabled",
            empty: "No source.",
            here: "This repository's own rules are source ",
            enabled: "enabled",
            overridden_by: "overridden by ",
            not_enabled: "not enabled",
            statement: "Statement",
            reason: "Reason",
            verification: "Verification",
            requires: "Requires",
            conflicts: "Conflicts",
            overrides: "Overrides",
            superseded_by: "Superseded by",
            children: "Below it",
            index: "All rules",
            kinds: ["one-of", "domain"],
        },
    }
}

const STYLE: &str = "body{font:16px/1.5 system-ui,sans-serif;max-width:52rem;margin:2rem auto;padding:0 1rem;color:#222}\
a{color:#05c}code{background:#f3f3f3;padding:0 .2em;border-radius:3px}\
.ref{font-family:monospace;color:#555;margin-right:.4em}\
.tag{font-size:.8em;color:#666;margin-left:.4em}.on{color:#080}\
ul.tree{list-style:none;padding-left:1.2rem}nav label{margin-right:1rem}\
section{border-top:1px solid #ddd;margin-top:1.5rem}\
.q{font-size:.8em;margin-left:.4em;padding:0 .4em;border-radius:3px;white-space:nowrap}\
.q.global-public{background:#dbeafe;color:#1e3a8a}.q.global-private{background:#fde7c8;color:#7c2d12}\
.q.local-public{background:#dcfce7;color:#14532d}.q.local-private{background:#f3e3f9;color:#581c87}\
body:has(#show-old:not(:checked)) .old,body:has(#show-off:not(:checked)) .off{display:none}";

/// One page of the site: its path below the site's directory, and its
/// HTML.
pub struct Page {
    pub path: PathBuf,
    pub html: String,
}

/// The pages of the site of `project`, the rule set with both layers when
/// there is a private one: the index, and one page per node,
/// `<source>/<id>.html`. Fails when a text cites a rule or provision that
/// is not there, or the constitution's labels cannot be read.
pub fn pages(project: &Project) -> Result<Vec<Page>, String> {
    let lang = project.manifest.lang.as_str();
    let w = words(lang);
    let labels = labels(project)?;
    let mut pages = vec![Page {
        path: PathBuf::from("index.html"),
        html: index(project, &w),
    }];
    for (key, node) in project.sources.nodes() {
        pages.push(Page {
            path: page(&key),
            html: node_page(project, &w, &labels, &key, node)?,
        });
    }
    Ok(pages)
}

/// The page of a node, relative to the site's directory.
fn page(key: &Key) -> PathBuf {
    Path::new(&key.source).join(format!("{}.html", key.id))
}

fn head(title: &str, lang: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n<style>{STYLE}</style>\n</head>\n<body>\n",
        escape(title)
    )
}

fn index(project: &Project, w: &Words) -> String {
    let lang = project.manifest.lang.as_str();
    let mut out = head(w.title, lang);
    let _ = writeln!(out, "<h1>{}</h1>\n<nav>", w.title);
    for q in QUADRANTS {
        let _ = writeln!(
            out,
            "<label><input type=\"checkbox\" id=\"show-{0}\" checked> {1}</label>",
            q.class(),
            q.name(w)
        );
    }
    let _ = writeln!(
        out,
        "<label><input type=\"checkbox\" id=\"show-off\" checked> {}</label>\n<label><input type=\"checkbox\" id=\"show-old\"> {}</label>\n</nav>",
        w.show_off, w.show_old
    );
    // The filters by quadrant, which the style sheet cannot list.
    out.push_str("<style>");
    for q in QUADRANTS {
        let _ = write!(
            out,
            "body:has(#show-{0}:not(:checked)) section.{0}{{display:none}}",
            q.class()
        );
    }
    out.push_str("</style>\n");
    // The source at `.` is global, and also this repository's own rules:
    // it is listed once, and the local section of its visibility points
    // to it.
    let here = project
        .manifest
        .sources
        .iter()
        .find(|s| s.is_here() && !s.local);
    for q in QUADRANTS {
        let _ = writeln!(
            out,
            "<section class=\"{}\">\n<h2>{}</h2>",
            q.class(),
            q.name(w)
        );
        let mut any = false;
        if let Some(spec) = here {
            let at = if spec.public {
                Quadrant::LocalPublic
            } else {
                Quadrant::LocalPrivate
            };
            if q == at {
                let _ = writeln!(
                    out,
                    "<p>{}<a href=\"#source-{1}\">{1}</a>.</p>",
                    w.here,
                    escape(&spec.name)
                );
                any = true;
            }
        }
        for source in project.sources.iter().filter(|s| Quadrant::of(s) == q) {
            any = true;
            let _ = writeln!(
                out,
                "<h3 id=\"source-{0}\">{0} <span class=\"tag\">{1}</span></h3>\n<ul class=\"tree\">",
                escape(&source.name),
                escape(&source.path)
            );
            for node in source.nodes.values().filter(|n| n.parent.is_none()) {
                tree(project, w, source, node, &mut out);
            }
            out.push_str("</ul>\n");
        }
        if !any {
            let _ = writeln!(out, "<p>{}</p>", w.empty);
        }
        out.push_str("</section>\n");
    }
    out.push_str("</body>\n</html>\n");
    out
}

/// A node and the nodes below it in its own source, as list items.
fn tree(project: &Project, w: &Words, source: &Source, node: &Node, out: &mut String) {
    let key = Key::new(&source.name, node.id);
    let mut class = Vec::new();
    if matches!(node.status, Status::Superseded | Status::Repealed) {
        class.push("old");
    }
    if node.kind == Kind::Rule && !project.resolution.enabled.contains_key(&key) {
        class.push("off");
    }
    let class = match class.as_slice() {
        [] => String::new(),
        c => format!(" class=\"{}\"", c.join(" ")),
    };
    let _ = write!(out, "<li{class}>{}", link(project, &key, ""));
    out.push_str(&tags(project, w, &key, node, ""));
    let children: Vec<&Node> = source.children(node.id).collect();
    if !children.is_empty() {
        out.push_str("\n<ul class=\"tree\">\n");
        for child in children {
            tree(project, w, source, child, out);
        }
        out.push_str("</ul>");
    }
    out.push_str("</li>\n");
}

/// A link to a node's page from a page `up` below the site's directory
/// (`""` or `"../"`): its reference and title.
fn link(project: &Project, key: &Key, up: &str) -> String {
    let title = project
        .sources
        .get(key)
        .map_or("", |n| pick(&n.title, n, &project.manifest.lang));
    format!(
        "<a href=\"{up}{}\"><span class=\"ref\">{}</span>{}</a>",
        page(key).display(),
        escape(&key.to_string()),
        escape(title)
    )
}

/// A node's quadrant, kind, status and, for a rule, what this rule set
/// does with it. The quadrant's class gives it its color, and its name
/// says it without one.
fn tags(project: &Project, w: &Words, key: &Key, node: &Node, up: &str) -> String {
    let mut out = String::new();
    if let Some(source) = project.sources.source(&key.source) {
        let q = Quadrant::of(source);
        let _ = write!(out, "<span class=\"q {}\">{}</span>", q.class(), q.name(w));
    }
    if node.kind != Kind::Rule {
        let _ = write!(out, "<span class=\"tag\">{}</span>", kind(w, node.kind));
    }
    let _ = write!(out, "<span class=\"tag\">{}</span>", status(node.status));
    if node.kind == Kind::Rule {
        let res = &project.resolution;
        let what = match res.overridden.get(key) {
            Some(by) => format!("{}{}", w.overridden_by, link(project, by, up)),
            None if res.enabled.contains_key(key) => w.enabled.to_owned(),
            None => w.not_enabled.to_owned(),
        };
        let class = if res.in_force(key) { "tag on" } else { "tag" };
        let _ = write!(out, "<span class=\"{class}\">{what}</span>");
    }
    out
}

fn kind(w: &Words, kind: Kind) -> &'static str {
    match kind {
        Kind::OneOf => w.kinds[0],
        Kind::Domain => w.kinds[1],
        Kind::Rule => "",
    }
}

fn status(status: Status) -> &'static str {
    match status {
        Status::Draft => "draft",
        Status::Trial => "trial",
        Status::Active => "active",
        Status::Superseded => "superseded",
        Status::Repealed => "repealed",
    }
}

fn node_page(
    project: &Project,
    w: &Words,
    labels: &Labels,
    key: &Key,
    node: &Node,
) -> Result<String, String> {
    let lang = project.manifest.lang.as_str();
    let title = pick(&node.title, node, lang);
    let mut out = head(&format!("{key} {title}"), lang);
    let sources = &project.sources;
    // The path from the top of the tree down to the node.
    let mut above = Vec::new();
    let mut at = sources.parent(&sources.place(key));
    while let Some(parent) = at {
        if above.contains(&parent) || sources.get(&parent).is_none() {
            break;
        }
        above.push(parent.clone());
        at = sources.parent(&sources.place(&parent));
    }
    let _ = write!(out, "<nav><a href=\"../index.html\">{}</a>", w.index);
    for parent in above.iter().rev() {
        let _ = write!(out, " / {}", link(project, parent, "../"));
    }
    let _ = writeln!(
        out,
        "</nav>\n<h1><span class=\"ref\">{}</span>{}</h1>\n<p>{}",
        escape(&key.to_string()),
        escape(title),
        tags(project, w, key, node, "../")
    );
    if let Some(slug) = &node.slug {
        let _ = write!(
            out,
            "<span class=\"tag\"><code>{}</code></span>",
            escape(slug)
        );
    }
    out.push_str("</p>\n");
    let text = |part: &str, text: &str| text_html(project, labels, key, part, text);
    if let Some(applicability) = &node.applicability {
        out.push_str(&text("applicability", pick(applicability, node, lang))?);
    }
    if let Some(rule) = &node.rule {
        for (name, heading, t) in [
            ("statement", w.statement, &rule.statement),
            ("reason", w.reason, &rule.reason),
            ("verification", w.verification, &rule.verification),
        ] {
            let _ = writeln!(out, "<h2>{heading}</h2>");
            out.push_str(&text(name, pick(t, node, lang))?);
        }
    }
    let successors = crate::resolve::successors(key, node);
    let lists = [
        (w.overrides, sources.overrides(key).into_iter().collect()),
        (w.requires, sources.requires(key)),
        (w.conflicts, sources.conflicts(key)),
        (w.superseded_by, successors),
        (w.children, sources.children(key)),
    ];
    for (heading, keys) in lists {
        if keys.is_empty() {
            continue;
        }
        let _ = writeln!(out, "<h2>{heading}</h2>\n<ul>");
        for k in &keys {
            let _ = writeln!(out, "<li>{}</li>", link(project, k, "../"));
        }
        out.push_str("</ul>\n");
    }
    out.push_str("</body>\n</html>\n");
    Ok(out)
}

/// A rule's text as HTML: paragraphs, lists whose items start with `- `,
/// and code in backticks; each citation of a rule a link to its page,
/// each of a provision its label.
fn text_html(
    project: &Project,
    labels: &Labels,
    key: &Key,
    part: &str,
    text: &str,
) -> Result<String, String> {
    let fail = |what: String| Err(format!("{}: {part}: {what}", project.sources.file(key)));
    // Escaping leaves the citations as they are: they hold no character
    // HTML treats specially.
    let text = cite::replace(&escape(text), |c| match c {
        Citation::Rule(text) => match node::cited_ref(text).map(|r| r.key(&key.source)) {
            Some(cited) if project.sources.get(&cited).is_some() => {
                Ok(link(project, &cited, "../"))
            }
            _ => fail(format!("cites `{{@rule:{text}}}`, which is not there")),
        },
        Citation::Constitution(id) => match labels.get(id) {
            Some(label) => Ok(escape(label)),
            None => fail(format!(
                "cites `{{@constitution:{id}}}`, which is not in edition {} of the constitution",
                project.manifest.edition
            )),
        },
        Citation::Bare(id) => fail(format!(
            "cites `{{@{id}}}` without `rule:` or `constitution:`"
        )),
    })?;
    let mut out = String::new();
    for block in text.split("\n\n").map(str::trim).filter(|b| !b.is_empty()) {
        if block.starts_with("- ") {
            out.push_str("<ul>\n");
            for item in block.split("\n- ") {
                let _ = writeln!(out, "<li>{}</li>", code(item.trim_start_matches("- ")));
            }
            out.push_str("</ul>\n");
        } else {
            let _ = writeln!(out, "<p>{}</p>", code(block));
        }
    }
    Ok(out)
}

/// Text between backticks as code.
fn code(text: &str) -> String {
    let mut out = String::new();
    for (i, part) in text.split('`').enumerate() {
        if i % 2 == 1 {
            let _ = write!(out, "<code>{part}</code>");
        } else {
            out.push_str(part);
        }
    }
    out
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Writes the pages to the site's directory below `root`, the directory
/// of `statute.toml`, in place of what was there, after the
/// directory's `.gitignore`. Writes nothing when git tracks a file in
/// the directory or cannot tell whether it does, or when the directory is there without statute's
/// `.gitignore`. Returns the index's path.
pub fn write(root: &Path, pages: &[Page]) -> Result<PathBuf, String> {
    let site = root.join(dir());
    let fail = |path: &Path, e: std::io::Error| format!("{}: cannot write: {e}", path.display());
    let gitignore = site.join(".gitignore");
    // The `.gitignore` keeps git from adding a page, but not from
    // tracking a file someone added already; outside a git repository
    // nothing can be committed either. Any other failure of git leaves
    // the question open, so nothing is written.
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--"])
        .arg(dir())
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| format!("cannot run git, so no page is written: {e}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    let tracked = if out.status.success() {
        String::from_utf8_lossy(&out.stdout).into_owned()
    } else if stderr.starts_with("fatal: not a git repository (") {
        String::new()
    } else {
        return Err(format!(
            "git cannot tell whether it tracks a file in {}, so no page is written: {}",
            dir().display(),
            stderr.trim()
        ));
    };
    if let Some(file) = tracked.lines().next() {
        return Err(format!(
            "{file}: git tracks it, so it would commit the pages, private rules included, and no page is written; take the directory out of git with `git rm -r --cached {}`",
            dir().display()
        ));
    }
    if site.symlink_metadata().is_ok() {
        let ours = site.symlink_metadata().is_ok_and(|m| m.is_dir())
            && std::fs::read_to_string(&gitignore).is_ok_and(|t| t == GITIGNORE);
        if !ours {
            return Err(format!(
                "{}: not written by statute, so it is left as it is and no page is written; move it away first",
                dir().display()
            ));
        }
        std::fs::remove_dir_all(&site).map_err(|e| fail(&site, e))?;
    }
    std::fs::create_dir_all(&site).map_err(|e| fail(&site, e))?;
    std::fs::write(&gitignore, GITIGNORE).map_err(|e| fail(&gitignore, e))?;
    for p in pages {
        let path = site.join(&p.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| fail(parent, e))?;
        }
        std::fs::write(&path, &p.html).map_err(|e| fail(&path, e))?;
    }
    Ok(site.join("index.html"))
}
