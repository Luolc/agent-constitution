# agent-constitution design

Status: draft. This is the one design of the repo, in the files listed
below. The constitution itself is built from `index.toml` and
`articles/` in its edition directory, `constitution/2026-10/`, into
[English](../../constitution/2026-10/generated/constitution.en.md) and
[Chinese](../../constitution/2026-10/generated/constitution.zh.md); this design cites it
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
- [Rule system](rule-system.md): the rule system (numbered rules,
  sources and references, rule sets and lock files, dependencies, checks
  and the review loop), built so far in part at the repository level,
  for one source, and which parts above it will replace; its topics are in the
  four files below.
- [Rule format](rule-format.md): what one rule holds, IDs and
  revisions, the rule file format, rules and skills, and tracing rules
  to the constitution.
- [Rule sets, references and dependencies](rule-sets.md): rule sets and
  the lock file, sources and references, and dependencies, one-ofs and
  packs.
- [Rule checks and review](rule-checks.md): checks, outcomes and the
  review loop, and trial rules.
- [The statute command](statute-command.md): compilation and the site,
  the command and the first version, and why Rust.
- [Rule sources](rule-sources.md): the four kinds of source (global or
  local, public or private), which may cite which, overrides,
  subdomains and the cutover from the hand-written guide; built in part.
- [Global configuration and git sources](global-configuration.md): the
  statute directories in the home directory, the global configuration,
  which source a name means, git sources and adding a source; built in
  part.
- [Rule configuration](rule-config.md): the directory of the local
  rules, parameters such as the languages, which domains and rules
  are on by default, where a rule is used, and no rule under a rule;
  built in part.
