//! The private layer of a public repository, end to end: a git repository
//! with the first-party source `R`, its own rules and a private global
//! source `me`, through `init`, `add`, `lock`, `compile` and `check`, and
//! the worktrees of one clone sharing the private rule set.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, rule, set_default};

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
    common::provisions(&dir.join("r/constitution/2026-10"));
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
    assert!(!public.contains("- me:4 "), "{public}");
    assert!(!public.contains("AGENTS.full.md"), "{public}");
    let full = read(&dir, ".statute/AGENTS.full.md");
    // The full text is the public file with me:4 at its place under R1.
    assert!(
        full.contains("\n- me:4 Do the thing 4, after R2.\n"),
        "{full}"
    );
    assert!(full.contains("\n- L3 "), "{full}");
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
fn compile_check_leaves_the_private_output_out() {
    let dir = ignoring("compile-check");
    set_up(&dir);
    ok(&statute(&dir, &["compile"]));
    write(&dir, ".statute/AGENTS.full.md", "edited by hand\n");
    ok(&statute(&dir, &["compile", "--check"]));
    write(&dir, "AGENTS.md", "edited by hand\n");
    fails_with(
        &statute(&dir, &["compile", "--check"]),
        "./AGENTS.md: error: not what `statute compile` writes; run it\n",
    );
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
            "{}: error: `me:5` overrides \"R2\", a rule of the public rule set `statute.toml`; in a public repository (`public = true`), a rule `private.toml` enables overrides no rule of the public layer",
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
    assert!(read(&dir, ".statute/AGENTS.full.md").contains("\n- me:4 "));
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
    assert!(here.contains("\n- me:4 ") && there.contains("\n- me:4 "));
    assert!(
        there.contains("\n- R6 ") && !here.contains("\n- R6 "),
        "{here}"
    );
    // A private rule taken out in one worktree is out in the other.
    ok(&statute(&tree, &["remove", "--private", "me:4"]));
    ok(&statute(&dir, &["compile"]));
    assert!(!read(&dir, ".statute/AGENTS.full.md").contains("\n- me:4 "));
}

#[test]
fn the_private_layer_enables_packs() {
    let dir = ignoring("private-packs");
    set_up(&dir);
    // R1 as a pack of the private layer adds R6, which only it enables.
    ok(&statute(&dir, &["add", "--private", "--pack", "R1"]));
    assert!(read_private(&dir, "private.toml").contains("packs = [\"R1\"]"));
    ok(&statute(&dir, &["compile"]));
    assert!(read(&dir, ".statute/AGENTS.full.md").contains("\n- R6 "));
    assert!(!read(&dir, "AGENTS.md").contains("\n- R6 "));
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
    assert!(read(&dir, ".statute/AGENTS.full.md").contains("\n- me:4 "));
}

#[test]
fn a_public_rule_overrides_a_rule_only_the_private_layer_brings_in() {
    for listed in [false, true] {
        let dir = ignoring(if listed {
            "cross-layer-listed"
        } else {
            "cross-layer-unlisted"
        });
        // L7 overrides R6, which only me:8, of the private layer, requires.
        write(
            &dir,
            ".statute/rules/7.toml",
            &edit(
                rule(7, 1, ""),
                "parent = 1\ndefault = \"off\"",
                "overrides = \"R6\"",
            )
            .1,
        );
        write(
            &dir,
            "me/rules/8.toml",
            &rule(8, 1, "requires = [\"R6\"]").1,
        );
        set_up(&dir);
        if listed {
            ok(&statute(&dir, &["add", "7"]));
        }
        write_private(
            &dir,
            &private(&dir).replace("rules = []", "rules = [\"me:4\", \"me:8\"]"),
        );
        let run = statute(&dir, &["compile"]);
        ok(&run);
        let public = read(&dir, "AGENTS.md");
        let full = read(&dir, ".statute/AGENTS.full.md");
        if listed {
            assert!(
                run.stderr.contains("statute.toml: warning: rule L7 (Rule 7) overrides R6 (Rule 6), which nothing else in the rule set enables; L7 is compiled where R6 would stand\n"),
                "{}",
                run.stderr
            );
            assert!(public.contains("\n- L7 Do the thing 7.\n"), "{public}");
            assert!(full.contains("\n- L7 Do the thing 7.\n"), "{full}");
            assert!(!full.contains("- R6 "), "{full}");
        } else {
            assert!(
                run.stderr.contains("statute.toml: warning: rule L7 (Rule 7) is one of this repository's own, and nothing in the rule set enables it, so it is compiled nowhere; list \"7\" in `rules` to compile it in place of R6\n"),
                "{}",
                run.stderr
            );
            assert!(!public.contains("- L7 "), "{public}");
            assert!(full.contains("\n- R6 Do the thing 6.\n"), "{full}");
        }
    }
}

#[test]
fn a_finding_of_the_private_layer_names_the_private_rule_set() {
    let dir = ignoring("private-finding");
    // me:9 is on in me:1, which is on: the private layer's defaults
    // enable it.
    set_up(&dir);
    write(&dir, "me/rules/1.toml", &set_default(domain(1), "\"on\"").1);
    write(
        &dir,
        "me/rules/9.toml",
        &set_default(rule(9, 1, ""), "\"on\"").1,
    );
    write_private(
        &dir,
        &private(&dir).replace("rules = []", "rules = [\"me:4\", \"me:9\"]"),
    );
    let run = statute(&dir, &["lock"]);
    ok(&run);
    assert!(
        run.stderr.contains(&format!(
            "{}: warning: `rules` lists me:9 (Rule 9), which the defaults enable; take it out\n",
            common(&dir).join("private.toml").display()
        )),
        "{}",
        run.stderr
    );
    // A finding that asks for a change to `exclude` stays with
    // statute.toml, though me:8 of the private layer brings it about.
    write(
        &dir,
        "me/rules/8.toml",
        &rule(8, 1, "requires = [\"R6\"]").1,
    );
    write(
        &dir,
        "statute.toml",
        &read(&dir, "statute.toml").replace("rules = [", "exclude = [\"R6\"]\nrules = ["),
    );
    write_private(
        &dir,
        &private(&dir).replace("rules = []", "rules = [\"me:4\", \"me:8\"]"),
    );
    let run = statute(&dir, &["lock"]);
    assert!(
        run.stderr
            .lines()
            .any(|l| l.starts_with("statute.toml: error: rule R6 (Rule 6) cannot be turned off: rule me:8 (Rule 8) requires it")),
        "{}",
        run.stderr
    );
}

#[test]
fn a_rule_of_the_repository_s_own_is_enabled_in_either_layer() {
    // L7, in no rule set, is compiled nowhere; listed by the private
    // layer alone, it is in the private output, and the public pass,
    // which cannot see that, says nothing.
    for private_lists in [false, true] {
        let dir = ignoring(if private_lists {
            "own-listed-privately"
        } else {
            "own-unlisted"
        });
        write(&dir, ".statute/rules/7.toml", &rule(7, 1, "").1);
        set_up(&dir);
        let rules = if private_lists {
            "rules = [\"me:4\", \"7\"]"
        } else {
            "rules = [\"me:4\"]"
        };
        write_private(&dir, &private(&dir).replace("rules = []", rules));
        let run = statute(&dir, &["compile"]);
        ok(&run);
        let warned = run.stderr.contains(
            "statute.toml: warning: rule L7 (Rule 7) is one of this repository's own, and nothing in the rule set enables it, so it is compiled nowhere; list \"7\" in `rules` to compile it\n",
        );
        assert_eq!(warned, !private_lists, "{}", run.stderr);
        let full = read(&dir, ".statute/AGENTS.full.md");
        assert_eq!(full.contains("\n- L7 "), private_lists, "{full}");
        assert!(!read(&dir, "AGENTS.md").contains("- L7 "));
    }
}

#[test]
fn a_rule_of_a_local_private_source_is_listed_in_the_private_rule_set() {
    let dir = ignoring("local-private-source");
    set_up(&dir);
    // me, now of `scope = "local"`, ranks with the repository's own rules;
    // me:5, which overrides R2, is listed nowhere.
    write_private(
        &dir,
        &private(&dir)
            .replace("rules = []", "rules = [\"me:4\"]")
            .replace("public = false }", "public = false, scope = \"local\" }"),
    );
    let run = statute(&dir, &["lock"]);
    ok(&run);
    assert!(
        run.stderr.contains(&format!(
            "{}: warning: rule me:5 (Rule 5) is one of this repository's own, and nothing in the rule set enables it, so it is compiled nowhere; list \"me:5\" in `rules` to compile it in place of R2\n",
            common(&dir).join("private.toml").display()
        )),
        "{}",
        run.stderr
    );
}

/// `ignoring` with R1 and R6 on by default, so that the defaults enable
/// R6, and the rule sets of `set_up`, R2 and L3 public and me:4 private.
fn r6_on_by_default(name: &str) -> PathBuf {
    let dir = ignoring(name);
    write(&dir, "r/rules/1.toml", &set_default(domain(1), "\"on\"").1);
    write(
        &dir,
        "r/rules/6.toml",
        &set_default(rule(6, 1, ""), "\"on\"").1,
    );
    ok(&statute(&dir, &["init", "--public", "--source", "r"]));
    ok(&statute(&dir, &["add", "R2"]));
    ok(&statute(&dir, &["add", "3"]));
    std::fs::create_dir_all(common(&dir)).unwrap();
    write_private(&dir, &private(&dir));
    ok(&statute(&dir, &["add", "--private", "me:4"]));
    dir
}

#[test]
fn exclude_in_the_private_rule_set_changes_the_private_output_alone() {
    let dir = r6_on_by_default("private-exclude");
    ok(&statute(&dir, &["compile"]));
    let agents = read(&dir, "AGENTS.md");
    let lock = read(&dir, "statute.lock");
    assert!(agents.contains("\n- R6 "), "{agents}");
    assert!(read(&dir, ".statute/AGENTS.full.md").contains("\n- R6 "));

    // Turned off in the private rule set: the private output alone
    // changes.
    let run = statute(&dir, &["remove", "--private", "R6"]);
    ok(&run);
    assert!(
        run.stdout
            .contains("added R6 to `exclude`, as the public layer enables it"),
        "{}",
        run.stdout
    );
    assert!(read_private(&dir, "private.toml").contains("exclude = [\"R6\"]"));
    ok(&statute(&dir, &["compile"]));
    assert_eq!(read(&dir, "AGENTS.md"), agents);
    assert_eq!(read(&dir, "statute.lock"), lock);
    let full = read(&dir, ".statute/AGENTS.full.md");
    assert!(!full.contains("\n- R6 "), "{full}");
    assert!(
        full.contains("\n- me:4 ") && full.contains("\n- R2 "),
        "{full}"
    );
    // The local site, of both layers, marks it as not enabled.
    ok(&statute(&dir, &["site"]));
    assert!(read(&dir, ".statute/site/R/6.html").contains("not enabled"));
    assert!(!read(&dir, ".statute/site/R/2.html").contains("not enabled"));

    // `add --private` takes it out of `exclude` again.
    ok(&statute(&dir, &["add", "--private", "R6"]));
    assert!(!read_private(&dir, "private.toml").contains("\"R6\""));
    ok(&statute(&dir, &["compile"]));
    assert!(read(&dir, ".statute/AGENTS.full.md").contains("\n- R6 "));

    // The control: the same entry in statute.toml changes AGENTS.md.
    ok(&statute(&dir, &["remove", "R6"]));
    assert!(read(&dir, "statute.toml").contains("exclude = [\"R6\"]"));
    ok(&statute(&dir, &["compile"]));
    assert!(!read(&dir, "AGENTS.md").contains("\n- R6 "));
    assert_ne!(read(&dir, "statute.lock"), lock);
}

#[test]
fn the_private_rule_set_turns_on_what_statute_toml_excludes() {
    let dir = r6_on_by_default("private-reopen");
    ok(&statute(&dir, &["remove", "R6"]));
    ok(&statute(&dir, &["compile"]));
    let agents = read(&dir, "AGENTS.md");
    assert!(!agents.contains("\n- R6 "), "{agents}");
    let run = statute(&dir, &["add", "--private", "R6"]);
    ok(&run);
    assert!(read_private(&dir, "private.toml").contains("\"R6\""));
    let run = statute(&dir, &["compile"]);
    ok(&run);
    assert!(!run.stderr.contains("take it out"), "{}", run.stderr);
    assert_eq!(read(&dir, "AGENTS.md"), agents);
    assert!(read(&dir, ".statute/AGENTS.full.md").contains("\n- R6 "));
}

#[test]
fn the_private_rule_set_turns_off_no_rule_an_enabled_rule_requires() {
    let dir = r6_on_by_default("private-exclude-required");
    let shown = common(&dir).join("private.toml").display().to_string();
    // R2 is required by L3 of the public layer and me:4 of the private.
    write_private(
        &dir,
        &private(&dir).replace("rules = []", "rules = [\"me:4\"]\nexclude = [\"R2\"]"),
    );
    let run = statute(&dir, &["compile"]);
    fails_with(
        &run,
        &format!(
            "{shown}: error: rule R2 (Rule 2) cannot be turned off: rule L3 (Rule 3) requires it"
        ),
    );
    assert!(run.stderr.contains(&format!(
        "{shown}: error: rule R2 (Rule 2) cannot be turned off: rule me:4 (Rule 4) requires it"
    )));
    // An entry of both `exclude` and `rules`, and one statute.toml
    // excludes already.
    write_private(
        &dir,
        &private(&dir).replace(
            "rules = []",
            "rules = [\"me:4\", \"R6\"]\nexclude = [\"R6\"]",
        ),
    );
    fails_with(
        &statute(&dir, &["compile"]),
        &format!("{shown}: error: `rules` lists R6 (Rule 6) and `exclude` turns it off; keep one"),
    );
    write_private(
        &dir,
        &private(&dir).replace("rules = []", "rules = [\"me:4\"]"),
    );
    ok(&statute(&dir, &["remove", "R6"]));
    write_private(
        &dir,
        &private(&dir).replace("rules = []", "rules = [\"me:4\"]\nexclude = [\"R6\"]"),
    );
    fails_with(
        &statute(&dir, &["compile"]),
        "`exclude` lists `R6`, which statute.toml lists already",
    );
}

#[test]
fn a_private_rule_needs_a_rule_statute_toml_excludes_in_private_rules() {
    let dir = r6_on_by_default("private-requires-excluded");
    write(
        &dir,
        "me/rules/7.toml",
        &rule(7, 1, "requires = [\"R6\"]").1,
    );
    ok(&statute(&dir, &["remove", "R6"]));
    write_private(
        &dir,
        &private(&dir).replace("rules = []", "rules = [\"me:4\", \"me:7\"]"),
    );
    fails_with(
        &statute(&dir, &["compile"]),
        "statute.toml: error: rule R6 (Rule 6) cannot be turned off: rule me:7 (Rule 7) requires it; take it out of `exclude`, or turn off me:7 too; or, for the private output alone, list it in `rules` of private.toml: `statute add --private R6`\n",
    );
    // Listed there, it is in the private output alone.
    write_private(
        &dir,
        &private(&dir).replace("rules = []", "rules = [\"me:4\", \"me:7\", \"R6\"]"),
    );
    ok(&statute(&dir, &["compile"]));
    assert!(read(&dir, ".statute/AGENTS.full.md").contains("\n- R6 "));
    assert!(!read(&dir, "AGENTS.md").contains("\n- R6 "));
}
