# agent-constitution design

Status: draft. This is the one design document of the repo. The constitution
itself is built from `constitution/index.toml` and `constitution/articles/`
into [English](../constitution/constitution.en.md)
and [Chinese](../constitution/constitution.zh.md); this document cites it
only by ID (see Source formats, IDs) and never restates it.

The repo provides three kinds of installable content: the constitution,
the general rules and the general skills. A user picks which of them to
install through configuration. How they are installed, and what metadata
that needs, is not decided yet; it will be settled as the toolchain is
built.

## Goals and non-goals

Goals:

- Hold the constitution: the few stable principles and the governance rules
  (what binds whom, what only a person may decide, how rules and the
  constitution change). It names no concrete practice: no command, storage
  place or tool.
- Hold the general rule library and general skills: the concrete practices
  that refine the constitution, written without one user's machines,
  accounts or history. The rules are a default set; each jurisdiction chooses
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
| General rules, a default set each jurisdiction chooses from | Private rules: rules that only make sense on that user's stack |
| General skills | Private skills (for example the orchestra skill) |
| Compiler, checkers, private-term scanner | Provenance: the casebook of incidents and decisions, with dates and sources |
| A fictional example overlay and its golden output | The private term list the scanner reads at run time |
| Static fixtures | The apply step that runs the compiler and installs the output |

The overlay never changes or relaxes the constitution. It chooses which
general rules it uses (`division-of-instructions.rules`; see Choosing rules), adds
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

The standing of the constitution's texts in its languages is set by the
constitution itself (`amendment-languages`,
`equal-languages`), as drafted; it stays open until a person
approves that text. The rules follow the same terms. What this repo adds:

- The checker only proves structural alignment (every text table has every
  language; the languages define the same IDs in the same order and cite
  the same IDs). There is no check, based on the git diff,
  that a change touching one language also touches the other: fixing a typo
  in one language only is a legitimate change, and such a check would block
  it or teach people to make empty edits. Review covers meaning.
- The Chinese text follows the typography that limae checks with its
  default rules (Checkers): full-width commas, full stops, enumeration
  commas, semicolons and colons; half-width parentheses with one space
  outside. Item numbers render half-width for the same reason, `(**三**) `.

## File structure

Stages follow the plan this repo was started from: A is the constitution
and this design, B is structured sources and deterministic compilation, C is
feedback and proposals (observation only), D is scoped automatic adoption.
Directories for C and D are reserved, not created.

```text
README.md
AGENTS.md                 repo rules for agents working here           A
constitution/
  index.toml              status, edition, revision, languages, title,
                          front                                        A
  articles/
    <NN>-<id>/            one directory per article, numbered          A
      article.toml        the article: title, intro                    A
      <NN>[a-z]-<id>.toml one section per file, numbered: status,
                          text in every language, list items           A
  constitution.en.md      generated: the constitution in English       A
  constitution.zh.md      generated: the constitution in Chinese       A
  constitution.en.agent.md  generated: the version for agents, English A
  constitution.zh.agent.md  generated: the version for agents, Chinese A
docs/
  design.md               this document                                A
rules/
  <id>.toml               one general rule per file, text in every
                          language                                     A
  retired-ids.txt         IDs of deleted rules, one per line           A
skills/
  <name>/SKILL.md         one general skill per directory, with any
                          template files beside it, installed as is    A
schema/                   schemas for the rules, fixtures and the
                          overlay manifest                             B
tools/
  build_constitution.py   builds and checks the constitution, from the
                          modules below                                A
  constitution_sources.py   the sources' data model, check 1           A
  constitution_checks.py  checks 2 to 5                                A
  constitution_languages.py   what differs between languages           A
  constitution_render.py  renders the constitution                     A
  constitution_base.py    check 7, against a base commit               A
  test_constitution_*.py  their tests: render, checks, base, and the
                          shared test tree (fixture)                   A
  count_tokens.py         counts the tokens the static site shows       A
  build_site.py           counts, builds the static site with Hugo
                          and checks it                                 A
  check_site_anchors.py   checks the built site's in-page links         A
  test_<script>.py        tests of the three scripts above              A
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
.pre-commit-config.yaml   commit hooks                                 A
pyproject.toml            the Python environment of tools/: Python
                          version, pins of limae, pre-commit and the
                          Python checkers, and their configuration     A
uv.lock                   its lock file, written by uv                 A
.python-version           the Python minor version uv runs tools/ on   A
.fleet/config.json        issue-tracker metadata for fleet; the
                          toolchain never reads it                     A
```

Three stage B pieces start early. `tools/` starts in stage A with the
constitution build: the constitution is itself compiled from one directory
per article and one file per section (see Source formats). `rules/` starts in stage A too,
holding the
concrete practices taken out of the constitution. `skills/` starts in
stage A as plain directories, each holding a `SKILL.md` with the usual
skill front matter (`name`, `description`) and no metadata of this
repo's own. Compiling and checking the
rules, and building the instruction file, stay in stage B.

Stage B, in the order to build it: `schema/` and the schema checker,
the rest of `rules/` (filled by migration), the compiler, `examples/`
with the drift check, the reference and relaxation checkers, the
private-term scanner, `fixtures/static/` with the fixture gate, then load
verification. Each step is usable on its own.

What the amendment article protects is the text of the constitution
(`protected-text`); in the core that is the sources under
`constitution/` (`index.toml` and `articles/`) and the wording
of the files built from them. A change to the text is an amendment and
takes effect as `approving-amendments` says (see Approving an
amendment).
Everything else is an ordinary change under `rule-evolution`
(`ordinary-tooling`), reviewed and tested like any other:
`tools/` and `schema/` (compiler, checkers, the fixture-gate runner),
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
in the sources (`audience`, below), so changing it is an amendment.

## Source formats

All sources are TOML, read with the standard library's `tomllib`. Every
text a reader sees is a table whose keys are the language codes listed in
`constitution/index.toml` and whose values are the text in that language,
so one file holds all languages of one provision side by side. Multi-line
text is a literal string (`'''...'''`); the build drops the newline after
the opening quotes and before the closing ones.

### IDs

Every provision of the constitution (article, section, list item) and
every rule has an ID: one or more lowercase words joined by hyphens,
`^[a-z0-9]+(-[a-z0-9]+)*$`, for example `leaks` or `credential-storage`.
The ID is a key, like a BibTeX key, chosen by a person once and never
changed; it says what the provision is about, not where it stands.

There are two namespaces. In the constitution, article and section IDs
are unique across the whole constitution, and a list item's ID only
within its section, so items can have short names (`capital`). In the
rules, a rule ID is unique among the rules, the private overlay's rules
included. The same word may be used once in each namespace. An ID is
never reused. From the first approved revision on, a provision of the
constitution is only ever repealed, never deleted, so the sources hold
every ID used since then and the build rejects a second one. While the
constitution was a draft, revisions deleted provisions outright; the
build does not see those IDs, so they are listed here, and a draft
revision that deletes another adds it:

- revision 0.5: the article `interpretation`;
- revision 0.7: the article `roles` and its sections `relay-role`,
  `job-role` and `worker-role`; the items `terms.deployment`, `terms.job`,
  `terms.layer`, `terms.project` and `terms.skill`;
- revision 0.9: the section `third-party-personal-info`;
- revision 0.11: the section `asking-once`;
- revision 0.14: the items `terms.authoritative-interpretation`,
  `terms.edition`, `terms.hard-rule` and `terms.undetermined`;
- revision 0.16: the items `must-ask.downtime`, `must-ask.machines`,
  `must-ask.money`, `must-ask.releases` and `must-ask.risk-controls`;
- revision 0.18: the item `must-ask.productive-resources`, renamed
  `must-ask.means-of-production`;
- revision 0.24: the article `protection-scope`, whose sections
  `protected-text` and `ordinary-tooling` moved into
  `amendment-procedure`;
- revision 0.28: the section `response-priority` and its items
  `response-priority.safety-incidents` (its text is now the section
  `safety-incidents`), `response-priority.productivity-gains` and
  `response-priority.minor-improvements`;
- revision 0.30: the section `checks-everywhere`, replaced by
  `decidable-provisions`;
- revision 0.33: the section `rules-refine`, whose text is now the item
  `division-of-instructions.rules`;
- revision 0.35: the section `role-definitions`;
- revision 0.36: the section `compliance`.

The date-prefixed IDs of the earlier flat sources (`20261012-...`) are
retired as well. A rule may be deleted, and its ID then goes into
`rules/retired-ids.txt` (see Rules).

Text cites with `{@...}`; nothing else in the text is markup, and any
other braces are output as is. The constitution cites only itself,
without a namespace: an article or section by its ID (`{@safety}`,
`{@leaks}`), a list item by its section's ID, a dot and the item's ID
(`{@must-ask.capital}`), always this full path, even inside the same
section. A rule always names the namespace: `{@constitution:leaks}`,
`{@constitution:must-ask.capital}`, `{@rule:credential-storage}`; the colon
says which body of text, the dot where in it. The build renders a
citation of the constitution as the number the provision has in that
language (below); the generated text shows numbers only, never IDs. A
long-lived record outside the sources (a case, a pull request, a report)
may cite a provision by ID or by number; a citation by number names the
edition, within which a number never changes (see Numbering). An ID
carries over from one edition to the next for every provision whose
content carries over.

### Constitution

The constitution is compiled by `tools/build_constitution.py` and the
`tools/constitution_*.py` modules it calls (Python standard library only,
run with `uv run` in the environment of `pyproject.toml`) from
`constitution/index.toml` and one directory per article under
`constitution/articles/`. It writes two files for each language in
`index.toml`, `constitution/constitution.<lang>.md` (the full version) and
`constitution/constitution.<lang>.agent.md` (the version for agents, see
The version for agents), committed and never edited by hand; `--check`
writes nothing and runs the checks under Checkers.

```text
constitution/articles/
  05-boundary-rule/
    article.toml
    01-must-ask.toml
    02-decide-yourself.toml
```

`constitution/index.toml`:

- `status`: `draft` until a person first approves the text, then
  `active`.
- `edition`: the name of the edition, the month a principal approved it as
  in force, `YYYY-MM`; required. A draft names the edition it prepares,
  the month it is meant to be approved in; if it is approved in another
  month, the edition is renamed then. An edition is a whole version of the
  text, like a constitution named by the year it was adopted. Starting a
  new edition is itself an amendment and needs a principal's approval, and
  at most one edition starts in a month, so no two share a name; within an
  edition, numbers never change (see Numbering), and a new edition may
  renumber freely.
- `revision`: a string, the revision within the edition: a draft is
  `0.<n>`, with `<n>` counting from 1 in every draft (the first
  edition's and each later one's), the first approved text of an edition
  is `1`, each adopted amendment adds one, and a new edition starts again
  at `1`. The build manifest records it. Within an edition it rises
  exactly when the constitution text changes (Checkers, check 7).
- `languages`: the language codes, the first being the default. Each
  must have a rendering entry in the build script (one table,
  `LANGUAGES`, holds everything that differs by language: the file header,
  the shapes of headings, numbers and citations, the repealed note). Adding
  a language means adding one entry there and one key to every text table.
- `[title]`: the title, one line per language; the build adds the
  edition after it (see Numbering).
- `[front]`: the Markdown between the title and the first article (draft
  notice, `## Preamble` and its text), per language. It may cite.

An article is a directory `constitution/articles/<NN>-<id>/`: its number,
two digits from `01`, a hyphen and its ID. In it, `article.toml` holds
`id` (equal to the directory name without the number), `[title]`, an
optional `[intro]` (the paragraph between the heading and the first
section) and, only when the whole article is repealed,
`repealed = "YYYY-MM"`.

Each section is a file `<NN>[a-z]-<id>.toml` in its article's directory:
its number, two digits from `01` with an optional lowercase letter, a
hyphen, its ID and `.toml`. The section belongs to the article whose
directory holds it; there is no field for it. The file holds `id` (equal
to the file name without the number and `.toml`), `status` (`draft`,
`active` or `repealed`), `repealed = "YYYY-MM"` exactly when the status is
`repealed`, an optional `audience = "maintainers"` (its only value) for a
provision on how the constitution itself is made, arranged and amended,
and `[content]`, the section's text per language: the first
line is the sentence after the section's number, further lines, nested
lists and blank lines follow as they are, and the build indents them
under the list item.

A section with list items has one `[[items]]` table per item, in the
section's file: `id`, `number` (`1`, `2`, `2a`, within the section),
`status`, `repealed` and `audience` as for a section, and
`[items.content]`, the item's text per language. The section's `[content]`
then holds only the lead-in; the build renders the items below it in number
order.

```toml
id = "must-ask"
status = "draft"

[content]
en = '''
**Must go through a principal** whose authority covers the item ({@explicit-yes}).
'''
zh = '''
**必须经过委托人**，即由权限覆盖该事项的委托人同意 ({@explicit-yes})。
'''

[[items]]
id = "capital"
number = "1"
status = "draft"

[items.content]
en = '''
Capital: allocating, spending, ...
'''
zh = '''
资本：资金和其他有价资源的配置……
'''
```

Article, section and item order come from the numbers alone; there is no
separate list. While `index.toml` is `draft`, every section, item and rule
is `draft` and nothing is repealed; once it is `active`, every section and
item is `active` or `repealed`.

### Numbering

Numbers are not derived from position. An article's number is the prefix
of its directory name and a section's the prefix of its file name, written
there and nowhere else; a list item's is its `number` field. A section or
item number is an integer with an optional lowercase letter suffix (`2`,
`2a`), the suffix marking one inserted later between `2` and `3`; articles
are only ever appended and take no suffix. Only after the first approved
revision does an insertion take a suffix (`<NN>a-<id>.toml`); while the
constitution is a draft, a new provision takes the next number in order
and the ones after it are renumbered. A name prefix is that number
with two digits (`02`, `02a`), so the files sort in reading order. Sort
order is by integer, then suffix: `2` < `2a` < `2b` < `3`. Within one
scope (all articles; the sections of one article; the items of one
section) numbers are unique; a draft need not be contiguous.

From the first approved revision on, numbers follow `stable-numbers`
within an edition; the check `--check --base <git-ref>` enforces it
against a base commit of the same edition (see Checkers, check 7):
renaming a directory or file to a new number, or moving a section to
another article's directory, is an error. A repealed provision is
rendered as a one-line note in place of its text (table below). While the
constitution is a draft, or when a new edition starts, numbers may
change, directories and files are renamed with them, and the check only
says that it did not compare.

Rendering, with `must-ask` as the first section of the fourth article,
`capital` as its third item and `authority` as the article's ID; `<a id="must-ask"></a>`
is written `⟨must-ask⟩` in the table:

| Item | en | zh |
| --- | --- | --- |
| Title | `# Agent Constitution (2026-10 Edition)`, the `edition` | `# Agent 宪法 (2026-10 版)` |
| Article heading | `## ⟨authority⟩Article IV. <Title>` | `## ⟨authority⟩第四条 <标题>` |
| Section | `-   ⟨must-ask⟩**§1** <content>` | `-   ⟨must-ask⟩**第一款** <content>` |
| List item | `    -   ⟨must-ask.capital⟩(**3**) <content>` | `    -   ⟨must-ask.capital⟩(**三**) <content>` |
| Citation of an article | `[Art. IV][authority]` | `[第四条][authority]` |
| Citation of a section | `[Art. IV §1][must-ask]` | `[第四条第一款][must-ask]` |
| Citation of a list item | `[Art. IV §1(3)][must-ask.capital]` | `[第四条第一款第三项][must-ask.capital]` |
| Suffix | `§1a`, `(3a)`, `Art. IV §1(3a)` | `第一款之一`, `(三之一)`, `第四条第一款第三项之一` (a is `之一`, b is `之二`) |
| Repealed section | `-   ⟨…⟩**§3** (repealed November 2026)` | `-   ⟨…⟩**第三款** (2026 年 11 月废止)` |
| Repealed list item | `    -   ⟨…⟩(**3**) (repealed November 2026)` | `    -   ⟨…⟩(**三**) (2026 年 11 月废止)` |
| Repealed article | `## ⟨…⟩Article IV (repealed November 2026)`, nothing below it | `## ⟨…⟩第四条 (2026 年 11 月废止)` |
| Adjacent item citations | `Art. IV §1(2) or (4)`, `Art. IV §1(1) to (7)`, `Art. IV §1(1), (2) and (4)` | `第四条第一款第二项或第四项`, `第四条第一款第一项至第七项`, `第四条第一款第一项、第二项和第四项` |
| Adjacent section citations | `Art. IV §1 and §2`, `Art. IV §1 to §3`, `Art. IV §1, §2 and §4` | `第四条第一款和第二款`, `第四条第一款至第三款`, `第四条第一款、第二款和第四款` |

A list marker is padded to four characters, and the further lines of a
section or an item, nested lists and paragraphs included, are indented
to its content column: four spaces for a section, eight for an item.
A section or an item whose text has a blank line in it is followed by a
blank line. Python-Markdown nests only what is indented four spaces and
joins a list line to the paragraph just above it; with this layout the
output renders alike there and in CommonMark (GitHub) as long as a
source text's own nested blocks are indented four spaces and a list line
after a paragraph has a blank line before it. A source text that breaks
either still renders wrong in Python-Markdown; the constitution has none
today.

An item citation renders short, as its item number alone (`(4)`, `(2a)`;
`第四项`, `第二项之一`), when the text just before it is exactly one
connector and the text before that is a citation of another item of the
same section. A section citation renders short the same way, as its
section number alone (`§2`, `§1a`; `第二款`, `第一款之一`), after a
connector and a citation of another section of the same article. The
connectors are ` or `, ` and `, `, ` and ` to ` in English and `或`,
`和`, `、` and `至` in Chinese; a range is written in the sources with
` to ` or `至` between its first and last citation. An item of another
section, a section of another article, a citation after a citation one
level up or down (an article and its section, a section and an item),
and a citation with any other text before it keep the full path. The
sources always write the full path.

Every article, section and item starts with an anchor named by its ID,
not its number: `<a id="<article-id>">`, `<a id="<section-id>">` and
`<a id="<section-id>.<item-id>">` (IDs). The anchors are the same in every
language and survive renumbering; article and section IDs are unique in
the whole constitution, so they do not collide. A citation is a link to
its provision's anchor, a short one too: `[Art. IV §1][must-ask] and
[§2][…]`, each number its own link. The links are reference links, with
one definition `[<ref>]: #<ref>` per cited provision at the end of the
file, in order of first citation: limae reads the `)` of an inline link
`[…](#…)` followed by a Chinese character as a parenthesis that needs a
space (limae 0.13.3, read 2026-10). Section numbers are bold; an item
number is bold inside its parentheses, because limae asks for a space
between `**` and a parenthesis.

Each output starts with a plain-text paragraph, in its own language:
generated file, do not edit it, edit the sources; and the `revision`,
followed by `(draft)` (`(草案)`) while `status` is `draft`, and the
language; in the full version, when a part has the
asterisk (a provision marked for maintainers, or the preamble's heading),
also what the asterisk means: the parts marked are background for people,
where the constitution comes from, how it is organized and how it is
amended, and the version for agents omits them. Output is UTF-8 with LF
line endings and a final newline, with no timestamp; the same sources give
byte-identical output.

### The version for agents

Agents read the constitution through the compiled instructions, where
every token counts; the preamble and the provisions on how the
constitution itself is made and amended matter to the people who maintain
it and to an agent drafting an amendment, who reads the full version. So
the build writes a second file per language, `constitution.<lang>.agent.md`,
which differs from the full version in this:

- The header line says it is the version for agents, without the text of
  the provisions written for human readers.
- `[front]` (the draft notice and the preamble) is left out, so in the
  full version each heading in it, `## Preamble`, has the asterisk after
  it. In its place
  is a note of the build script's own, one sentence per language in
  `LANGUAGES`: that this is the version for agents, while `index.toml` is
  `draft` that the constitution is a draft not in force, once `active`
  that every agent must follow it; that the preamble and the text of the
  provisions written for human readers, on how the constitution is
  organized and amended, are left out, with only a pointer to the full
  version in their place; and that before changing the constitution or the
  tools that build it, an agent reads the full version, named by its path.
  It is not the text of the constitution (see File structure).
- A section or item with `audience = "maintainers"` keeps its number and
  shows a note in place of its text: `-   **§2** (see the full text)`,
  `-   **第二款** (见完整版)`; a repealed one shows its repealed note as
  usual. In the full version the same provision keeps its text and has an
  asterisk after its number, `**§2**<sup class="omitted">*</sup>`, which
  the header line explains.
- There are no anchors and no link definitions: a citation is its label
  as plain text (`Art. VIII §6`, `第八条第六款`), short where the full
  version is short.

### Rules

One rule per file, `rules/<id>.toml`, so that one proposal is one file and
a diff shows exactly one rule changing.

```toml
id = "credential-storage"
status = "draft"
scope = "all"
authority = ["constitution:safety", "constitution:sensitive-info"]

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
- The text cites the constitution with `{@constitution:<id>}` and another
  rule with `{@rule:<id>}` (see IDs); in the compiled instruction file a
  citation of a rule renders as that rule's title, and the rest of the
  rendering is set in stage B. A rule has no number.
- A rule about messages or notices says whom it covers: agent to agent,
  agent to person, or both.
- `id`: stable, also the file name. Renaming the title never changes it.
- `status`: `draft`, `active` or `repealed`. Only `active` is compiled. A
  repealed rule keeps its file and its ID. While the constitution is a
  draft, every rule is `draft` too.
- Unlike the constitution, a rule may be deleted. Its ID then goes on a line
  of its own in `rules/retired-ids.txt` and is never used again; the build
  rejects a rule whose ID is listed there. A missing file is an empty
  list.
- `scope`: who it addresses: `all`, or one of a short list defined in
  `schema/` (for example `implementer`, `reviewer`, `merger`). The compiler
  renders it as a one-line prefix; it does not filter by it. There is one
  compiled file for every role.
- `authority`: the constitution articles, sections or list items the rule
  rests on, cited as in the text (`constitution:<id>`); none of them
  repealed. A rule with no authority does
  not compile.
- `adjustable` (optional): the values in this rule that a lower-ranking text may
  change under `relaxations.adjustable-rules`, each with its default, for
  example `adjustable = ["wait_hours=24"]`. A rule without it cannot be
  relaxed that way. These are distinct from local parameters (below): a
  local parameter is a value the private overlay fills in; an adjustable
  value is one a repo or a task may change.

More fields (`supersedes`, `verification`, `expires`) are added when a real
rule needs them, not before.

Private rules in the overlay have the same format and share the rules'
namespace. The compiler checks that the union of core and overlay rule IDs
is unique and that none of them is retired in either.

### Choosing rules

The general rules are a default set, not obligations on every jurisdiction
(`division-of-instructions.rules`): one jurisdiction may need a rule that another has
good reason to drop (a repo that stays private on a self-hosted server may
keep credentials in it on purpose). Open, for the person to decide. Recommended:

- A jurisdiction uses every general rule by default. Its overlay leaves a rule
  out only by naming it, with a one-line reason, in `overlay.toml`
  (`exclude = ["credential-storage"]`). The build manifest lists
  what was left out.
- A repo inside a jurisdiction departs from a rule only by a relaxation
  declaration (below), where the constitution permits it. It does not leave
  rules out of the compiled file, because one compiled file serves all of a
  jurisdiction's repos.
- To change a rule rather than drop it, the overlay leaves it out and adds a
  private rule in its place.
- The constitution cannot be left out or relaxed this way.

Reasons: defaulting to all means a new rule reaches every jurisdiction without
anyone remembering to list it, where an opt-in list would go stale as rules
are added. Leaving a rule out is not a relaxation (`division-of-instructions.rules`),
but naming each one, with its reason, keeps it visible and puts everything a
jurisdiction dropped in one place. Exclusion per jurisdiction, not per repo, matches how the output is
built: one file per jurisdiction.

### Relaxation declarations

A lower-ranking text (a repo's `AGENTS.md`, or a task file) relaxes a rule only by
naming the constitution's permission for it:

```markdown
## Relaxations

- relaxes: constitution:relaxations.adjustable-rules
  target: rule:example-rule/wait_hours=4
  scope: <this repo, or this task>
  condition: <how this text meets the condition that item states>
```

One item per relaxation, with all four keys: the permission (an item of
`relaxations`, cited as a rule cites it), what it relaxes (a rule, a
boundary item such as `must-ask.means-of-production`, or under
`relaxations.adjustable-rules` a rule's adjustable value and its new
setting), where it applies, and how the condition of that item is met. A relaxation not declared this
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
into the instruction file; they are installed as they are where the
harnesses look for skills. How they are installed is not decided yet (see
the opening of this document); the overlay's apply step under Compilation
is the current plan. A skill says how to do something; what must be
done stays in a rule, and the skill cites the rule ID. The instruction-only
check counts everything under `skills/` as instruction files, since a
skill directs the agents that read it.

## Compilation

Input: one pinned version of this repo (a tag, checked against its commit
SHA) and one overlay directory. The overlay selects the language. The
compiler takes the constitution from the version for agents in that
language, `constitution.<lang>.agent.md` (it does not read the
constitution sources) and each rule's block in that
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
   such as `<!-- constitution:authority -->`, so any paragraph of the output
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

Wiring on the private side, as currently planned (how the content is
installed is not decided yet): the overlay's apply step fetches the pinned core
version, verifies tag and commit, runs the compiler, runs the checkers that
need private input (the private-term scan), installs the output and the
skills, then runs load verification. The real output is never committed to
this repo.

## Static site

The built constitution is also published as static web pages, English at
`site/index.html` and Chinese at `site/zh/index.html`, each with the
version for agents under `agent/` (`site/agent/index.html`,
`site/zh/agent/index.html`). Hugo builds them, `hugo-0.167.0 -s web`
alone; `uv run tools/build_site.py` runs what CI runs: the token count
(below), Hugo, and the link check. Only the build exists here; publishing
it is a release (rule `must-ask-items`) and not set up.

- Input: the generated `constitution/constitution.<lang>.md`, not the
  sources: the citations, numbers and anchors are rendered once, by
  `build_constitution.py`. `web/hugo.toml` mounts them from
  `constitution/` as assets and `constitution/index.toml` as data; Python
  writes no page. The content adapter `web/content/_content.gotmpl` makes
  each into the home page of its language: the header line is dropped,
  the title becomes the page's, each heading's anchor (`## <a
  id="general-provisions"></a>Article I. ...`) becomes the heading's id,
  so the table of contents links to it, and the page gets the lengths
  (below) as params. It fails the build when a file does not start with
  the header line, a blank line and the `# ` title. The layout
  (`web/layouts/home.html`) sets the title, the list under it and the
  text in `<div class="constitution">`, the root the style sheet hangs
  on. The title (`web/layouts/_partials/constitution/title.html`) has no
  `#` link from the theme and keeps its name and its parenthesis each on
  one line, so a narrow screen breaks it only before the parenthesis; its
  size is chosen so that it fits on one line in a desktop window 1400
  pixels wide. The revision from `index.toml` is followed by `(draft)`
  (`(草案)`) while its `status` is `draft`. Its words, in both languages,
  are in
  `web/i18n/`. Under
  the title is a list without bullets: the revision and the length
  (below), each centered under the title, and, when the text has an
  asterisk, its legend, starting at the column's left edge, smaller and
  fainter than the two lines above it: the asterisk,
  then that the parts marked are background for people (where the
  constitution comes from, how it is organized and how it is amended) and
  that the version for agents, linked, omits them. The asterisk is set small and bold in
  the accent color, not a superscript and without opening up the line,
  lowered until its middle sits a little above the middle of an item
  number's parentheses and its top well under the top of the number. The
  legend and the table of contents show it the same way.
- Version for agents: each `constitution/constitution.<lang>.agent.md`
  becomes the page `agent/` under the home page of its language, made by
  the same content adapter (layout `web/layouts/page.html`) but with its
  text as is: no anchors, no links, the provisions
  written for human readers only their numbers and a pointer to the full
  version. Its headings carry `{anchor=false}`, so the theme adds no `#`
  link to them. Under the title is a list like the home page's, both lines
  centered: the revision, and a link back to the full version.
  The sidebar, the language switch (to the other language's version for
  agents), the color switch and the table of contents are as on the home
  page, the table of contents linking to the IDs Hugo gives the headings.
- Length: the second item of the list gives the length of the version
  for agents, bare, which is what takes room in an agent's context: the
  generated file without its header line, in characters in both
  languages, words split on white space (ASCII white space, the only kind
  the files hold) in English, and tokens as counted for the model
  `params.tokenCount.model` of `web/hugo.toml` by Anthropic's count_tokens
  API (`POST /v1/messages/count_tokens`, standard library only). The
  content adapter counts the characters and words; `tools/count_tokens.py`
  counts the tokens and writes them to `web/data/lengths.json` (not
  committed), with the SHA-256 of each text it counted and the abbreviated
  commit it ran on. The site shows a token count, and the commit, only
  when the SHA-256 is that of the text it builds, so a file left from
  another commit shows no stale number; without the file it says the
  tokens were not counted. An info
  button after it opens a popup, on hover or a click or tap; it is a
  disclosure button (`aria-expanded`), reached with the keyboard, and
  Escape closes it (`web/assets/count-info.js`, inlined in `<head>`). The
  popup hangs from the button with no gap, centered under the length and
  at least as wide as it, so the pointer reaches its link without leaving
  it; on a narrow screen it spans the column. The
  popup first says that the length under the title is that of the version
  for agents and gives the lengths of both versions in a small table
  (spans with ARIA table roles, so a screen reader names the columns; the
  full version is the generated file without its header line, anchors and
  link definitions included; a token count not made is a dash), the table
  as wide as the popup; under it, fainter than the table, a bulleted list
  (spans with ARIA list roles): what the version for agents omits, which
  API counted the tokens for which model or why they were not counted (the
  model named in words by `params.tokenCount.name` next to its ID), and
  the revision and the abbreviated commit of the count (left out without
  a count of this text, or outside git); last, on a line
  of its own, the link to the page of the version for agents. The text of
  the popup is the size of the legend under the title, the numbers of the
  table a little larger. Each page's two versions are counted
  separately, so a build with a credential makes four calls.
- Credential: only from the environment, never from a file in this repo,
  in one of two variables. `CONSTITUTION_TOKEN_COUNT_API_KEY` holds an API
  key, sent as `x-api-key`. `CONSTITUTION_TOKEN_COUNT_ACCESS_TOKEN` holds a
  short-lived access token that a workflow got through Anthropic's
  Workload Identity Federation (its OIDC token exchanged at
  `POST /v1/oauth/token`), sent as `Authorization: Bearer`; when both are
  set, the token is used, so a key left behind does not shadow the
  federation. The credential never reaches the output, the logs or the
  page; a failure prints one line with the HTTP status or the exception's
  class. Without a credential, or when the call fails or a connect or read
  stalls for 10 seconds, the page says the tokens were not counted and the
  build goes on; with `--require-tokens` (of `count_tokens.py` and
  `build_site.py`) the build fails instead. The
  workflow `.github/workflows/token-count.yml` runs on each push to `main`,
  the only event the federation rule accepts: it asks GitHub for the run's
  OIDC token (audience `https://api.anthropic.com`), exchanges it for an
  access token with the IDs of the federation rule, organization, service
  account and workspace, which are repository variables
  (`ANTHROPIC_FEDERATION_RULE_ID`, `ANTHROPIC_ORGANIZATION_ID`,
  `ANTHROPIC_SERVICE_ACCOUNT_ID`, `ANTHROPIC_WORKSPACE_ID`; identifiers,
  not secrets), builds the site with `--require-tokens`, and writes the
  length line of both pages into the job summary and into the log, which,
  unlike the summary, the API can read. It publishes nothing; a failed
  exchange or count fails the run. The future Pages workflow will get its
  token the same way. The CI on pull requests gets no credential,
  and neither does a pull request from a fork. A local build that wants the
  count sets the key variable for that one command, reading the key from
  the maintainer's secret store. So the pages of one commit differ with
  and without a credential, in that line and the popup only.
- Generator: Hugo, a single binary pinned by version in `build_site.py`
  and in CI, which writes to `site/` (`publishDir`), with the theme
  hugo-book (MIT license) as a git submodule at
  `web/themes/hugo-book`, pinned to a commit. The theme gives the language
  switch, the light, dark and auto color themes, the sidebar's list of
  pages and the table of contents at the right. The sidebar lists the
  pages, not the articles: today the constitution, the home page, with
  its version for agents under it, both named in `web/hugo.toml`'s menus
  (the version for agents is the home page's child, which the content
  adapter hides from the theme's own list with `bookHidden`). The table
  of contents lists the page's level-2
  headings, the preamble and the articles (`markup.tableOfContents` in
  `web/hugo.toml`), under a heading of its own; on a narrow screen the
  theme hides it behind a button at the right of the top bar, which opens
  it above the text, as the button at the left opens the sidebar. The
  site overrides the theme's sidebar (`layouts/_partials/docs/menu.html`)
  to put a link to the source repository, with GitHub's mark from
  Octicons (MIT license), between the site's title and the language
  switch, and adds, through the theme's inject partials, a
  switch for the color theme (follow the system, light, dark; the choice
  kept in `localStorage`, set in `<head>` before the page is drawn), and
  the fonts. A submodule keeps the theme's files out of this repo's
  diffs; a Hugo module would need Go on every machine that builds.
- Raw HTML: Goldmark's `unsafe` renderer is on, since the anchors
  (`<a id="...">`) and the asterisks are raw HTML. The content is only this
  repo's own built constitution, never outside input.
- Anchors: the ID of the article, section or item: `#general-provisions`,
  `#terms`, `#terms.principal`, the same in both languages and in the
  Markdown. A citation is a reference link to it. `:target` flashes the
  element an anchor starts.
- Style: `web/assets/styles/custom.css`, loaded by the theme after its own.
  Its first part uses only the page's own markup and moves to another
  generator as is; the last part maps it onto hugo-book's variables.
  Centered article headings, bold section and item numbers (bold in the
  Markdown), English text ragged right and Chinese justified, a print
  style without link decoration. Every link in the text, under the title
  and in the popup is underlined like the citations: the text color with
  a light underline in the color of the rules, both turning to the accent
  color under the pointer. The navigation (the sidebar, the table of
  contents and the top bar of a narrow screen) is not underlined, but is
  under the pointer or the keyboard's focus. The sidebar marks the
  current page in the accent color. The
  text size grows with the window
  between a floor and a ceiling (`clamp()`), and the column is set in em,
  so a line keeps about 90 characters in English and 40 in Chinese where
  the window has room for it; between hugo-book's breakpoint (56rem) and
  about 1100px, where both side columns are shown, the column is
  narrower. It is centered between the sidebar and the table of contents. Fonts: EB Garamond and Noto
  Serif SC (SIL Open Font License 1.1) from Google Fonts as variable
  fonts, falling back to local serif fonts. The colors are one palette
  block, light and dark; contrast of text on the page and the sidebar is
  above WCAG AA in both, the values are in the style sheet.
- The few Chinese strings of the site itself (`web/i18n/zh.yaml`, which
  holds the labels and the popup text, and the language label and menu
  names in `hugo.toml`) are outside limae's reach.
- `site/` is not committed: a generated page under review would make every
  amendment touch it, while an amendment may change only the constitution
  sources and their generated Markdown. Hugo needs a download, so the build
  is not a commit hook, and the tests of the templates
  (`test_build_site.py`, which builds a made-up constitution) are skipped
  without it, except in CI; CI installs Hugo before the tests, builds the
  site on every pull request and push and runs `check_site_anchors.py` on
  it.

## Checkers

Every checker is a command that exits non-zero on failure and prints file and
position. What each one proves, and what it does not:

| Checker | Proves | Does not prove |
| --- | --- | --- |
| Constitution (`build_constitution.py --check`) | Each constitution and rule source has the format and the name above; each ID is well formed and unique in its namespace, and no rule uses a retired ID; numbers are unique in their scope; the languages of each text cite the same IDs; every citation and authority resolves to a provision or rule that is not repealed, the constitution citing only itself; the committed outputs equal a fresh build; with `--base`, the edition is an active base's or a later one, and in the base's edition nothing numbered in the base has disappeared, changed number or moved, and no rule of the base is gone without being retired; with `--base` and whatever the base's status, within one edition, and from a draft base whatever edition each names, the revision is greater than the base's if the constitution text changed and equal to it if not | That the two languages mean the same thing, or that a change to one language was carried to the other |
| Instruction-only changes (`check_instruction_changes.py --base`) | The files a change touches since its base are all instruction files or all other files, generated files not counted; a rename counts as its old and its new path | That the path table is complete: a file an agent reads as instructions but the table does not list passes as an other file. That a generated file changed only in rendering when a tools change regenerated it; the reviewer checks that |
| Static site (`build_site.py`, then `check_site_anchors.py`) | Hugo builds all pages without a warning; on every page each in-page link `#id` has an element with that id, each link to another page of the site (relative, or from the site's root when it starts with `/`) leads to a file of the site, no id appears twice, and no list marker shows as literal text (`- §`, `- 第`) | That the pages look right, or that the fonts load |
| Chinese typography (`lint_zh_sources.py`; limae on `constitution.zh.md` and `constitution.zh.agent.md`) | Every `zh` text of the constitution and rule sources, and both versions of the built Chinese constitution, pass limae's default rules, a constitution citation judged as the Chinese text it renders to | That the Chinese reads naturally or says what the English says; the English text is not checked |
| Python code (`ruff check`, `ruff format --check`, `pydoclint`, `basedpyright`, configured in `pyproject.toml`) | The scripts under `tools/` follow the configured lint rules and formatting, their docstrings name each argument and return value, and the type checker finds no error in standard mode | That the scripts do what their docstrings say; that is what the tests are for |
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
   `index.toml`; everything under `constitution/articles/` is an
   article's directory, with an article's name and an `article.toml`, and
   everything else in an article's directory is a `.toml` file with a
   section's name; there is no `constitution/sections/`; `id` equals the
   name without its number; `status`, item `number` and `repealed` are
   well formed and agree with each other and with the status of
   `index.toml`; `edition` is a real month `YYYY-MM`. Hidden names
   (starting with `.`, such as `.DS_Store` or an editor's swap file) are
   skipped.
2. IDs: each has the format; article and section IDs are unique in the
   constitution, item IDs in their section, rule IDs among the rules; no
   rule's ID is listed in `rules/retired-ids.txt`, and each line there is
   an ID.
3. Structure: numbers are unique in their scope; a repealed article has
   only repealed sections.
4. Languages: in each text table, every language cites the same set of
   IDs as the default language.
5. References: each `{@...}` and each `authority` entry resolves; a
   repealed target (an item of a repealed section counts as repealed) is an
   error; the constitution cites only itself, without a namespace; a rule
   cites with `constitution:` or `rule:`; `authority` names only
   constitution provisions.
6. A fresh build equals the committed `constitution.<lang>.md` and
   `constitution.<lang>.agent.md` files byte for byte (only under
   `--check`).
7. Numbering against a base commit (only under `--check --base <git-ref>`,
   read with `git show`): when the base's `index.toml` is `active`, the
   status has not gone back to `draft`, and the current `edition` is the
   base's or a later month; when it is the base's, every article, section and list item of the base, matched
   by ID, still exists with the same number, a section in the same
   article's directory and an item in the same section; what was repealed
   is still repealed; every rule of the base still exists or has its ID in
   `rules/retired-ids.txt`, and every ID retired at the base is still
   listed. Each error says which it is: an edition not later than the
   base's, gone, renumbered, moved to another article,
   no longer repealed, no longer retired. A base that is `draft`, has no
   article directories yet, or has an earlier `edition` than the current
   one, skips the comparison with a printed note.

   The revision part of check 7 runs when the base has an `index.toml`,
   whatever its status, unless the base is `active` and its `edition`
   differs from the current one; a `draft` base is compared whatever
   edition each names, so that renaming the edition a draft prepares does
   not start its revisions again, and the approved text's revision is
   greater than the draft's. The constitution text has changed when a file
   under `constitution/articles/` was added, removed or edited, or
   `index.toml` differs in any line other than `revision`; the generated
   files and `rules/` do not count. If it changed, `revision` must be
   greater than the base's; if not, equal to it. "Greater" is strict, not
   "plus one", so that a change which jumps ahead is not an error. A draft
   `0.<n>` compares by the number `n` (`0.9` < `0.10`) and is below every
   whole-number revision; a revision of any other form is an error. After
   an active base of another edition, the revisions are not compared. Its
   purpose is that one revision names one text.

Each check has a test that makes it fail and a passing control; the
numbering check is tested on a temporary git repository
(`tools/test_constitution_checks.py`, `tools/test_constitution_base.py`).
Checks 1 to 6 and the tests run as commit hooks and in CI; check 7 runs in
CI only, on a pull request against the tip of the base branch the
checked-out merge commit was made on (its first parent), on a push to
`main` against the commit before the push.

The revision part is only as good as that base. The required check `check`
is strict: a pull request must be up to date with `main` before it merges,
so its CI run has seen the revision of every amendment merged before it,
and two amendments cannot both take the same revision. The push run on
`main` applies the same test to each merge commit as a backstop; if it
turns red on a repeated revision, the one who merged opens a correcting
amendment that gives the text a revision of its own. Revisions already
repeated in history stay; each push is judged on its own commit.

The instruction-only check (`tools/check_instruction_changes.py`) keeps
the changes that an agent reads as instructions apart from all others, so
each can be reviewed and reverted on its own. Which paths are instruction
files, which are generated and which are other files is one table at the
top of the script: a generic part that holds in any repo, this repo's own
part (the constitution, rule and skill sources) and this repo's generated
files.
A generated file goes with whichever side changed it: with the
constitution sources it is an amendment, alone with the tools it is a
tools change. The script compares `HEAD` with `--base` (`git diff
<base>...HEAD`, renames off) and lists both groups when they mix. CI runs
it against the same base as check 7; a push to `main` that carries
several changes at once is checked as one.

Another repo runs the same check in its own CI: it fetches this repo at a
pinned tag and commit SHA, as an overlay pins the core (Releases and
versions), and runs `python3 <core>/tools/check_instruction_changes.py
--root . --base <base>` in its checkout. The generic part always applies.
The repo declares its own instruction paths in its `AGENTS.md`, in words
for agents, and passes the same paths to the script as `--instructions
<regex>`, one per path, and its generated files as `--generated <regex>`;
either option replaces this repo's own part and generated files.

Chinese typography is checked twice, with limae and its default
configuration. `tools/lint_zh_sources.py` hands every `zh` text of
`constitution/index.toml`, `constitution/articles/` and `rules/` to limae
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
on `constitution/constitution.zh.md` and `constitution.zh.agent.md`
checks what exists only after the build: section and item numbers,
headings, the text around a rendered citation. A finding there is fixed in the source or in the build script,
never in the generated file. The rules have no built text yet, so the
source check is what covers their Chinese.

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
[constitution:asking, rule:credential-storage]`, `kind: happy` or
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
  the rule `fixture-outcomes` requires. Static fixtures run in this
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

## Approving an amendment

Built so that the person spends as little time as possible on it, which is
what `purpose` asks for. The person reads what the constitution
says and nothing
else: tools, checks and fixtures are ordinary changes (see File structure).

While the constitution is a draft, an amendment pull request is merged
once agent review has passed, and the person reads it afterwards on a
preview of the site; the description is still written as below, so that
the reading is quick. Only a change to an approved edition waits for the
person's yes on a named head commit before it is merged. An approved
edition and the draft of the next one are to be kept side by side; how
the repo holds the two (branches, directories or something else) is
open.

- An amendment pull request changes only `constitution/`: the sources and
  the generated files rebuilt from them.
- Its description opens with the diff of the generated text in the
  language the person reads, then the same diff in each other language,
  collapsed. The generated diff is what agents will read,
  numbers included; the person does not need to read the sources.
- Agent review is finished before the person is asked: CI's constitution
  check is green, and the reviewer has checked that the texts in all
  languages say the same thing. A person approving the change approves the
  texts in all languages on the strength of that review.
- The description ends with the open questions, each clear and easy to
  answer and with a recommended answer (`asking`).
- For an approved edition, the person's yes names the head commit it
  approves, and the merge lands exactly that commit (`independent-review`).
- `proposing-amendments` asks for the generated diff in the
  language the approving person reads, and `amendment-languages`
  for the texts in all languages to change together; the steps above are how this repo
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
- Pushing a release tag makes a version available. By default a release
  is an item of `must-ask`, added by the rule `must-ask-items`, and needs a
  person's yes; a project may move it under
  `relaxations.project-overrides`.
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
