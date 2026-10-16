//! `statute check`, run as a user runs it, in git repositories written
//! under Cargo's test directory.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, rule};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

/// Domain 1 with rules 43, 46, 50, 58, 61, 71 and 280 (built-in checks), rules 3 and 7 (the
/// rules they supersede, whose checks stay), rule 20 (a command) and
/// rule 21 (no program, for the reviewer, whose reason cites a provision
/// and rule 20).
fn nodes(command: &str) -> Vec<(u64, String)> {
    let builtin = |id| edit(rule(id, 1, ""), "check = \"none\"", "check = \"builtin\"");
    vec![
        domain(1),
        builtin(3),
        builtin(7),
        builtin(43),
        builtin(46),
        builtin(50),
        builtin(58),
        builtin(61),
        builtin(71),
        builtin(280),
        edit(
            rule(20, 1, ""),
            "check = \"none\"",
            &format!("check = \"command\"\nrun = {command:?}\ntools = [\"sh\"]"),
        ),
        edit(
            rule(21, 1, "uses = [\"instructions\", \"review\"]"),
            "Because.",
            "Because of {@constitution:principals} and {@rule:20}.",
        ),
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
    repo_at(name, "", rules, command)
}

/// As `repo`, with the rule set and its source in the directory `sub` of
/// the repository; returns that directory.
fn repo_at(name: &str, sub: &str, rules: &str, command: &str) -> PathBuf {
    let top = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("check")
        .join(name);
    let _ = std::fs::remove_dir_all(&top);
    std::fs::create_dir_all(&top).unwrap();
    git(&top, &["init", "-q"]);
    let dir = top.join(sub);
    write(&dir, "constitution/2026-10/index.toml", INDEX);
    common::provisions(&dir.join("constitution/2026-10"));
    write(
        &dir,
        "statute.toml",
        &format!(
            "constitution = {{ source = \"R\", edition = \"2026-10\" }}\npublic = false\nrules = [{rules}]\n\n[sources]\nR = {{ path = \".\" }}\n"
        ),
    );
    for (id, text) in nodes(command) {
        write(&dir, &format!("rules/{id}.toml"), &text);
    }
    for (path, text) in [
        (
            "constitution/2026-10/articles/04-authority/article.toml",
            "id = \"authority\"\n",
        ),
        (
            "constitution/2026-10/articles/04-authority/01-principals.toml",
            "id = \"principals\"\n",
        ),
        ("AGENTS.md", "agents\n"),
        (
            "constitution/2026-10/generated/constitution.en.agent.md",
            "agent constitution\n",
        ),
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
    git(&top, &["add", "-A"]);
    git(&top, &["commit", "-qm", "base"]);
    git(&top, &["tag", "base"]);
    dir
}

/// Changes `files` and commits them as the change on top of `base`: a
/// file that is there gets a comment line, so that statute's own files
/// still load; a new one holds a line.
fn change(dir: &Path, files: &[&str]) {
    for path in files {
        let text = std::fs::read_to_string(dir.join(path)).map_or_else(
            |_| "changed\n".to_owned(),
            |text| format!("{text}# changed\n"),
        );
        write(dir, path, &text);
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
    cmd.env("HOME", concat!(env!("CARGO_TARGET_TMPDIR"), "/home"))
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_CACHE_HOME");
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
        .find(|l| l.contains(&format!("  R{id} (Rule {id}): ")))
        .unwrap_or_else(|| panic!("no line for rule {id}: {}", run.stdout))
        .to_owned()
}

// Rule 46: a change does not mix instruction files with other files.

#[test]
fn r46_mixed_change_is_a_violation() {
    let dir = repo("r46-mixed", "\"R46\"", "true");
    change(&dir, &["AGENTS.md", "tools/build.py"]);
    let run = statute(&dir, &["check", "--base", "base"], None);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(line(&run, 46).starts_with("violation "), "{}", run.stdout);
    assert!(
        run.stdout.contains(
            "M AGENTS.md: an instruction file, not what statute compiles now, in the same change"
        ),
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
fn r46_unmixed_change_goes_to_the_reviewer() {
    let dir = repo("r46-other", "\"R46\"", "true");
    change(
        &dir,
        &[
            "tools/build.py",
            "constitution/2026-10/generated/constitution.en.md",
        ],
    );
    let run = statute(&dir, &["check", "--base", "base"], None);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let l = line(&run, 46);
    assert!(l.starts_with("inconclusive "), "{l}");
    assert!(l.contains("(0 instruction, 1 other, 1 generated)"), "{l}");
    assert!(l.contains("files of other agent tools, sources of instructions outside the path tables, and paths the repository declares in its AGENTS.md are for the reviewer"), "{l}");
    assert!(
        run.stdout.contains(&format!("{l}\n\nReason: Because.\n")),
        "{}",
        run.stdout
    );
}

#[test]
fn r46_without_a_base_is_inconclusive() {
    let dir = repo("r46-no-base", "\"R46\"", "true");
    let run = statute(&dir, &["check"], None);
    assert_eq!(run.code, 0);
    assert!(
        line(&run, 46).contains("give the base of the change with --base"),
        "{}",
        run.stdout
    );
}

/// statute's files beside the rule set are instruction files in any
/// repository, at its top or in a directory of it.
#[test]
fn r46_statute_files_are_instruction_files() {
    for (name, sub, file, status) in [
        ("toml", "", "statute.toml", "M"),
        ("lock", "", "statute.lock", "M"),
        ("local", "", ".statute/notes.md", "A"),
        ("sub-lock", "sub", "statute.lock", "M"),
    ] {
        let dir = repo_at(&format!("r46-statute-{name}"), sub, "\"R46\"", "true");
        change(&dir, &[file, "tools/build.py"]);
        let run = statute(&dir, &["check", "--base", "base"], None);
        assert_eq!(run.code, 1, "{name}: {}", run.stdout);
        let at = if sub.is_empty() {
            file.to_owned()
        } else {
            format!("{sub}/{file}")
        };
        assert!(
            run.stdout.contains(&format!(
                "{status} {at}: an instruction file, in the same change"
            )),
            "{name}: {}",
            run.stdout
        );
    }
}

/// An `AGENTS.md` that equals what statute compiles now is generated, and
/// goes with the tools; beside a rule set in a directory too.
#[test]
fn r46_compiled_files_go_with_the_side_that_changed_them() {
    for sub in ["", "sub"] {
        let dir = repo_at(
            &format!("r46-compiled-{}", sub.len()),
            sub,
            "\"R46\"",
            "true",
        );
        // The base's AGENTS.md is hand-written, which compile leaves alone.
        std::fs::remove_file(dir.join("AGENTS.md")).unwrap();
        let compile = statute(&dir, &["compile"], None);
        assert_eq!(compile.code, 0, "{}", compile.stderr);
        change(&dir, &["tools/build.py"]);
        let run = statute(&dir, &["check", "--base", "base"], None);
        assert_eq!(run.code, 0, "{sub}: {}", run.stdout);
        let l = line(&run, 46);
        assert!(l.starts_with("inconclusive "), "{sub}: {l}");
        let at = |f: &str| {
            if sub.is_empty() {
                f.to_owned()
            } else {
                format!("{sub}/{f}")
            }
        };
        assert!(
            l.contains(&format!(
                "generated files: M {}, which the reviewer checks",
                at("AGENTS.md")
            )),
            "{sub}: {l}"
        );
        // Edited by hand, the compiled file is an instruction file again.
        git(&dir, &["tag", "-f", "compiled"]);
        change(&dir, &["AGENTS.md", "tools/build.py"]);
        let run = statute(&dir, &["check", "--base", "compiled"], None);
        assert_eq!(run.code, 1, "{sub}: {}", run.stdout);
        assert!(
            run.stdout.contains(&format!(
                "M {}: an instruction file, not what statute compiles now, in the same change",
                at("AGENTS.md")
            )),
            "{sub}: {}",
            run.stdout
        );
    }
}

/// A lock file that changed in nothing but the version of statute, beside
/// tools, is an instruction file in the same change for rule 46, as in
/// any repository; only this repository's rule L31 counts it as
/// generated, in `check-instruction-changes`. So is the deletion of the
/// checks file an earlier statute wrote.
#[test]
fn r46_a_lock_file_changed_only_in_its_version_stays_an_instruction_file() {
    let version = format!("statute = \"{}\"", env!("CARGO_PKG_VERSION"));
    let dir = repo("r46-lock-version", "\"R46\"", "true");
    std::fs::remove_file(dir.join("AGENTS.md")).unwrap();
    let compile = statute(&dir, &["compile"], None);
    assert_eq!(compile.code, 0, "{}", compile.stderr);
    // The base as an earlier statute left it.
    let lock = std::fs::read_to_string(dir.join("statute.lock")).unwrap();
    assert!(lock.contains(&version), "{lock}");
    write(
        &dir,
        "statute.lock",
        &lock.replace(&version, "statute = \"0.0.1\""),
    );
    let old = format!(
        "{}\n\nstatute = \"0.0.1\"\ncommands = []\n",
        statute::compile::GENERATED_HASH
    );
    write(&dir, "statute-checks.toml", &old);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-qm", "old"]);
    git(&dir, &["tag", "old"]);
    let relock = statute(&dir, &["lock"], None);
    assert_eq!(relock.code, 0, "{}", relock.stderr);
    let compile = statute(&dir, &["compile"], None);
    assert_eq!(compile.code, 0, "{}", compile.stderr);
    assert!(!dir.join("statute-checks.toml").exists());
    change(&dir, &["tools/build.py"]);
    let run = statute(&dir, &["check", "--base", "old"], None);
    assert_eq!(run.code, 1, "{}", run.stdout);
    for file in ["statute.lock", "statute-checks.toml"] {
        let status = if file == "statute.lock" { "M" } else { "D" };
        assert!(
            run.stdout.contains(&format!(
                "{status} {file}: an instruction file, in the same change"
            )),
            "{file}: {}",
            run.stdout
        );
    }
}

/// The same changes, judged by statute's rule 46 and by this repository's
/// `check-instruction-changes`: a violation exactly where the Python check
/// exits 1. The cases leave out where the two differ by design: the
/// script, with this repository's tables, counts the top `AGENTS.md` as
/// generated always and the top `statute.lock` when its TOML differs only
/// in `statute` and `commands`, as this repository's rule L31 provides,
/// and `statute-checks.toml` always, as its rule L27 provides, while
/// statute's check, which holds for any repository, counts the
/// `AGENTS.md` only while it equals its compile (the hand-edited
/// `AGENTS.md` of `r46_mixed_change_is_a_violation`) and the other two
/// as instruction files; and the script
/// knows statute's files only at the top of the repository.
#[test]
fn r46_agrees_with_check_instruction_changes() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cases: &[(&str, &[&str])] = &[
        ("instructions", &["AGENTS.md", "skills/a/SKILL.md"]),
        ("other", &["tools/build.py", "docs/design/README.md"]),
        ("mixed", &["CLAUDE.md", "tools/build.py"]),
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
                "constitution/2026-10/articles/04-authority/article.toml",
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
        ("statute-toml", &["statute.toml", "tools/build.py"]),
        ("statute-local", &[".statute/notes.md", "tools/build.py"]),
        ("legacy", &["docs/CLAUDE.legacy.md", "tools/build.py"]),
    ];
    let mut seen = (0, 0);
    for (name, files) in cases {
        let dir = repo(&format!("r46-parity-{name}"), "\"R46\"", "true");
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
            .unwrap_or_else(|e| {
                panic!("cannot run uv ({e}); install uv, then run `uv sync` first")
            });
        let python_code = python.status.code().unwrap();
        assert!(
            python_code == 0 || python_code == 1,
            "{name}: check-instruction-changes could not run; run `uv sync` first: {}",
            String::from_utf8_lossy(&python.stderr)
        );
        if python_code == 1 {
            seen.1 += 1;
        } else {
            seen.0 += 1;
        }
        assert_eq!(
            line(&ours, 46).starts_with("violation "),
            python_code == 1,
            "{name}: statute says {:?}, check-instruction-changes exits {python_code}",
            line(&ours, 46)
        );
    }
    // Both conclusions occur among the cases, so the comparison can fail.
    assert!(seen.0 > 0 && seen.1 > 0, "{seen:?}");
    // A rename is a removal and an addition: AGENTS.md to docs/agents.md
    // mixes the two in both.
    let dir = repo("r46-parity-rename", "\"R46\"", "true");
    git(&dir, &["mv", "AGENTS.md", "docs/agents.md"]);
    git(&dir, &["commit", "-qm", "rename"]);
    let ours = statute(&dir, &["check", "--base", "base"], None);
    assert!(line(&ours, 46).starts_with("violation "), "{}", ours.stdout);
}

// Rule 280: rule 46, with the migration change as its one exception.

/// A repository whose base, tagged `pre`, has no rule set, and a change
/// that adds it with `files` changed beside it: a migration change.
fn migration(name: &str, rules: &str, files: &[&str]) -> PathBuf {
    let dir = repo(name, rules, "true");
    git(&dir, &["rm", "-q", "statute.toml", "statute.lock"]);
    git(&dir, &["commit", "-qm", "before statute"]);
    git(&dir, &["tag", "pre"]);
    git(
        &dir,
        &[
            "checkout",
            "-q",
            "base",
            "--",
            "statute.toml",
            "statute.lock",
        ],
    );
    change(&dir, files);
    dir
}

#[test]
fn r280_migration_change_goes_to_the_reviewer() {
    let dir = migration("r280-migration", "\"R280\"", &[".pre-commit-config.yaml"]);
    let run = statute(&dir, &["check", "--base", "pre"], None);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let l = line(&run, 280);
    assert!(l.starts_with("inconclusive "), "{l}");
    assert!(
        l.contains(
            "adds statute.toml, and its base has none in any directory: it is the migration change"
        ),
        "{l}"
    );
    assert!(
        run.stdout
            .contains("A .pre-commit-config.yaml: another file, in the migration change"),
        "{}",
        run.stdout
    );
    // Rule 46 has no exception: the same change is a violation.
    let dir = migration(
        "r280-migration-r46",
        "\"R46\"",
        &[".pre-commit-config.yaml"],
    );
    let run = statute(&dir, &["check", "--base", "pre"], None);
    assert!(line(&run, 46).starts_with("violation "), "{}", run.stdout);
}

#[test]
fn r280_other_changes_mix_as_under_rule_46() {
    let dir = repo("r280-mixed", "\"R280\"", "true");
    change(&dir, &["statute.toml", "tools/build.py"]);
    let run = statute(&dir, &["check", "--base", "base"], None);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(line(&run, 280).starts_with("violation "), "{}", run.stdout);
    // Moving a rule set into a directory adds its statute.toml there, and
    // is no migration: the base has one.
    let dir = repo("r280-moved", "\"R280\"", "true");
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    for path in ["statute.toml", "statute.lock", "rules", "constitution"] {
        git(&dir, &["mv", path, &format!("sub/{path}")]);
    }
    change(&dir, &["tools/build.py"]);
    let run = statute(&dir.join("sub"), &["check", "--base", "base"], None);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(line(&run, 280).starts_with("violation "), "{}", run.stdout);
}

// Rule 43: credentials are stored only where designated; gitleaks sees the
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
fn r43_a_leak_is_a_violation_with_its_place_only() {
    let dir = repo("r43-leak", "\"R43\"", "true");
    let report = r#"[{"File":"cfg.txt","StartLine":1,"RuleID":"github-pat","Commit":"abc123","Secret":"REDACTED","Match":"REDACTED"}]"#;
    let run = statute(&dir, &["check"], Some(&fake_gitleaks("leak", 3, report)));
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(line(&run, 43).starts_with("violation "), "{}", run.stdout);
    assert!(
        run.stdout
            .contains("cfg.txt:1 (commit abc123): gitleaks rule github-pat"),
        "{}",
        run.stdout
    );
    assert!(!run.stdout.contains("REDACTED"), "{}", run.stdout);
}

#[test]
fn r43_no_leak_goes_to_the_reviewer() {
    let dir = repo("r43-clean", "\"R43\"", "true");
    let run = statute(&dir, &["check"], Some(&fake_gitleaks("clean", 0, "[]")));
    assert_eq!(run.code, 0, "{}", run.stdout);
    let l = line(&run, 43);
    assert!(l.starts_with("inconclusive "), "{l}");
    assert!(l.contains("gitleaks found nothing in the whole history; credentials gitleaks does not recognize, and places outside the repository, are for the reviewer"), "{l}");
}

#[test]
fn r43_without_gitleaks_or_with_its_error_is_inconclusive() {
    let dir = repo("r43-missing", "\"R43\"", "true");
    // Asked without --redact, the fake exits 9: an error, not a leak.
    let run = statute(&dir, &["check"], Some(&fake_gitleaks("error", 1, "")));
    assert_eq!(run.code, 0);
    assert!(
        line(&run, 43).starts_with("inconclusive "),
        "{}",
        run.stdout
    );
    assert!(line(&run, 43).contains("gitleaks failed"), "{}", run.stdout);
    let git_only = git_only("git-without-gitleaks");
    let run = statute(&dir, &["check"], Some(git_only.to_str().unwrap()));
    assert_eq!(run.code, 0);
    assert!(
        line(&run, 43).contains("gitleaks is not installed"),
        "{}",
        run.stdout
    );
}

/// With the gitleaks on this machine: a token made up here, committed to
/// the repository, is found and its value never printed. The token is
/// built when the test runs, so no file of this repository holds one.
#[test]
fn r43_with_the_real_gitleaks() {
    let installed = std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("gitleaks").is_file()));
    if !installed {
        eprintln!("gitleaks is not installed; this test checks nothing here");
        return;
    }
    let dir = repo("r43-real", "\"R43\"", "true");
    let run = statute(&dir, &["check"], None);
    assert!(
        line(&run, 43).starts_with("inconclusive "),
        "{}",
        run.stdout
    );
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
        line(&run, 43).contains("in the commits since base"),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("cfg.txt:1"), "{}", run.stdout);
    assert!(!run.stdout.contains(&token) && !run.stderr.contains(&token));
}

// Rule 50: the commit hooks scan the staged changes for credentials.

/// gitleaks' own hook, as this repository configures it.
const GITLEAKS_HOOK: &str = "  - repo: https://github.com/gitleaks/gitleaks\n    rev: v8.30.1\n    hooks:\n      - id: gitleaks\n        args: [\"--redact\", \"--no-banner\", \".\"]\n";

/// Runs `statute check` with rule 50 on a pre-commit configuration.
fn r50(name: &str, config: &str) -> Run {
    let dir = repo(&format!("r50-{name}"), "\"R50\"", "true");
    write(&dir, ".pre-commit-config.yaml", config);
    statute(&dir, &["check"], None)
}

#[test]
fn r50_gitleaks_on_the_staged_changes_passes() {
    let run = r50("pass", &format!("repos:\n{GITLEAKS_HOOK}"));
    assert_eq!(run.code, 0, "{}", run.stdout);
    let l = line(&run, 50);
    assert!(l.starts_with("pass "), "{l}");
    assert!(l.contains("hook gitleaks of https://github.com/gitleaks/gitleaks in .pre-commit-config.yaml runs gitleaks on the staged changes, with --redact, at every commit"), "{l}");
}

#[test]
fn r50_a_scan_only_at_push_time_is_a_violation() {
    let config = format!(
        "default_stages: [pre-push]\nrepos:\n{GITLEAKS_HOOK}  - repo: local\n    hooks:\n      - id: fmt\n        entry: cargo fmt --check\n        language: system\n"
    );
    let run = r50("push", &config);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(line(&run, 50).starts_with("violation "), "{}", run.stdout);
    assert!(
        run.stdout.contains(".pre-commit-config.yaml: hook gitleaks of https://github.com/gitleaks/gitleaks: runs only at pre-push, not at the commit"),
        "{}",
        run.stdout
    );
}

#[test]
fn r50_reads_the_configuration_at_the_top_of_the_repository() {
    // The rule set is in a subdirectory; the hooks are configured at the
    // top, and a configuration next to the rule set is not the
    // repository's.
    let dir = repo_at("r50-sub", "sub", "\"R50\"", "true");
    let top = dir.parent().unwrap();
    write(
        top,
        ".pre-commit-config.yaml",
        &format!("repos:\n{GITLEAKS_HOOK}"),
    );
    let run = statute(&dir, &["check"], None);
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(line(&run, 50).starts_with("pass "), "{}", run.stdout);
    std::fs::rename(
        top.join(".pre-commit-config.yaml"),
        dir.join(".pre-commit-config.yaml"),
    )
    .unwrap();
    let run = statute(&dir, &["check"], None);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(line(&run, 50).starts_with("violation "), "{}", run.stdout);
}

#[test]
fn a_built_in_check_is_the_first_party_rules_only() {
    // The first-party source is in `r/`, so `rules/` holds the
    // repository's own rules; its rule 50 asks for a built-in check too.
    let dir = repo("builtin-local", "", "true");
    let first_party = dir.join("r");
    std::fs::create_dir_all(&first_party).unwrap();
    for entry in ["rules", "constitution"] {
        std::fs::rename(dir.join(entry), first_party.join(entry)).unwrap();
    }
    for (id, text) in [
        domain(1),
        edit(rule(50, 1, ""), "check = \"none\"", "check = \"builtin\""),
    ] {
        write(&dir, &format!("rules/{id}.toml"), &text);
    }
    write(
        &dir,
        "statute.toml",
        "constitution = { source = \"R\", edition = \"2026-10\" }\npublic = false\nlocal-rules = \"rules\"\nrules = [\"R50\", \"50\"]\n\n[sources]\nR = { path = \"r\" }\n",
    );
    write(
        &dir,
        ".pre-commit-config.yaml",
        &format!("repos:\n{GITLEAKS_HOOK}"),
    );
    assert_eq!(statute(&dir, &["lock"], None).code, 0);
    let run = statute(&dir, &["check"], None);
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(line(&run, 50).starts_with("pass "), "{}", run.stdout);
    let local = run
        .stdout
        .lines()
        .find(|l| l.contains("  L50 (Rule 50): "))
        .unwrap_or_else(|| panic!("no line for L50: {}", run.stdout));
    assert!(
        local.starts_with("inconclusive  L50 (Rule 50): statute has built-in checks only for rules of the first-party source, `R`, so none for L50"),
        "{local}"
    );
}

#[test]
fn r50_another_scanner_goes_to_the_reviewer() {
    let config = "repos:\n  - repo: https://example.test/Yelp/detect-secrets\n    rev: v1.5.0\n    hooks:\n      - id: detect-secrets\n";
    let run = r50("other", config);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let l = line(&run, 50);
    assert!(l.starts_with("inconclusive "), "{l}");
    assert!(
        l.contains("runs detect-secrets, which statute does not judge"),
        "{l}"
    );
}

#[test]
fn r50_gitleaks_under_its_former_name_passes() {
    let config = format!(
        "repos:\n{}",
        GITLEAKS_HOOK
            .replace("gitleaks/gitleaks", "zricethezav/gitleaks")
            .replace("v8.30.1", "v8.18.0")
    );
    let run = r50("former", &config);
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(line(&run, 50).starts_with("pass "), "{}", run.stdout);
}

#[test]
fn r50_gitleaks_before_its_staged_entry_goes_to_the_reviewer() {
    let config = format!("repos:\n{}", GITLEAKS_HOOK.replace("v8.30.1", "v8.1.0"));
    let run = r50("old", &config);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let l = line(&run, 50);
    assert!(l.starts_with("inconclusive "), "{l}");
    assert!(
        l.contains("is gitleaks at v8.1.0, whose entry statute does not know"),
        "{l}"
    );
}

#[test]
fn r50_a_hook_of_another_repository_goes_to_the_reviewer() {
    let config = "repos:\n  - repo: https://example.test/acme/secret-scan\n    rev: v1.0.0\n    hooks:\n      - id: scan\n";
    let run = r50("unknown", config);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let l = line(&run, 50);
    assert!(l.starts_with("inconclusive "), "{l}");
    assert!(
        l.contains("hook scan of https://example.test/acme/secret-scan has its entry in its repository, which statute does not read"),
        "{l}"
    );
}

// Rules 58, 61 and 71 read GitHub through gh; a fake gh on PATH answers
// each call from a file of its own directory, and a call it has no file
// for fails as gh does.

/// A fake gh answering with `answers`, keyed by the file each kind of
/// call reads; the PATH to run statute with.
fn fake_gh(name: &str, answers: &[(&str, &str)]) -> String {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("check")
        .join(format!("fake-gh-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (file, text) in answers {
        std::fs::write(dir.join(file), text).unwrap();
    }
    let script = dir.join("gh");
    std::fs::write(
        &script,
        format!(
            r#"#!/bin/sh
d='{}'
case "$*" in
  "api repos/{{owner}}/{{repo}}") f=repo ;;
  *"/rules/branches/main") f=rules ;;
  *"/branches/main") f=branch ;;
  *"/check-runs"*) f=check-runs ;;
  *"/actions/runs"*) f=runs ;;
  *"/commits/"*"/pulls") f=pulls ;;
  "api graphql"*) f=graphql ;;
  "pr view"*) f=pr ;;
  "repo view --json visibility") f=visibility ;;
  *) f=none ;;
esac
[ -f "$d/$f" ] || {{ echo "HTTP 404: Not Found ($f)" >&2; exit 1; }}
cat "$d/$f"
"#,
            dir.display()
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

fn head(dir: &Path) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

/// The answers of a repository whose main requires the check `check`, of
/// the workflow run in check suite 5, with that run's attempt and
/// conclusion; check suite 6 is another workflow, which failed.
fn ci(attempt: u64, conclusion: &str) -> Vec<(&'static str, String)> {
    let run = |suite, conclusion: &str, attempt| {
        format!(
            r#"{{"name":"W{suite}","html_url":"https://example.test/runs/{suite}","check_suite_id":{suite},"run_attempt":{attempt},"status":"completed","conclusion":"{conclusion}"}}"#
        )
    };
    vec![
        ("repo", r#"{"default_branch":"main"}"#.to_owned()),
        (
            "branch",
            r#"{"protection":{"required_status_checks":{"contexts":["check"]}}}"#.to_owned(),
        ),
        ("rules", "[]".to_owned()),
        (
            "check-runs",
            r#"{"check_runs":[{"name":"check","check_suite":{"id":5}},{"name":"deploy","check_suite":{"id":6}}]}"#
                .to_owned(),
        ),
        (
            "runs",
            format!(
                r#"{{"workflow_runs":[{},{}]}}"#,
                run(5, conclusion, attempt),
                run(6, "failure", 1)
            ),
        ),
    ]
}

fn answers<'a>(owned: &'a [(&'static str, String)]) -> Vec<(&'static str, &'a str)> {
    owned.iter().map(|(f, t)| (*f, t.as_str())).collect()
}

#[test]
fn r58_a_tagged_commit_green_on_its_first_attempt_passes() {
    let dir = repo("r58-pass", "\"R58\"", "true");
    git(&dir, &["tag", "v1"]);
    let path = fake_gh("r58-pass", &answers(&ci(1, "success")));
    let run = statute(&dir, &["check"], Some(&path));
    assert_eq!(run.code, 0, "{}", run.stdout);
    let l = line(&run, 58);
    assert!(l.starts_with("pass "), "{l}");
    assert!(l.contains("tagged base, v1"), "{l}");
    assert!(
        l.contains("succeeded on attempt 1 (https://example.test/runs/5)"),
        "{l}"
    );
}

#[test]
fn r58_a_tagged_commit_green_only_on_a_rerun_is_a_violation() {
    let dir = repo("r58-rerun", "\"R58\"", "true");
    let path = fake_gh("r58-rerun", &answers(&ci(2, "success")));
    let run = statute(&dir, &["check"], Some(&path));
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(line(&run, 58).starts_with("violation "), "{}", run.stdout);
    assert!(
        run.stdout
            .contains("https://example.test/runs/5: W5 concluded success on attempt 2"),
        "{}",
        run.stdout
    );
    assert!(!run.stdout.contains("runs/6"), "{}", run.stdout);
}

#[test]
fn r71_a_pull_request_never_set_to_merge_automatically_goes_to_the_reviewer() {
    let dir = repo("r71-clean", "\"R71\"", "true");
    let path = fake_gh(
        "r71-clean",
        &[
            ("pulls", r#"[{"number":7}]"#),
            (
                "graphql",
                r#"{"data":{"repository":{"pullRequest":{"timelineItems":{"nodes":[]}}}}}"#,
            ),
        ],
    );
    let run = statute(&dir, &["check"], Some(&path));
    assert_eq!(run.code, 0, "{}", run.stdout);
    let l = line(&run, 71);
    assert!(l.starts_with("inconclusive "), "{l}");
    assert!(
        l.contains("auto-merge was never enabled on pull request #7"),
        "{l}"
    );
}

#[test]
fn r71_auto_merge_is_a_violation() {
    let dir = repo("r71-auto", "\"R71\"", "true");
    let path = fake_gh(
        "r71-auto",
        &[
            ("pulls", r#"[{"number":7}]"#),
            (
                "graphql",
                r#"{"data":{"repository":{"pullRequest":{"timelineItems":{"nodes":[{"createdAt":"2026-10-01T00:00:00Z"}]}}}}}"#,
            ),
        ],
    );
    let run = statute(&dir, &["check"], Some(&path));
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(line(&run, 71).starts_with("violation "), "{}", run.stdout);
    assert!(
        run.stdout
            .contains("pull request #7: auto-merge enabled at 2026-10-01T00:00:00Z"),
        "{}",
        run.stdout
    );
}

/// A pull request #7 holding HEAD, with the given comment bodies and an
/// approval without text, which GitHub records on HEAD.
fn r61_answers(dir: &Path, comments: &[&str]) -> Vec<(&'static str, String)> {
    let sha = head(dir);
    let comments: Vec<String> = comments
        .iter()
        .enumerate()
        .map(|(i, body)| {
            format!(
                r#"{{"body":{},"url":"https://example.test/pull/7#c{i}"}}"#,
                serde_json::to_string(body).unwrap()
            )
        })
        .collect();
    vec![
        ("pulls", r#"[{"number":7}]"#.to_owned()),
        (
            "pr",
            format!(
                r#"{{"commits":[{{"oid":"{sha}"}}],"comments":[{}],"reviews":[{{"body":"","state":"APPROVED","submittedAt":"2026-10-01T00:00:00Z","commit":{{"oid":"{sha}"}}}}]}}"#,
                comments.join(",")
            ),
        ),
    ]
}

#[test]
fn r61_verdicts_naming_a_commit_go_to_the_reviewer() {
    let dir = repo("r61-named", "\"R61\"", "true");
    let sha = head(&dir);
    let owned = r61_answers(
        &dir,
        &[
            &format!("Verdict: LGTM\nReviewed head: `{sha}`\n"),
            "Thanks, fixed.",
        ],
    );
    let run = statute(
        &dir,
        &["check"],
        Some(&fake_gh("r61-named", &answers(&owned))),
    );
    assert_eq!(run.code, 0, "{}", run.stdout);
    let l = line(&run, 61);
    assert!(l.starts_with("inconclusive "), "{l}");
    assert!(
        l.contains(
            "each of the 2 verdict(s) on pull request #7 names a commit of its pull request"
        ),
        "{l}"
    );
}

#[test]
fn r61_a_verdict_without_a_full_sha_is_a_violation() {
    let dir = repo("r61-unnamed", "\"R61\"", "true");
    let short = &head(&dir)[..7];
    let owned = r61_answers(&dir, &[&format!("Verdict: LGTM\nReviewed head: {short}\n")]);
    let run = statute(
        &dir,
        &["check"],
        Some(&fake_gh("r61-unnamed", &answers(&owned))),
    );
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(line(&run, 61).starts_with("violation "), "{}", run.stdout);
    assert!(
        run.stdout.contains(
            "https://example.test/pull/7#c0: the verdict names no commit of the pull request by its full SHA"
        ),
        "{}",
        run.stdout
    );
}

/// The line of `statute check` on the rule set's `public`.
fn visibility(run: &Run) -> String {
    run.stdout
        .lines()
        .find(|l| l.contains("  public: "))
        .unwrap_or_else(|| panic!("no line for the visibility: {}", run.stdout))
        .to_owned()
}

#[test]
fn a_rule_set_that_says_private_in_a_public_repository_is_a_violation() {
    let dir = repo("visibility", "\"R21\"", "true");
    let answer = |v: &str| format!("{{\"visibility\":\"{v}\"}}");
    let run = statute(
        &dir,
        &["check"],
        Some(&fake_gh("public", &[("visibility", &answer("PUBLIC"))])),
    );
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert_eq!(
        visibility(&run),
        "violation     public: statute.toml says `public = false`, and GitHub shows the repository as PUBLIC"
    );
    assert!(
        run.stdout.contains("statute.toml: `public = false`, and anyone can read the repository; write `public = true`"),
        "{}",
        run.stdout
    );
    let run = statute(
        &dir,
        &["check"],
        Some(&fake_gh("private", &[("visibility", &answer("PRIVATE"))])),
    );
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(visibility(&run).starts_with("pass "), "{}", run.stdout);
    // Saying public in a repository GitHub shows as public is right, and
    // in a private one stricter than needed.
    let toml = std::fs::read_to_string(dir.join("statute.toml")).unwrap();
    write(
        &dir,
        "statute.toml",
        &toml.replace("public = false", "public = true"),
    );
    for v in ["PUBLIC", "PRIVATE"] {
        let run = statute(
            &dir,
            &["check"],
            Some(&fake_gh(
                &format!("declared-{v}"),
                &[("visibility", &answer(v))],
            )),
        );
        assert_eq!(run.code, 0, "{v}: {}", run.stdout);
        assert!(visibility(&run).starts_with("pass "), "{v}: {}", run.stdout);
    }
    // gh cannot answer.
    let run = statute(&dir, &["check"], Some(&fake_gh("no-answer", &[])));
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(
        visibility(&run).starts_with("inconclusive  public: statute.toml says `public = true`, and GitHub was not asked: gh repo failed"),
        "{}",
        run.stdout
    );
}

/// A directory `name` for `PATH` that holds git and nothing else, which
/// statute needs to find the private rule set.
fn git_only(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("check")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let git_bin = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|d| d.join("git"))
        .find(|p| p.is_file())
        .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(git_bin, dir.join("git")).unwrap();
    dir
}

#[test]
fn without_gh_or_with_its_error_the_github_checks_are_inconclusive() {
    let dir = repo("gh-missing", "\"R58\", \"R61\", \"R71\"", "true");
    // git on PATH, gh not.
    let git_only = git_only("git-without-gh");
    let run = statute(&dir, &["check"], Some(git_only.to_str().unwrap()));
    assert_eq!(run.code, 0, "{}", run.stdout);
    for id in [58, 61, 71] {
        let l = line(&run, id);
        assert!(l.contains("gh is not installed"), "{l}");
    }
    // A gh that answers nothing, as without a login.
    let run = statute(&dir, &["check"], Some(&fake_gh("gh-error", &[])));
    assert_eq!(run.code, 0, "{}", run.stdout);
    for id in [58, 61, 71] {
        let l = line(&run, id);
        assert!(
            l.starts_with("inconclusive ") && l.contains("failed"),
            "{l}"
        );
    }
}

// The rules 43 and 46 supersede keep their checks.

#[test]
fn rules_3_and_7_run_their_own_checks() {
    let dir = repo("superseded", "\"R3\", \"R7\"", "true");
    let git_only = git_only("git-for-superseded");
    let run = statute(&dir, &["check"], Some(git_only.to_str().unwrap()));
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(
        line(&run, 3).contains("gitleaks is not installed"),
        "{}",
        run.stdout
    );
    assert!(
        line(&run, 7).contains("give the base of the change with --base"),
        "{}",
        run.stdout
    );
}

// External commands and rules without a check.

#[test]
fn a_command_passes_on_0_and_is_violated_on_1() {
    let dir = repo("command-pass", "\"R20\"", "exit 0");
    let run = statute(&dir, &["check"], None);
    assert_eq!(run.code, 0);
    assert!(line(&run, 20).starts_with("pass "), "{}", run.stdout);
    let findings = r#"echo '[{"location": "a.txt:3", "reason": "no newline at the end"}]'; exit 1"#;
    let dir = repo("command-violation", "\"R20\"", findings);
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
        let dir = repo(&format!("command-{name}"), "\"R20\"", command);
        let run = statute(&dir, &["check"], None);
        assert_eq!(run.code, 0, "{name}");
        let l = line(&run, 20);
        assert!(
            l.starts_with("inconclusive ") && l.contains(note),
            "{name}: {l}"
        );
    }
    let dir = repo("command-tool", "\"R20\"", "exit 0");
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
        line(&run, 20).contains("inconclusive  R20 (Rule 20): not installed: no-such-tool"),
        "{}",
        run.stdout
    );
}

#[test]
fn a_rule_without_a_check_goes_to_the_reviewer() {
    let dir = repo("review", "\"R21\"", "true");
    let run = statute(&dir, &["check"], None);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(line(&run, 21).starts_with("review "), "{}", run.stdout);
    // No citation is left as written; each is rendered, as in the
    // compiled AGENTS.md.
    assert!(!run.stdout.contains("{@"), "{}", run.stdout);
    assert!(
        run.stdout.contains(&format!("{}\n\nReason: Because of Art. IV §1 and Rule 20 (rule R20).\n\nVerification: Violated when the thing is not done.\n", line(&run, 21))),
        "{}",
        run.stdout
    );
}

#[test]
fn a_rule_without_a_check_or_review_is_not_listed() {
    let dir = repo("not-for-review", "\"R20\"", "true");
    write(&dir, "rules/23.toml", &rule(23, 1, "").1);
    let manifest = std::fs::read_to_string(dir.join("statute.toml")).unwrap();
    write(
        &dir,
        "statute.toml",
        &manifest.replace("\"R20\"]", "\"R20\", \"R23\"]"),
    );
    assert_eq!(statute(&dir, &["lock"], None).code, 0);
    let run = statute(&dir, &["check"], None);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(line(&run, 20).starts_with("pass "), "{}", run.stdout);
    assert!(!run.stdout.contains("R23"), "{}", run.stdout);
}

#[test]
fn the_rules_are_listed_by_domain() {
    let dir = repo("by-domain", "\"R20\", \"R21\"", "true");
    for (id, text) in [domain(2), rule(22, 2, "uses = [\"review\"]")] {
        write(&dir, &format!("rules/{id}.toml"), &text);
    }
    let manifest = std::fs::read_to_string(dir.join("statute.toml")).unwrap();
    write(
        &dir,
        "statute.toml",
        &manifest.replace("\"R21\"]", "\"R21\", \"R22\"]"),
    );
    assert_eq!(statute(&dir, &["lock"], None).code, 0);
    let run = statute(&dir, &["check"], None);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let r20 = line(&run, 20);
    assert!(r20.starts_with("pass "), "{}", run.stdout);
    // Rule 20 passed, so it comes without its reason; rule 21 is for
    // review, so its reason follows it; rule 22 is under domain 2.
    let visibility = run
        .stdout
        .lines()
        .find(|l| l.contains("  public: statute.toml says `public = false`"))
        .unwrap_or_else(|| panic!("no line for the visibility: {}", run.stdout));
    let expected = format!(
        "## statute.toml\n\n{visibility}\n\n## R1 (Domain 1)\n\n{r20}\n\n{}\n\nReason: Because of Art. IV §1 and Rule 20 (rule R20).\n\nVerification: Violated when the thing is not done.\n\n## R2 (Domain 2)\n\n{}\n\nReason: Because.\n\nVerification: Violated when the thing is not done.\n",
        line(&run, 21),
        line(&run, 22),
    );
    assert_eq!(run.stdout, expected);
}
