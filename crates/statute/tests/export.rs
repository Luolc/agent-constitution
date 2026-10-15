//! `statute export`: the constitution and the rules of a directory as
//! JSON, after statute has checked them.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

mod common;
use common::{domain, edit, one_of, option, param, rule, set_default};

const INDEX: &str = r#"status = "active"
edition = "2026-10"
revision = "3"
languages = ["en", "zh"]
"#;

const SECTION: &str = r#"id = "one-a"

[[items]]
id = "x"
number = "1"

[[items]]
id = "y"
number = "2"
"#;

/// A directory with the given rules and, with `constitution`, one
/// edition: article `one`, its section `one-a` and that section's items
/// `x` and `y`.
fn dir(name: &str, constitution: bool, nodes: &[(u64, String)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("export")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("rules")).unwrap();
    if constitution {
        let edition = dir.join("constitution/2026-10");
        std::fs::create_dir_all(edition.join("articles/01-one")).unwrap();
        std::fs::write(edition.join("index.toml"), INDEX).unwrap();
        std::fs::write(
            edition.join("articles/01-one/article.toml"),
            "id = \"one\"\n",
        )
        .unwrap();
        std::fs::write(edition.join("articles/01-one/01-one-a.toml"), SECTION).unwrap();
    }
    for (id, text) in nodes {
        std::fs::write(dir.join(format!("rules/{id}.toml")), text).unwrap();
    }
    dir
}

fn export(dir: &Path) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_statute"))
        .arg("export")
        .arg(dir)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn constitution_and_nodes() {
    let dir = dir(
        "both",
        true,
        &[domain(1), rule(2, 1, "requires = [3]"), rule(3, 1, "")],
    );
    let (code, stdout, stderr) = export(&dir);
    assert_eq!((code, stderr.as_str()), (0, ""));
    let data: Value = serde_json::from_str(&stdout).unwrap();
    let constitution = &data["constitution"];
    assert_eq!(
        (
            &constitution["edition"],
            &constitution["revision"],
            &constitution["status"],
            &constitution["languages"]
        ),
        (
            &json!("2026-10"),
            &json!("3"),
            &json!("active"),
            &json!(["en", "zh"])
        )
    );
    assert_eq!(
        constitution["provisions"],
        json!([
            {"id": "one", "parent": null, "labels": {"en": "Art. I", "zh": "第一条"}},
            {"id": "one-a", "parent": "one", "labels": {"en": "Art. I §1", "zh": "第一条第一款"}},
            {"id": "one-a.x", "parent": "one-a",
             "labels": {"en": "Art. I §1(1)", "zh": "第一条第一款第一项"}},
            {"id": "one-a.y", "parent": "one-a",
             "labels": {"en": "Art. I §1(2)", "zh": "第一条第一款第二项"}},
        ])
    );
    let nodes = data["nodes"].as_array().unwrap();
    let files: Vec<&str> = nodes.iter().map(|n| n["file"].as_str().unwrap()).collect();
    assert_eq!(
        files,
        [
            format!("{}/rules/1.toml", dir.display()),
            format!("{}/rules/2.toml", dir.display()),
            format!("{}/rules/3.toml", dir.display()),
        ]
    );
    // The fields as the file writes them: `default`, the reason's
    // newlines, `requires`.
    let fields = &nodes[1]["fields"];
    assert_eq!(fields["default"], "off");
    assert_eq!(fields["text"]["reason"]["en"], "Because.\n");
    assert_eq!(fields["requires"], json!([3]));
    assert_eq!(nodes[0]["fields"]["kind"], "domain");
}

#[test]
fn on_by_default() {
    let nodes = [
        set_default(domain(1), "\"on\""),
        set_default(rule(2, 1, ""), "\"on\""),
        rule(3, 1, ""),
        one_of(4, 1, 5),
        option(5, 4, ""),
        option(6, 4, ""),
        domain(7),
        set_default(rule(8, 7, ""), "\"on\""),
        edit(
            set_default(rule(9, 1, ""), "\"on\""),
            "status = \"draft\"",
            "status = \"superseded\"\nsuperseded-by = 2",
        ),
    ];
    let (code, stdout, stderr) = export(&dir("on-by-default", false, &nodes));
    assert_eq!((code, stderr.as_str()), (0, ""));
    let data: Value = serde_json::from_str(&stdout).unwrap();
    let on: Vec<(u64, &Value)> = data["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["fields"]["id"].as_u64().unwrap(), &n["on-by-default"]))
        .collect();
    // On in a domain that is on; off; the one-of and its default option;
    // the other option; a domain that is off, and a rule on in it; a rule
    // on but no longer in force.
    assert_eq!(
        on,
        [
            (1, &json!(true)),
            (2, &json!(true)),
            (3, &json!(false)),
            (4, &json!(true)),
            (5, &json!(true)),
            (6, &json!(false)),
            (7, &json!(false)),
            (8, &json!(false)),
            (9, &json!(false)),
        ]
    );
}

#[test]
fn params() {
    // A domain written in Chinese: each default is shown in both.
    let zh = edit(domain(1), "lang = \"en\"", "lang = \"zh\"");
    let zh = edit(zh, "en = \"Domain 1\"", "zh = \"领域 1\"");
    let dir = dir("params", false, &[zh]);
    let declared = [
        param("language.docs", "type = \"language\"", "repo", "zh"),
        param(
            "style.quotes",
            "type = \"choice\"\nvalues = [\"double\", \"single\"]",
            "personal",
            "single",
        ),
    ]
    .concat();
    std::fs::write(dir.join("rules/params.toml"), declared).unwrap();
    let (code, stdout, stderr) = export(&dir);
    assert_eq!((code, stderr.as_str()), (0, ""));
    let data: Value = serde_json::from_str(&stdout).unwrap();
    // In the file's order; a language shown by its name, a choice as it is.
    assert_eq!(
        data["params"],
        json!([
            {"name": "language.docs", "type": "language", "values": null,
             "scope": "repo", "default": "zh",
             "title": {"en": "The language.docs"},
             "shown": {"en": "Chinese", "zh": "中文"}},
            {"name": "style.quotes", "type": "choice", "values": ["double", "single"],
             "scope": "personal", "default": "single",
             "title": {"en": "The style.quotes"},
             "shown": {"en": "single", "zh": "single"}},
        ])
    );
    assert_eq!(data["nodes"].as_array().unwrap().len(), 1);
}

#[test]
fn rules_alone() {
    let other = edit(
        set_default(rule(2, 1, ""), "\"on\""),
        "parent = 1",
        "parent = \"R1\"",
    );
    let (code, stdout, _) = export(&dir("rules-alone", false, &[other]));
    assert_eq!(code, 0);
    let data: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(data["constitution"], Value::Null);
    assert_eq!(data["nodes"][0]["fields"]["parent"], "R1");
    // On, but whether it comes in depends on R1, which is not here.
    assert_eq!(data["nodes"][0].get("on-by-default"), Some(&Value::Null));
}

#[test]
fn a_failed_check_prints_no_data() {
    let stale = edit(
        rule(2, 1, ""),
        "[title]\nen = \"Rule 2\"\n",
        "[title]\nen = \"Rule 2\"\nzh = \"规定 2\"\n",
    );
    let stale = edit(
        stale,
        "en = \"Do the thing 2.\"\n",
        "en = \"Do the thing 2.\"\nzh = \"做 2。\"\n",
    );
    let stale = edit(
        stale,
        "en = '''\nBecause.\n'''\n",
        "en = '''\nBecause.\n'''\nzh = \"因为。\"\n",
    );
    let stale = edit(
        stale,
        "en = \"Violated when the thing is not done.\"\n",
        "en = \"Violated when the thing is not done.\"\nzh = \"没做即违反。\"\n\n[translations]\nzh = \"0000\"\n",
    );
    let (code, stdout, stderr) = export(&dir("stale", true, &[domain(1), stale]));
    assert_eq!((code, stdout.as_str()), (1, ""));
    assert!(
        stderr.contains("rules/2.toml: error: the `zh` translation follows another original"),
        "{stderr}"
    );
}
