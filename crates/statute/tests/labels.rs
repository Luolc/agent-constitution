//! statute labels a citation of the constitution as the constitution's
//! build does: for every provision of this repository's edition, in every
//! language, `cite::Labels` gives the label of `render.labels` in
//! `tools/compiler`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use statute::cite::Labels;

/// What the constitution's build reads from the sources: the edition's
/// `articles` directory, every provision a citation can name, and the
/// labels by language.
#[derive(serde::Deserialize)]
struct Python {
    articles: String,
    provisions: BTreeSet<String>,
    labels: BTreeMap<String, BTreeMap<String, String>>,
}

const PYTHON: &str = r#"
import json
import sys
from pathlib import Path

from constitution_compiler import read, render
from constitution_compiler.model import Errors

errors = Errors()
src = read.load(Path(sys.argv[1]), errors)
if src is None:
  sys.exit("\n".join(errors))
json.dump(
  {
    "articles": str(src.layout.articles),
    "provisions": sorted(src.provisions),
    "labels": {lang: render.labels(lang, src) for lang in src.languages},
  },
  sys.stdout,
)
"#;

fn python(root: &Path) -> Python {
    let out = Command::new("uv")
        .args(["run", "--frozen", "--no-sync", "python", "-c", PYTHON])
        .arg(root)
        .current_dir(root)
        .output()
        .unwrap_or_else(|e| panic!("cannot run uv ({e}); install uv, then run `uv sync` first"));
    assert!(
        out.status.success(),
        "the constitution's build could not read the sources; run `uv sync` first: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// Each citation whose two labels differ, or that one side lacks, with
/// both labels.
fn mismatches(
    ours: &BTreeMap<String, String>,
    theirs: &BTreeMap<String, String>,
) -> Vec<(String, Option<String>, Option<String>)> {
    let ids: BTreeSet<&String> = ours.keys().chain(theirs.keys()).collect();
    ids.into_iter()
        .filter(|id| ours.get(*id) != theirs.get(*id))
        .map(|id| (id.clone(), ours.get(id).cloned(), theirs.get(id).cloned()))
        .collect()
}

#[test]
fn every_provision_is_labeled_as_the_build_labels_it() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let python = python(&root);
    assert!(!python.provisions.is_empty());
    assert!(!python.labels.is_empty());
    for (lang, theirs) in &python.labels {
        let ours = Labels::load(&root.join(&python.articles), lang).unwrap().0;
        // Both sides label every provision and nothing else, so the
        // comparison below covers each of them.
        let keys = |m: &BTreeMap<String, String>| m.keys().cloned().collect::<BTreeSet<_>>();
        assert_eq!(keys(theirs), python.provisions, "{lang}");
        assert_eq!(keys(&ours), python.provisions, "{lang}");
        let wrong = mismatches(&ours, theirs);
        assert!(
            wrong.is_empty(),
            "{lang}: (citation, statute, Python) {wrong:#?}"
        );

        // A label changed on one side is found, and named with both labels.
        let (id, label) = ours.iter().next_back().unwrap();
        let mut broken = ours.clone();
        broken.insert(id.clone(), format!("{label}x"));
        assert_eq!(
            mismatches(&broken, theirs),
            [(id.clone(), Some(format!("{label}x")), Some(label.clone()))],
            "{lang}"
        );
    }
}
