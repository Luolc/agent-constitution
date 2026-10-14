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
- [Rules and skills](#rules-and-skills)
- [Dependencies, capabilities and packs](#dependencies-capabilities-and-packs)
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
  that writes its own rules keeps them in a source of its own.
- **Rule set**: the rules one user or team has chosen, written as a
  configuration file that lists rule IDs and packs.
- **Lock file**: written next to the rule set; it pins the revision of
  every chosen rule and the commit of every source.
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
  for good.

The ID of a rule therefore always means the same requirement. The revision
is a number per rule, counted from 1; it has nothing to do with the
revision of the constitution.

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

## Rules and skills

A rule says what must be so; a skill says how to get there: steps,
templates, commands, known pitfalls (`skills`). What someone can point to
and say "you broke this" belongs in a rule; what only helps an agent get
it right belongs in a skill.

A skill declares which rules it implements, and it is installed only when
at least one of them is in the rule set. A repository's `AGENTS.md` keeps
only what is specific to that repository (what it is, its language, its
layout, its check commands) and names the rule set it uses
(`division-of-instructions.project-instructions`).

## Dependencies, capabilities and packs

- `requires`: enabling rule A requires rule B. A rule that says how to run
  checks through uv requires the rule that has uv manage the project's
  dependencies. The build fails when a rule set enables A without B.
- `provides`: a rule may provide a named capability, such as "Python
  package manager". Within one rule set, each capability is provided by at
  most one rule. That is how alternatives exclude each other: a uv rule, a
  Poetry rule and a Bazel rule all provide the same capability, and a rule
  set picks one. Rules never list conflicts with each other pairwise.
- Packs: a named group of rules that is often used together (for example,
  Python with uv) can be enabled in a rule set as a whole.

## Tracing rules to the constitution

Every rule cites the provisions of the constitution it refines, in the
`authority` field it has today ([Rules](../design.md#rules)). That link is
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
crate, with one module per rule beside that rule's text and metadata, the
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

For each finding the reviewer decides one of three things:

- a real violation: the change is fixed;
- a false positive: the check flagged something compliant;
- a false negative: something was found that the check let pass.

A false positive or a false negative leads to a proposed fix to the
rule's check. Fixing a check does not change what counts as compliant, so
the rule keeps its ID and its revision goes up by one
(`wrong-checkers.fixing`). People and agents alike may propose changes to
a rule, under `rule-evolution`. This is how the rules improve themselves:
the cheap checks run first, agents look only at what the checks could not
settle, and what the agents conclude goes back into the checks.

## Trial rules

When agents doing a piece of work find that no rule, skill or convention
covers it, they start writing the convention down. The first version is a
trial rule: it is used on real work, changed freely while it is tried, and
made a regular rule once it has settled. The rules, skills and conventions
of this repo itself are meant to come about the same way.

## Compilation and the site

**Locally**, the compiler takes the constitution (the version for agents)
and a rule set with its lock file, and writes:

- an `AGENTS.md`, with the chosen rules in one flat file, a section per
  pack; once its length passes a token threshold, measured with
  `count-tokens`, it is split into several files;
- the skill directories the chosen rules need;
- the check binary and the external commands of the chosen rules;
- the hook configuration that runs the checks.

Compiling, checking and upgrading a rule set are one command-line tool.
The constitution's compiler stays in Python for now
([Compilation](../design.md#compilation)) and moves into that tool later.

**The public site** lists every rule that is not superseded, so that
people can choose from the full list. A superseded rule keeps its page, so
links to it keep working, but it is left out of the lists by default.

## Why Rust

Speed is not the main reason. A few hundred file checks and regular
expressions take a second or two in Python as well; what costs time and
money is the reviewer's model calls. Rust is chosen because:

- the checks ship as one binary, which needs no Python or other runtime,
  so a downstream repository in any language can use them;
- running the checks in parallel is easier to get right;
- a Rust linter shipped as a Python wheel and run from pre-commit,
  [limae](https://github.com/Luolc/limae), already works that way, so the
  distribution path has been tried.

## What this replaces

Once this design is implemented, it replaces parts of the current design.
Those sections are not changed now; they will be when the work that
implements this design lands.

- [IDs](../design.md#ids): rule IDs become numbers, assigned in sequence,
  in place of words. The rules in `rules/*.toml` get numbers when they are
  moved to the new format, and a citation `{@rule:<id>}` then names a
  number.
- [Rules](../design.md#rules): no rule is deleted any more, so
  `rules/retired-ids.txt` is no longer needed; a rule is superseded
  instead, and the fields `supersedes`, a per-rule revision, `requires`
  and `provides` are added. The verification method is part of each
  rule's text, and its check is code (see [Checks](#checks)).
- [Choosing rules](../design.md#choosing-rules): a rule set with a lock
  file, in which rules are listed (by ID or pack), replaces "every general
  rule by default, with named exclusions".
- [Relaxation declarations](../design.md#relaxation-declarations): a
  rule set that leaves a rule out, or picks another rule for the same
  capability, needs no relaxation. Whether `adjustable` values and the
  `relaxes:` declarations stay as they are is decided when this design is
  implemented.
- [Skills](../design.md#skills): a skill declares the rules it implements
  and is installed only when one of them is chosen.
- [Compilation](../design.md#compilation): the output gains skill
  directories, checks and hook configuration, and the rules are grouped by
  pack, not by the article of their first `authority`.
- [Checkers](../design.md#checkers): the rule checks run by the Rust
  binary, and their three outcomes, are added; the existing checkers of the
  repo's sources stay.

## Open questions

- Flat or layered output: at what token count the compiled `AGENTS.md` is
  split into several files, and how.
- How a rule's source is written in an ID, for example `ac:42` for a rule
  of this repo and `myteam:7` for one of a downstream team.
- The format of the lock file.
- How a trial rule becomes a regular rule, and who decides.
- The rules and the skill for writing a design document itself, to be
  written as rules and skills later.
