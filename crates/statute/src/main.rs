use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use toml_edit::{Array, DocumentMut, Item, Value};

use statute::diag::Diagnostics;
use statute::lock;
use statute::manifest::{self, Key, Manifest};
use statute::node::{Kind, Status};
use statute::private;
use statute::project::Project;
use statute::resolve::{Reason, label, refs};
use statute::source::describe;

/// Compile a constitution and the rules a repository chooses into the
/// instruction files coding agents read. Run in the directory of
/// statute.toml.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Resolve statute.toml and write statute.lock.
    Lock,
    /// Say why a rule is enabled, along the chain that brings it in.
    Explain {
        /// The rule, as `R42`, `team:42`, or `42` for one of this repository's own.
        rule: String,
    },
    /// Enable a rule (or a pack) in statute.toml and update statute.lock.
    Add {
        /// The rule, as `R42`, `team:42`, or `42` for one of this repository's own.
        rule: String,
        /// Enable the pack rooted at this node, in `packs`.
        #[arg(long, conflicts_with = "private")]
        pack: bool,
        /// Enable it in statute.private.toml, and update statute.private.lock.
        #[arg(long)]
        private: bool,
    },
    /// Take a rule (or a pack) out of statute.toml and update statute.lock.
    Remove {
        /// The rule, as `R42`, `team:42`, or `42` for one of this repository's own.
        rule: String,
        /// Take the pack out of `packs`.
        #[arg(long, conflicts_with = "private")]
        pack: bool,
        /// Take it out of statute.private.toml, and update statute.private.lock.
        #[arg(long)]
        private: bool,
    },
    /// Write a new statute.toml in this directory, with no rules enabled.
    Init {
        /// The source of the rules and the constitution, a directory
        /// relative to this one.
        #[arg(long, default_value = ".")]
        source: String,
        /// The constitution's edition; by default the only one the source
        /// holds.
        #[arg(long)]
        edition: Option<String>,
        /// The repository is public: anyone can read it.
        #[arg(long, required_unless_present = "private", conflicts_with = "private")]
        public: bool,
        /// The repository is private.
        #[arg(long)]
        private: bool,
    },
    /// Write AGENTS.md from the constitution and the locked rule set, and
    /// next to it the skills and the checks.
    Compile {
        /// Write here instead of AGENTS.md next to statute.toml; the other
        /// files go next to it.
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    /// List the enabled rules for review by domain, with the outcome of
    /// those a script checks; exits 1 on a violation.
    Check {
        /// Check the change since this git ref (`git diff <base>...HEAD`).
        #[arg(long)]
        base: Option<String>,
    },
    /// Resolve statute.toml again, say how the result differs from
    /// statute.lock, and write statute.lock.
    Upgrade {
        /// Replace each superseded rule or pack root in statute.toml with
        /// the rule that supersedes it.
        #[arg(long)]
        replace_superseded: bool,
    },
}

/// Exit status when the rule set, the source or the arguments are wrong.
const FAILED: u8 = 1;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = Path::new(".");
    let result = match cli.command {
        Command::Lock => lock_command(root),
        Command::Explain { rule } => explain(root, &rule),
        Command::Add {
            rule,
            pack,
            private,
        } => edit(root, &rule, pack, true, private),
        Command::Remove {
            rule,
            pack,
            private,
        } => edit(root, &rule, pack, false, private),
        Command::Compile { output } => compile(root, output.as_deref()),
        Command::Check { base } => check(root, base.as_deref()),
        Command::Init {
            source,
            edition,
            public,
            private: _,
        } => init(root, &source, edition, public),
        Command::Upgrade { replace_superseded } => upgrade(root, replace_superseded),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::from(FAILED),
    }
}

fn print(diags: &Diagnostics) {
    for d in &diags.0 {
        eprintln!("{d}");
    }
}

/// Loads the project, and its private layer when `statute.private.toml`
/// is there: the public project and the one with both layers.
fn load_layers(root: &Path) -> Result<(Project, Option<Project>), ()> {
    let (project, diags) = Project::load(root);
    print(&diags);
    let project = project.ok_or(())?;
    let (both, more) = Project::load_private(&project, &diags);
    print(&more);
    if more.has_errors() {
        return Err(());
    }
    Ok((project, both))
}

/// Writes the private lock file of `both`, the project with both layers,
/// after checking that git ignores it.
fn write_private_lock(root: &Path, public: &Project, both: &Project) -> Result<(), ()> {
    statute::private::check_ignored_files(root).map_err(|e| eprintln!("statute: error: {e}"))?;
    std::fs::write(root.join(private::LOCK), lock::render(both)).map_err(|e| {
        eprintln!("{}: error: cannot write: {e}", private::LOCK);
    })?;
    print_private_locked(public, both);
    Ok(())
}

fn print_private_locked(public: &Project, both: &Project) {
    let all = both.resolution.enabled.len();
    let added = all - public.resolution.enabled.len();
    println!(
        "wrote {}: {all} rules enabled, {added} of them by the private layer",
        private::LOCK
    );
}

/// Locks the private layer again after `statute.toml` changed, when it
/// is there.
fn relock_private(root: &Path, public: &Project) -> Result<(), ()> {
    let (both, diags) = Project::load_private(public, &Diagnostics::default());
    print(&diags);
    match both {
        Some(both) => write_private_lock(root, public, &both),
        None if diags.has_errors() => Err(()),
        None => Ok(()),
    }
}

fn write_lock(root: &Path, project: &Project) -> Result<(), ()> {
    std::fs::write(root.join(lock::FILE), lock::render(project)).map_err(|e| {
        eprintln!("{}: error: cannot write: {e}", lock::FILE);
    })?;
    print_locked(project);
    Ok(())
}

fn compile(root: &Path, output: Option<&Path>) -> Result<(), ()> {
    let (project, both) = load_layers(root)?;
    let path = output.map_or_else(|| root.join(statute::compile::FILE), Path::to_path_buf);
    // The private output is made in full before anything is written.
    let full = both
        .as_ref()
        .map(statute::compile::full_text)
        .transpose()
        .map_err(|e| eprintln!("statute: error: {e}"))?;
    let mut written = statute::compile::compile(&project)
        .and_then(|out| statute::compile::write(&out, &path))
        .map_err(|e| {
            eprintln!("statute: error: {e}");
        })?;
    if let Some(full) = full {
        written.extend(private::write(root, &full).map_err(|e| eprintln!("statute: error: {e}"))?);
    }
    for path in written {
        println!("wrote {}", path.display());
    }
    let dir = path.parent().unwrap_or(Path::new(""));
    if let Some(why) = private::claude_md(dir, both.is_some()) {
        eprintln!("{}: warning: {why}", dir.join("CLAUDE.md").display());
    }
    if both.is_some() {
        for nested in private::nested_agents_md(root) {
            eprintln!("{}: warning: {}", nested.display(), private::NESTED);
        }
    }
    Ok(())
}

fn check(root: &Path, base: Option<&str>) -> Result<(), ()> {
    use statute::check::{Context, Outcome};
    let (public, both) = load_layers(root)?;
    let project = both.unwrap_or(public);
    let ctx = Context { root, base };
    let visibility = statute::check::visibility(&project, &ctx);
    let reports = statute::check::run(&project, &ctx);
    let sources = &project.sources;
    let for_reviewer =
        |r: &statute::check::Report| matches!(r.outcome, Outcome::Inconclusive | Outcome::Review);
    // The labels are read only when a rule's text is printed.
    let labels = if reports.iter().any(|(_, r)| for_reviewer(r)) {
        Some(statute::compile::labels(&project).map_err(|e| {
            eprintln!("statute: error: {e}");
        })?)
    } else {
        None
    };
    println!("## {}", manifest::FILE);
    println!(
        "\n{:<12}  public: {}",
        visibility.outcome.name(),
        visibility.note
    );
    for f in &visibility.findings {
        println!("{:<12}    {}: {}", "", f.location, f.reason);
    }
    let mut by_domain: BTreeMap<Key, Vec<_>> = BTreeMap::new();
    for entry in &reports {
        by_domain
            .entry(sources.root(&entry.0))
            .or_default()
            .push(entry);
    }
    for (domain, entries) in &by_domain {
        println!("\n## {}", label(sources, domain));
        for (key, report) in entries {
            println!(
                "\n{:<12}  {}: {}",
                report.outcome.name(),
                label(sources, key),
                report.note
            );
            for f in &report.findings {
                println!("{:<12}    {}: {}", "", f.location, f.reason);
            }
            let (true, Some(labels), Some(node)) =
                (for_reviewer(report), &labels, sources.get(key))
            else {
                continue;
            };
            let Some(rule) = &node.rule else { continue };
            let lang = &project.manifest.lang;
            let text = |part: &str, text| {
                let text = statute::compile::pick(text, node, lang);
                statute::compile::cited(&project, labels, key, part, text, false)
                    .map_err(|e| eprintln!("statute: error: {e}"))
            };
            let reason = text("reason", &rule.reason)?;
            let verification = text("verification", &rule.verification)?;
            println!("\nReason: {reason}\n\nVerification: {verification}");
        }
    }
    let violated = |r: &statute::check::Report| r.outcome == Outcome::Violation;
    if violated(&visibility) || reports.iter().any(|(_, r)| violated(r)) {
        Err(())
    } else {
        Ok(())
    }
}

/// The source's name in the rule set `init` writes.
const INIT_SOURCE: &str = manifest::FIRST_PARTY;

/// Writes a new `statute.toml` that rests on the constitution of `source`,
/// for a repository that is `public` or not, and enables no rule, after
/// trying that it resolves. An existing file is
/// left as it is.
fn init(root: &Path, source: &str, edition: Option<String>, public: bool) -> Result<(), ()> {
    let path = root.join(manifest::FILE);
    let exists = || {
        eprintln!(
            "{}: error: already exists, so it is left as it is; `statute add` and `statute remove` change it",
            manifest::FILE
        );
    };
    if path.exists() {
        exists();
        return Err(());
    }
    let edition = match edition {
        Some(edition) => edition,
        None => only_edition(&root.join(source), source)?,
    };
    let quote = |s: &str| toml_edit::Value::from(s).to_string();
    let text = format!(
        "# Written by `statute init`; `statute add` and `statute remove` change the rules.\n\
         constitution = {{ source = \"{INIT_SOURCE}\", edition = {} }}\n\
         public = {public}\n\
         rules = []\n\
         \n\
         [sources]\n\
         {INIT_SOURCE} = {{ path = {} }}\n",
        quote(&edition),
        quote(source)
    );
    let (project, diags) = Project::with_manifest(root, &text);
    print(&diags);
    if project.is_none() {
        eprintln!("statute: {} is not written", manifest::FILE);
        return Err(());
    }
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, text.as_bytes()));
    match written {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            exists();
            return Err(());
        }
        Err(e) => {
            eprintln!("{}: error: cannot write: {e}", manifest::FILE);
            return Err(());
        }
    }
    println!(
        "wrote {}: edition {edition} of the constitution, source `{INIT_SOURCE}` at {source}",
        manifest::FILE
    );
    println!("`statute add {INIT_SOURCE}<id>` enables a rule");
    Ok(())
}

/// The one edition under `<source>/constitution/`, a directory that holds
/// an `index.toml`; an error when there is none or more than one.
fn only_edition(dir: &Path, source: &str) -> Result<String, ()> {
    let shown = Path::new(source).join("constitution");
    let entries = std::fs::read_dir(dir.join("constitution")).map_err(|e| {
        eprintln!("{}: error: cannot read the directory: {e}", shown.display());
    })?;
    let mut editions = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| {
            eprintln!("{}: error: cannot read the directory: {e}", shown.display());
        })?;
        let index = entry.path().join("index.toml");
        match index.try_exists() {
            Ok(true) => editions.extend(entry.file_name().into_string().ok()),
            Ok(false) => {}
            Err(e) => {
                eprintln!(
                    "{}: error: cannot read: {e}",
                    shown.join(entry.file_name()).join("index.toml").display()
                );
                return Err(());
            }
        }
    }
    editions.sort();
    match editions.as_slice() {
        [edition] => Ok(edition.clone()),
        [] => {
            eprintln!(
                "{}: error: holds no edition of the constitution (a directory with an index.toml)",
                shown.display()
            );
            Err(())
        }
        _ => {
            eprintln!(
                "{}: error: holds editions {}; choose one with --edition",
                shown.display(),
                editions.join(", ")
            );
            Err(())
        }
    }
}

fn lock_command(root: &Path) -> Result<(), ()> {
    let (project, both) = load_layers(root)?;
    // A lock file that cannot be read has nothing to compare with.
    if let Some(old) = std::fs::read_to_string(root.join(lock::FILE))
        .ok()
        .and_then(|text| lock::read(&text).ok())
    {
        for (key, revision) in same_revision_new_content(&old, &project) {
            eprintln!(
                "{}: warning: rule {} changed since {} and is still at revision {revision}; {}",
                project.sources.file(&key),
                label(&project.sources, &key),
                lock::FILE,
                RAISE_REVISION
            );
        }
    }
    write_lock(root, &project)?;
    match both {
        Some(both) => write_private_lock(root, &project, &both),
        None => Ok(()),
    }
}

/// What to do about a rule edited without a new revision
/// (`docs/design/rule-format.md`, IDs and revisions).
const RAISE_REVISION: &str = "an edited rule's revision goes up by one";

/// The enabled rules whose file differs from the one `old` locked while
/// the revision is the same, each with that revision.
fn same_revision_new_content(old: &lock::Locked, project: &Project) -> Vec<(Key, u64)> {
    lock::current(project)
        .rules
        .into_iter()
        .filter_map(|(key, (revision, sha256))| match old.rules.get(&key) {
            Some((was, before)) if *was == revision && *before != sha256 => Some((key, revision)),
            _ => None,
        })
        .collect()
}

/// `upgrade`: resolves the rule set as `lock` does, after replacing
/// superseded rules when asked, prints what changed against the lock file,
/// and writes the lock file (and the rule set, when it changed).
fn upgrade(root: &Path, replace_superseded: bool) -> Result<(), ()> {
    let old = match std::fs::read_to_string(root.join(lock::FILE)) {
        Ok(text) => lock::read(&text).map_err(|e| {
            eprintln!("{}: error: {e}", lock::FILE);
        })?,
        Err(e) => {
            eprintln!(
                "{}: error: cannot read: {e}; `statute lock` writes it",
                lock::FILE
            );
            return Err(());
        }
    };
    let (project, diags) = Project::load(root);
    let Some(project) = project else {
        print(&diags);
        return Err(());
    };
    let replaced = if replace_superseded {
        replace(root, &project)?
    } else {
        None
    };
    let project = match replaced {
        Some((original, changed, lines)) => {
            let (next, diags) = Project::with_manifest(root, &changed);
            print(&diags);
            let Some(next) = next else {
                eprintln!(
                    "statute: {} and {} are left unchanged",
                    manifest::FILE,
                    lock::FILE
                );
                return Err(());
            };
            for line in lines {
                println!("{line}");
            }
            report_changes(&old, &next);
            write_both(
                root,
                false,
                (manifest::FILE, lock::FILE),
                &original,
                &changed,
                &lock::render(&next),
            )?;
            print_locked(&next);
            return relock_private(root, &next);
        }
        None => {
            print(&diags);
            project
        }
    };
    report_changes(&old, &project);
    write_lock(root, &project)?;
    relock_private(root, &project)
}

/// Prints how `project` differs from the lock file `old`.
fn report_changes(old: &lock::Locked, project: &Project) {
    let new = lock::current(project);
    let sources = &project.sources;
    let mut any = false;
    let mut line = |text: String| {
        any = true;
        println!("{text}");
    };
    if old.edition != new.edition {
        line(format!(
            "constitution: edition {} -> {}",
            old.edition, new.edition
        ));
    }
    if old.revision != new.revision {
        line(format!(
            "constitution: revision {} -> {}",
            old.revision, new.revision
        ));
    }
    for (key, (revision, sha256)) in &new.rules {
        // An overridden rule that changed asks whether the override still
        // holds.
        let still = match project.resolution.overridden.get(key) {
            Some(by) => format!("; {by} overrides it, so check that the override still holds"),
            None => String::new(),
        };
        match old.rules.get(key) {
            Some((was, _)) if was != revision => line(format!(
                "changed: rule {}: revision {was} -> {revision}{still}",
                label(sources, key)
            )),
            Some((_, was)) if was != sha256 => line(format!(
                "changed: rule {}: content changed, still revision {revision}; {RAISE_REVISION}{still}",
                label(sources, key)
            )),
            Some(_) => {}
            None => line(format!(
                "brought in: rule {}, {}",
                label(sources, key),
                why(project, key)
            )),
        }
    }
    for key in old.rules.keys().filter(|k| !new.rules.contains_key(k)) {
        line(format!("dropped: rule {}", label(sources, key)));
    }
    if !any {
        println!("{} is up to date", lock::FILE);
    }
}

/// The reasons `key` is enabled, one step down the chain `explain`
/// follows.
fn why(project: &Project, key: &Key) -> String {
    let sources = &project.sources;
    let reasons = project.resolution.enabled.get(key);
    reasons
        .into_iter()
        .flatten()
        .map(|r| match r {
            Reason::Direct => "listed in `rules`".to_owned(),
            Reason::Pack(root) => format!("in pack {}", label(sources, root)),
            Reason::ParentOf(c) => format!("the parent of {}", label(sources, c)),
            Reason::RequiredBy(b) => format!("required by {}", label(sources, b)),
            Reason::OverriddenBy(b) => format!("overridden by {}", label(sources, b)),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// What takes the place of superseded `key`, following the chain while
/// each rule on it has one successor: nothing when `key` is not
/// superseded, the rule at the end of the chain, or the successors of the
/// rule where the chain splits.
fn successors(project: &Project, key: &Key) -> Vec<Key> {
    let mut seen = vec![key.clone()];
    let mut at = key.clone();
    while let Some(node) = project.sources.get(&at)
        && node.status == Status::Superseded
    {
        match statute::resolve::successors(&at, node).as_slice() {
            [next] if !seen.contains(next) => {
                seen.push(next.clone());
                at = next.clone();
            }
            [_] | [] => break,
            many => return many.to_vec(),
        }
    }
    if at == *key { Vec::new() } else { vec![at] }
}

/// The rule set with each superseded entry of `rules` and `packs` replaced
/// by its successor, or taken out when the successor is listed already; an
/// entry with more than one successor is left for the user to choose:
/// the original text, the changed one and a line per replacement. `None`
/// when nothing is superseded.
fn replace(root: &Path, project: &Project) -> Result<Option<(String, String, Vec<String>)>, ()> {
    let original = std::fs::read_to_string(root.join(manifest::FILE)).map_err(|e| {
        eprintln!("{}: error: cannot read: {e}", manifest::FILE);
    })?;
    let mut doc: DocumentMut = original
        .parse()
        .map_err(|e| eprintln!("{}: error: {e}", manifest::FILE))?;
    let m = &project.manifest;
    let mut lines = Vec::new();
    for field in ["rules", "packs"] {
        let Some(list) = doc.get_mut(field).and_then(Item::as_array_mut) else {
            continue;
        };
        // Manifest::parse has already checked that each entry names a
        // rule of a source of the rule set.
        let keys: Vec<Option<Key>> = list.iter().map(|v| listed_key(v, m)).collect();
        // The keys listed once this pass is done, so that two rules with
        // one successor do not both become it.
        let mut listed = keys.clone();
        for i in (0..keys.len()).rev() {
            let Some(old) = &keys[i] else { continue };
            let new = match successors(project, old).as_slice() {
                [] => continue,
                [new] => new.clone(),
                many => {
                    lines.push(format!(
                        "left {old} in `{field}`: superseded by {}; choose among them with `statute add` and `statute remove`",
                        refs(many)
                    ));
                    continue;
                }
            };
            if listed.contains(&Some(new.clone())) {
                let removed = list.remove(i);
                // The next entry takes the place of the first, spacing
                // included.
                if i == 0
                    && let (Some(prefix), Some(first)) =
                        (removed.decor().prefix().cloned(), list.get_mut(0))
                {
                    first.decor_mut().set_prefix(prefix);
                }
                lines.push(format!(
                    "took {old} out of `{field}`: superseded by {new}, which it lists already"
                ));
            } else {
                let decor = list.get(i).map(|v| v.decor().clone());
                list.replace(i, Manifest::entry(&new));
                listed.push(Some(new.clone()));
                if let (Some(decor), Some(v)) = (decor, list.get_mut(i)) {
                    *v.decor_mut() = decor;
                }
                lines.push(format!("replaced {old} with {new} in `{field}`"));
            }
        }
    }
    if lines.is_empty() {
        return Ok(None);
    }
    lines.reverse();
    let changed = doc.to_string();
    Ok(Some((original, changed, lines)))
}

/// The key a command-line reference names in the rule set, written as in
/// `statute.toml`.
fn parse_ref(text: &str, manifest: &Manifest) -> Result<Key, ()> {
    manifest
        .key(text)
        .map_err(|e| eprintln!("statute: error: {e}"))
}

/// The key an entry of `rules` or `packs` names.
fn listed_key(value: &Value, manifest: &Manifest) -> Option<Key> {
    manifest.key(value.as_str()?).ok()
}

fn explain(root: &Path, text: &str) -> Result<(), ()> {
    let (public, both) = load_layers(root)?;
    let project = both.unwrap_or(public);
    let key = parse_ref(text, &project.manifest)?;
    let sources = &project.sources;
    let Some(node) = sources.get(&key) else {
        eprintln!(
            "statute: error: {key} does not exist in {}",
            describe(&key.source)
        );
        return Err(());
    };
    match node.kind {
        Kind::Rule => {}
        Kind::Domain => {
            println!(
                "{} is a domain: it is never enabled itself, and gives a heading to the enabled rules under it",
                label(sources, &key)
            );
            return Ok(());
        }
        Kind::OneOf => {
            let chosen: Vec<String> = sources
                .children(&key)
                .iter()
                .filter(|c| project.resolution.enabled.contains_key(*c))
                .map(|c| label(sources, c))
                .collect();
            if chosen.is_empty() {
                println!(
                    "one-of {} is not enabled: none of its children is",
                    label(sources, &key)
                );
            } else {
                println!(
                    "one-of {} is enabled through its child {}",
                    label(sources, &key),
                    chosen.join(", ")
                );
            }
            return Ok(());
        }
    }
    if !project.resolution.enabled.contains_key(&key) {
        println!(
            "rule {} is not enabled by the rule set",
            label(sources, &key)
        );
        return Err(());
    }
    let res = &project.resolution;
    if res.overridden.contains_key(&key) {
        println!(
            "rule {} is overridden by {}, which stands in its place; it counts as enabled:",
            label(sources, &key),
            label(sources, &res.standing(&key))
        );
    } else {
        println!("rule {} is enabled:", label(sources, &key));
    }
    let chain: Vec<String> = res.chain(&key).iter().map(|k| label(sources, k)).collect();
    if !chain.is_empty() {
        println!("- it overrides {}", chain.join(", which overrides "));
    }
    let mut out = String::new();
    reasons(&project, &key, 0, &mut vec![key.clone()], &mut out);
    print!("{out}");
    Ok(())
}

/// Writes why `key` is enabled, one line per reason, and under each rule
/// that brought it in, why that rule is enabled, down to the rules chosen
/// directly.
fn reasons(project: &Project, key: &Key, depth: usize, path: &mut Vec<Key>, out: &mut String) {
    let sources = &project.sources;
    let indent = "  ".repeat(depth);
    let Some(set) = project.resolution.enabled.get(key) else {
        return;
    };
    for reason in set {
        let by = match reason {
            Reason::Direct => {
                out.push_str(&format!("{indent}- listed in `rules`\n"));
                continue;
            }
            Reason::Pack(root) => {
                out.push_str(&format!("{indent}- in pack {}\n", label(sources, root)));
                continue;
            }
            Reason::ParentOf(child) => {
                out.push_str(&format!(
                    "{indent}- the parent of {}, which is enabled:\n",
                    label(sources, child)
                ));
                child
            }
            Reason::RequiredBy(by) => {
                out.push_str(&format!(
                    "{indent}- required by {}, which is enabled:\n",
                    label(sources, by)
                ));
                by
            }
            Reason::OverriddenBy(by) => {
                out.push_str(&format!(
                    "{indent}- overridden by {}, which stands in its place and is enabled:\n",
                    label(sources, by)
                ));
                by
            }
        };
        if !path.contains(by) {
            path.push(by.clone());
            reasons(project, by, depth + 1, path, out);
            path.pop();
        }
    }
}

/// `add` (`adding`) or `remove`: changes the list in statute.toml, or
/// with `private` in statute.private.toml, keeping the rest of the file as
/// written, resolves the result, and writes the file and its lock file
/// only when it resolves. A change to statute.toml locks the private
/// layer again, when there is one.
fn edit(root: &Path, text: &str, pack: bool, adding: bool, private: bool) -> Result<(), ()> {
    let file = if private {
        private::MANIFEST
    } else {
        manifest::FILE
    };
    let (public, given) = if private {
        let (public, given) = Project::load(root);
        print(&given);
        (Some(public.ok_or(())?), given)
    } else {
        (None, Diagnostics::default())
    };
    let original = std::fs::read_to_string(root.join(file)).map_err(|e| {
        eprintln!("{file}: error: cannot read: {e}");
    })?;
    let parsed = match &public {
        Some(public) => public.manifest.with_private(&original),
        None => Manifest::parse(&original),
    };
    let manifest = parsed.map_err(|errors| {
        for e in errors {
            eprintln!("{file}: error: {e}");
        }
    })?;
    let key = parse_ref(text, &manifest)?;
    let entry = Manifest::entry(&key);
    let field = if pack { "packs" } else { "rules" };

    let mut doc: DocumentMut = original
        .parse()
        .map_err(|e| eprintln!("{file}: error: {e}"))?;
    let list = doc
        .entry(field)
        .or_insert_with(|| Item::Value(Value::Array(Array::new())))
        .as_array_mut()
        .ok_or_else(|| eprintln!("{file}: error: `{field}` is not a list"))?;
    // Entries of the list that name this key; the rule set has been read,
    // so each names a rule of one of its sources.
    let position = list
        .iter()
        .position(|v| listed_key(v, &manifest).as_ref() == Some(&key));
    match (adding, position) {
        (true, Some(_)) => {
            println!("{key} is already in `{field}`");
            return Ok(());
        }
        (true, None) => list.push(entry.as_str()),
        (false, Some(i)) => {
            list.remove(i);
        }
        (false, None) => {
            eprintln!("{file}: error: `{field}` does not list {key}");
            return Err(());
        }
    }

    let changed = doc.to_string();
    let (project, diags) = match &public {
        Some(public) => Project::with_private(public, &given, &changed),
        None => Project::with_manifest(root, &changed),
    };
    print(&diags);
    let lock_file = if private { private::LOCK } else { lock::FILE };
    let Some(project) = project else {
        eprintln!("statute: {file} and {lock_file} are left unchanged");
        return Err(());
    };
    if private {
        private::check_ignored_files(root).map_err(|e| eprintln!("statute: error: {e}"))?;
    }
    write_both(
        root,
        private,
        (file, lock_file),
        &original,
        &changed,
        &lock::render(&project),
    )?;
    if adding {
        println!("added {key} to `{field}`");
    } else {
        println!("removed {key} from `{field}`");
        if !pack && project.resolution.enabled.contains_key(&key) {
            println!("{key} is still enabled; `statute explain {entry}` says why");
        }
    }
    match &public {
        Some(public) => print_private_locked(public, &project),
        None => {
            print_locked(&project);
            relock_private(root, &project)?;
        }
    }
    Ok(())
}

/// Writes a changed rule set, `files.0`, and its lock file, `files.1`,
/// so that the two stay in step: both go to temporary files first (for
/// the `private` layer, where git ignores them), then each is renamed
/// into place, and if the lock file cannot be, the rule set is put back.
fn write_both(
    root: &Path,
    private: bool,
    files: (&str, &str),
    original: &str,
    changed: &str,
    lock_text: &str,
) -> Result<(), ()> {
    let (manifest_file, lock_file) = files;
    let manifest_path = root.join(manifest_file);
    let lock_path = root.join(lock_file);
    let tmp = |file: &str| {
        root.join(if private {
            private::staged(file)
        } else {
            PathBuf::from(format!("{file}.tmp"))
        })
    };
    let manifest_tmp = tmp(manifest_file);
    let lock_tmp = tmp(lock_file);
    if private && let Err(e) = std::fs::create_dir_all(root.join(private::DIR)) {
        eprintln!("{}: error: cannot write: {e}", private::DIR);
        return Err(());
    }
    let unchanged = || {
        let _ = std::fs::remove_file(&manifest_tmp);
        let _ = std::fs::remove_file(&lock_tmp);
        eprintln!("statute: {manifest_file} and {lock_file} are left unchanged");
    };
    let staged = std::fs::write(&lock_tmp, lock_text)
        .map_err(|e| (lock_file, e))
        .and_then(|()| std::fs::write(&manifest_tmp, changed).map_err(|e| (manifest_file, e)))
        .and_then(|()| {
            std::fs::rename(&manifest_tmp, &manifest_path).map_err(|e| (manifest_file, e))
        });
    if let Err((file, e)) = staged {
        eprintln!("{file}: error: cannot write: {e}");
        unchanged();
        return Err(());
    }
    if let Err(e) = std::fs::rename(&lock_tmp, &lock_path) {
        eprintln!("{lock_file}: error: cannot write: {e}");
        if let Err(e) = std::fs::write(&manifest_path, original) {
            eprintln!(
                "{manifest_file}: error: cannot put back the rule set as it was: {e}; {lock_file} does not match it"
            );
            let _ = std::fs::remove_file(&lock_tmp);
            return Err(());
        }
        unchanged();
        return Err(());
    }
    Ok(())
}

fn print_locked(project: &Project) {
    println!(
        "wrote {}: {} rules enabled",
        lock::FILE,
        project.resolution.enabled.len()
    );
}
