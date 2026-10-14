//! Rule 43, credential-designated-place: credentials are stored only in
//! the places designated for them. The part gitleaks can see, a
//! credential committed to the repository, is checked by running the
//! gitleaks on the machine, with `--redact` so that a hit shows only its
//! place and the name of its gitleaks rule, never the value.
//!
//! It covers only part of the rule: gitleaks finds the credentials its
//! rules recognize, and only in the repository; a credential of another
//! form, and one stored outside the repository, are for the reviewer. So
//! a scan with no hit is inconclusive, not a pass.

use serde::Deserialize;

use super::{Context, Finding, Outcome, Report, on_path, without_git_locals};

/// The exit status gitleaks is told to use when it finds a leak, so that
/// a leak and an error of gitleaks are told apart.
const LEAKS: i32 = 3;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Leak {
    file: String,
    start_line: u64,
    #[serde(rename = "RuleID")]
    rule_id: String,
    #[serde(default)]
    commit: String,
}

pub fn check(ctx: &Context) -> Report {
    if !on_path("gitleaks") {
        return Report::new(
            Outcome::Inconclusive,
            "gitleaks is not installed, so nothing was scanned",
        );
    }
    let mut cmd = std::process::Command::new("gitleaks");
    without_git_locals(&mut cmd);
    cmd.args(["git", "--redact", "--no-banner", "--report-format", "json"])
        .args(["--report-path", "-", "--exit-code", &LEAKS.to_string()]);
    let scanned = match ctx.base {
        Some(base) => {
            cmd.arg(format!("--log-opts={base}..HEAD"));
            format!("the commits since {base}")
        }
        None => "the whole history".to_owned(),
    };
    cmd.arg(ctx.root);
    let out = match cmd.output() {
        Ok(out) => out,
        Err(e) => {
            return Report::new(Outcome::Inconclusive, format!("cannot run gitleaks: {e}"));
        }
    };
    match out.status.code() {
        Some(0) => Report::new(
            Outcome::Inconclusive,
            format!(
                "gitleaks found nothing in {scanned}; credentials gitleaks does not recognize, and places outside the repository, are for the reviewer"
            ),
        ),
        Some(LEAKS) => {
            let leaks: Vec<Leak> = match serde_json::from_slice(&out.stdout) {
                Ok(leaks) => leaks,
                Err(e) => {
                    return Report::new(
                        Outcome::Inconclusive,
                        format!("gitleaks reported leaks in a form statute cannot read: {e}"),
                    );
                }
            };
            let findings = leaks
                .iter()
                .map(|l| Finding {
                    location: if l.commit.is_empty() {
                        format!("{}:{}", l.file, l.start_line)
                    } else {
                        format!("{}:{} (commit {})", l.file, l.start_line, l.commit)
                    },
                    reason: format!("gitleaks rule {}", l.rule_id),
                })
                .collect();
            Report {
                outcome: Outcome::Violation,
                findings,
                note: format!("gitleaks found {} credential(s) in {scanned}", leaks.len()),
            }
        }
        _ => {
            // Only the last line of its log: gitleaks redacts values, and
            // an error message carries none.
            let err = String::from_utf8_lossy(&out.stderr);
            let last = err.lines().last().unwrap_or("").trim();
            Report::new(
                Outcome::Inconclusive,
                format!("gitleaks failed ({}): {last}", out.status),
            )
        }
    }
}
