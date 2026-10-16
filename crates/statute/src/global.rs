//! statute's own directories, after the XDG Base Directory
//! Specification on Linux and macOS alike: the global configuration in
//! `$XDG_CONFIG_HOME/statute/` (`~/.config/statute/` by default), and a
//! bare clone of each git source and the files of each commit read in
//! `$XDG_CACHE_HOME/statute/` (`~/.cache/statute/` by default)
//! (`docs/design/global-configuration.md`).

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::manifest::{Follow, Git, SourceSpec};

/// The global configuration, in the configuration directory.
pub const CONFIG: &str = "config.toml";

/// The address of the first-party source, which `R` is by default.
pub const FIRST_PARTY_ADDRESS: &str = "https://github.com/Luolc/agent-constitution";

/// The branch of the first-party source that `R` follows by default.
pub const FIRST_PARTY_BRANCH: &str = "main";

/// `statute` under the directory `var` names, or under `default` in the
/// home directory when `var` is unset, empty or relative, which the
/// specification says to ignore; `None` when the system reports no home
/// directory.
fn xdg(var: &str, default: &str) -> Option<PathBuf> {
    let base = match std::env::var_os(var).map(PathBuf::from) {
        Some(path) if path.is_absolute() => path,
        _ => std::env::home_dir()?.join(default),
    };
    Some(base.join("statute"))
}

/// The configuration directory, `$XDG_CONFIG_HOME/statute/`.
pub fn config_dir() -> Option<PathBuf> {
    xdg("XDG_CONFIG_HOME", ".config")
}

/// The cache directory, `$XDG_CACHE_HOME/statute/`, of the clones and
/// the snapshots.
pub fn cache_dir() -> Option<PathBuf> {
    xdg("XDG_CACHE_HOME", ".cache")
}

/// The global configuration's path, as messages name it: `~` for the
/// home directory.
pub fn shown_config() -> String {
    let Some(path) = config_dir().map(|dir| dir.join(CONFIG)) else {
        return format!("~/.config/statute/{CONFIG}");
    };
    match std::env::home_dir().and_then(|home| path.strip_prefix(home).ok().map(Path::to_owned)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// The text `statute init --global` writes, with the first-party source
/// at `address`.
pub fn initial_config(address: &str) -> String {
    let address = toml_edit::Value::from(address).to_string();
    format!(
        "# statute's global configuration: the sources every rule set may use\n\
         # by name. A rule set's own [sources] overrides an entry by name, and\n\
         # `name = false` there takes one out.\n\
         \n\
         [sources]\n\
         R = {{ git = {address}, branch = \"{FIRST_PARTY_BRANCH}\" }}\n\
         \n\
         # A private source of your own, used by every repository:\n\
         # me = {{ git = \"git@example.test:me/rules.git\", branch = \"main\", public = false }}\n"
    )
}

/// `~` at the start of `path` as the home directory.
pub fn expand(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::home_dir()) {
        (Some(rest), Some(home)) => home.join(rest),
        _ if path == "~" => std::env::home_dir().unwrap_or_else(|| PathBuf::from(path)),
        _ => PathBuf::from(path),
    }
}

/// `git` with the variables of an outer repository removed, so that a
/// hook or a test run inside one does not point it elsewhere.
fn git() -> Command {
    let mut cmd = Command::new("git");
    crate::check::without_git_locals(&mut cmd);
    cmd
}

/// Runs `cmd`, its standard output trimmed; else what git said.
fn run(mut cmd: Command, what: &str) -> Result<String, String> {
    let out = cmd
        .output()
        .map_err(|e| format!("cannot run git to {what}: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    } else {
        Err(format!(
            "git failed to {what}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// The directory of the bare clone of `address` under `dir`:
/// `repos/<name>-<hash>`, the last part of the address and the first 12
/// hex digits of its SHA-256.
pub fn clone_dir(dir: &Path, address: &str) -> PathBuf {
    let name = address
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .unwrap_or("")
        .trim_end_matches(".git");
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let hash = Sha256::digest(address.as_bytes());
    let hex: String = hash.iter().take(6).map(|b| format!("{b:02x}")).collect();
    dir.join("repos").join(format!("{name}-{hex}"))
}

/// Where the commits of `git` are read: the user's own clone, or the bare
/// clone under `dir`, made when it is missing.
fn repository(dir: &Path, git: &Git) -> Result<(PathBuf, bool), String> {
    if let Some(clone) = &git.clone {
        let clone = expand(clone);
        let mut cmd = self::git();
        cmd.arg("-C")
            .arg(&clone)
            .args(["remote", "get-url", "origin"]);
        let origin = run(cmd, &format!("read the origin of {}", clone.display()))?;
        if origin != git.address {
            return Err(format!(
                "the clone {} has origin {origin}, not {}",
                clone.display(),
                git.address
            ));
        }
        return Ok((clone, false));
    }
    let target = clone_dir(dir, &git.address);
    if !target.exists() {
        let repos = dir.join("repos");
        std::fs::create_dir_all(&repos)
            .map_err(|e| format!("cannot create {}: {e}", repos.display()))?;
        let temp = repos.join(format!(
            ".clone-{}-{}",
            std::process::id(),
            target.file_name().unwrap_or_default().to_string_lossy()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        let mut cmd = self::git();
        // `--`: an address is never read as an option.
        cmd.args(["clone", "--bare", "--quiet", "--", &git.address])
            .arg(&temp);
        if let Err(e) = run(cmd, &format!("clone {}", git.address)) {
            let _ = std::fs::remove_dir_all(&temp);
            return Err(e);
        }
        if let Err(e) = std::fs::rename(&temp, &target) {
            let _ = std::fs::remove_dir_all(&temp);
            if !target.exists() {
                return Err(format!(
                    "cannot move the clone to {}: {e}",
                    target.display()
                ));
            }
        }
    }
    Ok((target, true))
}

/// Fetches the branches and tags of `address` into the bare clone at
/// `repo`.
fn fetch(repo: &Path, address: &str) -> Result<(), String> {
    let mut cmd = git();
    cmd.arg("-C").arg(repo).args([
        "fetch",
        "--quiet",
        "--force",
        "--",
        address,
        "+refs/heads/*:refs/heads/*",
        "+refs/tags/*:refs/tags/*",
    ]);
    run(cmd, &format!("fetch {address}")).map(drop)
}

/// The commit `follow` names in `repo`: in a bare clone of statute's, the
/// branch or tag itself; in the user's clone, the remote branch as the
/// user's last fetch left it, or the tag.
fn head(repo: &Path, follow: &Follow, own: bool) -> Result<String, String> {
    let name = match (follow, own) {
        (Follow::Branch(b), true) => format!("refs/heads/{b}^{{commit}}"),
        (Follow::Branch(b), false) => format!("refs/remotes/origin/{b}^{{commit}}"),
        (Follow::Tag(t), _) => format!("refs/tags/{t}^{{commit}}"),
    };
    let mut cmd = git();
    cmd.arg("-C")
        .arg(repo)
        .args(["rev-parse", "--verify", "--quiet", &name]);
    run(cmd, "").map_err(|_| format!("{} has no {follow}", repo.display()))
}

/// Whether `repo` holds `commit`.
fn has(repo: &Path, commit: &str) -> bool {
    let mut cmd = git();
    cmd.arg("-C")
        .arg(repo)
        .args(["cat-file", "-e", &format!("{commit}^{{commit}}")]);
    run(cmd, "").is_ok()
}

/// The files of `commit` of `repo`, written once into
/// `snapshots/<commit>/` under `dir` through a temporary index, so that
/// no work tree, `HEAD` or index of `repo` changes.
fn snapshot(dir: &Path, repo: &Path, commit: &str) -> Result<PathBuf, String> {
    let target = dir.join("snapshots").join(commit);
    if target.is_dir() {
        return Ok(target);
    }
    let snapshots = dir.join("snapshots");
    let temp = snapshots.join(format!(".tmp-{}-{commit}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    let tree = temp.join("tree");
    std::fs::create_dir_all(&tree).map_err(|e| format!("cannot create {}: {e}", tree.display()))?;
    let index = temp.join("index");
    for args in [
        vec!["read-tree".to_owned(), commit.to_owned()],
        vec!["checkout-index".to_owned(), "--all".to_owned()],
    ] {
        let mut cmd = git();
        cmd.arg("-C")
            .arg(repo)
            .arg("--work-tree")
            .arg(&tree)
            .args(&args)
            .env("GIT_INDEX_FILE", &index);
        if let Err(e) = run(cmd, &format!("write the files of {commit}")) {
            let _ = std::fs::remove_dir_all(&temp);
            return Err(e);
        }
    }
    let moved = std::fs::rename(&tree, &target);
    let _ = std::fs::remove_dir_all(&temp);
    match moved {
        Ok(()) => Ok(target),
        // Another run wrote the same commit first.
        Err(_) if target.is_dir() => Ok(target),
        Err(e) => Err(format!("cannot write {}: {e}", target.display())),
    }
}

/// Makes the git source `spec` readable: picks its commit (the one
/// `locked` names unless `newest`, else the head of what it follows,
/// after a fetch), writes that commit's files, and sets the spec's
/// `path` to them and its `commit`.
pub fn materialize(
    spec: &mut SourceSpec,
    locked: Option<&str>,
    newest: bool,
) -> Result<(), String> {
    let Some(git) = spec.git.as_mut() else {
        return Ok(());
    };
    let (name, address) = (spec.name.clone(), git.address.clone());
    let err = |e: String| format!("source `{name}` ({address}): {e}");
    let dir = cache_dir().ok_or_else(|| err("the system reports no home directory".to_owned()))?;
    let (repo, own) = repository(&dir, git).map_err(&err)?;
    let commit = match locked {
        Some(commit) if !newest => {
            if own && !has(&repo, commit) {
                fetch(&repo, &git.address).map_err(&err)?;
            }
            if !has(&repo, commit) {
                return Err(err(if own {
                    format!(
                        "commit {commit}, which the lock file records, is not in {}",
                        git.address
                    )
                } else {
                    format!(
                        "commit {commit}, which the lock file records, is not in the clone {}; run `git fetch` there",
                        repo.display()
                    )
                }));
            }
            commit.to_owned()
        }
        // No commit to keep: the newest, fetched first, as for a new
        // source or `upgrade`.
        _ => {
            if own {
                fetch(&repo, &git.address).map_err(&err)?;
            }
            head(&repo, &git.follow, own).map_err(&err)?
        }
    };
    let files = snapshot(&dir, &repo, &commit).map_err(&err)?;
    spec.path = files.to_string_lossy().into_owned();
    git.commit = Some(commit);
    Ok(())
}

/// As `materialize` at `locked`, without the network: the commit's files
/// are read from the cache, or written there from a clone that holds the
/// commit already; nothing is cloned or fetched.
pub fn cached(spec: &mut SourceSpec, locked: &str) -> Result<(), String> {
    let Some(git) = spec.git.as_mut() else {
        return Ok(());
    };
    let (name, address) = (spec.name.clone(), git.address.clone());
    let err = |e: String| format!("source `{name}` ({address}): {e}");
    let dir = cache_dir().ok_or_else(|| err("the system reports no home directory".to_owned()))?;
    let target = dir.join("snapshots").join(locked);
    let files = if target.is_dir() {
        target
    } else {
        let repo = match &git.clone {
            Some(clone) => expand(clone),
            None => clone_dir(&dir, &git.address),
        };
        if !repo.is_dir() || !has(&repo, locked) {
            return Err(err(format!(
                "commit {locked}, which the lock file records, is not in statute's cache, and nothing is fetched"
            )));
        }
        snapshot(&dir, &repo, locked).map_err(&err)?
    };
    spec.path = files.to_string_lossy().into_owned();
    git.commit = Some(locked.to_owned());
    Ok(())
}

/// As `cached`, for a git source no lock file records, as one of the
/// global configuration's private sources is where there is no private
/// lock file: at the commit that what it follows names in the clone
/// statute reads, without the network.
pub fn cached_newest(spec: &mut SourceSpec) -> Result<(), String> {
    let Some(git) = &spec.git else {
        return Ok(());
    };
    let err = |e: String| format!("source `{}` ({}): {e}", spec.name, git.address);
    let dir = cache_dir().ok_or_else(|| err("the system reports no home directory".to_owned()))?;
    let (repo, own) = match &git.clone {
        Some(clone) => (expand(clone), false),
        None => (clone_dir(&dir, &git.address), true),
    };
    let commit = head(&repo, &git.follow, own).map_err(|e| {
        err(format!(
            "no lock file records a commit of it, and nothing is fetched: {e}"
        ))
    })?;
    cached(spec, &commit)
}

/// The branch the remote's `HEAD` names in the bare clone of `address`,
/// cloned when missing.
pub fn default_branch(address: &str) -> Result<String, String> {
    let dir = cache_dir().ok_or("the system reports no home directory")?;
    let git = Git {
        address: address.to_owned(),
        follow: Follow::Branch(String::new()),
        clone: None,
        commit: None,
    };
    let (repo, _) = repository(&dir, &git)?;
    let mut cmd = self::git();
    cmd.arg("-C")
        .arg(&repo)
        .args(["symbolic-ref", "--short", "HEAD"]);
    run(cmd, &format!("read the default branch of {address}"))
}

/// The GitHub repository, `owner/name`, that `address` names; `None` for
/// an address GitHub does not hold.
pub fn github_repo(address: &str) -> Option<String> {
    let rest = [
        "https://github.com/",
        "http://github.com/",
        "ssh://git@github.com/",
        "git@github.com:",
    ]
    .iter()
    .find_map(|prefix| address.strip_prefix(prefix))?;
    let repo = rest.trim_end_matches('/').trim_end_matches(".git");
    let (owner, name) = repo.split_once('/')?;
    (!owner.is_empty() && !name.is_empty() && !name.contains('/')).then(|| repo.to_owned())
}

/// The address of the `origin` of the clone at `dir`, and the branch its
/// `HEAD` names there, as the last fetch left it.
pub fn clone_origin(dir: &Path) -> Result<(String, Option<String>), String> {
    let mut cmd = git();
    cmd.arg("-C").arg(dir).args(["remote", "get-url", "origin"]);
    let origin = run(cmd, &format!("read the origin of {}", dir.display()))?;
    let mut cmd = git();
    cmd.arg("-C")
        .arg(dir)
        .args(["symbolic-ref", "--short", "refs/remotes/origin/HEAD"]);
    let branch = run(cmd, "")
        .ok()
        .and_then(|b| b.strip_prefix("origin/").map(str::to_owned));
    Ok((origin, branch))
}

/// Whether `dir` is a bare git repository, whose path is a git address
/// rather than a source on disk.
pub fn is_bare(dir: &Path) -> bool {
    let mut cmd = git();
    cmd.arg("-C")
        .arg(dir)
        .args(["rev-parse", "--is-bare-repository"]);
    run(cmd, "").is_ok_and(|answer| answer == "true")
}
