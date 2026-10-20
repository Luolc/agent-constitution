//! A repository that uses statute: its rule set, the sources the rule set
//! names with its own rules, and the constitution it rests on, read from
//! disk together.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::cite::Labels;
use crate::constitution::rules::{self, Provisions};
use crate::diag::Diagnostics;
use crate::manifest::{self, Declared, Key, LOCAL, Layers, Manifest, SourceSpec};
use crate::node::{self, Status};
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
    /// The constitution's revision as `index.toml` writes it; `None`
    /// without a constitution.
    pub revision: Option<String>,
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
    /// Whether each git source of the private layer alone is read at its
    /// newest commit, as `upgrade --private` does, rather than at the one
    /// `private.lock` records.
    pub private_newest: bool,
    /// Whether each git source is read from statute's cache alone, at the
    /// commit the lock file records, without the network.
    pub offline: bool,
}

#[derive(Deserialize)]
struct Index {
    edition: String,
    revision: String,
}

/// The edition of the constitution a rule set rests on, as read.
struct Edition {
    /// `constitution/<edition>/` in its source.
    dir: PathBuf,
    /// How its `index.toml` is named in messages.
    index_name: String,
    /// Its revision as `index.toml` writes it, and as a number.
    revision: String,
    number: u64,
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
        let (project, mut diags) =
            Project::from_manifest(root, manifest, diags, env, &locked, None);
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
    /// rule set of `dir` ([`private::dir`]), with the public rule set:
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
        dir: &private::Dir,
    ) -> (Option<Project>, Diagnostics) {
        let mut diags = Diagnostics::default();
        let path = dir.manifest();
        match std::fs::read_to_string(&path) {
            Ok(text) => Project::layer(project, given, Some(&text), env, dir),
            // What the global configuration turns on makes a private
            // layer of its own, in a private repository too.
            Err(e)
                if e.kind() == std::io::ErrorKind::NotFound
                    && !project.manifest.global_enables.is_empty() =>
            {
                Project::layer(project, given, None, env, dir)
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
    /// unless `env.private_newest`, or `statute.lock` for a source of the
    /// public layer.
    pub fn with_private(
        project: &Project,
        given: &Diagnostics,
        text: &str,
        env: &Env,
        dir: &private::Dir,
    ) -> (Option<Project>, Diagnostics) {
        Project::layer(project, given, Some(text), env, dir)
    }

    /// As `with_private`; without `text`, the layer of what the global
    /// configuration turns on alone, which a private repository has too.
    fn layer(
        project: &Project,
        given: &Diagnostics,
        text: Option<&str>,
        env: &Env,
        dir: &private::Dir,
    ) -> (Option<Project>, Diagnostics) {
        let root = &project.root;
        let mut diags = Diagnostics::default();
        let shown = dir.manifest().display().to_string();
        let path = dir.read_lock();
        let Some(own) = locked_sources(&path, &path.display().to_string(), env, &mut diags) else {
            return (None, diags);
        };
        let fallback = if env.from_lock { &own[..] } else { &[] };
        let manifest = match project.manifest.private_layer(text, fallback) {
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
        if !env.private_newest {
            locked.extend(own);
        }
        // What the private rule set excludes.
        let off: Vec<Key> = manifest
            .exclude
            .iter()
            .filter(|k| !project.manifest.exclude.contains(k))
            .cloned()
            .collect();
        let (combined, mut diags) = Project::from_manifest(
            root,
            manifest,
            diags,
            env,
            &locked,
            Some(&project.resolution),
        );
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
                && let Some(head) = d.message.strip_suffix(resolve::REVISION_ADVICE)
            {
                // The public layer has passed this check, so the rule is
                // one the private layer enables, and only the public
                // sources can bring another revision.
                let upgrade = if env.newest {
                    ""
                } else {
                    ": `statute upgrade` moves its sources to their newest commits"
                };
                d.message = format!(
                    "{head}take the rule out of {}, or move the public layer to a revision it accepts{upgrade}",
                    private::MANIFEST
                );
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
    /// `diags`; `public`, the public layer's resolution, for a private
    /// layer.
    fn from_manifest(
        root: &Path,
        mut manifest: Manifest,
        mut diags: Diagnostics,
        env: &Env,
        locked: &[SourceSpec],
        public: Option<&Resolution>,
    ) -> (Option<Project>, Diagnostics) {
        for spec in &mut manifest.sources {
            if let Err(e) = pick_commit(spec, locked, env) {
                diags.error(manifest::FILE, e);
            }
        }
        if diags.has_errors() {
            return (None, diags);
        }
        let load =
            |dir: &Path, label: &str| Source::load_for(dir, label, manifest.constitution.is_some());
        let mut list = Vec::new();
        // Where each source's rules are read from, by its name.
        let mut dirs = BTreeMap::new();
        for spec in &manifest.sources {
            let dir = root.join(&spec.path).join(&spec.rules_dir);
            let (mut source, more) = load(&dir, &spec.shown(&spec.rules_dir));
            dirs.insert(spec.name.clone(), dir);
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
            let (mut source, more) = load(&root.join(local_rules), local_rules);
            diags.0.extend(more.0);
            source.name = LOCAL.to_owned();
            dirs.insert(LOCAL.to_owned(), root.join(local_rules));
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

        let mut constitution = None;
        if let Some((c, spec)) = manifest.constitution_spec() {
            let edition_dir = root.join(&spec.path).join("constitution").join(&c.edition);
            let index_name = spec.shown(&format!("constitution/{}/index.toml", c.edition));
            let index = std::fs::read_to_string(edition_dir.join("index.toml"))
                .map_err(|e| {
                    format!(
                        "cannot read the constitution of edition `{}`: {e}",
                        c.edition
                    )
                })
                .and_then(|text| {
                    toml::from_str::<Index>(&text).map_err(|e| e.to_string().trim_end().to_owned())
                })
                .and_then(|index| {
                    if index.edition != c.edition {
                        return Err(format!(
                            "the index is of edition `{}`, and the rule set names `{}`",
                            index.edition, c.edition
                        ));
                    }
                    let number = index.revision.parse::<u64>().map_err(|_| {
                        format!("revision `{}` is not a whole number", index.revision)
                    })?;
                    Ok((index.revision, number))
                });
            match index {
                Ok((revision, number)) => {
                    constitution = Some(Edition {
                        dir: edition_dir,
                        index_name,
                        revision,
                        number,
                    });
                }
                Err(e) => {
                    diags.error(index_name, e);
                    return (None, diags);
                }
            }
        }

        if diags.has_errors() {
            return (None, diags);
        }
        let number = constitution.as_ref().map(|c| c.number);
        let lang = manifest.lang.clone();
        let cite = |resolution: &Resolution, diags: &mut Diagnostics| {
            check_citations(
                constitution.as_ref(),
                &lang,
                &sources,
                &dirs,
                resolution,
                diags,
            );
        };
        if let Some(public) = public {
            fit_global(&sources, &mut manifest, number, public, &cite);
        }
        let resolution = resolve::resolve(&sources, &manifest, number, &mut diags);
        let values = crate::param::settle(&sources, &manifest, &resolution, &mut diags);
        cite(&resolution, &mut diags);
        if diags.has_errors() {
            return (None, diags);
        }
        let revision = constitution.map(|c| c.revision);
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

/// What checks a resolution's citations of the constitution.
type Cite<'a> = dyn Fn(&Resolution, &mut Diagnostics) + 'a;

/// Why the private layer `manifest` fails to load over `public`, the
/// public layer's resolution: its first error in resolving or in what
/// `cite` checks, or a rule it enables that overrides a rule of the
/// public layer; `None` when it loads.
fn fails(
    sources: &Sources,
    manifest: &Manifest,
    number: Option<u64>,
    public: &Resolution,
    cite: &Cite,
) -> Option<String> {
    let mut diags = Diagnostics::default();
    let res = resolve::resolve(sources, manifest, number, &mut diags);
    cite(&res, &mut diags);
    if let Some(e) = diags.errors().next() {
        return Some(e.message.clone());
    }
    res.overridden
        .iter()
        .find(|(target, by)| {
            public.enabled.contains_key(*target) && !public.enabled.contains_key(*by)
        })
        .map(|(target, by)| format!("{by} would override {target}, a rule of the public layer"))
}

/// Leaves out of the private layer `manifest` each entry of the global
/// configuration that makes it fail, in order, with a note: the
/// repository's own choices, such as what `statute.toml` excludes, come
/// before the person's. Nothing is left out when the layer fails without
/// those entries too, for that failure is the private rule set's own.
fn fit_global(
    sources: &Sources,
    manifest: &mut Manifest,
    number: Option<u64>,
    public: &Resolution,
    cite: &Cite,
) {
    if manifest.from_global.is_empty() || fails(sources, manifest, number, public, cite).is_none() {
        return;
    }
    let given = manifest.from_global.clone();
    let without = |m: &Manifest, drop: &[Key]| {
        let mut m = m.clone();
        for list in [&mut m.rules, &mut m.packs, &mut m.domains] {
            list.retain(|k| !drop.contains(k));
        }
        m
    };
    let full = manifest.clone();
    if fails(sources, &without(&full, &given), number, public, cite).is_some() {
        return;
    }
    let mut kept: Vec<Key> = Vec::new();
    for key in &given {
        let rest: Vec<Key> = given
            .iter()
            .filter(|k| *k != key && !kept.contains(k))
            .cloned()
            .collect();
        match fails(sources, &without(&full, &rest), number, public, cite) {
            None => kept.push(key.clone()),
            Some(why) => manifest.left_out.push(format!(
                "the global configuration's `{key}` is left out here: {why}"
            )),
        }
    }
    let dropped: Vec<Key> = given.into_iter().filter(|k| !kept.contains(k)).collect();
    let left = without(manifest, &dropped);
    manifest.rules = left.rules;
    manifest.packs = left.packs;
    manifest.domains = left.domains;
    manifest.from_global = kept;
}

/// Checks what the repository's own nodes, all but the repealed, and the
/// enabled rules of the other sources cite of the edition `constitution`
/// of the constitution: each `authority` entry and `{@constitution:...}` citation
/// names a provision of it that is not repealed, as `statute validate`
/// checks a source beside its constitution, with each finding at its line
/// of the node's file in `dirs`, the directory of each source by name.
/// Without a constitution, `authority` is not checked, and every
/// `{@constitution:...}` citation is an error, for it has nothing to
/// render as.
fn check_citations(
    constitution: Option<&Edition>,
    lang: &str,
    sources: &Sources,
    dirs: &BTreeMap<String, PathBuf>,
    resolution: &Resolution,
    diags: &mut Diagnostics,
) {
    let provisions = match constitution {
        None => Provisions::none(),
        Some(edition) => match Labels::load(&edition.dir.join("articles"), lang) {
            Ok(labels) => Provisions::of_labels(&labels),
            Err(e) => {
                diags.error(
                    &edition.index_name,
                    format!("{e}, so what the rules cite of the constitution is not checked"),
                );
                return;
            }
        },
    };
    for (key, n) in sources.nodes() {
        let own = sources.source(&key.source).is_some_and(|s| s.local);
        if n.status == Status::Repealed || !(own || resolution.enabled.contains_key(&key)) {
            continue;
        }
        let file = dirs
            .get(&key.source)
            .map(|d| d.join(node::file_name(key.id)))
            .unwrap_or_default();
        for (needle, message) in provisions.findings(n) {
            diags.error(
                rules::at(&file, &sources.file(&key), &needle),
                format!("{} {key}: {message}", n.kind.name()),
            );
        }
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
    private::dir(root).is_ok_and(|dir| dir.is_some_and(|d| d.manifest().is_file()))
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
