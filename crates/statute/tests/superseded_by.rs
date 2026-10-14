//! `superseded-by`, one ID or a list: statute and the compiler agree on
//! the cases of `tools/compiler/tests/superseded_by_cases.toml`.

use std::path::Path;

use serde::Deserialize;
use statute::diag::Severity;
use statute::source::Source;

mod common;
use common::{domain, edit, one_of, rule};

#[derive(Deserialize)]
struct Cases {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    value: String,
    passes: bool,
}

#[test]
fn the_compilers_cases() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/compiler/tests/superseded_by_cases.toml");
    let cases: Cases = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(!cases.cases.is_empty());
    for case in cases.cases {
        let eleven = edit(
            rule(11, 2, &format!("superseded-by = {}", case.value)),
            "status = \"draft\"",
            "status = \"superseded\"",
        );
        let files = [
            domain(2),
            one_of(3, 2),
            rule(10, 2, ""),
            eleven,
            rule(12, 2, ""),
        ]
        .into_iter()
        .map(|(id, text)| (format!("rules/{id}.toml"), text.into_bytes()))
        .collect();
        let (_, diags) = Source::from_files(files);
        let errors: Vec<_> = diags
            .0
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert_eq!(
            errors.is_empty(),
            case.passes,
            "{}: {errors:#?}",
            case.value
        );
        assert!(
            errors.iter().all(|d| d.file == "rules/11.toml"),
            "{}: {errors:#?}",
            case.value
        );
    }
}
