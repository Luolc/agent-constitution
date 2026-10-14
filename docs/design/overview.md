# Overview

## Goals and non-goals

Goals:

- Hold the constitution: the few stable principles and the governance rules
  (what binds whom, what only a person may decide, how rules and the
  constitution change). It names no concrete practice: no command, storage
  place or tool.
- Hold the general rule library and general skills: the concrete practices
  that refine the constitution, written without one user's machines,
  accounts or history. The rules are a default set; each jurisdiction chooses
  which of them it uses.
- Ship the toolchain that turns those sources, plus a private overlay, into
  the one instruction file the coding agents read, and the checkers that keep
  the sources, the overlay and the output consistent.
- Be publishable as is. Every file is written so that the repo can be made
  public without rewriting history.

Non-goals:

- Holding anything private: machine names, accounts, secret-store paths,
  issue numbers, conversation history, who said what and when. That lives in
  the private overlay.
- Holding a compiled artifact for a real user. Real output is built on the
  user's side and never committed here.
- Enforcing anything at runtime. Permissions, branch protection and harness
  settings do that; this repo produces text and checks text. Each checker
  below says what it cannot prove.
- Building the feedback pipeline (events, proposals, automatic adoption) now.
  This document only reserves its places and interfaces.
- Depending on any orchestrator. See "[Relation to fleet](fleet-and-migration.md#relation-to-fleet)".

## Two layers

| Public core (this repo) | Private overlay (a separate private repo) |
| --- | --- |
| The constitution | Local parameters: the values the rules leave open (machines, model names, secret-store layout, lists of repos) |
| General rules, a default set each jurisdiction chooses from | Private rules: rules that only make sense on that user's stack |
| General skills | Private skills (tools and workflows specific to that user) |
| Compiler, checkers, private-term scanner | Provenance: the casebook of incidents and decisions, with dates and sources |
| A fictional example overlay and its golden output | The private term list the scanner reads at run time |
| Static fixtures | The apply step that runs the compiler and installs the output |

The overlay never changes or relaxes the constitution. It chooses which
general rules it uses (`division-of-instructions.rules`; see [Choosing rules](rules.md#choosing-rules)), adds
private rules and fills
in parameters. It never edits core files; it pins a core version and builds
against it.

## Language

The constitution and the general rules are kept in English and Chinese.
Every other file here (the skills, the tools, this design) is in English.

Why two languages: the person who approves the constitution and the rules
reads Chinese much faster than English, and that review is the one place
where a person's attention matters most. A Chinese version also helps
readers of the public repo. English stays the default: it is what a public
reader can be assumed to read, and it is the first language in
the constitution's `index.toml`.

Why the rest stays English: skills are installed as they are and tools are
code; one language there means one text to keep correct. The compiled
instruction file is built in one language, which the overlay selects, so an
agent reads one text. A rule is cited by ID from repo files, reviews and
tickets in either language; the ID, not the wording, carries the reference.

Both languages live in the same source file, as two keys of the same TOML
table (see [Source formats](source-formats.md)), so one change edits both and a reviewer sees
them side by side.

The standing of the constitution's texts in its languages is set by the
constitution itself (`amendment-languages`,
`equal-languages`), as drafted; it stays open until a person
approves that text. The rules follow the same terms. What this repo adds:

- The checker only proves structural alignment (every text table has every
  language; the languages define the same IDs in the same order and cite
  the same IDs). There is no check, based on the git diff,
  that a change touching one language also touches the other: fixing a typo
  in one language only is a legitimate change, and such a check would block
  it or teach people to make empty edits. Review covers meaning.
- The Chinese text follows the typography that limae checks with its
  default rules ([Checkers](checkers.md#checkers)): full-width commas, full stops, enumeration
  commas, semicolons and colons; half-width parentheses with one space
  outside. Item numbers render half-width for the same reason, `(**三**) `.
