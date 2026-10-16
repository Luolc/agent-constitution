//! A repository that uses statute: its rule set, the sources the rule set
//! names with its own rules, and the constitution it rests on, read from
//! disk together.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::diag::Diagnostics;
use crate::manifest::{self, Declared, Key, LOCAL, Layers, Manifest, SourceSpec};
use crate::param::Setting;
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
    /// The value of each parameter an enabled rule uses, by name.
    pub values: BTreeMap<String, Setting>,
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
    /// Whether each git source is read from statute's cache alone, at the
    /// commit the lock file records, without the network.
    pub offline: bool,
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
        let Some(locked) = locked_sources(
            &root.join(crate::lock::FILE),
            crate::lock::FILE,
            env,
            &mut diags,
        ) else {
            return (None, diags);
        };
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
        let (project, mut diags) = Project::from_manifest(root, manifest, diags, env, &locked);
        // Where there is a private rule set, the private layer may enable
        // the rule, and its own pass tells.
        if let Some(project) = &project
            && !(project.manifest.public && has_private_rule_set(root))
        {
            for key in
                resolve::unenabled_own(&project.sources, &project.manifest, &project.resolution)
            {
                diags.warning(
                    manifest::FILE,
                    resolve::unenabled_own_warning(&project.sources, &key),
                );
            }
        }
        (project, diags)
    }

    /// The private layer of the public `project`, read from the private
    /// rule set in `dir` ([`private::dir`]), with the public rule set:
    /// `None` and no finding when there is no such file and the global
    /// configuration turns nothing on, else a layer of its entries
    /// alone. It fails when a
    /// rule that only it enables overrides one that the public rule set
    /// enables (`docs/design/rule-sources.md`, Overrides). Its findings
    /// leave out those of `given`, the public rule set's.
    pub fn load_private(
        project: &Project,
        given: &Diagnostics,
        env: &Env,
        dir: &Path,
    ) -> (Option<Project>, Diagnostics) {
        let mut diags = Diagnostics::default();
        let path = dir.join(private::MANIFEST);
        match std::fs::read_to_string(&path) {
            Ok(text) => Project::with_private(project, given, &text, env, dir),
            // What the global configuration turns on makes a private
            // layer of its own.
            Err(e)
                if e.kind() == std::io::ErrorKind::NotFound
                    && project.manifest.public
                    && !project.manifest.global_enables.is_empty() =>
            {
                Project::with_private(project, given, "", env, dir)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (None, diags),
            Err(e) => {
                diags.error(path.display().to_string(), format!("cannot read: {e}"));
                (None, diags)
            }
        }
    }

    /// As `load_private`, with the text of `private.toml` given. Each git
    /// source is read at the commit that `private.lock` records for it,
    /// or `statute.lock` for a source of the public layer.
    pub fn with_private(
        project: &Project,
        given: &Diagnostics,
        text: &str,
        env: &Env,
        dir: &Path,
    ) -> (Option<Project>, Diagnostics) {
        let root = &project.root;
        let mut diags = Diagnostics::default();
        let shown = dir.join(private::MANIFEST).display().to_string();
        let path = dir.join(private::LOCK);
        let Some(own) = locked_sources(&path, &path.display().to_string(), env, &mut diags) else {
            return (None, diags);
        };
        let fallback = if env.from_lock { &own[..] } else { &[] };
        let manifest = match project.manifest.with_private_from(text, fallback) {
            Ok(m) => m,
            Err(errors) => {
                for e in errors {
                    diags.error(&shown, e);
                }
                return (None, diags);
            }
        };
        let Some(mut locked) = locked_sources(
            &root.join(crate::lock::FILE),
            crate::lock::FILE,
            env,
            &mut diags,
        ) else {
            return (None, diags);
        };
        locked.extend(own);
        // What the private rule set excludes.
        let off: Vec<Key> = manifest
            .exclude
            .iter()
            .filter(|k| !project.manifest.exclude.contains(k))
            .cloned()
            .collect();
        let (combined, mut diags) = Project::from_manifest(root, manifest, diags, env, &locked);
        diags.0.retain(|d| !given.0.contains(d));
        // A finding about an entry of `rules`, `packs`, `domains` or
        // `exclude`, or about a node that what the private rule set
        // excludes may not turn off, that the public rule set alone does
        // not have is about an entry of the private rule set; the rest
        // still ask for a change to `statute.toml`.
        for d in &mut diags.0 {
            if d.file == manifest::FILE
                && (["`rules` ", "`packs` ", "`domains` ", "`exclude` "]
                    .iter()
                    .any(|field| d.message.starts_with(field))
                    || (d.message.contains(" cannot be turned off: ")
                        && turned_off_by(&d.message, &project.sources, &off)))
            {
                d.file.clone_from(&shown);
            } else if d.file == manifest::FILE
                && d.message.starts_with("rule ")
                && d.message.contains(" cannot be turned off: ")
            {
                // A rule statute.toml excludes and only the private layer
                // needs can be turned on again there.
                if let Some(named) = d.message.split_whitespace().nth(1) {
                    // A rule of the repository's own is written by its number.
                    let named = named
                        .strip_prefix(LOCAL)
                        .filter(|id| id.parse::<u64>().is_ok())
                        .unwrap_or(named)
                        .to_owned();
                    d.message.push_str(&format!(
                        "; or, for the private output alone, list it in `rules` of {}: `statute add --private {named}`",
                        private::MANIFEST
                    ));
                }
            }
        }
        let Some(combined) = combined else {
            return (None, diags);
        };
        let public = &project.resolution.enabled;
        for (target, by) in &combined.resolution.overridden {
            if public.contains_key(target) && !public.contains_key(by) {
                diags.error(
                    &shown,
                    format!(
                        "`{by}` overrides \"{target}\", a rule of the public rule set `{}`; in a public repository (`public = true`), a rule `{}` enables overrides no rule of the public layer",
                        manifest::FILE,
                        private::MANIFEST
                    ),
                );
            }
        }
        if diags.has_errors() {
            return (None, diags);
        }
        // A rule of a source the private layer adds is listed in the
        // private rule set.
        for key in
            resolve::unenabled_own(&combined.sources, &combined.manifest, &combined.resolution)
        {
            let file = if project.sources.source(&key.source).is_some() {
                manifest::FILE
            } else {
                &shown
            };
            let message = resolve::unenabled_own_warning(&combined.sources, &key);
            if !given.0.iter().any(|d| d.message == message) {
                diags.warning(file, message);
            }
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
        // The global configuration serves every repository, whose sources
        // may lack a node it names.
        let absent: Vec<Key> = manifest
            .from_global
            .iter()
            .filter(|k| sources.get(k).is_none())
            .cloned()
            .collect();
        for list in [
            &mut manifest.rules,
            &mut manifest.packs,
            &mut manifest.domains,
            &mut manifest.from_global,
        ] {
            list.retain(|k| !absent.contains(k));
        }

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
        let values = crate::param::settle(&sources, &manifest, &resolution, &mut diags);
        if diags.has_errors() {
            return (None, diags);
        }
        let project = Project {
            root: root.to_owned(),
            manifest,
            sources,
            revision,
            resolution,
            values,
        };
        (Some(project), diags)
    }
}

/// Whether the node a finding names first, after its kind, is in `off`
/// or under a node of `off`, as far as `sources` know it.
fn turned_off_by(message: &str, sources: &Sources, off: &[Key]) -> bool {
    let Some(named) = message.split_whitespace().nth(1) else {
        return false;
    };
    let mut at = sources
        .nodes()
        .map(|(k, _)| k)
        .find(|k| k.to_string() == named);
    let mut seen = Vec::new();
    while let Some(key) = at {
        if off.contains(&key) {
            return true;
        }
        if seen.contains(&key) {
            break;
        }
        at = sources.parent(&key);
        seen.push(key);
    }
    off.iter().any(|k| k.to_string() == named)
}

/// Whether the work tree at `root` has a private rule set.
fn has_private_rule_set(root: &Path) -> bool {
    private::dir(root).is_ok_and(|dir| dir.is_some_and(|d| d.join(private::MANIFEST).is_file()))
}

/// The git sources the lock file at `path`, named `shown` in messages,
/// records; none when it is missing. With `env.from_lock`, a lock file
/// that cannot be read is an error in `diags`, and the result `None`; the
/// commands that write the lock file take it as missing.
fn locked_sources(
    path: &Path,
    shown: &str,
    env: &Env,
    diags: &mut Diagnostics,
) -> Option<Vec<SourceSpec>> {
    let read = match std::fs::read_to_string(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Some(Vec::new()),
        Err(e) => Err(format!("cannot read: {e}")),
        Ok(text) => crate::lock::sources(&text),
    };
    match read {
        Ok(sources) => Some(sources),
        Err(_) if !env.from_lock => Some(Vec::new()),
        Err(e) => {
            diags.error(shown, format!("{e}; run `statute lock` to write it anew"));
            None
        }
    }
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
                "source `{}` is {} at {}, and the lock file records {} at {}; run `statute lock`",
                spec.name, git.follow, git.address, r.follow, r.address
            ));
        }
        _ => None,
    };
    if env.offline {
        return match commit {
            Some(commit) => crate::global::cached(spec, &commit),
            None if spec.declared == Declared::Global => crate::global::cached_newest(spec),
            None => Err(format!(
                "source `{}`: the lock file records no commit of it, and nothing is fetched; run `statute lock`",
                spec.name
            )),
        };
    }
    crate::global::materialize(spec, commit.as_deref(), env.newest)
}
