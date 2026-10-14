use std::path::Path;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use toml_edit::{Array, DocumentMut, Item, Value};

use statute::diag::Diagnostics;
use statute::lock;
use statute::manifest::{self, Manifest};
use statute::node::{Id, Kind, id_from_file_name};
use statute::project::Project;
use statute::resolve::{Reason, label};

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
        /// The rule, as `ac:42` or `42`.
        rule: String,
    },
    /// Enable a rule (or a pack) in statute.toml and update statute.lock.
    Add {
        /// The rule, as `ac:42` or `42`.
        rule: String,
        /// Enable the pack rooted at this node, in `packs`.
        #[arg(long)]
        pack: bool,
    },
    /// Take a rule (or a pack) out of statute.toml and update statute.lock.
    Remove {
        /// The rule, as `ac:42` or `42`.
        rule: String,
        /// Take the pack out of `packs`.
        #[arg(long)]
        pack: bool,
    },
    /// Write a new statute.toml (not implemented yet).
    Init,
    /// Write AGENTS.md from the constitution and the locked rule set.
    Compile {
        /// Write here instead of AGENTS.md next to statute.toml.
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    /// Run the checks of the enabled rules; exits 1 on a violation.
    Check {
        /// Check the change since this git ref (`git diff <base>...HEAD`).
        #[arg(long)]
        base: Option<String>,
    },
    /// Move the sources and the constitution to newer versions (not
    /// implemented yet).
    Upgrade,
}

/// Exit status when the rule set, the source or the arguments are wrong.
const FAILED: u8 = 1;
/// Exit status of a command that does not exist yet.
const NOT_IMPLEMENTED: u8 = 2;

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
        Command::Init | Command::Upgrade => {
            eprintln!("statute: this command is not implemented yet");
            return ExitCode::from(NOT_IMPLEMENTED);
        }
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
    let text = statute::compile::compile(&project).map_err(|e| {
        eprintln!("statute: error: {e}");
    })?;
    let path = output.map_or_else(|| root.join(statute::compile::FILE), Path::to_path_buf);
    if let Ok(existing) = std::fs::read(&path)
        && !existing.starts_with(format!("{}\n", statute::compile::GENERATED).as_bytes())
    {
        eprintln!(
            "{}: error: not written by statute, so it is left as it is; write elsewhere with --output, or move the file away first",
            path.display()
        );
        return Err(());
    }
    std::fs::write(&path, text).map_err(|e| {
        eprintln!("{}: error: cannot write: {e}", path.display());
    })?;
    println!("wrote {}", path.display());
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
        println!("\nFor the reviewer:");
        for (id, report) in for_reviewer {
            let Some(node) = source.get(*id) else {
                continue;
            };
            let Some(rule) = &node.rule else { continue };
            let lang = &project.manifest.lang;
            println!("\n## {} ({})\n", label(source, *id), report.outcome.name());
            println!(
                "Reason: {}\n",
                statute::compile::pick(&rule.reason, node, lang)
            );
            println!(
                "Verification: {}",
                statute::compile::pick(&rule.verification, node, lang)
            );
        }
    }
    if reports.iter().any(|(_, r)| r.outcome == Outcome::Violation) {
        Err(())
    } else {
        Ok(())
    }
}

fn lock_command(root: &Path) -> Result<(), ()> {
    let project = load(root)?;
    write_lock(root, &project)
}

/// The ID a command-line reference names: `42`, or `<source>:42` with the
/// rule set's source.
fn parse_ref(text: &str, source: &str) -> Result<Id, ()> {
    let (name, id) = text.split_once(':').unwrap_or((source, text));
    match id_from_file_name(&format!("{id}.toml")) {
        Some(id) if name == source => Ok(id),
        Some(_) => {
            eprintln!(
                "statute: error: `{text}` names source `{name}`; the rule set's source is `{source}`"
            );
            Err(())
        }
        None => {
            eprintln!("statute: error: `{text}` is not a rule ID; write `42` or `{source}:42`");
            Err(())
        }
    }
}

fn explain(root: &Path, text: &str) -> Result<(), ()> {
    let project = load(root)?;
    let id = parse_ref(text, &project.manifest.source.name)?;
    let source = &project.source;
    let Some(node) = source.get(id) else {
        eprintln!(
            "statute: error: {id} does not exist in source `{}`",
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
    let name = &manifest.source.name;
    let id = parse_ref(text, name)?;
    let entry = format!("{name}:{id}");
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
    // checked that each is `<source>:<id>` with the one source.
    let position = list.iter().position(|v| {
        v.as_str()
            .and_then(|s| s.split_once(':'))
            .and_then(|(_, n)| id_from_file_name(&format!("{n}.toml")))
            == Some(id)
    });
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
