//! `statute compile`, run as a user runs it, in repositories written under
//! Cargo's test directory; and on this repository's own rules.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, one_of, rule};
use statute::node::original_hash;

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";
const CONSTITUTION_EN: &str = "# Constitution\n\nAgents follow it.\n";
const CONSTITUTION_ZH: &str = "# 宪法\n\nagent 遵守它。\n";

/// The constitution's sources, as far as citations read them: the
/// provisions' numbers and IDs, `(path, text)`.
const ARTICLES: [(&str, &str); 5] = [
    ("04-authority/article.toml", "id = \"authority\"\n"),
    (
        "04-authority/01-principals.toml",
        "id = \"principals\"\n\n[[items]]\nid = \"scope\"\nnumber = \"1\"\n\n[[items]]\nid = \"competent\"\nnumber = \"2\"\n",
    ),
    ("04-authority/02a-late.toml", "id = \"late\"\n"),
    ("08-supremacy/article.toml", "id = \"supremacy\"\n"),
    (
        "08-supremacy/07-precedence.toml",
        "id = \"precedence\"\n\n[[items]]\nid = \"first\"\nnumber = \"1\"\n\n[[items]]\nid = \"second\"\nnumber = \"1a\"\n",
    ),
];

/// Domain 1: rule 2, rule 3 requiring it, one-of 4 with children 5 and 6.
/// Domain 9: rule 10, which requires the one-of. Domain 11: rule 12.
fn nodes() -> Vec<(u64, String)> {
    vec![
        domain(1),
        rule(2, 1, ""),
        rule(3, 1, "requires = [2]"),
        one_of(4, 1),
        rule(5, 4, ""),
        rule(6, 4, ""),
        domain(9),
        rule(10, 9, "requires = [4]"),
        domain(11),
        rule(12, 11, ""),
    ]
}

fn manifest(compile: &str) -> String {
    format!(
        "constitution = {{ source = \"R\", edition = \"2026-10\" }}\npublic = false\nrules = [\"R3\", \"R5\", \"R10\"]\n\n[sources]\nR = {{ path = \".\" }}\n{compile}"
    )
}

fn repo(name: &str, manifest: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("compile")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("rules")).unwrap();
    let generated = dir.join("constitution/2026-10/generated");
    std::fs::create_dir_all(&generated).unwrap();
    std::fs::write(dir.join("constitution/2026-10/index.toml"), INDEX).unwrap();
    std::fs::write(generated.join("constitution.en.agent.md"), CONSTITUTION_EN).unwrap();
    std::fs::write(generated.join("constitution.zh.agent.md"), CONSTITUTION_ZH).unwrap();
    let articles = dir.join("constitution/2026-10/articles");
    for (path, text) in ARTICLES {
        let path = articles.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    std::fs::write(dir.join("statute.toml"), manifest).unwrap();
    for (id, text) in nodes() {
        std::fs::write(dir.join(format!("rules/{id}.toml")), text).unwrap();
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

/// Locks and compiles into `out.md`, and returns it.
fn compiled(dir: &Path) -> String {
    ok(&statute(dir, &["lock"]));
    ok(&statute(dir, &["compile", "--output", "out.md"]));
    std::fs::read_to_string(dir.join("out.md")).unwrap()
}

const RULES_EN: &str = r#"# Rules

<a id="rule-R1"></a>
## Domain 1 (R1)

<a id="rule-R2"></a>
### Rule 2 (R2)

Do the thing 2.

<a id="rule-R3"></a>
### Rule 3 (R3)

Do the thing 3.

<a id="rule-R4"></a>
### One-of 4 (R4)

<a id="rule-R5"></a>
#### Rule 5 (R5)

Do the thing 5.

<a id="rule-R9"></a>
## Domain 9 (R9)

<a id="rule-R10"></a>
### Rule 10 (R10)

Do the thing 10.

Requires: [One-of 4](#rule-R4)
"#;

#[test]
fn compiles_the_rules_along_the_tree() {
    let dir = repo("tree", &manifest(""));
    let expected = format!(
        "<!-- Written by statute from statute.toml and statute.lock; do not edit by hand. -->\n\nConstitution: edition 2026-10, revision 1, from source `R` (`.`).\n\n# Constitution\n\nAgents follow it.\n\n{RULES_EN}"
    );
    assert_eq!(compiled(&dir), expected);
    // AGENTS.md is written only without --output.
    assert!(!dir.join("AGENTS.md").exists());
}

#[test]
fn a_rule_only_for_review_is_not_compiled() {
    let dir = repo(
        "review-only",
        &manifest("").replace("\"R10\"]", "\"R10\", \"R12\"]"),
    );
    // Rule 12, the only rule of domain 11, goes to the reviewer alone;
    // rule 10 requires and cites it.
    let r12 = rule(12, 11, "uses = [\"review\"]");
    let r10 = edit(
        rule(10, 9, "requires = [4, 12]"),
        "Do the thing 10.",
        "Do the thing 10, after {@rule:12}.",
    );
    for (id, text) in [r12, r10] {
        std::fs::write(dir.join(format!("rules/{id}.toml")), text).unwrap();
    }
    let out = compiled(&dir);
    assert!(!out.contains("(R12)\n"), "{out}");
    assert!(!out.contains("Domain 11"), "{out}");
    assert!(
        out.contains("Do the thing 10, after Rule 12 (rule R12).\n\nRequires: [One-of 4](#rule-R4), Rule 12 (rule R12)\n"),
        "{out}"
    );
}

#[test]
fn the_same_input_compiles_to_the_same_bytes() {
    let dir = repo("same", &manifest(""));
    let first = compiled(&dir);
    assert_eq!(compiled(&dir), first);
}

#[test]
fn the_rule_set_adds_the_reason_and_the_verification() {
    let dir = repo(
        "parts",
        &manifest("\n[compile]\nagents-md = [\"statement\", \"reason\", \"verification\"]\n"),
    );
    let out = compiled(&dir);
    assert!(
        out.contains("### Rule 2 (R2)\n\nDo the thing 2.\n\nReason: Because.\n\nVerification: Violated when the thing is not done.\n"),
        "{out}"
    );
    let plain = compiled(&repo("parts-plain", &manifest("")));
    assert!(!plain.contains("Reason:"), "{plain}");
}

#[test]
fn compiles_in_the_language_of_the_rule_set() {
    let dir = repo(
        "zh",
        &manifest("\n[compile]\nlang = \"zh\"\nagents-md = [\"statement\", \"reason\"]\n"),
    );
    let out = compiled(&dir);
    assert!(
        out.contains("依据的宪法：2026-10 版，修订号 1，来自规定源 `R` (`.`)。\n\n# 宪法\n"),
        "{out}"
    );
    assert!(out.contains("\n# 规定\n"), "{out}");
    // No translation: the original.
    assert!(out.contains("Do the thing 2.\n\n理由：Because.\n"), "{out}");
    assert!(out.contains("依赖：[One-of 4](#rule-R4)"), "{out}");
}

#[test]
fn compile_needs_a_current_lock_file() {
    let dir = repo("no-lock", &manifest(""));
    let run = statute(&dir, &["compile", "--output", "out.md"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains("cannot read statute.lock"),
        "{}",
        run.stderr
    );
    ok(&statute(&dir, &["lock"]));
    let rule = dir.join("rules/3.toml");
    let text = std::fs::read_to_string(&rule).unwrap();
    std::fs::write(
        &rule,
        text.replace("Do the thing 3.", "Do another thing 3."),
    )
    .unwrap();
    let run = statute(&dir, &["compile", "--output", "out.md"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains(
            "statute.lock is not what statute.toml and the rules give now; run `statute lock` first"
        ),
        "{}",
        run.stderr
    );
    assert!(!dir.join("out.md").exists());
}

#[test]
fn compile_says_which_constitution_file_is_missing() {
    let dir = repo("no-constitution", &manifest("\n[compile]\nlang = \"fr\"\n"));
    ok(&statute(&dir, &["lock"]));
    let run = statute(&dir, &["compile", "--output", "out.md"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains(
            "cannot read constitution/2026-10/generated/constitution.fr.agent.md of source `R`"
        ) && run.stderr.contains("`uv run constitution-build` writes it"),
        "{}",
        run.stderr
    );
}

#[test]
fn this_repo_compiles_the_same_twice_with_every_citation_rendered() {
    // This repository's constitution and rules, every domain as a pack,
    // with every part of each rule, compiled in each language into a
    // directory under Cargo's test directory.
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let (source, _) = statute::source::Source::load(&repo_root.join("rules"), "rules");
    let is_domain = |id: &statute::node::Id| {
        source
            .nodes
            .get(id)
            .is_some_and(|n| n.kind == statute::node::Kind::Domain)
    };
    // A domain under a domain is a subdomain, a heading of level three.
    let levels: BTreeMap<statute::node::Id, &str> = source
        .nodes
        .values()
        .filter(|n| n.kind == statute::node::Kind::Domain)
        .map(|n| {
            let level = if n.parent.as_ref().is_some_and(is_domain) {
                "###"
            } else {
                "##"
            };
            (n.id, level)
        })
        .collect();
    let domains: Vec<String> = levels.keys().map(|id| format!("\"R{id}\"")).collect();
    assert!(!domains.is_empty());
    // The rules cite both rules and the constitution, so that the check
    // that none of it is left as written below can fail.
    let sources: String = std::fs::read_dir(repo_root.join("rules"))
        .unwrap()
        .map(|e| std::fs::read_to_string(e.unwrap().path()).unwrap())
        .collect();
    assert!(sources.contains("{@rule:") && sources.contains("{@constitution:"));
    for lang in ["en", "zh"] {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("compile")
            .join(format!("this-repo-{lang}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("statute.toml"),
            format!(
                "constitution = {{ source = \"R\", edition = \"2026-10\" }}\npublic = false\npacks = [{}]\n\n[sources]\nR = {{ path = {:?} }}\n\n[compile]\nlang = \"{lang}\"\nagents-md = [\"statement\", \"reason\", \"verification\"]\n",
                domains.join(", "),
                repo_root.to_str().unwrap()
            ),
        )
        .unwrap();
        let first = compiled(&dir);
        assert_eq!(compiled(&dir), first);
        let constitution = std::fs::read_to_string(repo_root.join(format!(
            "constitution/2026-10/generated/constitution.{lang}.agent.md"
        )))
        .unwrap();
        assert!(first.contains(constitution.trim_end()));
        assert!(
            !first.contains("{@"),
            "{lang}: a citation is left as written"
        );
        for (id, level) in &levels {
            assert!(
                first.contains(&format!("<a id=\"rule-R{id}\"></a>\n{level} ")),
                "{lang}: domain {id}"
            );
        }
    }
}

#[test]
fn the_deepest_tree_reaches_level_six() {
    let dir = repo(
        "deep",
        &manifest("").replace("\"R3\", \"R5\", \"R10\"", "\"R105\""),
    );
    // Under domain 1 (level 2): one-of 102, its option domain 103, one-of
    // 104 in it, and rule 105, its option, at level 6.
    let option = edit(domain(103), "lang = \"en\"", "lang = \"en\"\nparent = 102");
    for (id, text) in [one_of(102, 1), option, one_of(104, 103), rule(105, 104, "")] {
        std::fs::write(dir.join(format!("rules/{id}.toml")), text).unwrap();
    }
    let out = compiled(&dir);
    assert!(out.contains("\n#### Domain 103 (R103)\n"), "{out}");
    assert!(out.contains("\n###### Rule 105 (R105)\n"), "{out}");
}

/// Rule 3 of `nodes`, with a Chinese translation.
fn rule_3_in_chinese() -> String {
    let hash = original_hash(&[
        "Rule 3",
        "Do the thing 3.",
        "Because.\n",
        "Violated when the thing is not done.",
    ]);
    let r = rule(3, 1, "requires = [2]");
    let r = edit(r, "en = \"Rule 3\"", "en = \"Rule 3\"\nzh = \"规定 3\"");
    let r = edit(
        r,
        "en = \"Do the thing 3.\"",
        "en = \"Do the thing 3.\"\nzh = \"做事 3。\"",
    );
    let r = edit(r, "Because.\n'\'\'", "Because.\n'\'\'\nzh = \"因为。\"");
    let r = edit(
        r,
        "en = \"Violated when the thing is not done.\"",
        "en = \"Violated when the thing is not done.\"\nzh = \"没做即为违反。\"",
    );
    format!("{}\n[translations]\nzh = \"{hash}\"\n", r.1)
}

#[test]
fn a_translation_is_used_in_its_language() {
    let compile = "\n[compile]\nlang = \"zh\"\nagents-md = [\"statement\", \"reason\"]\n";
    let dir = repo("translated", &manifest(compile));
    std::fs::write(dir.join("rules/3.toml"), rule_3_in_chinese()).unwrap();
    let out = compiled(&dir);
    assert!(
        out.contains("### 规定 3 (R3)\n\n做事 3。\n\n理由：因为。\n"),
        "{out}"
    );
    // Rule 2 has no translation: its original.
    assert!(
        out.contains("### Rule 2 (R2)\n\nDo the thing 2.\n"),
        "{out}"
    );
    // In English, rule 3 is in its original.
    let dir = repo("translated-en", &manifest(""));
    std::fs::write(dir.join("rules/3.toml"), rule_3_in_chinese()).unwrap();
    let out = compiled(&dir);
    assert!(
        out.contains("### Rule 3 (R3)\n\nDo the thing 3.\n"),
        "{out}"
    );
}

/// Domain 1 of `nodes`, with an applicability that cites rule 3,
/// translated into Chinese.
fn domain_1_with_applicability() -> String {
    let applicability = "For every repository; see {@rule:3}.";
    let hash = original_hash(&["Domain 1", applicability]);
    let d = edit(
        domain(1),
        "en = \"Domain 1\"",
        "en = \"Domain 1\"\nzh = \"领域 1\"",
    );
    format!(
        "{}\n[applicability]\nen = \"{applicability}\"\nzh = \"所有仓库，见{{@rule:3}}。\"\n\n[translations]\nzh = \"{hash}\"\n",
        d.1
    )
}

#[test]
fn a_domains_applicability_follows_its_heading() {
    let dir = repo("applicability-en", &manifest(""));
    std::fs::write(dir.join("rules/1.toml"), domain_1_with_applicability()).unwrap();
    let out = compiled(&dir);
    assert!(
        out.contains("## Domain 1 (R1)\n\nFor every repository; see [Rule 3](#rule-R3).\n\n<a id=\"rule-R2\"></a>\n"),
        "{out}"
    );

    let compile = "\n[compile]\nlang = \"zh\"\n";
    let dir = repo("applicability-zh", &manifest(compile));
    std::fs::write(dir.join("rules/1.toml"), domain_1_with_applicability()).unwrap();
    let out = compiled(&dir);
    assert!(
        out.contains("## 领域 1 (R1)\n\n所有仓库，见[Rule 3](#rule-R3)。\n"),
        "{out}"
    );
}

#[test]
fn compile_replaces_a_generated_file_and_leaves_a_hand_written_one() {
    let dir = repo("default-path", &manifest(""));
    ok(&statute(&dir, &["lock"]));
    // An earlier output of statute is replaced.
    let old = format!("{}\n\nold\n", statute::compile::GENERATED);
    std::fs::write(dir.join("AGENTS.md"), &old).unwrap();
    let run = statute(&dir, &["compile"]);
    ok(&run);
    let written = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(written.starts_with(statute::compile::GENERATED) && written.contains("# Rules\n"));
    // A file written by hand is not.
    let hand = "# hand-written\n\nKeep me.\n";
    std::fs::write(dir.join("AGENTS.md"), hand).unwrap();
    let run = statute(&dir, &["compile"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains("AGENTS.md: not written by statute, so it is left as it is and nothing is written; write elsewhere with --output"),
        "{}",
        run.stderr
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("AGENTS.md")).unwrap(),
        hand
    );
}

/// Rule 2 of `nodes`, whose statement cites rule 3 (enabled), rule 6 (not
/// enabled), and sections and items of the constitution.
fn citing_rule_2() -> String {
    edit(
        rule(2, 1, ""),
        "en = \"Do the thing 2.\"",
        "en = \"Do {@rule:3}, not {@rule:6}, under {@constitution:principals}, {@constitution:principals.competent}, {@constitution:precedence.first}, {@constitution:precedence.second}, {@constitution:late} and {@constitution:supremacy}; keep {braces} and {@ this}.\"",
    )
    .1
}

#[test]
fn citations_render_as_titles_and_numbers() {
    let dir = repo("cite-en", &manifest(""));
    std::fs::write(dir.join("rules/2.toml"), citing_rule_2()).unwrap();
    let out = compiled(&dir);
    assert!(
        out.contains("### Rule 2 (R2)\n\nDo [Rule 3](#rule-R3), not Rule 6 (rule R6), under Art. IV §1, Art. IV §1(2), Art. VIII §7(1), Art. VIII §7(1a), Art. IV §2a and Art. VIII; keep {braces} and {@ this}.\n"),
        "{out}"
    );
    // The heading of rule 3 is the link's target; rule 6 has none.
    assert!(out.contains("<a id=\"rule-R3\"></a>"), "{out}");
    assert!(!out.contains("<a id=\"rule-R6\"></a>"), "{out}");

    let compile = "\n[compile]\nlang = \"zh\"\n";
    let dir = repo("cite-zh", &manifest(compile));
    std::fs::write(dir.join("rules/2.toml"), citing_rule_2()).unwrap();
    std::fs::write(dir.join("rules/3.toml"), rule_3_in_chinese()).unwrap();
    let out = compiled(&dir);
    assert!(
        out.contains("Do [规定 3](#rule-R3), not Rule 6 (规定 R6), under 第四条第一款, 第四条第一款第二项, 第八条第七款第一项, 第八条第七款第一项之一, 第四条第二款之一 and 第八条;"),
        "{out}"
    );
}

#[test]
fn a_citation_of_nothing_fails_and_names_where_it_is() {
    // A citation of a rule, or one without a namespace, fails when the
    // source is read; one of a provision, when the constitution is.
    for (citation, command, message) in [
        (
            "{@rule:99}",
            "lock",
            "rules/2.toml: error: [text.statement] cites `{@rule:99}`, which does not exist in this source",
        ),
        (
            "{@constitution:principals.nobody}",
            "compile",
            "rules/2.toml: statement: cites `{@constitution:principals.nobody}`, which is not in edition 2026-10 of the constitution",
        ),
        (
            "{@principals}",
            "lock",
            "rules/2.toml: error: [text.statement] cites `{@principals}` without `rule:` or `constitution:`",
        ),
    ] {
        let dir = repo("cite-missing", &manifest(""));
        let text = edit(
            rule(2, 1, ""),
            "Do the thing 2.",
            &format!("Do {citation}."),
        )
        .1;
        std::fs::write(dir.join("rules/2.toml"), text).unwrap();
        let lock = statute(&dir, &["lock"]);
        let run = if command == "lock" {
            lock
        } else {
            ok(&lock);
            statute(&dir, &["compile", "--output", "out.md"])
        };
        assert_eq!(run.code, 1);
        assert!(run.stderr.contains(message), "{}", run.stderr);
        assert!(!dir.join("out.md").exists());
    }
}

#[derive(serde::Deserialize)]
struct ApplicabilityCases {
    cases: Vec<ApplicabilityCase>,
}

#[derive(serde::Deserialize)]
struct ApplicabilityCase {
    name: String,
    passes: bool,
    node: String,
    /// What statute says of a case that fails.
    statute: Option<String>,
}

/// The compiler's cases of `[applicability]`
/// (`tools/compiler/tests/applicability_cases.toml`): node 20 with rule
/// 21 under it enabled, so that a domain's applicability is compiled.
#[test]
fn the_compilers_applicability_cases() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/compiler/tests/applicability_cases.toml");
    let cases: ApplicabilityCases =
        toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(!cases.cases.is_empty());
    for case in cases.cases {
        // Node 20's parent is 2, a domain in the cases; rule 21 is under
        // node 20 unless that is a rule, which is then enabled itself.
        let (enabled, parent) = if case.node.contains("kind = \"rule\"") {
            ("\"R20\"", 1)
        } else {
            ("\"R21\"", 20)
        };
        let manifest = manifest("").replace("\"R3\", \"R5\", \"R10\"", enabled);
        let dir = repo("applicability", &manifest);
        std::fs::write(dir.join("rules/2.toml"), domain(2).1).unwrap();
        std::fs::write(dir.join("rules/3.toml"), rule(3, 1, "").1).unwrap();
        std::fs::write(dir.join("rules/20.toml"), &case.node).unwrap();
        std::fs::write(dir.join("rules/21.toml"), rule(21, parent, "").1).unwrap();
        let lock = statute(&dir, &["lock"]);
        let run = if lock.code == 0 {
            statute(&dir, &["compile", "--output", "out.md"])
        } else {
            lock
        };
        assert_eq!(run.code == 0, case.passes, "{}: {}", case.name, run.stderr);
        // Rule 21 draws an error too when node 20 cannot be read at all.
        if !case.passes {
            let reason = case.statute.as_deref().unwrap();
            assert!(
                run.stderr.contains("rules/20.toml") && run.stderr.contains(reason),
                "{}: {}",
                case.name,
                run.stderr
            );
        }
    }
}

/// Writes `skills/<name>/SKILL.md` into the source, with `rules` as its
/// declaration (none when empty), and a file below it.
fn skill(dir: &Path, name: &str, rules: &str) {
    let skill = dir.join("skills").join(name);
    std::fs::create_dir_all(skill.join("references")).unwrap();
    let metadata = if rules.is_empty() {
        String::new()
    } else {
        format!("metadata:\n  statute-rules: \"{rules}\"\n")
    };
    std::fs::write(
        skill.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Skill {name}.\n{metadata}---\n\n# {name}\n"),
    )
    .unwrap();
    std::fs::write(
        skill.join("references/more.md"),
        format!("More on {name}.\n"),
    )
    .unwrap();
}

#[cfg(unix)]
/// A repository whose rule 3 is enabled and has the skill `three`, rule 6
/// is not and has the skill `six`, and the skill `none` declares nothing.
fn skills_repo(name: &str) -> PathBuf {
    let dir = repo(name, &manifest(""));
    skill(&dir, "three", "3");
    skill(&dir, "six", "6 12");
    skill(&dir, "none", "");
    dir
}

#[cfg(unix)]
#[test]
fn the_skills_of_enabled_rules_are_copied_and_linked() {
    let dir = skills_repo("skills");
    compiled(&dir);
    let three = dir.join(".agents/skills/three");
    assert_eq!(
        std::fs::read(three.join("SKILL.md")).unwrap(),
        std::fs::read(dir.join("skills/three/SKILL.md")).unwrap()
    );
    assert_eq!(
        std::fs::read_to_string(three.join("references/more.md")).unwrap(),
        "More on three.\n"
    );
    assert!(three.join(".statute-generated").is_file());
    let link = dir.join(".claude/skills/three");
    assert_eq!(
        std::fs::read_link(&link).unwrap(),
        Path::new("../../.agents/skills/three")
    );
    assert_eq!(link.canonicalize().unwrap(), three.canonicalize().unwrap());
    // Rule 6 and rule 12 are not enabled, and `none` implements nothing.
    for name in ["six", "none"] {
        assert!(!dir.join(".agents/skills").join(name).exists(), "{name}");
        assert!(
            dir.join(".claude/skills")
                .join(name)
                .symlink_metadata()
                .is_err(),
            "{name}"
        );
    }
}

#[test]
fn no_enabled_skill_writes_no_skill_directory() {
    let dir = repo("no-skills", &manifest(""));
    skill(&dir, "six", "6");
    compiled(&dir);
    assert!(!dir.join(".agents").exists());
    assert!(!dir.join(".claude").exists());
}

#[test]
fn a_skill_with_flow_style_metadata_fails() {
    let dir = repo("skill-flow-metadata", &manifest(""));
    skill(&dir, "flow", "");
    std::fs::write(
        dir.join("skills/flow/SKILL.md"),
        "---\nname: flow\ndescription: Skill flow.\nmetadata: {statute-rules: \"3\"}\n---\n",
    )
    .unwrap();
    ok(&statute(&dir, &["lock"]));
    let run = statute(&dir, &["compile", "--output", "out.md"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr
            .contains("skills/flow/SKILL.md: line 4: `metadata` is written in flow style"),
        "{}",
        run.stderr
    );
    assert!(!dir.join("out.md").exists());
}

#[test]
fn a_skill_naming_a_missing_rule_fails() {
    let dir = repo("skill-missing-rule", &manifest(""));
    skill(&dir, "lost", "3 99");
    ok(&statute(&dir, &["lock"]));
    let run = statute(&dir, &["compile", "--output", "out.md"]);
    assert_eq!(run.code, 1);
    assert!(
        run.stderr.contains(
            "skills/lost/SKILL.md: `metadata.statute-rules` names rule 99, which is not in source `R`"
        ),
        "{}",
        run.stderr
    );
    assert!(!dir.join("out.md").exists());
}

#[test]
fn the_checks_are_written_next_to_agents_md() {
    let dir = repo("checks", &manifest(""));
    // Rule 5 (enabled) and rule 6 (not) run a command; rule 3 is reviewed.
    for (id, parent) in [(5, 4), (6, 4)] {
        let text = edit(
            rule(id, parent, ""),
            "check = \"review\"",
            &format!(
                "check = \"command\"\nrun = \"./check-{id}.sh --strict\"\ntools = [\"uv\", \"go\"]"
            ),
        )
        .1;
        std::fs::write(dir.join(format!("rules/{id}.toml")), text).unwrap();
    }
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    ok(&statute(&dir, &["lock"]));
    ok(&statute(&dir, &["compile", "--output", "sub/out.md"]));
    let checks = std::fs::read_to_string(dir.join("sub/statute-checks.toml")).unwrap();
    assert_eq!(
        checks,
        format!(
            "{}\n\nstatute = \"{}\"\n\n[[commands]]\nrule = \"R5\"\nrun = \"./check-5.sh --strict\"\ntools = [\"uv\", \"go\"]\n",
            statute::compile::GENERATED_HASH,
            env!("CARGO_PKG_VERSION")
        )
    );
    assert!(!dir.join("statute-checks.toml").exists());
    // Without a command, the list is empty.
    let dir = repo("checks-none", &manifest(""));
    compiled(&dir);
    let checks = std::fs::read_to_string(dir.join("statute-checks.toml")).unwrap();
    assert!(checks.ends_with("\ncommands = []\n"), "{checks}");
}

#[cfg(unix)]
/// Every file and link below `dir`, with its content or target, sorted.
fn tree(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for entry in std::fs::read_dir(&d).unwrap() {
            let path = entry.unwrap().path();
            let meta = path.symlink_metadata().unwrap();
            let shown = path.strip_prefix(dir).unwrap().to_path_buf();
            if meta.is_symlink() {
                let target = std::fs::read_link(&path).unwrap();
                out.push((shown, format!("-> {}", target.display())));
            } else if meta.is_dir() {
                todo.push(path);
            } else {
                out.push((shown, std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

#[cfg(unix)]
#[test]
fn the_whole_output_compiles_to_the_same_bytes() {
    let dir = skills_repo("skills-same");
    compiled(&dir);
    let first = tree(&dir);
    assert!(first.iter().any(
        |(p, t)| p == Path::new(".claude/skills/three") && t == "-> ../../.agents/skills/three"
    ));
    compiled(&dir);
    assert_eq!(tree(&dir), first);
}

#[cfg(unix)]
#[test]
fn a_skill_no_longer_enabled_goes_with_its_link_and_nothing_else() {
    let dir = skills_repo("skills-stale");
    compiled(&dir);
    // Something statute did not write, which must stay.
    let mine = dir.join(".agents/skills/mine");
    std::fs::create_dir_all(&mine).unwrap();
    std::fs::write(mine.join("SKILL.md"), "mine\n").unwrap();
    std::fs::write(
        dir.join("statute.toml"),
        manifest("").replace("\"R3\", ", ""),
    )
    .unwrap();
    compiled(&dir);
    assert!(!dir.join(".agents/skills/three").exists());
    assert!(dir.join(".claude/skills/three").symlink_metadata().is_err());
    assert_eq!(
        std::fs::read_to_string(mine.join("SKILL.md")).unwrap(),
        "mine\n"
    );
}

#[cfg(unix)]
#[test]
fn compile_writes_nothing_where_something_is_not_its_own() {
    // Each case puts one thing statute did not write where it would
    // write, and leaves the rest empty.
    const NOT_OURS: &str = "not written by statute, so it is left as it is and nothing is written";
    const NOT_A_DIRECTORY: &str = "not a directory but a link or a file, so nothing is written; statute writes one link per skill in .claude/skills";
    type Place = fn(&Path);
    let cases: [(&str, Place, &str); 6] = [
        (
            "checks-file",
            |d| std::fs::write(d.join("statute-checks.toml"), "mine\n").unwrap(),
            NOT_OURS,
        ),
        (
            "skill-directory",
            |d| {
                std::fs::create_dir_all(d.join(".agents/skills/three")).unwrap();
                std::fs::write(d.join(".agents/skills/three/SKILL.md"), "mine\n").unwrap();
            },
            NOT_OURS,
        ),
        (
            "link-directory",
            |d| {
                std::fs::create_dir_all(d.join(".claude/skills/three")).unwrap();
                std::fs::write(d.join(".claude/skills/three/SKILL.md"), "mine\n").unwrap();
            },
            NOT_OURS,
        ),
        (
            "link-elsewhere",
            |d| {
                std::fs::create_dir_all(d.join(".claude/skills")).unwrap();
                std::os::unix::fs::symlink("../../skills/three", d.join(".claude/skills/three"))
                    .unwrap();
            },
            NOT_OURS,
        ),
        // The layout of a repository that links the whole directory.
        (
            "links-directory-a-link",
            |d| {
                std::fs::create_dir_all(d.join(".agents/skills")).unwrap();
                std::fs::create_dir_all(d.join(".claude")).unwrap();
                std::os::unix::fs::symlink("../.agents/skills", d.join(".claude/skills")).unwrap();
            },
            NOT_A_DIRECTORY,
        ),
        (
            "skills-directory-a-link",
            |d| {
                std::fs::create_dir_all(d.join("elsewhere")).unwrap();
                std::fs::create_dir_all(d.join(".agents")).unwrap();
                std::os::unix::fs::symlink("../elsewhere", d.join(".agents/skills")).unwrap();
            },
            NOT_A_DIRECTORY,
        ),
    ];
    for (name, place, message) in cases {
        let dir = skills_repo(&format!("refuse-{name}"));
        ok(&statute(&dir, &["lock"]));
        place(&dir);
        let before = tree(&dir);
        // A second run finds the same and does the same.
        for round in [1, 2] {
            let run = statute(&dir, &["compile"]);
            assert_eq!(run.code, 1, "{name} {round}");
            assert!(
                run.stderr.contains(message),
                "{name} {round}: {}",
                run.stderr
            );
            assert_eq!(tree(&dir), before, "{name} {round}");
            assert!(!dir.join("AGENTS.md").exists(), "{name} {round}");
        }
    }
}

#[cfg(unix)]
#[test]
fn a_run_stopped_halfway_is_finished_by_the_next() {
    use std::os::unix::fs::PermissionsExt as _;
    let whole = {
        let dir = skills_repo("halfway-whole");
        compiled(&dir);
        tree(&dir)
    };
    let dir = skills_repo("halfway");
    ok(&statute(&dir, &["lock"]));
    // A file of the skill that cannot be read stops the copy halfway.
    let more = dir.join("skills/three/references/more.md");
    std::fs::set_permissions(&more, std::fs::Permissions::from_mode(0o000)).unwrap();
    // Root reads the file anyway, and nothing stops the copy. CI does not
    // run as root, so there the test must not skip.
    if std::fs::File::open(&more).is_ok() {
        std::fs::set_permissions(&more, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(
            std::env::var_os("CI").is_none(),
            "an unreadable file is read in CI; the test would not run"
        );
        eprintln!("skipped: an unreadable file is read anyway (running as root?)");
        return;
    }
    let run = statute(&dir, &["compile", "--output", "out.md"]);
    std::fs::set_permissions(&more, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(dir.join(".agents/skills/three/SKILL.md").is_file());
    assert!(!dir.join("out.md").exists());
    // The next run takes the half-written skill for its own and finishes.
    compiled(&dir);
    assert_eq!(tree(&dir), whole);
}
