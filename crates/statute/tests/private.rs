//! The private layer of a public repository, end to end: a git repository
//! with the first-party source `R`, its own rules and a private global
//! source `me`, through `init`, `add`, `lock`, `compile` and `check`, and
//! the worktrees of one clone sharing the private rule set.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, rule};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

/// What git ignores through `.git/info/exclude`: the links.
const EXCLUDE: &str = "/CLAUDE.local.md\n/AGENTS.override.md\n";

/// What statute writes in `.statute/.gitignore`.
const GITIGNORE: &str = "# written by statute\n/AGENTS.full.md\n/site/\n";

/// The private rule set of the repository `dir` (or of a worktree's
/// clone), with `me` in it.
fn private(dir: &Path) -> String {
    format!(
        "rules = []\n\n[sources]\nme = {{ path = \"{}/me\", public = false }}\n",
        dir.display()
    )
}

/// The directory of the private rule set, as git names it for `dir`.
fn common(dir: &Path) -> PathBuf {
    PathBuf::from(
        git_out(
            dir,
            &[
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                "common/statute",
            ],
        )
        .trim_end(),
    )
}

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn read(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join(path)).unwrap()
}

fn git(dir: &Path, args: &[&str]) {
    git_out(dir, args);
}

fn git_out(dir: &Path, args: &[&str]) -> String {
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

fn commit(dir: &Path, message: &str) {
    git(dir, &["add", "."]);
    git(
        dir,
        &[
            "-c",
            "user.name=Example",
            "-c",
            "user.email=example@example.test",
            "commit",
            "-q",
            "-m",
            message,
        ],
    );
}

/// A git repository with `R` in `r/` (domain 1, rules 2 and 6), its own
/// rules (rule 3, which requires R2) and `me` in `me/`: rule 4, under
/// R1, which requires R2 and cites it, and rule 5, which overrides R2.
/// The links are not ignored yet.
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
        (".statute/rules", domain(1)),
        (".statute/rules", rule(3, 1, "requires = [\"R2\"]")),
        ("me/rules", domain(1)),
        (
            "me/rules",
            edit(
                edit(
                    rule(
                        4,
                        1,
                        "requires = [\"R2\"]\nuses = [\"instructions\", \"review\"]",
                    ),
                    "parent = 1",
                    "parent = \"R1\"",
                ),
                "Do the thing 4.",
                "Do the thing 4, after {@rule:R2}.",
            ),
        ),
        (
            "me/rules",
            edit(
                rule(5, 1, ""),
                "parent = 1\ndefault = \"off\"",
                "overrides = \"R2\"",
            ),
        ),
    ];
    for (sub, (id, text)) in nodes {
        write(&dir, &format!("{sub}/{id}.toml"), &text);
    }
    dir
}

/// `repo` with git ignoring the links.
fn ignoring(name: &str) -> PathBuf {
    let dir = repo(name);
    write(&dir, ".git/info/exclude", EXCLUDE);
    dir
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn statute(dir: &Path, args: &[&str]) -> Run {
    statute_with(dir, args, &[])
}

/// `statute` with `env` added to its environment.
fn statute_with(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_statute"))
        .env("HOME", concat!(env!("CARGO_TARGET_TMPDIR"), "/home"))
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_CACHE_HOME")
        .args(args)
        .current_dir(dir)
        // git looks for a repository no further up than the tests' own
        // directories, so one that `.git` was taken from is outside any.
        .env(
            "GIT_CEILING_DIRECTORIES",
            concat!(env!("CARGO_TARGET_TMPDIR"), "/private"),
        )
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .envs(env.iter().copied())
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
/// me:4 from `me/` of `at`, locked.
fn set_up_from(dir: &Path, at: &Path) {
    ok(&statute(dir, &["init", "--public", "--source", "r"]));
    ok(&statute(dir, &["add", "R2"]));
    ok(&statute(dir, &["add", "3"]));
    std::fs::create_dir_all(common(dir)).unwrap();
    std::fs::write(common(dir).join("private.toml"), private(at)).unwrap();
    let run = statute(dir, &["add", "--private", "me:4"]);
    ok(&run);
    assert!(
        run.stdout.contains(&format!(
            "wrote {}: 3 rules enabled, 1 of them by the private layer",
            common(dir).join("private.lock").display()
        )),
        "{}",
        run.stdout
    );
}

/// What statute wrote in `.statute/`: everything there but the
/// repository's own rules.
fn written_in_statute_dir(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir.join(".statute"))
        .map(|entries| {
            entries
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|name| name != "rules")
                .collect()
        })
        .unwrap_or_default()
}

fn set_up(dir: &Path) {
    set_up_from(dir, dir);
}

/// The private rule set or its lock file, `file`, of `dir`.
fn read_private(dir: &Path, file: &str) -> String {
    std::fs::read_to_string(common(dir).join(file)).unwrap()
}

fn write_private(dir: &Path, text: &str) {
    std::fs::write(common(dir).join("private.toml"), text).unwrap();
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
    // The private lock file pins the private source and nothing else.
    let lock = read_private(&dir, "private.lock");
    assert_eq!(
        lock,
        format!(
            "# Written by statute from private.toml; do not edit by hand.\n\n[[sources]]\nname = \"me\"\npath = \"{}/me\"\npublic = false\nscope = \"global\"\n",
            dir.display()
        )
    );

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
    assert_eq!(read(&dir, ".statute/.gitignore"), GITIGNORE);
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
        &format!(
            "{}: error: `me:5` overrides \"R2\", a rule of the public rule set `statute.toml`; in a public repository (`public = true`), `private.toml` may only add rules",
            common(&dir).join("private.toml").display()
        ),
    );
    assert!(!read_private(&dir, "private.toml").contains("me:5"));
    // An override of a rule only the private layer brings in holds.
    write(
        &dir,
        "me/rules/5.toml",
        &edit(
            rule(5, 1, ""),
            "parent = 1\ndefault = \"off\"",
            "overrides = \"R6\"",
        )
        .1,
    );
    write_private(
        &dir,
        &private(&dir).replace("rules = []", "rules = [\"me:4\", \"R6\"]"),
    );
    ok(&statute(&dir, &["add", "--private", "me:5"]));
    // A public change the private layer cannot follow is not written.
    let files = || {
        (
            read(&dir, "statute.toml"),
            read(&dir, "statute.lock"),
            read_private(&dir, "private.lock"),
        )
    };
    let before = files();
    fails_with(
        &statute(&dir, &["add", "R6"]),
        "error: `rules` lists `R6`, which statute.toml lists already",
    );
    assert_eq!(files(), before);
}

#[test]
fn the_links_are_ignored_through_the_exclude_file() {
    // Through a .gitignore, not .git/info/exclude.
    let dir = repo("links-in-gitignore");
    write(&dir, ".gitignore", EXCLUDE);
    set_up(&dir);
    let run = statute(&dir, &["compile"]);
    fails_with(
        &run,
        "CLAUDE.local.md, AGENTS.override.md: git must ignore these links through .git/info/exclude, not a .gitignore (Grok skips an instruction file a .gitignore matches), so nothing private is written; add these lines to .git/info/exclude: /CLAUDE.local.md, /AGENTS.override.md",
    );
    assert!(!dir.join(".statute/AGENTS.full.md").exists());
    assert!(!dir.join("AGENTS.md").exists());
    assert!(dir.join("CLAUDE.local.md").symlink_metadata().is_err());

    // A negation in a .gitignore takes one out of .git/info/exclude.
    let dir = ignoring("negation");
    write(&dir, ".gitignore", "!/AGENTS.override.md\n");
    set_up(&dir);
    let run = statute(&dir, &["compile"]);
    fails_with(
        &run,
        "add these lines to .git/info/exclude: /AGENTS.override.md",
    );
    assert!(!dir.join(".statute/AGENTS.full.md").exists());
    std::fs::remove_file(dir.join(".gitignore")).unwrap();
    ok(&statute(&dir, &["compile"]));
}

#[test]
fn nothing_is_written_when_git_cannot_tell() {
    let dir = ignoring("no-git");
    set_up(&dir);
    // R2 changed, so that a lock file written now would differ.
    let r2 = read(&dir, "r/rules/2.toml");
    write(
        &dir,
        "r/rules/2.toml",
        &r2.replace("Do the thing 2.", "Do the thing 2 well."),
    );
    let files = || {
        (
            read(&dir, "statute.toml"),
            read(&dir, "statute.lock"),
            read_private(&dir, "private.toml"),
            read_private(&dir, "private.lock"),
        )
    };
    let before = files();
    // No git on the path: nothing says where the private rule set is.
    let empty = dir.join("no-bin");
    std::fs::create_dir_all(&empty).unwrap();
    for args in [
        &["lock"][..],
        &["add", "R6"],
        &["remove", "3"],
        &["upgrade"],
        &["add", "--private", "R6"],
        &["compile"],
    ] {
        let run = statute_with(&dir, args, &[("PATH", empty.to_str().unwrap())]);
        fails_with(
            &run,
            "cannot run git, which says where the private rule set is",
        );
        assert_eq!(files(), before, "{args:?}");
    }
    assert!(!dir.join("AGENTS.md").exists());
    assert_eq!(written_in_statute_dir(&dir), Vec::<String>::new());
}

#[test]
fn outside_a_git_repository_there_is_no_private_layer() {
    let dir = ignoring("no-repository");
    set_up(&dir);
    std::fs::remove_dir_all(dir.join(".git")).unwrap();
    ok(&statute(&dir, &["compile"]));
    assert!(dir.join("AGENTS.md").exists());
    assert_eq!(written_in_statute_dir(&dir), Vec::<String>::new());
    assert!(dir.join("CLAUDE.local.md").symlink_metadata().is_err());
    fails_with(
        &statute(&dir, &["add", "--private", "R6"]),
        "statute: error: not in a git repository, so there is no private layer: its rule set, private.toml, lives in git's directory",
    );
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
    std::fs::create_dir_all(common(&dir)).unwrap();
    write_private(&dir, &private(&dir));
    let run = statute(&dir, &["lock"]);
    fails_with(
        &run,
        &format!(
            "{}: error: private.toml is for a public repository; in a private one (`public = false`), statute.toml names every source and rule",
            common(&dir).join("private.toml").display()
        ),
    );
}

#[test]
fn a_path_in_the_private_rule_set_is_absolute_or_in_the_home_directory() {
    let dir = ignoring("paths");
    set_up(&dir);
    write_private(
        &dir,
        "rules = [\"me:4\"]\n\n[sources]\nme = { path = \"me\", public = false }\n",
    );
    fails_with(
        &statute(&dir, &["lock"]),
        "[sources] `me`: `path = \"me\"` is relative; the private rule set serves every worktree, so a path in it is absolute or starts with `~/`",
    );
    // `~/` is the home directory.
    let home = Path::new(concat!(env!("CARGO_TARGET_TMPDIR"), "/home"));
    std::fs::create_dir_all(home).unwrap();
    let link = home.join("private-paths-me");
    let _ = std::fs::remove_file(&link);
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.join("me"), &link).unwrap();
    write_private(
        &dir,
        "rules = [\"me:4\"]\n\n[sources]\nme = { path = \"~/private-paths-me\", public = false }\n",
    );
    ok(&statute(&dir, &["lock"]));
    ok(&statute(&dir, &["compile"]));
    assert!(read(&dir, ".statute/AGENTS.full.md").contains("### Rule 4 (me:4)"));
}

#[test]
fn the_private_rule_set_is_written_in_git_s_directory() {
    let dir = ignoring("in-place-written");
    ok(&statute(&dir, &["init", "--public", "--source", "r"]));
    ok(&statute(&dir, &["add", "R2"]));
    // `add --private` makes the private rule set when there is none.
    assert!(!common(&dir).exists());
    ok(&statute(&dir, &["add", "--private", "R6"]));
    assert_eq!(read_private(&dir, "private.toml"), "rules = [\"R6\"]\n");
    // Its temporary files went to the same directory and are gone; the
    // work tree has none.
    let mut names: Vec<String> = std::fs::read_dir(common(&dir))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["private.lock", "private.toml"]);
    assert_eq!(written_in_statute_dir(&dir), Vec::<String>::new());
    assert!(!dir.join("statute.private.toml").exists());
    ok(&statute(&dir, &["remove", "--private", "R6"]));
    assert_eq!(read_private(&dir, "private.toml"), "rules = []\n");
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
    assert!(!dir.join(".statute/AGENTS.full.md").exists());
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
        run.stderr.contains("CLAUDE.md: warning: Claude Code now reads AGENTS.md itself, so CLAUDE.md is not needed to point to it. Here CLAUDE.local.md gives it the full text, public and private, in place of AGENTS.md, and it reads CLAUDE.md as well (for a link to AGENTS.md, the public rules twice). Consider deleting it; statute neither deletes nor renames it (a migration will rename it to a legacy copy, after asking)."),
        "{}",
        run.stderr
    );
    // Without a private layer, the warning says less; without CLAUDE.md,
    // nothing.
    std::fs::remove_file(common(&dir).join("private.toml")).unwrap();
    let run = statute(&dir, &["compile"]);
    ok(&run);
    assert!(
        run.stderr
            .contains("CLAUDE.md is not needed to point to it. Consider deleting it"),
        "{}",
        run.stderr
    );
    // A plain file warns as a link does.
    std::fs::remove_file(dir.join("CLAUDE.md")).unwrap();
    write(&dir, "CLAUDE.md", "mine\n");
    let run = statute(&dir, &["compile"]);
    ok(&run);
    assert!(
        run.stderr
            .contains("CLAUDE.md: warning: Claude Code now reads AGENTS.md itself"),
        "{}",
        run.stderr
    );
    std::fs::remove_file(dir.join("CLAUDE.md")).unwrap();
    let run = statute(&dir, &["compile"]);
    ok(&run);
    assert!(!run.stderr.contains("warning"), "{}", run.stderr);
}

#[test]
fn an_agents_md_below_the_top_is_not_loaded() {
    let dir = ignoring("nested");
    set_up(&dir);
    write(&dir, "sub/AGENTS.md", "mine\n");
    write(&dir, "sub/deeper/AGENTS.md", "mine\n");
    write(&dir, "untracked/AGENTS.md", "mine\n");
    git(&dir, &["add", "sub"]);
    let run = statute(&dir, &["compile"]);
    ok(&run);
    let warnings: Vec<&str> = run
        .stderr
        .lines()
        .filter(|l| l.contains("warning"))
        .collect();
    assert_eq!(
        warnings,
        [
            "sub/AGENTS.md: warning: Claude Code does not load an AGENTS.md below the top of the repository while CLAUDE.local.md is there; move its rules into statute's rules",
            "sub/deeper/AGENTS.md: warning: Claude Code does not load an AGENTS.md below the top of the repository while CLAUDE.local.md is there; move its rules into statute's rules",
        ]
    );
    // Without a private layer, there is no CLAUDE.local.md.
    std::fs::remove_file(common(&dir).join("private.toml")).unwrap();
    let run = statute(&dir, &["compile"]);
    ok(&run);
    assert!(!run.stderr.contains("warning"), "{}", run.stderr);
}

#[test]
fn statute_writes_its_own_gitignore() {
    let dir = ignoring("gitignore");
    set_up(&dir);
    write(
        &dir,
        ".statute/rules/README.md",
        "The rules of the repository.\n",
    );
    ok(&statute(&dir, &["compile"]));
    ok(&statute(&dir, &["site"]));
    assert_eq!(read(&dir, ".statute/.gitignore"), GITIGNORE);
    // git shows what is to be committed in `.statute/`, and none of the
    // generated files.
    let status = git_out(&dir, &["status", "--porcelain", "--untracked-files=all"]);
    assert!(status.contains("?? .statute/.gitignore\n"), "{status}");
    assert!(status.contains("?? .statute/rules/README.md\n"), "{status}");
    for generated in [
        "AGENTS.full.md",
        ".statute/site/",
        "CLAUDE.local.md",
        "AGENTS.override.md",
    ] {
        assert!(!status.contains(generated), "{generated}: {status}");
    }
    // statute's own file, as an older statute wrote it, is written anew.
    write(&dir, ".statute/.gitignore", "# written by statute\n/old\n");
    ok(&statute(&dir, &["compile"]));
    assert_eq!(read(&dir, ".statute/.gitignore"), GITIGNORE);
    // The repository's own is left as it is, and nothing is written.
    write(&dir, ".statute/.gitignore", "mine\n");
    std::fs::remove_file(dir.join("AGENTS.md")).unwrap();
    std::fs::remove_file(dir.join(".statute/AGENTS.full.md")).unwrap();
    fails_with(
        &statute(&dir, &["compile"]),
        ".statute/.gitignore: its first line is not `# written by statute`, so it is the repository's own and is left as it is, and nothing is written into .statute",
    );
    assert_eq!(read(&dir, ".statute/.gitignore"), "mine\n");
    assert!(!dir.join("AGENTS.md").exists());
    assert!(!dir.join(".statute/AGENTS.full.md").exists());
}

#[test]
fn a_worktree_names_the_clones_exclude_file() {
    let dir = repo("worktree");
    commit(&dir, "rules");
    let tree = dir.with_file_name("worktree-tree");
    let _ = std::fs::remove_dir_all(&tree);
    git(&dir, &["worktree", "add", "-q", tree.to_str().unwrap()]);
    set_up(&tree);
    let run = statute(&tree, &["compile"]);
    let exclude = dir.join(".git/info/exclude");
    fails_with(
        &run,
        &format!(
            "add these lines to {}: /CLAUDE.local.md, /AGENTS.override.md",
            exclude.display()
        ),
    );
    // The lines in the clone's exclude file hold for the worktree.
    write(&dir, ".git/info/exclude", EXCLUDE);
    ok(&statute(&tree, &["compile"]));
}

#[test]
fn worktrees_share_the_private_rule_set() {
    let dir = ignoring("shared");
    set_up(&dir);
    commit(&dir, "rules");
    let tree = dir.with_file_name("shared-tree");
    let _ = std::fs::remove_dir_all(&tree);
    git(
        &dir,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "other",
            tree.to_str().unwrap(),
        ],
    );
    // One private rule set and lock file for both, in the clone's
    // directory.
    assert_eq!(common(&tree), dir.join(".git/common/statute"));
    let lock = read_private(&dir, "private.lock");
    // The other branch enables R6 as well; its lock files are written,
    // and the private one stays as it was.
    let run = statute(&tree, &["add", "R6"]);
    ok(&run);
    assert!(
        run.stdout
            .contains("private.lock: 4 rules enabled, 1 of them by the private layer"),
        "{}",
        run.stdout
    );
    assert_eq!(read_private(&tree, "private.lock"), lock);
    assert_ne!(read(&tree, "statute.toml"), read(&dir, "statute.toml"));
    // Each worktree compiles its own branch with the private layer, and
    // neither finds the private lock file stale.
    ok(&statute(&tree, &["compile"]));
    ok(&statute(&dir, &["compile"]));
    let (here, there) = (
        read(&dir, ".statute/AGENTS.full.md"),
        read(&tree, ".statute/AGENTS.full.md"),
    );
    assert!(here.contains("### Rule 4 (me:4)") && there.contains("### Rule 4 (me:4)"));
    assert!(there.contains("(R6)") && !here.contains("(R6)"), "{here}");
    // A private rule taken out in one worktree is out in the other.
    ok(&statute(&tree, &["remove", "--private", "me:4"]));
    ok(&statute(&dir, &["compile"]));
    assert!(!read(&dir, ".statute/AGENTS.full.md").contains("(me:4)"));
}

#[test]
fn the_private_layer_enables_packs() {
    let dir = ignoring("private-packs");
    set_up(&dir);
    // R1 as a pack of the private layer adds R6, which only it enables.
    ok(&statute(&dir, &["add", "--private", "--pack", "R1"]));
    assert!(read_private(&dir, "private.toml").contains("packs = [\"R1\"]"));
    ok(&statute(&dir, &["compile"]));
    assert!(read(&dir, ".statute/AGENTS.full.md").contains("(R6)"));
    assert!(!read(&dir, "AGENTS.md").contains("(R6)"));
    // A pack of statute.toml is not listed again.
    ok(&statute(&dir, &["remove", "--private", "--pack", "R1"]));
    ok(&statute(&dir, &["add", "--pack", "R1"]));
    fails_with(
        &statute(&dir, &["add", "--private", "--pack", "R1"]),
        "`packs` lists `R1`, which statute.toml lists already",
    );
}

#[test]
fn the_private_lock_file_records_a_sources_rules_dir() {
    let dir = ignoring("rules-dir");
    set_up(&dir);
    std::fs::rename(dir.join("me/rules"), dir.join("me/policies")).unwrap();
    write_private(
        &dir,
        &private(&dir)
            .replace("rules = []", "rules = [\"me:4\"]")
            .replace(
                "public = false }",
                "public = false, rules-dir = \"policies\" }",
            ),
    );
    ok(&statute(&dir, &["lock"]));
    let lock = read_private(&dir, "private.lock");
    assert!(
        lock.ends_with("public = false\nscope = \"global\"\nrules-dir = \"policies\"\n"),
        "{lock}"
    );
    ok(&statute(&dir, &["compile"]));
    assert!(read(&dir, ".statute/AGENTS.full.md").contains("### Rule 4 (me:4)"));
}
