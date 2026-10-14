use std::path::Path;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use toml_edit::{Array, DocumentMut, Item, Value};

use statute::diag::Diagnostics;
use statute::lock;
use statute::manifest::{self, Manifest, RuleRef, SourceSpec, show};
use statute::node::{Id, Kind, Status};
use statute::project::Project;
use statute::resolve::{Reason, label, refs};

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
        #[arg(long)]
        pack: bool,
    },
    /// Take a rule (or a pack) out of statute.toml and update statute.lock.
    Remove {
        /// The rule, as `R42`, `team:42`, or `42` for one of this repository's own.
        rule: String,
        /// Take the pack out of `packs`.
        #[arg(long)]
        pack: bool,
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
    },
    /// Write AGENTS.md from the constitution and the locked rule set, and
    /// next to it the skills and the checks.
    Compile {
        /// Write here instead of AGENTS.md next to statute.toml; the other
        /// files go next to it.
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    /// List the enabled rules for review, with the outcome of those a
    /// script checks; exits 1 on a violation.
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
        Command::Add { rule, pack } => edit(root, &rule, pack, true),
        Command::Remove { rule, pack } => edit(root, &rule, pack, false),
        Command::Compile { output } => compile(root, output.as_deref()),
        Command::Check { base } => check(root, base.as_deref()),
        Command::Init { source, edition } => init(root, &source, edition),
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

/// Loads and resolves the project, printing every finding; `Err` when any
/// is an error.
fn load(root: &Path) -> Result<Project, ()> {
    let (project, diags) = Project::load(root);
    print(&diags);
    project.ok_or(())
}

fn write_lock(root: &Path, project: &Project) -> Result<(), ()> {
    std::fs::write(root.join(lock::FILE), lock::render(project)).map_err(|e| {
        eprintln!("{}: error: cannot write: {e}", lock::FILE);
    })?;
    print_locked(project);
    Ok(())
}

fn compile(root: &Path, output: Option<&Path>) -> Result<(), ()> {
    let project = load(root)?;
    let path = output.map_or_else(|| root.join(statute::compile::FILE), Path::to_path_buf);
    let written = statute::compile::compile(&project)
        .and_then(|out| statute::compile::write(&out, &path))
        .map_err(|e| {
            eprintln!("statute: error: {e}");
        })?;
    for path in written {
        println!("wrote {}", path.display());
    }
    Ok(())
}

fn check(root: &Path, base: Option<&str>) -> Result<(), ()> {
    use statute::check::{Context, Outcome};
    let project = load(root)?;
    let reports = statute::check::run(&project, &Context { root, base });
    let source = &project.source;
    for (id, report) in &reports {
        println!(
            "{:<12}  {}: {}",
            report.outcome.name(),
            label(source, *id),
            report.note
        );
        for f in &report.findings {
            println!("{:<12}    {}: {}", "", f.location, f.reason);
        }
    }
    let for_reviewer: Vec<_> = reports
        .iter()
        .filter(|(_, r)| matches!(r.outcome, Outcome::Inconclusive | Outcome::Review))
        .collect();
    if !for_reviewer.is_empty() {
        let labels = statute::compile::labels(&project).map_err(|e| {
            eprintln!("statute: error: {e}");
        })?;
        println!("\nFor the reviewer:");
        for (id, report) in for_reviewer {
            let Some(node) = source.get(*id) else {
                continue;
            };
            let Some(rule) = &node.rule else { continue };
            let lang = &project.manifest.lang;
            let text = |part: &str, text| {
                let text = statute::compile::pick(text, node, lang);
                statute::compile::cited(&project, &labels, node, part, text, false)
                    .map_err(|e| eprintln!("statute: error: {e}"))
            };
            let reason = text("reason", &rule.reason)?;
            let verification = text("verification", &rule.verification)?;
            println!("\n## {} ({})\n", label(source, *id), report.outcome.name());
            println!("Reason: {reason}\n");
            println!("Verification: {verification}");
        }
    }
    if reports.iter().any(|(_, r)| r.outcome == Outcome::Violation) {
        Err(())
    } else {
        Ok(())
    }
}

/// The source's name in the rule set `init` writes.
const INIT_SOURCE: &str = manifest::FIRST_PARTY;

/// Writes a new `statute.toml` that rests on the constitution of `source`
/// and enables no rule, after trying that it resolves. An existing file is
/// left as it is.
fn init(root: &Path, source: &str, edition: Option<String>) -> Result<(), ()> {
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
    let project = load(root)?;
    // A lock file that cannot be read has nothing to compare with.
    if let Some(old) = std::fs::read_to_string(root.join(lock::FILE))
        .ok()
        .and_then(|text| lock::read(&text).ok())
    {
        for (id, revision) in same_revision_new_content(&old, &project) {
            eprintln!(
                "{}: warning: rule {} changed since {} and is still at revision {revision}; {}",
                statute::project::shown(
                    &project.manifest.source.path,
                    &format!("rules/{}", statute::node::file_name(id))
                ),
                label(&project.source, id),
                lock::FILE,
                RAISE_REVISION
            );
        }
    }
    write_lock(root, &project)
}

/// What to do about a rule edited without a new revision
/// (`docs/design/rule-system.md`, IDs and revisions).
const RAISE_REVISION: &str = "an edited rule's revision goes up by one";

/// The enabled rules whose file differs from the one `old` locked while
/// the revision is the same, each with that revision.
fn same_revision_new_content(old: &lock::Locked, project: &Project) -> Vec<(Id, u64)> {
    lock::current(project)
        .rules
        .into_iter()
        .filter_map(|(id, (revision, sha256))| match old.rules.get(&id) {
            Some((was, before)) if *was == revision && *before != sha256 => Some((id, revision)),
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
            write_both(root, &original, &changed, &next)?;
            print_locked(&next);
            return Ok(());
        }
        None => {
            print(&diags);
            project
        }
    };
    report_changes(&old, &project);
    write_lock(root, &project)
}

/// Prints how `project` differs from the lock file `old`.
fn report_changes(old: &lock::Locked, project: &Project) {
    let new = lock::current(project);
    let source = &project.source;
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
    for (id, (revision, sha256)) in &new.rules {
        match old.rules.get(id) {
            Some((was, _)) if was != revision => line(format!(
                "changed: rule {}: revision {was} -> {revision}",
                label(source, *id)
            )),
            Some((_, was)) if was != sha256 => line(format!(
                "changed: rule {}: content changed, still revision {revision}; {RAISE_REVISION}",
                label(source, *id)
            )),
            Some(_) => {}
            None => line(format!(
                "brought in: rule {}, {}",
                label(source, *id),
                why(project, *id)
            )),
        }
    }
    for id in old.rules.keys().filter(|id| !new.rules.contains_key(id)) {
        line(format!("dropped: rule {}", label(source, *id)));
    }
    if !any {
        println!("{} is up to date", lock::FILE);
    }
}

/// The reasons `id` is enabled, one step down the chain `explain` follows.
fn why(project: &Project, id: Id) -> String {
    let source = &project.source;
    let reasons = project.resolution.enabled.get(&id);
    reasons
        .into_iter()
        .flatten()
        .map(|r| match *r {
            Reason::Direct => "listed in `rules`".to_owned(),
            Reason::Pack(root) => format!("in pack {}", label(source, root)),
            Reason::ParentOf(c) => format!("the parent of {}", label(source, c)),
            Reason::RequiredBy(b) => format!("required by {}", label(source, b)),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// What takes the place of superseded `id`, following the chain while
/// each rule on it has one successor: nothing when `id` is not superseded,
/// the rule at the end of the chain, or the successors of the rule where
/// the chain splits.
fn successors(project: &Project, id: Id) -> Vec<Id> {
    let mut seen = vec![id];
    let mut at = id;
    while let Some(node) = project.source.get(at)
        && node.status == Status::Superseded
    {
        match node.superseded_by.as_slice() {
            [next] if !seen.contains(next) => {
                seen.push(*next);
                at = *next;
            }
            [_] | [] => break,
            many => return many.to_vec(),
        }
    }
    if at == id { Vec::new() } else { vec![at] }
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
    let spec = &project.manifest.source;
    let show = |id: Id| show(&spec.name, id);
    let mut lines = Vec::new();
    for field in ["rules", "packs"] {
        let Some(list) = doc.get_mut(field).and_then(Item::as_array_mut) else {
            continue;
        };
        // Manifest::parse has already checked that each entry names a
        // rule of the one source.
        let ids: Vec<Option<Id>> = list.iter().map(|v| listed_id(v, spec)).collect();
        // The IDs listed once this pass is done, so that two rules with
        // one successor do not both become it.
        let mut listed = ids.clone();
        for i in (0..ids.len()).rev() {
            let Some(old) = ids[i] else { continue };
            let new = match successors(project, old).as_slice() {
                [] => continue,
                [new] => *new,
                many => {
                    lines.push(format!(
                        "left {} in `{field}`: superseded by {}; choose among them with `statute add` and `statute remove`",
                        show(old),
                        refs(&project.source, many)
                    ));
                    continue;
                }
            };
            if listed.contains(&Some(new)) {
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
                    "took {} out of `{field}`: superseded by {}, which it lists already",
                    show(old),
                    show(new)
                ));
            } else {
                let decor = list.get(i).map(|v| v.decor().clone());
                list.replace(i, show(new));
                listed.push(Some(new));
                if let (Some(decor), Some(v)) = (decor, list.get_mut(i)) {
                    *v.decor_mut() = decor;
                }
                lines.push(format!(
                    "replaced {} with {} in `{field}`",
                    show(old),
                    show(new)
                ));
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

/// The ID a command-line reference names in the rule set's source,
/// written as in `statute.toml`.
fn parse_ref(text: &str, source: &SourceSpec) -> Result<Id, ()> {
    RuleRef::parse(text)
        .and_then(|r| source.id_of(&r).map_err(|e| format!("`{text}` {e}")))
        .map_err(|e| eprintln!("statute: error: {e}"))
}

/// The ID an entry of `rules` or `packs` names, when it is a reference to
/// a rule of `source`.
fn listed_id(value: &Value, source: &SourceSpec) -> Option<Id> {
    let r = RuleRef::parse(value.as_str()?).ok()?;
    source.id_of(&r).ok()
}

fn explain(root: &Path, text: &str) -> Result<(), ()> {
    let project = load(root)?;
    let id = parse_ref(text, &project.manifest.source)?;
    let source = &project.source;
    let Some(node) = source.get(id) else {
        eprintln!(
            "statute: error: {} does not exist in source `{}`",
            source.show(id),
            project.manifest.source.name
        );
        return Err(());
    };
    match node.kind {
        Kind::Rule => {}
        Kind::Domain => {
            println!(
                "{} is a domain: it is never enabled itself, and gives a heading to the enabled rules under it",
                label(source, id)
            );
            return Ok(());
        }
        Kind::OneOf => {
            let chosen: Vec<String> = source
                .children(id)
                .filter(|c| project.resolution.enabled.contains_key(&c.id))
                .map(|c| label(source, c.id))
                .collect();
            if chosen.is_empty() {
                println!(
                    "one-of {} is not enabled: none of its children is",
                    label(source, id)
                );
            } else {
                println!(
                    "one-of {} is enabled through its child {}",
                    label(source, id),
                    chosen.join(", ")
                );
            }
            return Ok(());
        }
    }
    if !project.resolution.enabled.contains_key(&id) {
        println!("rule {} is not enabled by the rule set", label(source, id));
        return Err(());
    }
    println!("rule {} is enabled:", label(source, id));
    let mut out = String::new();
    reasons(&project, id, 0, &mut vec![id], &mut out);
    print!("{out}");
    Ok(())
}

/// Writes why `id` is enabled, one line per reason, and under each rule
/// that brought it in, why that rule is enabled, down to the rules chosen
/// directly.
fn reasons(project: &Project, id: Id, depth: usize, path: &mut Vec<Id>, out: &mut String) {
    let source = &project.source;
    let indent = "  ".repeat(depth);
    let Some(set) = project.resolution.enabled.get(&id) else {
        return;
    };
    for reason in set {
        let by = match *reason {
            Reason::Direct => {
                out.push_str(&format!("{indent}- listed in `rules`\n"));
                continue;
            }
            Reason::Pack(root) => {
                out.push_str(&format!("{indent}- in pack {}\n", label(source, root)));
                continue;
            }
            Reason::ParentOf(child) => {
                out.push_str(&format!(
                    "{indent}- the parent of {}, which is enabled:\n",
                    label(source, child)
                ));
                child
            }
            Reason::RequiredBy(by) => {
                out.push_str(&format!(
                    "{indent}- required by {}, which is enabled:\n",
                    label(source, by)
                ));
                by
            }
        };
        if !path.contains(&by) {
            path.push(by);
            reasons(project, by, depth + 1, path, out);
            path.pop();
        }
    }
}

/// `add` (`adding`) or `remove`: changes the list in statute.toml, keeping
/// the rest of the file as written, resolves the result, and writes both
/// files only when it resolves.
fn edit(root: &Path, text: &str, pack: bool, adding: bool) -> Result<(), ()> {
    let path = root.join(manifest::FILE);
    let original = std::fs::read_to_string(&path).map_err(|e| {
        eprintln!("{}: error: cannot read: {e}", manifest::FILE);
    })?;
    let manifest = Manifest::parse(&original).map_err(|errors| {
        for e in errors {
            eprintln!("{}: error: {e}", manifest::FILE);
        }
    })?;
    let id = parse_ref(text, &manifest.source)?;
    let entry = show(&manifest.source.name, id);
    let field = if pack { "packs" } else { "rules" };

    let mut doc: DocumentMut = original
        .parse()
        .map_err(|e| eprintln!("{}: error: {e}", manifest::FILE))?;
    let list = doc
        .entry(field)
        .or_insert_with(|| Item::Value(Value::Array(Array::new())))
        .as_array_mut()
        .ok_or_else(|| eprintln!("{}: error: `{field}` is not a list", manifest::FILE))?;
    // Entries of the list that name this ID; Manifest::parse has already
    // checked that each names a rule of the one source.
    let position = list
        .iter()
        .position(|v| listed_id(v, &manifest.source) == Some(id));
    match (adding, position) {
        (true, Some(_)) => {
            println!("{entry} is already in `{field}`");
            return Ok(());
        }
        (true, None) => list.push(entry.as_str()),
        (false, Some(i)) => {
            list.remove(i);
        }
        (false, None) => {
            eprintln!("{}: error: `{field}` does not list {entry}", manifest::FILE);
            return Err(());
        }
    }

    let changed = doc.to_string();
    let (project, diags) = Project::with_manifest(root, &changed);
    print(&diags);
    let Some(project) = project else {
        eprintln!(
            "statute: {} and {} are left unchanged",
            manifest::FILE,
            lock::FILE
        );
        return Err(());
    };
    write_both(root, &original, &changed, &project)?;
    if adding {
        println!("added {entry} to `{field}`");
    } else {
        println!("removed {entry} from `{field}`");
        if !pack && project.resolution.enabled.contains_key(&id) {
            println!("{entry} is still enabled; `statute explain {entry}` says why");
        }
    }
    print_locked(&project);
    Ok(())
}

/// Writes the changed `statute.toml` and its lock file so that the two
/// stay in step: both go to temporary files first, then each is renamed
/// into place, and if the lock file cannot be, the rule set is put back.
fn write_both(root: &Path, original: &str, changed: &str, project: &Project) -> Result<(), ()> {
    let manifest_path = root.join(manifest::FILE);
    let lock_path = root.join(lock::FILE);
    let manifest_tmp = root.join(format!("{}.tmp", manifest::FILE));
    let lock_tmp = root.join(format!("{}.tmp", lock::FILE));
    let unchanged = || {
        let _ = std::fs::remove_file(&manifest_tmp);
        let _ = std::fs::remove_file(&lock_tmp);
        eprintln!(
            "statute: {} and {} are left unchanged",
            manifest::FILE,
            lock::FILE
        );
    };
    let staged = std::fs::write(&lock_tmp, lock::render(project))
        .map_err(|e| (lock::FILE, e))
        .and_then(|()| std::fs::write(&manifest_tmp, changed).map_err(|e| (manifest::FILE, e)))
        .and_then(|()| {
            std::fs::rename(&manifest_tmp, &manifest_path).map_err(|e| (manifest::FILE, e))
        });
    if let Err((file, e)) = staged {
        eprintln!("{file}: error: cannot write: {e}");
        unchanged();
        return Err(());
    }
    if let Err(e) = std::fs::rename(&lock_tmp, &lock_path) {
        eprintln!("{}: error: cannot write: {e}", lock::FILE);
        if let Err(e) = std::fs::write(&manifest_path, original) {
            eprintln!(
                "{}: error: cannot put back the rule set as it was: {e}; {} does not match it",
                manifest::FILE,
                lock::FILE
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
