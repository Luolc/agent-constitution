//! The local site of a rule set: every node of its sources, the private
//! ones included, as static HTML pages in `.statute/site/`, which git
//! ignores and which a browser opens from disk. Nothing is published.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cite::{self, Citation, Labels};
use crate::compile::{filled, labels, pick};
use crate::manifest::Key;
use crate::node::{self, Kind, Node, Status, Switch};
use crate::param::{self, Origin, Scope, Type};
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
    /// The filter's button, and the words that join its rows.
    filter: &'static str,
    joins: [&'static str; 2],
    /// The fields of the filter: the kind of source, `uses`, and the
    /// nodes it shows besides.
    fields: [&'static str; 3],
    show_off: &'static str,
    show_old: &'static str,
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
    on_by_default: &'static str,
    off_by_default: &'static str,
    /// The option a one-of enables when the rule set picks none.
    default_option: &'static str,
    uses: &'static str,
    /// Where a rule goes: `AGENTS.md`, and review.
    use_names: [&'static str; 2],
    /// The label on a one-of's box.
    one_of: &'static str,
    parameters: &'static str,
    /// The columns of the page of parameters.
    param_columns: [&'static str; 5],
    origins: [&'static str; 4],
    /// Before the committed value, when the person's own differs.
    committed: &'static str,
    unused: &'static str,
}

fn words(lang: &str) -> Words {
    match lang {
        "zh" => Words {
            title: "本仓规定",
            quadrants: ["全局公开", "全局私有", "本仓公开", "本仓私有"],
            filter: "筛选",
            joins: ["当", "且"],
            fields: ["规定源的类别", "用在", "也显示"],
            show_off: "未启用的规定",
            show_old: "已被取代与已废止的",
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
            on_by_default: "默认开启",
            off_by_default: "默认关闭",
            default_option: "默认选项",
            uses: "用在",
            use_names: ["AGENTS.md", "审查"],
            one_of: "择一",
            parameters: "参数",
            param_columns: ["参数", "类型与范围", "默认值", "取值", "用到它的规定"],
            origins: ["statute.toml", "默认值", "私有规定清单", "全局配置"],
            committed: "AGENTS.md 里是 ",
            unused: "没有启用的规定用到它",
        },
        _ => Words {
            title: "Rules of this repository",
            quadrants: [
                "Global public",
                "Global private",
                "Local public",
                "Local private",
            ],
            filter: "Filter",
            joins: ["Where", "and"],
            fields: ["Kind of source", "Used in", "Also show"],
            show_off: "rules not enabled",
            show_old: "superseded and repealed",
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
            on_by_default: "on by default",
            off_by_default: "off by default",
            default_option: "default option",
            uses: "Used in",
            use_names: ["AGENTS.md", "review"],
            one_of: "One of",
            parameters: "Parameters",
            param_columns: ["Parameter", "Type and scope", "Default", "Value", "Used by"],
            origins: [
                "statute.toml",
                "the default",
                "the private rule set",
                "the global configuration",
            ],
            committed: "AGENTS.md has ",
            unused: "no enabled rule uses it",
        },
    }
}

/// The design tokens the public site uses too, then the local site's own
/// style sheet (`docs/design/frontend.md`).
const TOKENS: &str = include_str!("../assets/tokens.css");
const STYLE: &str = include_str!("site.css");

/// The Lucide icons the pages show (`assets/icons/lucide`).
const CHECK: &str = include_str!("../assets/icons/lucide/check.svg");
const CIRCLE: &str = include_str!("../assets/icons/lucide/circle.svg");
const CIRCLE_DOT: &str = include_str!("../assets/icons/lucide/circle-dot.svg");
const FUNNEL: &str = include_str!("../assets/icons/lucide/funnel.svg");

/// An icon's SVG on one line.
fn one_line(svg: &str) -> String {
    svg.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" />", "/>")
        .replace(" >", ">")
        .replace("> <", "><")
}

/// An icon inlined, in the color of its text, hidden from screen
/// readers; the class `icon` sets its size and stroke, and `class`
/// adds to it.
fn icon(svg: &str, class: &str) -> String {
    let class = if class.is_empty() {
        "icon".to_owned()
    } else {
        format!("icon {class}")
    };
    one_line(svg).replacen(
        "<svg ",
        &format!("<svg class=\"{class}\" aria-hidden=\"true\" focusable=\"false\" "),
        1,
    )
}

/// The style sheet of every page: the tokens, the site's own, and the
/// check of a check box, Lucide's `check` as a mask, drawn at the
/// stroke of every icon, `icon-stroke`, which a mask cannot take from
/// the style sheet.
fn style() -> String {
    let check = one_line(CHECK)
        .replace("stroke-width=\"2\"", "stroke-width=\"1.75\"")
        .replace('"', "'")
        .replace('<', "%3C")
        .replace('>', "%3E");
    format!("{TOKENS}\n{STYLE}\n:root {{ --icon-check: url(\"data:image/svg+xml,{check}\"); }}\n")
}

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
    for source in project.sources.iter().filter(|s| !s.params.is_empty()) {
        pages.push(Page {
            path: params_page(&source.name),
            html: parameters(project, &w, source),
        });
    }
    Ok(pages)
}

/// The page of a source's parameters, relative to the site's directory.
fn params_page(source: &str) -> PathBuf {
    Path::new(source).join("parameters.html")
}

/// The page of a source's parameters: each one's title, name, type,
/// scope, default and values, the value this rule set gives it and where
/// that comes from, and the enabled rules that use it
/// (`docs/design/rule-config.md`, Values in the lock file and the
/// outputs).
fn parameters(project: &Project, w: &Words, source: &Source) -> String {
    let lang = project.manifest.lang.as_str();
    let title = format!("{} · {}", w.parameters, source.name);
    let mut out = head(&title, lang, "text");
    let _ = writeln!(
        out,
        "<nav><a href=\"../index.html\">{}</a></nav>\n<h1>{}</h1>\n<table>\n<tr>{}</tr>",
        w.index,
        escape(&title),
        w.param_columns
            .iter()
            .map(|c| format!("<th>{c}</th>"))
            .collect::<String>()
    );
    for p in &source.params {
        let title = p
            .title
            .get(lang)
            .or_else(|| p.title.values().next())
            .map_or("", String::as_str);
        let scope = match p.scope {
            Scope::Repo => "repo",
            Scope::Personal => "personal",
        };
        let mut kind = p.kind.name().to_owned();
        if let Type::Choice(values) = &p.kind {
            let _ = write!(
                kind,
                ": {}",
                values
                    .iter()
                    .map(|v| code(&escape(&format!("`{v}`"))))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        // The setting is this source's when a rule of the rule set uses
        // the parameter of this source.
        let setting = project
            .values
            .get(&p.name)
            .filter(|s| s.source == source.name);
        let (value, rules) = match setting {
            Some(s) => {
                let mut value = format!(
                    "<code>{}</code> ({})",
                    escape(s.private_value()),
                    origin(w, s.personal.as_ref().map_or(s.from, |(_, o)| *o))
                );
                if s.personal.is_some() && s.private_value() != s.value {
                    let _ = write!(
                        value,
                        "; {}<code>{}</code> ({})",
                        w.committed,
                        escape(&s.value),
                        origin(w, s.from)
                    );
                }
                let rules: Vec<String> = s.rules.iter().map(|k| link(project, k, "../")).collect();
                (value, rules.join(", "))
            }
            None => (String::new(), w.unused.to_owned()),
        };
        let _ = writeln!(
            out,
            "<tr id=\"{name}\"><td>{}<br><code>{name}</code></td><td>{} · {scope}</td><td><code>{}</code></td><td>{value}</td><td>{rules}</td></tr>",
            escape(title),
            kind,
            escape(&p.default),
            name = escape(&p.name),
        );
    }
    out.push_str("</table>\n</main>\n</body>\n</html>\n");
    out
}

fn origin(w: &Words, from: Origin) -> &'static str {
    let [rule_set, default, private, global] = w.origins;
    match from {
        Origin::RuleSet => rule_set,
        Origin::Default => default,
        Origin::Private => private,
        Origin::Global => global,
    }
}

/// The page of a node, relative to the site's directory.
fn page(key: &Key) -> PathBuf {
    Path::new(&key.source).join(format!("{}.html", key.id))
}

/// The start of a page, up to its `main`, which takes `class`.
fn head(title: &str, lang: &str, class: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>{}</style>\n</head>\n<body>\n<main class=\"{class}\">\n",
        escape(title),
        style()
    )
}

fn index(project: &Project, w: &Words) -> String {
    let lang = project.manifest.lang.as_str();
    let mut out = head(w.title, lang, "list");
    let _ = writeln!(out, "<h1>{}</h1>", w.title);
    // The filter: a button that opens one row per field, its values as
    // check boxes, joined by and; the style sheet hides what a clear
    // box leaves out.
    let checkbox = |id: &str, checked: bool, label: &str| {
        let checked = if checked { " checked" } else { "" };
        format!("<label><input type=\"checkbox\" id=\"show-{id}\"{checked}> {label}</label>")
    };
    let fields = [
        QUADRANTS
            .iter()
            .map(|q| checkbox(q.class(), true, q.name(w)))
            .collect::<String>(),
        checkbox("instructions", true, w.use_names[0]) + &checkbox("review", true, w.use_names[1]),
        checkbox("off", true, w.show_off) + &checkbox("old", false, w.show_old),
    ];
    let _ = write!(
        out,
        "<details class=\"filter\"><summary>{}{}</summary>\n<div class=\"panel\">\n",
        icon(FUNNEL, ""),
        w.filter
    );
    for (i, (field, values)) in w.fields.iter().zip(&fields).enumerate() {
        let _ = writeln!(
            out,
            "<div class=\"cond\" role=\"group\" aria-label=\"{field}\"><span class=\"conn\">{}</span><span class=\"field\">{field}</span><span class=\"values\">{values}</span></div>",
            w.joins[usize::from(i > 0)]
        );
    }
    out.push_str("</div></details>\n");
    let _ = writeln!(
        out,
        "<p class=\"legend\"><span class=\"switch on\">{}</span><span class=\"switch off\">{}</span></p>",
        w.on_by_default, w.off_by_default
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
            // As in the compiled `AGENTS.md`: a git source by its address
            // and commit, not the snapshot directory it is read from.
            let place = match project.manifest.spec(&source.name) {
                Some(spec) => spec.place(),
                None => source.path.clone(),
            };
            let params = if source.params.is_empty() {
                String::new()
            } else {
                format!(
                    "<a class=\"params\" href=\"{}\">{}</a>",
                    params_page(&source.name).display(),
                    w.parameters
                )
            };
            let _ = writeln!(
                out,
                "<h3 id=\"source-{0}\">{0}<span class=\"place\">{1}</span>{params}</h3>\n<ul class=\"tree\">",
                escape(&source.name),
                escape(&place)
            );
            for node in source.nodes.values().filter(|n| n.parent.is_none()) {
                tree(project, w, source, node, 1, "", &mut out);
            }
            out.push_str("</ul>\n");
        }
        if !any {
            let _ = writeln!(out, "<p>{}</p>", w.empty);
        }
        out.push_str("</section>\n");
    }
    out.push_str("</main>\n</body>\n</html>\n");
    out
}

/// A node and the nodes below it in its own source, as list items, each
/// a row: `lead`, the icon in front of its label, its link and its
/// tags. A domain's `level` (1 at the top) sets its heading and its bar
/// (`docs/design/frontend.md`, Components, The tree of rules).
fn tree(
    project: &Project,
    w: &Words,
    source: &Source,
    node: &Node,
    level: usize,
    lead: &str,
    out: &mut String,
) {
    let key = Key::new(&source.name, node.id);
    let children: Vec<&Node> = source.children(node.id).collect();
    let mut class = Vec::new();
    match node.kind {
        // Domains nest two levels at most.
        Kind::Domain => class.push(if level == 1 { "domain d1" } else { "domain d2" }),
        // A one-of is a box that holds its options, the default marked
        // (`docs/design/rule-config.md`, No rule under a rule), lighter
        // when the rule set enables none of them.
        Kind::OneOf => {
            class.push("one-of");
            let enabled = &project.resolution.enabled;
            if !children
                .iter()
                .any(|c| enabled.contains_key(&Key::new(&source.name, c.id)))
            {
                class.push("unset");
            }
        }
        Kind::Rule => {}
    }
    if matches!(node.status, Status::Superseded | Status::Repealed) {
        class.push("old");
    }
    if node.kind == Kind::Rule && !project.resolution.enabled.contains_key(&key) {
        class.push("off");
    }
    if let Some(rule) = &node.rule {
        class.push(match (rule.uses.instructions, rule.uses.review) {
            (true, true) => "u-ir",
            (false, true) => "u-r",
            _ => "u-i",
        });
    }
    let class = match class.as_slice() {
        [] => String::new(),
        c => format!(" class=\"{}\"", c.join(" ")),
    };
    let mut title = link(project, &key, "");
    if node.kind == Kind::Domain {
        let h = 3 + level.min(2);
        title = format!("<h{h}>{title}</h{h}>");
    }
    let _ = write!(out, "<li{class}>");
    if node.kind == Kind::OneOf {
        let _ = write!(out, "<span class=\"one-of-label\">{}</span>", w.one_of);
    }
    let _ = write!(
        out,
        "<div class=\"row\">{lead}{title}<span class=\"tags\">{}{}</span></div>",
        tags(project, w, &key, node, "", true),
        mark(project, w, &key, false)
    );
    if !children.is_empty() {
        out.push_str("\n<ul class=\"tree\">\n");
        for child in children {
            // An option of a one-of is led by a circle, the default by
            // one with a dot.
            let lead = match node.kind {
                Kind::OneOf if node.default_child == Some(child.id) => icon(CIRCLE_DOT, "default"),
                Kind::OneOf => icon(CIRCLE, ""),
                _ => String::new(),
            };
            let level = if node.kind == Kind::Domain {
                level + 1
            } else {
                level
            };
            tree(project, w, source, child, level, &lead, out);
        }
        out.push_str("</ul>");
    }
    out.push_str("</li>\n");
}

/// A link to a node's page from a page `up` below the site's directory
/// (`""` or `"../"`): its reference and title.
fn link(project: &Project, key: &Key, up: &str) -> String {
    let title = project.sources.get(key).map_or(String::new(), |n| {
        filled(
            project,
            key,
            pick(&n.title, n, &project.manifest.lang),
            true,
        )
    });
    format!(
        "<a href=\"{up}{}\"><span class=\"ref\">{}</span><span class=\"title\">{}</span></a>",
        page(key).display(),
        escape(&key.to_string()),
        escape(&title)
    )
}

/// The quadrant of a node's source, and with `named` the source's name:
/// the class gives the mark its color, and the words say it without one.
fn mark(project: &Project, w: &Words, key: &Key, named: bool) -> String {
    let Some(source) = project.sources.source(&key.source) else {
        return String::new();
    };
    let q = Quadrant::of(source);
    let name = if named {
        format!(" · {}", escape(&source.name))
    } else {
        String::new()
    };
    format!("<span class=\"q {}\">{}{name}</span>", q.class(), q.name(w))
}

/// A node's status and, for a rule, what this rule set does with it;
/// unless `dense`, as in the tree, also its kind. Each fact has its own
/// form (`docs/design/frontend.md`, Components, Tags): the status a
/// pill, a default switch a dot, which in the tree stands alone with
/// its word as its name.
fn tags(project: &Project, w: &Words, key: &Key, node: &Node, up: &str, dense: bool) -> String {
    let mut out = String::new();
    if node.kind != Kind::Rule && !dense {
        let _ = write!(out, "<span class=\"kind\">{}</span>", kind(w, node.kind));
    }
    let status = status(node.status);
    let _ = write!(out, "<span class=\"status {status}\">{status}</span>");
    match switch(project, w, key, node) {
        Some(Mark::Switch(class, word)) if dense => {
            let _ = write!(
                out,
                "<span class=\"switch {class}\" role=\"img\" aria-label=\"{word}\" title=\"{word}\"></span>"
            );
        }
        Some(Mark::Switch(class, word)) => {
            let _ = write!(out, "<span class=\"switch {class}\">{word}</span>");
        }
        Some(Mark::DefaultOption(word)) => {
            let _ = write!(
                out,
                "<span class=\"default-option\">{}{word}</span>",
                icon(CHECK, "")
            );
        }
        None => {}
    }
    if node.kind == Kind::Rule {
        let res = &project.resolution;
        let what = match res.overridden.get(key) {
            Some(by) => format!("{}{}", w.overridden_by, link(project, by, up)),
            None if res.enabled.contains_key(key) => w.enabled.to_owned(),
            None => w.not_enabled.to_owned(),
        };
        if res.in_force(key) {
            let _ = write!(
                out,
                "<span class=\"enabled on\">{}{what}</span>",
                icon(CHECK, "")
            );
        } else {
            let _ = write!(out, "<span class=\"enabled\">{what}</span>");
        }
    }
    out
}

/// How a node shows what the defaults do with it.
enum Mark {
    /// On or off by default: the class of its dot, and the words.
    Switch(&'static str, &'static str),
    /// The default option of its one-of.
    DefaultOption(&'static str),
}

/// Whether a domain or a rule is on by default, by its own `default`
/// (`docs/design/rule-config.md`, Default switches), or for an option of
/// a one-of whether it is the one-of's default.
fn switch(project: &Project, w: &Words, key: &Key, node: &Node) -> Option<Mark> {
    match node.default {
        Some(Switch::On) => Some(Mark::Switch("on", w.on_by_default)),
        Some(Switch::Off) => Some(Mark::Switch("off", w.off_by_default)),
        None => {
            let parent = project.sources.parent(key)?;
            let one_of = project.sources.get(&parent)?;
            (one_of.kind == Kind::OneOf && one_of.default_child == Some(node.id))
                .then_some(Mark::DefaultOption(w.default_option))
        }
    }
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
    let title = filled(project, key, pick(&node.title, node, lang), true);
    let title = title.as_str();
    let mut out = head(&format!("{key} {title}"), lang, "text");
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
        "</nav>\n<h1><span class=\"ref\">{}</span>{}</h1>\n<p class=\"tag-line\">{}{}",
        escape(&key.to_string()),
        escape(title),
        mark(project, w, key, true),
        tags(project, w, key, node, "../", false)
    );
    if let Some(slug) = &node.slug {
        let _ = write!(out, "<code class=\"slug\">{}</code>", escape(slug));
    }
    out.push_str("</p>\n");
    if let Some(rule) = &node.rule {
        let uses: Vec<&str> = [rule.uses.instructions, rule.uses.review]
            .into_iter()
            .zip(w.use_names)
            .filter_map(|(on, name)| on.then_some(name))
            .collect();
        let _ = writeln!(out, "<p>{}: {}</p>", w.uses, uses.join(", "));
    }
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
    out.push_str("</main>\n</body>\n</html>\n");
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
    // Escaping leaves the citations and the placeholders as they are:
    // they hold no character HTML treats specially. A placeholder shows
    // the value, marked, with the parameter's name on hover, linked to
    // its entry on the page of parameters.
    let text = param::fill(&escape(text), |name| {
        let lang = project.manifest.lang.as_str();
        let (source, shown) = match project.values.get(name) {
            Some(s) => (s.source.clone(), s.param.shown(s.private_value(), lang)),
            None => match project.sources.param(&key.source, name) {
                Ok((source, p)) => (source.to_owned(), p.shown(&p.default, lang)),
                Err(_) => return format!("{{${name}}}"),
            },
        };
        format!(
            "<a class=\"value\" href=\"../{}#{name}\" title=\"{name}\">{}</a>",
            params_page(&source).display(),
            escape(&shown)
        )
    });
    let text = cite::replace(&text, |c| match c {
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
/// of `statute.toml`, in place of what was there, after
/// `.statute/.gitignore` and the directory's own. Writes nothing when git tracks a file in
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
    let there = site.symlink_metadata().is_ok();
    if there {
        let ours = site.symlink_metadata().is_ok_and(|m| m.is_dir())
            && std::fs::read_to_string(&gitignore).is_ok_and(|t| t == GITIGNORE);
        if !ours {
            return Err(format!(
                "{}: not written by statute, so it is left as it is and no page is written; move it away first",
                dir().display()
            ));
        }
    }
    crate::private::write_gitignore(root)?;
    if there {
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
