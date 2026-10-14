//! Running the checks of the enabled rules (`docs/design/rule-system.md`,
//! Checks, and Outcomes and the review loop). A check has three outcomes;
//! a rule without an automatic check goes to the reviewer as it is.

use std::path::Path;
use std::process::Command;

use crate::node::{Check, Id};
use crate::project::Project;

mod command;
mod r3;
mod r43;
mod r46;
mod r50;
mod r7;
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

/// The checks of every enabled rule, in order of ID.
pub fn run(project: &Project, ctx: &Context) -> Vec<(Id, Report)> {
    project
        .resolution
        .enabled
        .keys()
        .filter_map(|&id| {
            let rule = project.source.get(id)?.rule.as_ref()?;
            let report = match &rule.check {
                Check::Review => Report::new(Outcome::Review, "no automatic check"),
                Check::Builtin => match id {
                    3 => r3::check(ctx),
                    7 => r7::check(ctx),
                    43 => r43::check(ctx),
                    46 => r46::check(ctx),
                    50 => r50::check(ctx),
                    _ => Report::new(
                        Outcome::Inconclusive,
                        format!("no built-in check for rule {id} in this version of statute"),
                    ),
                },
                Check::Command { run, tools } => command::check(ctx, run, tools),
            };
            Some((id, report))
        })
        .collect()
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
fn without_git_locals(cmd: &mut Command) {
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
