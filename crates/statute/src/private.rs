//! The private layer of a public repository (`docs/design/rule-sources.md`,
//! Private rules in a public repository): `statute.private.toml` adds
//! private sources and rules to `statute.toml`, and compiles, with the
//! public rules, into one file that git does not track, which the agents
//! find through links. Where each of these files goes is decided here
//! alone.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The private layer's rule set and lock file, next to `statute.toml`.
pub const MANIFEST: &str = "statute.private.toml";
pub const LOCK: &str = "statute.private.lock";

/// The directory of the private output, which the public `.gitignore`
/// names, and the full text in it: the public rules and the private ones.
pub const DIR: &str = ".statute";
pub const FULL: &str = "AGENTS.full.md";

/// The links to the full text that the agents read: Claude Code reads
/// `CLAUDE.local.md` and then no `AGENTS.md`, Codex and Pi read
/// `AGENTS.override.md` in place of `AGENTS.md`.
pub const LINKS: [&str; 2] = ["CLAUDE.local.md", "AGENTS.override.md"];

/// The most of a project's instruction files Codex reads by default.
pub const MAX_BYTES: usize = 32 * 1024;

/// The full text, relative to the directory of `statute.toml`.
pub fn full() -> PathBuf {
    Path::new(DIR).join(FULL)
}

/// The line the committed `AGENTS.md` of a public repository ends with,
/// for an agent that finds neither link. The path is in backticks, so
/// that no agent imports it as an `@` path.
pub fn fallback(lang: &str) -> String {
    let path = format!("{DIR}/{FULL}");
    match lang {
        "zh" => format!("如果 `{path}` 存在，读它并照做：它是本文件加上本仓库的私有规定。"),
        _ => format!(
            "If `{path}` exists, read it and follow it: it is this file with the repository's private rules added."
        ),
    }
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
/// cannot tell, as outside a git repository.
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
            let source = text.split(':').next().unwrap_or("");
            if Path::new(source)
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

/// Fails unless git ignores the private layer's rule set and lock file.
/// Asked before either is written.
pub fn check_ignored_files(root: &Path) -> Result<(), String> {
    check_paths(root, &[PathBuf::from(MANIFEST), PathBuf::from(LOCK)])
}

/// Fails unless git ignores each of the private layer's files, the links
/// through a pattern outside every `.gitignore`. Asked before the
/// output is written.
pub fn check_ignored(root: &Path) -> Result<(), String> {
    check_paths(
        root,
        &[PathBuf::from(MANIFEST), PathBuf::from(LOCK), full()],
    )?;
    check_links(root)
}

fn check_paths(root: &Path, files: &[PathBuf]) -> Result<(), String> {
    for file in files {
        if ignored(root, file)? == Ignored::No {
            return Err(format!(
                "{}: git does not ignore it, so nothing private is written; add `/{}` to the repository's .gitignore",
                file.display(),
                file.iter()
                    .next()
                    .map_or(String::new(), |c| c.to_string_lossy().into_owned())
            ));
        }
    }
    Ok(())
}

fn check_links(root: &Path) -> Result<(), String> {
    // The lines of `.git/info/exclude` are relative to the top of the
    // work tree.
    let prefix = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--show-prefix"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_default();
    let mut missing = Vec::new();
    for link in LINKS {
        match ignored(root, Path::new(link))? {
            Ignored::Elsewhere => {}
            Ignored::No | Ignored::ByGitignore => missing.push(format!("/{prefix}{link}")),
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "{}: git must ignore these links through .git/info/exclude, not a .gitignore (Grok skips an instruction file a .gitignore matches), so nothing private is written; add these lines to .git/info/exclude: {}",
            LINKS.join(", "),
            missing.join(", ")
        ));
    }
    Ok(())
}

/// Writes the full text and the links to it, next to `statute.toml` in
/// `root`, after checking that git ignores them all; writes nothing when
/// it does not, or when a file at a link's path is not that link. Links
/// need a Unix system. Returns what was written.
pub fn write(root: &Path, text: &str) -> Result<Vec<PathBuf>, String> {
    check_ignored(root)?;
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

/// What to say about a `CLAUDE.md` in `dir`, a file or a link to
/// `AGENTS.md`, if there is one; `private` when the private layer is
/// compiled too, so that `CLAUDE.local.md` takes the place of `AGENTS.md`.
pub fn claude_md(dir: &Path, private: bool) -> Option<String> {
    dir.join("CLAUDE.md").symlink_metadata().ok()?;
    let twice = if private {
        " Here CLAUDE.local.md gives it the full text, public and private, in place of AGENTS.md, and it reads CLAUDE.md as well, so that its content comes on top of the rules (for a link to AGENTS.md, the public rules twice)."
    } else {
        ""
    };
    Some(format!(
        "Claude Code now reads AGENTS.md itself, so CLAUDE.md is not needed to point to it.{twice} Consider deleting it; statute neither deletes nor renames it (a migration will rename it to a legacy copy, after asking)."
    ))
}
