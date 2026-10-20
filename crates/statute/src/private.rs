//! The private layer of a public repository (`docs/design/rule-sources.md`,
//! Private rules in a public repository): the private rule set,
//! `private.toml` in git's own directory, adds private sources and rules
//! to `statute.toml`, and compiles, with the public rules, into one file
//! that git does not track, which the agents find through links. Where
//! each of these files goes is decided here alone.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The private layer's rule set and lock file.
pub const MANIFEST: &str = "private.toml";
pub const LOCK: &str = "private.lock";

/// Where the private rule set lives, as `git rev-parse --git-path` takes
/// it: below `common/`, which git shares between the worktrees of a clone.
const GIT_PATH: &str = "common/statute";

/// Where a linked worktree keeps its own lock file, as `git rev-parse
/// --git-path` takes it: in the worktree's own git directory.
const WORKTREE_GIT_PATH: &str = "statute";

/// The directory of the private output and the full text in it: the
/// public rules and the private ones.
pub const DIR: &str = ".statute";
pub const FULL: &str = "AGENTS.full.md";

/// The `.gitignore` statute writes in `DIR`: its first line marks it as
/// statute's, and the others name the generated files, so that git
/// tracks the rest of the directory.
pub const GITIGNORE: &str = "# written by statute\n/AGENTS.full.md\n/site/\n";

/// The links to the full text that the agents read: Claude Code reads
/// `CLAUDE.local.md` and then no `AGENTS.md`, Codex and Pi read
/// `AGENTS.override.md` in place of `AGENTS.md`.
pub const LINKS: [&str; 2] = ["CLAUDE.local.md", "AGENTS.override.md"];

/// Where the private layer of one work tree is kept: the rule set, which
/// every worktree of a clone shares, and the work tree's own lock file,
/// so that locking in one worktree leaves the others' commits as they
/// were. The main worktree's lock file sits next to the rule set.
#[derive(Debug, Clone)]
pub struct Dir {
    shared: PathBuf,
    lock: PathBuf,
}

impl Dir {
    /// The private rule set, the same for every worktree.
    pub fn manifest(&self) -> PathBuf {
        self.shared.join(MANIFEST)
    }

    /// The lock file this work tree writes.
    pub fn lock(&self) -> &Path {
        &self.lock
    }

    /// The lock file this work tree reads: its own, or, in a linked
    /// worktree that has not locked yet, the main worktree's.
    pub fn read_lock(&self) -> PathBuf {
        if self.lock.exists() {
            self.lock.clone()
        } else {
            self.shared.join(LOCK)
        }
    }
}

/// Where the private layer of the work tree at `root` is kept: `None`
/// outside a git repository, where there is no private layer; an error
/// when git cannot tell.
pub fn dir(root: &Path) -> Result<Option<Dir>, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
            "--git-path",
            GIT_PATH,
            "--git-path",
            WORKTREE_GIT_PATH,
        ])
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| format!("cannot run git, which says where the private rule set is: {e}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        return if stderr.starts_with("fatal: not a git repository") {
            Ok(None)
        } else {
            Err(format!(
                "git cannot tell where the private rule set is: {}",
                stderr.trim()
            ))
        };
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    let [git_dir, common_dir, shared, own] = lines[..] else {
        return Err(format!(
            "git cannot tell where the private rule set is: `git rev-parse` printed {stdout:?}"
        ));
    };
    let shared = PathBuf::from(shared);
    let lock = if git_dir == common_dir {
        shared.join(LOCK)
    } else {
        Path::new(own).join(LOCK)
    };
    Ok(Some(Dir { shared, lock }))
}

/// What a command that needs the private rule set says outside a git
/// repository.
pub const NO_REPOSITORY: &str = "not in a git repository, so there is no private layer: its rule set, private.toml, lives in git's directory";

/// Writes `text` to `path` through a temporary file in the same
/// directory, made first, so that the file is never seen half written.
pub fn replace(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = staged(path);
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, path))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
}

/// Where a changed `path` is written before it is renamed into place.
pub fn staged(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_owned();
    name.push(".tmp");
    path.with_file_name(name)
}

/// The full text, relative to the directory of `statute.toml`.
pub fn full() -> PathBuf {
    Path::new(DIR).join(FULL)
}

/// Whether the full text is on disk in `root`: a compile with nothing
/// left to call for the private output then rewrites it to say what
/// `AGENTS.md` says, instead of leaving what it said before.
pub fn written(root: &Path) -> bool {
    root.join(full()).symlink_metadata().is_ok()
}

/// The paragraph above `AGENTS.md` in a full text that `withdraw`
/// writes.
pub const WITHDRAWN: &str = "> statute could not write the private output of this work tree, so this file leaves out the private rules and says only what `AGENTS.md` says. `statute compile --private-only` names the cause; `statute compile` writes the private output, or says what to fix.\n\n";

/// After a compile that did not write the private output, rewrites a
/// full text already on disk in `root` to `WITHDRAWN` and `public`, the
/// `AGENTS.md` there, so that the links lead to no private rule that may
/// no longer hold. The text is public, so it is written whether git
/// ignores it or not; nothing is written where there is no full text,
/// or where it is the repository's own, tracked by git or beside a
/// `.statute/.gitignore` that is not statute's, and no link is touched.
/// Returns whether it wrote.
pub fn withdraw(root: &Path, public: &str) -> Result<bool, String> {
    if !written(root) || !gitignore_is_statutes(root) || tracked(root, &full())? {
        return Ok(false);
    }
    let file = root.join(full());
    replace(&file, &format!("{WITHDRAWN}{public}"))
        .map(|()| true)
        .map_err(|e| format!("{}: cannot write: {e}", file.display()))
}

/// Whether `.statute/.gitignore` in `root` starts with statute's line.
fn gitignore_is_statutes(root: &Path) -> bool {
    let head = GITIGNORE.lines().next().unwrap_or_default();
    std::fs::read_to_string(root.join(DIR).join(".gitignore"))
        .is_ok_and(|text| text.lines().next() == Some(head))
}

/// Whether git tracks `path`, relative to `root`; an error when git
/// cannot tell.
fn tracked(root: &Path, path: &Path) -> Result<bool, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "--"])
        .arg(path)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git cannot tell whether it tracks {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(!out.stdout.is_empty())
}

/// Writes `.statute/.gitignore` in `root` when it is missing or is
/// statute's and differs from `GITIGNORE`; fails, writing nothing, when
/// the file there is not statute's. Called before any generated file is
/// written into `.statute/`.
pub fn write_gitignore(root: &Path) -> Result<(), String> {
    let path = root.join(DIR).join(".gitignore");
    let shown = Path::new(DIR).join(".gitignore");
    let head = GITIGNORE.lines().next().unwrap_or_default();
    match std::fs::read_to_string(&path) {
        Ok(text) if text == GITIGNORE => return Ok(()),
        Ok(text) if text.lines().next() == Some(head) => {}
        Ok(_) => {
            return Err(format!(
                "{}: its first line is not `{head}`, so it is the repository's own and is left as it is, and nothing is written into {DIR}; move it away, and statute writes its own",
                shown.display()
            ));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("{}: cannot read: {e}", shown.display())),
    }
    std::fs::create_dir_all(root.join(DIR))
        .and_then(|()| std::fs::write(&path, GITIGNORE))
        .map_err(|e| format!("{}: cannot write: {e}", shown.display()))
}

/// How git ignores a path: not at all, through a `.gitignore`, or
/// through a pattern outside every `.gitignore`, such as
/// `.git/info/exclude`.
#[derive(Debug, PartialEq, Eq)]
pub enum Ignored {
    No,
    ByGitignore,
    Elsewhere,
}

/// Asks git how it ignores `path`, relative to `root`; an error when git
/// cannot tell, as outside a git repository. A path whose last matching
/// pattern is a negation, `!`, is not ignored, though git names that
/// pattern and exits 0 for it.
pub fn ignored(root: &Path, path: &Path) -> Result<Ignored, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["check-ignore", "--verbose", "--no-index", "--"])
        .arg(path)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    match out.status.code() {
        Some(0) => {
            // `<source>:<line>:<pattern>\t<path>`.
            let text = String::from_utf8_lossy(&out.stdout);
            let match_ = text.split('\t').next().unwrap_or("");
            let mut fields = match_.splitn(3, ':');
            let source = fields.next().unwrap_or("");
            if fields
                .nth(1)
                .is_some_and(|pattern| pattern.starts_with('!'))
            {
                Ok(Ignored::No)
            } else if Path::new(source)
                .file_name()
                .is_some_and(|n| n == ".gitignore")
            {
                Ok(Ignored::ByGitignore)
            } else {
                Ok(Ignored::Elsewhere)
            }
        }
        Some(1) => Ok(Ignored::No),
        _ => Err(format!(
            "git cannot tell whether it ignores {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        )),
    }
}

/// Writes `.statute/.gitignore` and checks that git ignores the private
/// output: asked before any file of a compile is written.
pub fn ready(root: &Path) -> Result<(), String> {
    write_gitignore(root)?;
    check_ignored(root)
}

/// Fails unless git ignores the full text, and the links through a
/// pattern outside every `.gitignore`.
fn check_ignored(root: &Path) -> Result<(), String> {
    let file = full();
    if ignored(root, &file)? == Ignored::No {
        return Err(format!(
            "{}: git does not ignore it, so nothing private is written; {DIR}/.gitignore names it, so a pattern elsewhere takes it out, which `git check-ignore --verbose --no-index {}` shows",
            file.display(),
            file.display()
        ));
    }
    check_links(root)
}

fn check_links(root: &Path) -> Result<(), String> {
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
            .unwrap_or_default()
    };
    // The lines of the exclude file are relative to the top of the work
    // tree; in a worktree, the file is the clone's.
    let prefix = git(&["rev-parse", "--show-prefix"]);
    let exclude = git(&["rev-parse", "--git-path", "info/exclude"]);
    let mut missing = Vec::new();
    for link in LINKS {
        match ignored(root, Path::new(link))? {
            Ignored::Elsewhere => {}
            Ignored::No | Ignored::ByGitignore => missing.push(format!("/{prefix}{link}")),
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "{}: git must ignore these links through {exclude}, not a .gitignore (Grok skips an instruction file a .gitignore matches), so nothing private is written; add these lines to {exclude}: {}",
            LINKS.join(", "),
            missing.join(", ")
        ));
    }
    Ok(())
}

/// Writes the full text and the links to it, next to `statute.toml` in
/// `root`, after `ready`; writes nothing when git does not ignore them,
/// or when a file at a link's path is not that link. Links
/// need a Unix system. Returns what was written.
pub fn write(root: &Path, text: &str) -> Result<Vec<PathBuf>, String> {
    ready(root)?;
    for link in LINKS {
        let path = root.join(link);
        if path.symlink_metadata().is_ok() && std::fs::read_link(&path).ok() != Some(full()) {
            return Err(format!(
                "{link}: not the link statute makes to {}, so it is left as it is and nothing private is written; move it away first",
                full().display()
            ));
        }
    }
    if !cfg!(unix) {
        return Err(format!(
            "the private output needs links, {}, which statute makes only on a Unix system",
            LINKS.join(" and ")
        ));
    }
    let fail = |path: &Path, e: std::io::Error| format!("{}: cannot write: {e}", path.display());
    let dir = root.join(DIR);
    std::fs::create_dir_all(&dir).map_err(|e| fail(&dir, e))?;
    let file = root.join(full());
    std::fs::write(&file, text).map_err(|e| fail(&file, e))?;
    let mut written = vec![file];
    for link in LINKS {
        let path = root.join(link);
        if path.symlink_metadata().is_err() {
            symlink(&full(), &path).map_err(|e| fail(&path, e))?;
        }
        written.push(path);
    }
    Ok(written)
}

#[cfg(unix)]
fn symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn symlink(_: &Path, _: &Path) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "links are made only on a Unix system",
    ))
}

/// The `AGENTS.md` files git tracks below `root`, other than the one at
/// `root`: Claude Code does not load them while `CLAUDE.local.md` is
/// there.
pub fn nested_agents_md(root: &Path) -> Vec<PathBuf> {
    let Ok(out) = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "--", ":(glob)**/AGENTS.md"])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty() && *p != "AGENTS.md")
        .map(PathBuf::from)
        .collect()
}

/// What to say about each of `nested_agents_md`.
pub const NESTED: &str = "Claude Code does not load an AGENTS.md below the top of the repository while CLAUDE.local.md is there; move its rules into statute's rules";

/// What to say about a `CLAUDE.md` in `dir`, a file or a link to
/// `AGENTS.md`, if there is one; `private` when the private layer is
/// compiled too, so that `CLAUDE.local.md` takes the place of `AGENTS.md`.
pub fn claude_md(dir: &Path, private: bool) -> Option<String> {
    dir.join("CLAUDE.md").symlink_metadata().ok()?;
    let twice = if private {
        " Here CLAUDE.local.md gives it the full text, public and private, in place of AGENTS.md, and it reads CLAUDE.md as well (for a link to AGENTS.md, the public rules twice)."
    } else {
        ""
    };
    Some(format!(
        "Claude Code now reads AGENTS.md itself, so CLAUDE.md is not needed to point to it.{twice} Consider deleting it; statute neither deletes nor renames it (a migration will rename it to a legacy copy, after asking)."
    ))
}
