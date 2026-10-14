//! Fixtures shared by the integration tests: rule files built from a few
//! values, as `(id, text)` pairs.

#![allow(dead_code)]

pub fn domain(id: u64) -> (u64, String) {
    (
        id,
        format!(
            "id = {id}\nslug = \"domain-{id}\"\nrevision = 1\nkind = \"domain\"\nstatus = \"active\"\nlang = \"en\"\n\n[title]\nen = \"Domain {id}\"\n"
        ),
    )
}

pub fn one_of(id: u64, parent: u64) -> (u64, String) {
    (
        id,
        format!(
            "id = {id}\nrevision = 1\nkind = \"one-of\"\nstatus = \"active\"\nlang = \"en\"\nparent = {parent}\n\n[title]\nen = \"One-of {id}\"\n"
        ),
    )
}

/// A rule under `parent`; `extra` is more top-level lines, such as
/// `requires = [3]`.
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
authority = ["simplicity"]
constitution = ">=1"
level = "project"
check = "review"
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
