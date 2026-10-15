//! The global configuration and git sources, end to end: a home directory
//! of its own, sources in bare repositories on disk (`file://`, no
//! network), and repositories that draw on them
//! (`docs/design/global-configuration.md`).

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, rule};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

/// The private rule set and its lock file of a repository made with
/// `git init`.
const PRIVATE: &str = ".git/common/statute/private.toml";
const PRIVATE_LOCK: &str = ".git/common/statute/private.lock";

fn base(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("global")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

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
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

/// A source of rules: a work tree `<name>` with the constitution and
/// `nodes`, committed and pushed to the bare repository `<name>.git`.
/// Returns the work tree and the bare repository's `file://` address.
fn source(at: &Path, name: &str, nodes: &[(u64, String)]) -> (PathBuf, String) {
    let work = at.join(name);
    std::fs::create_dir_all(&work).unwrap();
    git(&work, &["init", "-q", "-b", "main"]);
    git(&work, &["config", "user.email", "test@example.test"]);
    git(&work, &["config", "user.name", "Test"]);
    write(&work, "constitution/2026-10/index.toml", INDEX);
    write(
        &work,
        "constitution/2026-10/generated/constitution.en.agent.md",
        "# The constitution\n",
    );
    write(
        &work,
        "constitution/2026-10/articles/04-authority/article.toml",
        "id = \"authority\"\n",
    );
    for (id, text) in nodes {
        write(&work, &format!("rules/{id}.toml"), text);
    }
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "first"]);
    let bare = at.join(format!("{name}.git"));
    git(
        at,
        &[
            "clone",
            "-q",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
    (work, format!("file://{}", bare.display()))
}

/// Commits a new text of rule `node` in the work tree and pushes it;
/// returns the new commit.
fn change(work: &Path, (id, text): (u64, String)) -> String {
    write(work, &format!("rules/{id}.toml"), &text);
    git(work, &["commit", "-q", "-am", "change"]);
    git(work, &["push", "-q", "origin", "main"]);
    git(work, &["rev-parse", "HEAD"])
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn statute(home: &Path, dir: &Path, args: &[&str]) -> Run {
    statute_with(home, dir, args, &[])
}

/// As `statute`, with the variables `vars` set.
fn statute_with(home: &Path, dir: &Path, args: &[&str], vars: &[(&str, &Path)]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_statute"))
        .env("HOME", home)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_CACHE_HOME")
        .envs(vars.iter().copied())
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
    assert_eq!(run.code, 0, "{}\n{}", run.stdout, run.stderr);
}

#[track_caller]
fn fails(run: &Run, needle: &str) {
    assert_eq!(run.code, 1, "{}\n{}", run.stdout, run.stderr);
    assert!(
        run.stderr.contains(needle),
        "{needle:?} not in {}",
        run.stderr
    );
}

/// The first-party source, `r`, with domain 1 and rule 2; a home
/// directory whose global configuration `statute init --global` wrote
/// with `R` at it; and an empty git repository `repo`.
struct World {
    at: PathBuf,
    home: PathBuf,
    r: PathBuf,
    r_address: String,
    repo: PathBuf,
}

fn world(name: &str) -> World {
    let at = base(name);
    let (r, r_address) = source(&at, "r", &[domain(1), rule(2, 1, "")]);
    let home = at.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let repo = at.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    let run = statute(&home, &at, &["init", "--global", "--source", &r_address]);
    ok(&run);
    World {
        at,
        home,
        r,
        r_address,
        repo,
    }
}

impl World {
    fn run(&self, args: &[&str]) -> Run {
        statute(&self.home, &self.repo, args)
    }

    /// As `run`, with a fake gh first on `PATH` that answers `repo view
    /// --json visibility` with `visibility`, or fails as gh does outside
    /// a GitHub repository when it is `None`.
    fn run_gh(&self, args: &[&str], visibility: Option<&str>) -> Run {
        let bin = self.at.join("gh-bin");
        let _ = std::fs::remove_dir_all(&bin);
        std::fs::create_dir_all(&bin).unwrap();
        let answer = match visibility {
            Some(v) => format!("echo '{{\"visibility\":\"{v}\"}}'"),
            None => "echo 'not a GitHub repository' >&2; exit 1".to_owned(),
        };
        let script = bin.join("gh");
        std::fs::write(
            &script,
            format!("#!/bin/sh\n[ \"$*\" = 'repo view --json visibility' ] || exit 9\n{answer}\n"),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_statute"))
            .env("HOME", &self.home)
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("XDG_CACHE_HOME")
            .env(
                "PATH",
                format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
            )
            .args(args)
            .current_dir(&self.repo)
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

    /// A source on disk at `<at>/<name>` with domain 1 and rule `id`.
    fn on_disk(&self, name: &str, id: u64) {
        let dir = self.at.join(name);
        write(&dir, "rules/1.toml", &domain(1).1);
        write(&dir, &format!("rules/{id}.toml"), &rule(id, 1, "").1);
    }

    /// Adds a source to the global configuration.
    fn configure(&self, line: &str) {
        let path = self.home.join(".config/statute/config.toml");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, format!("{text}{line}\n")).unwrap();
    }
}

#[test]
fn init_global_writes_the_configuration_and_clones() {
    let w = world("init-global");
    let config = read(&w.home, ".config/statute/config.toml");
    assert!(
        config.contains(&format!(
            "R = {{ git = \"{}\", branch = \"main\" }}",
            w.r_address
        )),
        "{config}"
    );
    let repos: Vec<_> = std::fs::read_dir(w.home.join(".cache/statute/repos"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(repos.len(), 1, "{repos:?}");
    assert!(repos[0].starts_with("r-"), "{repos:?}");
    // A second run leaves the configuration as it is.
    fails(
        &statute(&w.home, &w.at, &["init", "--global"]),
        "already exists",
    );
    assert_eq!(read(&w.home, ".config/statute/config.toml"), config);
}

#[test]
fn the_xdg_variables_place_the_configuration_and_the_cache() {
    let at = base("xdg");
    let (_, address) = source(&at, "r", &[domain(1), rule(2, 1, "")]);
    let init = ["init", "--global", "--source", address.as_str()];
    // Unset: under the home directory.
    let home = at.join("home");
    let run = statute(&home, &at, &init);
    ok(&run);
    assert!(
        run.stdout
            .starts_with("wrote ~/.config/statute/config.toml\n"),
        "{}",
        run.stdout
    );
    assert!(home.join(".config/statute/config.toml").is_file());
    assert!(home.join(".cache/statute/repos").is_dir());
    // Set: there, and nothing under the home directory.
    let home = at.join("home-xdg");
    let (config, cache) = (at.join("config"), at.join("cache"));
    let vars: &[(&str, &Path)] = &[("XDG_CONFIG_HOME", &config), ("XDG_CACHE_HOME", &cache)];
    let run = statute_with(&home, &at, &init, vars);
    ok(&run);
    let wrote = format!("wrote {}\n", config.join("statute/config.toml").display());
    assert!(run.stdout.starts_with(&wrote), "{}", run.stdout);
    assert!(cache.join("statute/repos").is_dir());
    let repo = at.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    for args in [&["init", "--private"][..], &["add", "R2"], &["compile"]] {
        ok(&statute_with(&home, &repo, args, vars));
    }
    let commit = git(&at.join("r"), &["rev-parse", "HEAD"]);
    assert!(cache.join("statute/snapshots").join(&commit).is_dir());
    assert!(!home.exists());
    // Empty or relative: ignored, as the specification says.
    let home = at.join("home-ignored");
    let (empty, relative) = (Path::new(""), Path::new("config"));
    let vars: &[(&str, &Path)] = &[("XDG_CONFIG_HOME", empty), ("XDG_CACHE_HOME", relative)];
    ok(&statute_with(&home, &at, &init, vars));
    assert!(home.join(".config/statute/config.toml").is_file());
    assert!(home.join(".cache/statute/repos").is_dir());
    assert!(!at.join("config/statute/repos").exists());
}

#[test]
fn a_repository_draws_on_the_global_first_party_source() {
    let w = world("draws");
    let commit = git(&w.r, &["rev-parse", "HEAD"]);
    ok(&w.run(&["init", "--private"]));
    let manifest = read(&w.repo, "statute.toml");
    assert!(!manifest.contains("[sources]"), "{manifest}");
    ok(&w.run(&["add", "R2"]));
    let lock = read(&w.repo, "statute.lock");
    for line in [
        format!("git = \"{}\"", w.r_address),
        "branch = \"main\"".to_owned(),
        format!("commit = \"{commit}\""),
    ] {
        assert!(lock.contains(&line), "{line} not in {lock}");
    }
    let run = w.run(&["compile"]);
    ok(&run);
    let agents = read(&w.repo, "AGENTS.md");
    assert!(agents.contains("Do the thing 2."), "{agents}");
    assert!(
        agents.contains(&format!("(`{} at {commit}`)", w.r_address)),
        "{agents}"
    );
    // statute reads the commit's files, never a work tree of the clone.
    assert!(
        w.home
            .join(".cache/statute/snapshots")
            .join(&commit)
            .is_dir()
    );
}

#[test]
fn a_new_source_is_locked_at_the_newest_commit() {
    let w = world("new-source");
    // The clone `init --global` made is behind once `r` moves on.
    let new = change(
        &w.r,
        edit(rule(2, 1, ""), "Do the thing 2.", "Do the new thing 2."),
    );
    ok(&w.run(&["init", "--private"]));
    ok(&w.run(&["add", "R2"]));
    let lock = read(&w.repo, "statute.lock");
    assert!(lock.contains(&new), "{new} not in {lock}");
}

#[test]
fn nothing_changes_until_upgrade() {
    let w = world("upgrade");
    ok(&w.run(&["init", "--private"]));
    ok(&w.run(&["add", "R2"]));
    let old = git(&w.r, &["rev-parse", "HEAD"]);
    let new = change(
        &w.r,
        edit(rule(2, 1, ""), "Do the thing 2.", "Do the new thing 2."),
    );
    // Another repository upgrades, which fetches the new commit into the
    // clone both share.
    let other = w.at.join("other-repo");
    std::fs::create_dir_all(&other).unwrap();
    git(&other, &["init", "-q"]);
    ok(&statute(&w.home, &other, &["init", "--private"]));
    ok(&statute(&w.home, &other, &["add", "R2"]));
    ok(&statute(&w.home, &other, &["upgrade"]));
    assert!(read(&other, "statute.lock").contains(&new));
    // `lock` and `compile` keep the locked commit and its text.
    ok(&w.run(&["lock"]));
    assert!(read(&w.repo, "statute.lock").contains(&old));
    ok(&w.run(&["compile"]));
    assert!(read(&w.repo, "AGENTS.md").contains("Do the thing 2."));
    // `upgrade` fetches and moves to the newest commit.
    let run = w.run(&["upgrade"]);
    ok(&run);
    let lock = read(&w.repo, "statute.lock");
    assert!(lock.contains(&new) && !lock.contains(&old), "{lock}");
    ok(&w.run(&["compile"]));
    assert!(read(&w.repo, "AGENTS.md").contains("Do the new thing 2."));
}

#[test]
fn a_lock_file_of_another_branch_is_stale() {
    let w = world("stale");
    ok(&w.run(&["init", "--private"]));
    ok(&w.run(&["add", "R2"]));
    git(&w.r, &["push", "-q", "origin", "main:other"]);
    let path = w.home.join(".config/statute/config.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        text.replace("branch = \"main\"", "branch = \"other\""),
    )
    .unwrap();
    fails(&w.run(&["compile"]), "run `statute lock`");
    // `lock` follows the new branch.
    ok(&w.run(&["lock"]));
    assert!(read(&w.repo, "statute.lock").contains("branch = \"other\""));
    ok(&w.run(&["compile"]));
}

#[test]
fn the_lock_file_stands_in_for_a_missing_configuration() {
    let w = world("fallback");
    let (_, me) = source(&w.at, "me", &[domain(1), rule(4, 1, "")]);
    w.configure(&format!(
        "me = {{ git = \"{me}\", branch = \"main\", public = false }}"
    ));
    ok(&w.run(&["init", "--private"]));
    ok(&w.run(&["add", "me:4"]));
    let lock = read(&w.repo, "statute.lock");
    assert!(
        lock.contains("name = \"me\"") && lock.contains("public = false"),
        "{lock}"
    );
    // Another home, without the configuration: `compile` reads `me` from
    // the lock file, while `lock`, which reads the configuration alone,
    // cannot find it.
    let other = w.at.join("other-home");
    std::fs::create_dir_all(&other).unwrap();
    ok(&statute(&other, &w.repo, &["compile"]));
    assert!(read(&w.repo, "AGENTS.md").contains("Do the thing 4."));
    fails(&statute(&other, &w.repo, &["lock"]), "`me`");
}

#[test]
fn a_rule_set_takes_a_global_source_out_or_replaces_it() {
    let w = world("replace");
    let (_, team) = source(&w.at, "team", &[domain(1), rule(7, 1, "")]);
    w.configure(&format!(
        "team = {{ git = \"{team}\", branch = \"main\", public = true }}"
    ));
    ok(&w.run(&["init", "--private"]));
    ok(&w.run(&["add", "team:7"]));
    let path = w.repo.join("statute.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{text}\n[sources]\nteam = false\n")).unwrap();
    fails(
        &w.run(&["lock"]),
        "names source `team`, which [sources] does not define",
    );
    // A source on disk of the same name replaces it.
    let local = w.at.join("team-here");
    write(&local, "rules/1.toml", &domain(1).1);
    write(&local, "rules/7.toml", &rule(7, 1, "").1);
    std::fs::write(
        &path,
        format!("{text}\n[sources]\nteam = {{ path = \"../team-here\", public = true }}\n"),
    )
    .unwrap();
    ok(&w.run(&["lock"]));
    assert!(read(&w.repo, "statute.lock").contains("path = \"../team-here\""));
}

#[test]
fn a_private_global_source_joins_the_private_layer_of_a_public_repository() {
    let w = world("private-layer");
    let (_, me) = source(&w.at, "me", &[domain(1), rule(4, 1, "")]);
    w.configure(&format!(
        "me = {{ git = \"{me}\", branch = \"main\", public = false }}"
    ));
    ok(&w.run(&["init", "--public"]));
    // The public rule set cannot enable it.
    fails(&w.run(&["add", "me:4"]), "statute add --private me:4");
    ok(&w.run(&["add", "--private", "me:4"]));
    let public_lock = std::fs::read_to_string(w.repo.join("statute.lock")).unwrap_or_default();
    assert!(!public_lock.contains("name = \"me\""), "{public_lock}");
    let private_lock = read(&w.repo, PRIVATE_LOCK);
    assert!(
        private_lock.contains(&format!(
            "name = \"me\"\ngit = \"{me}\"\nbranch = \"main\"\ncommit = "
        )),
        "{private_lock}"
    );
    // The private layer adds names only.
    write(
        &w.repo,
        PRIVATE,
        &format!(
            "rules = [\"me:4\"]\n\n[sources]\nR = {{ path = \"{}\", public = true }}\n",
            w.r.display()
        ),
    );
    fails(
        &w.run(&["lock"]),
        "`R` is a source of the public layer already",
    );
}

/// The full chain of the design: a global configuration with a private
/// source, a public repository with rules of its own and a private local
/// source, and its site with the four quadrants.
#[test]
fn site_shows_the_four_quadrants_of_global_and_local_sources() {
    let w = world("site");
    let (me_work, me) = source(
        &w.at,
        "me",
        &[
            domain(1),
            edit(rule(4, 1, ""), "Rule 4\"", "Private rule 4\""),
        ],
    );
    w.configure(&format!(
        "me = {{ git = \"{me}\", branch = \"main\", public = false }}"
    ));
    ok(&w.run(&["init", "--public"]));
    write(&w.repo, "rules/1.toml", &domain(1).1);
    write(&w.repo, "rules/3.toml", &rule(3, 1, "").1);
    write(&w.at, "here/rules/1.toml", &domain(1).1);
    write(&w.at, "here/rules/5.toml", &rule(5, 1, "").1);
    write(
        &w.repo,
        PRIVATE,
        &format!(
            "[sources]\nhere = {{ path = \"{}\", public = false, scope = \"local\" }}\n",
            w.at.join("here").display()
        ),
    );
    write(
        &w.repo,
        ".git/info/exclude",
        "/CLAUDE.local.md\n/AGENTS.override.md\n",
    );
    ok(&w.run(&["add", "R2"]));
    ok(&w.run(&["add", "3"]));
    ok(&w.run(&["add", "--private", "me:4"]));
    ok(&w.run(&["compile"]));
    ok(&w.run(&["site"]));
    let index = read(&w.repo, ".statute/site/index.html");
    for (class, source, title) in [
        ("global-public", "R", "Rule 2"),
        ("global-private", "me", "Private rule 4"),
        ("local-public", "L", "Rule 3"),
        ("local-private", "here", "Rule 5"),
    ] {
        let start = format!("<section class=\"{class}\">");
        let rest = &index[index.find(&start).unwrap()..];
        let section = &rest[..rest.find("</section>").unwrap()];
        assert!(
            section.contains(&format!("id=\"source-{source}\"")),
            "{class}: {section}"
        );
        assert!(section.contains(title), "{class}: {section}");
    }
    // A git source is named by its address and commit, as in the compiled
    // `AGENTS.md`, not by the snapshot directory it is read from; a
    // source on disk by its path.
    let r_commit = git(&w.r, &["rev-parse", "HEAD"]);
    let me_commit = git(&me_work, &["rev-parse", "HEAD"]);
    for (source, place) in [
        ("R", format!("{} at {r_commit}", w.r_address)),
        ("me", format!("{me} at {me_commit}")),
        ("L", ".".to_owned()),
        ("here", w.at.join("here").display().to_string()),
    ] {
        let heading =
            format!("<h3 id=\"source-{source}\">{source} <span class=\"tag\">{place}</span></h3>");
        assert!(index.contains(&heading), "{heading}\n{index}");
    }
    assert!(!index.contains("snapshots"), "{index}");
}

#[test]
fn a_clone_of_the_users_own_is_read_as_its_last_fetch_left_it() {
    let at = base("clone");
    let (work, _) = source(&at, "r", &[domain(1), rule(2, 1, "")]);
    let bare = at.join("r.git");
    git(&work, &["fetch", "-q", "origin"]);
    let fetched = git(&work, &["rev-parse", "origin/main"]);
    // A commit of the work tree that was never pushed is not read.
    write(
        &work,
        "rules/2.toml",
        &edit(rule(2, 1, ""), "Do the thing 2.", "Unpushed.").1,
    );
    git(&work, &["commit", "-q", "-am", "local"]);
    let home = at.join("home");
    write(
        &home,
        ".config/statute/config.toml",
        &format!(
            "[sources]\nR = {{ git = \"{}\", branch = \"main\", clone = \"{}\" }}\n",
            bare.display(),
            work.display()
        ),
    );
    let repo = at.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    ok(&statute(&home, &repo, &["init", "--private"]));
    ok(&statute(&home, &repo, &["add", "R2"]));
    assert!(read(&repo, "statute.lock").contains(&fetched));
    assert!(!home.join(".cache/statute/repos").exists());
    // A recorded commit the clone lacks is fetched by the user, there.
    let lock = read(&repo, "statute.lock").replace(&fetched, &"0".repeat(40));
    std::fs::write(repo.join("statute.lock"), lock).unwrap();
    fails(
        &statute(&home, &repo, &["compile"]),
        &format!(
            "is not in the clone {}; run `git fetch` there",
            work.display()
        ),
    );
    // A clone of another address is refused.
    write(
        &home,
        ".config/statute/config.toml",
        &format!(
            "[sources]\nR = {{ git = \"https://example.test/other.git\", branch = \"main\", clone = \"{}\" }}\n",
            work.display()
        ),
    );
    fails(
        &statute(&home, &repo, &["lock"]),
        "not https://example.test/other.git",
    );
}

/// A repository that is itself the first-party source, `R = { path = "." }`
/// as this one, with a private source of the global configuration: its
/// site lists `R` once, as global public, and three quadrants hold
/// something.
#[test]
fn site_of_the_first_party_source_with_a_global_private_source() {
    let w = world("site-here");
    let (_, me) = source(
        &w.at,
        "me",
        &[
            domain(1),
            edit(rule(4, 1, ""), "Rule 4\"", "Private rule 4\""),
        ],
    );
    w.configure(&format!(
        "me = {{ git = \"{me}\", branch = \"main\", public = false }}"
    ));
    write(&w.repo, "constitution/2026-10/index.toml", INDEX);
    write(
        &w.repo,
        "constitution/2026-10/generated/constitution.en.agent.md",
        "# The constitution\n",
    );
    write(
        &w.repo,
        "constitution/2026-10/articles/04-authority/article.toml",
        "id = \"authority\"\n",
    );
    write(&w.repo, "rules/1.toml", &domain(1).1);
    write(&w.repo, "rules/2.toml", &rule(2, 1, "").1);
    write(
        &w.repo,
        ".git/info/exclude",
        "/CLAUDE.local.md\n/AGENTS.override.md\n",
    );
    ok(&w.run(&["init", "--public", "--source", "."]));
    ok(&w.run(&["add", "R2"]));
    ok(&w.run(&["add", "--private", "me:4"]));
    ok(&w.run(&["compile"]));
    ok(&w.run(&["site"]));
    let index = read(&w.repo, ".statute/site/index.html");
    let section = |class: &str| {
        let start = format!("<section class=\"{class}\">");
        let rest = &index[index.find(&start).unwrap()..];
        rest[..rest.find("</section>").unwrap()].to_owned()
    };
    assert!(section("global-public").contains("id=\"source-R\""));
    assert_eq!(index.matches("id=\"source-R\"").count(), 1, "{index}");
    assert!(section("global-private").contains("Private rule 4"));
    // The local public section points to `R` and lists no source of its own.
    let local = section("local-public");
    assert!(local.contains("<a href=\"#source-R\">R</a>"), "{local}");
    assert!(!local.contains("<h3"), "{local}");
    let held = [
        "global-public",
        "global-private",
        "local-public",
        "local-private",
    ]
    .iter()
    .filter(|c| !section(c).contains("No source."))
    .count();
    assert_eq!(held, 3, "{index}");
}

#[test]
fn source_add_writes_where_its_visibility_says() {
    let w = world("source-add");
    ok(&w.run(&["init", "--public"]));
    ok(&w.run(&["add", "R2"]));
    w.on_disk("team", 7);
    w.on_disk("me", 4);
    w.on_disk("ours", 5);
    // PUBLIC: statute.toml, and the lock file is written again.
    let run = w.run_gh(&["source", "add", "team", "../team"], Some("PUBLIC"));
    ok(&run);
    let manifest = read(&w.repo, "statute.toml");
    assert!(
        manifest.contains("team = { path = \"../team\", public = true }"),
        "{manifest}"
    );
    assert!(read(&w.repo, "statute.lock").contains("name = \"team\""));
    // INTERNAL: private, so the private rule set, made with the source's
    // absolute path.
    let run = w.run_gh(&["source", "add", "me", "../me"], Some("INTERNAL"));
    ok(&run);
    assert!(
        run.stdout.contains("GitHub shows it as INTERNAL"),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout.contains(&format!(
            "`me` is private, so it is named in the private rule set, {}, in git's directory",
            w.repo.join(PRIVATE).display()
        )),
        "{}",
        run.stdout
    );
    let private = read(&w.repo, PRIVATE);
    assert_eq!(
        private,
        format!(
            "[sources]\nme = {{ path = \"{}\", public = false }}\n",
            w.at.join("me").canonicalize().unwrap().display()
        )
    );
    assert!(read(&w.repo, PRIVATE_LOCK).contains("name = \"me\""));
    assert!(!read(&w.repo, "statute.toml").contains("me ="));
    // No answer: private as well.
    let run = w.run_gh(&["source", "add", "ours", "../ours"], None);
    ok(&run);
    assert!(
        run.stdout.contains("GitHub could not tell"),
        "{}",
        run.stdout
    );
    assert!(read(&w.repo, PRIVATE).contains("ours = { path"));
    // A name the file holds, or the public layer has, fails.
    fails(
        &w.run(&["source", "add", "team", "../team", "--public"]),
        "[sources] names `team` already",
    );
    fails(
        &w.run(&["source", "add", "team", "../team", "--private"]),
        "`team` is a source of the public layer already",
    );
}

#[test]
fn source_add_in_a_private_repository_and_globally() {
    let w = world("source-add-private");
    ok(&w.run(&["init", "--private"]));
    w.on_disk("me", 4);
    ok(&w.run(&["source", "add", "me", "../me", "--private"]));
    assert!(read(&w.repo, "statute.toml").contains("me = { path = \"../me\", public = false }"));
    assert!(!w.repo.join(PRIVATE).exists());
    // --global: a git address, its default branch written out.
    let (_, team) = source(&w.at, "team", &[domain(1), rule(7, 1, "")]);
    let run = w.run(&["source", "add", "--global", "--public", "team", &team]);
    ok(&run);
    let config = read(&w.home, ".config/statute/config.toml");
    assert!(
        config.contains(&format!(
            "team = {{ git = \"{team}\", branch = \"main\", public = true }}"
        )),
        "{config}"
    );
    ok(&w.run(&["add", "team:7"]));
    assert!(read(&w.repo, "statute.lock").contains(&format!("git = \"{team}\"")));
}

#[test]
fn a_git_source_with_rules_dir() {
    let w = world("rules-dir");
    ok(&w.run(&["init", "--private"]));
    let (team_work, team) = source(&w.at, "team", &[domain(1), rule(7, 1, "")]);
    git(&team_work, &["mv", "rules", "agent"]);
    git(&team_work, &["commit", "-q", "-m", "move"]);
    git(&team_work, &["push", "-q", "origin", "main"]);
    let add = [
        "source",
        "add",
        "--global",
        "--public",
        "team",
        team.as_str(),
    ];
    fails(&w.run(&add), "has no rules/");
    let config = read(&w.home, ".config/statute/config.toml");
    ok(&w.run(&[&add[..], &["--rules-dir", "agent"]].concat()));
    assert!(
        read(&w.home, ".config/statute/config.toml").contains(&format!(
            "team = {{ git = \"{team}\", branch = \"main\", public = true, rules-dir = \"agent\" }}"
        ))
    );
    ok(&w.run(&["add", "team:7"]));
    assert!(read(&w.repo, "statute.lock").contains("rules-dir = \"agent\""));
    // Named by the lock file alone, it is read from the same directory.
    std::fs::write(w.home.join(".config/statute/config.toml"), config).unwrap();
    ok(&w.run(&["compile"]));
    assert!(read(&w.repo, "AGENTS.md").contains("<a id=\"rule-team-7\"></a>"));
}

#[test]
fn github_addresses() {
    use statute::global::github_repo;
    assert_eq!(
        github_repo("https://github.com/o/r.git").as_deref(),
        Some("o/r")
    );
    assert_eq!(github_repo("git@github.com:o/r").as_deref(), Some("o/r"));
    assert_eq!(github_repo("https://example.test/o/r.git"), None);
    assert_eq!(github_repo("https://github.com/o"), None);
}

#[test]
fn source_add_writes_nothing_when_the_private_layer_would_fail() {
    let w = world("source-add-clash");
    ok(&w.run(&["init", "--public"]));
    ok(&w.run(&["add", "R2"]));
    w.on_disk("team", 7);
    write(
        &w.repo,
        PRIVATE,
        &format!(
            "[sources]\nteam = {{ path = \"{}\", public = false }}\n",
            w.at.join("team").display()
        ),
    );
    ok(&w.run(&["lock"]));
    let before = (read(&w.repo, "statute.toml"), read(&w.repo, "statute.lock"));
    fails(
        &w.run(&["source", "add", "team", "../team", "--public"]),
        "`team` is named in statute.toml already",
    );
    assert_eq!(
        (read(&w.repo, "statute.toml"), read(&w.repo, "statute.lock")),
        before
    );
}

#[test]
fn source_add_takes_a_bare_repository_as_a_git_address() {
    let w = world("source-add-bare");
    ok(&w.run(&["init", "--private"]));
    source(&w.at, "team", &[domain(1), rule(7, 1, "")]);
    let bare = w.at.join("team.git");
    ok(&w.run(&["source", "add", "team", bare.to_str().unwrap(), "--public"]));
    let manifest = read(&w.repo, "statute.toml");
    assert!(
        manifest.contains(&format!(
            "team = {{ git = \"{}\", branch = \"main\", public = true }}",
            bare.display()
        )),
        "{manifest}"
    );
}
