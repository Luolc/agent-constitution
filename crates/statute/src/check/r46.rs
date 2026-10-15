//! Rule 46, instruction-files-apart: a change does not mix instruction
//! files with other files. The same check as `check-instruction-changes`
//! of this repository's tools, with the same path tables.
//!
//! It covers only part of the rule: the tables hold the files of the
//! agent tools they name and this repository's own sources; the files of
//! other agent tools, the sources another repository compiles its
//! instructions from, and paths a repository declares as instructions in
//! its own `AGENTS.md` are for the reviewer. So a change it finds nothing
//! wrong with is inconclusive, not a pass. Whether a generated file
//! changed only in rendering is rule 47's, for the reviewer.

use regex::RegexSet;

use super::{Context, Finding, Outcome, Report, git};

/// Instruction files in any repository: what an agent harness reads as
/// instructions, skills, subagents, commands, hooks or permissions.
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
];

/// This repository's own instruction files: the constitution, rule and
/// skill sources.
const REPO: &[&str] = &[
    r"constitution/[^/]+/index\.toml",
    r"constitution/[^/]+/articles/.+",
    r"rules/.+",
    r"skills/.+",
];

/// Generated files, which go with whichever side changed them. The
/// script also counts this repository's top `AGENTS.md` as one, under
/// its local rule L28, which overrides rule 46 there, so this check does
/// not run there; elsewhere `statute compile --check` need not be in CI,
/// so here the top `AGENTS.md` stays an instruction file.
const GENERATED: &[&str] = &[r"constitution/[^/]+/generated/constitution\.[^/]+\.md"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Instruction,
    Generated,
    Other,
}

/// Which kind a path relative to the repository is; each pattern matches
/// the whole path, as Python's `re.fullmatch` does.
pub fn kind(path: &str) -> Kind {
    let whole = |patterns: &[&str]| {
        RegexSet::new(patterns.iter().map(|p| format!("^(?:{p})$")))
            .expect("the patterns are valid")
            .is_match(path)
    };
    if whole(GENERATED) {
        Kind::Generated
    } else if whole(GENERIC) || whole(REPO) {
        Kind::Instruction
    } else {
        Kind::Other
    }
}

pub fn check(ctx: &Context) -> Report {
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
    let mut instruction = Vec::new();
    let mut generated = 0;
    let mut other = Vec::new();
    for pair in fields.chunks(2) {
        let [status, path] = pair else { continue };
        let line = format!("{status} {path}");
        match kind(path) {
            Kind::Instruction => instruction.push(line),
            Kind::Generated => generated += 1,
            Kind::Other => other.push(line),
        }
    }
    let counts = format!(
        "{} instruction, {} other, {generated} generated",
        instruction.len(),
        other.len()
    );
    if !instruction.is_empty() && !other.is_empty() {
        let findings = instruction
            .iter()
            .map(|l| (l, "an instruction file"))
            .chain(other.iter().map(|l| (l, "another file")))
            .map(|(location, what)| Finding {
                location: location.clone(),
                reason: format!("{what}, in the same change"),
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
    Report::new(
        Outcome::Inconclusive,
        format!(
            "the change since {base} does not mix instruction files with other files ({counts}); files of other agent tools, sources of instructions outside the path tables, and paths the repository declares in its AGENTS.md are for the reviewer"
        ),
    )
}
