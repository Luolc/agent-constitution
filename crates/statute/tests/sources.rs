//! A rule set of several sources: the first-party source, a public and a
//! private source of others, and the repository's own rules, each with a
//! domain 1 and rules whose IDs repeat across sources.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, rule};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

/// The rule set: `R` in `r/`, `team` (public) in `team/`, `me` (private)
/// in `me/`, and the repository's own rules in `rules/`, named in
/// `local-rules`.
fn manifest(public: bool, rules: &str) -> String {
    format!(
        "constitution = {{ source = \"R\", edition = \"2026-10\" }}\npublic = {public}\nlocal-rules = \"rules\"\nrules = [{rules}]\n\n[sources]\nR = {{ path = \"r\" }}\nteam = {{ path = \"team\", public = true }}\nme = {{ path = \"me\", public = false }}\n"
    )
}

/// R: rules 2 and 3. team: rule 7, which requires R2. me: rule 4. The
/// repository's own rules: rule 3, which requires R2, and rule 5, which
/// requires me:4 and cites it and R2.
fn tree() -> Vec<(&'static str, Vec<(u64, String)>)> {
    vec![
        ("r/rules", vec![domain(1), rule(2, 1, ""), rule(3, 1, "")]),
        (
            "team/rules",
            vec![domain(1), rule(7, 1, "requires = [\"R2\"]")],
        ),
        ("me/rules", vec![domain(1), rule(4, 1, "")]),
        (
            "rules",
            vec![
                domain(1),
                rule(3, 1, "requires = [\"R2\"]"),
                edit(
                    rule(5, 1, "requires = [\"me:4\"]"),
                    "Do the thing 5.",
                    "Do the thing 5, after {@rule:me:4} and {@rule:R2}.",
                ),
            ],
        ),
    ]
}

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A fresh repository with the rule set and the sources of `tree`, each
/// node replaced by the one of the same directory and ID in `changed`.
fn repo(name: &str, manifest: &str, changed: &[(&str, (u64, String))]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("sources")
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
    for (sub, nodes) in tree() {
        for (id, text) in nodes {
            let text = changed
                .iter()
                .find(|(d, (i, _))| *d == sub && *i == id)
                .map_or(text, |(_, (_, t))| t.clone());
            write(&dir, &format!("{sub}/{id}.toml"), &text);
        }
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
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_CACHE_HOME")
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

/// `statute lock` fails with exactly one error, the one given.
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
    assert!(!dir.join("statute.lock").exists());
}

#[test]
fn the_sources_lock_and_compile_together() {
    let dir = repo(
        "together",
        &manifest(false, "\"R3\", \"team:7\", \"3\", \"5\""),
        &[],
    );
    ok(&statute(&dir, &["lock"]));
    let lock = std::fs::read_to_string(dir.join("statute.lock")).unwrap();
    // R3 and L3 are two rules; R2 comes in through team:7 and L3, me:4
    // through L5. The first-party source first, the repository's own
    // rules last.
    let rules: Vec<&str> = lock
        .lines()
        .filter_map(|l| l.strip_prefix("rule = "))
        .collect();
    assert_eq!(
        rules,
        [
            "\"R2\"",
            "\"R3\"",
            "\"me:4\"",
            "\"team:7\"",
            "\"L3\"",
            "\"L5\""
        ]
    );
    assert!(
        lock.contains("[[sources]]\nname = \"team\"\npath = \"team\"\n"),
        "{lock}"
    );

    ok(&statute(&dir, &["compile"]));
    let out = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    let headings: Vec<&str> = out.lines().filter(|l| l.starts_with("## ")).collect();
    assert_eq!(
        headings,
        [
            "## Domain 1 (R1)",
            "## Domain 1 (me:1)",
            "## Domain 1 (team:1)",
            "## Domain 1 (L1)"
        ]
    );
    for anchor in ["rule-R3", "rule-me-4", "rule-team-7", "rule-L3"] {
        assert!(
            out.contains(&format!("<a id=\"{anchor}\"></a>")),
            "{anchor}: {out}"
        );
    }
    // A requirement on another source's rule is a link to it, and so is
    // a citation of one.
    assert!(
        out.contains("### Rule 7 (team:7)\n\nDo the thing 7.\n\nRequires: [Rule 2](#rule-R2)\n"),
        "{out}"
    );
    assert!(
        out.contains("Do the thing 5, after [Rule 4](#rule-me-4) and [Rule 2](#rule-R2).\n"),
        "{out}"
    );

    let run = statute(&dir, &["explain", "3"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "rule L3 (Rule 3) is enabled:\n- listed in `rules`\n- it is compiled into AGENTS.md\n"
    );
    let run = statute(&dir, &["explain", "R2"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "rule R2 (Rule 2) is enabled:\n- required by team:7 (Rule 7), which is enabled:\n  - listed in `rules`\n- required by L3 (Rule 3), which is enabled:\n  - listed in `rules`\n- it is compiled into AGENTS.md\n"
    );
}

#[test]
fn add_and_remove_name_each_source() {
    let dir = repo("add", &manifest(false, ""), &[]);
    ok(&statute(&dir, &["add", "me:4"]));
    ok(&statute(&dir, &["add", "L3"]));
    let toml = std::fs::read_to_string(dir.join("statute.toml")).unwrap();
    // A rule of the repository's own is written by its ID alone.
    assert!(toml.contains("rules = [\"me:4\", \"3\"]"), "{toml}");
    let run = statute(&dir, &["remove", "3"]);
    ok(&run);
    assert!(
        run.stdout.contains("removed L3 from `rules`"),
        "{}",
        run.stdout
    );
    let run = statute(&dir, &["add", "nobody:1"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr
            .contains("`nobody:1` names source `nobody`, which [sources] does not define"),
        "{}",
        run.stderr
    );
}

#[test]
fn a_reference_to_another_source_names_a_node_it_has() {
    let in_team = |extra: &str| ("team/rules", rule(7, 1, extra));
    let dir = repo(
        "unknown-source",
        &manifest(false, "\"team:7\""),
        &[in_team("requires = [\"nobody:1\"]")],
    );
    fails(
        &dir,
        "team/rules/7.toml: error: `requires` names \"nobody:1\", a rule of source `nobody`, which the rule set does not name in [sources]",
    );
    let dir = repo(
        "unknown-rule",
        &manifest(false, "\"team:7\""),
        &[in_team("conflicts = [\"me:9\"]")],
    );
    fails(
        &dir,
        "team/rules/7.toml: error: `conflicts` names \"me:9\", which does not exist in source `me`",
    );
    let dir = repo(
        "a-domain",
        &manifest(false, "\"team:7\""),
        &[in_team("requires = [\"R1\"]")],
    );
    fails(
        &dir,
        "team/rules/7.toml: error: `requires` names domain \"R1\"; a domain cannot be required, require a rule or a one-of",
    );
    // In the first-party source, its own rule is named by number.
    let dir = repo(
        "first-party",
        &manifest(false, "\"R3\""),
        &[("r/rules", rule(3, 1, "requires = [\"R2\"]"))],
    );
    fails(
        &dir,
        "r/rules/3.toml: error: `requires` names \"R2\"; in a rule of the first-party source itself, write the number, 2",
    );
}

#[test]
fn a_public_rule_names_no_private_rule() {
    // team is public, me private: each place where team:7 can name me:4
    // fails, with the message the design gives.
    for (name, extra, text, error) in [
        (
            "requires",
            "requires = [\"me:4\"]",
            None,
            "`requires` names \"me:4\", a rule of the private source `me`; a public rule may name only rules of public sources",
        ),
        (
            "conflicts",
            "conflicts = [\"me:4\"]",
            None,
            "`conflicts` names \"me:4\", a rule of the private source `me`; a public rule may name only rules of public sources",
        ),
        (
            "citation",
            "",
            Some("See {@rule:me:4}."),
            "cites `{@rule:me:4}`, a rule of the private source `me`; a public rule may name only rules of public sources",
        ),
    ] {
        let mut node = rule(7, 1, extra);
        if let Some(text) = text {
            node = edit(node, "Do the thing 7.", text);
        }
        let dir = repo(
            &format!("direction-{name}"),
            &manifest(false, "\"team:7\""),
            &[("team/rules", node)],
        );
        fails(&dir, &format!("team/rules/7.toml: error: {error}"));
    }
    // A private rule may name it: the repository's own rules, in a
    // private repository, require and cite me:4 (`tree`).
    let dir = repo("direction-private", &manifest(false, "\"5\""), &[]);
    ok(&statute(&dir, &["lock"]));
    // In a public repository, they are public rules.
    let public = manifest(true, "\"3\"").replace("me = { path = \"me\", public = false }\n", "");
    let dir = repo(
        "direction-public",
        &public,
        &[("rules", rule(5, 1, "requires = [\"team:7\"]"))],
    );
    ok(&statute(&dir, &["lock"]));
}

#[test]
fn the_same_id_in_two_sources_is_two_rules() {
    let dir = repo("same-id", &manifest(false, "\"R3\""), &[]);
    ok(&statute(&dir, &["lock"]));
    let lock = std::fs::read_to_string(dir.join("statute.lock")).unwrap();
    assert!(lock.contains("rule = \"R3\""), "{lock}");
    assert!(!lock.contains("rule = \"L3\""), "{lock}");
    assert!(!lock.contains("rule = \"R2\""), "{lock}");
}

#[test]
fn a_cycle_of_requirements_across_sources_fails() {
    // team:7 and me:4 require each other; both private, so that only the
    // cycle is wrong.
    let private = manifest(false, "\"team:7\"").replace(
        "team = { path = \"team\", public = true }",
        "team = { path = \"team\", public = false }",
    );
    let team = ("team/rules", rule(7, 1, "requires = [\"me:4\"]"));
    let dir = repo(
        "cycle",
        &private,
        &[
            team.clone(),
            ("me/rules", rule(4, 1, "requires = [\"team:7\"]")),
        ],
    );
    fails(
        &dir,
        "me/rules/4.toml: error: the requirements form a cycle across sources: me:4 -> team:7 -> me:4",
    );
    // Without the edge back, it locks.
    let dir = repo("no-cycle", &private, &[team]);
    ok(&statute(&dir, &["lock"]));
}

#[test]
fn rules_dir_names_where_a_source_keeps_its_rules() {
    let with = |dir: &str| {
        manifest(false, "\"team:8\"").replace(
            "team = { path = \"team\", public = true }",
            &format!("team = {{ path = \"team\", rules-dir = \"{dir}\", public = true }}"),
        )
    };
    let dir = repo("rules-dir", &with("agent/rules"), &[]);
    write(&dir, "team/agent/rules/1.toml", &domain(1).1);
    write(&dir, "team/agent/rules/8.toml", &rule(8, 1, "").1);
    ok(&statute(&dir, &["lock"]));
    let lock = std::fs::read_to_string(dir.join("statute.lock")).unwrap();
    assert!(
        lock.contains(
            "name = \"team\"\npath = \"team\"\npublic = true\nrules-dir = \"agent/rules\"\n"
        ),
        "{lock}"
    );
    ok(&statute(&dir, &["compile"]));
    let out = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(out.contains("<a id=\"rule-team-8\"></a>"), "{out}");
    // Without it, `rules/` is read, which has no rule 8.
    let dir = repo("rules-dir-default", &manifest(false, "\"team:8\""), &[]);
    write(&dir, "team/agent/rules/8.toml", &rule(8, 1, "").1);
    let run = statute(&dir, &["lock"]);
    assert_eq!(run.code, 1, "{}", run.stdout);

    for (value, why) in [
        ("/team/rules", "is absolute"),
        ("../team/rules", "leads out of the source with `..`"),
        ("./.statute/rules", "leads into `.statute/`"),
    ] {
        let dir = repo("rules-dir-bad", &with(value), &[]);
        let run = statute(&dir, &["lock"]);
        assert_eq!(run.code, 1, "{value}: {}", run.stdout);
        assert!(
            run.stderr.contains(&format!(
                "[sources] `team`: `rules-dir = \"{value}\"` {why}"
            )),
            "{value}: {}",
            run.stderr
        );
    }
}
