//! `statute site`, end to end: a git repository whose sources fill the
//! four quadrants, and one that is itself the first-party source.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, rule};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn read(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join(path)).unwrap()
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// A git repository with the constitution of the first-party source in
/// `r/`, or at the top when `r` is `""`.
fn repo(name: &str, r: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("site")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q"]);
    write(&dir, &format!("{r}constitution/2026-10/index.toml"), INDEX);
    write(
        &dir,
        &format!("{r}constitution/2026-10/articles/04-authority/article.toml"),
        "id = \"authority\"\n",
    );
    dir
}

fn put(dir: &Path, sub: &str, (id, text): (u64, String)) {
    write(dir, &format!("{sub}/{id}.toml"), &text);
}

/// A public repository with a rule in each quadrant: `R` in `r/`, its
/// own rules, and in its private layer `me`, global, and `here`, local.
/// The private rule 4 cites R2.
fn four(name: &str) -> PathBuf {
    let dir = repo(name, "r/");
    write(
        &dir,
        "statute.toml",
        "constitution = { source = \"R\", edition = \"2026-10\" }\npublic = true\nlocal-rules = \"rules\"\nrules = [\"R2\", \"3\"]\n\n[sources]\nR = { path = \"r\" }\n",
    );
    write(
        &dir,
        "statute.private.toml",
        "base = \"statute.toml\"\nrules = [\"me:4\"]\n\n[sources]\nme = { path = \"me\", public = false }\nhere = { path = \"here\", public = false, scope = \"local\" }\n",
    );
    write(
        &dir,
        ".gitignore",
        "/statute.private.toml\n/statute.private.lock\n/.statute/\n/me/\n/here/\n",
    );
    put(&dir, "r/rules", domain(1));
    put(&dir, "r/rules", rule(2, 1, ""));
    put(&dir, "rules", domain(1));
    put(&dir, "rules", rule(3, 1, ""));
    put(&dir, "me/rules", domain(1));
    put(
        &dir,
        "me/rules",
        edit(
            edit(
                rule(4, 1, ""),
                "Do the thing 4.",
                "Keep the secret, after {@rule:R2}.",
            ),
            "Rule 4\"",
            "Private rule 4\"",
        ),
    );
    put(&dir, "here/rules", domain(1));
    put(&dir, "here/rules", rule(5, 1, ""));
    dir
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn statute(dir: &Path, args: &[&str]) -> Run {
    statute_with(dir, args, &[])
}

/// `statute` with `env` added to its environment.
fn statute_with(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_statute"))
        .env("HOME", concat!(env!("CARGO_TARGET_TMPDIR"), "/home"))
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_CACHE_HOME")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .envs(env.iter().copied())
        .output()
        .unwrap();
    Run {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

/// The section of a quadrant in the index.
fn section<'a>(index: &'a str, class: &str) -> &'a str {
    let start = format!("<section class=\"{class}\">");
    let rest = &index[index.find(&start).unwrap()..];
    &rest[..rest.find("</section>").unwrap()]
}

/// Every file below `dir`, relative to it, but `.git`.
fn files(dir: &Path) -> BTreeSet<PathBuf> {
    let mut out = BTreeSet::new();
    let mut todo = vec![PathBuf::new()];
    while let Some(rel) = todo.pop() {
        for entry in std::fs::read_dir(dir.join(&rel)).unwrap() {
            let entry = entry.unwrap();
            let path = rel.join(entry.file_name());
            if path == Path::new(".git") {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                todo.push(path);
            } else {
                out.insert(path);
            }
        }
    }
    out
}

#[test]
fn site_shows_the_four_quadrants() {
    let dir = four("four");
    let r = statute(&dir, &["site"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert!(
        r.stdout.contains("wrote 9 pages to .statute/site"),
        "{}",
        r.stdout
    );
    let index = read(&dir, ".statute/site/index.html");
    for (class, source, title) in [
        ("global-public", "R", "Rule 2"),
        ("global-private", "me", "Private rule 4"),
        ("local-public", "L", "Rule 3"),
        ("local-private", "here", "Rule 5"),
    ] {
        let s = section(&index, class);
        assert!(
            s.contains(&format!("id=\"source-{source}\"")),
            "{class}: {s}"
        );
        assert!(s.contains(title), "{class}: {s}");
        // No source is listed in another quadrant.
        assert_eq!(index.matches(&format!("id=\"source-{source}\"")).count(), 1);
    }
    // Rule 4 is enabled by the private layer, and its citation is a link.
    let page = read(&dir, ".statute/site/me/4.html");
    assert!(page.contains("Keep the secret"), "{page}");
    assert!(page.contains("<a href=\"../R/2.html\">"), "{page}");
    assert!(page.contains("class=\"tag on\">enabled"), "{page}");
    // Rule 5 is not.
    assert!(read(&dir, ".statute/site/here/5.html").contains("not enabled"));
}

#[test]
fn site_writes_nothing_else_and_git_ignores_it() {
    let dir = four("ignored");
    let before = files(&dir);
    let status = git(&dir, &["status", "--porcelain", "--untracked-files=all"]);
    let r = statute(&dir, &["site"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let after = files(&dir);
    let new: Vec<&PathBuf> = after.difference(&before).collect();
    assert!(!new.is_empty());
    for path in &new {
        assert!(path.starts_with(".statute/site"), "{}", path.display());
    }
    // The private text is in the site, and nowhere else.
    for path in &after {
        let text = std::fs::read_to_string(dir.join(path)).unwrap_or_default();
        if text.contains("Keep the secret") {
            assert!(
                path.starts_with(".statute/site") || path.starts_with("me"),
                "{}",
                path.display()
            );
        }
    }
    // Ignored by the site's own `.gitignore`, without the repository's
    // `/.statute/`.
    write(&dir, ".gitignore", "/statute.private.toml\n/me/\n/here/\n");
    let status_without = git(&dir, &["status", "--porcelain", "--untracked-files=all"]);
    assert!(!status_without.contains(".statute"), "{status_without}");
    assert!(status_without.contains(".gitignore"), "{status_without}");
    assert!(!status.contains(".statute"));
}

#[test]
fn site_of_the_first_party_source_lists_it_once() {
    let dir = repo("here", "");
    write(
        &dir,
        "statute.toml",
        "constitution = { source = \"R\", edition = \"2026-10\" }\npublic = true\nrules = [\"R2\"]\n\n[sources]\nR = { path = \".\" }\n",
    );
    write(
        &dir,
        "statute.private.toml",
        "base = \"statute.toml\"\n\n[sources]\nme = { path = \"me\", public = false }\n",
    );
    put(&dir, "rules", domain(1));
    put(&dir, "rules", rule(2, 1, ""));
    put(&dir, "me/rules", domain(1));
    let r = statute(&dir, &["site"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let index = read(&dir, ".statute/site/index.html");
    assert!(section(&index, "global-public").contains("id=\"source-R\""));
    // The local section of its visibility points to it, and lists no
    // source of its own.
    let local = section(&index, "local-public");
    assert!(local.contains("<a href=\"#source-R\">R</a>"), "{local}");
    assert!(!local.contains("<h3"), "{local}");
    assert_eq!(index.matches("href=\"R/2.html\"").count(), 1, "{index}");
    assert!(!dir.join(".statute/site/L").exists());
    // Three quadrants hold something: the local private one nothing.
    let held = [
        "global-public",
        "global-private",
        "local-public",
        "local-private",
    ]
    .iter()
    .filter(|c| !section(&index, c).contains("No source."))
    .count();
    assert_eq!(held, 3);
}

#[test]
fn site_rebuilds_its_own_directory_and_leaves_another() {
    let dir = four("rebuild");
    assert_eq!(statute(&dir, &["site"]).code, 0);
    // A page of a node that is gone goes with the next run.
    std::fs::remove_file(dir.join("here/rules/5.toml")).unwrap();
    assert_eq!(statute(&dir, &["site"]).code, 0);
    assert!(!dir.join(".statute/site/here/5.html").exists());
    assert!(dir.join(".statute/site/here/1.html").exists());

    // A directory statute did not write stays as it is.
    let dir = four("theirs");
    write(&dir, ".statute/site/notes.txt", "mine\n");
    let r = statute(&dir, &["site"]);
    assert_eq!(r.code, 1);
    assert!(r.stderr.contains("not written by statute"), "{}", r.stderr);
    assert_eq!(read(&dir, ".statute/site/notes.txt"), "mine\n");
    assert!(!dir.join(".statute/site/index.html").exists());
}

#[test]
fn site_writes_nothing_where_git_tracks_a_file() {
    let dir = four("tracked");
    // A page of an earlier run, added to git by hand.
    assert_eq!(statute(&dir, &["site"]).code, 0);
    git(&dir, &["add", "-f", ".statute/site/index.html"]);
    let before = read(&dir, ".statute/site/index.html");
    write(&dir, "rules/7.toml", &rule(7, 1, "").1);
    let r = statute(&dir, &["site"]);
    assert_eq!(r.code, 1, "{}", r.stdout);
    assert!(r.stderr.contains("git tracks it"), "{}", r.stderr);
    assert_eq!(read(&dir, ".statute/site/index.html"), before);
    assert!(!dir.join(".statute/site/L/7.html").exists());
}

#[test]
fn site_marks_the_quadrant_of_each_rule() {
    for (lang, names) in [
        (
            "en",
            [
                "Global public",
                "Global private",
                "Local public",
                "Local private",
            ],
        ),
        ("zh", ["全局公开", "全局私有", "本仓公开", "本仓私有"]),
    ] {
        let dir = four(&format!("quadrant-{lang}"));
        let manifest = read(&dir, "statute.toml");
        let manifest = manifest.replace(
            "\n\n[sources]",
            &format!("\n\n[compile]\nlang = \"{lang}\"\n\n[sources]"),
        );
        write(&dir, "statute.toml", &manifest);
        let r = statute(&dir, &["site"]);
        assert_eq!(r.code, 0, "{}", r.stderr);
        let index = read(&dir, ".statute/site/index.html");
        for ((class, source, page), name) in [
            ("global-public", "R", "R/2.html"),
            ("global-private", "me", "me/4.html"),
            ("local-public", "L", "L/3.html"),
            ("local-private", "here", "here/5.html"),
        ]
        .into_iter()
        .zip(names)
        {
            let mark = format!("<span class=\"q {class}\">{name}</span>");
            // The page names the source as well; the index lists it above.
            let named = format!("<span class=\"q {class}\">{name} · {source}</span>");
            let html = read(&dir, &format!(".statute/site/{page}"));
            let h1 = &html[html.find("<h1>").unwrap()..html.find("</p>").unwrap()];
            assert!(h1.contains(&named), "{lang} {page}: {h1}");
            let item = index
                .lines()
                .find(|l| l.starts_with("<li") && l.contains(&format!("href=\"{page}\"")))
                .unwrap();
            assert!(item.contains(&mark), "{lang} {page}: {item}");
        }
    }
}

#[test]
fn site_writes_nothing_where_git_cannot_tell() {
    let dir = four("git-fails");
    // A git that answers everything but `ls-files`, with an error.
    let bin = dir.join("bin");
    let real = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|d| d.join("git"))
        .find(|p| p.is_file())
        .unwrap();
    write(
        &dir,
        "bin/git",
        &format!(
            "#!/bin/sh\nfor a; do [ \"$a\" = ls-files ] && {{ echo 'fatal: index file corrupt' >&2; exit 128; }}; done\nexec {} \"$@\"\n",
            real.display()
        ),
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(bin.join("git"), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let r = statute_with(&dir, &["site"], &[("PATH", &path)]);
    assert_eq!(r.code, 1, "{}", r.stdout);
    assert!(r.stderr.contains("index file corrupt"), "{}", r.stderr);
    assert!(!dir.join(".statute/site").exists());
}

#[test]
fn site_outside_a_git_repository_writes_the_pages() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("site")
        .join("no-git");
    let _ = std::fs::remove_dir_all(&dir);
    write(&dir, "constitution/2026-10/index.toml", INDEX);
    write(
        &dir,
        "constitution/2026-10/articles/04-authority/article.toml",
        "id = \"authority\"\n",
    );
    write(
        &dir,
        "statute.toml",
        "constitution = { source = \"R\", edition = \"2026-10\" }\npublic = true\nrules = [\"R2\"]\n\n[sources]\nR = { path = \".\" }\n",
    );
    put(&dir, "rules", domain(1));
    put(&dir, "rules", rule(2, 1, ""));
    // The test's directory is inside this repository; the ceiling hides it.
    let ceiling = dir.parent().unwrap().to_str().unwrap().to_owned();
    let r = statute_with(&dir, &["site"], &[("GIT_CEILING_DIRECTORIES", &ceiling)]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert!(dir.join(".statute/site/index.html").is_file());
}
