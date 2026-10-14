# File structure

Stages follow the plan this repo was started from: A is the constitution
and this design, B is structured sources and deterministic compilation, C is
feedback and proposals (observation only), D is scoped automatic adoption.
Directories for C and D are reserved, not created.

```text
README.md                 the repo's front page, in English            A
README.zh-CN.md           the same page in Chinese                     A
LICENSE                   Apache-2.0, for everything but the text
                          below                                        A
LICENSE-CC-BY-4.0         CC BY 4.0, for the text of constitution/,
                          rules/ and skills/                           A
AGENTS.md                 repo rules for agents working here           A
constitution/
  <YYYY-MM>/              the edition directory, today 2026-10
                          (constitution.md)                            A
    index.toml            status, edition, revision, languages, title,
                          front                                        A
    articles/
      <NN>-<id>/          one directory per article, numbered          A
        article.toml      the article: title, intro                    A
        <NN>[a-z]-<id>.toml one section per file, numbered: status,
                          text in every language, list items           A
    generated/            built from the sources, committed            A
      constitution.en.md  the constitution in English                  A
      constitution.zh.md  the constitution in Chinese                  A
      constitution.en.agent.md  the version for agents, English        A
      constitution.zh.agent.md  the version for agents, Chinese        A
crates/                   the Rust workspace's crates, one directory
                          each                                         A
  statute/                the statute binary, under development        A
docs/
  design/                 this design, one file per topic, indexed
                          by README.md                                 A
    zh/                   its Chinese translation, one file of the
                          same name per file                           A
  pilot/                  this repo's trial of statute: its rule set,
                          lock file and compiled output, which no
                          agent reads (rule-system.md, Moving over)    A
rules/
  <id>.toml               one node of the rule tree per file, named
                          by its number                                A
skills/
  <name>/SKILL.md         one general skill per directory, with any
                          template files beside it, installed as is    A
schema/                   schemas for the rules, fixtures and the
                          overlay manifest                             B
tools/                    a uv workspace, one Python project each:
  compiler/               the constitution compiler, package
                          constitution_compiler, command
                          constitution-build                           A
    src/constitution_compiler/
      build.py            builds and checks the constitution, from the
                          modules below                                A
      model.py            where the sources are, the shapes of their
                          names, the records they are read into        A
      schema.py           the schemas and the per-table format checks
                          of check 1                                   A
      read.py             reads the sources, check 1                   A
      checks.py           checks 2 to 5                                A
      languages.py        what differs between languages               A
      render.py           renders the constitution                     A
      base.py             check 7, numbering against a base commit,
                          and the git access                           A
      revision.py         check 7, the revision against the base's     A
    tests/                render, checks, base, and the shared test
                          tree (test_constitution_fixture.py)          A
  site/                   package constitution_site, depends on the
                          compiler                                     A
    src/constitution_site/
      count_tokens.py     counts the tokens the static site shows
                          (count-tokens)                               A
      check_site_anchors.py   checks the built site's links
                          (check-site-anchors)                         A
    tests/                their tests, and test_site.py, the tests of
                          the site's Hugo templates                    A
  checks/                 package repo_checks, this repo's own checks  A
    src/repo_checks/
      check_instruction_changes.py   instruction files apart
                          (check-instruction-changes)                  A
      check_design_translation.py   the design's translation in step
                          with the English (check-design-translation)  A
      lint_zh_sources.py  limae on the zh sources (lint-zh-sources)    A
    tests/                their tests                                  A
  <others>                compiler, checkers, private-term scanner     B
examples/
  overlay/                a fictional private overlay                  B
  golden/                 its compiled output and build manifest       B
fixtures/
  static/                 deterministic fixtures, happy and redline    B
  behavior/               real-agent scenarios                         C
cases/<id>.md             decided cases                                C
proposals/                reserved for the adoption pipeline           D
web/                      the static site's Hugo project: hugo.toml,
                          content adapter, layouts, style sheet,
                          script, i18n                                 A
  themes/hugo-book/       the theme, a git submodule at a pinned
                          commit                                       A
site/                     built static site, not committed             A
.github/workflows/        CI                                           A
.github/assets/           the README's icon and diagrams, light and
                          dark                                         A
.pre-commit-config.yaml   commit hooks                                 A
pyproject.toml            the workspace root: its members, Python
                          version, pins of limae, pre-commit and the
                          Python checkers, and their configuration     A
uv.lock                   the workspace's one lock file, written by uv A
Cargo.toml                the Rust workspace root: its members and
                          shared package fields                        A
Cargo.lock                the Rust workspace's lock file, written by
                          cargo                                        A
rust-toolchain.toml       the Rust toolchain version, read by rustup
                          locally and in CI                            A
.python-version           the Python minor version uv runs tools/ on   A
.fleet/config.json        issue-tracker metadata for fleet; the
                          toolchain never reads it                     A
```

Four stage B pieces start early. `tools/` starts in stage A with the
constitution build: the constitution is itself compiled from one directory
per article and one file per section (see [Source formats](source-formats.md)). `rules/` starts in stage A too,
holding the
concrete practices taken out of the constitution. `skills/` starts in
stage A as plain directories, each holding a `SKILL.md` with the usual
skill front matter (`name`, `description`) and no metadata of this
repo's own. `crates/` starts in stage A with one crate, `statute`, whose
first release, a placeholder, was published early to hold the name on
crates.io; the binary that compiles and checks the rules grows in it in
stage B (what it does so far:
[Rule system](rule-system.md#the-command-and-the-first-version)). Compiling and checking the
rules, and building the instruction file, stay in stage B.

The design is split into files by topic, so that changes to different
topics touch different files, as rule 82 (`design-files-by-topic`) asks.
Planned, not done yet: `rule-system.md` still holds several topics and
will be split into files of its own, each listed in the index
([agent-constitution design](README.md)).

Stage B, in the order to build it: `schema/` and the schema checker,
the rest of `rules/` (filled by migration), the compiler, `examples/`
with the drift check, the reference and relaxation checkers, the
private-term scanner, `fixtures/static/` with the fixture gate, then load
verification. Each step is usable on its own.

What the amendment article protects is the text of the constitution
(`protected-text`); in the core that is the sources under
the edition directory (`index.toml` and `articles/`) and the wording
of the files built from them. A change to the text is an amendment and
takes effect as `approving-amendments` says (see [Approving an
amendment](amendments-and-releases.md#approving-an-amendment)).
Everything else is an ordinary change under `rule-evolution`
(`ordinary-tooling`), reviewed and tested like any other:
`tools/`, `crates/` and `schema/` (compiler, checkers, the fixture-gate runner),
`.github/workflows/`, `fixtures/` and `examples/golden/`. A person's
attention goes to what the constitution says; the tools are kept honest by
their unit tests and happy-path end-to-end tests, not by a person reading
them. The generated files change only by rebuilding. A change to the
build script that alters only their numbering or layout is ordinary, and its
pull request shows their diff so a person can glance at the format; one that
alters their wording changes the text and is an amendment
(`protected-text`). The notes the build script writes itself (the header
line, the repealed note, and in the version for agents the opening note and
the note in place of a provision's text) are not the text of the
constitution: they change as the script does, by an ordinary change
(`ordinary-tooling`), and may only say what the text already says (that
the constitution binds every agent, `terms.agent`), point to the full
text and describe the layout; a note that would change a meaning needs an
amendment first. Which provisions the version for agents leaves out is set
in the sources (`audience`, see [Constitution](constitution.md#constitution)), so changing it is an amendment.
