//! A rule's own check, an external command (`check = "command"`). The
//! contract: exit status 0 when the rule is met, 1 when it is violated,
//! anything else when the command could not decide; on standard output, a
//! JSON list of the findings, each `{"location": ..., "reason": ...}`.

use std::io::Read as _;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::{Context, Finding, Outcome, Report, on_path};

/// How long a command may run before it counts as inconclusive.
const TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Deserialize)]
struct RawFinding {
    location: String,
    reason: String,
}

pub fn check(ctx: &Context, run: &str, tools: &[String]) -> Report {
    check_within(ctx, run, tools, TIMEOUT)
}

fn check_within(ctx: &Context, run: &str, tools: &[String], timeout: Duration) -> Report {
    let missing: Vec<&str> = tools
        .iter()
        .map(String::as_str)
        .filter(|t| !on_path(t))
        .collect();
    if !missing.is_empty() {
        return Report::new(
            Outcome::Inconclusive,
            format!("not installed: {}", missing.join(", ")),
        );
    }
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(run)
        .current_dir(ctx.root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(base) = ctx.base {
        cmd.env("STATUTE_BASE", base);
    }
    // A group of its own, so that a timeout stops what the command started
    // too.
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => return Report::new(Outcome::Inconclusive, format!("cannot run `{run}`: {e}")),
    };
    // Read the output on another thread, so a full pipe cannot stall the
    // command while it is waited for.
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out);
        out
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() >= timeout => {
                #[cfg(unix)]
                let _ = Command::new("kill")
                    .args(["-KILL", "--", &format!("-{}", child.id())])
                    .status();
                let _ = child.kill();
                let _ = child.wait();
                return Report::new(
                    Outcome::Inconclusive,
                    format!("`{run}` did not finish within {} s", timeout.as_secs()),
                );
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => {
                return Report::new(
                    Outcome::Inconclusive,
                    format!("cannot wait for `{run}`: {e}"),
                );
            }
        }
    };
    let out = reader.join().unwrap_or_default();
    match status.code() {
        Some(0) => Report::new(Outcome::Pass, format!("`{run}` exited 0")),
        Some(1) => match serde_json::from_slice::<Vec<RawFinding>>(&out) {
            Ok(raw) => Report {
                outcome: Outcome::Violation,
                findings: raw
                    .into_iter()
                    .map(|f| Finding {
                        location: f.location,
                        reason: f.reason,
                    })
                    .collect(),
                note: format!("`{run}` exited 1"),
            },
            Err(e) => Report::new(
                Outcome::Inconclusive,
                format!("`{run}` exited 1, and its output is not the JSON list of findings: {e}"),
            ),
        },
        _ => Report::new(
            Outcome::Inconclusive,
            format!("`{run}` could not decide ({status})"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_past_its_time_is_inconclusive() {
        let root = std::env::current_dir().unwrap();
        let ctx = Context {
            root: &root,
            base: None,
        };
        let start = Instant::now();
        // A command that starts another and waits for it: the timeout
        // stops both, or the output would stay open.
        let report = check_within(&ctx, "sleep 30 & wait", &[], Duration::from_millis(300));
        assert_eq!(report.outcome, Outcome::Inconclusive);
        assert!(
            report.note.contains("did not finish within"),
            "{}",
            report.note
        );
        assert!(start.elapsed() < Duration::from_secs(10));
        let quick = check_within(&ctx, "exit 0", &[], Duration::from_secs(10));
        assert_eq!(quick.outcome, Outcome::Pass);
    }
}
