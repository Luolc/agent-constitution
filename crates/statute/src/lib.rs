//! statute: reads a source of rules and the rule set a repository chooses,
//! and resolves them into a lock file (`docs/design/rule-system.md`).

pub mod check;
pub mod cite;
pub mod compile;
pub mod diag;
pub mod export;
pub mod global;
pub mod lock;
pub mod manifest;
pub mod node;
pub mod private;
pub mod project;
pub mod resolve;
pub mod site;
pub mod skill;
pub mod source;
pub mod specifier;
