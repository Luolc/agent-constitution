//! `statute site`, end to end: a git repository whose sources fill the
//! four quadrants, and one that is itself the first-party source.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::{domain, edit, one_of, option, param, rule, set_default};

const INDEX: &str = "edition = \"2026-10\"\nrevision = \"1\"\n";

/// The private rule set of a repository made with `git init`.
const PRIVATE: &str = ".git/common/statute/private.toml";

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
    let at = dir.display();
    write(
        &dir,
        PRIVATE,
        &format!(
            "rules = [\"me:4\"]\n\n[sources]\nme = {{ path = \"{at}/me\", public = false }}\nhere = {{ path = \"{at}/here\", public = false, scope = \"local\" }}\n"
        ),
    );
    write(&dir, ".gitignore", "/me/\n/here/\n");
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

/// The item of the index that links `page`: a rule's row, or a one-of's
/// box with its options.
fn item<'a>(index: &'a str, page: &str) -> &'a str {
    let at = index
        .find(&format!("href=\"{page}\""))
        .unwrap_or_else(|| panic!("{page}: {index}"));
    let start = index[..at].rfind("<div class=\"item\"").unwrap();
    let end = at + index[at..].find("</div></div>\n").unwrap() + "</div></div>".len();
    &index[start..end]
}

/// The row of the index whose title links `page`.
fn entry<'a>(index: &'a str, page: &str) -> &'a str {
    let at = index
        .find(&format!("<a class=\"rule-text\" href=\"{page}\">"))
        .unwrap_or_else(|| panic!("{page}: {index}"));
    let start = index[..at].rfind("<div class=\"entry").unwrap();
    let enabled = at + index[at..].find("<span class=\"rule-enabled\">").unwrap();
    let end = enabled + index[enabled..].find("</div>").unwrap() + "</div>".len();
    &index[start..end]
}

/// The group row over the item that links `page`.
fn group<'a>(index: &'a str, page: &str) -> &'a str {
    let at = index.find(&format!("href=\"{page}\"")).unwrap();
    let start = index[..at].rfind(" class=\"group ").unwrap();
    let start = index[..start].rfind('<').unwrap();
    &index[start..start + index[start..].find('\n').unwrap()]
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
        r.stdout.contains("wrote 18 pages to .statute/site"),
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
    assert!(
        page.contains("<span class=\"enabled on\"><svg class=\"icon\""),
        "{page}"
    );
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
        assert!(
            path.starts_with(".statute/site") || *path == Path::new(".statute/.gitignore"),
            "{}",
            path.display()
        );
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
    // Ignored by statute's own `.gitignore` files, which git shows to be
    // committed.
    let status_after = git(&dir, &["status", "--porcelain", "--untracked-files=all"]);
    assert!(
        status_after.contains("?? .statute/.gitignore\n"),
        "{status_after}"
    );
    assert!(!status_after.contains(".statute/site"), "{status_after}");
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
        PRIVATE,
        &format!(
            "[sources]\nme = {{ path = \"{}/me\", public = false }}\n",
            dir.display()
        ),
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
            // The rule's page and its group row in the index name the
            // source as well.
            let named = format!("<span class=\"q {class}\">{name} · {source}</span>");
            let html = read(&dir, &format!(".statute/site/{page}"));
            let h1 = &html[html.find("<h1>").unwrap()..html.find("</p>").unwrap()];
            assert!(h1.contains(&named), "{lang} {page}: {h1}");
            let row = group(&index, page);
            assert!(row.contains(&named), "{lang} {page}: {row}");
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

/// The marks of `rule-config.md`: default switches, a one-of's default
/// option, `uses` and the values of parameters.
#[test]
fn site_marks_defaults_uses_and_values() {
    let dir = repo("marks", "r/");
    write(
        &dir,
        "statute.toml",
        "constitution = { source = \"R\", edition = \"2026-10\" }\npublic = false\nrules = [\"R7\"]\n\n[language]\ndocs = \"zh\"\n\n[sources]\nR = { path = \"r\" }\n",
    );
    write(
        &dir,
        "r/rules/params.toml",
        &param("language.docs", "type = \"language\"", "repo", "en"),
    );
    put(&dir, "r/rules", set_default(domain(1), "\"on\""));
    put(&dir, "r/rules", set_default(rule(2, 1, ""), "\"on\""));
    put(&dir, "r/rules", rule(3, 1, "uses = [\"review\"]"));
    put(&dir, "r/rules", one_of(4, 1, 5));
    put(&dir, "r/rules", option(5, 4, ""));
    put(&dir, "r/rules", option(6, 4, ""));
    put(
        &dir,
        "r/rules",
        edit(
            rule(7, 1, ""),
            "Do the thing 7.",
            "Write in {$language.docs}.",
        ),
    );
    for command in ["lock", "site"] {
        let r = statute(&dir, &[command]);
        assert_eq!(r.code, 0, "{command}: {}", r.stderr);
    }
    let site = dir.join(".statute/site");
    let index = read(&site, "index.html");
    // The first place of a row's marks says whether the defaults enable
    // it: a dot named by its words.
    for (id, mark) in [
        (
            2,
            "<span class=\"marks\"><span class=\"mark dot on\" role=\"img\" aria-label=\"on by default\"",
        ),
        (
            3,
            "<span class=\"marks\"><span class=\"mark dot\" role=\"img\" aria-label=\"off by default\"",
        ),
    ] {
        let row = entry(&index, &format!("R/{id}.html"));
        assert!(row.contains(mark), "{row}");
    }
    assert!(
        entry(&index, "R/5.html").contains("<span class=\"default-option\"><svg class=\"icon\""),
        "{}",
        entry(&index, "R/5.html")
    );
    assert!(!entry(&index, "R/6.html").contains("default-option"));
    // The options are led by Lucide's circles, the default's with a dot.
    let circle = include_str!("../assets/icons/lucide/circle.svg");
    let circle_dot = include_str!("../assets/icons/lucide/circle-dot.svg");
    let lead = |id: u64| {
        let at = index
            .find(&format!("<a class=\"rule-text\" href=\"R/{id}.html\">"))
            .unwrap();
        &index[index[..at].rfind("<svg").unwrap()..at]
    };
    assert!(
        lead(5).contains("<circle cx=\"12\" cy=\"12\" r=\"1\"/>"),
        "{}",
        lead(5)
    );
    assert!(circle_dot.contains("r=\"1\""));
    assert!(!lead(6).contains("r=\"1\""), "{}", lead(6));
    assert!(lead(6).contains("<circle cx=\"12\" cy=\"12\" r=\"10\"/>"));
    assert!(circle.contains("r=\"10\""));
    // The one-of is a box that holds its options, labeled on its border
    // and named by that label, and they stand nowhere else in the list.
    let one_of = item(&index, "R/4.html");
    assert!(
        one_of.contains("<div class=\"one-of\" role=\"group\" aria-labelledby=\"one-of-R-4\"><span class=\"one-of-label\" id=\"one-of-R-4\">One of</span><div class=\"entry\"><span class=\"ref\">R4</span>"),
        "{one_of}"
    );
    assert!(
        one_of.contains("R/5.html") && one_of.contains("R/6.html"),
        "{one_of}"
    );
    assert_eq!(index.matches("href=\"R/5.html\"").count(), 1, "{index}");
    // A rule only for review carries its use for the filter, which the
    // style sheet hides with the box of review.
    assert!(
        item(&index, "R/3.html").starts_with(
            "<div class=\"item\" data-status=\"draft\" data-default=\"off\" data-uses=\"review\" data-domain=\"R1\" data-enabled=\"off\">"
        ),
        "{}",
        item(&index, "R/3.html")
    );
    assert!(index.contains(
        "<input type=\"checkbox\" id=\"f-uses-1\" name=\"uses\" value=\"review\" checked>"
    ));
    assert!(
        index.contains(
            "body:has(#f-uses-1:not(:checked)) .item[data-uses=\"review\"]{display:none}"
        )
    );
    assert!(index.contains("<a class=\"params\" href=\"R/parameters.html\">Parameters</a>"));
    assert!(read(&site, "R/3.html").contains("<p>Used in: review</p>"));
    assert!(read(&site, "R/2.html").contains("<p>Used in: AGENTS.md</p>"));
    assert!(
        read(&site, "R/7.html").contains(
            "<p>Write in <a class=\"value\" href=\"../R/parameters.html#language.docs\" title=\"language.docs\">Chinese</a>.</p>"
        ),
        "{}",
        read(&site, "R/7.html")
    );
    let params = read(&site, "R/parameters.html");
    assert!(
        params.contains(
            "<tr id=\"language.docs\"><td>The language.docs<br><code>language.docs</code></td><td>language · repo</td><td><code>en</code></td><td><code>zh</code> (statute.toml)</td><td><a href=\"../R/7.html\">"
        ),
        "{params}"
    );
}

/// The style of `docs/design/frontend.md`: the tokens and the icons the
/// public site uses, the filter behind a button, and the level of a
/// domain shown by its heading and its class, not by its depth.
#[test]
fn site_draws_the_tree_by_level() {
    let dir = repo("levels", "r/");
    write(
        &dir,
        "statute.toml",
        "constitution = { source = \"R\", edition = \"2026-10\" }\npublic = false\nrules = [\"R4\", \"R12\"]\n\n[sources]\nR = { path = \"r\" }\n",
    );
    put(&dir, "r/rules", domain(1));
    put(
        &dir,
        "r/rules",
        edit(domain(2), "default", "parent = 1\ndefault"),
    );
    put(&dir, "r/rules", rule(4, 2, ""));
    put(&dir, "r/rules", one_of(5, 2, 6));
    put(&dir, "r/rules", option(6, 5, ""));
    // A one-of whose options are domains, one of them on.
    put(&dir, "r/rules", one_of(9, 1, 10));
    put(
        &dir,
        "r/rules",
        edit(domain(10), "default = \"off\"", "parent = 9"),
    );
    put(
        &dir,
        "r/rules",
        edit(domain(11), "default = \"off\"", "parent = 9"),
    );
    put(&dir, "r/rules", rule(12, 10, ""));
    // A third level, under domain 2.
    put(
        &dir,
        "r/rules",
        edit(domain(13), "default", "parent = 2\ndefault"),
    );
    put(&dir, "r/rules", rule(14, 13, ""));
    let r = statute(&dir, &["site"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let index = read(&dir, ".statute/site/index.html");
    // A domain is no row but the band over its rules, its path from the
    // source, the darker band and `h4` for the first level, a deeper one
    // `h5`; its first place of the marks says whether the defaults
    // enable it.
    let top = group(&index, "R/9.html");
    assert!(
        top.starts_with("<h4 class=\"group top\"><a href=\"R/1.html\" aria-label=\"R1 Domain 1\">"),
        "{top}"
    );
    let deep = group(&index, "R/4.html");
    assert!(
        deep.starts_with(
            "<h5 class=\"group deep\"><a href=\"R/2.html\" aria-label=\"R2 Domain 1, Domain 2\">"
        ),
        "{deep}"
    );
    assert!(deep.contains("<span class=\"marks\"><span class=\"mark dot\" role=\"img\" aria-label=\"off by default\""), "{deep}");
    // A third level goes a heading deeper and writes its whole path; its
    // block has its row alone, the one that sticks.
    let third = group(&index, "R/14.html");
    assert!(
        third.starts_with(
            "<h6 class=\"group deep\"><a href=\"R/13.html\" aria-label=\"R13 Domain 1, Domain 2, Domain 13\">"
        ),
        "{third}"
    );
    assert_eq!(
        third.matches("<span class=\"group-part\"").count(),
        3,
        "{third}"
    );
    let block = &index[..index.find("R/14.html\"").unwrap()];
    let block = &block[block.rfind("<div class=\"block\">").unwrap()..];
    assert_eq!(block.matches("class=\"group ").count(), 1, "{block}");
    // A rule under a domain that is an option of a one-of has that
    // domain in its path.
    assert!(group(&index, "R/12.html").contains("aria-label=\"R10 Domain 1, Domain 10\""));
    // A one-of none of whose options is enabled is drawn lighter.
    assert!(item(&index, "R/5.html").contains("<div class=\"one-of unset\""));
    // One with a domain on is not.
    assert!(item(&index, "R/9.html").contains("<div class=\"one-of\" role=\"group\""));
    // Every page carries the tokens, as the public site has them.
    let tokens = include_str!("../assets/tokens.css");
    assert!(index.contains(tokens));
    assert!(read(&dir, ".statute/site/R/4.html").contains(tokens));
    assert!(index.contains("--icon-check: url(\"data:image/svg+xml,%3Csvg "));
    // The legend draws each mark as the rows do, a line per kind, the
    // defaults first, open until folded.
    let legend = &index[index.find("<details class=\"legend\" open>").expect(&index)..];
    let legend = &legend[..legend.find("</details>").unwrap()];
    assert_eq!(legend.matches("<dt>").count(), 6, "{legend}");
    assert!(legend.find("<dt>Default</dt>") < legend.find("<dt>Status</dt>"));
    assert!(
        legend.contains(
            "<span class=\"legend-item\"><span class=\"mark dot on\"></span>on by default</span>"
        ),
        "{legend}"
    );
    // The filter is a menu per field, each with `chevron-down`, under its
    // name.
    let chevron = include_str!("../assets/icons/lucide/chevron-down.svg");
    let path = chevron
        .lines()
        .find(|l| l.contains("<path"))
        .unwrap()
        .trim();
    let menu = &index[index
        .find("<span class=\"control-label\" id=\"label-status\">Status</span><details class=\"menu\" data-field=\"status\"")
        .expect(&index)..];
    assert!(menu[..menu.find("</summary>").unwrap()].contains(&path.replace(" />", "/>")));
}

#[test]
fn site_names_the_parent_of_a_shared_title() {
    let dir = repo("shared-titles", "r/");
    write(
        &dir,
        "statute.toml",
        "constitution = { source = \"R\", edition = \"2026-10\" }\npublic = false\nrules = [\"R5\", \"R6\"]\n\n[sources]\nR = { path = \"r\" }\n",
    );
    put(&dir, "r/rules", domain(1));
    put(&dir, "r/rules", domain(2));
    // Domains 3 and 4, under 1 and 2, both titled Tiers; 7 has a title of
    // its own.
    for (id, parent) in [(3, 1), (4, 2), (7, 1)] {
        let tiers = edit(
            domain(id),
            "default",
            &format!("parent = {parent}\ndefault"),
        );
        let tiers = if id == 7 {
            tiers
        } else {
            edit(tiers, &format!("Domain {id}\""), "Tiers\"")
        };
        put(&dir, "r/rules", tiers);
    }
    put(&dir, "r/rules", rule(5, 3, ""));
    put(&dir, "r/rules", rule(6, 4, "requires = [5]"));
    // Rule 8, beside rule 5 under domain 3, has its title.
    put(
        &dir,
        "r/rules",
        edit(rule(8, 3, ""), "Rule 8\"", "Rule 5\""),
    );
    let r = statute(&dir, &["site"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let site = dir.join(".statute/site");
    let parent = |name: &str| format!(" <span class=\"parent\">({name})</span></a>");
    // A link to a domain whose title another has names its parent.
    let one = std::fs::read_to_string(site.join("R/1.html")).unwrap();
    assert!(
        one.contains(&format!("Tiers</span>{}", parent("Domain 1"))),
        "{one}"
    );
    assert!(one.contains("Domain 7</span></a>"), "{one}");
    // The domain menu does as well.
    let index = read(&dir, ".statute/site/index.html");
    let menu = &index[index.find("data-field=\"domain\"").expect(&index)..];
    assert!(
        menu.contains("Tiers</span> <span class=\"parent\">(Domain 2)</span>"),
        "{menu}"
    );
    // A rule's breadcrumb, which names the parents, does not.
    let five = std::fs::read_to_string(site.join("R/5.html")).unwrap();
    let nav = &five[five.find("<nav>").unwrap()..five.find("</nav>").unwrap()];
    assert!(nav.contains("Tiers</span></a>"), "{nav}");
    assert!(!nav.contains("class=\"parent\""), "{nav}");
    // Two rules of one title under one parent are told apart by their
    // references, not by their parent's title, which they share.
    let six = std::fs::read_to_string(site.join("R/6.html")).unwrap();
    assert!(six.contains("Rule 5</span></a>"), "{six}");
    let three = std::fs::read_to_string(site.join("R/3.html")).unwrap();
    assert_eq!(three.matches("Rule 5</span></a>").count(), 2, "{three}");
}

/// The path of the page the language switch of the page at `path` leads
/// to, both below the site's directory.
fn switched(site: &Path, path: &Path) -> PathBuf {
    let html = std::fs::read_to_string(site.join(path)).unwrap();
    let start = html.find("<a class=\"languages\" href=\"").unwrap() + 27;
    let href = &html[start..start + html[start..].find('"').unwrap()];
    let mut to = path.parent().unwrap().to_path_buf();
    for part in href.split('/') {
        if part == ".." {
            assert!(to.pop(), "{}: {href}", path.display());
        } else {
            to.push(part);
        }
    }
    to
}

#[test]
fn site_writes_both_languages() {
    let dir = four("languages");
    // R2 has a Chinese translation, here:5 is Chinese only, and me:4,
    // which cites R2, English only.
    let hash = statute::node::original_hash(&[
        "Rule 2",
        "Do the thing 2.",
        "Because.\n",
        "Violated when the thing is not done.",
    ]);
    let r2 = rule(2, 1, "");
    let r2 = edit(r2, "en = \"Rule 2\"", "en = \"Rule 2\"\nzh = \"规定 2\"");
    let r2 = edit(
        r2,
        "en = \"Do the thing 2.\"",
        "en = \"Do the thing 2.\"\nzh = \"做事 2。\"",
    );
    let r2 = edit(r2, "Because.\n'''", "Because.\n'''\nzh = \"因为。\"");
    let r2 = edit(
        r2,
        "en = \"Violated when the thing is not done.\"",
        "en = \"Violated when the thing is not done.\"\nzh = \"没做即为违反。\"",
    );
    write(
        &dir,
        "r/rules/2.toml",
        &format!("{}\n[translations]\nzh = \"{hash}\"\n", r2.1),
    );
    let (id, r5) = edit(rule(5, 1, ""), "lang = \"en\"", "lang = \"zh\"");
    let r5 = edit((id, r5.replace("\nen = ", "\nzh = ")), "Rule 5", "规定 5");
    put(&dir, "here/rules", r5);
    let r = statute(&dir, &["site"]);
    assert_eq!(r.code, 0, "{}", r.stderr);

    // The language of `[compile] lang` at the top, the other in `zh/`,
    // with the same pages.
    let site = dir.join(".statute/site");
    let all = files(&site);
    let top: BTreeSet<&PathBuf> = all.iter().filter(|p| !p.starts_with("zh")).collect();
    let zh: BTreeSet<PathBuf> = all
        .iter()
        .filter_map(|p| p.strip_prefix("zh").ok().map(Path::to_path_buf))
        .collect();
    assert_eq!(top.len(), 10, "{top:?}");
    for path in top
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "html"))
    {
        assert!(zh.contains(*path), "{}", path.display());
        // Each page's switch leads to the same page in the other
        // language, and back.
        let other = switched(&site, path);
        assert_eq!(other, Path::new("zh").join(path));
        assert_eq!(switched(&site, &other), **path);
    }
    let index = read(&site, "index.html");
    let index_zh = read(&site, "zh/index.html");
    assert!(index.contains("<html lang=\"en\">") && index.contains("Clear filters"));
    assert!(index_zh.contains("<html lang=\"zh\">") && index_zh.contains("清除筛选"));
    assert!(
        index
            .contains("<a class=\"languages\" href=\"zh/index.html\" hreflang=\"zh\" lang=\"zh\">")
    );
    assert!(index.contains("中文</a>") && index_zh.contains("English</a>"));

    // A title the page's language has, as it is; another, in the node's
    // language, marked with it.
    assert!(
        index_zh.contains("<span class=\"title\" title=\"规定 2\">规定 2</span>"),
        "{index_zh}"
    );
    assert!(index_zh.contains("<span class=\"title\" lang=\"en\" title=\"Rule 3\">Rule 3</span>"));
    assert!(index.contains("<span class=\"title\" lang=\"zh\" title=\"规定 5\">规定 5</span>"));

    // A node with every text in the page's language has no tag.
    for page in ["R/2.html", "zh/R/2.html", "zh/here/5.html"] {
        let html = read(&site, page);
        assert!(!html.contains("class=\"untranslated\""), "{page}");
        assert!(
            !html.contains("<span lang=") && !html.contains("<div lang="),
            "{page}"
        );
    }
    let r2 = read(&site, "zh/R/2.html");
    assert!(
        r2.contains("<span class=\"ref\">R2</span>规定 2</h1>"),
        "{r2}"
    );
    assert!(r2.contains("<p>做事 2。</p>"), "{r2}");

    // One without them is shown in its own, marked, with a tag; a rule it
    // cites is named in the language of its text.
    let r4 = read(&site, "zh/me/4.html");
    assert!(
        r4.contains("<span lang=\"en\">Private rule 4</span></h1>"),
        "{r4}"
    );
    assert!(r4.contains("<span class=\"untranslated\">"), "{r4}");
    assert!(r4.contains("无中文</span>"), "{r4}");
    assert!(
        r4.contains("<div lang=\"en\">\n<p>Keep the secret, after <a href=\"../R/2.html\"><span class=\"ref\">R2</span><span class=\"title\">Rule 2</span></a>.</p>\n</div>"),
        "{r4}"
    );
    let r5 = read(&site, "here/5.html");
    assert!(r5.contains("<span lang=\"zh\">规定 5</span></h1>"), "{r5}");
    assert!(r5.contains("Not translated</span>"), "{r5}");
}

/// The statement under each rule's row, its placeholders filled as in
/// `AGENTS.md`, in the page's language or marked as not translated, and
/// the three views of the rows, chosen by radio buttons the style sheet
/// follows.
/// A site whose rules have statements: R2 in English and Chinese, R7 in
/// English only, with a placeholder the rule set gives `zh`.
fn statements(name: &str) -> PathBuf {
    let dir = repo(name, "r/");
    write(
        &dir,
        "statute.toml",
        "constitution = { source = \"R\", edition = \"2026-10\" }\npublic = false\nrules = [\"R2\", \"R7\"]\n\n[language]\ndocs = \"zh\"\n\n[sources]\nR = { path = \"r\" }\n",
    );
    write(
        &dir,
        "r/rules/params.toml",
        &param("language.docs", "type = \"language\"", "repo", "en"),
    );
    put(&dir, "r/rules", domain(1));
    let hash = statute::node::original_hash(&[
        "Rule 2",
        "Do the thing 2.",
        "Because.\n",
        "Violated when the thing is not done.",
    ]);
    let r2 = rule(2, 1, "");
    let r2 = edit(r2, "en = \"Rule 2\"", "en = \"Rule 2\"\nzh = \"规定 2\"");
    let r2 = edit(
        r2,
        "en = \"Do the thing 2.\"",
        "en = \"Do the thing 2.\"\nzh = \"做事 2。\"",
    );
    let r2 = edit(r2, "Because.\n'''", "Because.\n'''\nzh = \"因为。\"");
    let r2 = edit(
        r2,
        "en = \"Violated when the thing is not done.\"",
        "en = \"Violated when the thing is not done.\"\nzh = \"没做即为违反。\"",
    );
    write(
        &dir,
        "r/rules/2.toml",
        &format!("{}\n[translations]\nzh = \"{hash}\"\n", r2.1),
    );
    put(
        &dir,
        "r/rules",
        edit(
            rule(7, 1, ""),
            "Do the thing 7.",
            "Write in {$language.docs}.",
        ),
    );
    for command in ["lock", "site"] {
        let r = statute(&dir, &[command]);
        assert_eq!(r.code, 0, "{command}: {}", r.stderr);
    }
    dir
}

#[test]
fn site_shows_statements_in_three_views() {
    let site = statements("statements").join(".statute/site");
    // The statement of a rule, at the end of its row.
    let statement = |index: &str, id: u64| {
        let row = entry(index, &format!("R/{id}.html"));
        let start = row.find("<div class=\"statement\"").unwrap();
        let end = start + row[start..].find("</div></span>").unwrap() + "</div>".len();
        row[start..end].to_owned()
    };
    let index = read(&site, "index.html");
    assert_eq!(
        statement(&index, 7),
        "<div class=\"statement\">\n<p>Write in <a class=\"value\" href=\"R/parameters.html#language.docs\" title=\"language.docs\">Chinese</a>.</p>\n</div>"
    );
    assert_eq!(
        statement(&index, 2),
        "<div class=\"statement\">\n<p>Do the thing 2.</p>\n</div>"
    );
    let zh = read(&site, "zh/index.html");
    assert_eq!(
        statement(&zh, 2),
        "<div class=\"statement\">\n<p>做事 2。</p>\n</div>"
    );
    let r7 = statement(&zh, 7);
    assert!(
        r7.starts_with("<div class=\"statement\" lang=\"en\">\n<span class=\"untranslated\" role=\"img\" aria-label=\"无中文\" title=\"无中文\" lang=\"zh\"><svg class=\"icon\""),
        "{r7}"
    );
    assert!(
        r7.contains("<p>Write in <a class=\"value\" href=\"R/parameters.html#language.docs\" title=\"language.docs\">Chinese</a>.</p>"),
        "{r7}"
    );
    // Three radio buttons, the title and the statement checked; the
    // style sheet hides the statements or the titles by them, and the
    // script keeps the choice in the address.
    for (page, names) in [
        (&index, ["Title and statement", "Title", "Statement"]),
        (&zh, ["标题和要求", "标题", "要求"]),
    ] {
        for (i, (view, name)) in ["both", "title", "statement"].iter().zip(names).enumerate() {
            let checked = if i == 0 { " checked" } else { "" };
            let radio = format!(
                "<input type=\"radio\" name=\"view\" id=\"view-{view}\" value=\"{view}\"{checked}><label for=\"view-{view}\">{name}</label>"
            );
            assert!(page.contains(&radio), "{radio}");
        }
        assert!(
            page.contains("body:has(#view-title:checked) .entry .statement { display: none; }")
        );
        assert!(
            page.contains("body:has(#view-statement:checked) .has-statement .rule-text .title {")
        );
        assert!(page.contains("query.push(`view=${view.value}`)"));
    }
    // A rule's own page is as it was.
    assert!(read(&site, "R/7.html").contains(
        "<p>Write in <a class=\"value\" href=\"../R/parameters.html#language.docs\" title=\"language.docs\">Chinese</a>.</p>"
    ));
}

/// Google Chrome or Chromium on `PATH`; `None` lets a test skip, but not
/// in CI, which has Chrome.
fn chrome() -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let found = std::env::split_paths(&path)
        .flat_map(|dir| {
            [
                "google-chrome",
                "google-chrome-stable",
                "chromium",
                "chromium-browser",
            ]
            .map(|name| dir.join(name))
        })
        .find(|path| path.is_file());
    assert!(
        found.is_some() || std::env::var_os("CI").is_none(),
        "no Chrome in CI"
    );
    found
}

/// What the script added to `page` writes on `<body data-probe>` once it
/// is loaded at `query` in headless Chrome, with the profile in `dir`,
/// which keeps what the page stores from one load to the next.
fn probe_run(chrome: &Path, dir: &Path, page: &Path, query: &str) -> String {
    let out = Command::new(chrome)
        .args([
            "--headless",
            "--no-proxy-server",
            "--virtual-time-budget=5000",
        ])
        .arg(format!("--user-data-dir={}", dir.join("profile").display()))
        .arg("--dump-dom")
        .arg(format!("file://{}{query}", page.display()))
        .output()
        .unwrap();
    let dom = String::from_utf8(out.stdout).unwrap();
    let start = dom.find("data-probe=\"").expect(&dom) + 12;
    dom[start..start + dom[start..].find('"').unwrap()].replace("&amp;", "&")
}

/// The view in the address, in headless Chrome, which runs the index's
/// script: an address without a view shows the default, also on the way
/// back to it; an index opened at such an address shows the view kept
/// and writes it in; an address's own view wins. A script added to the
/// page records what it shows on `<body data-probe>`. Skipped without
/// Chrome, except in CI.
#[test]
fn site_keeps_the_view_in_the_address() {
    let Some(chrome) = chrome() else {
        eprintln!("skipped: needs Google Chrome or Chromium");
        return;
    };
    let dir = statements("view");
    let index = read(&dir, ".statute/site/index.html");
    let probe = r##"<script>
const shown = () => document.querySelector("input[name='view']:checked").value;
const done = (r) => document.body.setAttribute("data-probe", r.join(" "));
addEventListener("load", () => {
  const r = [shown(), location.search || "-"];
  if (location.hash !== "#click") return done(r);
  document.querySelector("label[for='view-title']").click();
  r.push(shown(), location.search || "-");
  addEventListener("popstate", () => done([...r, shown(), location.search || "-"]), { once: true });
  history.back();
});
</script></body>"##;
    let page = dir.join("probe.html");
    std::fs::write(&page, index.replace("</body>", probe)).unwrap();
    let run = |query: &str| probe_run(&chrome, &dir, &page, query);
    // A fresh profile: the default; a click writes Title into the
    // address, and back returns to the address without a view, and so to
    // the default, though Title is kept.
    assert_eq!(
        run("?x=1#click"),
        "both ?x=1 title ?x=1&view=title both ?x=1"
    );
    // Opened again at an address without a view: the view kept, written
    // into the address.
    assert_eq!(run("?x=1"), "title ?x=1&view=title");
    // An address with a view: that view.
    assert_eq!(run("?view=statement"), "statement ?view=statement");
}

/// The filter, the search and the legend, in headless Chrome, which runs
/// the index's script, on the four rules of `four`, all drafts, `here:5`
/// alone not enabled: the number of rules shown, the sources shown, what
/// the menus of the status and of the rule set say, and whether the
/// legend is open; Clear filters, which brings back the default filter
/// and an empty search; and the legend kept folded. Skipped without
/// Chrome, except in CI.
#[test]
fn site_filters_and_searches_the_table() {
    let Some(chrome) = chrome() else {
        eprintln!("skipped: needs Google Chrome or Chromium");
        return;
    };
    let dir = four("filter");
    assert_eq!(statute(&dir, &["site"]).code, 0);
    let index = read(&dir, ".statute/site/index.html");
    let probe = r##"<script>
const state = () => [
  document.querySelector(".shown").textContent,
  document.querySelectorAll(".source:not([hidden])").length,
  document.querySelector("details.menu[data-field=status] .value").textContent,
  document.querySelector("details.menu[data-field=enabled] .value").textContent,
  document.querySelector(".none").hidden ? "-" : "none",
  document.querySelector("details.legend").open ? "open" : "folded",
  ["details.menu", "details.legend"]
    .map((d) => document.querySelector(`${d} > summary`).getAttribute("aria-expanded"))
    .join("/"),
  Array.from(document.querySelectorAll("summary")).every((s) => document.getElementById(s.getAttribute("aria-controls")))
    ? "controls"
    : "no controls",
];
const done = (r) => document.body.setAttribute("data-probe", r.join(" | "));
addEventListener("load", () => {
  const r = state();
  if (location.hash === "#clear") {
    document.querySelector(".clear").click();
    r.push(...state(), location.search || "-");
  }
  if (location.hash === "#fold") document.querySelector("details.legend > summary").click();
  done(r);
});
</script></body>"##;
    let page = dir.join("probe.html");
    std::fs::write(&page, index.replace("</body>", probe)).unwrap();
    let run = |query: &str| probe_run(&chrome, &dir, &page, query);
    // Each summary says whether it is open and what it opens.
    assert_eq!(
        run(""),
        "4 rules | 4 | 3/5 | All | - | open | false/true | controls"
    );
    assert_eq!(
        run("?enabled=off"),
        "1 rules | 1 | 3/5 | not enabled | - | open | false/true | controls"
    );
    assert_eq!(
        run("?q=secret"),
        "1 rules | 1 | 3/5 | All | - | open | false/true | controls"
    );
    assert_eq!(
        run("?status="),
        "0 rules | 0 | None | All | none | open | false/true | controls"
    );
    assert_eq!(
        run("?status=&q=x#clear"),
        "0 rules | 0 | None | All | none | open | false/true | controls | 4 rules | 4 | 3/5 | All | - | open | false/true | controls | -"
    );
    // Folded, the legend stays folded on the next load.
    run("#fold");
    assert_eq!(
        run(""),
        "4 rules | 4 | 3/5 | All | - | folded | false/false | controls"
    );
}

/// A statement with a path in code longer than a phone's row wraps: the
/// index, loaded 390px wide in a frame, as Chrome's headless window is
/// at least 500px wide, does not scroll sideways, in either language.
/// Skipped without Chrome, except in CI.
#[test]
fn site_fits_a_phone() {
    let Some(chrome) = chrome() else {
        eprintln!("skipped: needs Google Chrome or Chromium");
        return;
    };
    let dir = statements("phone");
    let rule = dir.join("r/rules/7.toml");
    let text = std::fs::read_to_string(&rule).unwrap();
    let code = "`.claude/skills/<name>/references/alongfilename.md`";
    std::fs::write(
        &rule,
        edit(
            (7, text),
            "Write in {$language.docs}.",
            &format!("Write in {{$language.docs}}, in {code}."),
        )
        .1,
    )
    .unwrap();
    for command in ["lock", "site"] {
        let r = statute(&dir, &[command]);
        assert_eq!(r.code, 0, "{command}: {}", r.stderr);
    }
    for page in ["index.html", "zh/index.html"] {
        assert!(read(&dir.join(".statute/site"), page).contains("alongfilename.md"));
    }
    let page = dir.join("phone.html");
    std::fs::write(
        &page,
        r##"<!DOCTYPE html><body>
<iframe src=".statute/site/index.html" style="width: 390px; height: 844px"></iframe>
<iframe src=".statute/site/zh/index.html" style="width: 390px; height: 844px"></iframe>
<script>
const frames = Array.from(document.querySelectorAll("iframe"));
Promise.all(frames.map((f) => new Promise((done) =>
  f.addEventListener("load", () => {
    const root = f.contentDocument.documentElement;
    done(`${root.clientWidth} ${root.scrollWidth <= root.clientWidth}`);
  })))).then((r) => document.body.setAttribute("data-probe", r.join(" | ")));
</script></body>"##,
    )
    .unwrap();
    let out = Command::new(chrome)
        .args([
            "--headless",
            "--no-proxy-server",
            "--hide-scrollbars",
            // The frames' documents, files too, are the page's to read.
            "--allow-file-access-from-files",
            "--window-size=1280,900",
            "--virtual-time-budget=5000",
        ])
        .arg(format!("--user-data-dir={}", dir.join("profile").display()))
        .arg("--dump-dom")
        .arg(format!("file://{}", page.display()))
        .output()
        .unwrap();
    let dom = String::from_utf8(out.stdout).unwrap();
    let start = dom.find("data-probe=\"").expect(&dom) + 12;
    assert_eq!(
        &dom[start..start + dom[start..].find('"').unwrap()],
        "390 true | 390 true"
    );
}
