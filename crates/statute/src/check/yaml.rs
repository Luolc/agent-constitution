//! The part of YAML a hook configuration is written in: block mappings
//! and sequences, sequences of scalars in flow style (`[a, "b"]`) on one
//! line, plain and quoted scalars, and comments. Anything else (flow
//! mappings, anchors and aliases, tags, block scalars, a scalar over
//! several lines, more than one document) is an error, so that a check
//! reading the file says it cannot rather than misreading it.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Str(String),
    Seq(Vec<Value>),
    Map(Vec<(String, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_seq(&self) -> Option<&[Value]> {
        match self {
            Value::Seq(items) => Some(items),
            _ => None,
        }
    }
}

struct Line {
    number: usize,
    indent: usize,
    text: String,
}

pub fn parse(text: &str) -> Result<Value, String> {
    let mut lines = Vec::new();
    for (number, raw) in (1..).zip(text.lines()) {
        let content = strip_comment(raw.trim_end_matches('\r')).trim_end();
        let text = content.trim_start_matches(' ');
        if text.is_empty() {
            continue;
        }
        if text.starts_with('\t') {
            return Err(format!("line {number}: a tab in the indentation"));
        }
        let indent = content.len() - text.len();
        if indent == 0 && (text == "---" || text.starts_with("--- ") || text == "...") {
            if lines.is_empty() && text == "---" {
                continue;
            }
            return Err(format!("line {number}: more than one document"));
        }
        lines.push(Line {
            number,
            indent,
            text: text.to_owned(),
        });
    }
    if lines.is_empty() {
        return Ok(Value::Null);
    }
    let mut i = 0;
    let first = lines[0].indent;
    let value = block(&mut lines, &mut i, first)?;
    match lines.get(i) {
        Some(line) => Err(format!("line {}: unexpected indentation", line.number)),
        None => Ok(value),
    }
}

/// The node that starts at line `i`, indented by `indent`.
fn block(lines: &mut [Line], i: &mut usize, indent: usize) -> Result<Value, String> {
    let line = &lines[*i];
    if is_item(&line.text) {
        seq(lines, i, indent)
    } else if split_key(&line.text, line.number)?.is_some() {
        map(lines, i, indent)
    } else {
        let value = scalar(&line.text, line.number)?;
        *i += 1;
        Ok(value)
    }
}

/// The node below a key or an item with nothing after it: the lines
/// indented deeper, or null.
fn nested(lines: &mut [Line], i: &mut usize, indent: usize) -> Result<Value, String> {
    match lines.get(*i) {
        Some(line) if line.indent > indent => {
            let deeper = line.indent;
            block(lines, i, deeper)
        }
        _ => Ok(Value::Null),
    }
}

fn is_item(text: &str) -> bool {
    text == "-" || text.starts_with("- ")
}

fn seq(lines: &mut [Line], i: &mut usize, indent: usize) -> Result<Value, String> {
    let mut items = Vec::new();
    while lines
        .get(*i)
        .is_some_and(|l| l.indent == indent && is_item(&l.text))
    {
        let line = &mut lines[*i];
        let rest = line.text[1..].trim_start_matches(' ').to_owned();
        if rest.is_empty() {
            *i += 1;
            items.push(nested(lines, i, indent)?);
        } else {
            // The item's content is read as a line of its own, indented
            // to where it starts, so that the keys below it line up.
            line.indent += line.text.len() - rest.len();
            line.text = rest;
            let inner = line.indent;
            items.push(block(lines, i, inner)?);
        }
    }
    Ok(Value::Seq(items))
}

fn map(lines: &mut [Line], i: &mut usize, indent: usize) -> Result<Value, String> {
    let mut entries: Vec<(String, Value)> = Vec::new();
    while lines
        .get(*i)
        .is_some_and(|l| l.indent == indent && !is_item(&l.text))
    {
        let number = lines[*i].number;
        let Some((key, value)) = split_key(&lines[*i].text, number)? else {
            return Err(format!("line {number}: expected `key: value`"));
        };
        *i += 1;
        let value = if value.is_empty() {
            // A sequence may sit at the indentation of its key.
            if lines
                .get(*i)
                .is_some_and(|l| l.indent == indent && is_item(&l.text))
            {
                seq(lines, i, indent)?
            } else {
                nested(lines, i, indent)?
            }
        } else {
            let value = inline(&value, number)?;
            if let Some(next) = lines.get(*i).filter(|l| l.indent > indent) {
                return Err(format!("line {}: a value over several lines", next.number));
            }
            value
        };
        if entries.iter().any(|(k, _)| *k == key) {
            return Err(format!("line {number}: `{key}` repeats"));
        }
        entries.push((key, value));
    }
    Ok(Value::Map(entries))
}

/// The key and the rest of a `key: value` line, or `None` when the line
/// is not one.
fn split_key(text: &str, number: usize) -> Result<Option<(String, String)>, String> {
    if text.starts_with(['[', '{']) {
        return Ok(None);
    }
    if text.starts_with('?') {
        return Err(format!("line {number}: a complex key"));
    }
    let (key, rest) = if text.starts_with(['"', '\'']) {
        let (key, end) = quoted(text, number)?;
        (key, &text[end..])
    } else {
        let colon = text
            .match_indices(':')
            .map(|(at, _)| at)
            .find(|&at| text[at + 1..].is_empty() || text[at + 1..].starts_with(' '));
        match colon {
            Some(at) => (text[..at].trim_end().to_owned(), &text[at..]),
            None => return Ok(None),
        }
    };
    match rest.strip_prefix(':') {
        Some(value) if value.is_empty() || value.starts_with(' ') => {
            Ok(Some((key, value.trim().to_owned())))
        }
        _ => Ok(None),
    }
}

/// A value on the same line as its key.
fn inline(text: &str, number: usize) -> Result<Value, String> {
    let Some(inner) = text.strip_prefix('[') else {
        return scalar(text, number);
    };
    let Some(inner) = inner.strip_suffix(']') else {
        return Err(format!("line {number}: a flow sequence over several lines"));
    };
    let mut items = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut chars = inner.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match (quote, c) {
            (None, '"' | '\'') if inner[start..at].trim().is_empty() => quote = Some(c),
            (Some('"'), '\\') => {
                chars.next();
            }
            (Some('\''), '\'') if chars.peek().is_some_and(|&(_, n)| n == '\'') => {
                chars.next();
            }
            (Some(q), _) if c == q => quote = None,
            (None, ',') => {
                items.push(scalar(inner[start..at].trim(), number)?);
                start = at + 1;
            }
            _ => {}
        }
    }
    let last = inner[start..].trim();
    // A trailing comma, and `[]`, leave nothing after the last comma.
    if !last.is_empty() {
        items.push(scalar(last, number)?);
    }
    Ok(Value::Seq(items))
}

fn scalar(text: &str, number: usize) -> Result<Value, String> {
    if text.starts_with(['"', '\'']) {
        let (value, end) = quoted(text, number)?;
        if end != text.len() {
            return Err(format!("line {number}: text after a quoted string"));
        }
        return Ok(Value::Str(value));
    }
    if text.is_empty() || text.starts_with(['[', '{', '&', '*', '!', '|', '>', '%', '@', '`']) {
        return Err(format!(
            "line {number}: `{text}` is YAML statute does not read"
        ));
    }
    Ok(Value::Str(text.to_owned()))
}

/// The string quoted at the start of `text`, and where the quotes end.
fn quoted(text: &str, number: usize) -> Result<(String, usize), String> {
    let mut chars = text.char_indices();
    let Some((_, q)) = chars.next() else {
        unreachable!("a quoted string starts with its quote")
    };
    let mut value = String::new();
    while let Some((at, c)) = chars.next() {
        match c {
            '\'' if q == '\'' => {
                if text[at + 1..].starts_with('\'') {
                    chars.next();
                    value.push('\'');
                } else {
                    return Ok((value, at + 1));
                }
            }
            '"' if q == '"' => return Ok((value, at + 1)),
            '\\' if q == '"' => match chars.next().map(|(_, e)| e) {
                Some(e @ ('\\' | '"' | '/')) => value.push(e),
                Some('n') => value.push('\n'),
                Some('t') => value.push('\t'),
                _ => return Err(format!("line {number}: an escape statute does not read")),
            },
            _ => value.push(c),
        }
    }
    Err(format!("line {number}: a quoted string over several lines"))
}

/// The line without its comment: a `#` at the start or after a space,
/// outside quotes.
fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    let mut prev = ' ';
    let mut chars = line.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match (quote, c) {
            // A quote opens a string only where a scalar can start.
            (None, '"' | '\'') if matches!(prev, ' ' | '[' | ',') => quote = Some(c),
            (None, '#') if prev == ' ' => return &line[..at],
            (Some('"'), '\\') => {
                chars.next();
            }
            (Some('\''), '\'') if chars.peek().is_some_and(|&(_, n)| n == '\'') => {
                chars.next();
            }
            (Some(q), _) if c == q => quote = None,
            _ => {}
        }
        prev = c;
    }
    line
}

#[cfg(test)]
mod tests {
    use super::{Value, parse};

    fn s(text: &str) -> Value {
        Value::Str(text.to_owned())
    }

    fn map(entries: &[(&str, Value)]) -> Value {
        Value::Map(
            entries
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
        )
    }

    #[test]
    fn reads_a_hook_configuration() {
        let text = r#"---
# A comment.
repos:
- repo: https://example.test/a/b  # trailing
  rev: v1.0
  hooks:
    - id: one
      args: ["--x", '--y=a # b', it''s]
      stages: []
    -   id: two
        entry: sh -c "echo 'a: b'"
- repo: local
  hooks:
  - id: three
    files:
"#;
        let hooks_ab = Value::Seq(vec![
            map(&[
                ("id", s("one")),
                (
                    "args",
                    Value::Seq(vec![s("--x"), s("--y=a # b"), s("it''s")]),
                ),
                ("stages", Value::Seq(vec![])),
            ]),
            map(&[("id", s("two")), ("entry", s("sh -c \"echo 'a: b'\""))]),
        ]);
        let expected = map(&[(
            "repos",
            Value::Seq(vec![
                map(&[
                    ("repo", s("https://example.test/a/b")),
                    ("rev", s("v1.0")),
                    ("hooks", hooks_ab),
                ]),
                map(&[
                    ("repo", s("local")),
                    (
                        "hooks",
                        Value::Seq(vec![map(&[("id", s("three")), ("files", Value::Null)])]),
                    ),
                ]),
            ]),
        )]);
        assert_eq!(parse(text), Ok(expected));
    }

    #[test]
    fn what_it_does_not_read_is_an_error() {
        for (text, error) in [
            (
                "a: {b: c}\n",
                "line 1: `{b: c}` is YAML statute does not read",
            ),
            ("a: &x b\n", "line 1: `&x b` is YAML statute does not read"),
            ("a: |\n  b\n", "line 1: `|` is YAML statute does not read"),
            ("a: b\n  c\n", "line 2: a value over several lines"),
            (
                "a: [b,\n  c]\n",
                "line 1: a flow sequence over several lines",
            ),
            ("a: b\na: c\n", "line 2: `a` repeats"),
            ("a:\n  b: c\n d: e\n", "line 3: unexpected indentation"),
            ("a: b\n---\nc: d\n", "line 2: more than one document"),
        ] {
            assert_eq!(parse(text), Err(error.to_owned()), "{text:?}");
        }
    }
}
