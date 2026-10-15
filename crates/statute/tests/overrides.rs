//! Overrides (`docs/design/rule-sources.md`, Overrides): a rule of one
//! source in place of a rule of another, of a lower rank.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, one_of, rule};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

/// The rule set of a private repository: `R` in `r/`, `team` (public,
/// global) in `team/`, `here` (public, local) in `here/`, `me` (private,
/// global) in `me/`, and the repository's own rules in `rules/`.
fn manifest(rules: &str) -> String {
    format!(
        "constitution = {{ source = \"R\", edition = \"2026-10\" }}\npublic = false\nrules = [{rules}]\n\n[sources]\nR = {{ path = \"r\" }}\nteam = {{ path = \"team\", public = true }}\nhere = {{ path = \"here\", public = true, scope = \"local\" }}\nme = {{ path = \"me\", public = false }}\n"
    )
}

/// R: rule 2, which requires rule 8, and rule 3, which requires 2; rule 9, which
/// requires 2; rule 10, which conflicts with 2; one-of 4 with children 5
/// and 6, and rule 11, which requires the one-of. The other sources have
/// a domain 1 each and me a rule 4; the nodes of each test come on top.
fn base() -> Vec<(&'static str, (u64, String))> {
    let mut nodes: Vec<(&str, (u64, String))> = [
        domain(1),
        rule(2, 1, "requires = [8]"),
        rule(3, 1, "requires = [2]"),
        one_of(4, 1),
        rule(5, 4, ""),
        rule(6, 4, ""),
        rule(8, 1, ""),
        rule(9, 1, "requires = [2]"),
        rule(10, 1, "conflicts = [2]"),
        rule(11, 1, "requires = [4]"),
    ]
    .into_iter()
    .map(|n| ("r/rules", n))
    .collect();
    for dir in ["team/rules", "here/rules", "me/rules", "rules"] {
        nodes.push((dir, domain(1)));
    }
    nodes.push(("me/rules", rule(4, 1, "")));
    nodes
}

/// A rule that overrides `target`, with `extra` lines.
fn overriding(id: u64, target: &str, extra: &str) -> (u64, String) {
    edit(
        rule(id, 1, extra),
        "parent = 1",
        &format!("overrides = \"{target}\""),
    )
}

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A fresh repository with the rule set, the nodes of `base` and `more`,
/// the latter replacing those of the same directory and ID.
fn repo(name: &str, manifest: &str, more: &[(&str, (u64, String))]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("overrides")
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
        // The repository's own rules are in the default `local-rules`.
        let sub = if *sub == "rules" {
            ".statute/rules"
        } else {
            sub
        };
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
}

/// The rules of the lock file, each with its `overridden-by` when it has
/// one.
fn locked(dir: &Path) -> Vec<String> {
    let lock = std::fs::read_to_string(dir.join("statute.lock")).unwrap();
    let mut out: Vec<String> = Vec::new();
    for l in lock.lines() {
        if let Some(rule) = l.strip_prefix("rule = ") {
            out.push(rule.trim_matches('"').to_owned());
        } else if let Some(by) = l.strip_prefix("overridden-by = ")
            && let Some(last) = out.last_mut()
        {
            last.push_str(&format!(" by {}", by.trim_matches('"')));
        }
    }
    out
}

#[test]
fn the_overriding_rule_stands_in_the_place_of_the_one_it_overrides() {
    let l20 = ("rules", overriding(20, "R2", ""));
    let dir = repo("place", &manifest("\"R3\", \"R9\", \"20\""), &[l20]);
    ok(&statute(&dir, &["lock"]));
    // R2 counts as enabled, for R3 and R9 that require it, and
    // brings in nothing: not R8, its requirement.
    assert_eq!(locked(&dir), ["R2 by L20", "R3", "R9", "L20"]);
    let without = repo("place-without", &manifest("\"R3\", \"R9\""), &[]);
    ok(&statute(&without, &["lock"]));
    assert_eq!(locked(&without), ["R2", "R3", "R8", "R9"]);

    ok(&statute(&dir, &["compile"]));
    let out = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(
        out.contains(
            "\n<a id=\"rule-R2\"></a>\n<a id=\"rule-L20\"></a>\n### Rule 20 (L20)\n\nOverrides R2.\n\nDo the thing 20.\n\n<a id=\"rule-R3\"></a>\n### Rule 3 (R3)\n"
        ),
        "{out}"
    );
    assert!(!out.contains("(R2)"), "{out}");
    // Its own rules' domain gets no heading: L20 is under R1.
    assert!(!out.contains("(L1)"), "{out}");

    let run = statute(&dir, &["explain", "R2"]);
    ok(&run);
    assert!(
        run.stdout.starts_with("rule R2 (Rule 2) is overridden by L20 (Rule 20), which stands in its place; it counts as enabled:\n"),
        "{}",
        run.stdout
    );
    let run = statute(&dir, &["explain", "20"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "rule L20 (Rule 20) is enabled:\n- it overrides R2 (Rule 2)\n- listed in `rules`\n- it is compiled into AGENTS.md\n"
    );
}

#[test]
fn a_chain_of_overrides() {
    let dir = repo(
        "chain",
        &manifest("\"20\""),
        &[
            ("team/rules", overriding(7, "R2", "")),
            ("rules", overriding(20, "team:7", "")),
        ],
    );
    ok(&statute(&dir, &["lock"]));
    assert_eq!(locked(&dir), ["R2 by team:7", "team:7 by L20", "L20"]);
    ok(&statute(&dir, &["compile"]));
    let out = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(
        out.contains("<a id=\"rule-R2\"></a>\n<a id=\"rule-team-7\"></a>\n<a id=\"rule-L20\"></a>\n### Rule 20 (L20)\n\nOverrides team:7, which overrides R2.\n"),
        "{out}"
    );
    let run = statute(&dir, &["explain", "20"]);
    assert!(
        run.stdout
            .contains("- it overrides team:7 (Rule 7), which overrides R2 (Rule 2)\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn a_rule_overrides_only_a_rule_of_a_lower_rank() {
    // The repository's own rules override a global source, and a source
    // of `scope = "local"` does too.
    for (name, enabled, node, by) in [
        (
            "rank-local",
            "\"20\"",
            ("rules", overriding(20, "team:3", "")),
            "L20",
        ),
        (
            "rank-scope",
            "\"here:20\"",
            ("here/rules", overriding(20, "team:3", "")),
            "here:20",
        ),
    ] {
        let dir = repo(
            name,
            &manifest(enabled),
            &[("team/rules", rule(3, 1, "")), node],
        );
        ok(&statute(&dir, &["lock"]));
        let rules = locked(&dir);
        assert!(
            rules.contains(&format!("team:3 by {by}")),
            "{name}: {rules:?}"
        );
    }
    for (name, node, error) in [
        (
            "rank-first-party",
            ("r/rules", overriding(20, "team:3", "")),
            "r/rules/20.toml: error: `overrides` names \"team:3\", a rule of source `team`, which ranks above source `R`; a rule may override only a rule of a lower rank: the first-party source, then the other global sources, then this repository's own rules and the sources with `scope = \"local\"`",
        ),
        (
            "rank-same",
            ("me/rules", overriding(20, "team:3", "")),
            "me/rules/20.toml: error: `overrides` names \"team:3\", a rule of source `team`, which ranks with source `me`; a rule may override only a rule of a lower rank: the first-party source, then the other global sources, then this repository's own rules and the sources with `scope = \"local\"`",
        ),
        (
            "rank-up",
            ("team/rules", overriding(20, "here:3", "")),
            "team/rules/20.toml: error: `overrides` names \"here:3\", a rule of source `here`, which ranks above source `team`; a rule may override only a rule of a lower rank: the first-party source, then the other global sources, then this repository's own rules and the sources with `scope = \"local\"`",
        ),
    ] {
        let dir = repo(
            name,
            &manifest(""),
            &[
                ("team/rules", rule(3, 1, "")),
                ("here/rules", rule(3, 1, "")),
                node,
            ],
        );
        fails(&dir, error);
    }
}

#[test]
fn a_public_rule_overrides_no_private_rule() {
    // `here` is public and local, so it ranks above `me`; only the
    // direction is wrong.
    let dir = repo(
        "direction",
        &manifest("\"here:20\""),
        &[("here/rules", overriding(20, "me:4", ""))],
    );
    fails(
        &dir,
        "here/rules/20.toml: error: `overrides` names \"me:4\", a rule of the private source `me`; a public rule may name only rules of public sources",
    );
    // The repository's own rules, in a private repository, may.
    let dir = repo(
        "direction-private",
        &manifest("\"20\""),
        &[("rules", overriding(20, "me:4", ""))],
    );
    ok(&statute(&dir, &["lock"]));
    assert_eq!(locked(&dir), ["me:4 by L20", "L20"]);
}

#[test]
fn only_a_rule_is_overridden_and_an_overriding_rule_has_no_parent() {
    for (name, target, error) in [
        (
            "a-domain",
            "R1",
            ".statute/rules/20.toml: error: `overrides` names \"R1\", a domain; only a rule can be overridden: in place of a one-of's child, choose another child, and propose to its source one it lacks",
        ),
        (
            "a-one-of",
            "R4",
            ".statute/rules/20.toml: error: `overrides` names \"R4\", a one-of; only a rule can be overridden: in place of a one-of's child, choose another child, and propose to its source one it lacks",
        ),
        (
            "missing",
            "R99",
            ".statute/rules/20.toml: error: `overrides` names \"R99\", which does not exist in source `R`",
        ),
    ] {
        let dir = repo(
            name,
            &manifest(""),
            &[("rules", overriding(20, target, ""))],
        );
        fails(&dir, error);
    }
    let both = edit(
        overriding(20, "R2", ""),
        "overrides",
        "parent = 1\noverrides",
    );
    let dir = repo("both", &manifest(""), &[("rules", both)]);
    fails(
        &dir,
        ".statute/rules/20.toml: error: a rule with `overrides` has no `parent`: it stands where the rule it overrides stands",
    );
    let same = edit(rule(20, 1, ""), "parent = 1", "overrides = 3");
    let dir = repo("same-source", &manifest(""), &[("rules", same)]);
    fails(
        &dir,
        ".statute/rules/20.toml: error: `overrides` names 3, a node of this source; a rule overrides a rule of another source, written \"R200\" or \"team:7\", and within one source a rule is changed in place or superseded",
    );
}

#[test]
fn two_rules_cannot_override_one() {
    let dir = repo(
        "two",
        &manifest("\"team:7\", \"20\""),
        &[
            ("team/rules", overriding(7, "R2", "")),
            ("rules", overriding(20, "R2", "")),
        ],
    );
    fails(
        &dir,
        "statute.toml: error: rule R2 (Rule 2) is overridden by team:7 (Rule 7) (listed in `rules`) and L20 (Rule 20) (listed in `rules`); one rule at most may override it, so keep one",
    );
}

#[test]
fn an_override_of_a_rule_nothing_else_enables_warns() {
    let dir = repo(
        "alone",
        &manifest("\"20\""),
        &[("rules", overriding(20, "R2", ""))],
    );
    let run = statute(&dir, &["lock"]);
    ok(&run);
    assert!(
        run.stderr.contains("statute.toml: warning: rule L20 (Rule 20) overrides R2 (Rule 2), which nothing else in the rule set enables; L20 is compiled where R2 would stand"),
        "{}",
        run.stderr
    );
    // With a rule that brings R2 in, no warning.
    let dir = repo(
        "not-alone",
        &manifest("\"R9\", \"20\""),
        &[("rules", overriding(20, "R2", ""))],
    );
    let run = statute(&dir, &["lock"]);
    ok(&run);
    assert!(!run.stderr.contains("warning"), "{}", run.stderr);
}

#[test]
fn the_overridden_rule_counts_for_its_one_of() {
    let l21 = ("rules", overriding(21, "R5", ""));
    // R11 requires one-of R4, and the child L21 stands for satisfies it.
    let dir = repo(
        "one-of",
        &manifest("\"R11\", \"21\""),
        std::slice::from_ref(&l21),
    );
    ok(&statute(&dir, &["lock"]));
    assert_eq!(locked(&dir), ["R5 by L21", "R11", "L21"]);
    // And R5's sibling is still excluded.
    let dir = repo("one-of-sibling", &manifest("\"R6\", \"21\""), &[l21]);
    let run = statute(&dir, &["lock"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr
            .contains("at most one child of one-of R4 (One-of 4) may be enabled"),
        "{}",
        run.stderr
    );
}

#[test]
fn the_overridden_rule_has_no_conflicts_and_no_check() {
    let failing = (
        "r/rules",
        edit(
            rule(2, 1, "requires = [8]"),
            "check = \"review\"",
            "check = \"command\"\nrun = \"exit 1\"\ntools = [\"sh\"]",
        ),
    );
    let l20 = (
        "rules",
        overriding(20, "R2", "uses = [\"instructions\", \"review\"]"),
    );
    let dir = repo(
        "conflict-check",
        &manifest("\"R10\", \"R3\", \"20\""),
        &[failing.clone(), l20],
    );
    ok(&statute(&dir, &["lock"]));
    // A repository of its own, so that `check` does not ask about the
    // one the tests run in.
    let git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .status()
        .unwrap();
    assert!(git.success());
    let run = statute(&dir, &["check"]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(!run.stdout.contains("R2 (Rule 2)"), "{}", run.stdout);
    assert!(
        run.stdout.contains("review        L20 (Rule 20)"),
        "{}",
        run.stdout
    );
    // Without the override, R10 and R2 conflict.
    let dir = repo("conflict-without", &manifest("\"R10\", \"R3\""), &[failing]);
    let run = statute(&dir, &["lock"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains("conflict (R10 says so)"),
        "{}",
        run.stderr
    );
}

#[test]
fn upgrade_asks_whether_an_override_still_holds() {
    let l20 = ("rules", overriding(20, "R2", ""));
    let dir = repo("upgrade", &manifest("\"R9\", \"20\""), &[l20]);
    ok(&statute(&dir, &["lock"]));
    write(
        &dir,
        "r/rules/2.toml",
        &edit(rule(2, 1, "requires = [8]"), "revision = 1", "revision = 2").1,
    );
    let run = statute(&dir, &["upgrade"]);
    ok(&run);
    assert!(
        run.stdout.contains("changed: rule R2 (Rule 2): revision 1 -> 2; L20 overrides it, so check that the override still holds\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn a_pack_enters_a_one_of_through_the_child_an_overriding_rule_takes() {
    let dir = repo(
        "pack",
        &manifest("\"21\"").replace("rules = [", "packs = [\"R1\"]\nrules = ["),
        &[
            ("rules", overriding(21, "R5", "")),
            // Not in conflict with R2, which the pack takes as well.
            ("r/rules", rule(10, 1, "")),
        ],
    );
    let run = statute(&dir, &["lock"]);
    ok(&run);
    // The pack takes R5, which L21 overrides; not R6.
    let rules = locked(&dir);
    assert!(rules.contains(&"R5 by L21".to_owned()), "{rules:?}");
    assert!(!rules.contains(&"R6".to_owned()), "{rules:?}");
    assert!(!run.stderr.contains("warning"), "{}", run.stderr);
}

#[test]
fn a_rule_does_not_override_a_rule_it_is_enabled_through() {
    let r13 = ("r/rules", rule(13, 1, "requires = [\"team:6\"]"));
    let dir = repo(
        "through",
        &manifest("\"R13\""),
        &[r13, ("team/rules", overriding(6, "R13", ""))],
    );
    fails(
        &dir,
        "team/rules/6.toml: error: `overrides` names \"R13\", which brings in team:6 along `requires`; a rule cannot take the place of a rule it is enabled through",
    );
    // Without the requirement, the override holds.
    let dir = repo(
        "through-not",
        &manifest("\"R13\", \"team:6\""),
        &[
            ("r/rules", rule(13, 1, "")),
            ("team/rules", overriding(6, "R13", "")),
        ],
    );
    ok(&statute(&dir, &["lock"]));
    assert_eq!(locked(&dir), ["R13 by team:6", "team:6"]);
}

#[test]
fn overrides_that_do_not_settle_fail() {
    // R13 brings in team:6, which overrides R14; R14 brings in here:20,
    // which overrides R13. Each override takes away what brings in the
    // other.
    let nodes = [
        ("r/rules", rule(13, 1, "requires = [\"team:6\"]")),
        ("r/rules", rule(14, 1, "requires = [\"here:20\"]")),
        ("team/rules", overriding(6, "R14", "")),
        ("here/rules", overriding(20, "R13", "")),
    ];
    let dir = repo("unsettled", &manifest("\"R13\", \"R14\""), &nodes);
    fails(
        &dir,
        "statute.toml: error: the overrides do not settle: here:20 (Rule 20), team:6 (Rule 6) come in only through rules that are overridden once they are in; list them in `rules`",
    );
    // Listed in `rules`, they settle.
    let dir = repo(
        "settled",
        &manifest("\"R13\", \"R14\", \"team:6\", \"here:20\""),
        &nodes,
    );
    ok(&statute(&dir, &["lock"]));
    assert_eq!(
        locked(&dir),
        ["R13 by here:20", "R14 by team:6", "here:20", "team:6"]
    );
}

#[test]
fn a_citation_of_an_overridden_rule_is_the_rule_in_its_place() {
    let r9 = (
        "r/rules",
        edit(
            rule(9, 1, "requires = [2]"),
            "Do the thing 9.",
            "Do the thing 9, after {@rule:2}.",
        ),
    );
    let dir = repo(
        "citation",
        &manifest("\"R9\", \"20\""),
        &[r9, ("rules", overriding(20, "R2", ""))],
    );
    ok(&statute(&dir, &["lock"]));
    ok(&statute(&dir, &["compile"]));
    let out = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(
        out.contains("Do the thing 9, after [Rule 20](#rule-L20).\n"),
        "{out}"
    );
}
