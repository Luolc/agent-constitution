//! A repository that uses statute: its rule set, the sources the rule set
//! names with its own rules, and the constitution it rests on, read from
//! disk together.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::diag::Diagnostics;
use crate::manifest::{self, LOCAL, Layers, Manifest, SourceSpec};
use crate::private;
use crate::resolve::{self, Resolution};
use crate::source::{Source, Sources};

#[derive(Debug)]
pub struct Project {
    /// The directory of `statute.toml`.
    pub root: PathBuf,
    pub manifest: Manifest,
    /// The sources of `[sources]`, and the repository's own rules, `L`,
    /// when it has a `rules/` and no source is at `.`.
    pub sources: Sources,
    /// The constitution's revision as `index.toml` writes it.
    pub revision: String,
    pub resolution: Resolution,
}

/// What a command reads besides the repository's own files
/// (`docs/design/global-configuration.md`).
#[derive(Debug, Clone, Default)]
pub struct Env {
    /// The global configuration's sources and the built-in `R`.
    pub layers: Layers,
    /// Whether a source the lock file records counts where nothing else
    /// names it, and a lock file whose git source has moved fails: so for
    /// `compile`, `check`, `explain` and `site`, never for the commands
    /// that write the lock file.
    pub from_lock: bool,
    /// Whether each git source is read at its newest commit, as `upgrade`
    /// does, rather than at the one the lock file records.
    pub newest: bool,
}

#[derive(Deserialize)]
struct Index {
    edition: String,
    revision: String,
}

/// `rules` for the source `.`, `../team/rules` for `../team`: how a path
/// below the source is named in messages.
pub fn shown(source_path: &str, below: &str) -> String {
    let base = source_path.trim_end_matches('/');
    if base == "." || base.is_empty() {
        below.to_owned()
    } else {
        format!("{base}/{below}")
    }
}

impl Project {
    /// Reads the rule set in `root`, its source and its constitution, and
    /// resolves the rule set. Returns the project only when nothing failed;
    /// the diagnostics hold the warnings either way.
    pub fn load(root: &Path, env: &Env) -> (Option<Project>, Diagnostics) {
        match std::fs::read_to_string(root.join(manifest::FILE)) {
            Ok(text) => Project::with_manifest(root, &text, env),
            Err(e) => {
                let mut diags = Diagnostics::default();
                diags.error(manifest::FILE, format!("cannot read: {e}"));
                (None, diags)
            }
        }
    }

    /// As `load`, with the text of `statute.toml` given, so a changed rule
    /// set can be tried before it is written.
    pub fn with_manifest(root: &Path, text: &str, env: &Env) -> (Option<Project>, Diagnostics) {
        let mut diags = Diagnostics::default();
        let locked = locked_sources(root, crate::lock::FILE);
        let mut layers = env.layers.clone();
        if env.from_lock {
            layers.locked = locked.clone();
        }
        let manifest = match Manifest::parse_with(text, &layers) {
            Ok(m) => m,
            Err(errors) => {
                for e in errors {
                    diags.error(manifest::FILE, e);
                }
                return (None, diags);
            }
        };
        Project::from_manifest(root, manifest, diags, env, &locked)
    }

    /// The private layer of the public `project`, read from
    /// `statute.private.toml` next to it, with the public rule set: `None`
    /// and no finding when there is no such file. It fails when a rule
    /// that only it enables overrides one that the public rule set
    /// enables (`docs/design/rule-sources.md`, Overrides). Its findings
    /// leave out those of `given`, the public rule set's.
    pub fn load_private(
        project: &Project,
        given: &Diagnostics,
        env: &Env,
    ) -> (Option<Project>, Diagnostics) {
        let root = &project.root;
        let mut diags = Diagnostics::default();
        match std::fs::read_to_string(root.join(private::MANIFEST)) {
            Ok(text) => Project::with_private(project, given, &text, env),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (None, diags),
            Err(e) => {
                diags.error(private::MANIFEST, format!("cannot read: {e}"));
                (None, diags)
            }
        }
    }

    /// As `load_private`, with the text of `statute.private.toml` given.
    pub fn with_private(
        project: &Project,
        given: &Diagnostics,
        text: &str,
        env: &Env,
    ) -> (Option<Project>, Diagnostics) {
        let root = &project.root;
        let mut diags = Diagnostics::default();
        let locked = locked_sources(root, private::LOCK);
        let fallback = if env.from_lock { &locked[..] } else { &[] };
        let manifest = match project.manifest.with_private_from(text, fallback) {
            Ok(m) => m,
            Err(errors) => {
                for e in errors {
                    diags.error(private::MANIFEST, e);
                }
                return (None, diags);
            }
        };
        let (combined, mut diags) = Project::from_manifest(root, manifest, diags, env, &locked);
        diags.0.retain(|d| !given.0.contains(d));
        let Some(combined) = combined else {
            return (None, diags);
        };
        let public = &project.resolution.enabled;
        for (target, by) in &combined.resolution.overridden {
            if public.contains_key(target) && !public.contains_key(by) {
                diags.error(
                    private::MANIFEST,
                    format!(
                        "`{by}` overrides \"{target}\", a rule of the public rule set `{}`; in a public repository (`public = true`), `{}` may only add rules",
                        manifest::FILE,
                        private::MANIFEST
                    ),
                );
            }
        }
        if diags.has_errors() {
            return (None, diags);
        }
        (Some(combined), diags)
    }

    /// The project of a rule set read already, its findings added to
    /// `diags`.
    fn from_manifest(
        root: &Path,
        mut manifest: Manifest,
        mut diags: Diagnostics,
        env: &Env,
        locked: &[SourceSpec],
    ) -> (Option<Project>, Diagnostics) {
        for spec in &mut manifest.sources {
            if let Err(e) = pick_commit(spec, locked, env) {
                diags.error(manifest::FILE, e);
            }
        }
        if diags.has_errors() {
            return (None, diags);
        }
        let mut list = Vec::new();
        for spec in &manifest.sources {
            let (mut source, more) = Source::load(
                &root.join(&spec.path).join(&spec.rules_dir),
                &spec.shown(&spec.rules_dir),
            );
            diags.0.extend(more.0);
            source.name = spec.name.clone();
            source.path = spec.path.clone();
            source.public = spec.public;
            source.local = spec.local;
            list.push(source);
        }
        let local_rules = &manifest.local_rules;
        if !manifest.sources.iter().any(|s| s.holds(local_rules)) && root.join(local_rules).is_dir()
        {
            let (mut source, more) = Source::load(&root.join(local_rules), local_rules);
            diags.0.extend(more.0);
            source.name = LOCAL.to_owned();
            source.path = ".".to_owned();
            source.public = manifest.public;
            source.local = true;
            list.push(source);
        }
        let sources = Sources::new(list);
        sources.check(&mut diags);

        let spec = manifest.constitution_spec();
        let source_root = root.join(&spec.path);
        let index_name = spec.shown(&format!("constitution/{}/index.toml", manifest.edition));
        let index = std::fs::read_to_string(
            source_root
                .join("constitution")
                .join(&manifest.edition)
                .join("index.toml"),
        )
        .map_err(|e| {
            format!(
                "cannot read the constitution of edition `{}`: {e}",
                manifest.edition
            )
        })
        .and_then(|text| {
            toml::from_str::<Index>(&text).map_err(|e| e.to_string().trim_end().to_owned())
        })
        .and_then(|index| {
            if index.edition != manifest.edition {
                return Err(format!(
                    "the index is of edition `{}`, and the rule set names `{}`",
                    index.edition, manifest.edition
                ));
            }
            let number = index
                .revision
                .parse::<u64>()
                .map_err(|_| format!("revision `{}` is not a whole number", index.revision))?;
            Ok((index.revision, number))
        });
        let (revision, number) = match index {
            Ok(r) => r,
            Err(e) => {
                diags.error(index_name, e);
                return (None, diags);
            }
        };

        if diags.has_errors() {
            return (None, diags);
        }
        let resolution = resolve::resolve(&sources, &manifest, number, &mut diags);
        if diags.has_errors() {
            return (None, diags);
        }
        let project = Project {
            root: root.to_owned(),
            manifest,
            sources,
            revision,
            resolution,
        };
        (Some(project), diags)
    }
}

/// The git sources the lock file `name` in `root` records; none when it
/// is missing or cannot be read.
fn locked_sources(root: &Path, name: &str) -> Vec<SourceSpec> {
    std::fs::read_to_string(root.join(name))
        .map(|text| crate::lock::sources(&text))
        .unwrap_or_default()
}

/// Reads the git source `spec` at its commit: the one `locked` records
/// for the same address and branch or tag, unless `env` asks for the
/// newest. With `env.from_lock`, a recorded source that has moved makes
/// the lock file stale, an error.
fn pick_commit(spec: &mut SourceSpec, locked: &[SourceSpec], env: &Env) -> Result<(), String> {
    let Some(git) = &spec.git else {
        return Ok(());
    };
    let recorded = locked
        .iter()
        .find(|l| l.name == spec.name)
        .and_then(|l| l.git.as_ref());
    let commit = match recorded {
        Some(r) if r.address == git.address && r.follow == git.follow => r.commit.clone(),
        Some(r) if env.from_lock => {
            return Err(format!(
                "source `{}` is {} at {}, and {} records {} at {}; run `statute lock`",
                spec.name,
                git.follow,
                git.address,
                crate::lock::FILE,
                r.follow,
                r.address
            ));
        }
        _ => None,
    };
    crate::global::materialize(spec, commit.as_deref(), env.newest)
}
