//! Rule 50, commit-hook-credential-scan: the repository's commit hooks
//! scan the staged changes for credentials at every commit. The check
//! reads the pre-commit configuration, `.pre-commit-config.yaml`, and
//! passes when one of its hooks runs gitleaks on the staged changes, with
//! `--redact`, at the commit stage and for every commit. Whether the hooks
//! are installed in a checkout is outside the rule.
//!
//! What it cannot judge is inconclusive: another scanner than gitleaks, a
//! hook whose command it cannot see (a script of the repository, or a
//! hook of another repository, whose entry is in that repository), a
//! scan that runs only when certain files change, a gitleaks without
//! `--redact`, and the configuration of another hook framework. No hook
//! configuration at all, or one whose hooks at the commit stage all show
//! their command and none of them scans the staged changes, is a
//! violation.

use super::yaml::{self, Value};
use super::{Context, Finding, Outcome, Report};

const CONFIG: &str = ".pre-commit-config.yaml";

/// Hook configurations statute does not read.
const OTHER_CONFIGS: &[&str] = &[
    ".pre-commit-config.yml",
    ".husky",
    ".githooks",
    "lefthook.yml",
    "lefthook.yaml",
    ".lefthook.yml",
    ".lefthook.yaml",
    ".overcommit.yml",
];

/// gitleaks' repository, under its current and its former name, and the
/// hooks it publishes there.
const GITLEAKS_REPOS: &[&str] = &["/gitleaks/gitleaks", "/zricethezav/gitleaks"];
const GITLEAKS_HOOKS: &[&str] = &["gitleaks", "gitleaks-docker", "gitleaks-system"];
/// The entry of those hooks at v8.30.1, read on 2026-10-14. From v8.1.1
/// to v8.18.0 it was `gitleaks protect --verbose --redact --staged`, with
/// the same flags; before v8.1.1, a bare `gitleaks`, with neither.
const GITLEAKS_ENTRY: &str = "gitleaks git --pre-commit --redact --staged --verbose";
const GITLEAKS_FIRST: (u64, u64, u64) = (8, 1, 1);

/// Credential scanners other than gitleaks, recognized by name only.
const OTHER_SCANNERS: &[&str] = &[
    "detect-secrets",
    "trufflehog",
    "ggshield",
    "git-secrets",
    "secretlint",
    "talisman",
    "ripsecrets",
];

/// The keys that make a hook run only for some files.
const FILTERS: &[&str] = &["files", "exclude", "types", "types_or", "exclude_types"];

struct Hook {
    repo: String,
    id: String,
    /// The command line: the entry, the given one or gitleaks' own, and
    /// the arguments.
    words: Vec<String>,
    /// The stages it runs at; `None` is every stage.
    stages: Option<Vec<String>>,
    filtered: bool,
    /// Why its command cannot be seen, when it cannot.
    unseen: Option<String>,
}

impl Hook {
    fn at_commit(&self) -> bool {
        self.stages
            .as_ref()
            .is_none_or(|s| s.iter().any(|s| s == "pre-commit" || s == "commit"))
    }

    fn gitleaks(&self) -> bool {
        self.words
            .iter()
            .any(|w| w.rsplit('/').next().and_then(|w| w.split(':').next()) == Some("gitleaks"))
    }

    /// Whether the command has `flag`, or `flag=on`.
    fn has(&self, flag: &str, on: &str) -> bool {
        self.words.iter().any(|w| {
            w == flag
                || w.strip_prefix(flag)
                    .and_then(|v| v.strip_prefix('='))
                    .is_some_and(|v| v == on)
        })
    }
}

pub fn check(ctx: &Context) -> Report {
    let others: Vec<&str> = OTHER_CONFIGS
        .iter()
        .copied()
        .filter(|p| ctx.root.join(p).exists())
        .collect();
    let text = match std::fs::read_to_string(ctx.root.join(CONFIG)) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if others.is_empty() {
                return Report {
                    outcome: Outcome::Violation,
                    findings: vec![Finding {
                        location: CONFIG.to_owned(),
                        reason: "missing, and no other hook configuration was found".to_owned(),
                    }],
                    note: "the repository has no commit hook configuration".to_owned(),
                };
            }
            return Report::new(
                Outcome::Inconclusive,
                format!(
                    "no {CONFIG}; statute does not read {}, which is for the reviewer",
                    others.join(", ")
                ),
            );
        }
        Err(e) => {
            return Report::new(Outcome::Inconclusive, format!("cannot read {CONFIG}: {e}"));
        }
    };
    let hooks = match yaml::parse(&text).and_then(|config| hooks(&config)) {
        Ok(hooks) => hooks,
        Err(e) => {
            return Report::new(
                Outcome::Inconclusive,
                format!("statute cannot read {CONFIG}: {e}"),
            );
        }
    };

    let mut open = Vec::new();
    let mut findings = Vec::new();
    for hook in &hooks {
        let name = format!("hook {} of {}", hook.id, hook.repo);
        if !hook.at_commit() {
            if hook.gitleaks() {
                findings.push(Finding {
                    location: format!("{CONFIG}: {name}"),
                    reason: format!(
                        "runs only at {}, not at the commit",
                        hook.stages.as_deref().unwrap_or_default().join(", ")
                    ),
                });
            }
            continue;
        }
        if hook.gitleaks() {
            if !hook.has("--staged", "true") {
                findings.push(Finding {
                    location: format!("{CONFIG}: {name}"),
                    reason: "runs gitleaks without --staged, so not on the staged changes"
                        .to_owned(),
                });
            } else if !hook.has("--redact", "100") {
                open.push(format!(
                    "{name} runs gitleaks without --redact, so a hit may show the value"
                ));
            } else if hook.filtered {
                open.push(format!(
                    "{name} runs gitleaks only when certain files change"
                ));
            } else {
                return Report::new(
                    Outcome::Pass,
                    format!(
                        "{name} in {CONFIG} runs gitleaks on the staged changes, with --redact, at every commit"
                    ),
                );
            }
        } else if let Some(scanner) = OTHER_SCANNERS.iter().find(|s| {
            hook.repo.contains(*s)
                || hook.id.contains(*s)
                || hook.words.iter().any(|w| w.contains(*s))
        }) {
            open.push(format!(
                "{name} runs {scanner}, which statute does not judge"
            ));
        } else if let Some(why) = &hook.unseen {
            open.push(format!("{name} {why}"));
        }
    }
    if !others.is_empty() {
        open.push(format!("statute does not read {}", others.join(", ")));
    }
    if !open.is_empty() {
        return Report::new(
            Outcome::Inconclusive,
            format!(
                "no hook in {CONFIG} is known to scan the staged changes for credentials; {}; for the reviewer",
                open.join("; ")
            ),
        );
    }
    findings.push(Finding {
        location: CONFIG.to_owned(),
        reason: "no hook scans the staged changes for credentials at every commit".to_owned(),
    });
    Report {
        outcome: Outcome::Violation,
        findings,
        note: format!(
            "{CONFIG} has {} hook(s), and none scans the staged changes for credentials at every commit",
            hooks.len()
        ),
    }
}

/// Every hook of a pre-commit configuration.
fn hooks(config: &Value) -> Result<Vec<Hook>, String> {
    let strings = |value: Option<&Value>, key: &str| -> Result<Option<Vec<String>>, String> {
        match value {
            None | Some(Value::Null) => Ok(None),
            Some(v) => v
                .as_seq()
                .and_then(|items| {
                    items
                        .iter()
                        .map(|i| i.as_str().map(str::to_owned))
                        .collect()
                })
                .map(Some)
                .ok_or_else(|| format!("`{key}` is not a list of strings")),
        }
    };
    let string = |value: &Value, key: &str| -> Result<Option<String>, String> {
        match value.get(key) {
            None => Ok(None),
            Some(v) => v
                .as_str()
                .map(|s| Some(s.to_owned()))
                .ok_or_else(|| format!("`{key}` is not a string")),
        }
    };
    let filtered_all = config.get("files").is_some() || config.get("exclude").is_some();
    let default_stages = strings(config.get("default_stages"), "default_stages")?;
    let repos = config
        .get("repos")
        .and_then(Value::as_seq)
        .ok_or("no list `repos`")?;
    let mut hooks = Vec::new();
    for repo in repos {
        let url = string(repo, "repo")?.ok_or("a repo without `repo`")?;
        let Some(list) = repo.get("hooks") else {
            continue;
        };
        for hook in list.as_seq().ok_or("`hooks` is not a list")? {
            let id = string(hook, "id")?.ok_or("a hook without `id`")?;
            let entry = string(hook, "entry")?;
            let rev = string(repo, "rev")?.unwrap_or_default();
            let language = string(hook, "language")?;
            let gitleaks_repo = GITLEAKS_REPOS.iter().any(|r| {
                url.trim_end_matches('/')
                    .trim_end_matches(".git")
                    .ends_with(r)
            }) && GITLEAKS_HOOKS.contains(&id.as_str());
            let (mut words, unseen): (Vec<String>, _) = match &entry {
                Some(entry) => {
                    let script = url == "local"
                        && (language.as_deref() == Some("script")
                            || (language.as_deref() != Some("docker_image")
                                && entry
                                    .split_whitespace()
                                    .next()
                                    .is_some_and(|w| w.contains('/'))));
                    (
                        entry.split_whitespace().map(str::to_owned).collect(),
                        script.then(|| "runs a script statute does not read".to_owned()),
                    )
                }
                None if gitleaks_repo && version(&rev).is_some_and(|v| v >= GITLEAKS_FIRST) => {
                    (GITLEAKS_ENTRY.split(' ').map(str::to_owned).collect(), None)
                }
                None if gitleaks_repo => (
                    Vec::new(),
                    Some(format!(
                        "is gitleaks at {rev}, whose entry statute does not know"
                    )),
                ),
                None if url == "meta" => (Vec::new(), None),
                None => (
                    Vec::new(),
                    Some("has its entry in its repository, which statute does not read".to_owned()),
                ),
            };
            words.extend(strings(hook.get("args"), "args")?.unwrap_or_default());
            let always = string(hook, "always_run")?.as_deref() == Some("true");
            hooks.push(Hook {
                repo: url.clone(),
                id,
                words,
                stages: strings(hook.get("stages"), "stages")?.or(default_stages.clone()),
                filtered: !always
                    && (filtered_all || FILTERS.iter().any(|k| hook.get(k).is_some())),
                unseen,
            });
        }
    }
    Ok(hooks)
}

/// `vX.Y.Z` as numbers.
fn version(rev: &str) -> Option<(u64, u64, u64)> {
    let mut parts = rev.strip_prefix('v')?.split('.').map(|p| p.parse().ok());
    let v = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(v)
}
