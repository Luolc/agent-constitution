//! Findings of a validation, each tied to the file it is about.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// The file the finding is about, as the user would name it
    /// (`rules/42.toml`).
    pub file: String,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{}: {label}: {}", self.file, self.message)
    }
}

/// The findings of one run, errors and warnings together.
#[derive(Debug, Default)]
pub struct Diagnostics(pub Vec<Diagnostic>);

impl Diagnostics {
    pub fn error(&mut self, file: impl Into<String>, message: impl Into<String>) {
        self.push(Severity::Error, file, message);
    }

    pub fn warning(&mut self, file: impl Into<String>, message: impl Into<String>) {
        self.push(Severity::Warning, file, message);
    }

    fn push(&mut self, severity: Severity, file: impl Into<String>, message: impl Into<String>) {
        self.0.push(Diagnostic {
            severity,
            file: file.into(),
            message: message.into(),
        });
    }

    pub fn has_errors(&self) -> bool {
        self.0.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter().filter(|d| d.severity == Severity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter().filter(|d| d.severity == Severity::Warning)
    }
}
