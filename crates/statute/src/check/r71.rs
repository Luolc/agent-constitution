//! Rule 71, no-automatic-merge: never set a change to merge
//! automatically. For each pull request of `HEAD` on GitHub, it reads
//! whether auto-merge was ever enabled on it, from the pull request's
//! timeline, so an auto-merge disabled again or one that has already
//! merged the pull request is found as well.
//!
//! It covers only part of the rule: other ways of merging without someone
//! deciding at that moment, such as a bot or a merge queue, and changes
//! that are not pull requests on GitHub, are for the reviewer. So a pull
//! request that never had auto-merge is inconclusive, not a pass. Who
//! enabled it, an agent or a person, is for the reviewer too.

use serde::Deserialize;

use super::github::{gh, list, pull_requests};
use super::{Context, Finding, Outcome, Report};

const QUERY: &str = "query($owner: String!, $repo: String!, $number: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      timelineItems(itemTypes: [AUTO_MERGE_ENABLED_EVENT], first: 100) {
        nodes { ... on AutoMergeEnabledEvent { createdAt } }
      }
    }
  }
}";

#[derive(Deserialize)]
struct Answer {
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    repository: Repository,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Repository {
    pull_request: PullRequest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRequest {
    timeline_items: Nodes,
}

#[derive(Deserialize)]
struct Nodes {
    nodes: Vec<Event>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Event {
    created_at: String,
}

pub fn check(ctx: &Context) -> Report {
    match findings(ctx) {
        Ok((prs, findings)) if findings.is_empty() => Report::new(
            Outcome::Inconclusive,
            format!(
                "auto-merge was never enabled on {}; other ways of merging automatically are for the reviewer",
                list(&prs)
            ),
        ),
        Ok((prs, findings)) => {
            let enabled: Vec<u64> = prs
                .into_iter()
                .filter(|n| {
                    findings
                        .iter()
                        .any(|f| f.location == format!("pull request #{n}"))
                })
                .collect();
            Report {
                outcome: Outcome::Violation,
                findings,
                note: format!("auto-merge was enabled on {}", list(&enabled)),
            }
        }
        Err(report) => report,
    }
}

fn findings(ctx: &Context) -> Result<(Vec<u64>, Vec<Finding>), Report> {
    let prs = pull_requests(ctx)?;
    let mut findings = Vec::new();
    for &number in &prs {
        let answer: Answer = gh(
            ctx,
            &[
                "api",
                "graphql",
                "-F",
                "owner={owner}",
                "-F",
                "repo={repo}",
                "-F",
                &format!("number={number}"),
                "-f",
                &format!("query={QUERY}"),
            ],
        )?;
        findings.extend(
            answer
                .data
                .repository
                .pull_request
                .timeline_items
                .nodes
                .into_iter()
                .map(|e| Finding {
                    location: format!("pull request #{number}"),
                    reason: format!("auto-merge enabled at {}", e.created_at),
                }),
        );
    }
    Ok((prs, findings))
}
