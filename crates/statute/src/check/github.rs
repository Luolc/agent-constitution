//! Reading GitHub through the `gh` on the machine, for the checks that
//! need the state of a pull request or a CI run (`docs/design/rule-system.md`,
//! Checks). statute never reads, stores or prints a token: `gh` uses the
//! login of its own configuration. No `gh` on `PATH`, no login, a
//! repository not on GitHub and a failed call all make the check
//! inconclusive.

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::{Context, Outcome, Report, git, on_path, without_git_locals};

/// Runs `gh` in the repository and reads its standard output as JSON. In
/// `gh api`, `{owner}` and `{repo}` stand for the repository's GitHub
/// remote.
pub fn gh<T: DeserializeOwned>(ctx: &Context, args: &[&str]) -> Result<T, Report> {
    if !on_path("gh") {
        return Err(Report::new(
            Outcome::Inconclusive,
            "gh is not installed, so GitHub was not read",
        ));
    }
    let mut cmd = std::process::Command::new("gh");
    without_git_locals(&mut cmd);
    let out = cmd
        .args(args)
        .current_dir(ctx.root)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| Report::new(Outcome::Inconclusive, format!("cannot run gh: {e}")))?;
    if !out.status.success() {
        // The last line says what went wrong: no login, no GitHub remote,
        // a call GitHub refused.
        let err = String::from_utf8_lossy(&out.stderr);
        let last = err.lines().last().unwrap_or("").trim();
        return Err(Report::new(
            Outcome::Inconclusive,
            format!("gh {} failed ({}): {last}", args[0], out.status),
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| {
        Report::new(
            Outcome::Inconclusive,
            format!("gh {} answered in a form statute cannot read: {e}", args[0]),
        )
    })
}

/// The full SHA of `HEAD`.
pub fn head(ctx: &Context) -> Result<String, Report> {
    let out = git(ctx.root)
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .map_err(|e| Report::new(Outcome::Inconclusive, format!("cannot run git: {e}")))?;
    if !out.status.success() {
        return Err(Report::new(
            Outcome::Inconclusive,
            "the repository has no commit at HEAD",
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

#[derive(Deserialize)]
struct PullRequest {
    number: u64,
}

/// The pull requests of `HEAD`, as GitHub associates them with the
/// commit: the one that merged it into the default branch, or else the
/// open and merged ones that hold it. None is inconclusive.
pub fn pull_requests(ctx: &Context) -> Result<Vec<u64>, Report> {
    let sha = head(ctx)?;
    let prs: Vec<PullRequest> = gh(
        ctx,
        &[
            "api",
            &format!("repos/{{owner}}/{{repo}}/commits/{sha}/pulls"),
        ],
    )?;
    if prs.is_empty() {
        return Err(Report::new(
            Outcome::Inconclusive,
            format!("no pull request on GitHub holds HEAD ({sha})"),
        ));
    }
    Ok(prs.into_iter().map(|p| p.number).collect())
}

/// "pull request #3" or "pull requests #3, #5".
pub fn list(prs: &[u64]) -> String {
    let numbers: Vec<String> = prs.iter().map(|n| format!("#{n}")).collect();
    let noun = if prs.len() == 1 {
        "pull request"
    } else {
        "pull requests"
    };
    format!("{noun} {}", numbers.join(", "))
}
