# agent-constitution design

Status: draft. This is the one design document of the repo. The constitution
itself is `constitution/constitution.md`; this document cites it only by ID
(`C<n>`, `C<n>.<k>`, `L<n>`, `H<n>`, `A<n>`) and never restates it.

## Goals and non-goals

Goals:

- Hold the constitution: the few stable principles and the governance rules
  (what binds whom, what only a person may decide, how rules and the
  constitution change).
- Hold the general rule library and general skills: everything that is true
  for any user of this setup, written without one user's machines, accounts
  or history.
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
- Depending on any orchestrator. See "Relation to fleet".

## Two layers

| Public core (this repo) | Private overlay (a separate private repo) |
| --- | --- |
| The constitution | Local parameters: the values the rules leave open (machines, model names, secret-store layout, lists of repos) |
| General rules (`R<n>`) | Private rules (`P<n>`): rules that only make sense on that user's stack |
| General skills | Private skills (for example the orchestra skill) |
| Compiler, checkers, private-term scanner | Provenance: the casebook of incidents and decisions, with dates and sources |
| A fictional example overlay and its golden output | The private term list the scanner reads at run time |
| Static fixtures | The apply step that runs the compiler and installs the output |

The overlay may add and tighten. It may relax only through a declared
`relaxes: L<n>` (see "Relaxation declarations"). It never edits core files;
it pins a core version and builds against it.

## Language

The constitution, the rules, the skills and every other file here are in
English. Reasons:

- The repo is meant to become public, and English is what a public reader
  can be assumed to read.
- All three harnesses read the same compiled file; one language means one
  text to keep correct, with no translation drift between two authoritative
  copies.
- A rule is cited by ID from repo files, reviews and tickets in either
  language; the ID, not the wording, carries the reference.

The cost is that the person who approves the constitution reads it in a
second language, and that review is the one place where a person's attention
matters most. Mitigation: a plain-language explanation in the person's own
language, written per constitution revision, kept outside this repo (on the
private side) and marked as non-authoritative. If the explanation and the
text disagree, the English text wins and the explanation is fixed.

## File structure

Stages follow the plan this repo was started from: A is the constitution
and this design, B is structured sources and deterministic compilation, C is
feedback and proposals (observation only), D is scoped automatic adoption.
Directories for C and D are reserved, not created.

```text
README.md
AGENTS.md                 repo rules for agents working here           A
constitution/
  constitution.md         the constitution                             A
docs/
  design.md               this document                                A
rules/
  R<n>.md                 one general rule per file                    B
skills/
  <name>/SKILL.md         general skills, installed as is              B
schema/                   front matter schemas for constitution,
                          rules, fixtures, the overlay manifest        B
tools/                    compiler, checkers, private-term scanner     B
examples/
  overlay/                a fictional private overlay                  B
  golden/                 its compiled output and build manifest       B
fixtures/
  static/                 deterministic fixtures, happy and redline    B
  behavior/               real-agent scenarios                         C
cases/                    K<n>: decided cases                          C
proposals/                reserved for the adoption pipeline           D
.github/workflows/        CI                                           A
.pre-commit-config.yaml   commit hook                                  A
.fleet/config.json        issue-tracker metadata for fleet; the
                          toolchain never reads it                     A
```

Stage B, in the order to build it: `schema/` and the schema checker,
`rules/` (empty at first, filled by migration), the compiler, `examples/`
with the drift check, the reference and relaxation checkers, the
private-term scanner, `fixtures/static/` with the fixture gate, then load
verification. Each step is usable on its own.

The protected foundations of C8 map to these paths in the core:
`constitution/` (the constitution), `tools/` and `schema/` (compiler and
checkers), `fixtures/` and `examples/golden/` (the fixture gate and what it
compares against), and `.github/workflows/` (the CI that runs the checkers
and gates releases). The configuration that loads instructions and guards
lives on the private side and is protected there. A change to any of these
paths follows C8, not the rule-library path of C7.

## Source formats

All sources are Markdown with a YAML front matter block. Front matter is
restricted to a flat subset (`key: scalar` and `key: [a, b]`), so the tools
need no YAML library and the schema checker can reject anything else.

### Constitution

As agreed with the constitution PR: `constitution/constitution.md`, front
matter with at least `status` and `revision`; articles are `## C<n>. <Title>`;
clauses that need their own citation are `C<n>.<k>`; the permitted
relaxations are `L<n>`; the boundary lists are `H<n>` (must go to a person)
and `A<n>` (the agent settles it, never asks). IDs are stable and never
reused. `revision` goes up by one with each adopted amendment; the build
manifest records it.

### Rules

One rule per file, `rules/<id>.md`, so that one proposal is one file and a
diff shows exactly one rule changing.

```markdown
---
id: R12
status: active
scope: all
authority: [C3, C3.2]
---

# <Title>

<statement: condition, action, exception, what to do when unsure>
```

- `id`: stable, also the file name. Renaming the title never changes it.
- `status`: `draft`, `active` or `repealed`. Only `active` is compiled. A
  repealed rule keeps its file and its ID.
- `scope`: who it addresses: `all`, or one of a short list defined in
  `schema/` (for example `implementer`, `reviewer`, `merger`). The compiler
  renders it as a one-line prefix; it does not filter by it. There is one
  compiled file for every role.
- `authority`: the constitution articles or clauses the rule rests on. A
  rule with no authority does not compile.
- `adjustable` (optional): the values in this rule that a lower layer may
  change under L2, each with its default, for example
  `adjustable: [wait_hours=24]`. A rule without it cannot be relaxed under
  L2. These are distinct from local parameters (below): a local parameter is
  a value the private overlay fills in; an adjustable value is one a repo or
  a task may change.

More fields (`supersedes`, `verification`, `expires`) are added when a real
rule needs them, not before.

IDs: rules migrated from the current hand-written guide keep their `R<n>`, so
the references already in repo files, reviews and the casebook keep their
meaning. New general rules continue the `R` sequence in this repo. Rules
that exist only in the private overlay use `P<n>`, so the two sides never
compete for the next number. Retired numbers stay retired. The compiler
checks that the union of core and overlay IDs is unique.

### Relaxation declarations

A lower layer (a repo's `AGENTS.md`, or a task file) relaxes a rule only by
naming the constitution's permission for it:

```markdown
## Relaxations

- relaxes: L2
  target: R42/wait_hours=4
  scope: <this repo, or this task>
  condition: <how this layer meets the condition L2 states>
```

One item per relaxation, with all four keys: the permission (`L<n>` from the
list in C6.6), what it relaxes (a rule, a boundary item such as `H2`, or
under L2 a rule's adjustable value and its new setting), where it applies,
and how the condition of that `L<n>` is met. The private overlay declares its
relaxations of core rules the same way, in a `relaxations.md` of its own. A relaxation not declared this way has
no effect, whatever else the file says.

### Local parameters

A rule that needs a local value names a parameter instead of the value:
`{{ models.reviewer.default }}`. The overlay's `params.toml` supplies the
values. Parameters are plain values, never rule text, so the private side can
change a model name or a machine list without touching a rule. An
unresolved or unused parameter fails the build.

### Skills

`skills/<name>/SKILL.md` in the usual skill layout. Skills are not compiled
into the instruction file; the overlay's apply step installs them where the
harnesses look for skills. A skill says how to do something; what must be
done stays in a rule, and the skill cites the rule ID.

## Compilation

Input: one pinned version of this repo (a tag, checked against its commit
SHA) and one overlay directory. Output: one file, installed by the overlay as
the user-level instruction file that all three harnesses read through
symlinks (`~/.agents/AGENTS.md` in the current setup), plus a build
manifest. Nothing else.

Overlay layout:

```text
overlay.toml       core version, output path, size budget
params.toml        parameter values
rules/P<n>.md      private rules
relaxations.md     the overlay's relaxations of core rules, if any
skills/<name>/     private skills
```

The compiler is a deterministic concatenation, no model involved:

1. Validate every input (schema, references, the overlay's relaxations);
   stop on the first class of error, print all errors of that class.
2. Emit a header: generated file, do not edit; the core version, the
   constitution revision, the manifest digest; edit the sources instead.
3. Emit the constitution, then active rules (core and overlay) grouped by
   their first `authority` article and ordered by ID inside a group.
   Parameters are substituted here.
4. Before each article and rule, emit a source comment such as `<!-- C3 -->`
   or `<!-- R12 -->`, so any paragraph of the output can be traced to its
   file.
5. Measure the result against the budget in `overlay.toml` (the core sets a
   ceiling the overlay cannot raise). Over budget fails the build; nothing is
   silently dropped.
6. Write the output and the manifest: the SHA-256 of each input file (sorted
   by path), core version and commit, compiler version (the core version,
   since the compiler lives here), constitution revision, output digest and
   size. No timestamps; the same inputs give byte-identical output.

Roles: there is one output for everyone. The core has no role sections. The
overlay contributes, as a private rule, the one or two sentences that tell
an agent how to tell whether it runs under fleet or as an orchestra, and
which skill an orchestra reads.

Wiring on the private side: the overlay's apply step fetches the pinned core
version, verifies tag and commit, runs the compiler, runs the checkers that
need private input (the private-term scan), installs the output and the
skills, then runs load verification. The real output is never committed to
this repo.

## Checkers

Every checker is a command that exits non-zero on failure and prints file and
position. What each one proves, and what it does not:

| Checker | Proves | Does not prove |
| --- | --- | --- |
| Schema | Front matter has the required keys, allowed values and the flat subset; IDs match their file names | That the text means what the fields claim |
| References | Every cited ID (`C`, `L`, `H`, `A`, `R`, `P`, `K`) exists and is not repealed; no source points to a file outside the core or the overlay | That the cited article actually supports the rule |
| Drift | `examples/golden/` equals what the compiler builds from `examples/overlay/` today | That a real overlay builds; only that the compiler did not change behavior unnoticed |
| Private-term scan | No hit from the generic classes (secret-store paths, private address ranges, home-directory paths, email addresses) in CI; no hit from the private term list when the overlay runs it | That nothing private is present: a term not on the list passes |
| Relaxations | Each `relaxes:` names an existing `L<n>` open to that layer, an existing target, a scope and a condition; under L2 the target rule lists the parameter as `adjustable` | That the condition is really met, or that no undeclared relaxation exists |
| Fixture gate | Every fixture binds to existing IDs and passes; redline fixtures are never skipped | That agents behave as the fixtures expect outside these cases |

The private-term scan prints only position and class, never the matched
text. In CI it runs the generic classes only; the term list is injected at
run time on the private side (`--terms-file`) and never committed. Each class
is first run against a control string that must match, so a broken pattern
cannot pass silently.

Undeclared relaxations cannot be caught completely. The relaxation checker
also flags, as a warning, lower-layer text that cites a rule or article ID
next to wording that lifts an obligation ("not required", "instead of",
"may skip") without a `relaxes:` entry. Review and behavior fixtures carry
the rest. The checker runs where the lower layer lives: in the compiler for
the overlay, and in each repo's own CI (against a pinned core version) for
that repo's `AGENTS.md`.

A check that the output contains a sentence proves the sentence is there. It
does not prove an agent follows it; only behavior fixtures and real runs say
anything about that, and even they only sample.

## Fixtures

```text
fixtures/static/happy/<name>/
fixtures/static/redline/<name>/
fixtures/behavior/happy/<name>/     stage C
fixtures/behavior/redline/<name>/   stage C
```

Each fixture directory holds a `fixture.md` (front matter `binds: [C2.3,
L1]`, `kind: happy` or `redline`, and a short statement of what it shows),
its inputs and its expected result.

- Static fixtures are deterministic: an input overlay or a lower-layer file,
  and an assertion on the compiler or a checker (builds; fails with this
  error; this declaration is rejected). They run in every CI run.
- Behavior fixtures (later) give a real agent a scenario and judge what it
  did. They record the harness, model and settings, the number of runs and
  how each failed; one pass is never reported as a guarantee.
- Happy-path and redline fixtures stay apart, so a fix for a safety case that
  breaks ordinary work shows up as a failing happy fixture.
- Every fixture binds to at least one constitution or rule ID, and the
  checker rejects a fixture whose IDs do not resolve.
- Before a release, CI runs every fixture, as C5.5 requires: all happy and all redline, static
  and (once they exist) behavior.
- Fixtures follow the constitution like any other file: made-up data only
  (`example.test` domains, documentation address ranges, fictional names), no
  real credentials, no private terms. The private-term scan covers
  `fixtures/`.

## Load verification

Compiling the file does not show that an agent read it. The overlay's apply
step checks each harness with its own read-only entry point (as of
2026-10-12: Codex `debug prompt-input`, Grok `inspect`, and Claude Code's
`InstructionsLoaded` hook):

- Positive arm: the manifest digest from the output header appears in what
  each harness actually loaded.
- Control arm: a string that must not appear does not: the statement of a
  `draft` rule that is present in the sources but excluded from the build.

If the positive arm passes and the control arm also finds its string, the
check cannot tell a correct load from a load of the wrong file, and it fails.
The core ships the check; it runs on the private side because it needs the
installed harnesses.

## Feedback loop: reserved places

Not built now. The shape, so that later work has somewhere to go:

```text
event -> proposal -> rule revision, case, or amendment -> release
```

- Events come from where work already leaves records: worker reports,
  review findings, failed checks, a person's corrections and answers. The
  core does not store events; an event source only has to deliver a short
  record (what happened, which build and rule IDs were in force, evidence
  reference, first classification). "No rule change needed" is a valid
  outcome.
- Cases (`cases/K<n>.md`, stage C): the facts, the build in force, the rule
  IDs applied, the decision and its status. A case explains a rule; it never
  grants a permission. A case that would change a right, a duty or a
  prohibition becomes a rule revision; one that would relax a boundary item
  becomes an amendment proposal.
- Middle ground (C4.3): when an agent is unsure and asks a person, the
  answer is written as a case candidate on the private side, with its
  source. When the same kind of answer recurs, it becomes a rule proposal.
  Tightening the boundary (a new item for C4.1, moving an item from C4.2 to
  C4.1) can go through the rule-library path of C7; relaxing it goes only
  through amendment (C9).
- Rule proposals are pull requests that change `rules/`, adopted under C7,
  each answering what problem, what evidence, why a rule and not a tool fix,
  which IDs, what authority, what could break, how to test and roll back.
  The priority classes of C7.4 decide how fast a proposal is handled.
- Amendment proposals are pull requests that change `constitution/`; they
  take effect only under C9.
- Finding the next work and verifying it (stage D): the interface is two
  record shapes, kept independent of any tracker. A work item: what, why,
  acceptance criteria, stop conditions. An acceptance record: each check run,
  at which commit, the output line that shows the result, and each check not
  run with the reason. Whatever finds work and whatever verifies it exchanges
  these; the core defines them in `schema/` when stage D starts.

## Releases and versions

- Versions are tags `v<major>.<minor>.<patch>` on `main`. A constitution
  amendment or a rule change that alters behavior is at least a minor bump;
  a change to tools that alters output is at least a minor bump; the rest is
  a patch.
- A release tag goes only on a `main` commit whose push-triggered `check` run
  succeeded on its first attempt. A rerun that turned green does not
  qualify; tag a later commit that passed the first time.
- Pushing a release tag makes a version available, so the boundary list of
  C4.1 decides whether an agent may push it or must ask a person.
- The overlay pins a tag and its commit SHA and refuses to build if they
  disagree. It moves to a new version by a change in the overlay, reviewed
  there.

## Relation to fleet

The toolchain is plain commands over files: it reads sources and an overlay,
writes an output and a manifest, and exits with a status. It does not call
fleet, read fleet's state or depend on how fleet starts agents. Fleet (and
any other orchestrator) consumes the output the way any harness does. The
only fleet-specific file is `.fleet/config.json`, which tells fleet where
this repo's issues live.

## Migrating from the current hand-written guide

Method and order only; the per-rule mapping is its own piece of work.

1. The hand-written guide stays in force until cutover. Nothing is deleted
   early.
2. The constitution is approved first, because every rule needs an
   `authority` to point at.
3. Classify each current rule into one destination: constitution, general
   rule, private rule, parameter (the value moves to `params.toml`, the rule
   keeps a placeholder), procedure (moves into a skill), provenance (moves to
   the private casebook), or removal.
4. Build the compiler and the schema and reference checkers, then port rules
   in batches, compiling after each batch.
5. Compare the compiled output with the hand-written guide, section by
   section, and record every intended difference.
6. Each existing repo-level override either becomes a `relaxes: L<n>`
   declaration or is removed; a person decides which.
7. Cutover: the overlay's apply step installs the compiled file in place of
   the hand-written one; load verification passes; the hand-written source
   is retired.
