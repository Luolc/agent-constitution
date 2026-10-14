//! `statute check`, run as a user runs it, in git repositories written
//! under Cargo's test directory.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, rule};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

/// Domain 1 with rule 3 and rule 7 (built-in checks), rule 20 (a command)
/// and rule 21 (review only).
fn nodes(command: &str) -> Vec<(u64, String)> {
    let builtin = |id| edit(rule(id, 1, ""), "check = \"review\"", "check = \"builtin\"");
    vec![
        domain(1),
        builtin(3),
        builtin(7),
        edit(
            rule(20, 1, ""),
            "check = \"review\"",
            &format!("check = \"command\"\nrun = {command:?}\ntools = [\"sh\"]"),
        ),
        rule(21, 1, ""),
    ]
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.test",
            "-c",
            "maintenance.auto=false",
            "-c",
            "core.excludesFile=/dev/null",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A git repository with the rule set enabling `rules`, committed and
/// tagged `base`, and locked.
fn repo(name: &str, rules: &str, command: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("check")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q"]);
    write(&dir, "constitution/2026-10/index.toml", INDEX);
    write(
        &dir,
        "statute.toml",
        &format!(
            "constitution = {{ source = \"ac\", edition = \"2026-10\" }}\nrules = [{rules}]\n\n[sources]\nac = {{ path = \".\" }}\n"
        ),
    );
    for (id, text) in nodes(command) {
        write(&dir, &format!("rules/{id}.toml"), &text);
    }
    for (path, text) in [
        ("AGENTS.md", "agents\n"),
        ("tools/build.py", "tool\n"),
        ("docs/design/README.md", "design\n"),
        (
            "constitution/2026-10/generated/constitution.en.md",
            "generated\n",
        ),
    ] {
        write(&dir, path, text);
    }
    let lock = statute(&dir, &["lock"], None);
    assert_eq!(lock.code, 0, "{}", lock.stderr);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-qm", "base"]);
    git(&dir, &["tag", "base"]);
    dir
}

/// Writes `files` and commits them as the change on top of `base`.
fn change(dir: &Path, files: &[&str]) {
    for path in files {
        write(dir, path, "changed\n");
    }
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", "change"]);
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

/// Runs statute in `dir`; `path` replaces `PATH` when given.
fn statute(dir: &Path, args: &[&str], path: Option<&str>) -> Run {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_statute"));
    cmd.args(args).current_dir(dir);
    if let Some(path) = path {
        cmd.env("PATH", path);
    }
    let out = cmd.output().unwrap();
    Run {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

/// The line of rule `id` in the output of `statute check`.
fn line(run: &Run, id: u64) -> String {
    run.stdout
        .lines()
        .find(|l| l.contains(&format!("  {id} (Rule {id}): ")))
        .unwrap_or_else(|| panic!("no line for rule {id}: {}", run.stdout))
        .to_owned()
}

// Rule 7: a change does not mix instruction files with other files.

#[test]
fn r7_mixed_change_is_a_violation() {
    let dir = repo("r7-mixed", "\"ac:7\"", "true");
    change(&dir, &["AGENTS.md", "tools/build.py"]);
    let run = statute(&dir, &["check", "--base", "base"], None);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(line(&run, 7).starts_with("violation "), "{}", run.stdout);
    assert!(
        run.stdout
            .contains("M AGENTS.md: an instruction file, in the same change"),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .contains("M tools/build.py: another file, in the same change"),
        "{}",
        run.stdout
    );
}

#[test]
fn r7_unmixed_change_goes_to_the_reviewer() {
    let dir = repo("r7-other", "\"ac:7\"", "true");
    change(
        &dir,
        &[
            "tools/build.py",
            "constitution/2026-10/generated/constitution.en.md",
        ],
    );
    let run = statute(&dir, &["check", "--base", "base"], None);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let l = line(&run, 7);
    assert!(l.starts_with("inconclusive "), "{l}");
    assert!(l.contains("(0 instruction, 1 other, 1 generated)"), "{l}");
    assert!(
        run.stdout
            .contains("For the reviewer:\n\n## 7 (Rule 7) (inconclusive)\n\nReason: Because.\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn r7_without_a_base_is_inconclusive() {
    let dir = repo("r7-no-base", "\"ac:7\"", "true");
    let run = statute(&dir, &["check"], None);
    assert_eq!(run.code, 0);
    assert!(
        line(&run, 7).contains("give the base of the change with --base"),
        "{}",
        run.stdout
    );
}

/// The same changes, judged by statute's rule 7 and by this repository's
/// `check-instruction-changes`: a violation exactly where the Python check
/// exits 1.
#[test]
fn r7_agrees_with_check_instruction_changes() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cases: &[(&str, &[&str])] = &[
        ("instructions", &["AGENTS.md", "skills/a/SKILL.md"]),
        ("other", &["tools/build.py", "docs/design/README.md"]),
        ("mixed", &["AGENTS.md", "tools/build.py"]),
        (
            "generated-with-tools",
            &[
                "tools/build.py",
                "constitution/2026-10/generated/constitution.en.md",
            ],
        ),
        (
            "generated-with-sources",
            &[
                "constitution/2026-10/articles/01-a/article.toml",
                "constitution/2026-10/generated/constitution.en.md",
            ],
        ),
        (
            "nested-agents",
            &["docs/AGENTS.md", "docs/design/README.md"],
        ),
        (
            "claude-settings",
            &[".claude/settings.local.json", "tools/build.py"],
        ),
        ("codex", &["sub/.codex/config.toml"]),
        (
            "skills-and-docs",
            &["skills/a/SKILL.md", "docs/design/README.md"],
        ),
        ("lookalike", &["AGENTS.md.bak", "tools/build.py"]),
    ];
    let mut seen = (0, 0);
    for (name, files) in cases {
        let dir = repo(&format!("r7-parity-{name}"), "\"ac:7\"", "true");
        change(&dir, files);
        let ours = statute(&dir, &["check", "--base", "base"], None);
        let python = Command::new("uv")
            .args([
                "run",
                "--frozen",
                "--no-sync",
                "check-instruction-changes",
                "--base",
                "base",
                "--root",
            ])
            .arg(&dir)
            .current_dir(&repo_root)
            .output()
            .unwrap();
        let python_code = python.status.code().unwrap();
        assert!(
            python_code == 0 || python_code == 1,
            "{name}: {}",
            String::from_utf8_lossy(&python.stderr)
        );
        if python_code == 1 {
            seen.1 += 1;
        } else {
            seen.0 += 1;
        }
        assert_eq!(
            line(&ours, 7).starts_with("violation "),
            python_code == 1,
            "{name}: statute says {:?}, check-instruction-changes exits {python_code}",
            line(&ours, 7)
        );
    }
    // Both conclusions occur among the cases, so the comparison can fail.
    assert!(seen.0 > 0 && seen.1 > 0, "{seen:?}");
    // A rename is a removal and an addition: AGENTS.md to docs/agents.md
    // mixes the two in both.
    let dir = repo("r7-parity-rename", "\"ac:7\"", "true");
    git(&dir, &["mv", "AGENTS.md", "docs/agents.md"]);
    git(&dir, &["commit", "-qm", "rename"]);
    let ours = statute(&dir, &["check", "--base", "base"], None);
    assert!(line(&ours, 7).starts_with("violation "), "{}", ours.stdout);
}

// Rule 3: credentials stay where they are designated; gitleaks sees the
// part in the repository.

/// A directory holding a fake `gitleaks` that answers as given, plus the
/// programs statute needs, to stand first on `PATH`. It exits 9 unless it
/// was asked to redact.
fn fake_gitleaks(name: &str, exit: i32, stdout: &str) -> String {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("check")
        .join(format!("fake-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("gitleaks");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ncase \" $* \" in *\" --redact \"*) ;; *) exit 9 ;; esac\nprintf '%s' '{stdout}'\nexit {exit}\n"
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    format!("{}:{}", dir.display(), std::env::var("PATH").unwrap())
}

#[test]
fn r3_a_leak_is_a_violation_with_its_place_only() {
    let dir = repo("r3-leak", "\"ac:3\"", "true");
    let report = r#"[{"File":"cfg.txt","StartLine":1,"RuleID":"github-pat","Commit":"abc123","Secret":"REDACTED","Match":"REDACTED"}]"#;
    let run = statute(&dir, &["check"], Some(&fake_gitleaks("leak", 3, report)));
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(line(&run, 3).starts_with("violation "), "{}", run.stdout);
    assert!(
        run.stdout
            .contains("cfg.txt:1 (commit abc123): gitleaks rule github-pat"),
        "{}",
        run.stdout
    );
    assert!(!run.stdout.contains("REDACTED"), "{}", run.stdout);
}

#[test]
fn r3_no_leak_goes_to_the_reviewer() {
    let dir = repo("r3-clean", "\"ac:3\"", "true");
    let run = statute(&dir, &["check"], Some(&fake_gitleaks("clean", 0, "[]")));
    assert_eq!(run.code, 0, "{}", run.stdout);
    let l = line(&run, 3);
    assert!(l.starts_with("inconclusive "), "{l}");
    assert!(l.contains("gitleaks found nothing in the whole history; the parts of the rule outside the repository are for the reviewer"), "{l}");
}

#[test]
fn r3_without_gitleaks_or_with_its_error_is_inconclusive() {
    let dir = repo("r3-missing", "\"ac:3\"", "true");
    // Asked without --redact, the fake exits 9: an error, not a leak.
    let run = statute(&dir, &["check"], Some(&fake_gitleaks("error", 1, "")));
    assert_eq!(run.code, 0);
    assert!(line(&run, 3).starts_with("inconclusive "), "{}", run.stdout);
    assert!(line(&run, 3).contains("gitleaks failed"), "{}", run.stdout);
    let git_only = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("check")
        .join("git-only");
    std::fs::create_dir_all(&git_only).unwrap();
    let run = statute(&dir, &["check"], Some(git_only.to_str().unwrap()));
    assert_eq!(run.code, 0);
    assert!(
        line(&run, 3).contains("gitleaks is not installed"),
        "{}",
        run.stdout
    );
}

/// With the gitleaks on this machine: a token made up here, committed to
/// the repository, is found and its value never printed. The token is
/// built when the test runs, so no file of this repository holds one.
#[test]
fn r3_with_the_real_gitleaks() {
    let installed = std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("gitleaks").is_file()));
    if !installed {
        eprintln!("gitleaks is not installed; this test checks nothing here");
        return;
    }
    let dir = repo("r3-real", "\"ac:3\"", "true");
    let run = statute(&dir, &["check"], None);
    assert!(line(&run, 3).starts_with("inconclusive "), "{}", run.stdout);
    // 36 letters and digits from a fixed linear congruential sequence.
    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut x: u64 = 7;
    let body: String = (0..36)
        .map(|_| {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            alphabet[(x >> 33) as usize % alphabet.len()] as char
        })
        .collect();
    let token = format!("ghp_{body}");
    write(&dir, "cfg.txt", &format!("token = \"{token}\"\n"));
    git(&dir, &["add", "cfg.txt"]);
    git(&dir, &["commit", "-qm", "leak"]);
    let run = statute(&dir, &["check", "--base", "base"], None);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(
        line(&run, 3).contains("in the commits since base"),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("cfg.txt:1"), "{}", run.stdout);
    assert!(!run.stdout.contains(&token) && !run.stderr.contains(&token));
}

// External commands and rules without a check.

#[test]
fn a_command_passes_on_0_and_is_violated_on_1() {
    let dir = repo("command-pass", "\"ac:20\"", "exit 0");
    let run = statute(&dir, &["check"], None);
    assert_eq!(run.code, 0);
    assert!(line(&run, 20).starts_with("pass "), "{}", run.stdout);
    let findings = r#"echo '[{"location": "a.txt:3", "reason": "no newline at the end"}]'; exit 1"#;
    let dir = repo("command-violation", "\"ac:20\"", findings);
    let run = statute(&dir, &["check"], None);
    assert_eq!(run.code, 1);
    assert!(line(&run, 20).starts_with("violation "), "{}", run.stdout);
    assert!(
        run.stdout.contains("a.txt:3: no newline at the end"),
        "{}",
        run.stdout
    );
}

#[test]
fn a_command_that_cannot_decide_is_inconclusive() {
    for (name, command, note) in [
        ("other-status", "exit 2", "could not decide"),
        (
            "not-json",
            "echo oops; exit 1",
            "is not the JSON list of findings",
        ),
    ] {
        let dir = repo(&format!("command-{name}"), "\"ac:20\"", command);
        let run = statute(&dir, &["check"], None);
        assert_eq!(run.code, 0, "{name}");
        let l = line(&run, 20);
        assert!(
            l.starts_with("inconclusive ") && l.contains(note),
            "{name}: {l}"
        );
    }
    let dir = repo("command-tool", "\"ac:20\"", "exit 0");
    let rule = dir.join("rules/20.toml");
    let text = std::fs::read_to_string(&rule).unwrap();
    std::fs::write(
        &rule,
        text.replace("tools = [\"sh\"]", "tools = [\"sh\", \"no-such-tool\"]"),
    )
    .unwrap();
    assert_eq!(statute(&dir, &["lock"], None).code, 0);
    let run = statute(&dir, &["check"], None);
    assert!(
        line(&run, 20).contains("inconclusive  20 (Rule 20): not installed: no-such-tool"),
        "{}",
        run.stdout
    );
}

#[test]
fn a_rule_without_a_check_goes_to_the_reviewer() {
    let dir = repo("review", "\"ac:21\"", "true");
    let run = statute(&dir, &["check"], None);
    assert_eq!(run.code, 0);
    assert!(line(&run, 21).starts_with("review "), "{}", run.stdout);
    assert!(
        run.stdout.contains("## 21 (Rule 21) (review)\n\nReason: Because.\n\nVerification: Violated when the thing is not done.\n"),
        "{}",
        run.stdout
    );
}
