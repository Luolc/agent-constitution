# agent-constitution design

Status: draft. This is the one design document of the repo. The constitution
itself is built from `constitution/index.md` and `constitution/articles/`
into [English](../constitution/constitution.en.md) and
[Chinese](../constitution/constitution.zh.md); this document cites it only by
ID (`C<n>`, `C<n>.<k>`, `L<n>`, `H<n>`, `A<n>`) and never restates it.

## Goals and non-goals

Goals:

- Hold the constitution: the few stable principles and the governance rules
  (what binds whom, what only a person may decide, how rules and the
  constitution change). It names no concrete practice: no command, storage
  place or tool.
- Hold the general rule library and general skills: the concrete practices
  that refine the constitution, written without one user's machines,
  accounts or history. The rules are a default set; each deployment chooses
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
- Depending on any orchestrator. See "Relation to fleet".

## Two layers

| Public core (this repo) | Private overlay (a separate private repo) |
| --- | --- |
| The constitution | Local parameters: the values the rules leave open (machines, model names, secret-store layout, lists of repos) |
| General rules (`R<n>`), a default set each deployment chooses from | Private rules (`O<n>`): rules that only make sense on that user's stack |
| General skills | Private skills (for example the orchestra skill) |
| Compiler, checkers, private-term scanner | Provenance: the casebook of incidents and decisions, with dates and sources |
| A fictional example overlay and its golden output | The private term list the scanner reads at run time |
| Static fixtures | The apply step that runs the compiler and installs the output |

The overlay never changes or relaxes the constitution. It chooses which
general rules it uses (C6.7; see Choosing rules), adds private rules and fills
in parameters. It never edits core files; it pins a core version and builds
against it.

## Language

The constitution and the general rules are kept in English and Chinese.
Every other file here (the skills, the tools, this document) is in English.

Why two languages: the person who approves the constitution and the rules
reads Chinese much faster than English, and that review is the one place
where a person's attention matters most. A Chinese version also helps
readers of the public repo. English stays the default: it is what a public
reader can be assumed to read, and it is the first block of every source
file.

Why the rest stays English: skills are installed as they are and tools are
code; one language there means one text to keep correct. The compiled
instruction file is built in one language, which the overlay selects, so an
agent reads one text. A rule is cited by ID from repo files, reviews and
tickets in either language; the ID, not the wording, carries the reference.

Both languages live in the same source file (see Source formats), so one
change edits both and a reviewer sees them side by side.

The standing of the two texts of the constitution is set by the
constitution itself (C9.5, C11.3), as drafted; it stays open until a person
approves that text. The rules follow the same terms. What this repo adds:

- The checker only proves structural alignment (the same IDs defined in the
  same order, the same IDs cited). There is no check, based on the git diff,
  that a change touching one language also touches the other: fixing a typo
  in one language only is a legitimate change, and such a check would block
  it or teach people to make empty edits. Review covers meaning.

## File structure

Stages follow the plan this repo was started from: A is the constitution
and this design, B is structured sources and deterministic compilation, C is
feedback and proposals (observation only), D is scoped automatic adoption.
Directories for C and D are reserved, not created.

```text
README.md
AGENTS.md                 repo rules for agents working here           A
constitution/
  index.md                front matter, title and preamble             A
  articles/C<n>.md        one article per file, English and Chinese    A
  constitution.en.md      generated: the constitution in English       A
  constitution.zh.md      generated: the constitution in Chinese       A
docs/
  design.md               this document                                A
rules/
  R<n>.md                 one general rule per file, English and
                          Chinese                                      A
skills/
  <name>/SKILL.md         general skills, installed as is              B
schema/                   front matter schemas for constitution,
                          rules, fixtures, the overlay manifest        B
tools/
  build_constitution.py   builds and checks the constitution           A
  test_build_constitution.py   its tests                               A
  <others>                compiler, checkers, private-term scanner     B
examples/
  overlay/                a fictional private overlay                  B
  golden/                 its compiled output and build manifest       B
fixtures/
  static/                 deterministic fixtures, happy and redline    B
  behavior/               real-agent scenarios                         C
cases/                    K<n>: decided cases                          C
proposals/                reserved for the adoption pipeline           D
.github/workflows/        CI                                           A
.pre-commit-config.yaml   commit hooks                                 A
.fleet/config.json        issue-tracker metadata for fleet; the
                          toolchain never reads it                     A
```

Two stage B pieces start early. `tools/` starts in stage A with the
constitution build: the constitution is itself compiled from per-article
sources (see Source formats). `rules/` starts in stage A too, holding the
concrete practices taken out of the constitution. Compiling and checking the
rules, and building the instruction file, stay in stage B.

Stage B, in the order to build it: `schema/` and the schema checker,
the rest of `rules/` (filled by migration), the compiler, `examples/`
with the drift check, the reference and relaxation checkers, the
private-term scanner, `fixtures/static/` with the fixture gate, then load
verification. Each step is usable on its own.

What C8 protects is the text of the constitution (C8.1); in the core that
is the sources under `constitution/` (`index.md` and `articles/`) and the
wording of the two files built from them. A change to the text needs a
person's approval under C9 (see Approving an amendment). Everything else is
an ordinary change under C7 (C8.2), reviewed and tested like any other:
`tools/` and `schema/` (compiler, checkers, the fixture-gate runner),
`.github/workflows/`, `fixtures/` and `examples/golden/`. A person's
attention goes to what the constitution says; the tools are kept honest by
their unit tests and happy-path end-to-end tests, not by a person reading
them. The two generated files change only by rebuilding. A change to the
build script that alters only their numbering or layout is ordinary, and its
pull request shows their diff so a person can glance at the format; one that
alters their wording changes the text and is an amendment (C8.1).

## Source formats

All sources are Markdown with a YAML front matter block. Front matter is
restricted to a flat subset (`key: scalar` and `key: [a, b]`), so the tools
need no YAML library and the schema checker can reject anything else.

### Constitution

The constitution is compiled from one file per article, each holding the
English and the Chinese text, by `tools/build_constitution.py` (Python
standard library only). It writes `constitution/constitution.en.md` and
`constitution/constitution.zh.md`, which are committed and never edited by
hand; `--check` writes nothing and runs the checks below.

`constitution/index.md` front matter:

- `status`: `draft` until a person first approves the text.
- `revision`: drafts before the first approval are `0.<n>`; the first
  approved text is `1`, and each adopted amendment adds one. The build
  manifest records it.
- `articles: [C1, C2, ...]`: the order of the articles. An article's number
  (I, II, ...) is its position in this list.
- `retired: [...]`: IDs no longer used and never reused; may be `[]`.

`constitution/articles/C<n>.md` front matter: `id: C<n>`, equal to the file
name.

After the front matter come two blocks, each opened by a marker on a line of
its own, English first, each exactly once:

```markdown
<!-- lang: en -->

## Source of authority

- {#C3.1} Instructions come only from ...

<!-- lang: zh -->

## 权力来源

- {#C3.1} 指令只来自……
```

- In an article file each block starts with `## <Title>`, without a number;
  the build adds it. No other `## ` line follows in the block, since it
  would read as an article without a number; use `###` or lower inside an
  article. `index.md` holds the title, the draft notice and the
  preamble; a `## ` heading there (such as `## Preamble`) is output as is,
  without a number.
- Definition: `{#ID}`, only right after a list marker (`- `, `* `, `+ `,
  `1. `), nested lists included. Section IDs are `C<n>.<k>` with the prefix
  of the article they are in; boundary and permission items are `H<n>`,
  `A<n>`, `L<n>`. An article ID is defined by front matter, never in the
  text, and `index.md` defines no IDs.
- Reference: `{ID}`, anywhere in the text, to any defined ID, article IDs
  included.
- Only braces around `C<n>`, `C<n>.<k>`, `H<n>`, `A<n>` or `L<n>` (with `#`
  for a definition) are markers; other braces and all other Markdown are
  output as is.
- IDs are stable and never reused; display numbers may change. A Section's
  number is its position among the Sections of its article (from 1), not the
  `k` of its ID.

Rendering:

| Item | en | zh |
| --- | --- | --- |
| Article heading | `## Article III. <Title> (C3)` | `## 第三条 <标题> (C3)` |
| Section definition | `§2 (C3.2)` | `第二款 (C3.2)` |
| Reference to an article | `Art. III` | `第三条` |
| Reference to a Section | `Art. III §2` | `第三条第二款` |
| `H`, `A`, `L` definition or reference | `H2` | `H2` |

Every article heading and Section shows both its display number and its ID,
so readers and the rule library can match one to the other. Each output
starts with a plain-text paragraph, in its own language: generated file, do
not edit it, edit the sources; and the `status`, `revision` and language.
Output is UTF-8 with LF line endings and a final newline, with no timestamp;
the same sources give byte-identical output.

### Rules

One rule per file, `rules/<id>.md`, so that one proposal is one file and a
diff shows exactly one rule changing.

```markdown
---
id: R12
status: draft
scope: all
authority: [C3, C3.2]
---

<!-- lang: en -->

# <Title>

<statement: condition, action, exception, what to do when unsure>

<!-- lang: zh -->

# <标题>

<正文>
```

- Two language blocks, with the same markers and order as the constitution
  sources; each block starts with `# <Title>`.
- The text cites the constitution or another rule with the same `{ID}`
  marker (`{C1.1}`, `{R12}`, `{H2}`); how a reference renders is set in
  stage B. A rule defines no `{#...}` IDs.
- A rule about messages or notices says whom it covers: agent to agent,
  agent to person, or both.

- `id`: stable, also the file name. Renaming the title never changes it.
- `status`: `draft`, `active` or `repealed`. Only `active` is compiled. A
  repealed rule keeps its file and its ID. While the constitution is a
  draft, every rule is `draft` too.
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

IDs: rules migrated from the current hand-written guide keep their `R<n>`,
whichever side they move to, so the references already in repo files,
reviews and the casebook keep their meaning. New general rules continue the
`R` sequence in this repo, above the highest migrated number. New rules that
exist only in the private overlay use `O<n>`, so the two sides never compete
for the next number (`P0`–`P2` are taken by the priority classes of C7.4).
Retired numbers stay retired. The compiler checks that the union of core and
overlay IDs is unique.

### Choosing rules

The general rules are a default set, not obligations on every deployment
(C6.7): one deployment may need a rule that another has good reason to drop (a
repo that stays private on a self-hosted server may keep credentials in it on
purpose). Open, for the person to decide. Recommended:

- A deployment uses every general rule by default. Its overlay leaves a rule
  out only by naming it, with a one-line reason, in `overlay.toml`
  (`exclude: [R12]`). The build manifest lists what was left out.
- A repo inside a deployment departs from a rule only by a relaxation
  declaration (below), where the constitution permits it. It does not leave
  rules out of the compiled file, because one compiled file serves all of a
  deployment's repos.
- To change a rule rather than drop it, the overlay leaves it out and adds a
  private rule in its place.
- The constitution cannot be left out or relaxed this way.

Reasons: defaulting to all means a new rule reaches every deployment without
anyone remembering to list it, where an opt-in list would go stale as rules
are added. Leaving a rule out is not a relaxation (C6.7), but naming each one,
with its reason, keeps it visible and puts everything a deployment dropped in
one place. Exclusion per deployment, not per repo, matches how the output is
built: one file per deployment.

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
and how the condition of that `L<n>` is met. A relaxation not declared this
way has no effect, whatever else the file says. The private overlay declares
no relaxations: it leaves rules out instead (see Choosing rules).

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
SHA) and one overlay directory. The overlay selects the language. The
compiler takes the constitution from the generated file in that language
(it does not read the constitution sources) and each rule's block in that
language. Output: one file, installed by the overlay as
the user-level instruction file that all three harnesses read through
symlinks (`~/.agents/AGENTS.md` in the current setup), plus a build
manifest. Nothing else.

Overlay layout:

```text
overlay.toml       core version, language, rules left out, output path,
                   size budget
params.toml        parameter values
rules/O<n>.md      private rules
skills/<name>/     private skills
```

The compiler is a deterministic concatenation, no model involved:

1. Validate every input (schema, references); stop on the first class of
   error, print all errors of that class.
2. Emit a header in plain text, not an HTML comment: generated file, do not
   edit, edit the sources instead; the core version, the constitution
   revision, and the input digest (the SHA-256 of the sorted list of
   per-input hashes).
3. Emit the constitution, then active rules (core and overlay, minus those
   the overlay leaves out) grouped by their first `authority` article and
   ordered by ID inside a group.
   Parameters are substituted here.
4. Before each article and rule, emit a source comment such as `<!-- C3 -->`
   or `<!-- R12 -->`, so any paragraph of the output can be traced to its
   file.
5. Measure the result against the budget in `overlay.toml` (the core sets a
   ceiling the overlay cannot raise). Over budget fails the build; nothing is
   silently dropped.
6. Write the output and the manifest: the SHA-256 of each input file (sorted
   by path), core version and commit, compiler version (the core version,
   since the compiler lives here), constitution revision, language, the rules
   left out, input digest, output digest and size. No timestamps; the same
   inputs give byte-identical output.

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
| Constitution (`build_constitution.py --check`) | Each source file has the format above; `articles` and the files match; each ID is defined once, in its own article, and is not retired; the English and Chinese blocks of each file define the same IDs in the same order and cite the same IDs; every reference resolves to an ID that is not retired; the committed outputs equal a fresh build | That the two languages mean the same thing, or that a change to one language was carried to the other |
| Schema | Front matter has the required keys, allowed values and the flat subset; IDs match their file names | That the text means what the fields claim |
| References | Every cited ID (`C`, `L`, `H`, `A`, `R`, `O`, `K`) exists and is not repealed; no source points to a file outside the core or the overlay | That the cited article actually supports the rule |
| Drift | `examples/golden/` equals what the compiler builds from `examples/overlay/` today | That a real overlay builds; only that the compiler did not change behavior unnoticed |
| Private-term scan | No hit from the generic classes (secret-store paths, private address ranges, home-directory paths, email addresses) in CI; no hit from the private term list when the overlay runs it | That nothing private is present: a term not on the list passes |
| Relaxations | Each `relaxes:` names an existing `L<n>` open to that layer, an existing target, a scope and a condition; under L2 the target rule lists the parameter as `adjustable` | That the condition is really met, or that no undeclared relaxation exists |
| Fixture gate | Every fixture binds to existing IDs and passes; redline fixtures are never skipped, except those of rules a deployment left out, which its manifest lists as not run | That agents behave as the fixtures expect outside these cases |

The constitution check runs its checks in this order and stops after the
first one that finds errors, printing all of that one's errors:

1. Format: front matter in the flat subset with exactly the required keys;
   the en and zh blocks, each once, en first; each article block starts
   with one `## ` title and has no other; definitions only at the start of a
   list item.
2. `articles` in `index.md` and the files in `articles/` match one to one,
   and each file name equals its `id`.
3. Each ID is defined once; a Section ID's prefix is its article; no
   defined ID is retired; `index.md` defines none.
4. In each file, the en and zh blocks define the same IDs in the same order
   and cite the same set of IDs.
5. Each reference points to a defined ID; a reference to a retired ID is an
   error.
6. A fresh build equals the committed `constitution.en.md` and
   `constitution.zh.md` byte for byte (only under `--check`).

Each check has a test that makes it fail and a passing control
(`tools/test_build_constitution.py`). The check and the tests run as
commit hooks and in CI.

The private-term scan prints only position and class, never the matched
text. In CI it runs the generic classes only; the term list is injected at
run time on the private side (`--terms-file`) and never committed. Each class
is first run against a control string that must match, so a broken pattern
cannot pass silently.

Undeclared relaxations cannot be caught completely. The relaxation checker
also flags, as a warning, lower-layer text that cites a rule or article ID
next to wording that lifts an obligation ("not required", "instead of",
"may skip") without a `relaxes:` entry. Review and behavior fixtures carry
the rest. The checker runs in each repo's own CI, against a pinned core
version, over that repo's `AGENTS.md`. The compiler runs the same warning over
the overlay's private rules: a private rule that lifts an obligation of a
general rule in use should leave that rule out by name instead.

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
L2]`, `kind: happy` or `redline`, and a short statement of what it shows),
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
- Before a release, every fixture runs, all happy and all redline, since C5.5
  requires every check to pass before a release. Static fixtures run in this
  repo's CI. Behavior fixtures (once they exist) need installed harnesses and
  model credentials, which this repo's CI does not have; they run on the
  private side, and a release needs the record of that run at the commit being
  tagged.
- Fixtures follow the constitution like any other file: made-up data only
  (`example.test` domains, documentation address ranges, fictional names), no
  real credentials, no private terms. The private-term scan covers
  `fixtures/`.

Open, for the person to decide: where red-line fixtures live. Recommended:
with the text they test, bound to its ID.

- Constitution: its red-line fixtures live in the core. Those for what no
  project may change (for example, that an amendment goes through a person,
  H8) run for every deployment. Those for a default boundary item test the
  default; a project that overrides that item under L3 runs its own fixture
  for the override in place of the core's, as with a left-out rule.
- General rules: each rule's red-line fixtures live in the core, bound to
  that rule's ID. The core's CI runs all of them. A deployment runs those of
  the rules it uses; the fixtures of a rule it left out are not run there,
  and its manifest lists them as not run, never as passed.
- Private rules and a repo's relaxations: their fixtures live on that side
  (the overlay, the repo) and run in its CI against the pinned core
  version, the way the relaxation checker already runs there.

Reasons: a fixture changes together with the text it tests, so it belongs
where that text is, and binding it to the ID means that leaving a rule out
leaves its fixtures out too, with no second list to keep. The concrete hard
rules (credentials, for example) are now optional rules, not constitution,
so their fixtures cannot sit at the constitution level and run everywhere.
A copy of the core's fixtures in every project would drift and test text
the project does not own.

## Load verification

Compiling the file does not show that an agent read it. The overlay's apply
step checks each harness with its own read-only entry point (as of
2026-10-12: Codex `debug prompt-input`, Grok `inspect`, and Claude Code's
`InstructionsLoaded` hook):

- Positive arm: the input digest from the plain-text header appears in what
  each harness actually loaded. (Claude Code strips block-level HTML
  comments before injecting the file, so the digest must not be in one.)
- Control arm: a string that must not appear does not. The core ships a
  fixed `draft` canary rule whose statement is a unique string; it is in
  every build's sources and never in any output.
- An entry point counts only if it returns the loaded content, not just the
  file paths; which ones do is confirmed per harness when stage B builds
  this check.

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
  for every project becomes an amendment proposal (for one project, an L3
  declaration does it).
- Middle ground (C4.3): when an agent is unsure and asks a person, the
  answer is written as a case candidate on the private side, with its
  source. When the same kind of answer recurs, it becomes a rule proposal.
  Which boundary changes the rule library may make, and which need an
  amendment, is C4.4.
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

## Approving an amendment

Built so that the person spends as little time as possible on it, which is
what C2 asks for. The person reads what the constitution says and nothing
else: tools, checks and fixtures are ordinary changes (see File structure).

- An amendment pull request changes only `constitution/`: the sources and
  the two generated files rebuilt from them.
- Its description opens with the diff of the generated text in the
  language the person reads, then the same diff in the other language,
  collapsed. The generated diff is what agents will read,
  numbers included; the person does not need to read the sources.
- Agent review is finished before the person is asked: CI's constitution
  check is green, and the reviewer has checked that the two languages say
  the same thing. A person approving the change approves both texts on the
  strength of that review.
- The description ends with the open questions, each with a recommended
  default, so one answer settles them (C2.3).
- The person's yes names the head commit it approves, and the merge lands
  exactly that commit (C5.3).
- C9.1 asks for the generated diff in the language the approving person
  reads, and C9.5 for both languages to change together; the steps above are
  how this repo meets them. If the head changes afterwards, the person is
  asked again with only the diff since the approved head.

## Releases and versions

- Versions are tags `v<major>.<minor>.<patch>` on `main`. A constitution
  amendment or a rule change that alters behavior is at least a minor bump;
  a change to tools that alters output is at least a minor bump; the rest is
  a patch.
- A release tag goes only on a `main` commit whose push-triggered `check` run
  succeeded on its first attempt. A rerun that turned green does not
  qualify; tag a later commit that passed the first time.
- Pushing a release tag makes a version available. By default that is H4
  and needs a person's yes; a project may move it under L3.
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
