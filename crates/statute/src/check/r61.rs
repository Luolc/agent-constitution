//! Rule 61, verdict-names-version: every review verdict names the exact
//! version reviewed. For each pull request of `HEAD` on GitHub, it reads
//! the verdicts on it and looks in each for the full SHA of a commit of
//! that pull request; an abbreviated SHA, a branch name or a pull request
//! number does not name a version exactly. A verdict is a comment or a
//! review whose text opens with `Verdict:`, and a review that approves the
//! change or asks for changes.
//!
//! It covers only part of the rule: verdicts given elsewhere or in
//! another form, and whether the SHA named is the version the reviewer
//! actually reviewed, are for the reviewer. So verdicts that all name a
//! commit of their pull request are inconclusive, not a pass.

use regex::Regex;
use serde::Deserialize;

use super::github::{gh, list, pull_requests};
use super::{Context, Finding, Outcome, Report};

#[derive(Deserialize)]
struct PullRequest {
    commits: Vec<Commit>,
    comments: Vec<Comment>,
    reviews: Vec<Review>,
}

#[derive(Deserialize)]
struct Commit {
    oid: String,
}

#[derive(Deserialize)]
struct Comment {
    body: String,
    url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Review {
    body: String,
    state: String,
    submitted_at: Option<String>,
}

pub fn check(ctx: &Context) -> Report {
    match findings(ctx) {
        Ok((prs, 0, _)) => Report::new(
            Outcome::Inconclusive,
            format!("no verdict on {}", list(&prs)),
        ),
        Ok((prs, verdicts, findings)) if findings.is_empty() => Report::new(
            Outcome::Inconclusive,
            format!(
                "each of the {verdicts} verdict(s) on {} names a commit of its pull request; whether it is the version reviewed, and verdicts given elsewhere, are for the reviewer",
                list(&prs)
            ),
        ),
        Ok((prs, verdicts, findings)) => Report {
            outcome: Outcome::Violation,
            note: format!(
                "{} of the {verdicts} verdict(s) on {} name no commit of their pull request by its full SHA",
                findings.len(),
                list(&prs)
            ),
            findings,
        },
        Err(report) => report,
    }
}

/// The pull requests, how many verdicts are on them, and one finding per
/// verdict that names no commit.
fn findings(ctx: &Context) -> Result<(Vec<u64>, usize, Vec<Finding>), Report> {
    let sha = Regex::new(r"\b[0-9a-fA-F]{40}\b").expect("the pattern is valid");
    let prs = pull_requests(ctx)?;
    let mut verdicts = 0;
    let mut findings = Vec::new();
    for &number in &prs {
        let pr: PullRequest = gh(
            ctx,
            &[
                "pr",
                "view",
                &number.to_string(),
                "--json",
                "commits,comments,reviews",
            ],
        )?;
        let names_a_commit = |body: &str| {
            sha.find_iter(body).any(|m| {
                pr.commits
                    .iter()
                    .any(|c| c.oid.eq_ignore_ascii_case(m.as_str()))
            })
        };
        let comments = pr
            .comments
            .iter()
            .filter(|c| opens_with_verdict(&c.body))
            .map(|c| (c.body.as_str(), c.url.clone()));
        let reviews = pr
            .reviews
            .iter()
            .filter(|r| {
                matches!(r.state.as_str(), "APPROVED" | "CHANGES_REQUESTED")
                    || opens_with_verdict(&r.body)
            })
            .map(|r| {
                let when = r.submitted_at.as_deref().unwrap_or("not submitted");
                (
                    r.body.as_str(),
                    format!("pull request #{number}, review {when}"),
                )
            });
        for (body, location) in comments.chain(reviews) {
            verdicts += 1;
            if !names_a_commit(body) {
                findings.push(Finding {
                    location,
                    reason: "the verdict names no commit of the pull request by its full SHA"
                        .to_owned(),
                });
            }
        }
    }
    Ok((prs, verdicts, findings))
}

fn opens_with_verdict(body: &str) -> bool {
    body.trim_start().starts_with("Verdict:")
}
