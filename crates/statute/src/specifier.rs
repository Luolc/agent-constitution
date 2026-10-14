//! The `constitution` field of a rule: the revisions of the constitution it
//! is compatible with, written as a Python version specifier (`">=1"`,
//! `">=1,<4"`). Constitution revisions are whole numbers, so only the
//! comparison operators on whole numbers are accepted.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Ge,
    Le,
    Gt,
    Lt,
    Eq,
    Ne,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Specifier {
    text: String,
    clauses: Vec<(Op, u64)>,
}

impl Specifier {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut clauses = Vec::new();
        for clause in text.split(',') {
            let clause = clause.trim();
            // Two-character operators first, so `>=` is not read as `>`.
            let (op, rest) = [
                ("===", None),
                ("~=", None),
                (">=", Some(Op::Ge)),
                ("<=", Some(Op::Le)),
                ("==", Some(Op::Eq)),
                ("!=", Some(Op::Ne)),
                (">", Some(Op::Gt)),
                ("<", Some(Op::Lt)),
            ]
            .into_iter()
            .find_map(|(sym, op)| clause.strip_prefix(sym).map(|rest| (op.ok_or(sym), rest)))
            .ok_or_else(|| {
                format!(
                    "`{text}` is not a version specifier: each clause is an operator and a revision, as in `>=1` or `>=1,<4`"
                )
            })?;
            let op = op.map_err(|sym| {
                format!("`{text}`: `{sym}` is not supported; use >=, <=, >, <, == or !=")
            })?;
            let rest = rest.trim();
            let number = rest
                .parse::<u64>()
                .ok()
                .filter(|_| rest.bytes().all(|b| b.is_ascii_digit()))
                .ok_or_else(|| {
                    format!("`{text}`: `{rest}` is not a revision; constitution revisions are whole numbers")
                })?;
            clauses.push((op, number));
        }
        Ok(Self {
            text: text.to_owned(),
            clauses,
        })
    }

    pub fn contains(&self, revision: u64) -> bool {
        self.clauses.iter().all(|&(op, n)| match op {
            Op::Ge => revision >= n,
            Op::Le => revision <= n,
            Op::Gt => revision > n,
            Op::Lt => revision < n,
            Op::Eq => revision == n,
            Op::Ne => revision != n,
        })
    }
}

impl fmt::Display for Specifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_bound() {
        let s = Specifier::parse(">=2").unwrap();
        assert!(!s.contains(1));
        assert!(s.contains(2));
        assert!(s.contains(9));
    }

    #[test]
    fn range_with_spaces() {
        let s = Specifier::parse(">= 1, <4").unwrap();
        assert!(s.contains(1));
        assert!(s.contains(3));
        assert!(!s.contains(4));
    }

    #[test]
    fn each_operator() {
        for (text, inside, outside) in [
            ("<=3", 3, 4),
            (">3", 4, 3),
            ("<3", 2, 3),
            ("==3", 3, 2),
            ("!=3", 2, 3),
        ] {
            let s = Specifier::parse(text).unwrap();
            assert!(s.contains(inside), "{text} should contain {inside}");
            assert!(!s.contains(outside), "{text} should not contain {outside}");
        }
    }

    #[test]
    fn rejects_what_is_not_a_specifier() {
        for text in [
            "", "1", ">=", ">=1.0", ">=-1", "~=1", "===1", ">=1,", ">=+1",
        ] {
            assert!(
                Specifier::parse(text).is_err(),
                "{text:?} should be rejected"
            );
        }
    }
}
