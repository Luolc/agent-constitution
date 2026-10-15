use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use toml_edit::{Array, DocumentMut, Item, Value};

use statute::diag::Diagnostics;
use statute::lock;
use statute::manifest::{self, Follow, Key, Manifest};
use statute::node::Kind;
use statute::private;
use statute::project::{Env, Project};
use statute::resolve::{Reason, Switches, current_successors, label, refs};
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
    /// Say why a rule is enabled, along the chain that brings it in, and
    /// whether it is on by default.
    Explain {
        /// The rule, as `R42`, `team:42`, or `42` for one of this repository's own.
        #[arg(required_unless_present = "defaults", conflicts_with = "defaults")]
        rule: Option<String>,
        /// List the rules on by default that the rule set keeps, those it
        /// turns off, and those off by default that it turns on.
        #[arg(long)]
        defaults: bool,
    },
    /// Enable a rule (or a pack) in statute.toml and update statute.lock.
    Add {
        /// The rule, as `R42`, `team:42`, or `42` for one of this repository's own.
        rule: String,
        /// Enable the pack rooted at this node, in `packs`.
        #[arg(long)]
        pack: bool,
        /// Enable it in the private rule set, private.toml in git's directory, and update private.lock.
        #[arg(long)]
        private: bool,
    },
    /// Take a rule (or a pack) out of statute.toml and update statute.lock.
    Remove {
        /// The rule, as `R42`, `team:42`, or `42` for one of this repository's own.
        rule: String,
        /// Take the pack out of `packs`.
        #[arg(long)]
        pack: bool,
        /// Take it out of the private rule set, and update private.lock.
        #[arg(long)]
        private: bool,
    },
    /// Write a new statute.toml in this directory, with no rules enabled.
    Init {
        /// Write the global configuration, ~/.config/statute/config.toml, and
        /// clone the sources it names, in place of a rule set.
        #[arg(long, conflicts_with_all = ["edition", "public", "private"])]
        global: bool,
        /// The source of the rules and the constitution, a directory
        /// relative to this one; by default the first-party source of the
        /// global configuration, or statute's own default. With
        /// `--global`, the git address of the first-party source.
        #[arg(long)]
        source: Option<String>,
        /// The constitution's edition; by default the only one the source
        /// holds.
        #[arg(long)]
        edition: Option<String>,
        /// The repository is public: anyone can read it. Without
        /// `--public` or `--private`, GitHub is asked through gh.
        #[arg(long, conflicts_with = "private")]
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
        /// Write only the private output, for a post-checkout hook: no
        /// network, and warnings in place of errors, exiting 0.
        #[arg(long, conflicts_with = "output")]
        private_only: bool,
    },
    /// List the enabled rules for review by domain, with the outcome of
    /// those a script checks; exits 1 on a violation.
    Check {
        /// Check the change since this git ref (`git diff <base>...HEAD`).
        #[arg(long)]
        base: Option<String>,
    },
    /// Write a local site of every rule of the sources, the private ones
    /// included, to .statute/site, which git ignores; open its
    /// index.html in a browser.
    Site,
    /// Build or check the constitution of this repository from its
    /// sources.
    Constitution {
        #[command(subcommand)]
        action: statute::constitution::cli::Action,
    },
    /// Name a source of rules.
    Source {
        #[command(subcommand)]
        action: SourceAction,
    },
    /// Check a directory of rule files as the first-party source's
    /// `rules/`, with every check that needs no rule set, and against the
    /// constitution beside it when there is one; exits 1 on an error.
    Validate {
        /// The directory of the rule files, as `rules`.
        dir: std::path::PathBuf,
        /// Also check that every node of this commit's rule files still
        /// exists and stays repealed or superseded.
        #[arg(long, value_name = "GIT_REF")]
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

#[derive(Subcommand)]
enum SourceAction {
    /// Name a source in statute.toml, in the private rule set when it is
    /// private and the repository public, or with --global in the global
    /// configuration; and update the lock file when there is one.
    Add {
        /// The source's short name, as in `team:7`.
        name: String,
        /// A git address, or a directory on disk (with --global, a clone).
        address: String,
        /// The branch to follow; by default the one the remote's HEAD names.
        #[arg(long, conflicts_with = "tag")]
        branch: Option<String>,
        /// The tag to follow.
        #[arg(long)]
        tag: Option<String>,
        /// Anyone can read the source. Without `--public` or `--private`,
        /// GitHub is asked through gh, and anything but PUBLIC is private.
        #[arg(long, conflicts_with = "private")]
        public: bool,
        /// The source is private.
        #[arg(long)]
        private: bool,
        /// Name it in the global configuration, for every rule set.
        #[arg(long)]
        global: bool,
        /// The directory of the source's rules, below its top; `rules` by
        /// default.
        #[arg(long)]
        rules_dir: Option<String>,
    },
}

/// Exit status when the rule set, the source or the arguments are wrong.
const FAILED: u8 = 1;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = Path::new(".");
    let result = match cli.command {
        Command::Lock => lock_command(root),
        Command::Explain { rule, defaults } => match rule {
            Some(rule) if !defaults => explain(root, &rule),
            _ => explain_defaults(root),
        },
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
        Command::Compile {
            private_only: true, ..
        } => {
            compile_private_only(root);
            Ok(())
        }
        Command::Compile { output, .. } => compile(root, output.as_deref()),
        Command::Check { base } => check(root, base.as_deref()),
        Command::Init {
            global: true,
            source,
            ..
        } => init_global(source.as_deref()),
        Command::Init {
            source,
            edition,
            public,
            private,
            ..
        } => init(
            root,
            source.as_deref(),
            edition,
            (public || private).then_some(public),
        ),
        Command::Upgrade { replace_superseded } => upgrade(root, replace_superseded),
        Command::Site => site(root),
        Command::Validate { dir, base } => validate(&dir, base.as_deref()),
        Command::Constitution { action } => statute::constitution::cli::run(action).map_err(|_| ()),
        Command::Source {
            action:
                SourceAction::Add {
                    name,
                    address,
                    branch,
                    tag,
                    public,
                    private,
                    global,
                    rules_dir,
                },
        } => source_add(
            root,
            &name,
            &address,
            branch.map(Follow::Branch).or(tag.map(Follow::Tag)),
            (public || private).then_some(public),
            global,
            rules_dir.as_deref(),
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::from(FAILED),
    }
}

/// What a command needs of the sources (`docs/design/global-configuration.md`).
#[derive(Clone, Copy)]
enum Mode {
    /// Reads at the commits the lock file records, its sources counting
    /// where nothing else names them.
    Read,
    /// Writes the lock file, keeping the commits it records.
    Lock,
    /// Writes the lock file at the newest commits.
    Newest,
}

/// The environment of a command in `mode`, with the global
/// configuration's sources; an error when the configuration cannot be
/// read.
fn env(mode: Mode) -> Result<Env, ()> {
    read_env(mode).map_err(|errors| {
        for e in errors {
            eprintln!("{e}");
        }
    })
}

/// As `env`, with what is wrong as lines rather than printed.
fn read_env(mode: Mode) -> Result<Env, Vec<String>> {
    let mut env = Env::default();
    env.layers.built_in = true;
    env.from_lock = matches!(mode, Mode::Read);
    env.newest = matches!(mode, Mode::Newest);
    let Some(path) = statute::global::config_dir().map(|d| d.join(statute::global::CONFIG)) else {
        return Ok(env);
    };
    let shown = statute::global::shown_config();
    match std::fs::read_to_string(&path) {
        Ok(text) => match manifest::read_config(&text) {
            Ok(sources) => env.layers.global = sources,
            Err(errors) => {
                return Err(errors
                    .into_iter()
                    .map(|e| format!("{shown}: error: {e}"))
                    .collect());
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(vec![format!("{shown}: error: cannot read: {e}")]),
    }
    Ok(env)
}

fn print(diags: &Diagnostics) {
    for d in &diags.0 {
        eprintln!("{d}");
    }
}

/// `validate`: reads the rule files of `dir` as the first-party source,
/// `R`, checks them, and against the constitution beside `dir` when there
/// is one, and prints every finding; fails when one is an error.
fn validate(dir: &Path, base: Option<&str>) -> Result<(), ()> {
    let label = dir.to_string_lossy();
    let label = label.trim_end_matches('/');
    let (mut source, mut diags) = statute::source::Source::load(dir, label);
    source.name = manifest::FIRST_PARTY.to_owned();
    source.public = true;
    let sources = statute::source::Sources::new(vec![source]);
    sources.check(&mut diags);
    let first = sources.iter().next().expect("one source");
    let notes = statute::constitution::rules::validate(dir, first, base, &mut diags);
    print(&diags);
    for note in notes {
        println!("{note}");
    }
    if diags.has_errors() {
        return Err(());
    }
    let count = sources.iter().map(|s| s.nodes.len()).sum::<usize>();
    println!("{label}: {count} nodes, no errors");
    Ok(())
}

/// The private layer of a public repository over the public project:
/// the directory of its rule set and the project with both layers.
struct Private {
    dir: PathBuf,
    both: Project,
}

/// The directory of the private rule set, [`private::dir`]: `None`
/// outside a git repository.
fn private_dir(root: &Path) -> Result<Option<PathBuf>, ()> {
    private::dir(root).map_err(|e| eprintln!("statute: error: {e}"))
}

/// Loads the project, and its private layer when there is a private rule
/// set: the public project and the private layer over it.
fn load_layers(root: &Path, env: &Env) -> Result<(Project, Option<Private>), ()> {
    let (project, diags) = Project::load(root, env);
    print(&diags);
    let project = project.ok_or(())?;
    let Some(dir) = private_dir(root)? else {
        return Ok((project, None));
    };
    let (both, more) = Project::load_private(&project, &diags, env, &dir);
    print(&more);
    if more.has_errors() {
        return Err(());
    }
    Ok((project, both.map(|both| Private { dir, both })))
}

/// Writes the private lock file of the private layer over `public`.
fn write_private_lock(public: &Project, private: &Private) -> Result<(), ()> {
    let path = private.dir.join(private::LOCK);
    private::replace(&path, &lock::render_private(public, &private.both)).map_err(|e| {
        eprintln!("{}: error: cannot write: {e}", path.display());
    })?;
    print_private_locked(&path, public, &private.both);
    Ok(())
}

fn print_private_locked(path: &Path, public: &Project, both: &Project) {
    let all = both.resolution.enabled.len();
    let added = all - public.resolution.enabled.len();
    println!(
        "wrote {}: {all} rules enabled, {added} of them by the private layer",
        path.display()
    );
}

/// The private layer over `public`, the public project as it is to be
/// written, resolved before anything is written: `None` when there is
/// none. A command that changes the public layer calls it first and
/// writes nothing when it fails, then writes the private lock file with
/// `relock_private`.
fn private_ready(root: &Path, public: &Project, env: &Env) -> Result<Option<Private>, ()> {
    let Some(dir) = private_dir(root)? else {
        return Ok(None);
    };
    let (both, diags) = Project::load_private(public, &Diagnostics::default(), env, &dir);
    print(&diags);
    if diags.has_errors() {
        return Err(());
    }
    Ok(both.map(|both| Private { dir, both }))
}

/// Writes the private lock file of `private`, from `private_ready`, when
/// there is a private layer.
fn relock_private(public: &Project, private: Option<&Private>) -> Result<(), ()> {
    private.map_or(Ok(()), |private| write_private_lock(public, private))
}

fn write_lock(root: &Path, project: &Project) -> Result<(), ()> {
    std::fs::write(root.join(lock::FILE), lock::render(project)).map_err(|e| {
        eprintln!("{}: error: cannot write: {e}", lock::FILE);
    })?;
    print_locked(project);
    Ok(())
}

fn compile(root: &Path, output: Option<&Path>) -> Result<(), ()> {
    let (project, private) = load_layers(root, &env(Mode::Read)?)?;
    let path = output.map_or_else(|| root.join(statute::compile::FILE), Path::to_path_buf);
    // The private output is made in full before anything is written.
    let full = private
        .as_ref()
        .map(|p| statute::compile::full_text(&project, &p.both, &p.dir))
        .transpose()
        .map_err(|e| eprintln!("statute: error: {e}"))?;
    let out = statute::compile::compile(&project).map_err(|e| eprintln!("statute: error: {e}"))?;
    // `.statute/.gitignore` is the first file written, once nothing is
    // left that can fail before the public files.
    if full.is_some() {
        private::ready(root).map_err(|e| eprintln!("statute: error: {e}"))?;
    }
    let mut written = statute::compile::write(&out, &path).map_err(|e| {
        eprintln!("statute: error: {e}");
    })?;
    if let Some(full) = full {
        written.extend(private::write(root, &full).map_err(|e| eprintln!("statute: error: {e}"))?);
    }
    for path in written {
        println!("wrote {}", path.display());
    }
    let dir = path.parent().unwrap_or(Path::new(""));
    if let Some(why) = private::claude_md(dir, private.is_some()) {
        eprintln!("{}: warning: {why}", dir.join("CLAUDE.md").display());
    }
    if private.is_some() {
        for nested in private::nested_agents_md(root) {
            eprintln!("{}: warning: {}", nested.display(), private::NESTED);
        }
    }
    Ok(())
}

/// `compile --private-only`, which a git `post-checkout` hook runs in a
/// new worktree (`docs/design/rule-sources.md`, A new worktree): writes
/// the private output alone, reading git sources from the cache only.
/// Without a public rule set or a private rule set it does nothing; any
/// other problem is a warning, and nothing is written. It never fails, so
/// that the hook never fails the checkout.
fn compile_private_only(root: &Path) {
    match private_only(root) {
        Ok(written) => {
            for path in written {
                println!("wrote {}", path.display());
            }
        }
        Err(causes) => {
            for cause in causes {
                eprintln!("statute: warning: {cause}");
            }
            eprintln!(
                "statute: warning: the private output is not written; `statute compile` writes it, or says what to fix"
            );
        }
    }
}

/// The work of `compile_private_only`: what it wrote, or each cause.
fn private_only(root: &Path) -> Result<Vec<PathBuf>, Vec<String>> {
    let one = |e: String| vec![e];
    let text = match std::fs::read_to_string(root.join(manifest::FILE)) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(one(format!("{}: cannot read: {e}", manifest::FILE))),
    };
    let Some(dir) = private::dir(root).map_err(one)? else {
        return Ok(Vec::new());
    };
    if !dir.join(private::MANIFEST).exists() {
        return Ok(Vec::new());
    }
    let public = toml::from_str::<toml::Table>(&text)
        .ok()
        .and_then(|t| t.get("public").and_then(toml::Value::as_bool));
    if public == Some(false) {
        return Ok(Vec::new());
    }
    let mut env = read_env(Mode::Read)?;
    env.offline = true;
    let errors =
        |diags: &Diagnostics| -> Vec<String> { diags.errors().map(ToString::to_string).collect() };
    let (project, diags) = Project::load(root, &env);
    let project = project.ok_or_else(|| errors(&diags))?;
    let (both, more) = Project::load_private(&project, &diags, &env, &dir);
    let both = match both {
        Some(both) => both,
        None if more.has_errors() => return Err(errors(&more)),
        None => return Ok(Vec::new()),
    };
    let full = statute::compile::full_text(&project, &both, &dir).map_err(one)?;
    private::write(root, &full).map_err(one)
}

fn site(root: &Path) -> Result<(), ()> {
    let (public, private) = load_layers(root, &env(Mode::Read)?)?;
    let project = private.map_or(public, |p| p.both);
    let fail = |e: String| eprintln!("statute: error: {e}");
    let pages = statute::site::pages(&project).map_err(fail)?;
    let index = statute::site::write(root, &pages).map_err(fail)?;
    let index = index.canonicalize().unwrap_or(index);
    println!(
        "wrote {} pages to {}; open file://{}",
        pages.len(),
        statute::site::dir().display(),
        index.display()
    );
    Ok(())
}

fn check(root: &Path, base: Option<&str>) -> Result<(), ()> {
    use statute::check::{Context, Outcome};
    let (public, private) = load_layers(root, &env(Mode::Read)?)?;
    let project = private.map_or(public, |p| p.both);
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

/// Writes the global configuration with the built-in default, or the
/// first-party source at `address`, then
/// clones each git source it names, so that a missing network or account
/// shows at once. An existing configuration is left as it is.
fn init_global(address: Option<&str>) -> Result<(), ()> {
    let shown = statute::global::shown_config();
    let Some(dir) = statute::global::config_dir() else {
        eprintln!("{shown}: error: the system reports no home directory");
        return Err(());
    };
    let path = dir.join(statute::global::CONFIG);
    let text =
        statute::global::initial_config(address.unwrap_or(statute::global::FIRST_PARTY_ADDRESS));
    let written = std::fs::create_dir_all(&dir).and_then(|()| {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, text.as_bytes()))
    });
    match written {
        Ok(()) => println!("wrote {shown}"),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            eprintln!("{shown}: error: already exists, so it is left as it is");
            return Err(());
        }
        Err(e) => {
            eprintln!("{shown}: error: cannot write: {e}");
            return Err(());
        }
    }
    let mut failed = false;
    for mut spec in env(Mode::Lock)?.layers.global {
        match statute::global::materialize(&mut spec, None, false) {
            Ok(()) => println!("source `{}`: {}", spec.name, spec.place()),
            Err(e) => {
                eprintln!("{shown}: error: {e}");
                failed = true;
            }
        }
    }
    if failed { Err(()) } else { Ok(()) }
}

/// The source's name in the rule set `init` writes.
const INIT_SOURCE: &str = manifest::FIRST_PARTY;

/// Writes a new `statute.toml` that rests on the constitution of `source`,
/// for a repository that is `public` or not (by default, as GitHub shows
/// it), and enables no rule, after trying that it resolves. An existing
/// file is left as it is.
fn init(
    root: &Path,
    source: Option<&str>,
    edition: Option<String>,
    public: Option<bool>,
) -> Result<(), ()> {
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
    let env = env(Mode::Lock)?;
    let edition = match (edition, source) {
        (Some(edition), _) => edition,
        (None, Some(source)) => only_edition(&root.join(source), source)?,
        (None, None) => {
            // The first-party source as a rule set without [sources] has it.
            let mut spec = env
                .layers
                .global
                .iter()
                .find(|s| s.name == INIT_SOURCE)
                .cloned()
                .unwrap_or_else(manifest::built_in_first_party);
            statute::global::materialize(&mut spec, None, false)
                .map_err(|e| eprintln!("statute: error: {e}"))?;
            only_edition(Path::new(&spec.path), &spec.shown(""))?
        }
    };
    let (public, said) = match public {
        Some(public) => (public, None),
        None => asked_public(root),
    };
    let quote = |s: &str| toml_edit::Value::from(s).to_string();
    let mut text = format!(
        "# Written by `statute init`; `statute add` and `statute remove` change the rules.\n\
         constitution = {{ source = \"{INIT_SOURCE}\", edition = {} }}\n\
         public = {public}\n\
         rules = []\n",
        quote(&edition),
    );
    if let Some(source) = source {
        text.push_str(&format!(
            "\n[sources]\n{INIT_SOURCE} = {{ path = {} }}\n",
            quote(source)
        ));
    }
    let (project, diags) = Project::with_manifest(root, &text, &env);
    print(&diags);
    let Some(project) = project else {
        eprintln!("statute: {} is not written", manifest::FILE);
        return Err(());
    };
    let at = project.manifest.constitution_spec().place();
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
        "wrote {}: edition {edition} of the constitution, source `{INIT_SOURCE}` at {at}",
        manifest::FILE
    );
    match said {
        Some(Ok(line)) => println!("{line}"),
        Some(Err(warning)) => eprintln!("{}: warning: {warning}", manifest::FILE),
        None => {}
    }
    println!("`statute add {INIT_SOURCE}<id>` enables a rule");
    Ok(())
}

/// Whether the repository at `root` is public, as GitHub shows it, and
/// what to say about it: a line, or a warning when the answer is taken
/// to the safe side, public, since a public rule set compiles nothing
/// private into the committed files.
fn asked_public(root: &Path) -> (bool, Option<Result<String, String>>) {
    let private =
        "if it is to be private, write `public = false`, as `statute init --private` does";
    let said = match statute::check::github_visibility(root).as_deref() {
        Ok("PUBLIC") => {
            return (
                true,
                Some(Ok(
                    "`public = true`: GitHub shows the repository as PUBLIC".to_owned()
                )),
            );
        }
        Ok("PRIVATE") => {
            return (
                false,
                Some(Ok(
                    "`public = false`: GitHub shows the repository as PRIVATE".to_owned(),
                )),
            );
        }
        Ok("INTERNAL") => format!(
            "`public = true`: GitHub shows the repository as INTERNAL, which everyone in its enterprise can read, so nothing private is compiled into the committed AGENTS.md; {private}"
        ),
        Ok(other) => format!(
            "`public = true`: GitHub shows the repository as {other}, which statute does not know, so it is taken as public, the safe side; {private}"
        ),
        Err(why) => format!(
            "`public = true`: GitHub could not tell whether the repository is public ({why}), so it is taken as public, the safe side, where nothing private is compiled into the committed AGENTS.md; {private}"
        ),
    };
    (true, Some(Err(said)))
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
    let (project, private) = load_layers(root, &env(Mode::Lock)?)?;
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
    relock_private(&project, private.as_ref())
}

/// What to do about a rule edited without a new revision
/// (`docs/design/rule-format.md`, IDs and revisions).
const RAISE_REVISION: &str = "an edited rule's revision goes up by one";

/// The enabled rules that changed beyond their translations since `old`
/// locked them while the revision is the same, each with that revision.
fn same_revision_new_content(old: &lock::Locked, project: &Project) -> Vec<(Key, u64)> {
    lock::current(project)
        .rules
        .into_iter()
        .filter_map(|(key, now)| match old.rules.get(&key) {
            Some(was) if was.revision == now.revision && was.original_changed(&now) => {
                Some((key, now.revision))
            }
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
    let env = env(Mode::Newest)?;
    let (project, diags) = Project::load(root, &env);
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
            let (next, diags) = Project::with_manifest(root, &changed, &env);
            print(&diags);
            let Some(next) = next else {
                eprintln!(
                    "statute: {} and {} are left unchanged",
                    manifest::FILE,
                    lock::FILE
                );
                return Err(());
            };
            let private = private_ready(root, &next, &env)?;
            for line in lines {
                println!("{line}");
            }
            report_changes(&old, &next);
            write_both(
                (&root.join(manifest::FILE), manifest::FILE),
                (&root.join(lock::FILE), lock::FILE),
                Some(&original),
                &changed,
                &lock::render(&next),
            )?;
            print_locked(&next);
            return relock_private(&next, private.as_ref());
        }
        None => {
            print(&diags);
            project
        }
    };
    let private = private_ready(root, &project, &env)?;
    report_changes(&old, &project);
    write_lock(root, &project)?;
    relock_private(&project, private.as_ref())
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
    for (key, now) in &new.rules {
        let revision = now.revision;
        // An overridden rule that changed asks whether the override still
        // holds.
        let still = match project.resolution.overridden.get(key) {
            Some(by) => format!("; {by} overrides it, so check that the override still holds"),
            None => String::new(),
        };
        match old.rules.get(key) {
            Some(was) if was.revision != revision => line(format!(
                "changed: rule {}: revision {} -> {revision}{still}",
                label(sources, key),
                was.revision
            )),
            Some(was) if was.original_changed(now) => line(format!(
                "changed: rule {}: content changed, still revision {revision}; {RAISE_REVISION}{still}",
                label(sources, key)
            )),
            Some(was) if was.sha256 != now.sha256 => line(format!(
                "changed: rule {}: only the translations or the layout changed, still revision {revision}{still}",
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
            Reason::Default => "on by default".to_owned(),
            Reason::DefaultOf(o) => format!("the default of one-of {}", label(sources, o)),
            Reason::RequiredBy(b) => format!("required by {}", label(sources, b)),
            Reason::OverriddenBy(b) => format!("overridden by {}", label(sources, b)),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The rule set with each superseded entry of `rules` and `packs` replaced
/// by its successor, or taken out when the successor is listed already; an
/// entry that several rules together take the place of is left for the
/// user to replace with them:
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
            let new = match current_successors(&project.sources, old).as_slice() {
                [] => continue,
                [new] => new.clone(),
                many => {
                    lines.push(format!(
                        "left {old} in `{field}`: superseded by {}, which together take its place; add them with `statute add`, leaving out any that does not apply, and take {old} out with `statute remove`",
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
    let (public, private) = load_layers(root, &env(Mode::Read)?)?;
    let project = private.map_or(public, |p| p.both);
    let key = parse_ref(text, &project.manifest)?;
    let sources = &project.sources;
    let Some(node) = sources.get(&key) else {
        eprintln!(
            "statute: error: {key} does not exist in {}",
            describe(&key.source)
        );
        return Err(());
    };
    let sw = Switches::new(sources, &project.manifest);
    match node.kind {
        Kind::Rule => {}
        Kind::Domain => {
            println!(
                "{} is a domain: it is never enabled itself, and gives a heading to the enabled rules under it",
                label(sources, &key)
            );
            println!("- {}", switch_state(&project, &sw, &key));
            return Ok(());
        }
        Kind::OneOf => {
            match (sw.default_child(&key), sw.chosen(&key)) {
                (Some(d), Some(c)) if c != d => println!(
                    "one-of {}: its default is {}, and the rule set picks {}",
                    label(sources, &key),
                    label(sources, &d),
                    label(sources, &c)
                ),
                (Some(d), _) => println!(
                    "one-of {}: its default is {}",
                    label(sources, &key),
                    label(sources, &d)
                ),
                (None, _) => {}
            }
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
        println!("- {}", switch_state(&project, &sw, &key));
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
    if let Some(uses) = sources
        .get(&res.standing(&key))
        .and_then(|n| n.rule.as_ref())
        .map(|r| r.uses)
    {
        let goes = match (uses.instructions, uses.review) {
            (true, true) => "it is compiled into AGENTS.md and listed for review",
            (true, false) => "it is compiled into AGENTS.md",
            (false, _) => "it is listed for review, and not compiled into AGENTS.md",
        };
        println!("- {goes}");
    }
    Ok(())
}

/// Whether a domain or a rule is on by default, and what the rule set
/// does with that: "R44 is on by default, and its domain R13 is on".
fn switch_state(project: &Project, sw: &Switches, key: &Key) -> String {
    let sources = &project.sources;
    let m = &project.manifest;
    let entry = Manifest::entry(key);
    let Some(node) = sources.get(key) else {
        return String::new();
    };
    let shown = key.to_string();
    let switch = match node.default {
        Some(statute::node::Switch::On) => "on by default",
        Some(statute::node::Switch::Off) => "off by default",
        // An option of a one-of, or a rule that overrides another.
        None => "without a `default` of its own",
    };
    if let Some(off) = sw
        .excluded(key)
        .then(|| m.exclude.iter().find(|e| sw.above(key).contains(e)))
        .flatten()
    {
        return if off == key {
            format!("{shown} is {switch} and taken out by `exclude`")
        } else {
            format!("{shown} is {switch}, and `exclude` turns off {off}, above it")
        };
    }
    if !sw.takes_defaults(&key.source) {
        return format!(
            "{shown} is {switch}, and the rule set takes no defaults of {}",
            describe(&key.source)
        );
    }
    let parent = sources.parent(key);
    let parent_on = parent.as_ref().is_none_or(|p| sw.active.contains(p));
    match (node.kind, sw.switched_on(key), parent.as_ref(), parent_on) {
        (Kind::Domain, _, _, _) if m.domains.contains(key) => {
            format!("{shown} is {switch}, and `domains` turns it on")
        }
        (_, true, Some(p), true) => format!("{shown} is {switch}, and its domain {p} is on"),
        (_, true, None, _) => format!("{shown} is {switch}"),
        (_, true, Some(p), false) => format!(
            "{shown} is {switch}, and its domain {p} is off; `statute add {}` turns that on",
            Manifest::entry(p)
        ),
        (_, false, _, _) => format!("{shown} is {switch}; `statute add {entry}` turns it on"),
    }
}

/// `explain --defaults`: the three lists of moving a rule set over to the
/// defaults (`docs/design/rule-config.md`, Moving a repository over).
fn explain_defaults(root: &Path) -> Result<(), ()> {
    let (public, private) = load_layers(root, &env(Mode::Read)?)?;
    let project = private.map_or(public, |p| p.both);
    let sources = &project.sources;
    // What the rule set would hold with nothing listed.
    let bare = Manifest {
        rules: Vec::new(),
        packs: Vec::new(),
        domains: Vec::new(),
        exclude: Vec::new(),
        ..project.manifest.clone()
    };
    let number = project.revision.parse().unwrap_or(0);
    let mut ignored = Diagnostics::default();
    let defaults = statute::resolve::resolve(sources, &bare, number, &mut ignored).enabled;
    let enabled = &project.resolution.enabled;
    let list = |title: &str, keys: Vec<&Key>| {
        println!("{title}:");
        if keys.is_empty() {
            println!("- none");
        }
        for key in keys {
            println!("- {}", label(sources, key));
        }
    };
    list(
        "On by default, and kept",
        defaults
            .keys()
            .filter(|k| enabled.contains_key(*k))
            .collect(),
    );
    println!();
    list(
        "On by default, and turned off",
        defaults
            .keys()
            .filter(|k| !enabled.contains_key(*k))
            .collect(),
    );
    println!();
    list(
        "Off by default, and turned on",
        enabled
            .keys()
            .filter(|k| !defaults.contains_key(*k))
            .collect(),
    );
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
            Reason::Default => {
                let domain = sources.parent(key).map(|d| label(sources, &d));
                out.push_str(&format!(
                    "{indent}- on by default, and its domain {} is on\n",
                    domain.unwrap_or_default()
                ));
                continue;
            }
            Reason::DefaultOf(one_of) => {
                out.push_str(&format!(
                    "{indent}- the default of one-of {}, which needs a child and has no other\n",
                    label(sources, one_of)
                ));
                continue;
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

/// Adds `key` to the list `field` of `doc`, or takes it out; `false` when
/// it is there already, to add. Fails when it is not there, to take out.
fn edit_list(
    doc: &mut DocumentMut,
    field: &str,
    key: &Key,
    manifest: &Manifest,
    adding: bool,
    file: &str,
) -> Result<bool, ()> {
    let list = doc
        .entry(field)
        .or_insert_with(|| Item::Value(Value::Array(Array::new())))
        .as_array_mut()
        .ok_or_else(|| eprintln!("{file}: error: `{field}` is not a list"))?;
    // Entries of the list that name this key; the rule set has been read,
    // so each names a node of one of its sources.
    let position = list
        .iter()
        .position(|v| listed_key(v, manifest).as_ref() == Some(key));
    match (adding, position) {
        (true, Some(_)) => Ok(false),
        (true, None) => {
            list.push(Manifest::entry(key).as_str());
            Ok(true)
        }
        (false, Some(i)) => {
            list.remove(i);
            Ok(true)
        }
        (false, None) => {
            eprintln!("{file}: error: `{field}` does not list {key}");
            Err(())
        }
    }
}

/// `add` (`adding`) or `remove`: changes the list in statute.toml, or
/// with `private` in the private rule set, keeping the rest of the file as
/// written, resolves the result, and writes the file and its lock file
/// only when it resolves. A change to statute.toml locks the private
/// layer again, when there is one.
fn edit(root: &Path, text: &str, pack: bool, adding: bool, private: bool) -> Result<(), ()> {
    let env = env(Mode::Lock)?;
    let dir = if private {
        let Some(dir) = private_dir(root)? else {
            eprintln!("statute: error: {}", private::NO_REPOSITORY);
            return Err(());
        };
        Some(dir)
    } else {
        None
    };
    let (path, lock_path) = match &dir {
        Some(dir) => (dir.join(private::MANIFEST), dir.join(private::LOCK)),
        None => (root.join(manifest::FILE), root.join(lock::FILE)),
    };
    let (file, lock_file) = match &dir {
        Some(_) => (path.display().to_string(), lock_path.display().to_string()),
        None => (manifest::FILE.to_owned(), lock::FILE.to_owned()),
    };
    let (public, given) = if private {
        let (public, given) = Project::load(root, &env);
        print(&given);
        (Some(public.ok_or(())?), given)
    } else {
        (None, Diagnostics::default())
    };
    // A private rule set not made yet is an empty one.
    let original = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(e) if private && e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            eprintln!("{file}: error: cannot read: {e}");
            return Err(());
        }
    };
    let start = original.as_deref().unwrap_or_default();
    let parsed = match &public {
        Some(public) => public.manifest.with_private(start),
        None => Manifest::parse_with(start, &env.layers),
    };
    let manifest = parsed.map_err(|errors| {
        for e in errors {
            eprintln!("{file}: error: {e}");
        }
    })?;
    let key = parse_ref(text, &manifest)?;
    let entry = Manifest::entry(&key);
    // The rule set as it is, to tell a domain from a rule; when it does
    // not resolve, the key is taken for a rule.
    let try_load = |text: &str| match (&public, &dir) {
        (Some(public), Some(dir)) => Project::with_private(public, &given, text, &env, dir),
        _ => Project::with_manifest(root, text, &env),
    };
    let current = try_load(start).0;
    let is_domain = current
        .as_ref()
        .and_then(|p| p.sources.get(&key))
        .is_some_and(|n| n.kind == Kind::Domain);
    let field = match (pack, is_domain) {
        (true, _) => "packs",
        (false, true) => "domains",
        (false, false) => "rules",
    };

    let mut doc: DocumentMut = start.parse().map_err(|e| eprintln!("{file}: error: {e}"))?;
    let mut done = Vec::new();
    // `add` of an entry of `exclude` takes it out of there first.
    if adding && !pack && manifest.exclude.contains(&key) {
        edit_list(&mut doc, "exclude", &key, &manifest, false, &file)?;
        done.push(format!("removed {key} from `exclude`"));
    }
    let now_on = |doc: &DocumentMut| {
        try_load(&doc.to_string()).0.is_some_and(|p| {
            p.resolution.enabled.contains_key(&key) || p.resolution.on.contains(&key)
        })
    };
    // `remove` of a rule or domain the defaults enable and the rule set
    // does not list only writes it to `exclude`, below.
    let listed = match field {
        "packs" => &manifest.packs,
        "domains" => &manifest.domains,
        _ => &manifest.rules,
    }
    .contains(&key);
    let only_exclude = !adding && !pack && !private && !listed;
    if !(adding && !done.is_empty() && now_on(&doc)) && !only_exclude {
        match edit_list(&mut doc, field, &key, &manifest, adding, &file)? {
            true => done.push(format!(
                "{} {key} {} `{field}`",
                if adding { "added" } else { "removed" },
                if adding { "to" } else { "from" }
            )),
            false => {
                println!("{key} is already in `{field}`");
                return Ok(());
            }
        }
    }
    // What the defaults still enable after `remove` goes to `exclude`.
    if !adding && !pack && !private && now_on(&doc) {
        let defaults_only = try_load(&doc.to_string()).0.is_some_and(|p| {
            if is_domain {
                return Switches::new(&p.sources, &p.manifest).on_by_default(&key);
            }
            p.resolution.enabled.get(&key).is_some_and(|reasons| {
                reasons
                    .iter()
                    .all(|r| matches!(r, Reason::Default | Reason::DefaultOf(_)))
            })
        });
        if defaults_only {
            edit_list(&mut doc, "exclude", &key, &manifest, true, &file)?;
            done.push(format!("added {key} to `exclude`, as it is on by default"));
        }
    }
    if only_exclude && done.is_empty() {
        eprintln!("{file}: error: `{field}` does not list {key}");
        return Err(());
    }

    let changed = doc.to_string();
    let (project, diags) = try_load(&changed);
    print(&diags);
    let Some(project) = project else {
        eprintln!("statute: {file} and {lock_file} are left unchanged");
        return Err(());
    };
    let both = if private {
        None
    } else {
        private_ready(root, &project, &env)?
    };
    let lock_text = match &public {
        Some(public) => lock::render_private(public, &project),
        None => lock::render(&project),
    };
    write_both(
        (&path, &file),
        (&lock_path, &lock_file),
        original.as_deref(),
        &changed,
        &lock_text,
    )?;
    for line in &done {
        println!("{line}");
    }
    if !adding && !pack && project.resolution.enabled.contains_key(&key) {
        println!("{key} is still enabled; `statute explain {entry}` says why");
    }
    match &public {
        Some(public) => print_private_locked(&lock_path, public, &project),
        None => {
            print_locked(&project);
            relock_private(&project, both.as_ref())?;
        }
    }
    Ok(())
}

/// Writes a changed rule set, `files.0`, and its lock file, `files.1`,
/// each a path and how it is shown, so that the two stay in step: both
/// go to temporary files in their directories first, then each is
/// renamed into place, and if the lock file cannot be, the rule set is
/// put back as it was, `original` (`None` when there was none).
fn write_both(
    manifest_file: (&Path, &str),
    lock_file: (&Path, &str),
    original: Option<&str>,
    changed: &str,
    lock_text: &str,
) -> Result<(), ()> {
    let (manifest_path, manifest_file) = manifest_file;
    let (lock_path, lock_file) = lock_file;
    let manifest_tmp = private::staged(manifest_path);
    let lock_tmp = private::staged(lock_path);
    let unchanged = || {
        let _ = std::fs::remove_file(&manifest_tmp);
        let _ = std::fs::remove_file(&lock_tmp);
        eprintln!("statute: {manifest_file} and {lock_file} are left unchanged");
    };
    let staged = manifest_path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .map_err(|e| (manifest_file, e))
        .and_then(|()| std::fs::write(&lock_tmp, lock_text).map_err(|e| (lock_file, e)))
        .and_then(|()| std::fs::write(&manifest_tmp, changed).map_err(|e| (manifest_file, e)))
        .and_then(|()| {
            std::fs::rename(&manifest_tmp, manifest_path).map_err(|e| (manifest_file, e))
        });
    if let Err((file, e)) = staged {
        eprintln!("{file}: error: cannot write: {e}");
        unchanged();
        return Err(());
    }
    if let Err(e) = std::fs::rename(&lock_tmp, lock_path) {
        eprintln!("{lock_file}: error: cannot write: {e}");
        let put_back = match original {
            Some(original) => std::fs::write(manifest_path, original),
            None => std::fs::remove_file(manifest_path),
        };
        if let Err(e) = put_back {
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

/// `statute source add`: names a source in the file the design says
/// (`docs/design/global-configuration.md`, Adding a source), after
/// reading it and resolving the rule set with it; nothing is written when
/// that fails.
fn source_add(
    root: &Path,
    name: &str,
    address: &str,
    follow: Option<Follow>,
    public: Option<bool>,
    global: bool,
    rules_dir: Option<&str>,
) -> Result<(), ()> {
    let env = env(Mode::Lock)?;
    let fail = |e: String| eprintln!("statute: error: {e}");
    let dir = root.join(address);
    // A bare repository's path is a git address: it has no files to read.
    let bare = dir.is_dir() && statute::global::is_bare(&dir);
    let absolute;
    let address = if bare {
        absolute = std::path::absolute(&dir).map_err(|e| fail(format!("{address}: {e}")))?;
        absolute.to_str().unwrap_or(address)
    } else {
        address
    };
    let mut entry = toml_edit::InlineTable::new();
    // Where GitHub is asked: a directory's repository, or `owner/name`.
    let (asked_at, repo): (PathBuf, Option<String>);
    let follow = if dir.is_dir() && !bare {
        asked_at = dir.clone();
        repo = None;
        if global {
            let clone = std::path::absolute(&dir).map_err(|e| fail(format!("{address}: {e}")))?;
            let (origin, head) = statute::global::clone_origin(&clone).map_err(fail)?;
            entry.insert("git", origin.as_str().into());
            entry.insert("clone", clone.to_string_lossy().as_ref().into());
            Some(follow.or(head.map(Follow::Branch)).ok_or_else(|| {
                fail(format!(
                    "{address}: its origin's HEAD names no branch; say which with --branch"
                ))
            })?)
        } else {
            entry.insert("path", address.into());
            if follow.is_some() {
                fail(format!(
                    "{address} is a directory, read as it is; --branch and --tag are for a git address"
                ));
                return Err(());
            }
            None
        }
    } else {
        asked_at = root.to_path_buf();
        repo = statute::global::github_repo(address);
        entry.insert("git", address.into());
        Some(match follow {
            Some(f) => f,
            None => Follow::Branch(statute::global::default_branch(address).map_err(fail)?),
        })
    };
    match &follow {
        Some(Follow::Branch(b)) => entry.insert("branch", b.as_str().into()),
        Some(Follow::Tag(t)) => entry.insert("tag", t.as_str().into()),
        None => None,
    };
    let (public, note) = match public {
        Some(p) => (p, None),
        None => {
            let answer = match (&repo, dir.is_dir() && !bare) {
                (None, false) => Err(format!("GitHub does not hold {address}")),
                _ => statute::check::visibility_of(&asked_at, repo.as_deref()),
            };
            match answer.as_deref() {
                Ok("PUBLIC") => (true, None),
                Ok(other) => (
                    false,
                    Some(format!(
                        "GitHub shows it as {other}, so it is private; --public makes it public"
                    )),
                ),
                Err(why) => (
                    false,
                    Some(format!(
                        "GitHub could not tell whether it is public ({why}), so it is private; --public makes it public"
                    )),
                ),
            }
        }
    };
    if name != manifest::FIRST_PARTY || !public {
        entry.insert("public", public.into());
    }
    if let Some(dir) = rules_dir {
        entry.insert("rules-dir", dir.into());
    }

    let global_path = statute::global::config_dir().map(|d| d.join(statute::global::CONFIG));
    // The private rule set's directory, when the source goes there.
    let mut private_at = None;
    let (file, path) = if global {
        let Some(path) = global_path else {
            fail("the system reports no home directory".to_owned());
            return Err(());
        };
        (statute::global::shown_config(), path)
    } else {
        let text = std::fs::read_to_string(root.join(manifest::FILE)).map_err(|e| {
            eprintln!("{}: error: cannot read: {e}", manifest::FILE);
        })?;
        let repo_public = Manifest::parse_with(&text, &env.layers)
            .map(|m| m.public)
            .map_err(|errors| {
                for e in errors {
                    eprintln!("{}: error: {e}", manifest::FILE);
                }
            })?;
        if repo_public && !public {
            let Some(at) = private_dir(root)? else {
                fail(private::NO_REPOSITORY.to_owned());
                return Err(());
            };
            // The private rule set serves every worktree of the clone.
            if entry.contains_key("path") {
                let absolute = dir
                    .canonicalize()
                    .map_err(|e| fail(format!("{address}: {e}")))?;
                entry.insert("path", absolute.to_string_lossy().as_ref().into());
            }
            let path = at.join(private::MANIFEST);
            private_at = Some(at);
            (path.display().to_string(), path)
        } else {
            (manifest::FILE.to_owned(), root.join(manifest::FILE))
        }
    };
    let original = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            eprintln!("{file}: error: cannot read: {e}");
            return Err(());
        }
    };
    let start = original.clone().unwrap_or_default();
    let mut doc: DocumentMut = start.parse().map_err(|e| eprintln!("{file}: error: {e}"))?;
    let sources = doc
        .entry("sources")
        .or_insert_with(|| Item::Table(toml_edit::Table::new()))
        .as_table_mut()
        .ok_or_else(|| eprintln!("{file}: error: `sources` is not a table"))?;
    if sources.contains_key(name) {
        eprintln!("{file}: error: [sources] names `{name}` already");
        return Err(());
    }
    sources.insert(name, Item::Value(Value::InlineTable(entry)));
    let changed = doc.to_string();
    let replaces = env.layers.global.iter().any(|s| s.name == name) && !global;

    if global {
        let mut spec = manifest::read_config(&changed)
            .map_err(|errors| {
                for e in errors {
                    eprintln!("{file}: error: {e}");
                }
            })?
            .into_iter()
            .find(|s| s.name == name)
            .expect("the source was just added");
        statute::global::materialize(&mut spec, None, false).map_err(fail)?;
        if !Path::new(&spec.path).join(&spec.rules_dir).is_dir() {
            fail(format!(
                "source `{name}` ({address}) has no {}/",
                spec.rules_dir
            ));
            return Err(());
        }
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, &changed));
        written.map_err(|e| eprintln!("{file}: error: cannot write: {e}"))?;
        println!("wrote {file}: source `{name}`, {}", spec.place());
    } else if let Some(at) = &private_at {
        let (public_project, given) = Project::load(root, &env);
        print(&given);
        let public_project = public_project.ok_or(())?;
        if public_project.manifest.spec(name).is_some() {
            eprintln!(
                "{file}: error: `{name}` is a source of the public layer already; the private layer only adds sources"
            );
            return Err(());
        }
        let (both, diags) = Project::with_private(&public_project, &given, &changed, &env, at);
        print(&diags);
        let both = both.ok_or_else(|| eprintln!("statute: {file} is left unchanged"))?;
        let lock_path = at.join(private::LOCK);
        write_both(
            (&path, &file),
            (&lock_path, &lock_path.display().to_string()),
            original.as_deref(),
            &changed,
            &lock::render_private(&public_project, &both),
        )?;
        println!(
            "wrote {file}: source `{name}`, {}",
            both.manifest
                .spec(name)
                .map(|s| s.place())
                .unwrap_or_default()
        );
        print_private_locked(&lock_path, &public_project, &both);
    } else {
        let (project, diags) = Project::with_manifest(root, &changed, &env);
        print(&diags);
        let project = project.ok_or_else(|| eprintln!("statute: {file} is left unchanged"))?;
        let private = private_ready(root, &project, &env)
            .map_err(|()| eprintln!("statute: {file} is left unchanged"))?;
        if root.join(lock::FILE).exists() {
            write_both(
                (&path, &file),
                (&root.join(lock::FILE), lock::FILE),
                original.as_deref(),
                &changed,
                &lock::render(&project),
            )?;
            print_locked(&project);
            relock_private(&project, private.as_ref())?;
        } else {
            std::fs::write(&path, &changed)
                .map_err(|e| eprintln!("{file}: error: cannot write: {e}"))?;
        }
        println!(
            "wrote {file}: source `{name}`, {}",
            project
                .manifest
                .spec(name)
                .map(|s| s.place())
                .unwrap_or_default()
        );
    }
    if let Some(note) = note {
        println!("`{name}`: {note}");
    }
    if private_at.is_some() {
        println!(
            "`{name}` is private, so it is named in the private rule set, {file}, in git's directory, which no commit holds; --public names it in {}",
            manifest::FILE
        );
    }
    if replaces {
        println!("`{name}` here replaces the source of that name in the global configuration");
    }
    Ok(())
}
