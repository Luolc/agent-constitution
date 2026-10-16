//! Fixtures shared by the integration tests: rule files built from a few
//! values, as `(id, text)` pairs.

#![allow(dead_code)]

/// A domain, off by default.
pub fn domain(id: u64) -> (u64, String) {
    (
        id,
        format!(
            "id = {id}\nslug = \"domain-{id}\"\nrevision = 1\nkind = \"domain\"\nstatus = \"active\"\nlang = \"en\"\ndefault = \"off\"\n\n[title]\nen = \"Domain {id}\"\n"
        ),
    )
}

/// A one-of under `parent` whose default is its child `default`.
pub fn one_of(id: u64, parent: u64, default: u64) -> (u64, String) {
    (
        id,
        format!(
            "id = {id}\nrevision = 1\nkind = \"one-of\"\nstatus = \"active\"\nlang = \"en\"\nparent = {parent}\ndefault = {default}\n\n[title]\nen = \"One-of {id}\"\n"
        ),
    )
}

/// A one-of under `parent` with no children yet, and so no `default`.
pub fn empty_one_of(id: u64, parent: u64) -> (u64, String) {
    edit(one_of(id, parent, 0), "default = 0\n", "")
}

/// A rule under `parent`, off by default; `extra` is more top-level
/// lines, such as `requires = [3]`.
pub fn rule(id: u64, parent: u64, extra: &str) -> (u64, String) {
    (
        id,
        format!(
            r#"id = {id}
revision = 1
kind = "rule"
status = "draft"
lang = "en"
parent = {parent}
default = "off"
authority = ["simplicity"]
constitution = ">=1"
level = "project"
check = "none"
{extra}

[title]
en = "Rule {id}"

[text.statement]
en = "Do the thing {id}."

[text.reason]
en = '''
Because.
'''

[text.verification]
en = "Violated when the thing is not done."
"#
        ),
    )
}

/// An option of the one-of `one_of`: a rule with no `default` of its own.
pub fn option(id: u64, one_of: u64, extra: &str) -> (u64, String) {
    edit(rule(id, one_of, extra), "default = \"off\"\n", "")
}

/// The node with `default = <value>` in place of the fixture's.
pub fn set_default((id, text): (u64, String), value: &str) -> (u64, String) {
    let line = text
        .lines()
        .find(|l| l.starts_with("default = "))
        .expect("the fixture has a `default`")
        .to_owned();
    (id, text.replacen(&line, &format!("default = {value}"), 1))
}

pub fn edit(node: (u64, String), from: &str, to: &str) -> (u64, String) {
    assert!(node.1.contains(from), "{from:?} is not in the fixture");
    (node.0, node.1.replacen(from, to, 1))
}

/// The rule set's one source, the first-party source `R`, from the given
/// nodes, which must pass their checks.
pub fn source(nodes: &[(u64, String)]) -> statute::source::Sources {
    let files = nodes
        .iter()
        .map(|(id, text)| (format!("rules/{id}.toml"), text.clone().into_bytes()))
        .collect();
    let (mut source, diags) = statute::source::Source::from_files(files);
    source.name = "R".to_owned();
    source.public = true;
    assert!(
        diags.0.is_empty(),
        "the fixture has findings: {:#?}",
        diags.0
    );
    statute::source::Sources::new(vec![source])
}

/// One `[[parameter]]` table of `params.toml`: `kind` is its type line
/// and anything after it, as `type = "language"`.
pub fn param(name: &str, kind: &str, scope: &str, default: &str) -> String {
    format!(
        "[[parameter]]\nname = \"{name}\"\n{kind}\nscope = \"{scope}\"\ndefault = \"{default}\"\n\n[parameter.title]\nen = \"The {name}\"\n\n"
    )
}

/// Writes under the edition directory `edition` the provisions the
/// fixtures cite in `authority`: `simplicity`, as [`rule`] does, a
/// section, and `alpha`, an article.
pub fn provisions(edition: &std::path::Path) {
    let articles = edition.join("articles");
    std::fs::create_dir_all(articles.join("01-alpha")).unwrap();
    std::fs::create_dir_all(articles.join("03-purpose")).unwrap();
    std::fs::write(
        articles.join("03-purpose/04-simplicity.toml"),
        "id = \"simplicity\"\n",
    )
    .unwrap();
}
