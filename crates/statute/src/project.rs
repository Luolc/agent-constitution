//! A repository that uses statute: its rule set, the sources the rule set
//! names with its own rules, and the constitution it rests on, read from
//! disk together.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::diag::Diagnostics;
use crate::manifest::{self, LOCAL, Manifest};
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
    pub fn load(root: &Path) -> (Option<Project>, Diagnostics) {
        match std::fs::read_to_string(root.join(manifest::FILE)) {
            Ok(text) => Project::with_manifest(root, &text),
            Err(e) => {
                let mut diags = Diagnostics::default();
                diags.error(manifest::FILE, format!("cannot read: {e}"));
                (None, diags)
            }
        }
    }

    /// As `load`, with the text of `statute.toml` given, so a changed rule
    /// set can be tried before it is written.
    pub fn with_manifest(root: &Path, text: &str) -> (Option<Project>, Diagnostics) {
        let mut diags = Diagnostics::default();
        let manifest = match Manifest::parse(text) {
            Ok(m) => m,
            Err(errors) => {
                for e in errors {
                    diags.error(manifest::FILE, e);
                }
                return (None, diags);
            }
        };

        let mut list = Vec::new();
        for spec in &manifest.sources {
            let (mut source, more) = Source::load(
                &root.join(&spec.path).join("rules"),
                &shown(&spec.path, "rules"),
            );
            diags.0.extend(more.0);
            source.name = spec.name.clone();
            source.path = spec.path.clone();
            source.public = spec.public;
            source.local = spec.local;
            list.push(source);
        }
        if !manifest.sources.iter().any(|s| s.is_here()) && root.join("rules").is_dir() {
            let (mut source, more) = Source::load(&root.join("rules"), "rules");
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
        let index_name = shown(
            &spec.path,
            &format!("constitution/{}/index.toml", manifest.edition),
        );
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
