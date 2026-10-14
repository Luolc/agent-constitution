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
    cmd.arg("-c").arg(run).current_dir(ctx.root);
    if let Some(base) = ctx.base {
        cmd.env("STATUTE_BASE", base);
    }
    let Ran {
        status,
        stdout: out,
        ..
    } = match run_within(cmd, timeout) {
        Ok(ran) => ran,
        Err(Stopped::Spawn(e)) => {
            return Report::new(Outcome::Inconclusive, format!("cannot run `{run}`: {e}"));
        }
        Err(Stopped::TimedOut) => {
            return Report::new(
                Outcome::Inconclusive,
                format!(
                    "`{run}` did not finish within {} s, or left a process holding its output",
                    timeout.as_secs()
                ),
            );
        }
        Err(Stopped::TooLong) => {
            return Report::new(
                Outcome::Inconclusive,
                format!(
                    "`{run}` wrote more than {} MiB of output, so it was cut off and not read",
                    OUTPUT_LIMIT >> 20
                ),
            );
        }
        Err(Stopped::Wait(e)) => {
            return Report::new(
                Outcome::Inconclusive,
                format!("cannot wait for `{run}`: {e}"),
            );
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

/// Why a command run by [`run_within`] gave no result.
pub(super) enum Stopped {
    Spawn(std::io::Error),
    /// It did not exit and close its output within the time.
    TimedOut,
    /// Its standard output passed [`OUTPUT_LIMIT`].
    TooLong,
    Wait(std::io::Error),
}

/// What a command run by [`run_within`] left: its standard error is cut
/// at [`OUTPUT_LIMIT`].
pub(super) struct Ran {
    pub status: std::process::ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Runs a command, without input, to its end, within `timeout`; on a
/// timeout or too much output it is stopped with what it started.
pub(super) fn run_within(mut cmd: Command, timeout: Duration) -> Result<Ran, Stopped> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // A group of its own, so that a timeout stops what the command started
    // too, and nothing else.
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child = cmd.spawn().map_err(Stopped::Spawn)?;
    let deadline = Instant::now() + timeout;
    // Read the output on other threads, so that the wait for it, which
    // lasts as long as any process the command started keeps it open, is
    // bounded by the timeout too.
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.take(OUTPUT_LIMIT + 1).read_to_end(&mut out);
        let _ = sender.send(out);
    });
    let (err_sender, err_receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut err = Vec::new();
        let mut stderr = stderr;
        let _ = (&mut stderr).take(OUTPUT_LIMIT).read_to_end(&mut err);
        let _ = std::io::copy(&mut stderr, &mut std::io::sink());
        let _ = err_sender.send(err);
    });
    let left = || deadline.saturating_duration_since(Instant::now());
    let Ok(stdout) = receiver.recv_timeout(left()) else {
        stop(&mut child);
        return Err(Stopped::TimedOut);
    };
    if stdout.len() as u64 > OUTPUT_LIMIT {
        stop(&mut child);
        return Err(Stopped::TooLong);
    }
    let Ok(stderr) = err_receiver.recv_timeout(left()) else {
        stop(&mut child);
        return Err(Stopped::TimedOut);
    };
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                stop(&mut child);
                return Err(Stopped::TimedOut);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => {
                stop(&mut child);
                return Err(Stopped::Wait(e));
            }
        }
    };
    Ok(Ran {
        status,
        stdout,
        stderr,
    })
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
