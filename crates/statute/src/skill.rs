//! The skills of a source, `skills/<name>/SKILL.md`, and the rules each
//! declares it implements (`docs/design/rule-system.md`, Rules and
//! skills): in the front matter, `metadata`, the key `statute-rules`, the
//! IDs separated by spaces, as in `statute-rules: "42 51"`.

use std::path::{Path, PathBuf};

use crate::node::Id;

/// The key under `metadata` that lists the rules a skill implements.
pub const KEY: &str = "statute-rules";

#[derive(Debug)]
pub struct Skill {
    pub name: String,
    /// The skill's directory in the source.
    pub dir: PathBuf,
    /// Empty when the skill declares no rules; it is then never installed.
    pub rules: Vec<Id>,
}

/// Reads every `skills/<name>/SKILL.md` below `source`, sorted by name.
/// No `skills/` directory is no skills. `shown` is how `source` is named in
/// messages.
pub fn load(source: &Path, shown: &str) -> Result<Vec<Skill>, String> {
    let dir = source.join("skills");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(format!(
                "{}: cannot read: {e}",
                crate::project::shown(shown, "skills")
            ));
        }
    };
    let mut skills = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: cannot read: {e}", dir.display()))?;
        let path = entry.path();
        let file = path.join("SKILL.md");
        if !file.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let named = crate::project::shown(shown, &format!("skills/{name}/SKILL.md"));
        let text =
            std::fs::read_to_string(&file).map_err(|e| format!("{named}: cannot read: {e}"))?;
        let rules = declared(&text).map_err(|e| format!("{named}: {e}"))?;
        skills.push(Skill {
            name,
            dir: path,
            rules,
        });
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

/// The rules a `SKILL.md` declares: the value of `statute-rules` in the
/// `metadata` map of its front matter. Reads only that much YAML: a
/// top-level line `metadata:`, then its indented `key: value` lines. A
/// `metadata` written in flow style (`{...}`) that names the key fails
/// instead of passing for no declaration.
fn declared(text: &str) -> Result<Vec<Id>, String> {
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    if lines.next() != Some("---") {
        return Ok(Vec::new());
    }
    let mut in_metadata = false;
    let mut in_flow = false;
    // The front matter starts on line 2.
    for (number, line) in (2..).zip(lines.take_while(|l| *l != "---")) {
        if !line.starts_with([' ', '\t']) {
            in_metadata = false;
            in_flow = false;
            if let Some(rest) = line.strip_prefix("metadata:") {
                let rest = rest.trim();
                in_metadata = rest.is_empty() || rest.starts_with('#');
                in_flow = !in_metadata;
            }
        } else if in_metadata && line.trim_start().starts_with('{') {
            in_metadata = false;
            in_flow = true;
        }
        if in_flow {
            if line.contains(KEY) {
                return Err(flow_style(number));
            }
            continue;
        }
        if !in_metadata {
            continue;
        }
        let Some((key, value)) = line.trim().split_once(':') else {
            continue;
        };
        if key.trim() != KEY {
            continue;
        }
        let value = value.trim();
        let value = ['"', '\'']
            .iter()
            .find_map(|q| value.strip_prefix(*q).and_then(|v| v.strip_suffix(*q)))
            .unwrap_or(value);
        let ids: Result<Vec<Id>, _> = value.split_whitespace().map(str::parse).collect();
        return match ids {
            Ok(ids) if !ids.is_empty() => Ok(ids),
            _ => Err(format!(
                "`metadata.{KEY}` must be rule IDs separated by spaces, as `{KEY}: \"42 51\"`, not `{value}`"
            )),
        };
    }
    Ok(Vec::new())
}

fn flow_style(line: usize) -> String {
    format!(
        "line {line}: `metadata` is written in flow style (`{{...}}`); statute reads `{KEY}` only from a block mapping, the key on its own indented line:\n  metadata:\n    {KEY}: \"42 51\""
    )
}

#[cfg(test)]
mod tests {
    use super::declared;

    #[test]
    fn reads_the_rules_under_metadata() {
        let text = "---\nname: a\ndescription: b\nmetadata:\n  author: x\n  statute-rules: \"42 51\"\n---\n\n# A\n";
        assert_eq!(declared(text), Ok(vec![42, 51]));
        assert_eq!(declared(&text.replace("\"42 51\"", "7")), Ok(vec![7]));
    }

    #[test]
    fn the_key_counts_only_under_metadata_and_in_the_front_matter() {
        assert_eq!(
            declared("---\nname: a\nstatute-rules: \"42\"\n---\n"),
            Ok(vec![])
        );
        assert_eq!(
            declared("---\nname: a\n---\nmetadata:\n  statute-rules: \"42\"\n"),
            Ok(vec![])
        );
        assert_eq!(declared("# no front matter\n"), Ok(vec![]));
    }

    #[test]
    fn a_value_that_is_not_ids_fails() {
        let e = declared("---\nmetadata:\n  statute-rules: \"ac:42\"\n---\n").unwrap_err();
        assert!(
            e.contains("`metadata.statute-rules` must be rule IDs"),
            "{e}"
        );
        assert!(declared("---\nmetadata:\n  statute-rules: \"\"\n---\n").is_err());
    }

    #[test]
    fn a_flow_style_metadata_that_names_the_key_fails() {
        for text in [
            "---\nname: a\nmetadata: {statute-rules: \"42\"}\n---\n",
            "---\nname: a\nmetadata: {author: x,\n  statute-rules: \"42\"}\n---\n",
            "---\nname: a\nmetadata:\n  {statute-rules: \"42\"}\n---\n",
        ] {
            let e = declared(text).unwrap_err();
            assert!(e.starts_with("line "), "{e}");
            assert!(e.contains("only from a block mapping"), "{e}");
            assert!(
                e.contains("\n  metadata:\n    statute-rules: \"42 51\""),
                "{e}"
            );
        }
        assert!(
            declared("---\nname: a\nmetadata: {statute-rules: \"42\"}\n---\n")
                .unwrap_err()
                .starts_with("line 3: ")
        );
        assert!(
            declared("---\nname: a\nmetadata: {author: x,\n  statute-rules: \"42\"}\n---\n")
                .unwrap_err()
                .starts_with("line 4: ")
        );
    }

    #[test]
    fn a_metadata_without_the_key_declares_nothing() {
        for text in [
            "---\nname: a\n---\n",
            "---\nname: a\nmetadata:\n  author: x\n---\n",
            "---\nname: a\nmetadata: {author: x}\n---\n",
        ] {
            assert_eq!(declared(text), Ok(vec![]), "{text}");
        }
    }
}
