//! Rule 50, commit-hook-credential-scan: the repository's commit hooks
//! scan the staged changes for credentials at every commit. The check
//! reads the pre-commit configuration, `.pre-commit-config.yaml`, and
//! passes when one of its hooks runs gitleaks on the staged changes, with
//! `--redact`, at the commit stage and for every commit. Whether the hooks
//! are installed in a checkout is outside the rule.
//!
//! What it cannot judge is inconclusive: another scanner than gitleaks, a
//! hook that runs a script of the repository, a scan that runs only when
//! certain files change, a gitleaks without `--redact`, and the
//! configuration of another hook framework. A configuration with none of
//! these and no gitleaks hook that scans the staged changes at commit
//! time, or no hook configuration at all, is a violation.

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

/// The hooks gitleaks publishes. Their entry, as of 2026-10-14, is
/// `gitleaks git --pre-commit --redact --staged --verbose`.
const GITLEAKS_HOOKS: &[&str] = &["gitleaks", "gitleaks-docker", "gitleaks-system"];
const GITLEAKS_ENTRY: &str = "gitleaks git --pre-commit --redact --staged --verbose";

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
    script: bool,
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
            .any(|w| w.rsplit('/').next() == Some("gitleaks"))
    }

    fn has(&self, flag: &str) -> bool {
        self.words.iter().any(|w| w == flag)
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
            if !hook.has("--staged") {
                findings.push(Finding {
                    location: format!("{CONFIG}: {name}"),
                    reason: "runs gitleaks without --staged, so not on the staged changes"
                        .to_owned(),
                });
            } else if !hook.has("--redact") {
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
        } else if hook.script {
            open.push(format!("{name} runs a script statute does not read"));
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
            let upstream = url
                .trim_end_matches('/')
                .trim_end_matches(".git")
                .ends_with("/gitleaks/gitleaks")
                && GITLEAKS_HOOKS.contains(&id.as_str());
            let script = url == "local"
                && (string(hook, "language")?.as_deref() == Some("script")
                    || entry
                        .as_deref()
                        .and_then(|e| e.split_whitespace().next())
                        .is_some_and(|w| w.contains('/')));
            let mut words: Vec<String> = match (&entry, upstream) {
                (Some(entry), _) => entry.split_whitespace().map(str::to_owned).collect(),
                (None, true) => GITLEAKS_ENTRY.split(' ').map(str::to_owned).collect(),
                (None, false) => Vec::new(),
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
                script,
            });
        }
    }
    Ok(hooks)
}
