//! `statute compile --private-only`, end to end: a public repository with
//! a private git source, whose new worktrees get the private output from
//! the command alone or from a `post-checkout` hook, without the network.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, rule};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

/// What git ignores through `.git/info/exclude`: the links.
const EXCLUDE: &str = "/CLAUDE.local.md\n/AGENTS.override.md\n";

/// The line the design gives for the hook, with statute's path.
fn hook() -> String {
    format!(
        "#!/bin/sh\n[ \"$3\" = 1 ] && {} compile --private-only; exit 0\n",
        env!("CARGO_BIN_EXE_statute")
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

/// A test's directory: the repository, the private source and a home
/// directory of its own, so that statute's cache is the test's.
struct World {
    at: PathBuf,
    home: PathBuf,
    repo: PathBuf,
}

/// git as the tests run it: in `dir`, with the test's home and none of
/// an outer repository's variables.
fn git_in(w: &World, dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=Example",
            "-c",
            "user.email=example@example.test",
        ])
        .args(args)
        .env("HOME", &w.home)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_CACHE_HOME")
        .env("GIT_CEILING_DIRECTORIES", &w.at)
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

impl World {
    fn git(&self, args: &[&str]) -> String {
        git_in(self, &self.repo, args)
    }

    fn run(&self, dir: &Path, args: &[&str]) -> Run {
        self.run_with(dir, args, &[])
    }

    fn run_with(&self, dir: &Path, args: &[&str], vars: &[(&str, &str)]) -> Run {
        let out = Command::new(env!("CARGO_BIN_EXE_statute"))
            .args(args)
            .current_dir(dir)
            .env("HOME", &self.home)
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("XDG_CACHE_HOME")
            .env("GIT_CEILING_DIRECTORIES", &self.at)
            .env_remove("GIT_DIR")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_WORK_TREE")
            .envs(vars.iter().copied())
            .output()
            .unwrap();
        Run {
            code: out.status.code().unwrap(),
            stdout: String::from_utf8(out.stdout).unwrap(),
            stderr: String::from_utf8(out.stderr).unwrap(),
        }
    }

    /// The private rule set's directory of the clone.
    fn common(&self) -> PathBuf {
        self.repo.join(".git/common/statute")
    }

    /// A new worktree of the clone on a branch of its own, made without
    /// running a hook.
    fn worktree(&self, name: &str) -> PathBuf {
        let tree = self.at.join(name);
        self.git(&[
            "-c",
            "core.hooksPath=/dev/null",
            "worktree",
            "add",
            "-q",
            "-b",
            name,
            tree.to_str().unwrap(),
        ]);
        tree
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

/// The last line of `compile --private-only` when it writes nothing.
const NOT_WRITTEN: &str = "statute: error: the private output is not written; `statute compile` writes it, or says what to fix\n";

/// Exit status 1, with the line that says nothing was written.
#[track_caller]
fn failed(run: &Run) {
    assert_eq!(
        run.code, 1,
        "stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
    assert!(run.stderr.ends_with(NOT_WRITTEN), "{}", run.stderr);
}

/// Nothing printed and nothing written below `.statute/`.
#[track_caller]
fn silent(run: &Run, dir: &Path) {
    ok(run);
    assert_eq!((run.stdout.as_str(), run.stderr.as_str()), ("", ""));
    assert!(!dir.join(".statute").exists());
}

/// A public repository enabling R2 of `R` in `r/`, its private layer
/// enabling me:4 of the private git source `me` (a bare repository in
/// `me.git`), locked and committed; the links ignored. The git source is
/// in statute's cache once `lock` has read it.
fn world(name: &str) -> World {
    let at = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("private-only")
        .join(name);
    let _ = std::fs::remove_dir_all(&at);
    let w = World {
        home: at.join("home"),
        repo: at.join("repo"),
        at,
    };
    std::fs::create_dir_all(&w.home).unwrap();
    std::fs::create_dir_all(&w.repo).unwrap();
    w.git(&["init", "-q", "-b", "main"]);
    write(&w.repo, "r/constitution/2026-10/index.toml", INDEX);
    common::provisions(&w.repo.join("r/constitution/2026-10"));
    write(
        &w.repo,
        "r/constitution/2026-10/generated/constitution.en.agent.md",
        "# Constitution\n",
    );
    write(
        &w.repo,
        "r/constitution/2026-10/articles/04-authority/article.toml",
        "id = \"authority\"\n",
    );
    for (sub, (id, text)) in [
        ("r/rules", domain(1)),
        ("r/rules", rule(2, 1, "")),
        ("me-work/rules", domain(1)),
        ("me-work/rules", rule(4, 1, "")),
    ] {
        let dir = if sub.starts_with("me-work") {
            &w.at
        } else {
            &w.repo
        };
        write(dir, &format!("{sub}/{id}.toml"), &text);
    }
    let work = w.at.join("me-work");
    git_in(&w, &work, &["init", "-q", "-b", "main"]);
    git_in(&w, &work, &["add", "-A"]);
    git_in(&w, &work, &["commit", "-q", "-m", "rules"]);
    let bare = w.at.join("me.git");
    git_in(
        &w,
        &w.at,
        &[
            "clone",
            "-q",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    write(&w.repo, ".git/info/exclude", EXCLUDE);
    ok(&w.run(&w.repo, &["init", "--public", "--source", "r"]));
    ok(&w.run(&w.repo, &["add", "R2"]));
    std::fs::create_dir_all(w.common()).unwrap();
    std::fs::write(
        w.common().join("private.toml"),
        format!(
            "rules = [\"me:4\"]\n\n[sources]\nme = {{ git = \"{}\", branch = \"main\", public = false }}\n",
            bare.display()
        ),
    )
    .unwrap();
    ok(&w.run(&w.repo, &["lock"]));
    w.git(&["add", "."]);
    w.git(&["commit", "-q", "-m", "rule set"]);
    w
}

#[test]
fn a_new_worktree_gets_the_private_output_without_the_network() {
    let w = world("worktree");
    // The source's address is gone: only the cache has its commit.
    std::fs::rename(w.at.join("me.git"), w.at.join("moved.git")).unwrap();
    let tree = w.worktree("other");
    let run = w.run(&tree, &["compile", "--private-only"]);
    ok(&run);
    assert_eq!(run.stderr, "", "{}", run.stderr);
    let full = read(&tree, ".statute/AGENTS.full.md");
    assert!(full.contains("\n- me:4 "), "{full}");
    assert!(full.contains("\n- R2 "), "{full}");
    assert_eq!(
        read(&tree, ".statute/.gitignore"),
        "# written by statute\n/AGENTS.full.md\n/site/\n"
    );
    for link in ["CLAUDE.local.md", "AGENTS.override.md"] {
        assert_eq!(
            std::fs::read_link(tree.join(link)).unwrap(),
            Path::new(".statute/AGENTS.full.md"),
            "{link}"
        );
    }
    // Nothing public is written.
    assert!(!tree.join("AGENTS.md").exists());
    assert!(!tree.join("statute-checks.toml").exists());
}

#[test]
fn a_commit_missing_from_the_cache_fails_and_nothing_is_fetched() {
    let w = world("missing");
    // The cache is gone, and the source's address leads nowhere.
    std::fs::remove_dir_all(w.home.join(".cache")).unwrap();
    let gone = w.at.join("gone.git");
    let me = w.at.join("me.git");
    for file in ["private.toml", "private.lock"] {
        let path = w.common().join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains(me.to_str().unwrap()), "{file}: {text}");
        std::fs::write(
            &path,
            text.replace(me.to_str().unwrap(), gone.to_str().unwrap()),
        )
        .unwrap();
    }
    // A git on PATH that writes down how it is called.
    let bin = w.at.join("bin");
    let log = w.at.join("git.log");
    let real = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|d| d.join("git"))
        .find(|p| p.is_file())
        .unwrap();
    write(
        &w.at,
        "bin/git",
        &format!(
            "#!/bin/sh\necho \"$*\" >> {}\nexec {} \"$@\"\n",
            log.display(),
            real.display()
        ),
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(bin.join("git"), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let tree = w.worktree("other");
    let run = w.run_with(&tree, &["compile", "--private-only"], &[("PATH", &path)]);
    failed(&run);
    assert!(
        run.stderr.contains(&format!(
            "statute: error: statute.toml: error: source `me` ({}): commit ",
            gone.display()
        )) && run.stderr.contains(
            "which the lock file records, is not in statute's cache, and nothing is fetched"
        ),
        "{}",
        run.stderr
    );
    assert!(run.stderr.ends_with(NOT_WRITTEN), "{}", run.stderr);
    assert!(!tree.join(".statute").exists());
    assert!(tree.join("CLAUDE.local.md").symlink_metadata().is_err());
    let calls = std::fs::read_to_string(&log).unwrap();
    assert!(calls.contains("rev-parse"), "{calls}");
    assert!(
        !calls.contains("clone") && !calls.contains("fetch"),
        "{calls}"
    );
    // The plain compile does go to the address, through the same git.
    let run = w.run_with(&tree, &["compile"], &[("PATH", &path)]);
    assert_eq!(run.code, 1, "{}", run.stderr);
    assert!(std::fs::read_to_string(&log).unwrap().contains("clone"));
}

#[test]
fn nothing_to_compile_is_silent() {
    let w = world("silent");
    // No statute.toml.
    let empty = w.at.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    git_in(&w, &empty, &["init", "-q"]);
    silent(&w.run(&empty, &["compile", "--private-only"]), &empty);
    // Outside a git repository.
    std::fs::remove_dir_all(empty.join(".git")).unwrap();
    std::fs::copy(w.repo.join("statute.toml"), empty.join("statute.toml")).unwrap();
    silent(&w.run(&empty, &["compile", "--private-only"]), &empty);
    // No private rule set.
    let tree = w.worktree("other");
    let private = std::fs::read_to_string(w.common().join("private.toml")).unwrap();
    std::fs::remove_file(w.common().join("private.toml")).unwrap();
    silent(&w.run(&tree, &["compile", "--private-only"]), &tree);
    // A private repository.
    std::fs::write(w.common().join("private.toml"), private).unwrap();
    let text = read(&tree, "statute.toml");
    write(
        &tree,
        "statute.toml",
        &text.replace("public = true", "public = false"),
    );
    silent(&w.run(&tree, &["compile", "--private-only"]), &tree);
}

#[test]
fn what_compile_would_fail_on_fails_it() {
    let w = world("warnings");
    let tree = w.worktree("other");
    // R2 changed since the lock file.
    let r2 = read(&tree, "r/rules/2.toml");
    write(
        &tree,
        "r/rules/2.toml",
        &r2.replace("Do the thing 2.", "Do the thing 2 well."),
    );
    let run = w.run(&tree, &["compile", "--private-only"]);
    failed(&run);
    assert!(
        run.stderr.starts_with("statute: error: statute.lock is not what statute.toml and the rules give now; run `statute lock` first\n"),
        "{}",
        run.stderr
    );
    assert!(!tree.join(".statute").exists());
    // The links not ignored through .git/info/exclude.
    write(&tree, "r/rules/2.toml", &r2);
    write(&w.repo, ".git/info/exclude", "");
    let run = w.run(&tree, &["compile", "--private-only"]);
    failed(&run);
    assert!(
        run.stderr.contains("add these lines to")
            && run.stderr.contains(
                "statute: error: CLAUDE.local.md, AGENTS.override.md: git must ignore these links"
            ),
        "{}",
        run.stderr
    );
    assert!(!tree.join(".statute/AGENTS.full.md").exists());
}

#[test]
fn a_post_checkout_hook_writes_the_private_output_of_a_new_worktree() {
    let w = world("hook");
    let path = w.git(&[
        "rev-parse",
        "--path-format=absolute",
        "--git-path",
        "hooks/post-checkout",
    ]);
    let path = Path::new(path.trim_end());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, hook()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let tree = w.at.join("hooked");
    w.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "hooked",
        tree.to_str().unwrap(),
    ]);
    let full = read(&tree, ".statute/AGENTS.full.md");
    assert!(full.contains("\n- me:4 "), "{full}");
    assert!(tree.join("CLAUDE.local.md").symlink_metadata().is_ok());
    assert!(!tree.join("AGENTS.md").exists());
    // A checkout the hook cannot serve still succeeds.
    std::fs::remove_dir_all(w.home.join(".cache")).unwrap();
    let other = w.at.join("unserved");
    w.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "unserved",
        other.to_str().unwrap(),
    ]);
    assert!(!other.join(".statute").exists());
}

/// `world` with `R` as a git source, so that `statute.lock` records its
/// commit, and then both sources moved on: `R` drops R2, which the rule
/// set enables, so that reading `R` at its newest commit fails, and `me`
/// changes me:4's text and revision. Also the commits of `me` before and
/// after.
fn moved(name: &str) -> (World, String, String) {
    let w = world(name);
    let r_work = w.at.join("r-work");
    let r_bare = w.at.join("r.git");
    assert!(
        Command::new("cp")
            .arg("-r")
            .arg(w.repo.join("r"))
            .arg(&r_work)
            .status()
            .unwrap()
            .success()
    );
    git_in(&w, &r_work, &["init", "-q", "-b", "main"]);
    git_in(&w, &r_work, &["add", "-A"]);
    git_in(&w, &r_work, &["commit", "-q", "-m", "rules"]);
    let r_bare = r_bare.to_str().unwrap();
    git_in(
        &w,
        &w.at,
        &["clone", "-q", "--bare", r_work.to_str().unwrap(), r_bare],
    );
    let manifest = read(&w.repo, "statute.toml");
    write(
        &w.repo,
        "statute.toml",
        &manifest.replace(
            "R = { path = \"r\" }",
            &format!("R = {{ git = \"{r_bare}\", branch = \"main\", public = true }}"),
        ),
    );
    ok(&w.run(&w.repo, &["lock"]));
    w.git(&["commit", "-q", "-am", "R from git"]);
    let old = head(&w, &w.at.join("me.git"));
    git_in(&w, &r_work, &["rm", "-q", "rules/2.toml"]);
    git_in(&w, &r_work, &["commit", "-q", "-m", "no R2"]);
    git_in(&w, &r_work, &["push", "-q", r_bare, "main"]);
    let work = w.at.join("me-work");
    let me4 = read(&work, "rules/4.toml");
    write(
        &work,
        "rules/4.toml",
        &me4.replace("revision = 1", "revision = 2")
            .replace("Do the thing 4.", "Do the new thing 4."),
    );
    git_in(&w, &work, &["commit", "-q", "-am", "new"]);
    git_in(
        &w,
        &work,
        &["push", "-q", w.at.join("me.git").to_str().unwrap(), "main"],
    );
    let new = head(&w, &work);
    (w, old, new)
}

fn text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

fn head(w: &World, dir: &Path) -> String {
    git_in(w, dir, &["rev-parse", "HEAD"]).trim_end().to_owned()
}

/// What `upgrade --private` prints when `me` moved from `old` to `new`.
fn moved_lines(old: &str, new: &str) -> String {
    format!(
        "source `me`: commit {} -> {}\nchanged: rule me:4 (Rule 4): revision 1 -> 2\n",
        &old[..8],
        &new[..8]
    )
}

/// `upgrade --private` in `tree` moves its private output to me:4's new
/// text and leaves every tracked file as it is.
#[track_caller]
fn upgrades(w: &World, tree: &Path) -> Run {
    let run = w.run(tree, &["upgrade", "--private"]);
    ok(&run);
    assert_eq!(git_in(w, tree, &["status", "--porcelain"]), "");
    ok(&w.run(tree, &["compile", "--private-only"]));
    let full = read(tree, ".statute/AGENTS.full.md");
    assert!(
        full.contains("Do the new thing 4.") && !full.contains("Do the thing 4."),
        "{full}"
    );
    run
}

#[test]
fn upgrade_private_moves_only_a_new_worktrees_private_lock() {
    let (w, old, new) = moved("upgrade");
    let tree = w.worktree("upgraded");
    let run = upgrades(&w, &tree);
    let main = w.common().join("private.lock");
    let own = w.repo.join(".git/worktrees/upgraded/statute/private.lock");
    assert_eq!(
        run.stdout,
        format!(
            "compared with {}, the main worktree's: this worktree has no private.lock of its own\n{}wrote {}: 2 rules enabled, 1 of them by the private layer\n",
            main.display(),
            moved_lines(&old, &new),
            own.display()
        )
    );
    assert!(text(&own).contains(&new));
    let main = text(&main);
    assert!(main.contains(&old) && !main.contains(&new), "{main}");
    // Without it, a new worktree reads the main worktree's lock.
    let tree = w.worktree("plain");
    ok(&w.run(&tree, &["compile", "--private-only"]));
    let full = read(&tree, ".statute/AGENTS.full.md");
    assert!(
        full.contains("Do the thing 4.") && !full.contains("Do the new thing 4."),
        "{full}"
    );
}

#[test]
fn upgrade_private_moves_a_worktrees_own_private_lock() {
    let (w, old, new) = moved("upgrade-own");
    let tree = w.worktree("own");
    ok(&w.run(&tree, &["lock"]));
    let own = w.repo.join(".git/worktrees/own/statute/private.lock");
    assert!(text(&own).contains(&old));
    let run = upgrades(&w, &tree);
    assert_eq!(
        run.stdout,
        format!(
            "{}wrote {}: 2 rules enabled, 1 of them by the private layer\n",
            moved_lines(&old, &new),
            own.display()
        )
    );
    assert!(text(&own).contains(&new));
    let main = read(&w.common(), "private.lock");
    assert!(main.contains(&old) && !main.contains(&new), "{main}");
    // Run again, nothing moves.
    let again = w.run(&tree, &["upgrade", "--private"]);
    ok(&again);
    assert_eq!(
        again.stdout,
        format!(
            "source `me`: still at commit {}\n{} is up to date\nwrote {1}: 2 rules enabled, 1 of them by the private layer\n",
            &new[..8],
            own.display()
        )
    );
}

#[test]
fn upgrade_private_moves_the_main_worktrees_private_lock() {
    let (w, old, new) = moved("upgrade-main");
    let tree = w.worktree("own");
    ok(&w.run(&tree, &["lock"]));
    let run = upgrades(&w, &w.repo);
    let main = w.common().join("private.lock");
    assert_eq!(
        run.stdout,
        format!(
            "{}wrote {}: 2 rules enabled, 1 of them by the private layer\n",
            moved_lines(&old, &new),
            main.display()
        )
    );
    assert!(text(&main).contains(&new));
    let own = read(&w.repo, ".git/worktrees/own/statute/private.lock");
    assert!(own.contains(&old) && !own.contains(&new), "{own}");
}

#[test]
fn upgrade_private_says_when_the_old_commits_do_not_resolve() {
    let (w, old, _) = moved("upgrade-unresolved");
    // me:5 exists only at the new commit, so the old one does not resolve.
    let work = w.at.join("me-work");
    write(&work, "rules/5.toml", &rule(5, 1, "").1);
    git_in(&w, &work, &["add", "-A"]);
    git_in(&w, &work, &["commit", "-q", "-m", "me:5"]);
    git_in(
        &w,
        &work,
        &["push", "-q", w.at.join("me.git").to_str().unwrap(), "main"],
    );
    let new = head(&w, &work);
    let manifest = w.common().join("private.toml");
    write(
        &w.common(),
        "private.toml",
        &text(&manifest).replace("[\"me:4\"]", "[\"me:4\", \"me:5\"]"),
    );
    let run = upgrades(&w, &w.repo);
    let main = w.common().join("private.lock");
    assert_eq!(
        run.stdout,
        format!(
            "source `me`: commit {} -> {}\nnote: the rules are not compared: the private layer does not resolve at the commits of {}\nwrote {2}: 3 rules enabled, 2 of them by the private layer\n",
            &old[..8],
            &new[..8],
            main.display()
        )
    );
}
