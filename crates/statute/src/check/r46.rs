//! Rule 46, instruction-files-apart: a change does not mix instruction
//! files with other files; and rule 280, which supersedes it, with its
//! one exception, the change that migrates a repository onto statute.
//! The path tables of `check-instruction-changes`
//! of this repository's tools, with statute's own files beside the rule
//! set (`docs/design/checkers.md` says where the two differ).
//!
//! It covers only part of the rule: the tables hold the files of the
//! agent tools they name, statute's files and this repository's own
//! sources; the files of other agent tools, the sources another
//! repository compiles its instructions from, and paths a repository
//! declares as instructions in its own `AGENTS.md` are for the reviewer.
//! So a change it finds nothing wrong with is inconclusive, not a pass.
//! Whether a generated file changed only in rendering is rule 47's, for
//! the reviewer.

use std::path::Path;

use regex::RegexSet;

use super::{Context, Finding, Outcome, Report, git};
use crate::compile::{CHECKS, FILE as AGENTS_MD, Output};
use crate::manifest::FILE as RULE_SET;

/// Instruction files in any repository: what an agent harness reads as
/// instructions, skills, subagents, commands, hooks or permissions, and
/// the legacy copies of the hand-written instructions a migration keeps.
const GENERIC: &[&str] = &[
    r"(.*/)?AGENTS\.md",
    r"(.*/)?CLAUDE\.md",
    r"(.*/)?\.agents/skills/.+",
    r"(.*/)?\.claude/skills/.+",
    r"(.*/)?\.claude/agents/.+",
    r"(.*/)?\.claude/commands/.+",
    r"(.*/)?\.claude/settings(\.local)?\.json",
    r"(.*/)?\.claude/hooks/.+",
    r"(.*/)?\.codex/.+",
    r"(.*/)?[^/]+\.legacy\.md",
];

/// statute's files beside the rule set, relative to its directory: the
/// rule set, its lock file and `.statute/`, with the local rules, which
/// the instructions are compiled from, and `statute-checks.toml`, which
/// counts as generated only while it equals the compile (`Classes`). The
/// lock file is no generated file:
/// it pins the commit of each git source, and `statute upgrade` moves a
/// source to newer rules by rewriting the lock file alone, which
/// `statute compile --check` accepts.
const STATUTE: &[&str] = &[
    r"statute\.toml",
    r"statute\.lock",
    r"\.statute/.+",
    r"statute-checks\.toml",
];

/// This repository's own instruction files: the constitution, rule and
/// skill sources.
const REPO: &[&str] = &[
    r"constitution/[^/]+/index\.toml",
    r"constitution/[^/]+/articles/.+",
    r"rules/.+",
    r"skills/.+",
];

/// Generated files, which go with whichever side changed them.
const GENERATED: &[&str] = &[r"constitution/[^/]+/generated/constitution\.[^/]+\.md"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Instruction,
    Generated,
    Other,
}

/// Tells the kind of a path relative to the top of the repository.
pub struct Classes {
    /// The rule set's directory relative to the top, with a trailing
    /// `/`; empty at the top.
    prefix: String,
    /// The files statute compiles beside the rule set, `AGENTS.md` and
    /// `statute-checks.toml`, that equal what it compiles from the public
    /// layer now, relative to the top. Rule 47 takes a generated file to
    /// be one shown to equal its build, so only these are generated; one
    /// that differs (edited by hand, not recompiled, or statute cannot
    /// compile) stays an instruction file.
    compiled: Vec<String>,
}

impl Classes {
    /// The classes for the rule set at `root`, in the repository whose
    /// top is `top`, with `output` what statute compiles there now.
    pub fn new(root: &Path, top: &Path, output: Option<&Output>) -> Self {
        let canonical = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let prefix = canonical(root)
            .strip_prefix(canonical(top))
            .map(|p| p.to_string_lossy().into_owned())
            .ok()
            .filter(|p| !p.is_empty())
            .map_or_else(String::new, |p| format!("{p}/"));
        let compiled = output
            .into_iter()
            .flat_map(|out| [(AGENTS_MD, &out.agents_md), (CHECKS, &out.checks)])
            .filter(|(name, text)| {
                std::fs::read(root.join(name)).is_ok_and(|t| t == text.as_bytes())
            })
            .map(|(name, _)| format!("{prefix}{name}"))
            .collect();
        Classes { prefix, compiled }
    }

    /// Which kind `path` is; each pattern matches the whole path, as
    /// Python's `re.fullmatch` does.
    pub fn kind(&self, path: &str) -> Kind {
        let whole = |patterns: &[&str], path: &str| {
            RegexSet::new(patterns.iter().map(|p| format!("^(?:{p})$")))
                .expect("the patterns are valid")
                .is_match(path)
        };
        if whole(GENERATED, path) || self.compiled.iter().any(|c| c == path) {
            Kind::Generated
        } else if whole(GENERIC, path)
            || whole(REPO, path)
            || self.beside(path).is_some_and(|p| whole(STATUTE, p))
        {
            Kind::Instruction
        } else {
            Kind::Other
        }
    }

    /// `path` relative to the rule set's directory, when it is inside it.
    fn beside<'p>(&self, path: &'p str) -> Option<&'p str> {
        path.strip_prefix(&self.prefix)
    }

    /// Whether `path` is a file statute compiles beside the rule set.
    fn compiles(&self, path: &str) -> bool {
        self.beside(path)
            .is_some_and(|p| p == AGENTS_MD || p == CHECKS)
    }
}

/// The check of rule 46, or of rule 280 when `migration` is set: then a
/// change that adds `statute.toml`, which its base does not have, is the
/// migration change, which may mix the two, and what else it holds is
/// for the reviewer.
pub fn check(ctx: &Context, migration: bool) -> Report {
    let Some(base) = ctx.base else {
        return Report::new(
            Outcome::Inconclusive,
            "no change to check; give the base of the change with --base",
        );
    };
    let out = git(ctx.root)
        .args(["diff", "--name-status", "--no-renames", "-z"])
        .arg(format!("{base}...HEAD"))
        .output();
    let out = match out {
        Ok(out) if out.status.success() => out.stdout,
        Ok(out) => {
            return Report::new(
                Outcome::Inconclusive,
                format!(
                    "git cannot give the change since {base}: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                ),
            );
        }
        Err(e) => return Report::new(Outcome::Inconclusive, format!("cannot run git: {e}")),
    };
    let text = String::from_utf8_lossy(&out);
    let fields: Vec<&str> = text.split('\0').filter(|f| !f.is_empty()).collect();
    let classes = Classes::new(ctx.root, &ctx.top(), ctx.compiled);
    let mut instruction = Vec::new();
    let mut generated = Vec::new();
    let mut other = Vec::new();
    let mut adds_rule_set = false;
    for pair in fields.chunks(2) {
        let [status, path] = pair else { continue };
        adds_rule_set |= *status == "A" && classes.beside(path) == Some(RULE_SET);
        let line = format!("{status} {path}");
        match classes.kind(path) {
            Kind::Instruction if classes.compiles(path) => instruction.push((
                line,
                "an instruction file, not what statute compiles now, in the same change",
            )),
            Kind::Instruction => {
                instruction.push((line, "an instruction file, in the same change"))
            }
            Kind::Generated => generated.push(line),
            Kind::Other => other.push((line, "another file, in the same change")),
        }
    }
    let counts = format!(
        "{} instruction, {} other, {} generated",
        instruction.len(),
        other.len(),
        generated.len()
    );
    if migration && adds_rule_set && !other.is_empty() {
        let findings = other
            .iter()
            .map(|(location, _)| Finding {
                location: location.clone(),
                reason: "another file, in the migration change".to_owned(),
            })
            .collect();
        return Report {
            outcome: Outcome::Inconclusive,
            findings,
            note: format!(
                "the change since {base} adds {RULE_SET}, which its base does not have: it is the migration change, which may hold instruction files and other files ({counts}); the reviewer checks that the other files only add `statute compile --check` to each of the commit hooks and the CI the repository has at its base, and that the change holds nothing besides the move and that check"
            ),
        };
    }
    if !instruction.is_empty() && !other.is_empty() {
        let findings = instruction
            .iter()
            .chain(&other)
            .map(|(location, reason)| Finding {
                location: location.clone(),
                reason: (*reason).to_owned(),
            })
            .collect();
        return Report {
            outcome: Outcome::Violation,
            findings,
            note: format!(
                "the change since {base} mixes instruction files with other files ({counts}); split it into two changes"
            ),
        };
    }
    let rendering = if generated.is_empty() {
        String::new()
    } else {
        format!(
            "; generated files: {}, which the reviewer checks changed only in rendering when tools changed them",
            generated.join(", ")
        )
    };
    Report::new(
        Outcome::Inconclusive,
        format!(
            "the change since {base} does not mix instruction files with other files ({counts}){rendering}; files of other agent tools, sources of instructions outside the path tables, and paths the repository declares in its AGENTS.md are for the reviewer"
        ),
    )
}
