//! The private layer of a public repository, end to end: a git repository
//! with the first-party source `R`, its own rules and a private global
//! source `me`, through `init`, `add`, `lock`, `compile` and `check`.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, rule};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

/// What git ignores: the private files through `.gitignore`, the links
/// through `.git/info/exclude`.
const GITIGNORE: &str = "/statute.private.toml\n/statute.private.lock\n/.statute/\n";
const EXCLUDE: &str = "/CLAUDE.local.md\n/AGENTS.override.md\n";

const PRIVATE: &str =
    "base = \"statute.toml\"\nrules = []\n\n[sources]\nme = { path = \"me\", public = false }\n";

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn read(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join(path)).unwrap()
}

fn git(dir: &Path, args: &[&str]) {
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
}

/// A git repository with `R` in `r/` (domain 1, rules 2 and 6), its own
/// rules (rule 3, which requires R2) and `me` in `me/`: rule 4, under
/// R1, which requires R2 and cites it, and rule 5, which overrides R2.
/// Nothing is ignored yet.
fn repo(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("private")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q"]);
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
    let nodes = [
        ("r/rules", domain(1)),
        ("r/rules", rule(2, 1, "")),
        ("r/rules", rule(6, 1, "")),
        ("rules", domain(1)),
        ("rules", rule(3, 1, "requires = [\"R2\"]")),
        ("me/rules", domain(1)),
        (
            "me/rules",
            edit(
                edit(
                    rule(4, 1, "requires = [\"R2\"]"),
                    "parent = 1",
                    "parent = \"R1\"",
                ),
                "Do the thing 4.",
                "Do the thing 4, after {@rule:R2}.",
            ),
        ),
        (
            "me/rules",
            edit(rule(5, 1, ""), "parent = 1", "overrides = \"R2\""),
        ),
    ];
    for (sub, (id, text)) in nodes {
        write(&dir, &format!("{sub}/{id}.toml"), &text);
    }
    dir
}

/// `repo` with git ignoring what it should.
fn ignoring(name: &str) -> PathBuf {
    let dir = repo(name);
    write(&dir, ".gitignore", GITIGNORE);
    write(&dir, ".git/info/exclude", EXCLUDE);
    dir
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn statute(dir: &Path, args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_statute"))
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
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

/// A public rule set enabling R2 and L3, and the private layer enabling
/// me:4, locked.
fn set_up(dir: &Path) {
    ok(&statute(dir, &["init", "--public", "--source", "r"]));
    ok(&statute(dir, &["add", "R2"]));
    ok(&statute(dir, &["add", "3"]));
    write(dir, "statute.private.toml", PRIVATE);
    let run = statute(dir, &["add", "--private", "me:4"]);
    ok(&run);
    assert!(
        run.stdout.contains(
            "wrote statute.private.lock: 3 rules enabled, 1 of them by the private layer"
        ),
        "{}",
        run.stdout
    );
}

#[test]
fn three_sources_from_init_to_check() {
    let dir = ignoring("end-to-end");
    set_up(&dir);
    ok(&statute(&dir, &["lock"]));
    // The committed files hold nothing of the private layer.
    for file in ["statute.toml", "statute.lock"] {
        assert!(
            !read(&dir, file).contains("me:") && !read(&dir, file).contains("\"me\""),
            "{file}: {}",
            read(&dir, file)
        );
    }
    assert!(read(&dir, "statute.private.lock").contains("rule = \"me:4\""));

    let run = statute(&dir, &["compile"]);
    ok(&run);
    let public = read(&dir, "AGENTS.md");
    assert!(!public.contains("Rule 4"), "{public}");
    assert!(
        public.ends_with("\n---\n\nIf `.statute/AGENTS.full.md` exists, read it and follow it: it is this file with the repository's private rules added.\n"),
        "{public}"
    );
    let full = read(&dir, ".statute/AGENTS.full.md");
    // The full text is the public file with me:4 at its place under R1,
    // and without the line that points to it.
    assert!(
        full.contains("### Rule 4 (me:4)\n\nDo the thing 4, after [Rule 2](#rule-R2).\n"),
        "{full}"
    );
    assert!(full.contains("### Rule 3 (L3)"), "{full}");
    assert!(!full.contains("AGENTS.full.md"), "{full}");
    for link in ["CLAUDE.local.md", "AGENTS.override.md"] {
        assert_eq!(
            std::fs::read_link(dir.join(link)).unwrap(),
            Path::new(".statute/AGENTS.full.md"),
            "{link}"
        );
        assert_eq!(read(&dir, link), full);
    }
    // A second compile finds its own links and writes again.
    ok(&statute(&dir, &["compile"]));

    let run = statute(&dir, &["check"]);
    assert!(
        run.stdout.contains("review        me:4 (Rule 4)"),
        "{}",
        run.stdout
    );
    let run = statute(&dir, &["explain", "me:4"]);
    ok(&run);
    assert!(run.stdout.contains("- listed in `rules`"), "{}", run.stdout);
}

#[test]
fn the_private_layer_overrides_no_public_rule() {
    let dir = ignoring("override");
    set_up(&dir);
    // me:5 overrides R2, which statute.toml enables.
    let run = statute(&dir, &["add", "--private", "me:5"]);
    fails_with(
        &run,
        "statute.private.toml: error: `me:5` overrides \"R2\", a rule of the public rule set `statute.toml`; in a public repository (`public = true`), `statute.private.toml` may only add rules",
    );
    assert!(!read(&dir, "statute.private.toml").contains("me:5"));
    // An override of a rule only the private layer brings in holds.
    write(
        &dir,
        "me/rules/5.toml",
        &edit(rule(5, 1, ""), "parent = 1", "overrides = \"R6\"").1,
    );
    write(
        &dir,
        "statute.private.toml",
        &PRIVATE.replace("rules = []", "rules = [\"me:4\", \"R6\"]"),
    );
    ok(&statute(&dir, &["add", "--private", "me:5"]));
}

#[test]
fn nothing_private_is_written_unless_git_ignores_it() {
    // The lock file not ignored.
    let dir = repo("not-ignored");
    write(&dir, ".gitignore", "/statute.private.toml\n");
    ok(&statute(&dir, &["init", "--public", "--source", "r"]));
    write(&dir, "statute.private.toml", PRIVATE);
    let run = statute(&dir, &["add", "--private", "me:4"]);
    fails_with(
        &run,
        "statute.private.lock: git does not ignore it, so nothing private is written; add `/statute.private.lock` to the repository's .gitignore",
    );
    assert!(!dir.join("statute.private.lock").exists());
    assert!(!read(&dir, "statute.private.toml").contains("me:4"));

    // The links ignored through .gitignore, not .git/info/exclude.
    let dir = repo("links-in-gitignore");
    write(&dir, ".gitignore", &format!("{GITIGNORE}{EXCLUDE}"));
    set_up(&dir);
    let run = statute(&dir, &["compile"]);
    fails_with(
        &run,
        "CLAUDE.local.md, AGENTS.override.md: git must ignore these links through .git/info/exclude, not a .gitignore (Grok skips an instruction file a .gitignore matches), so nothing private is written; add these lines to .git/info/exclude: /CLAUDE.local.md, /AGENTS.override.md",
    );
    assert!(!dir.join(".statute").exists());
    assert!(dir.join("CLAUDE.local.md").symlink_metadata().is_err());
}

#[test]
fn a_file_in_a_links_place_is_left_as_it_is() {
    let dir = ignoring("in-place");
    set_up(&dir);
    write(&dir, "CLAUDE.local.md", "mine\n");
    let run = statute(&dir, &["compile"]);
    fails_with(
        &run,
        "CLAUDE.local.md: not the link statute makes to .statute/AGENTS.full.md, so it is left as it is and nothing private is written; move it away first",
    );
    assert_eq!(read(&dir, "CLAUDE.local.md"), "mine\n");
    assert!(dir.join("AGENTS.override.md").symlink_metadata().is_err());
}

#[test]
fn a_private_repository_has_no_private_layer() {
    let dir = ignoring("private-repository");
    ok(&statute(&dir, &["init", "--private", "--source", "r"]));
    write(&dir, "statute.private.toml", PRIVATE);
    let run = statute(&dir, &["lock"]);
    fails_with(
        &run,
        "statute.private.toml: error: statute.private.toml is for a public repository; in a private one (`public = false`), statute.toml names every source and rule",
    );
}

#[test]
fn the_full_text_fits_what_codex_reads() {
    let dir = ignoring("too-long");
    write(
        &dir,
        "me/rules/4.toml",
        &edit(
            rule(4, 1, ""),
            "Do the thing 4.",
            &"Do the thing 4. ".repeat(2100),
        )
        .1,
    );
    set_up(&dir);
    let run = statute(&dir, &["compile"]);
    fails_with(
        &run,
        "more than the 32768 that Codex reads of a project's instruction files",
    );
    assert!(!dir.join(".statute").exists());
}

#[test]
fn a_claude_md_is_not_needed() {
    let dir = ignoring("claude-md");
    set_up(&dir);
    #[cfg(unix)]
    std::os::unix::fs::symlink("AGENTS.md", dir.join("CLAUDE.md")).unwrap();
    let run = statute(&dir, &["compile"]);
    ok(&run);
    assert!(
        run.stderr.contains("CLAUDE.md: warning: Claude Code now reads AGENTS.md itself, so CLAUDE.md is not needed to point to it. Here CLAUDE.local.md gives it the full text, public and private, in place of AGENTS.md, and it reads CLAUDE.md as well, so that its content comes on top of the rules (for a link to AGENTS.md, the public rules twice). Consider deleting it; statute neither deletes nor renames it (a migration will rename it to a legacy copy, after asking)."),
        "{}",
        run.stderr
    );
    // Without a private layer, the warning says less; without CLAUDE.md,
    // nothing.
    std::fs::remove_file(dir.join("statute.private.toml")).unwrap();
    let run = statute(&dir, &["compile"]);
    ok(&run);
    assert!(
        run.stderr
            .contains("CLAUDE.md is not needed to point to it. Consider deleting it"),
        "{}",
        run.stderr
    );
    std::fs::remove_file(dir.join("CLAUDE.md")).unwrap();
    let run = statute(&dir, &["compile"]);
    ok(&run);
    assert!(!run.stderr.contains("warning"), "{}", run.stderr);
}
