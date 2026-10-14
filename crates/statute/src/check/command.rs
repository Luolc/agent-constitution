//! A rule's own check, an external command (`check = "command"`). The
//! contract: exit status 0 when the rule is met, 1 when it is violated,
//! anything else when the command could not decide; on standard output, a
//! JSON list of the findings, each `{"location": ..., "reason": ...}`.

use std::io::Read as _;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::{Context, Finding, Outcome, Report, on_path};

/// How long a command may run before it counts as inconclusive.
const TIMEOUT: Duration = Duration::from_secs(600);

/// How much output a command may write; more counts as inconclusive.
const OUTPUT_LIMIT: u64 = 1 << 20;

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
    // too, and nothing else.
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => return Report::new(Outcome::Inconclusive, format!("cannot run `{run}`: {e}")),
    };
    let deadline = Instant::now() + timeout;
    let timed_out = || {
        Report::new(
            Outcome::Inconclusive,
            format!(
                "`{run}` did not finish within {} s, or left a process holding its output",
                timeout.as_secs()
            ),
        )
    };
    // Read the output on another thread, so that the wait for it, which
    // lasts as long as any process the command started keeps it open, is
    // bounded by the timeout too.
    let stdout = child.stdout.take().expect("stdout is piped");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.take(OUTPUT_LIMIT + 1).read_to_end(&mut out);
        let _ = sender.send(out);
    });
    let Ok(out) = receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) else {
        stop(&mut child);
        return timed_out();
    };
    if out.len() as u64 > OUTPUT_LIMIT {
        stop(&mut child);
        return Report::new(
            Outcome::Inconclusive,
            format!(
                "`{run}` wrote more than {} MiB of output, so it was cut off and not read",
                OUTPUT_LIMIT >> 20
            ),
        );
    }
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                stop(&mut child);
                return timed_out();
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => {
                stop(&mut child);
                return Report::new(
                    Outcome::Inconclusive,
                    format!("cannot wait for `{run}`: {e}"),
                );
            }
        }
    };
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

/// Kills the command and the processes of its group. The child is not
/// reaped before the group is signalled, so its ID cannot have passed to
/// another group. Outside Unix there is no group, and only the command
/// itself is killed.
fn stop(child: &mut Child) {
    #[cfg(unix)]
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{}", child.id())])
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
    let _ = child.wait();
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

    #[test]
    fn a_process_left_holding_the_output_is_stopped_at_the_timeout() {
        let root = std::env::current_dir().unwrap();
        let ctx = Context {
            root: &root,
            base: None,
        };
        // Next to the test binary, under the build directory.
        let pid_file = std::env::current_exe()
            .unwrap()
            .with_file_name(format!("statute-left-behind-{}", std::process::id()));
        let run = format!("sleep 4 & echo $! > '{}'; exit 0", pid_file.display());
        let start = Instant::now();
        let report = check_within(&ctx, &run, &[], Duration::from_millis(500));
        let elapsed = start.elapsed();
        let pid = std::fs::read_to_string(&pid_file).unwrap();
        std::fs::remove_file(&pid_file).unwrap();
        let pid = pid.trim();
        assert!(!pid.is_empty());
        assert_eq!(report.outcome, Outcome::Inconclusive, "{}", report.note);
        assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
        // The `sleep` it started is gone: killed, then reaped by init.
        let alive = || {
            Command::new("kill")
                .args(["-0", pid])
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success()
        };
        let gone_by = Instant::now() + Duration::from_secs(2);
        while alive() && Instant::now() < gone_by {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!alive(), "sleep {pid} is still running");
    }

    #[test]
    fn output_past_the_limit_is_inconclusive() {
        let root = std::env::current_dir().unwrap();
        let ctx = Context {
            root: &root,
            base: None,
        };
        let limit = OUTPUT_LIMIT.to_string();
        let at_limit = check_within(
            &ctx,
            &format!("head -c {limit} /dev/zero; exit 0"),
            &[],
            Duration::from_secs(10),
        );
        assert_eq!(at_limit.outcome, Outcome::Pass, "{}", at_limit.note);
        let past = check_within(&ctx, "yes; exit 0", &[], Duration::from_secs(10));
        assert_eq!(past.outcome, Outcome::Inconclusive);
        assert!(past.note.contains("cut off"), "{}", past.note);
    }
}
