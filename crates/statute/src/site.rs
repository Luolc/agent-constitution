//! The local site of a rule set: every node of its sources, the private
//! ones included, as static HTML pages in `.statute/site/`, which git
//! ignores and which a browser opens from disk. Nothing is published.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cite::{self, Citation, Labels};
use crate::compile::{filled, labels, pick};
use crate::manifest::Key;
use crate::node::{self, Kind, Node, Status, Switch, Text};
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
    /// The language's name in itself, on the switch to it.
    name: &'static str,
    /// On a node's page when a text of the node is not in the page's
    /// language, and shown in the node's own.
    untranslated: &'static str,
    title: &'static str,
    quadrants: [&'static str; 4],
    /// The lines of the legend: the status, the defaults, `uses`, the
    /// one-of, this rule set and the places of the marks.
    legend: [&'static str; 6],
    /// The legend's own name, on the button that folds it.
    legend_name: &'static str,
    /// What the places of the marks hold, in order.
    slots: &'static str,
    /// After each use's name in the legend, what it means.
    use_meanings: [&'static str; 2],
    /// In the legend, what a one-of is, and an option not its default.
    one_of_meaning: &'static str,
    another_option: &'static str,
    /// The switch between the views of the rows, and each view: title,
    /// statement.
    view: &'static str,
    views: [&'static str; 2],
    /// The head of a table: the label and the rule.
    columns: [&'static str; 2],
    /// The menus of the filter, in the order of `FIELDS`.
    fields: [&'static str; 6],
    /// In a menu: choose every value, and none; search the domains.
    all: &'static str,
    /// What a menu shows when it chooses every value, and none.
    every: &'static str,
    nothing: &'static str,
    none: &'static str,
    search: &'static str,
    /// The search of the rows: its name and what it searches.
    find: &'static str,
    find_placeholder: &'static str,
    /// The button that brings back the default filter, at the end of the
    /// row, and the number of rules shown, `{}` the number.
    clear: &'static str,
    shown: &'static str,
    /// When the filter leaves no row, and the button that resets it.
    no_match: &'static str,
    reset: &'static str,
    empty: &'static str,
    /// Before and after the name of the source at `.`, in the local
    /// section of its visibility.
    here: [&'static str; 2],
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
    /// Between the items of a list.
    and: &'static str,
    /// Between a git source's address and its commit.
    at: &'static str,
    unused: &'static str,
}

fn words(lang: &str) -> Words {
    match lang {
        "zh" => Words {
            name: "中文",
            untranslated: "无中文",
            title: "本仓规定",
            quadrants: ["全局公开", "全局私有", "本仓公开", "本仓私有"],
            legend: ["默认", "状态", "用途", "择一", "规定清单", "行末"],
            legend_name: "图例",
            use_meanings: ["：写进 agent 的指令", "：审查方按它审查"],
            slots: "依次是默认、状态、指令、审查，没有的留空",
            one_of_meaning: "规定清单恰好取其中一个选项",
            another_option: "其他选项",
            view: "显示",
            views: ["短标题", "要求"],
            columns: ["编号", "规定"],
            fields: ["状态", "默认", "用途", "领域", "象限", "规定清单"],
            all: "全选",
            every: "全部",
            nothing: "无",
            none: "不选",
            search: "搜索领域",
            find: "搜索",
            find_placeholder: "编号、短标题或要求",
            clear: "清除筛选",
            shown: "{} 条规定",
            no_match: "没有符合筛选的规定。",
            reset: "重置筛选",
            empty: "没有规定源。",
            here: ["本仓自己的规定就是规定源 ", "。"],
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
            uses: "用在：",
            use_names: ["AGENTS.md", "审查"],
            one_of: "择一",
            parameters: "参数",
            param_columns: ["参数", "类型与范围", "默认值", "取值", "用到它的规定"],
            origins: ["statute.toml", "默认值", "私有规定清单", "全局配置"],
            committed: "；AGENTS.md 里是 ",
            and: "、",
            at: "，提交 ",
            unused: "没有启用的规定用到它",
        },
        _ => Words {
            name: "English",
            untranslated: "Not translated",
            title: "Rules of this repository",
            quadrants: [
                "Global public",
                "Global private",
                "Local public",
                "Local private",
            ],
            legend: ["Default", "Status", "Uses", "One of", "Rule set", "Row end"],
            legend_name: "Legend",
            use_meanings: [
                ": goes into the agent's instructions",
                ": a reviewer checks it",
            ],
            slots: "default, status, instructions, review, in this order; empty where a rule has none",
            one_of_meaning: "the rule set takes exactly one option",
            another_option: "Another option",
            view: "Show",
            views: ["Short title", "Statement"],
            columns: ["ID", "Rule"],
            fields: [
                "Status", "Default", "Uses", "Domain", "Quadrant", "Rule set",
            ],
            all: "All",
            every: "All",
            nothing: "None",
            none: "None",
            search: "Search domains",
            find: "Search",
            find_placeholder: "ID, short title or statement",
            clear: "Clear filters",
            shown: "{} rules",
            no_match: "No rule matches the filter.",
            reset: "Reset filter",
            empty: "No source.",
            here: ["This repository's own rules are source ", "."],
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
            uses: "Used in: ",
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
            committed: "; AGENTS.md has ",
            and: ", ",
            at: " at ",
            unused: "no enabled rule uses it",
        },
    }
}

/// A language of the site: its code, its words, the constitution's labels
/// in it, and the directory of its pages below the site's, `""` or
/// `"<code>/"`.
struct Lang<'a> {
    code: &'a str,
    w: Words,
    labels: Labels,
    dir: String,
}

/// The design tokens the public site uses too, then the local site's own
/// style sheet (`docs/design/frontend.md`).
const TOKENS: &str = include_str!("../assets/tokens.css");
const STYLE: &str = include_str!("site.css");

/// The Lucide icons the pages show (`assets/icons/lucide`).
const ARROW_RIGHT_LEFT: &str = include_str!("../assets/icons/lucide/arrow-right-left.svg");
const CHECK: &str = include_str!("../assets/icons/lucide/check.svg");
const CHEVRON_DOWN: &str = include_str!("../assets/icons/lucide/chevron-down.svg");
const CHEVRON_RIGHT: &str = include_str!("../assets/icons/lucide/chevron-right.svg");
const CIRCLE: &str = include_str!("../assets/icons/lucide/circle.svg");
const CIRCLE_CHECK: &str = include_str!("../assets/icons/lucide/circle-check.svg");
const CIRCLE_DOT: &str = include_str!("../assets/icons/lucide/circle-dot.svg");
const CIRCLE_X: &str = include_str!("../assets/icons/lucide/circle-x.svg");
const CORNER_DOWN_RIGHT: &str = include_str!("../assets/icons/lucide/corner-down-right.svg");
const FLASK_CONICAL: &str = include_str!("../assets/icons/lucide/flask-conical.svg");
const LANGUAGES: &str = include_str!("../assets/icons/lucide/languages.svg");
const PENCIL: &str = include_str!("../assets/icons/lucide/pencil.svg");
const SCAN_EYE: &str = include_str!("../assets/icons/lucide/scan-eye.svg");
const SCROLL_TEXT: &str = include_str!("../assets/icons/lucide/scroll-text.svg");

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
/// there is a private one, in two languages: at the top in the language
/// of `[compile] lang`, and in `zh/`, or `en/` when that language is
/// Chinese, in the other. In each, the index, one page per node,
/// `<source>/<id>.html`, and the parameters of each source that has
/// them. Fails when a text cites a rule or provision that is not there,
/// or the constitution's labels cannot be read.
pub fn pages(project: &Project) -> Result<Vec<Page>, String> {
    let first = project.manifest.lang.as_str();
    let second = if first == "zh" { "en" } else { "zh" };
    let lang = |code, dir: &str| -> Result<Lang, String> {
        Ok(Lang {
            code,
            w: words(code),
            labels: labels(project, code)?,
            dir: dir.to_owned(),
        })
    };
    let langs = [lang(first, "")?, lang(second, &format!("{second}/"))?];
    let mut pages = Vec::new();
    for (l, other) in [(&langs[0], &langs[1]), (&langs[1], &langs[0])] {
        let at = |path: &Path| Path::new(&l.dir).join(path);
        let path = PathBuf::from("index.html");
        pages.push(Page {
            html: index(project, l, &langs, &languages(l, other, &path))?,
            path: at(&path),
        });
        for (key, node) in project.sources.nodes() {
            let path = page(&key);
            pages.push(Page {
                html: node_page(project, l, &langs, &key, node, &languages(l, other, &path))?,
                path: at(&path),
            });
        }
        for source in project.sources.iter().filter(|s| !s.params.is_empty()) {
            let path = params_page(&source.name);
            pages.push(Page {
                html: parameters(project, l, source, &languages(l, other, &path)),
                path: at(&path),
            });
        }
    }
    Ok(pages)
}

/// The switch from the page at `path` in `from` to the same page in
/// `to`: `languages` and the name of `to`, in `to`.
fn languages(from: &Lang, to: &Lang, path: &Path) -> String {
    let depth = path.components().count() - 1 + usize::from(!from.dir.is_empty());
    format!(
        "<a class=\"languages\" href=\"{}{}{}\" hreflang=\"{code}\" lang=\"{code}\">{}{}</a>",
        "../".repeat(depth),
        to.dir,
        path.display(),
        icon(LANGUAGES, ""),
        to.w.name,
        code = to.code,
    )
}

/// ` lang="<text>"` when a text in `text` stands in a page or an element
/// in `around`, else nothing.
fn lang_attr(text: &str, around: &str) -> String {
    if text == around {
        String::new()
    } else {
        format!(" lang=\"{}\"", escape(text))
    }
}

/// `html`, in `text`, in a `span` that says so when it stands in
/// `around`, another language.
fn in_lang(html: &str, text: &str, around: &str) -> String {
    if text == around {
        html.to_owned()
    } else {
        format!("<span{}>{html}</span>", lang_attr(text, around))
    }
}

/// A node's `text` in `lang` or, when it has none there, in the node's
/// own language, and the language it is in.
fn pick_lang<'a>(text: &'a Text, node: &'a Node, lang: &'a str) -> (&'a str, &'a str) {
    let shown = if text.contains_key(lang) {
        lang
    } else {
        node.lang.as_str()
    };
    (pick(text, node, lang), shown)
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
fn parameters(project: &Project, l: &Lang, source: &Source, switch: &str) -> String {
    let (w, lang) = (&l.w, l.code);
    let title = format!("{} · {}", w.parameters, source.name);
    let mut out = head(&title, lang, "list", switch);
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
        let (title, title_lang) = p
            .title
            .get_key_value(lang)
            .or_else(|| p.title.iter().next())
            .map_or(("", lang), |(k, v)| (v.as_str(), k.as_str()));
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
                    .join(w.and)
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
                        "{}<code>{}</code> ({})",
                        w.committed,
                        escape(&s.value),
                        origin(w, s.from)
                    );
                }
                let rules: Vec<String> = s
                    .rules
                    .iter()
                    .map(|k| link(project, k, "../", lang))
                    .collect();
                (value, rules.join(w.and))
            }
            None => (String::new(), w.unused.to_owned()),
        };
        let _ = writeln!(
            out,
            "<tr id=\"{name}\"><td>{}<br><code>{name}</code></td><td>{} · {scope}</td><td><code>{}</code></td><td>{value}</td><td>{rules}</td></tr>",
            in_lang(&escape(title), title_lang, lang),
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

/// The start of a page, up to its `main`, which takes `class`, and the
/// switch to the other language at the top of it.
fn head(title: &str, lang: &str, class: &str, switch: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>{}</style>\n</head>\n<body>\n<main class=\"{class}\">\n{switch}\n",
        escape(title),
        style()
    )
}

/// The fields of the index's filter, in the order of their menus: each
/// one's name in the address and in its items' `data-` attribute.
const FIELDS: [&str; 6] = ["status", "default", "uses", "domain", "quadrant", "enabled"];

const STATUSES: [Status; 5] = [
    Status::Active,
    Status::Trial,
    Status::Draft,
    Status::Superseded,
    Status::Repealed,
];

/// The ways the index shows a rule, its value in the address and the
/// radio button's: its title, its statement.
const VIEWS: [&str; 2] = ["title", "statement"];

/// A row of the index: a rule or a one-of, with its options, and the
/// domains above it, outermost first; `under` when its parent is a rule.
struct Item<'a> {
    node: &'a Node,
    groups: Vec<&'a Node>,
    under: bool,
}

/// The rows of a node and of the nodes below it in its own source, in
/// the order of the tree: a domain is no row, but the group of those
/// below it; a one-of's options are in its row, and what stands below
/// them follows it.
fn items<'a>(
    source: &'a Source,
    node: &'a Node,
    groups: &[&'a Node],
    under: bool,
    out: &mut Vec<Item<'a>>,
) {
    match node.kind {
        Kind::Domain => {
            let mut inner = groups.to_vec();
            inner.push(node);
            for child in source.children(node.id) {
                items(source, child, &inner, false, out);
            }
        }
        Kind::OneOf => {
            out.push(Item {
                node,
                groups: groups.to_vec(),
                under,
            });
            for option in source.children(node.id) {
                if option.kind == Kind::Rule {
                    for child in source.children(option.id) {
                        items(source, child, groups, true, out);
                    }
                } else {
                    items(source, option, groups, false, out);
                }
            }
        }
        Kind::Rule => {
            out.push(Item {
                node,
                groups: groups.to_vec(),
                under,
            });
            for child in source.children(node.id) {
                items(source, child, groups, true, out);
            }
        }
    }
}

/// The domains of a source in the order of the tree, each with its depth
/// and the nearest domain above it.
fn domains<'a>(
    source: &'a Source,
    node: &'a Node,
    above: Option<&'a Node>,
    depth: usize,
    out: &mut Vec<(&'a Node, Option<&'a Node>, usize)>,
) {
    let (above, depth) = if node.kind == Kind::Domain {
        out.push((node, above, depth));
        (Some(node), depth + 1)
    } else {
        (above, depth)
    };
    for child in source.children(node.id) {
        domains(source, child, above, depth, out);
    }
}

/// The script of the index: the filter and the view in the address, All
/// and None, the counts, the domains below one checked or cleared with
/// it, the search, the menus closing, the legend kept folded or open,
/// and the groups the filter empties hidden.
const SCRIPT: &str = include_str!("site.js");

/// The script as the index inlines it, without its comments, which are
/// for its readers in the repository.
fn script() -> String {
    SCRIPT
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// What the rows of the index are drawn from: the project, the page's
/// language and every language of the site.
struct Site<'a> {
    project: &'a Project,
    l: &'a Lang<'a>,
    langs: &'a [Lang<'a>],
}

fn index(project: &Project, l: &Lang, langs: &[Lang], switch: &str) -> Result<String, String> {
    let site = Site { project, l, langs };
    let w = &l.w;
    let mut out = head(w.title, l.code, "list", switch);
    let _ = writeln!(out, "<h1>{}</h1>", w.title);
    out.push_str(&legend(w));
    out.push_str(&controls(project, l));
    let _ = writeln!(
        out,
        "<p class=\"none\" hidden>{} <button type=\"button\" class=\"reset\">{}</button></p>",
        w.no_match, w.reset
    );
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
                    "<p>{}<a href=\"#source-{1}\">{1}</a>{2}</p>",
                    w.here[0],
                    escape(&spec.name),
                    w.here[1]
                );
                any = true;
            }
        }
        for source in project.sources.iter().filter(|s| Quadrant::of(s) == q) {
            any = true;
            table(&site, source, &mut out)?;
        }
        if !any {
            let _ = writeln!(out, "<p>{}</p>", w.empty);
        }
        out.push_str("</section>\n");
    }
    let _ = write!(
        out,
        "</main>\n<script>{}</script>\n</body>\n</html>\n",
        script()
    );
    Ok(out)
}

/// A source in the index: its name, where it is read from and the link
/// to its parameters, then the table of its rules under a head, in
/// blocks of the rows of one group, each led by the group's row
/// (`docs/design/frontend.md`, Components, The table of rules).
fn table(site: &Site, source: &Source, out: &mut String) -> Result<(), String> {
    let (project, w) = (site.project, &site.l.w);
    // As in the compiled `AGENTS.md`: a git source by its address and
    // commit, not the snapshot directory it is read from.
    let place = match project.manifest.spec(&source.name) {
        Some(spec) => match &spec.git {
            Some(git) => format!(
                "{}{}{}",
                git.address,
                w.at,
                git.commit.as_deref().unwrap_or("")
            ),
            None => spec.path.clone(),
        },
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
        "<div class=\"source\">\n<h3 id=\"source-{0}\">{0}<span class=\"place\">{1}</span>{params}</h3>\n<div class=\"table\">\n<div class=\"head\" aria-hidden=\"true\"><span class=\"ref\">{2}</span><span>{3}</span></div>",
        escape(&source.name),
        escape(&place),
        w.columns[0],
        w.columns[1]
    );
    let mut rows = Vec::new();
    for node in source.nodes.values().filter(|n| n.parent.is_none()) {
        items(source, node, &[], false, &mut rows);
    }
    let mut previous: Option<&[&Node]> = None;
    for item in &rows {
        if !previous.is_some_and(|p| same(p, &item.groups)) {
            if previous.is_some() {
                out.push_str("</div>\n");
            }
            out.push_str("<div class=\"block\">\n");
            out.push_str(&group(site, source, &item.groups));
            previous = Some(&item.groups);
        }
        out.push_str(&row(site, source, item)?);
    }
    if previous.is_some() {
        out.push_str("</div>\n");
    }
    out.push_str("</div>\n</div>\n");
    Ok(())
}

fn same(a: &[&Node], b: &[&Node]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.id == y.id)
}

/// The row of a group: the innermost domain's label, the path from the
/// source, its quadrant and name, through the domains, the whole a link
/// to the innermost domain's page, and in the first place of the marks
/// whether that domain is on by default; a band, the darker for a
/// group of the first level.
fn group(site: &Site, source: &Source, groups: &[&Node]) -> String {
    let (project, l) = (site.project, site.l);
    let chevron = icon(CHEVRON_RIGHT, "");
    let mut path = quadrant_tag(&l.w, source);
    let mut names = Vec::new();
    for domain in groups {
        let key = Key::new(&source.name, domain.id);
        let (title, title_lang) = pick_lang(&domain.title, domain, l.code);
        let title = filled(project, &key, title, true, title_lang);
        let _ = write!(
            path,
            "{chevron}<span class=\"group-part\"{}>{}</span>",
            lang_attr(title_lang, l.code),
            escape(&title)
        );
        names.push(title);
    }
    // Under the source's `h3`, one level deeper per group of the path, to
    // `h6` at most.
    let level = (3 + groups.len()).min(6);
    let band = if groups.len() > 1 { "deep" } else { "top" };
    let dot = groups.last().map_or_else(
        || "<span class=\"mark\"></span>".to_owned(),
        |d| dot(&l.w, d),
    );
    let marks = format!(
        "<span class=\"marks\">{dot}<span class=\"mark\"></span><span class=\"mark\"></span><span class=\"mark\"></span></span>"
    );
    match groups.last() {
        Some(domain) => {
            let key = Key::new(&source.name, domain.id);
            format!(
                "<h{level} class=\"group {band}\"><a href=\"{}\" aria-label=\"{} {}\"><span class=\"ref\">{}</span><span class=\"group-path\">{path}</span></a>{marks}</h{level}>\n",
                page(&key).display(),
                escape(&key.to_string()),
                escape(&names.join(", ")),
                escape(&key.to_string()),
            )
        }
        None => format!(
            "<h{level} class=\"group {band}\"><span class=\"ref\"></span><span class=\"group-path\">{path}</span>{marks}</h{level}>\n"
        ),
    }
}

/// A rule's or a one-of's row, its values for the filter in `data-`
/// attributes and its text, for the search, in `data-text`; a one-of a
/// box of its own row and its options'.
fn row(site: &Site, source: &Source, item: &Item) -> Result<String, String> {
    let project = site.project;
    let node = item.node;
    let key = Key::new(&source.name, node.id);
    let options: Vec<&Node> = if node.kind == Kind::OneOf {
        source.children(node.id).collect()
    } else {
        Vec::new()
    };
    // A one-of is used as its options are.
    let uses = |n: &Node| {
        n.rule
            .as_ref()
            .map(|r| (r.uses.instructions, r.uses.review))
    };
    let (instructions, review) = uses(node).unwrap_or_else(|| {
        options
            .iter()
            .filter_map(|o| uses(o))
            .fold((false, false), |(i, r), (oi, or)| (i || oi, r || or))
    });
    let names: Vec<&str> = [(instructions, "instructions"), (review, "review")]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name))
        .collect();
    let enabled = match node.kind {
        Kind::OneOf => in_use(project, source, node),
        _ => project.resolution.enabled.contains_key(&key),
    };
    let domain = item
        .groups
        .last()
        .map(|d| Key::new(&source.name, d.id).to_string())
        .unwrap_or_default();
    let mut out = format!(
        "<div class=\"item\" data-status=\"{}\" data-default=\"{}\" data-uses=\"{}\" data-domain=\"{}\" data-enabled=\"{}\">",
        status(node.status),
        on_off(node.default == Some(Switch::On)),
        names.join(" "),
        escape(&domain),
        on_off(enabled)
    );
    let lead = if item.under {
        icon(CORNER_DOWN_RIGHT, "")
    } else {
        String::new()
    };
    let own = entry(site, &key, node, &lead, (instructions, review))?;
    if node.kind != Kind::OneOf {
        out.push_str(&own);
        out.push_str("</div>\n");
        return Ok(out);
    }
    let id = format!("one-of-{}-{}", escape(&source.name), node.id);
    let unset = if in_use(project, source, node) {
        ""
    } else {
        " unset"
    };
    let _ = write!(
        out,
        "<div class=\"one-of{unset}\" role=\"group\" aria-labelledby=\"{id}\"><span class=\"one-of-label\" id=\"{id}\">{}</span>{own}<ul class=\"options\">",
        site.l.w.one_of
    );
    for option in options {
        let default = node.default_child == Some(option.id);
        let (class, lead) = if default {
            (" class=\"default\"", icon(CIRCLE_DOT, "default"))
        } else {
            ("", icon(CIRCLE, ""))
        };
        let uses = uses(option).unwrap_or((false, false));
        let option_key = Key::new(&source.name, option.id);
        let _ = write!(
            out,
            "<li{class}>{}</li>",
            entry(site, &option_key, option, &lead, uses)?
        );
    }
    out.push_str("</ul></div></div>\n");
    Ok(out)
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

/// The cells of a row: the label; `lead`, the title, with its whole text
/// on hover, and for a rule its statement; the four places of the marks;
/// and for a rule whether the rule set enables it. The title's link
/// covers the row.
fn entry(
    site: &Site,
    key: &Key,
    node: &Node,
    lead: &str,
    uses: (bool, bool),
) -> Result<String, String> {
    let (project, l) = (site.project, site.l);
    let w = &l.w;
    let (title, title_lang) = pick_lang(&node.title, node, l.code);
    let title = escape(&filled(project, key, title, true, title_lang));
    let default_option = match switch(project, w, key, node) {
        Some(mark @ Mark::DefaultOption(_)) => mark.tag(),
        _ => String::new(),
    };
    let (class, statement) = match &node.rule {
        Some(rule) => (
            "entry has-statement",
            statement(site, key, node, &rule.statement)?,
        ),
        None => ("entry", String::new()),
    };
    let enabled = if node.kind == Kind::Rule {
        enabled(project, l, key, "")
    } else {
        String::new()
    };
    Ok(format!(
        "<div class=\"{class}\"><span class=\"ref\">{}</span><span class=\"rule-title\">{lead}<span class=\"rule-main\"><span class=\"rule-head\"><a class=\"rule-text\" href=\"{}\"><span class=\"title\"{} title=\"{title}\">{title}</span></a>{default_option}</span>{statement}</span></span>{}<span class=\"rule-enabled\">{enabled}</span></div>",
        escape(&key.to_string()),
        page(key).display(),
        lang_attr(title_lang, l.code),
        marks(w, node, uses)
    ))
}

/// A rule's statement in the index, in the page's language or, marked
/// with `languages`, in the rule's own; its placeholders filled with the
/// values the compiled `AGENTS.md` has. Fails when it cites a rule or
/// provision that is not there.
fn statement(site: &Site, key: &Key, node: &Node, text: &Text) -> Result<String, String> {
    let l = site.l;
    let (text, text_lang) = pick_lang(text, node, l.code);
    let labels = site
        .langs
        .iter()
        .find(|l| l.code == text_lang)
        .map_or(&l.labels, |l| &l.labels);
    let html = text_html(site.project, labels, key, "statement", text, text_lang, "")?;
    if text_lang == l.code {
        return Ok(format!("<div class=\"statement\">\n{html}</div>"));
    }
    Ok(format!(
        "<div class=\"statement\"{}>\n<span class=\"untranslated\" role=\"img\" aria-label=\"{2}\" title=\"{2}\" lang=\"{3}\">{}</span>{html}</div>",
        lang_attr(text_lang, l.code),
        icon(LANGUAGES, ""),
        l.w.untranslated,
        l.code
    ))
}

/// Whether a domain or a rule is on by default, by its own `default`, as
/// a mark: a dot, filled or a ring, named by its words, or an empty place.
fn dot(w: &Words, node: &Node) -> String {
    let (class, name) = match node.default {
        Some(Switch::On) => ("dot on", w.on_by_default),
        Some(Switch::Off) => ("dot", w.off_by_default),
        None => return "<span class=\"mark\"></span>".to_owned(),
    };
    format!(
        "<span class=\"mark {class}\" role=\"img\" aria-label=\"{name}\" title=\"{name}\"></span>"
    )
}

/// A row's marks in four places: whether it is on by default, its
/// status, and its uses, each place empty without its mark.
fn marks(w: &Words, node: &Node, (instructions, review): (bool, bool)) -> String {
    let mut out = String::from("<span class=\"marks\">");
    out.push_str(&dot(w, node));
    out.push_str(&status_mark(node.status));
    for (on, name, svg) in [
        (instructions, w.use_names[0], SCROLL_TEXT),
        (review, w.use_names[1], SCAN_EYE),
    ] {
        out.push_str(&if on {
            format!(
                "<span class=\"mark use\" role=\"img\" aria-label=\"{name}\" title=\"{name}\">{}</span>",
                icon(svg, "")
            )
        } else {
            "<span class=\"mark\"></span>".to_owned()
        });
    }
    out.push_str("</span>");
    out
}

/// A status as its icon in its color, beside its word.
fn status_icon(s: Status) -> String {
    let svg = match s {
        Status::Active => CIRCLE_CHECK,
        Status::Trial => FLASK_CONICAL,
        Status::Draft => PENCIL,
        Status::Superseded => ARROW_RIGHT_LEFT,
        Status::Repealed => CIRCLE_X,
    };
    format!(
        "<span class=\"mark status-{}\">{}</span>",
        status(s),
        icon(svg, "")
    )
}

/// A status as its icon alone, named by its word.
fn status_mark(s: Status) -> String {
    let word = status(s);
    status_icon(s).replacen(
        "\">",
        &format!("\" role=\"img\" aria-label=\"{word}\" title=\"{word}\">"),
        1,
    )
}

/// A value of a field of the filter: its key, its label as HTML, whether
/// the default filter chooses it, and for a domain the nearest domain
/// above it and its depth.
struct Value {
    key: String,
    html: String,
    checked: bool,
    above: Option<String>,
    depth: usize,
}

/// The controls above the tables: the switch between the two views,
/// then a menu of check boxes per field of the filter, and the style
/// sheet that hides the rows a clear box leaves out, so that the filter
/// works without the script.
fn controls(project: &Project, l: &Lang) -> String {
    let w = &l.w;
    // The search, which needs the script, comes first; the view and the
    // menus, each under its name; then Clear filters, which needs the
    // script too; under the row, how many rules are shown.
    let mut out = format!(
        "<div class=\"controls\">\n<label class=\"control find\" hidden><span class=\"control-label\">{}</span><input type=\"search\" name=\"q\" placeholder=\"{}\" autocomplete=\"off\"></label>\n<fieldset class=\"control views\"><legend class=\"control-label\">{}</legend><span class=\"segments\">",
        w.find, w.find_placeholder, w.view
    );
    for (i, (view, name)) in VIEWS.iter().zip(w.views).enumerate() {
        let checked = if i == 0 { " checked" } else { "" };
        let _ = write!(
            out,
            "<input type=\"radio\" name=\"view\" id=\"view-{view}\" value=\"{view}\"{checked}><label for=\"view-{view}\">{name}</label>"
        );
    }
    out.push_str("</span></fieldset>\n");
    let plain = |key: &str, html: String| Value {
        key: key.to_owned(),
        html,
        checked: true,
        above: None,
        depth: 0,
    };
    let mut fields: Vec<Vec<Value>> = Vec::new();
    fields.push(
        STATUSES
            .iter()
            .map(|&s| Value {
                checked: !matches!(s, Status::Superseded | Status::Repealed),
                ..plain(status(s), format!("{}{}", status_icon(s), status(s)))
            })
            .collect(),
    );
    fields.push(vec![
        plain(
            "on",
            format!("<span class=\"mark dot on\"></span>{}", w.on_by_default),
        ),
        plain(
            "off",
            format!("<span class=\"mark dot\"></span>{}", w.off_by_default),
        ),
    ]);
    fields.push(vec![
        plain(
            "instructions",
            format!("{}{}", icon(SCROLL_TEXT, ""), w.use_names[0]),
        ),
        plain(
            "review",
            format!("{}{}", icon(SCAN_EYE, ""), w.use_names[1]),
        ),
    ]);
    let mut list = Vec::new();
    for source in project.sources.iter() {
        let mut found = Vec::new();
        for node in source.nodes.values().filter(|n| n.parent.is_none()) {
            domains(source, node, None, 0, &mut found);
        }
        for (domain, above, depth) in found {
            let key = Key::new(&source.name, domain.id);
            let (title, title_lang) = pick_lang(&domain.title, domain, l.code);
            let title = filled(project, &key, title, true, title_lang);
            let html = format!(
                "<span class=\"ref\">{}</span><span{}>{}</span>{}",
                escape(&key.to_string()),
                lang_attr(title_lang, l.code),
                escape(&title),
                parent_title(project, &key, l.code)
            );
            list.push(Value {
                above: above.map(|a| Key::new(&source.name, a.id).to_string()),
                depth,
                ..plain(&key.to_string(), html)
            });
        }
    }
    fields.push(list);
    fields.push(
        QUADRANTS
            .iter()
            .map(|q| plain(q.class(), q.name(w).to_owned()))
            .collect(),
    );
    fields.push(vec![
        plain("on", enabled_tag(w.enabled, true)),
        plain("off", enabled_tag(w.not_enabled, false)),
    ]);
    let mut style = String::from("<style>");
    for ((field, label), values) in FIELDS.iter().zip(w.fields).zip(&fields) {
        // What the menu chooses: All, or how many of its values.
        let chosen = values.iter().filter(|v| v.checked).count();
        let shown = if chosen == values.len() {
            w.every.to_owned()
        } else {
            format!("{chosen}/{}", values.len())
        };
        let _ = write!(
            out,
            "<div class=\"control\"><span class=\"control-label\" id=\"label-{field}\">{label}</span><details class=\"menu\" data-field=\"{field}\" data-all-text=\"{}\" data-none-text=\"{}\"><summary aria-labelledby=\"label-{field} value-{field}\" aria-controls=\"menu-{field}\"><span class=\"value\" id=\"value-{field}\">{shown}</span>{}</summary>\n<div class=\"menu-panel\" id=\"menu-{field}\" role=\"group\" aria-label=\"{label}\">",
            w.every,
            w.nothing,
            icon(CHEVRON_DOWN, "")
        );
        if *field == "domain" {
            let _ = write!(
                out,
                "<input type=\"search\" class=\"search\" placeholder=\"{}\" aria-label=\"{}\" hidden>",
                w.search, w.search
            );
        }
        let _ = write!(
            out,
            "<span class=\"all\" hidden><button type=\"button\" data-all=\"true\">{}</button><button type=\"button\" data-all=\"false\">{}</button></span><span class=\"values\">",
            w.all, w.none
        );
        for (i, v) in values.iter().enumerate() {
            let (value, html, depth) = (&v.key, &v.html, v.depth);
            let id = format!("f-{field}-{i}");
            let checked = if v.checked { " checked" } else { "" };
            let above = v
                .above
                .as_ref()
                .map(|a| format!(" data-above=\"{}\"", escape(a)))
                .unwrap_or_default();
            let indent = if depth > 0 {
                format!(" style=\"--indent: {depth}\"")
            } else {
                String::new()
            };
            let _ = write!(
                out,
                "<label{indent}><input type=\"checkbox\" id=\"{id}\" name=\"{field}\" value=\"{}\"{above}{checked}>{html}</label>",
                escape(value)
            );
            let hidden = format!("body:has(#{id}:not(:checked))");
            match *field {
                "quadrant" => {
                    let _ = write!(style, "{hidden} section.{value}{{display:none}}");
                }
                // A rule for both uses goes only when both are cleared.
                "uses" => {
                    let _ = write!(
                        style,
                        "{hidden} .item[data-uses=\"{value}\"]{{display:none}}"
                    );
                }
                _ => {
                    let _ = write!(
                        style,
                        "{hidden} .item[data-{field}=\"{}\"]{{display:none}}",
                        escape(value)
                    );
                }
            }
        }
        out.push_str("</span></div></details></div>\n");
    }
    style.push_str(
        "body:has(#f-uses-0:not(:checked)):has(#f-uses-1:not(:checked)) .item[data-uses=\"instructions review\"]{display:none}",
    );
    let _ = write!(
        out,
        "<button type=\"button\" class=\"clear\" hidden>{}</button>\n</div>\n<p class=\"shown\" data-words=\"{}\" hidden></p>\n{style}</style>\n",
        w.clear, w.shown
    );
    out
}

/// Whether the rule set uses a node: an enabled rule, a domain or
/// one-of that is on, or one that holds an enabled rule.
fn in_use(project: &Project, source: &Source, node: &Node) -> bool {
    let key = Key::new(&source.name, node.id);
    let res = &project.resolution;
    res.enabled.contains_key(&key)
        || res.on.contains(&key)
        || source.children(node.id).any(|c| in_use(project, source, c))
}

/// A link to a node's page from a page `up` below the directory of its
/// language (`""` or `"../"`), in text in `lang`: its reference and its
/// title in `lang`, else in the node's own language, and its parent's
/// title when another node has its title (`parent_title`).
fn link(project: &Project, key: &Key, up: &str, lang: &str) -> String {
    title_link(project, key, up, lang, &parent_title(project, key, lang))
}

/// A link as `link` makes it, with `after` after the title in place of
/// the parent's.
fn title_link(project: &Project, key: &Key, up: &str, lang: &str, after: &str) -> String {
    let (title, title_lang) = project.sources.get(key).map_or((String::new(), lang), |n| {
        let (title, title_lang) = pick_lang(&n.title, n, lang);
        (filled(project, key, title, true, title_lang), title_lang)
    });
    format!(
        "<a href=\"{up}{}\"><span class=\"ref\">{}</span><span class=\"title\"{}>{}</span>{after}</a>",
        page(key).display(),
        escape(&key.to_string()),
        lang_attr(title_lang, lang),
        escape(&title),
    )
}

/// A node's parent's title, in `lang` as `link` gives a title, muted in
/// parentheses after a space, when a node under another parent has the
/// node's title in `lang`, so that the two are told apart where a title
/// stands alone; under one parent their references tell them apart.
/// Else nothing.
fn parent_title(project: &Project, key: &Key, lang: &str) -> String {
    let sources = &project.sources;
    let Some((node, parent_key)) = sources.get(key).zip(sources.parent(key)) else {
        return String::new();
    };
    let Some(parent) = sources.get(&parent_key) else {
        return String::new();
    };
    let title = pick_lang(&node.title, node, lang).0;
    let shared = sources.nodes().any(|(k, n)| {
        pick_lang(&n.title, n, lang).0 == title && sources.parent(&k).as_ref() != Some(&parent_key)
    });
    if !shared {
        return String::new();
    }
    let (title, title_lang) = pick_lang(&parent.title, parent, lang);
    format!(
        " <span class=\"parent\"{}>({})</span>",
        lang_attr(title_lang, lang),
        escape(&filled(project, &parent_key, title, true, title_lang))
    )
}

/// The quadrant of a source and its name: the class gives the mark its
/// color, and the words say it without one.
fn quadrant_tag(w: &Words, source: &Source) -> String {
    let q = Quadrant::of(source);
    format!(
        "<span class=\"q {}\">{} · {}</span>",
        q.class(),
        q.name(w),
        escape(&source.name)
    )
}

/// On a node's page, its kind unless it is a rule, its status and
/// whether it is on by default, and for a rule what this rule set does
/// with it. Each fact has its own form (`docs/design/frontend.md`,
/// Components, Tags): the status a pill, a default switch a dot.
fn tags(project: &Project, l: &Lang, key: &Key, node: &Node) -> String {
    let w = &l.w;
    let mut out = String::new();
    if node.kind != Kind::Rule {
        let _ = write!(out, "<span class=\"kind\">{}</span>", kind(w, node.kind));
    }
    out.push_str(&status_tag(node.status));
    if let Some(mark) = switch(project, w, key, node) {
        out.push_str(&mark.tag());
    }
    if node.kind == Kind::Rule {
        out.push_str(&enabled(project, l, key, "../"));
    }
    out
}

/// What this rule set does with a rule, from a page `up` below the
/// directory of its language: enables it, leaves it out, or has another
/// rule override it.
fn enabled(project: &Project, l: &Lang, key: &Key, up: &str) -> String {
    let (w, res) = (&l.w, &project.resolution);
    let what = match res.overridden.get(key) {
        Some(by) => format!("{}{}", w.overridden_by, link(project, by, up, l.code)),
        None if res.enabled.contains_key(key) => w.enabled.to_owned(),
        None => w.not_enabled.to_owned(),
    };
    enabled_tag(&what, res.in_force(key))
}

fn status_tag(s: Status) -> String {
    let s = status(s);
    format!("<span class=\"status {s}\">{s}</span>")
}

/// Whether a rule is enabled: `check` and the words when it is in force.
fn enabled_tag(what: &str, on: bool) -> String {
    if on {
        format!(
            "<span class=\"enabled on\">{}{what}</span>",
            icon(CHECK, "")
        )
    } else {
        format!("<span class=\"enabled\">{what}</span>")
    }
}

/// The legend above the tables, after Ruff's, open until the reader
/// folds it: a box of a line per kind of mark, its name in a column,
/// then each mark as the rows draw it with its word, the defaults first
/// as in a row.
fn legend(w: &Words) -> String {
    let [defaults, statuses, uses, one_of, enabled, places] = w.legend;
    let item = |html: String| format!("<span class=\"legend-item\">{html}</span>");
    let lines = [
        (
            defaults,
            item(format!(
                "<span class=\"mark dot on\"></span>{}",
                w.on_by_default
            )) + &item(format!(
                "<span class=\"mark dot\"></span>{}",
                w.off_by_default
            )),
        ),
        (
            statuses,
            STATUSES
                .iter()
                .map(|&s| item(format!("{}{}", status_icon(s), status(s))))
                .collect::<String>(),
        ),
        (
            uses,
            [(SCROLL_TEXT, 0), (SCAN_EYE, 1)]
                .into_iter()
                .map(|(svg, i)| {
                    item(format!(
                        "<span class=\"mark use\">{}</span><span><strong>{}</strong>{}</span>",
                        icon(svg, ""),
                        w.use_names[i],
                        w.use_meanings[i]
                    ))
                })
                .collect(),
        ),
        (
            one_of,
            item(w.one_of_meaning.to_owned())
                + &item(format!(
                    "<span class=\"default-option\">{}{}</span>",
                    icon(CIRCLE_DOT, ""),
                    w.default_option
                ))
                + &item(format!(
                    "<span class=\"option\">{}</span>{}",
                    icon(CIRCLE, ""),
                    w.another_option
                )),
        ),
        (
            enabled,
            item(enabled_tag(w.enabled, true)) + &item(enabled_tag(w.not_enabled, false)),
        ),
        (places, w.slots.to_owned()),
    ];
    let mut out = format!(
        "<details class=\"legend\" open><summary aria-controls=\"legend-list\">{}{}</summary>\n<dl id=\"legend-list\">\n",
        icon(CHEVRON_RIGHT, ""),
        w.legend_name
    );
    for (name, marks) in lines {
        let class = if name == places {
            " class=\"places\""
        } else {
            ""
        };
        let _ = writeln!(out, "<div{class}><dt>{name}</dt><dd>{marks}</dd></div>");
    }
    out.push_str("</dl>\n</details>\n");
    out
}

/// How a node shows what the defaults do with it.
enum Mark {
    /// On or off by default: the class of its dot, and the words.
    Switch(&'static str, &'static str),
    /// The default option of its one-of.
    DefaultOption(&'static str),
}

impl Mark {
    /// The mark as a tag: a default switch its dot before its words.
    fn tag(&self) -> String {
        match *self {
            Mark::Switch(class, word) => format!("<span class=\"switch {class}\">{word}</span>"),
            Mark::DefaultOption(word) => format!(
                "<span class=\"default-option\">{}{word}</span>",
                icon(CHECK, "")
            ),
        }
    }
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

/// The page of a node in `l`, `langs` the site's languages. A text the
/// node does not have in `l` is shown in the node's own language, marked
/// with its `lang`, and a tag says so.
fn node_page(
    project: &Project,
    l: &Lang,
    langs: &[Lang],
    key: &Key,
    node: &Node,
    switch: &str,
) -> Result<String, String> {
    let (w, lang) = (&l.w, l.code);
    let (title, title_lang) = pick_lang(&node.title, node, lang);
    let title = filled(project, key, title, true, title_lang);
    let title = title.as_str();
    let mut out = head(&format!("{key} {title}"), lang, "text", switch);
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
        // The path names each node's parents: no parent's title after one.
        let _ = write!(out, " / {}", title_link(project, parent, "../", lang, ""));
    }
    let texts = [node.applicability.as_ref()].into_iter().flatten().chain(
        node.rule
            .iter()
            .flat_map(|r| [&r.statement, &r.reason, &r.verification]),
    );
    let untranslated = std::iter::once(&node.title)
        .chain(texts)
        .any(|t| !t.contains_key(lang));
    let _ = writeln!(
        out,
        "</nav>\n<h1><span class=\"ref\">{}</span>{}</h1>\n<p class=\"tag-line\">{}{}",
        escape(&key.to_string()),
        in_lang(&escape(title), title_lang, lang),
        project
            .sources
            .source(&key.source)
            .map(|s| quadrant_tag(w, s))
            .unwrap_or_default(),
        tags(project, l, key, node)
    );
    if untranslated {
        let _ = write!(
            out,
            "<span class=\"untranslated\">{}{}</span>",
            icon(LANGUAGES, ""),
            w.untranslated
        );
    }
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
        let _ = writeln!(out, "<p>{}{}</p>", w.uses, uses.join(w.and));
    }
    // A text not in the page's language is in the node's, with that
    // language's labels and names.
    let text = |part: &str, text: &Text| {
        let (text, text_lang) = pick_lang(text, node, lang);
        let labels = langs
            .iter()
            .find(|l| l.code == text_lang)
            .map_or(&l.labels, |l| &l.labels);
        let html = text_html(project, labels, key, part, text, text_lang, "../")?;
        Ok::<_, String>(match text_lang {
            t if t == lang => html,
            t => format!("<div{}>\n{html}</div>\n", lang_attr(t, lang)),
        })
    };
    if let Some(applicability) = &node.applicability {
        out.push_str(&text("applicability", applicability)?);
    }
    if let Some(rule) = &node.rule {
        for (name, heading, t) in [
            ("statement", w.statement, &rule.statement),
            ("reason", w.reason, &rule.reason),
            ("verification", w.verification, &rule.verification),
        ] {
            let _ = writeln!(out, "<h2>{heading}</h2>");
            out.push_str(&text(name, t)?);
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
            let _ = writeln!(out, "<li>{}</li>", link(project, k, "../", lang));
        }
        out.push_str("</ul>\n");
    }
    out.push_str("</main>\n</body>\n</html>\n");
    Ok(out)
}

/// A rule's text, in `lang`, as HTML: paragraphs, lists whose items
/// start with `- `, and code in backticks; each citation of a rule a link
/// to its page, each of a provision its label.
fn text_html(
    project: &Project,
    labels: &Labels,
    key: &Key,
    part: &str,
    text: &str,
    lang: &str,
    up: &str,
) -> Result<String, String> {
    let fail = |what: String| Err(format!("{}: {part}: {what}", project.sources.file(key)));
    // Escaping leaves the citations and the placeholders as they are:
    // they hold no character HTML treats specially. A placeholder shows
    // the value, marked, with the parameter's name on hover, linked to
    // its entry on the page of parameters.
    let text = param::fill(&escape(text), |name| {
        let (source, shown) = match project.values.get(name) {
            Some(s) => (s.source.clone(), s.param.shown(s.private_value(), lang)),
            None => match project.sources.param(&key.source, name) {
                Ok((source, p)) => (source.to_owned(), p.shown(&p.default, lang)),
                Err(_) => return format!("{{${name}}}"),
            },
        };
        format!(
            "<a class=\"value\" href=\"{up}{}#{name}\" title=\"{name}\">{}</a>",
            params_page(&source).display(),
            escape(&shown)
        )
    });
    let text = cite::replace(&text, |c| match c {
        Citation::Rule(text) => match node::cited_ref(text).map(|r| r.key(&key.source)) {
            Some(cited) if project.sources.get(&cited).is_some() => {
                Ok(link(project, &cited, up, lang))
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
