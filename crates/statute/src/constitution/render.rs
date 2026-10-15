//! Renders the constitution, in full and for agents, in each language.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;

use super::lang::{Language, fill, language_or_default};
use super::{AGENT, Article, Item, MARK, MONTH, Section, Sources, number_key};

/// List markers padded to the indent, so that a section's or an item's
/// further lines start at its content column: CommonMark takes any indent
/// up to that column, Python-Markdown wants four spaces.
const INDENT: &str = "    ";
const SECTION: &str = "-   ";
const ITEM: &str = "    -   ";

/// After a provision the version for agents leaves out, in the full
/// version.
pub const OMITTED_MARK: &str = "<sup class=\"omitted\">*</sup>";

/// A heading of the front text (the preamble's): the version for agents
/// leaves the front text out, so in the full version it is marked as well.
static FRONT_HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^#{2,6} .+$").unwrap());

/// The constitution in language `lang`: the full version, or with
/// `agent` the version for agents, without anchors and links, its
/// citations as plain text, a note of its own in place of the front text,
/// and each provision marked for maintainers as its number and a note.
/// `lang` is one of `src.languages`, which check 1 found a rendering for.
pub fn render(lang: &str, src: &Sources, agent: bool) -> String {
    Renderer::new(lang, src, agent).text()
}

/// How the full version labels a citation of each provision, by
/// citation: `Art. IV §1(2)` for `principals.competent`.
pub fn labels(lang: &str, src: &Sources) -> BTreeMap<String, String> {
    Renderer::new(lang, src, false).labels
}

/// The rendering of one language and version, with its number tables.
struct Renderer<'a> {
    lang: &'a str,
    src: &'a Sources,
    agent: bool,
    lng: &'static Language,
    labels: BTreeMap<String, String>,
    numbers: BTreeMap<String, String>,
    /// A section's or an item's citation without its article or section.
    short: BTreeMap<String, String>,
    /// The article or section that `short` leaves out.
    parent: BTreeMap<String, String>,
    /// The citations, in order of first use: the link definitions.
    cited: Vec<String>,
}

impl<'a> Renderer<'a> {
    fn new(lang: &'a str, src: &'a Sources, agent: bool) -> Self {
        let lng = language_or_default(lang);
        let mut r = Renderer {
            lang,
            src,
            agent,
            lng,
            labels: BTreeMap::new(),
            numbers: BTreeMap::new(),
            short: BTreeMap::new(),
            parent: BTreeMap::new(),
            cited: Vec::new(),
        };
        for a in &src.articles {
            let (n, s) = number_key(&a.number);
            let number = (lng.article_number)(n, &s);
            r.labels.insert(a.id.clone(), lng.article_label(n, &s));
            r.numbers.insert(a.id.clone(), number);
        }
        for s in &src.sections {
            let (n, suffix) = number_key(&s.number);
            let number = (lng.section_number)(n, &suffix);
            let article = r.labels.get(&s.article).cloned().unwrap_or_default();
            let label = lng.section_label(&article, n, &suffix);
            r.numbers.insert(s.id.clone(), number.clone());
            r.short.insert(s.id.clone(), number);
            r.parent.insert(s.id.clone(), s.article.clone());
            for i in &s.items {
                let (n, suffix) = number_key(&i.number);
                let short = (lng.item_ref)(n, &suffix);
                r.labels
                    .insert(i.cite(), lng.item_label(&label, n, &suffix));
                r.short.insert(i.cite(), short);
                r.parent.insert(i.cite(), s.id.clone());
            }
            r.labels.insert(s.id.clone(), label);
        }
        r
    }

    /// The anchor of a provision, none in the version for agents.
    fn anchor(&self, cite: &str) -> String {
        if self.agent {
            String::new()
        } else {
            format!("<a id=\"{cite}\"></a>")
        }
    }

    /// The note that a provision was repealed in `month`, `YYYY-MM`.
    fn note(&self, month: Option<&str>) -> String {
        let m = MONTH.captures(month.unwrap_or_default());
        let (y, m) = m.map_or((0, 0), |m| {
            (m[1].parse().unwrap_or(0), m[2].parse().unwrap_or(0))
        });
        (self.lng.repealed_note)(y, m)
    }

    /// The text with each citation rendered: a link to its provision's
    /// anchor, its label as the link text, except that a section or item
    /// citation joined by a connector to the citation of another section
    /// of the same article, or another item of the same section, just
    /// before it renders short: `Art. IV §1 and §2`, `Art. IV §1(2) or
    /// (4)`. Reference links, not inline ones: limae reads the `)` of an
    /// inline link followed by a Chinese character as a parenthesis that
    /// needs a space after it.
    fn refs(&mut self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let (mut end, mut previous): (usize, Option<String>) = (0, None);
        for c in MARK.captures_iter(text) {
            let whole = c.get(0).unwrap();
            let cite = &c[2];
            let between = &text[end..whole.start()];
            let joined = self
                .parent
                .get(cite)
                .is_some_and(|p| previous.as_ref() == Some(p))
                && self.lng.connectors.contains(&between);
            let table = if joined { &self.short } else { &self.labels };
            let mut label = table
                .get(cite)
                .cloned()
                .unwrap_or_else(|| whole.as_str().to_string());
            if !self.agent {
                if !self.cited.iter().any(|c| c == cite) {
                    self.cited.push(cite.to_string());
                }
                label = format!("[{label}][{cite}]");
            }
            out.push_str(between);
            out.push_str(&label);
            previous = self.parent.get(cite).cloned();
            end = whole.end();
        }
        out.push_str(&text[end..]);
        out
    }

    /// A list line, `first` and the text's first line, then the text's
    /// other lines indented by `indent`.
    fn indented(&mut self, first: &str, text: &str, indent: &str) -> Vec<String> {
        let rendered = self.refs(text);
        let mut lines = rendered.split('\n');
        let mut out = vec![format!("{first}{}", lines.next().unwrap_or_default())];
        out.extend(lines.map(|t| {
            if t.is_empty() {
                String::new()
            } else {
                format!("{indent}{t}")
            }
        }));
        out
    }

    fn item_entry(&mut self, i: &Item) -> Vec<String> {
        let (n, s) = number_key(&i.number);
        let marker = self.anchor(&i.cite()) + &(self.lng.item_marker)(n, &s);
        let content = &i.content[self.lang];
        if i.status == "repealed" {
            vec![format!(
                "{ITEM}{} ({})",
                marker.trim_end(),
                self.note(i.repealed.as_deref())
            )]
        } else if i.omitted && self.agent {
            vec![format!("{ITEM}{marker}{}", self.lng.omitted)]
        } else if i.omitted {
            let first = format!("{ITEM}{}{OMITTED_MARK} ", marker.trim_end());
            self.indented(&first, content, &INDENT.repeat(2))
        } else {
            self.indented(&format!("{ITEM}{marker}"), content, &INDENT.repeat(2))
        }
    }

    /// The entries of one section: the section, then its items.
    fn section_entries(&mut self, s: &Section) -> Vec<Vec<String>> {
        let number = format!("{SECTION}{}**{}**", self.anchor(&s.id), self.numbers[&s.id]);
        if s.status == "repealed" {
            return vec![vec![format!(
                "{number} ({})",
                self.note(s.repealed.as_deref())
            )]];
        }
        if s.omitted && self.agent {
            return vec![vec![format!("{number} {}", self.lng.omitted)]];
        }
        let mark = if s.omitted { OMITTED_MARK } else { "" };
        let mut entries =
            vec![self.indented(&format!("{number}{mark} "), &s.content[self.lang], INDENT)];
        let mut items: Vec<&Item> = s.items.iter().collect();
        items.sort_by_key(|i| number_key(&i.number));
        for i in items {
            entries.push(self.item_entry(i));
        }
        entries
    }

    /// One article: its heading, intro and sections.
    fn article(&mut self, a: &Article) -> String {
        let (anchor, number) = (self.anchor(&a.id), self.numbers[&a.id].clone());
        if a.repealed.is_some() {
            let note = self.note(a.repealed.as_deref());
            return fill(
                self.lng.repealed_article_heading,
                &[("anchor", &anchor), ("number", &number), ("note", &note)],
            );
        }
        let title = self.refs(&a.title[self.lang]);
        let mut block = vec![fill(
            self.lng.article_heading,
            &[("anchor", &anchor), ("number", &number), ("title", &title)],
        )];
        if let Some(intro) = &a.intro {
            block.push(self.refs(&intro[self.lang]));
        }
        let mut own: Vec<&Section> = self
            .src
            .sections
            .iter()
            .filter(|s| s.article == a.id)
            .collect();
        own.sort_by_key(|s| number_key(&s.number));
        let mut body: Vec<String> = Vec::new();
        for s in own {
            for e in self.section_entries(s) {
                // An entry of several paragraphs ends with a blank line, or
                // a list line right after it would join its last paragraph.
                let blank = e.iter().any(String::is_empty);
                body.extend(e);
                if blank {
                    body.push(String::new());
                }
            }
        }
        if body.last().is_some_and(String::is_empty) {
            body.pop();
        }
        if !body.is_empty() {
            block.push(body.join("\n"));
        }
        block.join("\n\n")
    }

    /// The front text; for agents, the line in its place: the full
    /// version's URL, when `index.toml` gives one, and what the mark of a
    /// left-out provision means, when one is marked; empty without both.
    fn front(&mut self) -> String {
        if self.agent {
            let lng = self.lng;
            let mut line: Vec<String> = Vec::new();
            if let Some(url) = self.src.url.as_ref().and_then(|u| u.get(self.lang)) {
                line.push(fill(lng.full_text, &[("url", url)]));
            }
            if self.marked() {
                line.push(lng.omitted_note.to_string());
            }
            return line.join(lng.space);
        }
        let front = self.refs(&self.src.front[self.lang]);
        FRONT_HEADING
            .replace_all(&front, |c: &regex::Captures| {
                format!("{}{OMITTED_MARK}", &c[0])
            })
            .into_owned()
    }

    /// Whether a section or an item is marked for leaving out.
    fn marked(&self) -> bool {
        self.src
            .sections
            .iter()
            .any(|s| s.omitted || s.items.iter().any(|i| i.omitted))
    }

    /// The header line of the full version, with the legend when a text
    /// is marked.
    fn header(&self, front: &str) -> String {
        let lng = self.lng;
        let revision = if self.src.status == "draft" {
            fill(lng.draft_revision, &[("revision", &self.src.revision)])
        } else {
            self.src.revision.clone()
        };
        let layout = &self.src.layout;
        let (index, articles) = (
            layout.index().display().to_string(),
            layout.articles().display().to_string(),
        );
        let mut header = fill(
            lng.header,
            &[
                ("revision", &revision),
                ("index", &index),
                ("articles", &articles),
            ],
        );
        if front.contains(OMITTED_MARK) || self.marked() {
            let agent = layout.output(&format!("{}{AGENT}", self.lang));
            header += &fill(lng.legend, &[("agent", &agent.display().to_string())]);
        }
        header
    }

    fn text(mut self) -> String {
        let edition = fill(self.lng.edition, &[("edition", &self.src.edition)]);
        let front = self.front();
        let title = self.refs(&self.src.title[self.lang]);
        let mut parts = Vec::new();
        if !self.agent {
            parts.push(self.header(&front));
        }
        parts.push(format!("# {title} ({edition})"));
        if !front.is_empty() {
            parts.push(front);
        }
        let mut articles: Vec<&Article> = self.src.articles.iter().collect();
        articles.sort_by_key(|a| number_key(&a.number));
        for a in articles {
            let block = self.article(a);
            parts.push(block);
        }
        if !self.cited.is_empty() {
            let links: Vec<String> = self.cited.iter().map(|c| format!("[{c}]: #{c}")).collect();
            parts.push(links.join("\n"));
        }
        parts.join("\n\n") + "\n"
    }
}
