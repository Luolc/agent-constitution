# agent-constitution

A constitution for coding agents, a general rule library and skills, and a
planned toolchain that compiles them, together with a private overlay that
holds one user's local values and rules, into the single instruction file
that Claude Code, Codex and Grok read. The repo is at the draft stage: it
holds the constitution draft, the [design](docs/design.md) and the script
that builds the constitution; the rule compiler and checkers are designed
but not yet written.

The constitution is kept in English and Chinese, both built from the same
sources: [English](constitution/constitution.en.md),
[中文](constitution/constitution.zh.md). The sources are TOML files under
`constitution/articles/`, one directory per article and one file per
section, each holding both languages; edit them and rebuild, never the two
generated files. The general rules under `rules/` are TOML files of the
same kind, one per rule. The format, the IDs and the numbering are in the
design.
