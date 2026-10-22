//! The git hooks that write the private output of each worktree
//! (`docs/design/rule-sources.md`, A new worktree), which `statute hooks
//! install` writes and no other command touches.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The second line of a hook statute wrote, by which it knows the file
/// as its own.
pub const MARKER: &str = "# written by statute";

/// One hook: its name in git's hooks directory and the line it runs,
/// which is also the line to add to a hook statute did not write.
pub struct Hook {
    pub name: &'static str,
    pub line: &'static str,
}

/// `post-checkout` runs on a checkout of a branch only (its third
/// argument is 1), `post-merge` after every merge; neither fails git.
pub const HOOKS: [Hook; 2] = [
    Hook {
        name: "post-checkout",
        line: "[ \"$3\" = 1 ] && statute compile --private-only || true",
    },
    Hook {
        name: "post-merge",
        line: "statute compile --private-only || true",
    },
];

impl Hook {
    /// The whole file statute writes.
    pub fn text(&self) -> String {
        format!("#!/bin/sh\n{MARKER}\n{}\n", self.line)
    }
}

/// What `install` found at a hook's path.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// No hook, or an older one of statute's: written.
    Written,
    /// statute's hook as it is now: left as it is.
    Unchanged,
    /// A hook statute did not write: left as it is.
    Foreign,
}

/// The paths of `HOOKS` for the work tree at `root`, as `git rev-parse
/// --git-path hooks/<name>` prints them, following `core.hooksPath`; an
/// error outside a git repository, and when `root` is not the top of a
/// work tree, where git runs the hooks.
pub fn paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(root).args([
        "rev-parse",
        "--path-format=absolute",
        "--is-inside-work-tree",
        "--show-prefix",
    ]);
    for hook in &HOOKS {
        cmd.args(["--git-path", &format!("hooks/{}", hook.name)]);
    }
    let out = cmd
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| format!("cannot run git, which says where the hooks go: {e}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        return Err(if stderr.starts_with("fatal: not a git repository") {
            "not in a git repository, so there are no hooks to install".to_owned()
        } else {
            format!("git cannot tell where the hooks go: {}", stderr.trim())
        });
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    let [inside, prefix, hooks @ ..] = &lines[..] else {
        return Err(format!(
            "git cannot tell where the hooks go: `git rev-parse` printed {stdout:?}"
        ));
    };
    if hooks.len() != HOOKS.len() {
        return Err(format!(
            "git cannot tell where the hooks go: `git rev-parse` printed {stdout:?}"
        ));
    }
    if *inside != "true" || !prefix.is_empty() {
        return Err(
            "not at the top of a work tree: git runs the hooks there, so statute.toml must be there"
                .to_owned(),
        );
    }
    Ok(hooks.iter().map(PathBuf::from).collect())
}

/// Writes `hook` at `path`, executable, unless a hook is there that
/// statute did not write, or statute's as it is now; makes the
/// directory when it is missing.
pub fn install(path: &Path, hook: &Hook) -> std::io::Result<Outcome> {
    let text = hook.text();
    match std::fs::read(path) {
        Ok(old) => {
            let ours = std::str::from_utf8(&old).is_ok_and(|old| {
                old.starts_with("#!/bin/sh\n") && old.lines().nth(1) == Some(MARKER)
            });
            if !ours {
                return Ok(Outcome::Foreign);
            }
            if old == text.as_bytes() && executable(path)? {
                return Ok(Outcome::Unchanged);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    crate::private::replace(path, &text)?;
    make_executable(path)?;
    Ok(Outcome::Written)
}

#[cfg(unix)]
fn executable(path: &Path) -> std::io::Result<bool> {
    use std::os::unix::fs::PermissionsExt;
    Ok(std::fs::metadata(path)?.permissions().mode() & 0o111 == 0o111)
}

#[cfg(not(unix))]
fn executable(_: &Path) -> std::io::Result<bool> {
    Ok(true)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn make_executable(_: &Path) -> std::io::Result<()> {
    Ok(())
}
