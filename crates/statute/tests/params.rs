//! Parameters (`docs/design/rule-config.md`, Parameters): their
//! declarations, the placeholders in the rules, and their values in the
//! rule set, the lock file and the outputs.

use std::path::{Path, PathBuf};
use std::process::Command;

use statute::diag::{Diagnostics, Severity};
use statute::source::{Source, Sources};

mod common;
use common::{domain, edit, param, rule};

/// Rule `id` under domain 1 whose statement is `text`.
fn saying(id: u64, text: &str) -> (u64, String) {
    edit(rule(id, 1, ""), &format!("Do the thing {id}."), text)
}

/// The language parameters of the tests: `language.docs` of scope `repo`
/// and `language.communication` of scope `personal`, both `en`.
fn languages() -> String {
    param("language.docs", "type = \"language\"", "repo", "en")
        + &param(
            "language.communication",
            "type = \"language\"",
            "personal",
            "en",
        )
}

/// One source of `nodes` and `params.toml`, read as `rules/`.
fn source(name: &str, nodes: &[(u64, String)], params: &str) -> (Source, Diagnostics) {
    let mut files: Vec<(String, Vec<u8>)> = nodes
        .iter()
        .map(|(id, text)| (format!("rules/{id}.toml"), text.clone().into_bytes()))
        .collect();
    files.push(("rules/params.toml".to_owned(), params.as_bytes().to_vec()));
    let (mut source, diags) = Source::from_files(files);
    source.name = name.to_owned();
    (source, diags)
}

fn errors(diags: &Diagnostics) -> Vec<String> {
    diags
        .0
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(ToString::to_string)
        .collect()
}

#[track_caller]
fn one_error(diags: &Diagnostics, file: &str, needle: &str) {
    let found = errors(diags);
    assert!(
        found.len() == 1
            && found[0].starts_with(&format!("{file}: error: "))
            && found[0].contains(needle),
        "expected one error in {file} containing {needle:?}, got {found:#?}"
    );
}

#[test]
fn a_source_declares_its_parameters() {
    let params = languages()
        + &param(
            "work-items.label",
            "type = \"string\"",
            "repo",
            "needs-user",
        )
        + &param(
            "python.quotes",
            "type = \"choice\"\nvalues = [\"double\", \"single\"]",
            "repo",
            "double",
        );
    let (s, diags) = source(
        "R",
        &[domain(1), saying(2, "Write in {$language.docs}.")],
        &params,
    );
    assert!(errors(&diags).is_empty(), "{:#?}", diags.0);
    let names: Vec<&str> = s.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "language.docs",
            "language.communication",
            "work-items.label",
            "python.quotes"
        ]
    );
}

#[test]
fn a_parameter_has_its_keys_and_a_value_of_its_type() {
    let docs = param("language.docs", "type = \"language\"", "repo", "en");
    let hash = statute::node::original_hash(&["The language.docs"]);
    for (text, needle) in [
        (
            docs.replace("scope = \"repo\"", "scope = \"repo\"\nlang = \"en\""),
            "`lang` is no key of a parameter",
        ),
        (
            docs.replace("language.docs", "Language.docs"),
            "a name is two words joined by a dot",
        ),
        (
            docs.replace("language.docs", "rules.docs"),
            "`rules` is a key of the rule set already",
        ),
        (
            docs.replace("type = \"language\"", "type = \"choice\""),
            "a `choice` lists its `values`",
        ),
        (
            docs.replace("type = \"language\"", "type = \"string\"\nvalues = [\"a\"]"),
            "`values` belongs to a `choice` only",
        ),
        (
            docs.replace("default = \"en\"", "default = \"fr\""),
            "`default`: `fr` is not a language statute has a name for",
        ),
        (
            param(
                "python.quotes",
                "type = \"choice\"\nvalues = [\"double\", \"single\"]",
                "repo",
                "back",
            ),
            "`default`: `back` is not one of its values",
        ),
        (
            param("work-items.label", "type = \"string\"", "repo", "needs user "),
            "has a space at one end",
        ),
        (
            docs.replace("scope = \"repo\"", "scope = \"team\""),
            "`scope = \"team\"` must be \"repo\" or \"personal\"",
        ),
        (
            docs.clone() + "[parameter.translations]\nzh = \"0\"\n",
            "[translations] has `zh`, but [title] gives no `zh`",
        ),
        (
            docs.replace(
                "en = \"The language.docs\"\n",
                "en = \"The language.docs\"\nzh = \"文档语言\"\n\n[parameter.translations]\nzh = \"0\"\n",
            ),
            &format!("set `zh = \"{hash}\"` in [translations]"),
        ),
        (
            docs.replace("The language.docs", "In {$language.docs}"),
            "[title] `en` holds a placeholder",
        ),
        (docs.clone() + &docs, "another parameter of the file has this name"),
    ] {
        let (s, diags) = source("R", &[domain(1)], &text);
        one_error(&diags, "rules/params.toml", needle);
        assert!(s.params.len() <= 1, "{needle}");
    }
    // The translation with the hash of its original passes.
    let translated = docs.replace(
        "en = \"The language.docs\"\n",
        &format!("en = \"The language.docs\"\nzh = \"文档语言\"\n\n[parameter.translations]\nzh = \"{hash}\"\n"),
    );
    let (_, diags) = source("R", &[domain(1)], &translated);
    assert!(errors(&diags).is_empty(), "{:#?}", diags.0);
}

#[test]
fn placeholders_stand_in_a_rules_title_and_text_only() {
    let params = languages();
    let (_, diags) = source(
        "R",
        &[
            edit(
                domain(1),
                "en = \"Domain 1\"",
                "en = \"In {$language.docs}\"",
            ),
            saying(2, "Write in {$language.docs}."),
        ],
        &params,
    );
    one_error(
        &diags,
        "rules/1.toml",
        "[title] `en` holds `{$language.docs}`; only a rule's title and text hold placeholders, not a domain's",
    );
    let (_, diags) = source(
        "R",
        &[domain(1), saying(2, "Write in {$Language}.")],
        &params,
    );
    one_error(&diags, "rules/2.toml", "`{$Language}` is no placeholder");
    // A translation holds the same placeholders, each as many times.
    let hash = statute::node::original_hash(&[
        "Rule 2",
        "Write in {$language.docs}, always {$language.docs}.",
        "Because.\n",
        "Violated when the thing is not done.",
    ]);
    let translated = |zh: &str| {
        let r = saying(2, "Write in {$language.docs}, always {$language.docs}.");
        let r = edit(r, "en = \"Rule 2\"", "en = \"Rule 2\"\nzh = \"规定 2\"");
        let r = edit(
            r,
            "Write in {$language.docs}, always {$language.docs}.\"",
            &format!("Write in {{$language.docs}}, always {{$language.docs}}.\"\nzh = \"{zh}\""),
        );
        let r = edit(r, "Because.\n'''", "Because.\n'''\nzh = \"因为。\"");
        let r = edit(
            r,
            "en = \"Violated when the thing is not done.\"",
            "en = \"Violated when the thing is not done.\"\nzh = \"没做就算违反。\"",
        );
        (r.0, format!("{}\n[translations]\nzh = \"{hash}\"\n", r.1))
    };
    let (_, diags) = source(
        "R",
        &[
            domain(1),
            translated("用{$language.docs}写，总是{$language.docs}。"),
        ],
        &params,
    );
    assert!(errors(&diags).is_empty(), "{:#?}", diags.0);
    let (_, diags) = source(
        "R",
        &[domain(1), translated("用{$language.docs}写。")],
        &params,
    );
    one_error(
        &diags,
        "rules/2.toml",
        "[text.statement] `zh` holds other placeholders than `en`",
    );
}

#[test]
fn a_rule_uses_its_own_sources_parameters_and_the_first_partys() {
    let check = |team_params: &str, text: &str| {
        let (r, mut diags) = source("R", &[domain(1)], &languages());
        let (mut team, more) = source("team", &[domain(1), saying(2, text)], team_params);
        team.dir = "team/rules".to_owned();
        diags.0.extend(more.0);
        let sources = Sources::new(vec![r, team]);
        sources.check(&mut diags);
        errors(&diags)
    };
    let own = param("team.board", "type = \"string\"", "repo", "Main");
    assert_eq!(
        check(&own, "Use {$team.board} in {$language.docs}."),
        Vec::<String>::new()
    );
    let both = param("language.docs", "type = \"language\"", "repo", "zh");
    let found = check(&both, "Write in {$language.docs}.");
    assert!(
        found.len() == 1
            && found[0].contains("names `{$language.docs}`, which both")
            && found[0].contains("declare; one of them has to rename it"),
        "{found:#?}"
    );
    let found = check(&own, "Write in {$language.none}.");
    assert!(
        found.len() == 1 && found[0].contains("names `{$language.none}`, a parameter that neither"),
        "{found:#?}"
    );
}

// The values, end to end.

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn read(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join(path)).unwrap()
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// A git repository with `R` in `r/`: domain 1, on, with rule 2, which
/// writes documents in `{$language.docs}`, and rule 3, which replies in
/// `{$language.communication}`; and a rule set of `public` with the
/// lines `extra` that enables both, and a global configuration of its
/// own, empty.
fn repo(name: &str, public: bool, extra: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("params")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let work = dir.join("work");
    git(&work, &["init", "-q"]);
    write(&work, "r/constitution/2026-10/index.toml", INDEX);
    write(
        &work,
        "r/constitution/2026-10/generated/constitution.en.agent.md",
        "# Constitution\n",
    );
    write(
        &work,
        "r/constitution/2026-10/articles/04-authority/article.toml",
        "id = \"authority\"\n",
    );
    write(&work, "r/rules/params.toml", &languages());
    for (id, text) in [
        domain(1),
        saying(2, "Write documents in {$language.docs}."),
        saying(3, "Reply in {$language.communication}."),
    ] {
        write(&work, &format!("r/rules/{id}.toml"), &text);
    }
    write(
        &work,
        "statute.toml",
        &format!(
            "constitution = {{ source = \"R\", edition = \"2026-10\" }}\npublic = {public}\nrules = [\"R2\", \"R3\"]\n{extra}\n[sources]\nR = {{ path = \"r\" }}\n"
        ),
    );
    write(&dir, "config/statute/config.toml", "");
    // Git ignores the links of the private output.
    write(
        &work,
        ".git/info/exclude",
        "/CLAUDE.local.md\n/AGENTS.override.md\n",
    );
    work
}

/// Sets the global configuration of the repository `work` to `text`.
fn global(work: &Path, text: &str) {
    write(work.parent().unwrap(), "config/statute/config.toml", text);
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn statute(work: &Path, args: &[&str]) -> Run {
    let base = work.parent().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_statute"))
        .env("HOME", base.join("home"))
        .env("XDG_CONFIG_HOME", base.join("config"))
        .env("XDG_CACHE_HOME", base.join("cache"))
        .env(
            "GIT_CEILING_DIRECTORIES",
            concat!(env!("CARGO_TARGET_TMPDIR"), "/params"),
        )
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .args(args)
        .current_dir(work)
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
fn fails_with(run: &Run, needle: &str) {
    assert_eq!(
        run.code, 1,
        "stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
    assert!(run.stderr.contains(needle), "{}", run.stderr);
}

#[test]
fn the_values_go_into_the_lock_file_and_agents_md() {
    let work = repo("values", false, "[language]\ndocs = \"zh\"\n");
    ok(&statute(&work, &["lock"]));
    assert!(
        read(&work, "statute.lock").ends_with(
            "[[parameters]]\nname = \"language.communication\"\nvalue = \"en\"\n\n[[parameters]]\nname = \"language.docs\"\nvalue = \"zh\"\n"
        ),
        "{}",
        read(&work, "statute.lock")
    );
    ok(&statute(&work, &["compile"]));
    let out = read(&work, "AGENTS.md");
    assert!(out.contains("\nWrite documents in Chinese.\n"), "{out}");
    assert!(out.contains("\nReply in English.\n"), "{out}");
    // In Chinese, a language is named in Chinese.
    let work = repo(
        "values-zh",
        false,
        "[language]\ndocs = \"zh\"\n\n[compile]\nlang = \"zh\"\n",
    );
    write(
        &work,
        "r/constitution/2026-10/generated/constitution.zh.agent.md",
        "# 宪法\n",
    );
    ok(&statute(&work, &["lock"]));
    ok(&statute(&work, &["compile"]));
    let out = read(&work, "AGENTS.md");
    assert!(out.contains("\nWrite documents in 中文.\n"), "{out}");
}

#[test]
fn a_value_names_a_parameter_and_has_its_type() {
    let work = repo("unknown", false, "[language]\nslides = \"zh\"\n");
    fails_with(
        &statute(&work, &["lock"]),
        "statute.toml: error: [language] `slides` names no parameter of the rule set's sources",
    );
    let work = repo("wrong-type", false, "[language]\ndocs = \"fr\"\n");
    fails_with(
        &statute(&work, &["lock"]),
        "statute.toml: error: [language] `docs`: `fr` is not a language statute has a name for",
    );
    let work = repo("not-a-string", false, "[language]\ndocs = 3\n");
    fails_with(
        &statute(&work, &["lock"]),
        "[language] `docs = 3`: a parameter's value is a string",
    );
    // A value no enabled rule uses stays, with a warning.
    let work = repo("unused", false, "[language]\ndocs = \"zh\"\n");
    write(
        &work,
        "statute.toml",
        &read(&work, "statute.toml").replace("[\"R2\", \"R3\"]", "[\"R3\"]"),
    );
    let run = statute(&work, &["lock"]);
    ok(&run);
    assert!(
        run.stderr.contains(
            "statute.toml: warning: [language] `docs` is set, and no enabled rule uses `{$language.docs}`"
        ),
        "{}",
        run.stderr
    );
}

#[test]
fn upgrade_lists_a_changed_value_and_explain_says_where_it_comes_from() {
    let work = repo("upgrade", false, "");
    ok(&statute(&work, &["lock"]));
    let run = statute(&work, &["explain", "language.docs"]);
    ok(&run);
    assert_eq!(
        run.stdout,
        "parameter `language.docs` of source `R`, a language of scope `repo`: The language.docs\n- value: `en` (English), from the default\n- used by: R2 (Rule 2)\n"
    );
    write(
        &work,
        "statute.toml",
        &read(&work, "statute.toml").replace("[sources]", "[language]\ndocs = \"zh\"\n\n[sources]"),
    );
    // The lock file is stale with the new value.
    fails_with(
        &statute(&work, &["compile"]),
        "statute.lock is not what statute.toml and the rules give now",
    );
    let run = statute(&work, &["upgrade"]);
    ok(&run);
    assert!(
        run.stdout
            .contains("changed: parameter `language.docs`: `en` -> `zh`"),
        "{}",
        run.stdout
    );
    let run = statute(&work, &["explain", "language.docs"]);
    assert!(
        run.stdout
            .contains("- value: `zh` (Chinese), from statute.toml\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn explain_fills_the_titles_of_the_rules_it_lists() {
    let work = repo("explain-titles", false, "[language]\ndocs = \"zh\"\n");
    let titled = edit(
        saying(2, "Write documents in {$language.docs}."),
        "en = \"Rule 2\"",
        "en = \"Documents in {$language.docs}\"",
    );
    write(&work, "r/rules/2.toml", &titled.1);
    ok(&statute(&work, &["lock"]));
    let run = statute(&work, &["explain", "language.docs"]);
    ok(&run);
    assert!(
        run.stdout
            .ends_with("- used by: R2 (Documents in Chinese)\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn a_persons_own_value_goes_to_the_private_output_only() {
    let work = repo("personal", true, "");
    global(&work, "[language]\ncommunication = \"zh\"\ndocs = \"zh\"\n");
    let run = statute(&work, &["lock"]);
    ok(&run);
    // A value of scope `repo` is the rule set's alone.
    assert!(
        run.stderr.contains(
            "warning: [language] `docs` is a parameter of scope `repo`, which only statute.toml sets; the value here is ignored"
        ),
        "{}",
        run.stderr
    );
    assert!(
        read(&work, "statute.lock").contains("name = \"language.communication\"\nvalue = \"en\"\n")
    );
    ok(&statute(&work, &["compile"]));
    let public = read(&work, "AGENTS.md");
    assert!(public.contains("\nReply in English.\n"), "{public}");
    let full = read(&work, ".statute/AGENTS.full.md");
    assert!(full.contains("\nReply in Chinese.\n"), "{full}");
    assert!(full.contains("\nWrite documents in English.\n"), "{full}");
    let run = statute(&work, &["explain", "language.communication"]);
    assert!(
        run.stdout.contains("- value: `en` (English), from the default\n- in the private output: `zh` (Chinese), from the global configuration ("),
        "{}",
        run.stdout
    );
    // A new worktree's hook writes it too, with no private rule set.
    std::fs::remove_file(work.join(".statute/AGENTS.full.md")).unwrap();
    let run = statute(&work, &["compile", "--private-only"]);
    ok(&run);
    assert!(read(&work, ".statute/AGENTS.full.md").contains("\nReply in Chinese.\n"));
    // The private rule set's own value comes first.
    let private = PathBuf::from(
        git(
            &work,
            &[
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                "common/statute",
            ],
        )
        .trim_end(),
    );
    write(
        &private,
        "private.toml",
        "rules = []\n\n[language]\ncommunication = \"en\"\n",
    );
    ok(&statute(&work, &["lock"]));
    ok(&statute(&work, &["compile"]));
    assert!(read(&work, ".statute/AGENTS.full.md").contains("\nReply in English.\n"));
}

#[test]
fn without_a_value_of_ones_own_there_is_no_private_output() {
    let work = repo("no-personal", true, "");
    ok(&statute(&work, &["lock"]));
    ok(&statute(&work, &["compile"]));
    assert!(!work.join(".statute/AGENTS.full.md").exists());
    let run = statute(&work, &["compile", "--private-only"]);
    ok(&run);
    assert_eq!(run.stdout, "");
    assert!(!work.join(".statute/AGENTS.full.md").exists());
}

#[test]
fn a_private_repository_says_a_persons_own_value_is_not_used() {
    let work = repo("private-repo", false, "");
    global(&work, "[language]\ncommunication = \"zh\"\n");
    ok(&statute(&work, &["lock"]));
    let run = statute(&work, &["compile"]);
    ok(&run);
    assert!(
        run.stderr.contains(
            "statute: note: the global configuration sets `language.communication` to `zh`, and this repository is private (`public = false`), with no private output; AGENTS.md has `en`"
        ),
        "{}",
        run.stderr
    );
    assert!(!work.join(".statute/AGENTS.full.md").exists());
}

#[test]
fn init_copies_no_value_from_the_global_configuration() {
    let work = repo("init", false, "");
    std::fs::remove_file(work.join("statute.toml")).unwrap();
    global(&work, "[language]\ncommunication = \"zh\"\n");
    ok(&statute(&work, &["init", "--private", "--source", "r"]));
    assert!(!read(&work, "statute.toml").contains("language"));
}
