//! Running the checks of the enabled rules (`docs/design/rule-checks.md`,
//! Checks, and Outcomes and the review loop). A check has three outcomes;
//! a rule without an automatic check goes to the reviewer as it is.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::manifest::{FIRST_PARTY, Key};
use crate::node::Check;
use crate::project::Project;

mod command;
mod github;
mod r3;
mod r43;
mod r46;
mod r50;
mod r58;
mod r61;
mod r7;
mod r71;
mod yaml;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Violation,
    /// The check could not decide: a missing tool, an error, a timeout, or
    /// a part of the rule the check does not cover.
    Inconclusive,
    /// The rule has no automatic check (`check = "review"`).
    Review,
}

impl Outcome {
    pub fn name(self) -> &'static str {
        match self {
            Outcome::Pass => "pass",
            Outcome::Violation => "violation",
            Outcome::Inconclusive => "inconclusive",
            Outcome::Review => "review",
        }
    }
}

/// One place where a rule is broken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub location: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub outcome: Outcome,
    pub findings: Vec<Finding>,
    /// What the check did or could not do, in a sentence.
    pub note: String,
}

impl Report {
    fn new(outcome: Outcome, note: impl Into<String>) -> Self {
        Report {
            outcome,
            findings: Vec::new(),
            note: note.into(),
        }
    }
}

/// What the checks look at: the repository of `statute.toml`, and the
/// change to check, `git diff <base>...HEAD`, when a base is given.
pub struct Context<'a> {
    pub root: &'a Path,
    pub base: Option<&'a str>,
}

impl Context<'_> {
    /// The top of the git work tree that holds `root`, where a
    /// repository's own configuration lives; `root` itself when git
    /// cannot say.
    pub fn top(&self) -> PathBuf {
        git(self.root)
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| PathBuf::from(String::from_utf8_lossy(&out.stdout).trim_end_matches('\n')))
            .unwrap_or_else(|| self.root.to_path_buf())
    }
}

/// The checks of every enabled rule that is not overridden, in order of
/// key. A built-in check
/// belongs to a rule of the first-party source; a rule of another source
/// with `check = "builtin"` has none in statute.
pub fn run(project: &Project, ctx: &Context) -> Vec<(Key, Report)> {
    project
        .resolution
        .enabled
        .keys()
        .filter(|key| project.resolution.in_force(key))
        .filter_map(|key| {
            let rule = project.sources.get(key)?.rule.as_ref()?;
            let first_party = key.source == FIRST_PARTY;
            let report = match &rule.check {
                Check::Review => Report::new(Outcome::Review, "no automatic check"),
                Check::Builtin => match key.id {
                    3 if first_party => r3::check(ctx),
                    7 if first_party => r7::check(ctx),
                    43 if first_party => r43::check(ctx),
                    46 if first_party => r46::check(ctx),
                    50 if first_party => r50::check(ctx),
                    58 if first_party => r58::check(ctx),
                    61 if first_party => r61::check(ctx),
                    71 if first_party => r71::check(ctx),
                    _ if first_party => Report::new(
                        Outcome::Inconclusive,
                        format!("no built-in check for rule {key} in this version of statute"),
                    ),
                    _ => Report::new(
                        Outcome::Inconclusive,
                        format!(
                            "statute has built-in checks only for rules of the first-party source, `{FIRST_PARTY}`, so none for {key}"
                        ),
                    ),
                },
                Check::Command { run, tools } => command::check(ctx, run, tools),
            };
            Some((key.clone(), report))
        })
        .collect()
}

#[derive(serde::Deserialize)]
struct Visibility {
    visibility: String,
}

/// The visibility GitHub shows for the repository at `root`, `PUBLIC`,
/// `PRIVATE` or `INTERNAL`, as `gh repo view` gives it; or why gh could
/// not tell.
pub fn github_visibility(root: &Path) -> Result<String, String> {
    visibility_of(root, None)
}

/// As `github_visibility`, for the repository `repo` (`owner/name`) when
/// given, else the one at `root`.
pub fn visibility_of(root: &Path, repo: Option<&str>) -> Result<String, String> {
    let ctx = Context { root, base: None };
    let mut args = vec!["repo", "view"];
    args.extend(repo);
    args.extend(["--json", "visibility"]);
    github::gh::<Visibility>(&ctx, &args)
        .map(|answer| answer.visibility)
        .map_err(|report| report.note)
}

/// Compares the rule set's `public` with the repository's visibility on
/// GitHub (`docs/design/rule-sources.md`, Public or not): a rule set that
/// says private in a repository GitHub shows as public is a violation,
/// since its private rules would be published.
pub fn visibility(project: &Project, ctx: &Context) -> Report {
    let declared = project.manifest.public;
    let said = format!("statute.toml says `public = {declared}`");
    let on_github = match github_visibility(ctx.root) {
        Ok(on_github) => on_github,
        Err(why) => {
            return Report::new(
                Outcome::Inconclusive,
                format!("{said}, and GitHub was not asked: {why}"),
            );
        }
    };
    match (declared, on_github.as_str()) {
        (false, "PUBLIC") => Report {
            outcome: Outcome::Violation,
            findings: vec![Finding {
                location: crate::manifest::FILE.to_owned(),
                reason: "`public = false`, and anyone can read the repository; write `public = true`, and name private sources in statute.private.toml".to_owned(),
            }],
            note: format!("{said}, and GitHub shows the repository as {on_github}"),
        },
        _ => Report::new(
            Outcome::Pass,
            format!("{said}, and GitHub shows the repository as {on_github}"),
        ),
    }
}

/// `git` for the repository at `root`.
fn git(root: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(root);
    without_git_locals(&mut cmd);
    cmd
}

/// Takes git's repository-local variables (`git rev-parse
/// --local-env-vars`) out of a command's environment: a git hook sets
/// them, and they would win over the repository the command is given.
pub(crate) fn without_git_locals(cmd: &mut Command) {
    let local = Command::new("git")
        .args(["rev-parse", "--local-env-vars"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default();
    for var in local.split_whitespace() {
        cmd.env_remove(var);
    }
}

/// Whether a program of this name is on `PATH`.
fn on_path(tool: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(tool).is_file()))
}
