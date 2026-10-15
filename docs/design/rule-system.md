# Rule system

Status: draft. The rules of this repo are written in the
[Rule file format](rule-format.md#rule-file-format), which
`statute validate rules` checks. The command, `statute`, builds part of
the repository level of this design, for sources on disk and git
sources: it writes,
resolves and upgrades a rule set and its lock file, compiles them into
an `AGENTS.md` with the skills the enabled rules need, and runs the
checks. What it builds so far, and what it does not build yet,
is under
[The command and the first version](statute-command.md#the-command-and-the-first-version);
the user level is not built. This part of the design, in the files
below, describes how rules are written, numbered, chosen, checked and
compiled, the parts not built yet included. Where it differs from what
the rest of the design says today, the section
[What this replaces](#what-this-replaces) lists the difference; those
sections stay as they are until the work that implements this design
changes them.

## Contents

- [Terms](#terms)
- [Rule format](rule-format.md): what one rule holds, IDs and revisions,
  the rule file format, rules and skills, and tracing rules to the
  constitution.
- [Rule sets, references and dependencies](rule-sets.md): rule sets and
  the lock file, sources and references, and dependencies, one-ofs and
  packs.
- [Rule checks and review](rule-checks.md): checks, outcomes and the
  review loop, and trial rules.
- [The statute command](statute-command.md): compilation and the site,
  the command and the first version, and why Rust.
- [What this replaces](#what-this-replaces)
- [Open questions](#open-questions)

## Terms

- **Rule**: one requirement that refines a provision of the constitution
  (`division-of-instructions.rules`), with a way to tell whether it is
  violated.
- **Source**: a repository that holds rules. This repo is one; a team
  that writes its own rules keeps them in a source of its own (see
  [Sources and references](rule-sets.md#sources-and-references)).
- **Rule set**: the rules one repository has chosen, written as a
  configuration file that lists rule IDs and packs; it may inherit another
  rule set.
- **Domain**: a node at the top of the parent tree (Python, CI, Git,
  security and so on) that only gives a heading (see
  [Dependencies, one-ofs and packs](rule-sets.md#dependencies-one-ofs-and-packs)).
- **One-of**: a node of the parent tree of which at most one child is
  enabled; enabling none is allowed too (see
  [Dependencies, one-ofs and packs](rule-sets.md#dependencies-one-ofs-and-packs)).
- **Lock file**: written next to the rule set; it pins the revision of
  every enabled rule and resolves every source to an exact commit.
- **Check**: the code that decides, for one rule, whether a repository, a
  change or a record complies.
- **Reviewer**: an agent that judges what the checks could not settle.

## What this replaces

[Rule file format](rule-format.md#rule-file-format) is implemented, and
[IDs](source-formats.md#ids), [Source formats](source-formats.md) and
[Rules](rules.md#rules) now describe the rules in it.
[Skills](skills.md) says that `statute compile` installs the skills the
enabled rules need in a repository. The rest of this design, once
implemented, replaces parts of the current design. Those sections are
not changed now; they will be when the work that implements this design
lands.

- [Choosing rules](rules.md#choosing-rules): a rule set with a lock
  file, in which rules are listed (by ID or pack), replaces "every general
  rule by default, with named exclusions". The rule set belongs to a
  repository and may inherit a base rule set, which takes over the part
  of the jurisdiction. Each rule set also names the constitution it rests
  on (see [Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file)).
- [Relaxation declarations](rules.md#relaxation-declarations): a
  rule set that leaves a rule out, or picks another child of a one-of,
  needs no relaxation. Whether `adjustable` values and the `relaxes:`
  declarations stay as they are is decided when this design is
  implemented.
- [Overview](overview.md): the goal "each jurisdiction chooses which
  general rules it uses" and the private overlay's job of choosing them
  move to each repository's rule set (with an inherited base), and the
  overlay's apply step becomes the compile command; the overlay keeps the
  personal part and its own rules.
- [Compilation](compilation.md): `statute compile` already writes a
  repository's `AGENTS.md`, with the skills and checks next to it
  ([Compilation and the site](statute-command.md#compilation-and-the-site)).
  Compilation still describes the planned compiler of the user-level
  file, whose rules are grouped by the article of their first
  `authority`; it changes when the rule system builds the user level
  (see [Open questions](#open-questions)).
- [Checkers](checkers.md#checkers): the rule checks run by the Rust
  binary, and their three outcomes, are added; the existing checkers of
  the repo's sources stay.

## Open questions

- The index of public sources: its format, who may add to it, and when
  it is built.
- How versions are resolved across sources in detail: how the ranges
  that sources declare for each other are matched against tags and
  branches, and what the build reports when they disagree.
- Path segments for monorepos: how an agent behaves in interactive mode,
  which was not measured, and the syntax with which a rule in the root
  file names the directory it applies to.
- Rules at the user level: where a rule whose default level is the user
  level is chosen, where it is locked and where it is compiled to.
- Picking another option of a one-of in the private layer: when
  `private.toml` lists an option other than the one `statute.toml`
  picks in `rules` or `domains`, both are enabled and the build fails
  ([In the rule set](rule-config.md#in-the-rule-set)). Whether the
  private layer's choice should replace the public one in the private
  output is not decided.
- A switch that enables only stable rules: statute has none. Whether to
  build one, and what it would do with a domain enabled for its stable
  rules that also holds trial ones, is not decided.
