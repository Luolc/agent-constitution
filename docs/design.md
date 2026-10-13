# agent-constitution design

Status: draft. This is the one design document of the repo. The constitution
itself is built from `constitution/index.toml`, `constitution/articles/` and
`constitution/sections/` into [English](../constitution/constitution.en.md)
and [Chinese](../constitution/constitution.zh.md); this document cites it
only by ID (see Source formats, IDs) and never restates it.

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
| General rules, a default set each deployment chooses from | Private rules: rules that only make sense on that user's stack |
| General skills | Private skills (for example the orchestra skill) |
| Compiler, checkers, private-term scanner | Provenance: the casebook of incidents and decisions, with dates and sources |
| A fictional example overlay and its golden output | The private term list the scanner reads at run time |
| Static fixtures | The apply step that runs the compiler and installs the output |

The overlay never changes or relaxes the constitution. It chooses which
general rules it uses (`20261012-rules-refine`; see Choosing rules), adds
private rules and fills
in parameters. It never edits core files; it pins a core version and builds
against it.

## Language

The constitution and the general rules are kept in English and Chinese.
Every other file here (the skills, the tools, this document) is in English.

Why two languages: the person who approves the constitution and the rules
reads Chinese much faster than English, and that review is the one place
where a person's attention matters most. A Chinese version also helps
readers of the public repo. English stays the default: it is what a public
reader can be assumed to read, and it is the first language in
`constitution/index.toml`.

Why the rest stays English: skills are installed as they are and tools are
code; one language there means one text to keep correct. The compiled
instruction file is built in one language, which the overlay selects, so an
agent reads one text. A rule is cited by ID from repo files, reviews and
tickets in either language; the ID, not the wording, carries the reference.

Both languages live in the same source file, as two keys of the same TOML
table (see Source formats), so one change edits both and a reviewer sees
them side by side.

The standing of the two texts of the constitution is set by the
constitution itself (`20261012-amendment-languages`,
`20261012-equal-languages`), as drafted; it stays open until a person
approves that text. The rules follow the same terms. What this repo adds:

- The checker only proves structural alignment (every text table has every
  language; the languages define the same IDs in the same order and cite
  the same IDs). There is no check, based on the git diff,
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
  index.toml              status, revision, languages, title, front    A
  articles/<id>.toml      one article per file: number, title, intro   A
  sections/<id>.toml      one section per file: its article, number,
                          status, text in every language, list items   A
  constitution.en.md      generated: the constitution in English       A
  constitution.zh.md      generated: the constitution in Chinese       A
docs/
  design.md               this document                                A
rules/
  <id>.toml               one general rule per file, text in every
                          language                                     A
skills/
  <name>/SKILL.md         general skills, installed as is              B
schema/                   schemas for the rules, fixtures and the
                          overlay manifest                             B
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
cases/<id>.md             decided cases                                C
proposals/                reserved for the adoption pipeline           D
.github/workflows/        CI                                           A
.pre-commit-config.yaml   commit hooks                                 A
.fleet/config.json        issue-tracker metadata for fleet; the
                          toolchain never reads it                     A
```

Two stage B pieces start early. `tools/` starts in stage A with the
constitution build: the constitution is itself compiled from per-article
and per-section sources (see Source formats). `rules/` starts in stage A too,
holding the
concrete practices taken out of the constitution. Compiling and checking the
rules, and building the instruction file, stay in stage B.

Stage B, in the order to build it: `schema/` and the schema checker,
the rest of `rules/` (filled by migration), the compiler, `examples/`
with the drift check, the reference and relaxation checkers, the
private-term scanner, `fixtures/static/` with the fixture gate, then load
verification. Each step is usable on its own.

What `20261012-protection-scope` protects is the text of the constitution
(`20261012-protected-text`); in the core that is the sources under
`constitution/` (`index.toml`, `articles/` and `sections/`) and the wording
of the files built from them. A change to the text needs a person's
approval under `20261012-amendment-procedure` (see Approving an amendment).
Everything else is an ordinary change under `20261012-rule-evolution`
(`20261012-ordinary-tooling`), reviewed and tested like any other:
`tools/` and `schema/` (compiler, checkers, the fixture-gate runner),
`.github/workflows/`, `fixtures/` and `examples/golden/`. A person's
attention goes to what the constitution says; the tools are kept honest by
their unit tests and happy-path end-to-end tests, not by a person reading
them. The two generated files change only by rebuilding. A change to the
build script that alters only their numbering or layout is ordinary, and its
pull request shows their diff so a person can glance at the format; one that
alters their wording changes the text and is an amendment
(`20261012-protected-text`).

## Source formats

All sources are TOML, read with the standard library's `tomllib`. Every
text a reader sees is a table whose keys are the language codes listed in
`constitution/index.toml` and whose values are the text in that language,
so one file holds all languages of one provision side by side. Multi-line
text is a literal string (`'''...'''`); the build drops the newline after
the opening quotes and before the closing ones.

### IDs

Every provision of the constitution (article, section, list item) and
every rule has an ID of the form `<YYYYMMDD>-<words>`: the date it was
first written into the repo, a hyphen, and one or more lowercase words
joined by hyphens, for example `20261012-credential-storage`. The format
is `^[0-9]{8}-[a-z0-9]+(-[a-z0-9]+)*$` with a real date. There is no
prefix for the article or the kind of provision: the ID is a key, like a
BibTeX key, chosen by a person once and never changed. The constitution
and the rules share one ID space, and the private overlay's rules are in
it too: an ID is unique across all of them, a deleted ID is never reused,
and a repealed provision keeps its file and its ID.

Text cites a provision with `{@<id>}` and a section's content defines a
list item with `{#<id>}`; only `{@` or `{#`, an ID of the form above and
`}` make a mark, all other braces are output as is. The build renders a
citation as the number the provision has in that language (below); the
generated text shows numbers only, never IDs. A long-lived record outside
the sources (a case, a pull request, a report) may cite a provision by
ID or by number: once the constitution is active a number never changes
(see Numbering).

### Constitution

The constitution is compiled from one file per article and one file per
section by `tools/build_constitution.py` (Python standard library only).
It writes `constitution/constitution.<lang>.md` for each language in
`index.toml`, committed and never edited by hand; `--check` writes
nothing and runs the checks under Checkers.

`constitution/index.toml`:

- `status`: `draft` until a person first approves the text, then
  `active`.
- `revision`: a string; drafts before the first approval are `0.<n>`, the
  first approved text is `1`, and each adopted amendment adds one. The
  build manifest records it.
- `languages`: the language codes, the first being the default. Each
  must have a rendering entry in the build script (one table,
  `LANGUAGES`, holds everything that differs by language: the file header,
  the shapes of headings, numbers and citations, the repealed note). Adding
  a language means adding one entry there and one key to every text table.
- `[title]`: the title, one line per language.
- `[front]`: the Markdown between the title and the first article (draft
  notice, `## Preamble` and its text), per language. It may cite; it
  defines nothing.

`constitution/articles/<id>.toml`: `id` (equal to the file name), `number`,
`[title]`, an optional `[intro]` (the paragraph between the heading and
the first section) and, only when the whole article is repealed,
`repealed = "YYYY-MM"`.

`constitution/sections/<id>.toml`: `id` (equal to the file name),
`article` (the ID of its article), `number` (within that article),
`status` (`draft`, `active` or `repealed`), `repealed = "YYYY-MM"` exactly
when the status is `repealed`, and `[content]`, the section's text per
language: the first line is the sentence after the section's number,
further lines, nested lists and blank lines follow as they are, and the
build indents them under the list item. A section whose text defines list
items has one `[[items]]` entry per item, with `id`, `number` and an
optional `repealed = "YYYY-MM"`; the `{#id}` definitions in the content,
each at the start of a list item, are the items in number order, in every
language. A repealed item's line in the content is `- {#id}` and nothing
else.

Section order inside an article, and article order, come from `number`
alone; there is no separate list. While `index.toml` is `draft`, every
section is `draft` and nothing is repealed; once it is `active`, every
section is `active` or `repealed`.

### Numbering

Numbers are written in the sources, not derived from position. An article
or section number is an integer with an optional lowercase letter suffix
(`2`, `2a`); a list item number is a capital-letter prefix and an integer
with the same optional suffix (`H2`, `H2a`). Sort order is by integer, then
suffix: `2` < `2a` < `2b` < `3`. Within one scope (all articles; the
sections of one article; all list items) numbers are unique; a draft need
not be contiguous.

From the first approved revision on, numbers follow
`20261012-stable-numbers`; the check `--check --base <git-ref>` enforces
it against a base commit (see Checkers, check 7). A repealed provision is
rendered as a one-line note in place of its text (table below). While the
constitution is a draft, numbers may change and the check only says that
it did not compare.

Rendering, with `20261012-authority` as the third article, its second
section and a list item `H2` as examples:

| Item | en | zh |
| --- | --- | --- |
| Article heading | `## Article III. <Title>` | `## 第三条 <标题>` |
| Section definition | `- §2 <content>` | `- 第二款 <content>` |
| List item definition | `  - H2 <text>` | `  - H2 <text>` |
| Citation of an article | `Art. III` | `第三条` |
| Citation of a section | `Art. III §2` | `第三条第二款` |
| Citation of a list item | `H2` | `H2` |
| Suffix | `Article IIIa`, `§2a`, `H2a` | `第三条之一`, `第二款之一` (a is 之一, b is 之二), `H2a` |
| Repealed section | `- §3 (repealed November 2026)` | `- 第三款 (2026 年 11 月废止)` |
| Repealed list item | `  - H3 (repealed November 2026)` | `  - H3 (2026 年 11 月废止)` |
| Repealed article | `## Article IV (repealed November 2026)`, nothing below it | `## 第四条 (2026 年 11 月废止)` |

Each output starts with a plain-text paragraph, in its own language:
generated file, do not edit it, edit the sources; and the `status`,
`revision` and language. Output is UTF-8 with LF line endings and a final
newline, with no timestamp; the same sources give byte-identical output.

### Rules

One rule per file, `rules/<id>.toml`, so that one proposal is one file and
a diff shows exactly one rule changing.

```toml
id = "20261012-credential-storage"
status = "draft"
scope = "all"
authority = ["20261012-safety", "20261012-sensitive-info"]

[title]
en = "<Title>"
zh = "<标题>"

[content]
en = '''
<statement: condition, action, exception, what to do when unsure>
'''
zh = '''
<正文>
'''
```

- `[title]` and `[content]` have one key per language of `index.toml`.
- The text cites the constitution or another rule with `{@<id>}`; how a
  citation renders in the compiled instruction file is set in stage B. A
  rule has no number and defines no `{#...}` IDs.
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
- `authority`: the constitution articles, sections or list items the rule
  rests on, by ID; none of them repealed. A rule with no authority does
  not compile.
- `adjustable` (optional): the values in this rule that a lower layer may
  change under `20261012-adjustable-rules`, each with its default, for
  example `adjustable = ["wait_hours=24"]`. A rule without it cannot be
  relaxed that way. These are distinct from local parameters (below): a
  local parameter is a value the private overlay fills in; an adjustable
  value is one a repo or a task may change.

More fields (`supersedes`, `verification`, `expires`) are added when a real
rule needs them, not before.

Private rules in the overlay have the same format and the same kind of ID
(the date they were written, then words). The compiler checks that the
union of core and overlay IDs is unique. `P0`–`P2` are the priority
classes of `20261012-response-priority`, not IDs.

### Choosing rules

The general rules are a default set, not obligations on every deployment
(`20261012-rules-refine`): one deployment may need a rule that another has
good reason to drop (a repo that stays private on a self-hosted server may
keep credentials in it on purpose). Open, for the person to decide. Recommended:

- A deployment uses every general rule by default. Its overlay leaves a rule
  out only by naming it, with a one-line reason, in `overlay.toml`
  (`exclude = ["20261012-credential-storage"]`). The build manifest lists
  what was left out.
- A repo inside a deployment departs from a rule only by a relaxation
  declaration (below), where the constitution permits it. It does not leave
  rules out of the compiled file, because one compiled file serves all of a
  deployment's repos.
- To change a rule rather than drop it, the overlay leaves it out and adds a
  private rule in its place.
- The constitution cannot be left out or relaxed this way.

Reasons: defaulting to all means a new rule reaches every deployment without
anyone remembering to list it, where an opt-in list would go stale as rules
are added. Leaving a rule out is not a relaxation (`20261012-rules-refine`),
but naming each one, with its reason, keeps it visible and puts everything a
deployment dropped in one place. Exclusion per deployment, not per repo, matches how the output is
built: one file per deployment.

### Relaxation declarations

A lower layer (a repo's `AGENTS.md`, or a task file) relaxes a rule only by
naming the constitution's permission for it:

```markdown
## Relaxations

- relaxes: 20261012-adjustable-rules
  target: 20261012-example-rule/wait_hours=4
  scope: <this repo, or this task>
  condition: <how this layer meets the condition that L item states>
```

One item per relaxation, with all four keys: the permission (an L item of
`20261012-l-list`, by ID), what it relaxes (a rule, a boundary item such as
`20261012-machines`, or under `20261012-adjustable-rules` a rule's
adjustable value and its new setting), where it applies, and how the
condition of that L item is met. A relaxation not declared this
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
rules/<id>.toml    private rules
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
   the overlay leaves out) grouped by the article of their first
   `authority` and ordered by ID inside a group.
   Parameters are substituted here.
4. Before each article and rule, emit a source comment holding its ID,
   such as `<!-- 20261012-authority -->`, so any paragraph of the output
   can be traced to its file.
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
| Constitution (`build_constitution.py --check`) | Each constitution and rule source has the format above; each ID is well formed and defined once; numbers are unique in their scope and the list items match their definitions; the languages of each file define the same IDs in the same order and cite the same IDs; every citation and authority resolves to an ID that is not repealed; the committed outputs equal a fresh build; with `--base`, nothing numbered in an active base has disappeared or changed number | That the two languages mean the same thing, or that a change to one language was carried to the other |
| Schema | Fixtures and the overlay manifest have the required keys and allowed values; IDs match their file names | That the text means what the fields claim |
| References | Every cited ID (a constitution provision, a rule, a case) exists and is not repealed; no source points to a file outside the core or the overlay | That the cited article actually supports the rule |
| Drift | `examples/golden/` equals what the compiler builds from `examples/overlay/` today | That a real overlay builds; only that the compiler did not change behavior unnoticed |
| Private-term scan | No hit from the generic classes (secret-store paths, private address ranges, home-directory paths, email addresses) in CI; no hit from the private term list when the overlay runs it | That nothing private is present: a term not on the list passes |
| Relaxations | Each `relaxes:` names an existing L item open to that layer, an existing target, a scope and a condition; under `20261012-adjustable-rules` the target rule lists the parameter as `adjustable` | That the condition is really met, or that no undeclared relaxation exists |
| Fixture gate | Every fixture binds to existing IDs and passes; redline fixtures are never skipped, except those of rules a deployment left out, which its manifest lists as not run | That agents behave as the fixtures expect outside these cases |

The constitution check runs its checks in this order and stops after the
first one that finds errors, printing all of that one's errors:

1. Format: every TOML file parses; it has exactly the required keys with
   the right types; every text table has exactly the languages of
   `index.toml`; `id` equals the file name; `status`, `number` and
   `repealed` are well formed and agree with each other and with the status
   of `index.toml`.
2. IDs: each has the format with a real date and is defined once across
   the constitution and the rules.
3. Structure: each section's `article` exists; numbers are unique in their
   scope; a repealed article has only repealed sections; `{#id}`
   definitions stand only at the start of a list item of a section's
   content and are exactly that section's `[[items]]` in number order.
4. Languages: in each text table, every language defines the same IDs in
   the same order and cites the same set of IDs as the default language.
5. References: each `{@id}` and each `authority` entry resolves; a repealed
   target is an error; the constitution cites only itself; `authority`
   names only constitution provisions.
6. A fresh build equals the committed `constitution.<lang>.md` files byte
   for byte (only under `--check`).
7. Numbering against a base commit (only under `--check --base <git-ref>`,
   read with `git show`): when the base's `index.toml` is `active`, every
   article, section and list item of the base still exists with the same
   number (a section also in the same article), what was repealed is still
   repealed, every rule file of the base still exists, and the status has
   not gone back to `draft`. A base that is `draft`, or has no TOML
   sources yet, skips the step with a printed note.

Each check has a test that makes it fail and a passing control; the
numbering check is tested on a temporary git repository
(`tools/test_build_constitution.py`). Checks 1 to 6 and the tests run as
commit hooks and in CI; check 7 runs in CI only, against the pull
request's base commit or, on a push to `main`, the commit before the push.

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

Each fixture directory holds a `fixture.md` (front matter `binds:
[20261012-asking-once, 20261012-adjustable-rules]`, `kind: happy` or
`redline`, and a short statement of what it shows),
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
- Before a release, every fixture runs, all happy and all redline, since
  `20261012-checks-everywhere` requires every check to pass before a
  release. Static fixtures run in this
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
  `20261012-amendments`) run for every deployment. Those for a default
  boundary item test the default; a project that overrides that item under
  `20261012-h-overrides` runs its own fixture
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
- Cases (`cases/<id>.md`, stage C): the facts, the build in force, the rule
  IDs applied, the decision and its status. A case explains a rule; it never
  grants a permission. A case that would change a right, a duty or a
  prohibition becomes a rule revision; one that would relax a boundary item
  for every project becomes an amendment proposal (for one project, a
  `20261012-h-overrides` declaration does it).
- Middle ground (`20261012-middle-ground`): when an agent is unsure and
  asks a person, the
  answer is written as a case candidate on the private side, with its
  source. When the same kind of answer recurs, it becomes a rule proposal.
  Which boundary changes the rule library may make, and which need an
  amendment, is `20261012-list-defaults`.
- Rule proposals are pull requests that change `rules/`, adopted under
  `20261012-rule-evolution`,
  each answering what problem, what evidence, why a rule and not a tool fix,
  which IDs, what authority, what could break, how to test and roll back.
  The priority classes of `20261012-response-priority` decide how fast a
  proposal is handled.
- Amendment proposals are pull requests that change `constitution/`; they
  take effect only under `20261012-amendment-procedure`.
- Finding the next work and verifying it (stage D): the interface is two
  record shapes, kept independent of any tracker. A work item: what, why,
  acceptance criteria, stop conditions. An acceptance record: each check run,
  at which commit, the output line that shows the result, and each check not
  run with the reason. Whatever finds work and whatever verifies it exchanges
  these; the core defines them in `schema/` when stage D starts.

## Approving an amendment

Built so that the person spends as little time as possible on it, which is
what `20261012-purpose` asks for. The person reads what the constitution
says and nothing
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
  default, so one answer settles them (`20261012-asking-once`).
- The person's yes names the head commit it approves, and the merge lands
  exactly that commit (`20261012-independent-review`).
- `20261012-proposing-amendments` asks for the generated diff in the
  language the approving person reads, and `20261012-amendment-languages`
  for both languages to change together; the steps above are how this repo
  meets them. If the head changes afterwards, the person is
  asked again with only the diff since the approved head.

## Releases and versions

- Versions are tags `v<major>.<minor>.<patch>` on `main`. A constitution
  amendment or a rule change that alters behavior is at least a minor bump;
  a change to tools that alters output is at least a minor bump; the rest is
  a patch.
- A release tag goes only on a `main` commit whose push-triggered `check` run
  succeeded on its first attempt. A rerun that turned green does not
  qualify; tag a later commit that passed the first time.
- Pushing a release tag makes a version available. By default that is
  `20261012-releases` and needs a person's yes; a project may move it under
  `20261012-h-overrides`.
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
6. Each existing repo-level override either becomes a `relaxes:`
   declaration or is removed; a person decides which.
7. Cutover: the overlay's apply step installs the compiled file in place of
   the hand-written one; load verification passes; the hand-written source
   is retired.
