# Rule system

Status: draft. The rules of this repo are written in the
[Rule file format](rule-format.md#rule-file-format), which
`constitution-build` checks, and the command reads them and a rule set
and writes the lock file
([The command and the first version](statute-command.md#the-command-and-the-first-version));
the rest is not built yet. This part of the design, in the files below,
describes how rules are written, numbered, chosen, checked and compiled
once the rule system is built. Where it differs from what the rest of
the design says today, the section [What this replaces](#what-this-replaces)
lists the difference; those sections stay as they are until the work that
implements this design changes them.

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
[Rules](rules.md#rules) now describe the rules in it. The rest of this
design, once implemented, replaces parts of the current design. Those
sections are not changed now; they will be when the work that implements
this design lands.

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
- [Skills](skills.md): a skill declares the rules it implements
  and is installed only when one of them is enabled.
- [Compilation](compilation.md): the output gains skill
  directories and checks, and the compiled file opens
  with the repository, edition and revision of the constitution it rests
  on, and the rules are grouped along the parent tree, not by the article
  of their first `authority`.
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
- The command's final name, chosen before its first real release. `statute`
  is held by a placeholder release; another name would be published
  anew.
