//! `statute hooks install`, run as a user runs it, in repositories written
//! under Cargo's test directory.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const POST_CHECKOUT: &str =
    "#!/bin/sh\n# written by statute\n[ \"$3\" = 1 ] && statute compile --private-only || true\n";
const POST_MERGE: &str =
    "#!/bin/sh\n# written by statute\nstatute compile --private-only || true\n";

/// A test's directory, holding a repository `repo` with a `statute.toml`.
fn world(name: &str) -> (PathBuf, PathBuf) {
    let at = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("hooks")
        .join(name);
    let _ = std::fs::remove_dir_all(&at);
    let repo = at.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&at, &repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("statute.toml"), "rules = []\n").unwrap();
    (at, repo)
}

/// git as the tests run it: in `dir`, with none of an outer repository's
/// variables and no repository above `at`.
fn git(at: &Path, dir: &Path, args: &[&str]) -> String {
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
        .env("GIT_CEILING_DIRECTORIES", at)
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

fn install(at: &Path, dir: &Path) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_statute"))
        .args(["hooks", "install"])
        .current_dir(dir)
        .env("GIT_CEILING_DIRECTORIES", at)
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
fn code(run: &Run, code: i32) {
    assert_eq!(
        run.code, code,
        "stdout: {}\nstderr: {}",
        run.stdout, run.stderr
    );
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn writes_both_hooks_executable_in_gits_hooks_directory() {
    let (at, repo) = world("written");
    let hooks = repo.join(".git/hooks");
    let run = install(&at, &repo);
    code(&run, 0);
    assert_eq!(
        run.stdout,
        format!(
            "wrote {}\nwrote {}\n",
            hooks.join("post-checkout").display(),
            hooks.join("post-merge").display()
        )
    );
    assert_eq!(run.stderr, "");
    assert_eq!(read(&hooks.join("post-checkout")), POST_CHECKOUT);
    assert_eq!(read(&hooks.join("post-merge")), POST_MERGE);
    assert_eq!(mode(&hooks.join("post-checkout")), 0o755);
    assert_eq!(mode(&hooks.join("post-merge")), 0o755);
}

#[test]
fn follows_core_hooks_path_and_makes_its_directory() {
    let (at, repo) = world("hooks-path");
    git(&at, &repo, &["config", "core.hooksPath", "tools/hooks"]);
    let run = install(&at, &repo);
    code(&run, 0);
    assert_eq!(read(&repo.join("tools/hooks/post-checkout")), POST_CHECKOUT);
    assert_eq!(read(&repo.join("tools/hooks/post-merge")), POST_MERGE);
    assert!(!repo.join(".git/hooks/post-checkout").exists());
}

#[test]
fn a_linked_worktree_installs_the_clones_hooks() {
    let (at, repo) = world("worktree");
    git(&at, &repo, &["add", "."]);
    git(&at, &repo, &["commit", "-q", "-m", "rule set"]);
    let tree = at.join("tree");
    git(
        &at,
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "tree",
            tree.to_str().unwrap(),
        ],
    );
    code(&install(&at, &tree), 0);
    assert_eq!(read(&repo.join(".git/hooks/post-merge")), POST_MERGE);
}

#[test]
fn statutes_hooks_as_they_are_now_are_left_and_older_ones_rewritten() {
    let (at, repo) = world("again");
    let hooks = repo.join(".git/hooks");
    code(&install(&at, &repo), 0);
    let run = install(&at, &repo);
    code(&run, 0);
    assert_eq!(
        run.stdout,
        format!(
            "{}: up to date\n{}: up to date\n",
            hooks.join("post-checkout").display(),
            hooks.join("post-merge").display()
        )
    );
    // An older hook of statute's, and one that lost its mode.
    let old = "#!/bin/sh\n# written by statute\nstatute compile --private-only; exit 0\n";
    std::fs::write(hooks.join("post-merge"), old).unwrap();
    std::fs::set_permissions(
        hooks.join("post-checkout"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    let run = install(&at, &repo);
    code(&run, 0);
    assert_eq!(
        run.stdout,
        format!(
            "wrote {}\nwrote {}\n",
            hooks.join("post-checkout").display(),
            hooks.join("post-merge").display()
        )
    );
    assert_eq!(read(&hooks.join("post-merge")), POST_MERGE);
    assert_eq!(mode(&hooks.join("post-checkout")), 0o755);
}

#[test]
fn a_hook_statute_did_not_write_is_left_and_fails_after_the_other_is_written() {
    let (at, repo) = world("foreign");
    let hooks = repo.join(".git/hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    // statute's marker, but not as its second line.
    let mine = "#!/bin/sh\necho mine\n# written by statute\n";
    std::fs::write(hooks.join("post-checkout"), mine).unwrap();
    std::fs::set_permissions(
        hooks.join("post-checkout"),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    let run = install(&at, &repo);
    code(&run, 1);
    assert_eq!(read(&hooks.join("post-checkout")), mine);
    assert_eq!(mode(&hooks.join("post-checkout")), 0o700);
    assert_eq!(read(&hooks.join("post-merge")), POST_MERGE);
    assert_eq!(
        run.stdout,
        format!("wrote {}\n", hooks.join("post-merge").display())
    );
    let path = hooks.join("post-checkout");
    assert!(
        run.stderr.starts_with(&format!(
            "{}: error: a hook statute did not write is there, and is left as it is; add this line to it, where it runs before the hook exits: [ \"$3\" = 1 ] && statute compile --private-only || true\n",
            path.display()
        )),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr
            .contains("`pre-commit install --hook-type post-checkout`"),
        "{}",
        run.stderr
    );
}

#[test]
fn without_statute_toml_nothing_is_written() {
    let (at, repo) = world("no-rule-set");
    std::fs::remove_file(repo.join("statute.toml")).unwrap();
    let run = install(&at, &repo);
    code(&run, 1);
    assert!(
        run.stderr.starts_with("statute.toml: error: cannot read:"),
        "{}",
        run.stderr
    );
    assert!(!repo.join(".git/hooks/post-checkout").exists());
}

#[test]
fn outside_a_git_repository_it_fails() {
    let (at, _) = world("no-git");
    let plain = at.join("plain");
    std::fs::create_dir_all(&plain).unwrap();
    std::fs::write(plain.join("statute.toml"), "rules = []\n").unwrap();
    let run = install(&at, &plain);
    code(&run, 1);
    assert_eq!(
        run.stderr,
        "statute: error: not in a git repository, so there are no hooks to install\n"
    );
}

#[test]
fn below_the_top_of_the_work_tree_it_fails() {
    let (at, repo) = world("below");
    let sub = repo.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("statute.toml"), "rules = []\n").unwrap();
    let run = install(&at, &sub);
    code(&run, 1);
    assert!(
        run.stderr.contains("not at the top of a work tree"),
        "{}",
        run.stderr
    );
    assert!(!repo.join(".git/hooks/post-checkout").exists());
}
