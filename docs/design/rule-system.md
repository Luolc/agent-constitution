# Rule system

Status: draft. Nothing here is built yet. This document describes how
rules are written, numbered, chosen, checked and compiled once the rule
system is built. Where it differs from what the rest of the design says
today, the section [What this replaces](#what-this-replaces) lists the
difference; those sections stay as they are until the work that implements
this design changes them.

## Contents

- [Terms](#terms)
- [What one rule holds](#what-one-rule-holds)
- [IDs and revisions](#ids-and-revisions)
- [Rule sets and the lock file](#rule-sets-and-the-lock-file)
- [Sources and references](#sources-and-references)
- [Rules and skills](#rules-and-skills)
- [Dependencies, choices and packs](#dependencies-choices-and-packs)
- [Tracing rules to the constitution](#tracing-rules-to-the-constitution)
- [Checks](#checks)
- [Outcomes and the review loop](#outcomes-and-the-review-loop)
- [Trial rules](#trial-rules)
- [Compilation and the site](#compilation-and-the-site)
- [Why Rust](#why-rust)
- [What this replaces](#what-this-replaces)
- [Open questions](#open-questions)

## Terms

- **Rule**: one requirement that refines a provision of the constitution
  (`division-of-instructions.rules`), with a way to tell whether it is
  violated.
- **Source**: a repository that holds rules. This repo is one; a team
  that writes its own rules keeps them in a source of its own (see
  [Sources and references](#sources-and-references)).
- **Rule set**: the rules one repository has chosen, written as a
  configuration file that lists rule IDs and packs; it may inherit another
  rule set.
- **Choice**: a node of the parent tree whose children are mutually
  exclusive alternatives (see
  [Dependencies, choices and packs](#dependencies-choices-and-packs)).
- **Lock file**: written next to the rule set; it pins the revision of
  every chosen rule and resolves every source to an exact commit.
- **Check**: the code that decides, for one rule, whether a repository, a
  change or a record complies.
- **Reviewer**: an agent that judges what the checks could not settle.

## What one rule holds

A rule requires one thing, and one thing only, whose violation can be
decided: given a repository, a change or a record, someone can answer
"was this rule broken?". It works like a lint rule in a linter, scoped
narrowly enough to be checked, which is what `decidable-provisions` asks
of every lower-ranking provision.

The text is three to five sentences: the requirement, the reason, and how
compliance is checked. A rule that needs more is usually several rules
written as one, and is split.

## IDs and revisions

A rule's ID is a number, assigned in sequence and never reused. Rules are
expected to grow fast and to be written and checked mostly by agents, so a
person does not read each one closely; a number avoids debating names, and
it never has to change when the wording does.

No rule is ever deleted. Whether a change keeps the ID depends on one
question: does it change what counts as compliant?

- If not, the rule is edited in place and its revision goes up by one.
  Rewording, a new example, a fix to a bug in its check: what was
  compliant stays compliant, and what was a violation stays one.
- If it does, the change gets a new ID. The old rule is marked
  superseded, names the rule that supersedes it, and stays in the source
  for good. Only a rule of the same source can supersede it (see
  [Sources and references](#sources-and-references)).

The ID of a rule therefore always means the same requirement. The revision
is a number per rule, counted from 1; it has nothing to do with the
revision of the constitution. A rule's status is one of `draft`,
`trial`, `active`, `superseded` or `repealed`. `draft` is what every rule
is while the constitution is a draft; `trial` marks a rule being tried on
real work before it settles (see [Trial rules](#trial-rules)). A rule
withdrawn with nothing in its place is repealed and, like a superseded
one, keeps its ID and its file.

## Rule sets and the lock file

A rule set lists rule IDs (and packs, below). Its lock file records, for
each chosen rule, the revision in use and the commit of the source it came
from, so the same rule set and lock file always build the same output.

Upgrading is an explicit step that rewrites the lock file and shows the
diff of every rule whose revision moved. When the rule set names a
superseded rule, the build says so and names its successor, and one
command replaces the old ID with the new one in the rule set. Downstream
users do not have to follow the supersession chain by hand, and nothing
changes under them until they upgrade.

**Per repository, with inheritance.** A rule set belongs to a repository,
and it may inherit a shared base rule set, so the rules every repository
of a user or team agrees on are written once. A repository adds to the
base, leaves rules out, and picks among the alternatives of a choice. The
jurisdiction of the current design (`division-of-instructions.rules`) is
then the owner of a base rule set that its repositories inherit; the
rule set and compiled output of each repository replace the one rule
choice and compiled file per jurisdiction. Exclusion is checked per rule
set, so one repository can use uv for its Python packages and another
Bazel.

**Monorepos.** A rule set may be split by path into segments, each
naming a directory and the rules for it. Exclusion is checked within each
segment, and the compiler writes each segment's instruction file into its
directory, so one part of a monorepo can use uv and another Bazel. This
depends on how each agent reads instruction files in subdirectories,
which differs between agents and has not been measured here. The first
thing to check when this is implemented is, for each supported agent,
whether it reads the instruction file of a subdirectory it works in; the
segment design is fixed only after that.

**Where it lives.** The design follows a package manager such as uv: a
source is like a package index, a rule set like a project manifest, the
lock file like a lock file, compiling like a sync, and the user-level
cache like a package cache. The repository holds the rule set, the lock
file and the compiled output; an upgrade is one command in the repository
and shows up as a pull request. The user level holds only the tool, a
cache of the sources, and a thin personal layer for what concerns the
person, not the project: the language of replies, reading messages as
speech-to-text, where credentials are kept, how directories are laid out
on the machine. That layer takes over the personal part of the private
overlay already in the design. Each rule declares a default level
(project rules at the repository level, rules about the person or the
machine at the user level), and the user can change it. The reason is
that a person who clones the repository, CI and a cloud agent cannot read
user-level files, and each repository pins its own versions and upgrades
on its own schedule.

## Sources and references

A rule is found by its source and its ID. The commit is not part of
that identity.

**Naming a source.** A rule set gives each source a short local name, in
a `[sources]` table:

```toml
[sources]
ac = "github.com/Luolc/agent-constitution"
team = "example.test/their-team/rules"
```

A reference is the name, a colon and the rule's ID: `ac:42`, `team:7`.
The name means something only inside the rule set that defines it, so two
rule sets may use `ac` for different sources. When a source moves to
another address, one line of the rule set changes and no reference does.

**Versions sit on the source.** The rule set says which tag or branch of
a source to follow; the lock file resolves it to an exact commit. This is
how `pyproject.toml` and `uv.lock` divide the work: the manifest states a
range, the lock file records the exact version. A reference carries no
commit, for two reasons. Upgrading would mean editing every reference.
And if two rules of one source were pinned to different commits, a
`requires` or `conflicts` between them would point at a rule that
differs from the one that was chosen. So a rule set takes one version of
each source.

**Dependencies across sources.** A rule of one source may `requires` a
rule of another, such as `ac:12`. The source of the requiring rule then
declares which versions of `ac` it works with, as a range. At compile
time the build looks for one version of each source that every
declaration accepts. If there is none, it fails and names the sources
whose declarations disagree.

**Supersession stays inside a source.** Only a source can supersede its
own rules. Another source cannot declare that its rule replaces ours. It
can recommend the replacement in the text of its own rule; whether to
switch is decided by the user, in the user's rule set.

**Finding public rules.** Nobody has to run a central server for this.
An index repository can list the known public sources, as Homebrew lists
its taps, and the full list on the website can be built from that index.
This comes late and is not part of the first version.

## Rules and skills

A rule says what must be so; a skill says how to get there: steps,
templates, commands, known pitfalls (`skills`). What someone can point to
and say "you broke this" belongs in a rule; what only helps an agent get
it right belongs in a skill.

A skill declares which rules it implements, and it is installed only when
at least one of them is in the rule set. A repository's `AGENTS.md` keeps
only what is specific to that repository (what it is, its language, its
layout, its check commands) and names the rule set it uses
(`division-of-instructions.project-instructions`). A rule set is chosen
per repository ([Rule sets and the lock file](#rule-sets-and-the-lock-file)).

## Dependencies, choices and packs

Rules are related in two ways, and the tree has a second kind of node:

- `parent`: every rule has exactly one parent, either another rule, a
  choice (below) or one of the domains at the top of the tree (Python, CI,
  Git, security and so on), so the rules form a tree, each domain refined
  level by level. The tree decides where a rule stands in the compiled text
  and on the site, which block it is compiled in and which pack it belongs
  to.
- `requires`: enabling rule A requires another rule B, or a choice
  (below). A rule may require several, and the requirements form a
  directed acyclic graph. A rule that says how to run checks through uv
  requires the rule that has uv manage the project's dependencies. A
  rule of another source may be required too; see
  [Sources and references](#sources-and-references).

One parent is not enough on its own because many rules span domains: "in
CI, run `uv sync --locked` first" depends both on uv and on the CI
service. It sits under the uv rule and requires the choice "CI service",
satisfied by any of its alternatives, such as a GitHub Actions rule.

**Choices.** A choice is a node of the parent tree whose children are
alternatives that exclude each other: within one rule set (or one path
segment of it), at most one child is enabled. The choice "Python package
manager" under Python has the children uv, Poetry and Bazel. A choice has
an ID, and adding one goes through review like adding a rule. A rule that
requires a choice is satisfied when any one of its children is enabled.

**Conflicts found later.** Two rules that nobody saw as alternatives may
turn out not to work together. An agent that suspects this reports it, and
a reviewer judges it. Usually the fix is to change a rule: split it,
narrow it or reword it, and whether it keeps its ID follows the rule in
[IDs and revisions](#ids-and-revisions). When both rules are sound and
cannot be used together, one of them gets `conflicts = [ID]`. The relation
is symmetric and written on one side only; the build checks it both ways.
Three or more rules at one place that conflict with each other are
alternatives, and are collected under a choice instead. A rule may not
be declared in conflict with one that enabling it brings in along `parent`
or `requires`, directly or through other rules, since enabling the one
would enable the other; the build fails on it. Sibling rules under one
parent may conflict.

The build checks these constraints:

- No cycle, in the tree or in the requirements.
- A rule's parent, when it is a rule, counts as one of its requirements:
  enabling a rule requires enabling its parent, and the parent is not
  written again under `requires`. When the parent is a choice, the
  requirement passes through it to the choice's own parent: enabling uv
  under "Python package manager" still requires the rule above that
  choice. A choice counts as enabled when one of its children is.
- No redundant requirement: if A requires B and B requires C, A does not
  also list C.
- A requirement that crosses to another subtree should name a choice, not
  a particular rule. The number of such requirements has no hard limit;
  the build warns about them.
- At most one child of a choice is enabled in a rule set or path segment,
  and no two enabled rules conflict.

When a rule set enables a rule without one it requires, the build either
fails or adds the missing rule; which of the two is an open question. It
is a separate matter from the parent counting as a requirement: that only
spares writing the parent under `requires`, and a missing parent is
handled like any other missing requirement.

Packs: a pack is a subtree, named by its root, and a rule set can enable
it as a whole. When the subtree holds a choice, enabling the pack requires
the rule set to name one alternative for each choice in it; the build
fails otherwise.

## Tracing rules to the constitution

Every rule cites the provisions of the constitution it refines, in the
`authority` field it has today ([Rules](rules.md#rules)). That link is
used in both directions:

- When a provision of the constitution is amended, the rules that cite it
  are listed, and a task is opened to check whether each still fits.
- On the site, under each provision of the constitution, the rules that
  cite it are listed, so a reader sees what the provision turns into in
  practice.

## Checks

There is no language for writing checks, declarative or otherwise. A small
set of built-in check primitives ("this file exists", "this key in this
TOML file has this value") would keep needing one more case, "or", and
"unless", until it became a language of its own that nobody reads well. A
check is code, of one of two kinds, and there is no third.

**Our rules: Rust.** The checks of the rules in this repo are one Rust
crate, with one module per rule that points at the rule's file, the
way ruff and clippy implement each lint as a function. Common operations,
such as reading a key from a TOML file, are ordinary helper functions in
the same crate; an operation becomes a helper on its third repetition, not
before (`simplicity`). The crate builds into one binary, released with the
rules, so a downstream user installs one binary and has every check, with
no scripts to deploy.

When a rule is superseded, its successor gets a new function, copied from
the old one and changed as needed. The old function stays in the code,
marked superseded, and does not run unless asked to.

**Rules written downstream: external commands.** A rule kept in another
source declares its check as a command, in any language. The script lives
in that source's own repository and runs with its own toolchain; the rule
names the tools it needs (for example, uv or Go). The contract:

- exit code 0: the rule is met; 1: it is violated; anything else: the
  check could not decide;
- standard output: JSON giving, for each finding, the location and the
  reason.

A rule without an automatic check is allowed; it goes straight to the
reviewer (below).

## Outcomes and the review loop

A check has three outcomes: pass, violation, and inconclusive. A missing
tool, a timeout and a script that crashes are inconclusive, never a
violation, so a missing toolchain does not turn the checks red.

The automatic checks run first, because they cost least. Then the reviewer
looks at the violations, the inconclusive results and the rules that have
no automatic check. Whether one reviewer takes all of them or several take
a group each, and how the groups are formed, is set in the rule set.

For each rule it is handed, the reviewer decides whether the rule is met
or violated. A violation is fixed. Where the reviewer's decision differs
from the check's, the check was wrong:

- a false positive: the check flagged something that meets the rule;
- a false negative: the check passed something that violates the rule,
  found while reviewing another result or in later work.

Either leads to a proposed fix to the rule's check. Fixing a check does
not change what counts as compliant, so the rule keeps its ID and its
revision goes up by one (`wrong-checkers.fixing`). People and agents alike
may propose changes to a rule, under `rule-evolution`. This is how the
rules improve themselves: the cheap checks run first, agents look only at
what the checks could not settle, and what the agents conclude goes back
into the checks.

## Trial rules

When agents doing a piece of work find that no rule, skill or convention
covers it, they start writing the convention down. The first version is a
trial rule: it is used on real work, changed freely while it is tried, and
made a regular rule once it has settled. The rules, skills and conventions
of this repo itself are meant to come about the same way.

## Compilation and the site

**Locally**, the compiler takes the constitution (the version for agents)
and a rule set with its lock file, and writes:

- an `AGENTS.md`, with the chosen rules in nested blocks along the parent
  tree, one heading level per level of the tree; a subtree whose root is
  not enabled (for a choice: none of its children is) contributes
  nothing, since none of its rules can be enabled without it, and a
  requirement that crosses subtrees is rendered as a link.
  Once the file's length passes a token threshold, measured with
  `count-tokens`, it is split into several files;
- the skill directories the chosen rules need;
- the version of the check binary to install, and the external commands
  of the chosen rules;
- the hook configuration that runs the checks.

Compiling, checking and upgrading a rule set are one command-line tool.
The constitution's compiler stays in Python for now
([Compilation](compilation.md)) and moves into that tool later.

**The public site** lists every rule that is not superseded, so that
people can choose from the full list. The parent tree is its table of
contents, and each rule's page shows its requirements. A superseded rule
keeps its page, so links to it keep working, but it is left out of the
lists by default.

## Why Rust

Speed is not the main reason. A few hundred file checks and regular
expressions take a second or two in Python as well; what costs time and
money is the reviewer's model calls. Rust is chosen because:

- the checks ship as one binary, which needs no Python or other runtime,
  so a downstream repository in any language can use them;
- running the checks in parallel is easier to get right;
- [limae](https://github.com/Luolc/limae), a Rust linter shipped as a
  Python wheel and run from pre-commit, already reaches downstream
  repositories this way, so the distribution path has been tried.

## What this replaces

Once this design is implemented, it replaces parts of the current design.
Those sections are not changed now; they will be when the work that
implements this design lands.

- [IDs](source-formats.md#ids): rule IDs become numbers, assigned in
  sequence, in place of words. The rules in `rules/*.toml` get numbers
  when they are moved to the new format, and a citation `{@rule:<id>}`
  then names a number.
- [Rules](rules.md#rules): no rule is deleted any more, so
  `rules/retired-ids.txt` is no longer needed; a rule is superseded or
  repealed instead. `status` gains `trial` and `superseded`, and the
  fields `supersedes`, a per-rule revision, `parent`, `requires`,
  `conflicts` and a default level (repository or user) are added; choices
  become a kind of node in the parent tree. The verification method is part
  of each rule's text, and its check is code (see [Checks](#checks)).
- [Choosing rules](rules.md#choosing-rules): a rule set with a lock
  file, in which rules are listed (by ID or pack), replaces "every general
  rule by default, with named exclusions". The rule set belongs to a
  repository and may inherit a base rule set, which takes over the part
  of the jurisdiction (see
  [Rule sets and the lock file](#rule-sets-and-the-lock-file)).
- [Relaxation declarations](rules.md#relaxation-declarations): a
  rule set that leaves a rule out, or picks another alternative of a
  choice, needs no relaxation. Whether `adjustable` values and the
  `relaxes:` declarations stay as they are is decided when this design is
  implemented.
- [Overview](overview.md): the goal "each jurisdiction chooses which
  general rules it uses" and the private overlay's job of choosing them
  move to each repository's rule set (with an inherited base), and the
  overlay's apply step becomes the compile command; the overlay keeps the
  personal part and its own rules.
- [Skills](skills.md): a skill declares the rules it implements
  and is installed only when one of them is chosen.
- [Compilation](compilation.md): the output gains skill
  directories, checks and hook configuration, and the rules are grouped
  along the parent tree, not by the article of their first `authority`.
- [Checkers](checkers.md#checkers): the rule checks run by the Rust
  binary, and their three outcomes, are added; the existing checkers of
  the repo's sources stay.

## Open questions

- At what token count the compiled `AGENTS.md` is split into several
  files, and along which subtrees.
- The format of the lock file, and with it how a tag or branch and a
  version range are written in a rule set and in a source's declaration
  of the versions it needs.
- The index of public sources: its format, who may add to it, and when
  it is built.
- Whether a domain at the top of the tree is a rule of its own or only a
  heading.
- The concrete fields and commands of the package-manager model
  ([Rule sets and the lock file](#rule-sets-and-the-lock-file)), settled
  when it is implemented.
- Where a rule whose default level is the user level is chosen, where it
  is locked and where it is compiled to.
- When a rule set enables a rule without one it `requires`, whether the
  build fails or adds the missing rule.
- The directory layout of the rules and their checks: whether a rule's
  text and metadata stay in their own file under `rules/`, with the
  crate's module pointing at it, or move next to the check.
- How a trial rule becomes a regular rule, and who decides.
- The rules and the skill for writing a design document itself, to be
  written as rules and skills later.
