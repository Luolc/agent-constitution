# agent-constitution design

Status: draft. This is the one design of the repo, in the files listed
below. The constitution itself is built from `constitution/index.toml`
and `constitution/articles/` into [English](../../constitution/constitution.en.md)
and [Chinese](../../constitution/constitution.zh.md); this design cites it
only by ID (see [Source formats, IDs](source-formats.md#ids)) and never
restates it.

The repo provides three kinds of installable content: the constitution,
the general rules and the general skills. A user picks which of them to
install through configuration. How they are installed, and what metadata
that needs, is not decided yet; it will be settled as the toolchain is
built.

## Contents

Each file covers one part and reads on its own; in this order they make
the whole design.

- [Overview](overview.md): goals and non-goals, the two layers, language.
- [File structure](file-structure.md): the directory plan, its stages, and
  what the constitution protects.
- [Source formats](source-formats.md): TOML text tables and IDs.
- [The constitution](constitution.md): its sources, numbering and the
  version for agents.
- [Rules and relaxations](rules.md): rule sources, choosing rules,
  relaxation declarations and local parameters.
- [Skills](skills.md): the skill layout.
- [Compilation](compilation.md): building the instruction file.
- [Static site](static-site.md): the Hugo site and its deployment.
- [Checkers, fixtures and feedback](checkers.md): the checkers, fixtures,
  load verification and the reserved feedback loop.
- [Amendments and releases](amendments-and-releases.md): approving an
  amendment, releases and versions.
- [Fleet and migration](fleet-and-migration.md): relation to fleet and
  migrating from the hand-written guide.
- [Rule system](rule-system.md): the planned rule system (numbered
  rules, rule sets and lock files, dependencies, checks and the review
  loop), not built yet, and which parts above it will replace.
