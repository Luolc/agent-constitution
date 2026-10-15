# Checkers, fixtures and feedback

## Checkers

Every checker is a command that exits non-zero on failure and prints file and
position. What each one proves, and what it does not:

| Checker | Proves | Does not prove |
| --- | --- | --- |
| Constitution (`statute constitution build --check`) | Each constitution source has the format and the name in [Source formats](source-formats.md) and [The constitution](constitution.md); each ID is well formed and unique in its namespace; numbers are unique in their scope; the languages of each text cite the same IDs; every citation of the constitution resolves to a provision that is not repealed, the constitution citing only itself; the committed outputs equal a fresh build; with `--base`, the edition is an active base's or a later one, and in the base's edition nothing numbered in the base has disappeared, changed number or moved; with `--base` and whatever the base's status, within one edition, and from a draft base whatever edition each names, the revision is greater than the base's if the constitution text changed and equal to it if not | That the two languages mean the same thing, or that a change to one language was carried to the other |
| Rules (`statute validate rules`) | Each rule file has the format of [Rule file format](rule-format.md#rule-file-format), the tree the shape of [No rule under a rule](rule-config.md#no-rule-under-a-rule), every reference a target of the right kind, every translation the original's hash and citations, and the defaults resolve on their own; each rule's status agrees with the constitution's, and every citation of the constitution and every `authority` entry resolves to a provision that is not repealed; with `--base`, no node of the base is gone or no longer repealed or superseded (below) | That a rule says what it should, or that two rules that nobody marked as conflicting work together |
| Compiled output (`statute compile --check`) | `AGENTS.md`, `statute-checks.toml` and the skills' copies and links are what `statute compile` writes from `statute.toml` and `statute.lock` now, and no skill directory statute wrote is left that the rule set no longer needs; `statute.lock` is what `statute lock` would write now | Anything about the private output, which git ignores and the check neither compiles nor compares |
| Instruction-only changes (`check-instruction-changes --base`) | The files a change touches since its base are all instruction files or all other files, generated files not counted; a rename counts as its old and its new path | That the path table is complete: a file an agent reads as instructions but the table does not list passes as an other file. That a generated file changed only in rendering when a tools change regenerated it; the reviewer checks that |
| Release tag (`check-release-tag`, in `release.yml` before publishing) | The tag is `statute-v` followed by the version of `crates/statute/Cargo.toml`, and that version can be read; the tagged commit is `main` or an ancestor of it; that commit has exactly one run of `ci.yml` triggered by a push to `main`, and it completed with `success` on its first attempt | That the commit is the one a person approved for release, or that crates.io accepts the upload |
| Static site (`hugo-0.167.0 --panicOnWarning`, then `check-site-anchors`) | Hugo builds all pages without a warning, every relative link of the design to a design file leads to its page and none leads out of the repo; on every page each in-page link `#id` has an element with that id, each link to another page of the site (relative, or from the site's root when it starts with `/`) leads to a file of the site and, with a fragment, to an element with that id on that page, no id appears twice, and no list marker shows as literal text (`- §`, `- 第`) | That the pages look right, or that the fonts load |
| Chinese typography (`lint-zh-sources`; limae on every Markdown file of the repo) | Every `zh` text of the constitution and rule sources, both versions of the built Chinese constitution and the design's translation pass limae's default rules, a constitution citation judged as the Chinese text it renders to | That the Chinese reads naturally or says what the English says; the English text gets limae's rules too, which mostly find nothing there |
| Design translation (`check-design-translation`) | `docs/design/zh/` holds a file for each of `docs/design/` and no other; each starts with the SHA-256 of the English file as it is now, then its title, and has the English file's headings, in the same order and at the same levels, each with the English heading's anchor; every relative link of the design, in either language, leads to a file or directory of the repo | That the translation says what the English says, or that it was brought up to date when its hash line was |
| Python code (`ruff check`, `ruff format --check` and `pydoclint` as commit hooks; `basedpyright` in CI; all configured in `pyproject.toml`) | The scripts under `tools/` follow the configured lint rules and formatting, their docstrings name each argument and return value, and the type checker finds no error in standard mode | That the scripts do what their docstrings say; that is what the tests are for |
| Rust code (`cargo fmt --check` as a commit hook; `cargo clippy --all-targets -- -D warnings`, `cargo test` and `cargo publish --dry-run -p statute` in CI) | The workspace under `crates/` is formatted, clippy finds no warning, the tests pass, and `statute` packages and builds as crates.io would receive it | That the commands do what the design says where no test covers them, or that crates.io would accept the upload |
| Schema | Fixtures and the overlay manifest have the required keys and allowed values; IDs match their file names | That the text means what the fields claim |
| References | Every cited ID (a constitution provision, a rule, a case) exists and is not repealed; no source points to a file outside the core or the overlay | That the cited article actually supports the rule |
| Drift | `examples/golden/` equals what the compiler builds from `examples/overlay/` today | That a real overlay builds; only that the compiler did not change behavior unnoticed |
| Private-term scan | No hit from the generic classes (secret-store paths, private address ranges, home-directory paths, email addresses) in CI; no hit from the private term list when the overlay runs it | That nothing private is present: a term not on the list passes |
| Relaxations | Each `relaxes:` names an existing item of `relaxations` open to that rank, an existing target, a scope and a condition; under `relaxations.adjustable-rules` the target rule lists the parameter as `adjustable` | That the condition is really met, or that no undeclared relaxation exists |
| Fixture gate | Every fixture binds to existing IDs and passes; redline fixtures are never skipped, except those of rules a jurisdiction left out, which its manifest lists as not run | That agents behave as the fixtures expect outside these cases |

The constitution check runs its checks in this order and stops after the
first one that finds errors, printing all of that one's errors:

1. Format: every TOML file parses; it has exactly the required keys with
   the right types; every text table has exactly the languages of
   `index.toml`; `constitution/` has exactly one edition directory, a
   directory holding an `index.toml`; everything under its `articles/` is an
   article's directory, with an article's name and an `article.toml`, and
   everything else in an article's directory is a `.toml` file with a
   section's name; there is no `sections/` beside `articles/`; `id` equals the
   name without its number; `status`, item `number` and `repealed` are
   well formed and agree with each other and with the status of
   `index.toml`; `edition` is a real month `YYYY-MM`. Hidden names
   (starting with `.`, such as `.DS_Store` or an editor's swap file) are
   skipped.
2. IDs: each has the format; article and section IDs are unique in the
   constitution, item IDs in their section.
3. Structure: numbers are unique in their scope; a repealed article has
   only repealed sections.
4. Languages: in each text table, every language cites the same set of
   IDs as the default language.
5. References: each `{@...}` resolves; a repealed target (an item of a
   repealed section counts as repealed) is an error; the constitution
   cites only itself, without a namespace.
6. A fresh build equals the committed `constitution.<lang>.md` and
   `constitution.<lang>.agent.md` files byte for byte (only under
   `--check`).
7. Numbering against a base commit (only under `--check --base <git-ref>`,
   read with `git show`): when the base's `index.toml` is `active`, the
   status has not gone back to `draft`, and the current `edition` is the
   base's or a later month; when it is the base's, every article, section and list item of the base, matched
   by ID, still exists with the same number, a section in the same
   article's directory and an item in the same section; what was repealed
   is still repealed. Each error says which it is: an edition not later than the
   base's, gone, renumbered, moved to another article,
   no longer repealed. The base's files are found in
   the base's own edition directory, which may have another name than the
   current one. A base that is `draft`, has no edition directory or no
   article directories yet, or has an earlier `edition` than the current
   one, skips the comparison with a printed note.

   The revision part of check 7 runs when the base has an `index.toml`,
   whatever its status, unless the base is `active` and its `edition`
   differs from the current one; a `draft` base is compared whatever
   edition each names, so that renaming the edition a draft prepares does
   not start its revisions again, and the approved text's revision is
   greater than the draft's. The constitution text has changed when a file
   under `articles/` was added, removed or edited (compared by its path
   within each side's edition directory), or
   `index.toml` differs in any line other than `revision`; the generated
   files and `rules/` do not count. If it changed, `revision` must be
   greater than the base's; if not, equal to it. "Greater" is strict, not
   "plus one", so that a change which jumps ahead is not an error. A draft
   `0.<n>` compares by the number `n` (`0.9` < `0.10`) and is below every
   whole-number revision; a revision of any other form is an error. After
   an active base of another edition, the revisions are not compared. Its
   purpose is that one revision names one text.

The rules are nodes of the rule tree in the format of
[Rule file format](rule-format.md#rule-file-format), each in
`rules/<id>.toml`; the checks above do not read them. `statute validate
rules`, which finds the constitution beside `rules/`, checks what the
constitution decides of them, with the messages of the checks above: 1:
a node's status agrees with `index.toml` as a rule's does; 5: each
`{@constitution:...}` citation and each `authority` entry, without a
namespace, resolves to a provision that is not repealed; a rule, neither
superseded nor repealed, whose `authority` names a provision and one
above it (an item and its section or article, a section and its
article) is not an error, but draws a warning for each such pair,
`rules/<id>.toml:<line>: warning: rule <id>: authority names both
<above> and <below>, which is under it`; 7, with `--base` and where the
numbering is compared: a node of the base still exists, and stays
repealed or superseded once it was; a rule of a base from before the
rule tree, named by a word, lives on as the node whose `slug` is that
word ([The command and the first version](statute-command.md#the-command-and-the-first-version)).

Everything else of the rule files is checked by `statute validate
rules` ([The command and the first version](statute-command.md#the-command-and-the-first-version)),
a commit hook when `rules/` or the crates change, and so in CI. It
reads the directory as the first-party source and checks what one
source shows: the keys of each node's `kind`, with their types and
values, among them `default` and `uses`
([Rule configuration](rule-config.md)); `id` is the file name, with no
leading zeros; the slugs are unique; each `parent`, `requires`,
`conflicts`, `overrides` and `superseded-by` names a node that exists
and is of the right kind, the parents form a tree of the shape of
[No rule under a rule](rule-config.md#no-rule-under-a-rule) and
[Subdomains](rule-sources.md#subdomains), and `requires` has no cycle
and nothing redundant; each language of a text cites what the original
cites, every translated language has every text and a hash in
`[translations]` that is the original's now, and when one is not, the
error prints the new hash; each `{@rule:<id>}` names a rule of the
source that exists and is not repealed; the defaults of the source
resolve on their own ([Default switches](rule-config.md#default-switches)),
and the warnings of
[Dependencies, one-ofs and packs](rule-sets.md#dependencies-one-ofs-and-packs).
Each finding is a line on standard error; an error makes it exit with
status 1. It reads `params.toml` too and checks each parameter there,
and each placeholder of a rule names one of them
([Parameters](rule-config.md#parameters)).
`crates/statute/tests/real_rules.rs` also reads this repo's `rules/`
and fails on any error or warning.

[Rule sources](rule-sources.md) divides its checks the same way.
`statute validate` checks what one source shows: a rule has `parent`
or `overrides`, one of them and never both; `overrides` is one rule of
another source, `"team:7"`, never a list nor a number, which would name
a node of the same source; and a `parent` in another source is a
rule's only, written `"team:3"`, checked for its form alone, since the
first-party source's domains are the roots
([Subdomains](rule-sources.md#subdomains)); statute accepts one on a
domain of any other source. Since this repo is the first-party source,
`"R200"` in `overrides` or `parent` names a rule of the same source here
and fails, with a note to write the number for a `parent`. What needs
another source is checked when a rule set is resolved: the reference
direction, the ranks of an override, an override by the private layer
of a public repository, and whether a `parent` in another source exists.
All of these are built.

Each check has a test that makes it fail and a passing control; the
numbering check is tested on a temporary git repository
(`crates/statute/tests/constitution.rs`; what `statute validate` checks
of the rules against the constitution in `validate_constitution.rs`,
the rest in `validate.rs` and the crate's other tests).
Checks 1 to 6 run as commit hooks and in CI; the tests run in CI only,
and so does check 7, on a pull request against the tip of the base branch the
checked-out merge commit was made on (its first parent), on a push to
`main` against the commit before the push.

The revision part is only as good as that base. The required check `check`
is not strict: a pull request may merge while it is behind `main`, so its
CI run may not have seen an amendment merged after it last ran, and two
amendments can both pass with the same revision. The pull request run
cannot rule this out; the push run on `main`, which applies the same test
to each merge commit against the commit before it, is what finds it. If it
turns red on a repeated revision, the one who merged opens a correcting
amendment that gives the text a revision of its own. Revisions already
repeated in history stay; each push is judged on its own commit.

The instruction-only check (`check-instruction-changes`) keeps
the changes that an agent reads as instructions apart from all others, so
each can be reviewed and reverted on its own. Which paths are instruction
files, which are generated and which are other files is one table at the
top of the script: a generic part that holds in any repo, this repo's own
part (the constitution, rule and skill sources) and this repo's generated
files. The generic part has what an agent harness reads in any directory
and, at the top of the repo, statute's files: `statute.toml`,
`statute.lock` and `.statute/`, which it compiles the instruction file
from; `statute-checks.toml`, which `statute compile` writes beside the
rule set; and the legacy copies `<name>.legacy.md` in any directory
(the Files table of [statute-migrate](../../skills/statute-migrate/SKILL.md)).
A generated file goes with whichever side changed it: with the
constitution sources it is an amendment, alone with the tools it is a
tools change. This repo's generated files are the built constitution,
`statute-checks.toml` and the `AGENTS.md` at the top of the repo, which
the generic part also lists: generated counts first, since `statute
compile --check` proves each is a compile of the rule set, and each
changes with statute as well as with the rule set (`statute-checks.toml`
with its version, `AGENTS.md` with how it renders the rules), as this
repo's rule L28 provides in place of R46. An `AGENTS.md` in a
subdirectory stays an instruction file. The
script compares `HEAD` with `--base` (`git diff <base>...HEAD`, renames
off), lists both groups when they mix, and on a pass names the
generated files, for the reviewer. CI runs
it against the same base as check 7; a push to `main` that carries
several changes at once is checked as one.

Another repo runs the same check in its own CI: it fetches this repo at a
pinned tag and commit SHA, as an overlay pins the core (Releases and
versions), and runs `PYTHONPATH=<core>/tools/checks/src python3 -m
repo_checks.check_instruction_changes --root . --base <base>` in its
checkout: the script needs only the standard library, so that repo needs
neither uv nor this workspace. The generic part always applies.
The repo declares its own instruction paths in its `AGENTS.md`, in words
for agents, and passes the same paths to the script as `--instructions
<regex>`, one per path, and its generated files as `--generated <regex>`;
either option replaces this repo's own part and generated files.
`statute-checks.toml` and the top `AGENTS.md` stay instruction files
there until the repo declares them with `--generated`, once its CI runs
`statute compile --check`.

The design translation check (`check-design-translation`) keeps the
Chinese translation of the design, `docs/design/zh/<name>.md`, in step
with the English `docs/design/<name>.md`, which is the source; the site
shows both ([Static site](static-site.md)). A translation's first line is
`<!-- en-sha256: <hex> -->`, the SHA-256 of the English file it was
translated from, and its second its `# ` title. Each heading below the
title starts with the anchor of the English heading in its place,
`## <a id="checks"></a>检查`: the anchor GitHub and Hugo give the English
heading (lowercased, punctuation dropped, spaces as hyphens, `-1` after a
repeat), so a link to `#checks` works in both languages, on GitHub and on
the site. Links between design files are written as in the English, and
a link out of `docs/design/` takes one more `../`, so each resolves from
its own file. The check fails on a file missing on either side, a hash
that is not the English file's now, and any heading whose level or
anchor differs from the English one's or that one side lacks; lines in
fenced code blocks are not headings. It also fails on a relative link,
in an English file or a translation, that leads to no file or directory
of the repo from its own file's directory (a translation's link one
`../` short, say), which the site would turn into a link to a missing
file on GitHub; links in code are skipped. So any change to an English file,
a typo fix included, fails the check until the same pull request carries
it into the translation and updates the hash line; the check prints the
new hash. While `docs/design/zh/` does not exist the check prints so and
checks the English links only; once it exists, every file needs its
translation.

Chinese typography is checked with limae and its default
configuration. `lint-zh-sources` hands every `zh` text of
`index.toml` and `articles/` in the edition directory and of `rules/` to limae
as a Markdown file of its own and prints each finding as `path:line:
field: <message>`, the field being the TOML table, with an item's ID
(`items.capital.content`). Before that it replaces each citation `{@...}`
with Chinese text shaped like a constitution citation's rendering
(`第某条`) and puts the citation back afterwards: `agent{@x}` renders as
`agent第十条` and needs a space that limae cannot see next to a brace,
while `依照{@x}批准` needs none (limae 0.13.3, read 2026-10). A rule
citation gets the same placeholder, which stands in for its rendering
only until stage B sets it (a rule's title, which may start or end in
Latin letters). With `--fix` it writes limae's fixes back in place,
changing no other byte, and parses the file again to make sure.
The commit hook and CI only check; the fix is run by hand. The limae hook
runs on every Markdown file of the repo except the theme under `web/themes/`,
English files included, where limae mostly finds nothing. The Chinese
files in the edition directory's `generated/`, `constitution.zh.md` and
`constitution.zh.agent.md`, get a check of what exists only after the
build: section and item numbers, headings, the text around a rendered
citation. A finding in any of the four files there is fixed in the source
or in the build script, never in the generated file. The rules have no built
text yet, so the source check is what covers their Chinese. A finding in a
Markdown file that is not generated is fixed in the file itself. A false positive is passed
over on its line with a `<!-- limae-disable-next-line <rule> -->` comment
above it, never by switching the rule off for the repo.

The private-term scan prints only position and class, never the matched
text. In CI it runs the generic classes only; the term list is injected at
run time on the private side (`--terms-file`) and never committed. Each class
is first run against a control string that must match, so a broken pattern
cannot pass silently.

Undeclared relaxations cannot be caught completely. The relaxation checker
also flags, as a warning, lower-ranking text that cites a rule or article ID
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
[constitution:asking, rule:3]`, `kind: happy` or
`redline`, and a short statement of what it shows),
its inputs and its expected result.

- Static fixtures are deterministic: an input overlay or a lower-ranking file,
  and an assertion on the compiler or a checker (builds; fails with this
  error; this declaration is rejected). They run in every CI run.
- Behavior fixtures (later) give a real agent a scenario and judge what it
  did. They record the harness, model and settings, the number of runs and
  how each failed; one pass is never reported as a guarantee.
- Happy-path and redline fixtures stay apart, so a fix for a safety case that
  breaks ordinary work shows up as a failing happy fixture.
- Every fixture binds to at least one constitution or rule ID, written
  with its namespace as a rule cites it, and the checker rejects a fixture
  whose IDs do not resolve.
- Before a release, every fixture runs, all happy and all redline, as
  rule 26 (`fixture-runs`) requires. Static fixtures run in this
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
  `must-ask.amendments`) run for every jurisdiction. Those for a default
  boundary item test the default; a project that overrides that item under
  `relaxations.project-overrides` runs its own fixture
  for the override in place of the core's, as with a left-out rule.
- General rules: each rule's red-line fixtures live in the core, bound to
  that rule's ID. The core's CI runs all of them. A jurisdiction runs those of
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
  for every project becomes an amendment proposal or, for an item the rule
  library added, a change to that rule approved by a person on that change
  (`list-defaults`); for one project, a `relaxations.project-overrides`
  declaration does it.
- Middle ground (`middle-ground`): when an agent is unsure and
  asks a person, the
  answer is written as a case candidate on the private side, with its
  source. When the same kind of answer recurs, it becomes a rule proposal.
  Which boundary changes the rule library may make, and which need an
  amendment, is `list-defaults`.
- Rule proposals are pull requests that change `rules/`, adopted under
  `rule-evolution`,
  each answering what problem, what evidence, why a rule and not a tool fix,
  which IDs, what authority, what could break, how to test and roll back.
  A clear productivity gain is adopted under `ordinary-rule-adoption`;
  minor improvements are batched (`prefer-tooling`).
- Amendment proposals are pull requests that change `constitution/`; they
  take effect only under `amendment-procedure`.
- Finding the next work and verifying it (stage D): the interface is two
  record shapes, kept independent of any tracker. A work item: what, why,
  acceptance criteria, stop conditions. An acceptance record: each check run,
  at which commit, the output line that shows the result, and each check not
  run with the reason. Whatever finds work and whatever verifies it exchanges
  these; the core defines them in `schema/` when stage D starts.
