//! Rule 58, release-first-green-commit: release only a commit whose CI
//! run for its push to the main branch succeeded on its first attempt.
//! The release it sees is a tag on `HEAD`; the main branch is the
//! repository's default branch on GitHub, and CI is the workflow that
//! holds a check the default branch requires, by branch protection or by
//! a ruleset. It reads that workflow's runs for the push of `HEAD` to the
//! default branch: it passes when each concluded in success on attempt 1,
//! and is violated when one did not.
//!
//! Without a tag on `HEAD` there is no release to check, and without a
//! required check, which workflow is CI cannot be told; both are
//! inconclusive. So is a commit without such a run, since the rule asks
//! nothing where no CI runs on pushes to the main branch, and a run that
//! has not finished, which is waited for.

use std::collections::BTreeSet;

use serde::Deserialize;

use super::github::{gh, head};
use super::{Context, Finding, Outcome, Report, git};

#[derive(Deserialize)]
struct Repository {
    default_branch: String,
}

#[derive(Deserialize)]
struct Branch {
    #[serde(default)]
    protection: Protection,
}

#[derive(Deserialize, Default)]
struct Protection {
    #[serde(default)]
    required_status_checks: Option<Contexts>,
}

#[derive(Deserialize)]
struct Contexts {
    #[serde(default)]
    contexts: Vec<String>,
}

#[derive(Deserialize)]
struct Rule {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    parameters: Option<RuleParameters>,
}

#[derive(Deserialize)]
struct RuleParameters {
    #[serde(default)]
    required_status_checks: Vec<RequiredCheck>,
}

#[derive(Deserialize)]
struct RequiredCheck {
    context: String,
}

#[derive(Deserialize)]
struct CheckRuns {
    check_runs: Vec<CheckRun>,
}

#[derive(Deserialize)]
struct CheckRun {
    name: String,
    check_suite: Option<Suite>,
}

#[derive(Deserialize)]
struct Suite {
    id: u64,
}

#[derive(Deserialize)]
struct Runs {
    workflow_runs: Vec<Run>,
}

#[derive(Deserialize)]
struct Run {
    name: Option<String>,
    html_url: String,
    check_suite_id: u64,
    run_attempt: u64,
    status: String,
    conclusion: Option<String>,
}

pub fn check(ctx: &Context) -> Report {
    check_or_stop(ctx).unwrap_or_else(|report| report)
}

fn check_or_stop(ctx: &Context) -> Result<Report, Report> {
    let tags = tags_on_head(ctx)?;
    if tags.is_empty() {
        return Ok(Report::new(
            Outcome::Inconclusive,
            "no tag on HEAD, so no release to check; a release by other means is for the reviewer",
        ));
    }
    let tags = tags.join(", ");
    let sha = head(ctx)?;
    let repo: Repository = gh(ctx, &["api", "repos/{owner}/{repo}"])?;
    let main = repo.default_branch;
    let required = required_checks(ctx, &main)?;
    if required.is_empty() {
        return Ok(Report::new(
            Outcome::Inconclusive,
            format!("{main} requires no check, so which workflow is CI cannot be told"),
        ));
    }
    let check_runs: CheckRuns = gh(
        ctx,
        &[
            "api",
            &format!("repos/{{owner}}/{{repo}}/commits/{sha}/check-runs?per_page=100"),
        ],
    )?;
    let suites: BTreeSet<u64> = check_runs
        .check_runs
        .iter()
        .filter(|c| required.contains(&c.name))
        .filter_map(|c| c.check_suite.as_ref().map(|s| s.id))
        .collect();
    let runs: Runs = gh(
        ctx,
        &[
            "api",
            &format!(
                "repos/{{owner}}/{{repo}}/actions/runs?head_sha={sha}&event=push&branch={main}"
            ),
        ],
    )?;
    let runs: Vec<Run> = runs
        .workflow_runs
        .into_iter()
        .filter(|r| suites.contains(&r.check_suite_id))
        .collect();
    if runs.is_empty() {
        return Ok(Report::new(
            Outcome::Inconclusive,
            format!(
                "HEAD ({sha}), tagged {tags}, has no run of the required check's workflow for a push to {main}; whether CI runs on pushes to {main}, and whether the commit is on it, are for the reviewer"
            ),
        ));
    }
    if let Some(run) = runs.iter().find(|r| r.status != "completed") {
        return Ok(Report::new(
            Outcome::Inconclusive,
            format!(
                "the run {} has not finished ({}); wait for it",
                run.html_url, run.status
            ),
        ));
    }
    let findings: Vec<Finding> = runs
        .iter()
        .filter(|r| r.conclusion.as_deref() != Some("success") || r.run_attempt != 1)
        .map(|r| Finding {
            location: r.html_url.clone(),
            reason: format!(
                "{} concluded {} on attempt {}",
                r.name.as_deref().unwrap_or("the run"),
                r.conclusion.as_deref().unwrap_or("nothing"),
                r.run_attempt
            ),
        })
        .collect();
    let what = format!("HEAD ({sha}), tagged {tags}");
    if findings.is_empty() {
        let urls: Vec<&str> = runs.iter().map(|r| r.html_url.as_str()).collect();
        Ok(Report::new(
            Outcome::Pass,
            format!(
                "{what}: the push to {main} succeeded on attempt 1 ({})",
                urls.join(", ")
            ),
        ))
    } else {
        Ok(Report {
            outcome: Outcome::Violation,
            findings,
            note: format!(
                "{what}: the push to {main} did not succeed on attempt 1; release a later commit of {main} that did"
            ),
        })
    }
}

fn tags_on_head(ctx: &Context) -> Result<Vec<String>, Report> {
    let out = git(ctx.root)
        .args(["tag", "--points-at", "HEAD"])
        .output()
        .map_err(|e| Report::new(Outcome::Inconclusive, format!("cannot run git: {e}")))?;
    if !out.status.success() {
        return Err(Report::new(
            Outcome::Inconclusive,
            format!(
                "git cannot list the tags on HEAD: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect())
}

/// The checks `main` requires, by branch protection and by rulesets.
fn required_checks(ctx: &Context, main: &str) -> Result<BTreeSet<String>, Report> {
    let branch: Branch = gh(
        ctx,
        &["api", &format!("repos/{{owner}}/{{repo}}/branches/{main}")],
    )?;
    let rules: Vec<Rule> = gh(
        ctx,
        &[
            "api",
            &format!("repos/{{owner}}/{{repo}}/rules/branches/{main}"),
        ],
    )?;
    let protected = branch
        .protection
        .required_status_checks
        .into_iter()
        .flat_map(|c| c.contexts);
    let ruled = rules
        .into_iter()
        .filter(|r| r.kind == "required_status_checks")
        .filter_map(|r| r.parameters)
        .flat_map(|p| p.required_status_checks)
        .map(|c| c.context);
    Ok(protected.chain(ruled).collect())
}
