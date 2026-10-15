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
    let item = |id: u64| {
        let start = index
            .find(&format!("<a href=\"R/{id}.html\">"))
            .unwrap_or_else(|| panic!("R{id}: {index}"));
        let end = index[start..]
            .find("</li>")
            .map_or(index.len(), |e| start + e);
        &index[start..end]
    };
    // In the list a default switch is a dot named by its words.
    assert!(
        item(2).contains("<span class=\"switch on\" role=\"img\" aria-label=\"on by default\""),
        "{}",
        item(2)
    );
    assert!(
        item(3).contains("<span class=\"switch off\" role=\"img\" aria-label=\"off by default\""),
        "{}",
        item(3)
    );
    assert!(
        item(5).contains("<span class=\"default-option\"><svg class=\"icon\""),
        "{}",
        item(5)
    );
    assert!(!item(6).contains("default"), "{}", item(6));
    // The options are led by Lucide's circles, the default's with a dot.
    let circle = include_str!("../assets/icons/lucide/circle.svg");
    let circle_dot = include_str!("../assets/icons/lucide/circle-dot.svg");
    let lead = |id: u64| {
        let at = index.find(&format!("<a href=\"R/{id}.html\">")).unwrap();
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
    // The one-of is a box that holds its options, labeled on its border,
    // and they stand nowhere else in the list.
    let one_of = index
        .find("<li class=\"one-of\"><span class=\"one-of-label\">One of</span><div class=\"row\"><a href=\"R/4.html\">")
        .expect(&index);
    let options = index[one_of..]
        .find("</ul></li>")
        .map(|e| &index[one_of..one_of + e])
        .unwrap();
    assert!(
        options.contains("R/5.html") && options.contains("R/6.html"),
        "{options}"
    );
    assert_eq!(index.matches("<a href=\"R/5.html\">").count(), 1, "{index}");
    // A rule only for review is hidden with its filter.
    assert!(
        index.contains(
            "<li class=\"off has-statement u-r\"><div class=\"row\"><a href=\"R/3.html\">"
        ),
        "{index}"
    );
    assert!(index.contains("<input type=\"checkbox\" id=\"show-review\" checked>"));
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
    let r = statute(&dir, &["site"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let index = read(&dir, ".statute/site/index.html");
    for (id, class, h) in [(1, "d1", 4), (2, "d2", 5)] {
        let row = format!(
            "<li class=\"domain {class}\"><div class=\"row\"><h{h}><a href=\"R/{id}.html\">"
        );
        assert!(index.contains(&row), "{row}: {index}");
    }
    assert!(
        index.contains("<li class=\"has-statement u-i\"><div class=\"row\"><a href=\"R/4.html\">"),
        "{index}"
    );
    // A one-of none of whose options is enabled is drawn lighter.
    assert!(index.contains("<li class=\"one-of unset\">"), "{index}");
    // One with a domain on is not.
    assert!(
        index.contains("<li class=\"one-of\"><span class=\"one-of-label\">One of</span><div class=\"row\"><a href=\"R/9.html\">"),
        "{index}"
    );
    // Every page carries the tokens, as the public site has them.
    let tokens = include_str!("../assets/tokens.css");
    assert!(index.contains(tokens));
    assert!(read(&dir, ".statute/site/R/4.html").contains(tokens));
    assert!(index.contains("--icon-check: url(\"data:image/svg+xml,%3Csvg "));
    // The legend draws each mark as the tree does and says what it means.
    let legend = &index[index.find("<dl class=\"legend\">").expect(&index)..];
    let legend = &legend[..legend.find("</dl>").unwrap()];
    assert_eq!(legend.matches("<div><dt>").count(), 11, "{legend}");
    assert!(legend.contains("<dt><span class=\"switch on\" role=\"img\" aria-label=\"on by default\" title=\"on by default\"></span></dt><dd>A new rule set gets it without listing it.</dd>"), "{legend}");
    assert!(
        legend.contains("<dt><span class=\"status trial\">trial</span></dt>"),
        "{legend}"
    );
    // The filter is a button with Lucide's funnel.
    let funnel = include_str!("../assets/icons/lucide/funnel.svg");
    let path = funnel.lines().find(|l| l.contains("<path")).unwrap().trim();
    let filter = &index[index
        .find("<details class=\"filter\"><summary><svg class=\"icon\"")
        .expect(&index)..];
    assert!(filter[..filter.find("</summary>").unwrap()].contains(&path.replace(" />", "/>")));
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
    assert!(index.contains("<html lang=\"en\">") && index.contains("Filter"));
    assert!(index_zh.contains("<html lang=\"zh\">") && index_zh.contains("筛选"));
    assert!(
        index
            .contains("<a class=\"languages\" href=\"zh/index.html\" hreflang=\"zh\" lang=\"zh\">")
    );
    assert!(index.contains("中文</a>") && index_zh.contains("English</a>"));

    // A title the page's language has, as it is; another, in the node's
    // language, marked with it.
    assert!(
        index_zh.contains("<span class=\"title\">规定 2</span>"),
        "{index_zh}"
    );
    assert!(index_zh.contains("<span class=\"title\" lang=\"en\">Rule 3</span>"));
    assert!(index.contains("<span class=\"title\" lang=\"zh\">规定 5</span>"));

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
#[test]
fn site_shows_statements_in_three_views() {
    let dir = repo("statements", "r/");
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
    let site = dir.join(".statute/site");
    // The statement of a rule, after its row and before the next one.
    let statement = |index: &str, id: u64| {
        let at = index.find(&format!("<a href=\"R/{id}.html\">")).unwrap();
        let row = at + index[at..].find("</div>").unwrap() + "</div>".len();
        index[row..row + index[row..].find("</li>").unwrap()].to_owned()
    };
    let index = read(&site, "index.html");
    assert_eq!(
        statement(&index, 7),
        "<div class=\"statement\">\n<p>Write in <a class=\"value\" href=\"R/parameters.html#language.docs\" title=\"language.docs\">Chinese</a>.</p>\n</div>\n"
    );
    assert_eq!(
        statement(&index, 2),
        "<div class=\"statement\">\n<p>Do the thing 2.</p>\n</div>\n"
    );
    let zh = read(&site, "zh/index.html");
    assert_eq!(
        statement(&zh, 2),
        "<div class=\"statement\">\n<p>做事 2。</p>\n</div>\n"
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
        (&zh, ["标题与规定", "标题", "规定"]),
    ] {
        for (i, (view, name)) in ["both", "title", "statement"].iter().zip(names).enumerate() {
            let checked = if i == 0 { " checked" } else { "" };
            let radio = format!(
                "<input type=\"radio\" name=\"view\" id=\"view-{view}\" value=\"{view}\"{checked}><label for=\"view-{view}\">{name}</label>"
            );
            assert!(page.contains(&radio), "{radio}");
        }
        assert!(page.contains("body:has(#view-title:checked) .tree .statement { display: none; }"));
        assert!(page.contains("body:has(#view-statement:checked) .has-statement > .row .title {"));
        assert!(page.contains("query.set(\"view\", view.value)"));
    }
    // A rule's own page is as it was.
    assert!(read(&site, "R/7.html").contains(
        "<p>Write in <a class=\"value\" href=\"../R/parameters.html#language.docs\" title=\"language.docs\">Chinese</a>.</p>"
    ));
}
