//! Subdomains (`docs/design/rule-sources.md`, Subdomains): a domain under
//! a domain, two levels at most, and rules hung under a domain of another
//! source.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, rule};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

/// A private repository's rule set: `R` in `r/`, `team` (public) in
/// `team/`, `me` (private) in `me/`, and the repository's own rules.
fn manifest(rules: &str, packs: &str) -> String {
    format!(
        "constitution = {{ source = \"R\", edition = \"2026-10\" }}\npublic = false\nrules = [{rules}]\npacks = [{packs}]\n\n[sources]\nR = {{ path = \"r\" }}\nteam = {{ path = \"team\", public = true }}\nme = {{ path = \"me\", public = false }}\n"
    )
}

/// Domain `id` under domain `parent`.
fn subdomain(id: u64, parent: u64) -> (u64, String) {
    edit(
        domain(id),
        "lang = \"en\"",
        &format!("lang = \"en\"\nparent = {parent}"),
    )
}

/// A rule whose parent is `parent`, a domain of another source.
fn hung(id: u64, parent: &str) -> (u64, String) {
    edit(
        rule(id, 1, ""),
        "parent = 1",
        &format!("parent = \"{parent}\""),
    )
}

/// R: domain 1 with rule 4 and subdomains 2 and 5; under 2, rules 3, 7
/// (which requires 4) and 8 (which requires 3); under 5, rule 6. The
/// other sources have a domain 1 each; the nodes of each test come on
/// top.
fn base() -> Vec<(&'static str, (u64, String))> {
    let mut nodes: Vec<(&str, (u64, String))> = [
        domain(1),
        subdomain(2, 1),
        rule(3, 2, ""),
        rule(4, 1, ""),
        subdomain(5, 1),
        rule(6, 5, ""),
        rule(7, 2, "requires = [4]"),
        rule(8, 2, "requires = [3]"),
    ]
    .into_iter()
    .map(|n| ("r/rules", n))
    .collect();
    for dir in ["team/rules", "me/rules", "rules"] {
        nodes.push((dir, domain(1)));
    }
    nodes
}

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn repo(name: &str, manifest: &str, more: &[(&str, (u64, String))]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("subdomains")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    write(&dir, "statute.toml", manifest);
    write(&dir, "r/constitution/2026-10/index.toml", INDEX);
    write(
        &dir,
        "r/constitution/2026-10/generated/constitution.en.agent.md",
        "# Constitution\n",
    );
    write(
        &dir,
        "r/constitution/2026-10/articles/04-authority/article.toml",
        "id = \"authority\"\n",
    );
    for (sub, (id, text)) in base().iter().chain(more) {
        write(&dir, &format!("{sub}/{id}.toml"), text);
    }
    dir
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn statute(dir: &Path, args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_statute"))
        .env("HOME", concat!(env!("CARGO_TARGET_TMPDIR"), "/home"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    Run {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

#[track_caller]
fn ok(run: &Run) {
    assert_eq!(
        run.code, 0,
        "stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
}

#[track_caller]
fn fails(dir: &Path, error: &str) {
    let run = statute(dir, &["lock"]);
    assert_eq!(run.code, 1, "{}", run.stdout);
    let errors: Vec<&str> = run
        .stderr
        .lines()
        .filter(|l| l.contains(": error: "))
        .collect();
    assert_eq!(errors, [error], "{}", run.stderr);
}

fn locked(dir: &Path) -> Vec<String> {
    let lock = std::fs::read_to_string(dir.join("statute.lock")).unwrap();
    lock.lines()
        .filter_map(|l| l.strip_prefix("rule = "))
        .map(|r| r.trim_matches('"').to_owned())
        .collect()
}

/// The headings of the compiled file, below the constitution's.
fn headings(dir: &Path) -> Vec<String> {
    ok(&statute(dir, &["lock"]));
    ok(&statute(dir, &["compile"]));
    let out = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    out.lines()
        .filter(|l| l.starts_with("##"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_subdomain_is_a_heading_below_its_domain() {
    let dir = repo(
        "headings",
        &manifest("\"R3\", \"R4\", \"R7\", \"R8\"", ""),
        &[],
    );
    // Subdomain 5 has no enabled rule, so no heading; nor has the domain
    // of the repository's own rules.
    assert_eq!(
        headings(&dir),
        [
            "## Domain 1 (R1)",
            "### Domain 2 (R2)",
            "#### Rule 3 (R3)",
            "#### Rule 7 (R7)",
            "#### Rule 8 (R8)",
            "### Rule 4 (R4)",
        ]
    );
    let out = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    // R7 requires R4, out of subdomain 2: a link. R8 requires R3, in it:
    // none.
    assert!(
        out.contains("#### Rule 7 (R7)\n\nDo the thing 7.\n\nRequires: [Rule 4](#rule-R4)\n"),
        "{out}"
    );
    assert!(
        out.contains("#### Rule 8 (R8)\n\nDo the thing 8.\n\n<a id=\"rule-R4\"></a>"),
        "{out}"
    );
}

#[test]
fn a_rule_hangs_under_a_domain_of_another_source() {
    let dir = repo(
        "hung",
        &manifest("\"R3\", \"20\", \"team:21\"", ""),
        &[("rules", hung(20, "R2")), ("team/rules", hung(21, "R1"))],
    );
    assert_eq!(
        headings(&dir),
        [
            "## Domain 1 (R1)",
            "### Domain 2 (R2)",
            "#### Rule 3 (R3)",
            "#### Rule 20 (L20)",
            "### Rule 21 (team:21)",
        ]
    );
    // Its parent is a domain or a subdomain there, never a rule or a
    // one-of.
    let dir = repo(
        "hung-rule",
        &manifest("\"20\"", ""),
        &[("rules", hung(20, "R3"))],
    );
    fails(
        &dir,
        "rules/20.toml: error: `parent` names \"R3\", a rule; a rule's parent in another source is a domain or a subdomain",
    );
    let dir = repo(
        "hung-missing",
        &manifest("\"20\"", ""),
        &[("rules", hung(20, "R99"))],
    );
    fails(
        &dir,
        "rules/20.toml: error: `parent` names \"R99\", which does not exist in source `R`",
    );
    // A public rule's parent is a public domain.
    let dir = repo(
        "hung-private",
        &manifest("", ""),
        &[("team/rules", hung(21, "me:1"))],
    );
    fails(
        &dir,
        "team/rules/21.toml: error: `parent` names \"me:1\", a domain of the private source `me`; a public rule may name only rules of public sources",
    );
    let dir = repo(
        "hung-public",
        &manifest("\"20\"", ""),
        &[("rules", hung(20, "me:1"))],
    );
    ok(&statute(&dir, &["lock"]));
}

#[test]
fn a_pack_is_a_subtree_of_one_source() {
    let more = [("rules", hung(20, "R2"))];
    // Domain 1 as a pack takes the rules of its subdomains, not L20.
    let dir = repo("pack", &manifest("", "\"R1\""), &more);
    ok(&statute(&dir, &["lock"]));
    assert_eq!(locked(&dir), ["R3", "R4", "R6", "R7", "R8"]);
    // A subdomain may be a pack's root.
    let dir = repo("pack-sub", &manifest("", "\"R2\""), &more);
    ok(&statute(&dir, &["lock"]));
    assert_eq!(locked(&dir), ["R3", "R4", "R7", "R8"]);
}

#[derive(serde::Deserialize)]
struct SourceCases {
    subdomain: String,
    cases: Vec<SourceCase>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
struct SourceCase {
    name: String,
    passes: bool,
    /// Where statute, reading node 20 as a local rule, differs.
    statute_passes: Option<bool>,
    node: String,
    statute: Option<String>,
}

/// The compiler's cases of what one source shows
/// (`tools/compiler/tests/source_cases.toml`): node 20 in the repository's
/// own rules, with domain 2, one-of 3, rule 10 and
/// subdomain 4; `team` holds domain 7 and rule 8.
#[test]
fn the_compilers_source_cases() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/compiler/tests/source_cases.toml");
    let cases: SourceCases = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(!cases.cases.is_empty());
    for case in cases.cases {
        let dir = repo(
            "source-cases",
            &manifest("", ""),
            &[
                ("rules", domain(2)),
                ("rules", common::one_of(3, 2)),
                ("rules", rule(10, 2, "")),
                ("rules", (4, cases.subdomain.clone())),
                ("rules", (20, case.node.clone())),
                ("team/rules", domain(7)),
                ("team/rules", rule(8, 7, "")),
            ],
        );
        let run = statute(&dir, &["lock"]);
        let passes = case.statute_passes.unwrap_or(case.passes);
        assert_eq!(run.code == 0, passes, "{}: {}", case.name, run.stderr);
        if !passes {
            let errors: Vec<&str> = run
                .stderr
                .lines()
                .filter(|l| l.contains(": error: "))
                .collect();
            let reason = case
                .statute
                .as_deref()
                .unwrap_or_else(|| panic!("{}: {errors:#?}", case.name));
            assert!(
                errors
                    .iter()
                    .all(|e| e.starts_with("rules/20.toml: error: "))
                    && errors.iter().any(|e| e.contains(reason)),
                "{}: {errors:#?}",
                case.name
            );
        }
    }
}

/// A domain whose parent is `parent`, a domain of another source.
fn hung_domain(id: u64, parent: &str) -> (u64, String) {
    edit(
        domain(id),
        "lang = \"en\"",
        &format!("lang = \"en\"\nparent = \"{parent}\""),
    )
}

#[test]
fn a_domain_hangs_under_a_domain_of_another_source() {
    let more = [
        ("me/rules", hung_domain(30, "R1")),
        ("me/rules", rule(31, 30, "")),
    ];
    let dir = repo("hung-domain", &manifest("\"R3\", \"me:31\"", ""), &more);
    assert_eq!(
        headings(&dir),
        [
            "## Domain 1 (R1)",
            "### Domain 2 (R2)",
            "#### Rule 3 (R3)",
            "### Domain 30 (me:30)",
            "#### Rule 31 (me:31)",
        ]
    );
    // A pack stays in its source: R1 does not reach me:31, me:30 does.
    let dir = repo("hung-domain-pack", &manifest("", "\"R1\""), &more);
    ok(&statute(&dir, &["lock"]));
    assert!(!locked(&dir).contains(&"me:31".to_owned()));
    let dir = repo("hung-domain-own-pack", &manifest("", "\"me:30\""), &more);
    ok(&statute(&dir, &["lock"]));
    assert_eq!(locked(&dir), ["me:31"]);
    // Two levels at most, on either side.
    let dir = repo(
        "hung-domain-sub",
        &manifest("", ""),
        &[("me/rules", hung_domain(30, "R2"))],
    );
    fails(
        &dir,
        "me/rules/30.toml: error: `parent` names \"R2\", a subdomain already; domains nest two levels at most, so name a domain with no parent",
    );
    let dir = repo(
        "hung-domain-under",
        &manifest("", ""),
        &[
            ("me/rules", hung_domain(30, "R1")),
            (
                "me/rules",
                edit(domain(32), "lang = \"en\"", "lang = \"en\"\nparent = 30"),
            ),
        ],
    );
    fails(
        &dir,
        "me/rules/32.toml: error: parent 30 is a subdomain already, under a domain of another source; domains nest two levels at most, so name a domain with no parent",
    );
    // The reference direction: a public domain hangs under a public one.
    let dir = repo(
        "hung-domain-private",
        &manifest("", ""),
        &[("team/rules", hung_domain(30, "me:1"))],
    );
    fails(
        &dir,
        "team/rules/30.toml: error: `parent` names \"me:1\", a domain of the private source `me`; a public rule may name only rules of public sources",
    );
    // The first-party source's domains are the roots.
    let dir = repo(
        "hung-domain-first-party",
        &manifest("", ""),
        &[("r/rules", hung_domain(30, "team:1"))],
    );
    fails(
        &dir,
        "r/rules/30.toml: error: `parent` names \"team:1\"; the first-party source's domains are the roots, with no parent in another source",
    );
}
