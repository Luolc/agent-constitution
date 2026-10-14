# Rule system

Status: draft. The rules of this repo are written in the
[Rule file format](#rule-file-format), which `constitution-build` checks,
and the command reads them and a rule set and writes the lock file
([The command and the first version](#the-command-and-the-first-version)); the
rest is not built yet. This document describes how rules are written,
numbered, chosen, checked and compiled once the rule system is built. Where it differs from what the rest of the design says
today, the section [What this replaces](#what-this-replaces) lists the
difference; those sections stay as they are until the work that implements
this design changes them.

## Contents

- [Terms](#terms)
- [What one rule holds](#what-one-rule-holds)
- [IDs and revisions](#ids-and-revisions)
- [Rule file format](#rule-file-format)
- [Rule sets and the lock file](#rule-sets-and-the-lock-file)
- [Sources and references](#sources-and-references)
- [Rules and skills](#rules-and-skills)
- [Dependencies, one-ofs and packs](#dependencies-one-ofs-and-packs)
- [Tracing rules to the constitution](#tracing-rules-to-the-constitution)
- [Checks](#checks)
- [Outcomes and the review loop](#outcomes-and-the-review-loop)
- [Trial rules](#trial-rules)
- [Compilation and the site](#compilation-and-the-site)
- [The command and the first version](#the-command-and-the-first-version)
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
- **Domain**: a node at the top of the parent tree (Python, CI, Git,
  security and so on) that only gives a heading (see
  [Dependencies, one-ofs and packs](#dependencies-one-ofs-and-packs)).
- **One-of**: a node of the parent tree of which at most one child is
  enabled; enabling none is allowed too (see
  [Dependencies, one-ofs and packs](#dependencies-one-ofs-and-packs)).
- **Lock file**: written next to the rule set; it pins the revision of
  every enabled rule and resolves every source to an exact commit.
- **Check**: the code that decides, for one rule, whether a repository, a
  change or a record complies.
- **Reviewer**: an agent that judges what the checks could not settle.

## What one rule holds

A rule requires one thing, and one thing only, whose violation can be
decided: given a repository, a change or a record, someone can answer
"was this rule broken?". It works like a lint rule in a linter, scoped
narrowly enough to be checked, which is what `decidable-provisions` asks
of every lower-ranking provision.

Its text has three parts: the statement, one sentence in the imperative
that says what to do; the reason; and the verification, how to decide
whether the rule was broken. Each part is short. A rule that needs more is
usually several rules written as one, and is split.

## IDs and revisions

A rule's ID is a number, assigned in sequence and never reused. Rules are
expected to grow fast and to be written and checked mostly by agents, so a
person does not read each one closely; a number avoids debating names, and
it never has to change when the wording does. Domains and one-ofs take
their IDs from the same sequence, so a number names exactly one node of
its source.

No rule is ever deleted. Whether a change keeps the ID depends on one
question: does it change what counts as compliant?

- If not, the rule is edited in place and its revision goes up by one.
  Rewording, a new example, a fix to a bug in its check: what was
  compliant stays compliant, and what was a violation stays one.
- If it does, the change gets a new ID. The old rule is marked
  superseded, names the rule that supersedes it (or the rules, when it
  is split into several), and stays in the source
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

## Rule file format

Each node of the parent tree (a rule, a one-of or a domain) is one TOML
file, `rules/<id>.toml`, flat in one directory: `rules/42.toml`, with no
zero padding, since the IDs keep growing and any fixed width would one
day be too short. The tree is recorded only in the `parent` field, never
in directories, so moving a rule under another parent changes one line
and moves no file. Tools always sort the files by number, not by name.

A rule, here one that a translation into Chinese accompanies:

```toml
id = 42
slug = "uv-sync-locked-in-ci"
revision = 1
kind = "rule"
status = "trial"
lang = "en"
parent = 7
requires = [12]
authority = ["simplicity"]
constitution = ">=1"
level = "project"
check = "builtin"

[title]
en = "Sync from the lock file first in CI"
zh = "CI 里先按锁文件同步依赖"

[text.statement]
en = "In CI, run `uv sync --locked` before any step that runs Python."
zh = "在 CI 里，先运行 `uv sync --locked`，再运行任何执行 Python 的步骤。"

[text.reason]
en = '''
A job that resolves its dependencies anew can test other versions than the
ones the lock file records, and a stale lock file goes unnoticed.
'''
zh = '''
重新解析依赖的 job 测到的版本可能与锁文件记录的不同，锁文件过期了也发现不了。
'''

[text.verification]
en = '''
Violated when a CI job runs Python before a `uv sync --locked` step, or
runs `uv sync` without `--locked`.
'''
zh = '''
CI 的 job 在 `uv sync --locked` 这一步之前运行了 Python，或者运行 `uv sync` 时没带 `--locked`，即为违反。
'''

[translations]
zh = "<SHA-256 of the original text>"
```

A one-of and a domain, in the same format:

```toml
id = 5
slug = "python-package-manager"
revision = 1
kind = "one-of"
status = "active"
lang = "en"
parent = 3

[title]
en = "Python package manager"
```

```toml
id = 3
slug = "python"
revision = 1
kind = "domain"
status = "active"
lang = "en"

[title]
en = "Python"
```

The fields:

- `id`: an integer from 1, unique within the source across rules,
  one-ofs and domains, and never reused. The file name is the ID.
- `slug` (optional): a short readable name, lowercase words joined by
  hyphens, unique within the source. It is used only in web addresses and
  to make the file easier to recognize; no reference ever uses it. The
  rules of this repo kept their earlier word IDs here when they moved to
  this format.
- `revision`: an integer from 1, raised by one at each change in place
  ([IDs and revisions](#ids-and-revisions)).
- `kind`: `rule`, `one-of` or `domain`.
- `status`: `draft`, `trial`, `active`, `superseded` or `repealed`.
- `superseded-by`: the ID of the rule of the same source that supersedes
  this one, or a list of IDs when several rules together take its place,
  `superseded-by = [22, 23]`; present only when `status` is
  `superseded`. Each names another rule, and none twice; a list is never
  empty.
- `lang`: the language the rule was written in, its original, as a
  language code such as `"en"` or `"zh"`.
- `parent`: the ID of the parent node, a rule, a one-of or a domain of the
  same source. Every rule and one-of has one; a domain has none.
- `requires` (optional): the rules or one-ofs that enabling this rule
  requires. A node of the same source is written as a number, one of
  another source as a string of the source's short name, a colon and the
  ID (`"team:7"`), the name coming from the source's `[sources]` table
  ([Sources and references](#sources-and-references)).
- `conflicts` (optional): the rules this rule cannot be used with,
  written like `requires` and on one side only
  ([Dependencies, one-ofs and packs](#dependencies-one-ofs-and-packs)).
- `authority`: the IDs of the provisions of the constitution the rule
  refines, written as the constitution writes its own citations, without a
  namespace (`"simplicity"`, `"must-ask.capital"`). This is the value of a
  field, not a `{@...}` citation in a text, so the namespace that
  [IDs](source-formats.md#ids) asks of a rule's citations does not apply;
  a citation in a rule's text still names it (`{@constitution:simplicity}`).
- `constitution`: the revisions of the constitution the rule is
  compatible with, written as a Python version specifier (`">=1"`,
  `">=1,<4"`; see
  [Tracing rules to the constitution](#tracing-rules-to-the-constitution)).
- `level`: the default level, `project` or `user`
  ([Rule sets and the lock file](#rule-sets-and-the-lock-file)).
- `check`: how compliance is checked ([Checks](#checks)). `builtin`: the
  rule has a module in this repo's crate. `command`: an external command,
  given by two more fields, `run`, the command line, and `tools`, the list
  of tools it needs (`["uv"]`). `review`: there is no automatic check, and
  the rule goes straight to the reviewer.
- `[title]`: a short title, on one line (a multi-line string may end in a
  newline).
- `[text.statement]`, `[text.reason]` and `[text.verification]`: the three
  parts of the text ([What one rule holds](#what-one-rule-holds)).
- `[translations]` (optional): one key per translated language, never the
  original's, whose value is the SHA-256 of the original text the
  translation was made from.

A domain and a one-of have only `id`, `slug`, `revision`, `kind`,
`status`, `lang`, `parent` (a one-of only), `[title]` and, when the title
is translated, `[translations]`. They have no text, check or other
metadata.

**Languages.** Each text, `[title]` and the three parts, is a table whose
keys are language codes, as in [Source formats](source-formats.md). The
key `lang` names is required; every other language is an optional
translation. A rule may therefore be written in one language only, by an
author of any language. A translation gives the title and all three parts
in its language, and its key in `[translations]` records which original it
follows. That hash is the SHA-256 of the original-language values of
`title`, `text.statement`, `text.reason` and `text.verification`, in this
order, as the TOML parser returns them (before the build drops the
newlines next to the quotes of a multi-line string, see
[Source formats](source-formats.md)), with each CRLF line end of the file
taken as LF, so that a file checked out with either hashes the same (a
`\r\n` written as an escape is not a line end and stays), encoded in
UTF-8 and joined with one NUL byte (0x00) between each two; for a domain or a one-of it is the
hash of the title alone. When the original changes and a translation's
hash no longer matches, the check fails and prints the new hash, as
`check-design-translation` does for the design. Compiled into a language,
a rule uses its translation into that language when it has one, and its
original otherwise.

## Rule sets and the lock file

A rule set lists rule IDs (and packs, below). Its lock file records, for
each enabled rule, the revision in use and the commit of the source it came
from, so the same rule set and lock file always build the same output.

The rule set is `statute.toml` and the lock file `statute.lock`, at the
root of the repository (named for the command,
[The command and the first version](#the-command-and-the-first-version)). The
rule set lists only the rules chosen directly:

```toml
constitution = { source = "R", edition = "2026-10" }
rules = ["R42", "R51", "12"]
packs = ["R3"]

[sources]
R = { git = "github.com/Luolc/agent-constitution", tag = "<tag>" }

[compile]
agents-md = ["statement"]
```

The rule set names the constitution's source and edition; the revision
is the one at the commit of that source the lock file pins, and the lock
file records it. `rules` lists the rules chosen directly and `packs` the
roots of the packs enabled, each as a reference
([Sources and references](#sources-and-references)): here `R42` and
`R51` are first-party rules and `12` is one of the repository's own. A
child of a one-of is chosen by listing it in `rules`, here `R51`; there
is no other
field for it. A domain or a one-of listed in `rules` fails the build,
which says to enable the domain as a pack and the one-of through one of
its children. `[compile]` sets what the compiled `AGENTS.md` carries of each rule:
by default only `statement`; adding `reason` or `verification` puts those
parts in as well ([Compilation and the site](#compilation-and-the-site)).
`lang` in `[compile]` names the language it is written in, `en` by
default.

The lock file is TOML, written by statute and never edited by hand. It
records the constitution (its repository, edition, revision and commit),
each source (its short name, address and commit), and each enabled rule,
whether chosen directly or brought in by a requirement (its reference,
written as the rule set writes it, `R42` or `team:7`, and `L12` for a
local rule; its revision; and the SHA-256 of its file):

```toml
[constitution]
repository = "github.com/Luolc/agent-constitution"
edition = "2026-10"
revision = "<revision>"
commit = "<commit>"

[[sources]]
name = "R"
address = "github.com/Luolc/agent-constitution"
commit = "<commit>"

[[rules]]
rule = "R7"
revision = 2
sha256 = "<SHA-256 of rules/7.toml>"

[[rules]]
rule = "R42"
revision = 1
sha256 = "<SHA-256 of rules/42.toml>"
```

One field, `rule`, names the rule in the same form everywhere else, in
place of a source and an ID in two fields, so that a line of the lock
file can be searched for as the rule set and the messages write it.
A lock file of the earlier format, which named a rule by `source` and
`id`, fails `upgrade` with a note to run `statute lock`, which writes it
anew.

A source on disk (`path`, see
[Sources and references](#sources-and-references)) has no commit to
record: the commit that records the lock file would change it, so the
lock file could never be current. The constitution and the source are
then recorded by their `path` in place of their address and commit, and
each rule's `sha256` alone pins its content:

```toml
[constitution]
path = "."
edition = "2026-10"
revision = "1"

[[sources]]
name = "R"
path = "."
```

Upgrading is an explicit step that rewrites the lock file and shows the
diff of every rule whose revision moved, and of every rule a requirement
brought in or dropped. When the rule set enables a superseded rule, the
build warns and names its successors. For a rule listed in `rules` or a
pack root in `packs`, one command, `statute upgrade
--replace-superseded`, replaces the old ID with the new one in the rule
set when there is one successor, and leaves the choice among several to
the user; for a rule brought in by another, the warning names the rule that
brings it in. An enabled repealed rule draws a warning too. These are
warnings, not errors, since the rule set keeps working as it did.
Downstream users do not have to follow the supersession chain by hand.

The promise that nothing changes under a rule set until it upgrades
holds only where the lock file can keep the old version: a git source,
locked at a commit. A
source on disk has no old version to keep: the lock file records each
rule's SHA-256, and once the rule is edited the file it hashed is gone.
So `lock` reads a source on disk as it is now, as uv does a path
dependency, and `upgrade` writes the same lock file and shows what
changed. For the same reason the first version cannot show the text of
a change: it lists the constitution's revision, each rule whose revision
or content moved, and each rule brought in, with why, or dropped. The
text diff waits for git sources, whose old commit can be read. A rule
whose content changed while its revision did not draws a warning from
`lock` and a note on its line from `upgrade`, since an edited rule's
revision goes up by one ([IDs and revisions](#ids-and-revisions)). The
two cannot tell a changed translation from a changed original, so either
draws it.

**Bound to one constitution.** A rule set names the constitution it
rests on: the repository it comes from, the edition and the revision.
The lock file pins that revision as it pins a source's commit, and one
compilation uses exactly one revision of one constitution. Rule sets
that rest on different constitutions are never combined: at least the
repository and the edition must agree, and the revision is checked
against each rule (see
[Tracing rules to the constitution](#tracing-rules-to-the-constitution)).
When the constitution is amended, a rule set pinned to the older
revision does not change; moving it to the new revision is an explicit
upgrade, like any other. A rule set that inherits a base rule set may pin
another revision than the base does. The build then takes the newer of
the two, and fails if some enabled rule's range cannot hold it.

**Per repository, with inheritance.** A rule set belongs to a repository,
and it may inherit a shared base rule set, so the rules every repository
of a user or team agrees on are written once. A repository adds to the
base, leaves rules out, and picks among the children of a one-of. The
jurisdiction of the current design (`division-of-instructions.rules`) is
then the owner of a base rule set that its repositories inherit; the
rule set and compiled output of each repository replace the one selection
of rules and compiled file per jurisdiction. Exclusion is checked per rule
set, so one repository can use uv for its Python packages and another
Bazel.

**Monorepos.** A rule set may be split by path into segments, each
naming a directory and the rules for it. Exclusion is checked within each
segment, so one part of a monorepo can use uv and another Bazel. What
the agents do with instruction files in subdirectories was measured on
2026-10-14, in non-interactive mode only (Claude Code 2.1.296, Codex
0.159.3, Grok 1.0.50; Grok's project instructions load only in a trusted
folder):

- With the working directory in the subdirectory, all three load that
  subdirectory's instruction file at the start.
- With the working directory at the repository root and a file in the
  subdirectory being changed, only Claude Code loads the subdirectory's
  `CLAUDE.md` by itself, when it touches a file there. Codex and Grok
  load nothing; the model has to go and read the file, and in the runs
  here it did. In both runs where Grok read the subdirectory's file
  before the root's, it followed the root's on a conflict, against its
  documentation.
- By default, Claude Code reads `AGENTS.md` only when no `CLAUDE.md` is
  in the working directory or above it (a setting can make it read both;
  not tested).

So a segment cannot rely on its own directory's instruction file being
read. The compiler writes every segment's rules into the instruction
file at the repository root, and each such rule says which directory it
applies to; the compiled output of one repository uses one file name. A
subdirectory file may repeat a segment, but nothing depends on it being
read.

**Where it lives.** The design follows a package manager such as uv: a
source is like a package index, a rule set like a project manifest, the
lock file like a lock file, compiling like a sync, and the user-level
cache like a package cache. The repository holds the rule set, the lock
file and the compiled output; an upgrade is one command in the repository
and shows up as a pull request. The user level holds only the command, a
cache of the sources, and a thin personal layer for what concerns the
person, not the project: the language of replies, reading messages as
speech-to-text, where credentials are kept, how directories are laid out
on the machine. That layer takes over the personal part of the private
overlay already in the design. Each rule declares a default level
(project rules at the repository level, rules about the person or the
machine at the user level), and the user can change it. The reason is
that a person who clones the repository, CI and a cloud agent cannot read
user-level files, and each repository pins its own versions and upgrades
on its own schedule. The first version builds the repository level only
([The command and the first version](#the-command-and-the-first-version)).

## Sources and references

A rule is found by its source and its ID. The commit is not part of
that identity.

**Naming a source.** A rule set names each source in a `[sources]`
table, with its address and the tag or branch to follow:

```toml
[sources]
R = { git = "github.com/Luolc/agent-constitution", tag = "<tag>" }
team = { git = "example.test/their-team/rules", branch = "main" }
```

The key `R` is reserved for the first-party source: this repo, which
publishes the constitution and the general rules. A rule set declares
it as it declares any other source, with the tag or the path to read,
since a source the command knew by itself would need a built-in address
and version. Every other source has a short name of lowercase words
joined by hyphens, which the rule set chooses. `L` and `local` are
reserved for the repository's own rules (below), and `ac`, the
first-party source's earlier name, fails with a note to name it `R`.
A source may also be a directory on disk, `R = { path = "." }`,
relative to the rule set's directory: a repository that holds its own
rules, as this one does, names itself this way. Its rules are in
`rules/` below that directory and its constitution in
`constitution/<edition>/`. The first version reads only one source,
on disk, and reports a `git` source as not supported yet.

**Three kinds of reference.** Each kind has a form of its own, so no
reference can be read as another kind:

- a first-party rule: `R` and the ID, `R42`;
- a local rule, one in the `rules/` directory next to the rule set,
  that is, of the repository that holds it: the ID alone, `12`; `L12`
  and `local:12` mean the same;
- a rule of another source: its short name, a colon and the ID,
  `team:7`.

statute names a rule in one form wherever it shows one: in its
messages, in `statute explain`, in the lock file and in the compiled
`AGENTS.md`. That form is `R42`, `L12` or `team:7`: a local rule is
shown as `L12`, so that it never reads like `R12`. The site lists the
first-party rules alone, as `R42`. A rule set keeps each entry as
written, and the commands that add one write `R42`, `12` or `team:7`.

When the first-party source is the rule set's own directory,
`R = { path = "." }`, as in this repo, its `rules/` are the
first-party rules. They are written and shown as `R12`, and a local
reference to one, such as `12`, fails with a note to write `R12`, so
that one file never has two names.

The short name of another source means something only inside the file
that defines it, so two rule sets may use `team` for different
sources. When a source moves to another address, one line of the rule
set changes and no reference does. A source that references rules of
other sources has its own `[sources]` table in the same shape, and its
names need not agree with those of any rule set that uses it. The
build matches sources by address, not by name: the `team` of one file
and the `core` of another are the same source when both point to the
same address.

**References in a rule.** In a rule's own file, a number in `requires`
or `conflicts` and a citation `{@rule:<id>}` in its text name a rule
of the same source. A rule names a rule of another source in
`requires` or `conflicts` as `"R12"` or `"team:7"`, with the names of
its own source's `[sources]` table. A rule of the first-party source
names another first-party rule by its number alone; for `"R12"` there,
statute says to write `12`. When compiled, a rule of the same
source is shown with that source's prefix: `{@rule:12}` in a
first-party rule is `R12`, in a local rule `L12`.

**Versions sit on the source.** The rule set says which tag or branch of
a source to follow; the lock file resolves it to an exact commit. This is
how `pyproject.toml` and `uv.lock` divide the work: the manifest states a
range, the lock file records the exact version. A reference carries no
commit, for two reasons. Upgrading would mean editing every reference.
And if two rules of one source were pinned to different commits, a
`requires` or `conflicts` between them would point at a rule that
differs from the one that was chosen. So a rule set takes one version of
each source.

**Dependencies across sources.** A rule of one source may list a rule of
another under `requires`, such as `"R12"`. The source of the requiring
rule then declares, in its `[sources]` table, which versions of that
other source it works with, as a range written as a Python version
specifier. At compile time the build looks for one version of each source
that every declaration accepts. The tag or branch the rule set itself
follows counts as one of the declarations. If there is no such version,
the build fails and names the declarations that disagree.

**Supersession stays inside a source.** Only a source can supersede its
own rules. Another source cannot declare that its rule replaces ours. It
can recommend the replacement in the text of its own rule; whether to
switch is decided by the user, in the user's rule set.

**Rules from third parties.** A user may enable rules from sources the
user chose, such as a team's own or someone else's public source. That is
the user's right and the user's risk: a third-party rule is not reviewed
with this repo's rules, and it steers the agents that read the compiled
file. The built-in rules say how rules and skills from a source outside
this repo are to be checked before they are enabled. By default a user
takes the default constitution and the default rule set, with no
third-party source. Bringing in third-party rules is an advanced use, and
its details are left for later; the constitution holds none of it.

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
at least one of them is in the rule set. It declares them in the
`metadata` map of its front matter, under the key `statute-rules`: the
IDs of rules of its own source, separated by spaces.

```yaml
---
name: example-skill
description: How to do the thing that rules 42 and 51 require.
metadata:
  statute-rules: "42 51"
---
```

The [Agent Skills specification](https://agentskills.io/specification)
allows only `name`, `description`, `license`, `compatibility`,
`metadata` and `allowed-tools` in the front matter, and only strings as
the values of `metadata` (read 2026-10-14); hence a string under
`metadata` and not a key of its own. statute reads the declaration only
from a block mapping, as above, the key on its own line below
`metadata:`; a `metadata` in flow style (`metadata: {statute-rules:
"42"}`) that names the key fails the compile, naming the skill and the
line. A skill that declares no rules is
never installed, and one that names a rule its source does not have
fails the compile. A repository's `AGENTS.md` keeps
only what is specific to that repository (what it is, its language, its
layout, its check commands) and names the rule set it uses
(`division-of-instructions.project-instructions`). A rule set is chosen
per repository ([Rule sets and the lock file](#rule-sets-and-the-lock-file)).

## Dependencies, one-ofs and packs

Rules are related in two ways, and the tree has two more kinds of node,
domains and one-ofs:

- `parent`: every rule has exactly one parent, either another rule, a
  one-of or a domain (below), so the rules form a tree, each domain
  refined level by level. The tree decides where a rule stands in the
  compiled text and on the site, which block it is compiled in and which
  pack it belongs to.
- `requires`: enabling rule A requires another rule B, or a one-of
  (below). A rule may require several, and the requirements form a
  directed acyclic graph. A rule that says how to run checks through uv
  requires the rule that has uv manage the project's dependencies. A
  rule of another source may be required too; see
  [Sources and references](#sources-and-references).

One parent is not enough on its own because many rules span domains: "in
CI, run `uv sync --locked` first" depends both on uv and on the CI
service. It sits under the uv rule and requires the one-of "CI service",
satisfied by any of its children, such as a GitHub Actions rule.

**Domains.** A domain is a node at the top of the tree, such as Python,
CI, Git or security. Besides its ID and title it has only the few fields
every node has ([Rule file format](#rule-file-format)), and no text or
check; it has no parent and cannot be enabled on its own. When any rule
below it is enabled, the domain appears in the compiled output as a heading. Domains
do not nest: a finer grouping is a rule or a one-of under the domain.

**One-ofs.** A one-of is a node of the parent tree of which at most one
child is enabled within one rule set (or one path segment of it).
Enabling none of its children is allowed, as in a repository with no
Python at all; "one of" here never means "exactly one". The one-of
"Python package manager" under Python has the children uv, Poetry and
Bazel. A one-of has an ID, and adding one goes through review like adding
a rule. A rule that requires a one-of is satisfied when any one of its
children is enabled.

**Conflicts found later.** Two rules that nobody saw as alternatives may
turn out not to work together. An agent that suspects this reports it, and
a reviewer judges it. Usually the fix is to change a rule: split it,
narrow it or reword it, and whether it keeps its ID follows the rule in
[IDs and revisions](#ids-and-revisions). When both rules are sound and
cannot be used together, one of them gets `conflicts = [ID]`. The relation
is symmetric and written on one side only: on the rule that came later,
the one being added or changed when the conflict was found, and never on
the one that was already there. Across sources it can only be written on
one's own side anyway. The command derives the other side, and both the
build check and the site show the conflict on both rules. A conflict
written on both sides draws a warning, not an error. The reasons: a fact
is written in one place (`no-restating`), and another source's files
cannot be changed. Three or more rules at one place that conflict with
each other are alternatives, and are collected under a one-of instead. A
rule may not be declared in conflict with one that enabling it brings in
along `parent` or `requires`, directly or through other rules, since
enabling the one would enable the other; the build fails on it. Sibling
rules under one parent may conflict.

The build checks these constraints:

- No cycle, in the tree or in the requirements.
- A domain has no parent; every rule and one-of has one.
- A rule's parent, when it is a rule, counts as one of its requirements:
  enabling a rule requires enabling its parent, and the parent is not
  written again under `requires`. When the parent is a one-of, the
  requirement passes through it to the one-of's own parent: enabling uv
  under "Python package manager" still requires the rule above that
  one-of. A parent that is a domain requires nothing. A one-of counts as
  enabled when one of its children is.
- No redundant requirement: if A requires B and B requires C, A does not
  also list C.
- A requirement that crosses to another subtree should name a one-of, not
  a particular rule. The number of such requirements has no hard limit;
  the build warns about them.
- At most one child of a one-of is enabled in a rule set or path segment,
  and no two enabled rules conflict. A rule that brings in two children of
  one one-of, along `parent` or `requires`, could never be enabled, and
  the build fails on it; so it does on a rule that requires a one-of with
  no children yet.
- A rule that requires a superseded or repealed rule draws a warning,
  which names the successors when there are any.

**Missing requirements are added.** When a rule set enables a rule
without one it requires, statute adds the missing rule, as a package
manager adds a dependency: the rule set lists only the rules chosen
directly, the lock file records the ones brought in, and an upgrade's
diff shows them. This applies equally to a parent that counts as a
requirement. Among missing requirements, the one case that fails is a
requirement of a one-of when the rule set has enabled none of its
children: statute cannot choose for
the user, so it fails and lists the children to choose from.

Packs: a pack is a subtree, named by its root, and a rule set can enable
it as a whole, by listing the root in `packs`. A one-of in the pack of
which the rule set names no child contributes nothing, like any one-of
with no child enabled; the build fails only when an enabled rule requires
that one-of, as above. When the rule set names a child in `rules`, the
pack goes on into that child's subtree. In the first version that is the
only way into a one-of: a child enabled only because a rule requires it
does not take the pack into its subtree, and a one-of that is a child of
a one-of is never entered. A superseded or repealed rule of the pack is
left out, with the rules below it; a pack whose root is superseded or
repealed enables nothing, and the build warns as it does for such a rule
in `rules`.

## Tracing rules to the constitution

Every rule cites the provisions of the constitution it refines, in the
`authority` field ([Rule file format](#rule-file-format)). That link is
used in both directions:

- When a provision of the constitution is amended, the rules that cite it
  are listed, and a task is opened to check whether each still fits.
- On the site, under each provision of the constitution, the rules that
  cite it are listed, so a reader sees what the provision turns into in
  practice.

**Compatible revisions.** Each rule declares the range of constitution
revisions it is compatible with, in its `constitution` field, as a Python
version specifier such as `">=1"` or `">=1,<4"`. Revisions are whole
numbers, so only the part of the specifier syntax that compares whole
numbers is accepted: one or more clauses joined by commas, each one of
`>=`, `<=`, `>`, `<`, `==` or `!=` followed by a revision, with spaces
allowed around them. `~=1`, `===1`, `==1.*`, `>=1.0`, a bare `1`, an empty
string and a trailing comma are errors. The lowest revision is
nearly always written. The highest is written only when the rule is known
not to work with a later revision, for example because the constitution
has dropped what the rule relied on; an upper bound is never written as a
precaution, as Python packaging has found with upper bounds on
dependencies. When a provision is amended, the rules listed above are
judged one by one: one that still fits is left alone, and one that does
not gets an upper bound, or is rewritten under a new ID if the change
alters what counts as compliant ([IDs and revisions](#ids-and-revisions)).
At compile time the revision the rule set pins must lie inside the range
of every enabled rule; otherwise the build fails and names the rule and
its range.

## Checks

Most rules cannot be decided by a script: a rule a script could decide is
usually a CI check or a lint already. So a rule is kept mainly through
review, and a check serves the review: it settles the few rules a script
can, and hands the reviewer the rest.

There is no language for writing checks, declarative or otherwise. A small
set of built-in check primitives ("this file exists", "this key in this
TOML file has this value") would keep needing one more case, "or", and
"unless", until it became a language of its own that nobody reads well. A
check is code, of one of two kinds, and there is no third.

**Our rules: Rust.** The checks of the rules in this repo are in the
command's crate
([The command and the first version](#the-command-and-the-first-version)),
with one module per rule, named for the rule's ID (`r42.rs` for
rule 42), the way ruff and clippy implement each lint as a function. The
rule's text and metadata stay in `rules/42.toml`, so the person writing a
rule and the one writing its check each change their own file; the
module's test cases sit next to the check, in the crate. Common
operations, such as reading a key from a TOML file, are ordinary helper
functions in the same crate; an operation becomes a helper on its third
repetition, not before (`simplicity`). The crate builds into one binary, so a
downstream user installs one binary and has every check, with no scripts
to deploy. The crate is published to crates.io with its own version, and
publishing it is a release; the rules are released with tags of the
repository ([Releases and versions](amendments-and-releases.md#releases-and-versions)).

A check that needs the state of GitHub, of a pull request or of a CI
run, reads it through the `gh` on the machine, run in the repository,
where `gh api` finds the repository from its GitHub remote. statute never
reads, stores or prints a token: the credential stays in `gh`'s own
configuration. No `gh` on `PATH`, no login, a repository that is not on
GitHub, a call that fails and one that has not answered within 60 s
(stopped like a command past its time, below) are inconclusive, like any
missing tool. What such a check looks at comes from the repository
itself: the pull requests are those GitHub associates with `HEAD` (the
one that merged it into the default branch, or else the open and merged
ones that hold it), and a release is a tag on `HEAD`. A commit not
pushed yet has no pull request, and a `HEAD` without a tag no release,
and both are inconclusive.

When a rule is superseded, its successor gets a new module, copied from
the old one and changed as needed. The old module stays in the code,
marked superseded, and does not run unless asked to.

**Rules written downstream: external commands.** A rule kept in another
source declares its check as a command (`check = "command"`, with `run`
and `tools`), in any language. The script lives in that source's own
repository and runs with its own toolchain; the rule names the tools it
needs (for example, uv or Go). The contract:

- exit code 0: the rule is met; 1: it is violated; anything else: the
  check could not decide;
- standard output: JSON giving, for each finding, the location and the
  reason, as a list of objects `{"location": ..., "reason": ...}`.

The command runs with `sh -c` in the directory of the rule set; when a
change is checked, `STATUTE_BASE` holds the git ref it starts from. A
tool of `tools` that is not on `PATH`, a command that after 10 minutes
has not exited and closed its output (what it started is stopped with
it), an output longer than 1 MiB, and an exit status of 1 whose output
is not that list are inconclusive.

A rule without an automatic check (`check = "review"`) is allowed; it
goes straight to the reviewer (below).

## Outcomes and the review loop

A check has three outcomes: pass, violation, and inconclusive. A missing
tool, a timeout and a script that crashes are inconclusive, never a
violation, so a missing toolchain does not turn the checks red.

The review starts from `statute check`: the reviewer runs it and gets a
list of the enabled rules, the few a script settled with their outcome
and the rest for review, each with its reason and verification, which
the compiled `AGENTS.md` leaves out by default
([Compilation and the site](#compilation-and-the-site)). The reviewer
goes through the list domain by domain and rule by rule: the rules for
review, the inconclusive results and the violations. Whether
one reviewer takes all of them or several take a group each, and how the
groups are formed, is set in the rule set.

In the first version, `statute check` lists the enabled rules of the
repository for review and runs the checks of those that have one; with `--base <ref>` it checks the change
`git diff <ref>...HEAD`. It prints one line per rule, its outcome
(`pass`, `violation`, `inconclusive`, or `review` for a rule without an
automatic check) and a note, with the findings below, and then, for the
reviewer, the reason and the verification of each rule that is
inconclusive or for review, with their citations rendered as in the
compiled `AGENTS.md`, a cited rule always as its title and reference
(`Title (rule R42)`), since the output has no anchors to link to. It
exits with status 1 when a rule is
violated, and 0 otherwise. A built-in check that covers only part of its
rule, finding nothing, is inconclusive, not a pass, since the rest of
the rule is still for the reviewer. The built-in checks:

- rule 46 (instruction-files-apart) tells instruction files, generated
  files and other files apart with the same path tables as
  `check-instruction-changes`, and is violated by a change that mixes
  instruction files with other files; files of agent tools the tables do
  not name, sources of instructions outside them, and paths a repository
  declares in its `AGENTS.md` are left to the reviewer, and without
  `--base` there is no change to check;
- rule 43 (credential-designated-place) runs the gitleaks on the
  machine, with `--redact`, on the commits of the change or on the whole
  history, and is violated by a hit, shown by its place and gitleaks rule
  only; credentials gitleaks does not recognize, and places outside the
  repository, are left to the reviewer, and without gitleaks it is
  inconclusive;
- rule 50 (commit-hook-credential-scan) reads `.pre-commit-config.yaml`,
  and passes when a hook runs gitleaks with `--staged` and `--redact`,
  at the commit stage and for every commit, with no file filter; the
  hooks gitleaks publishes, under its present or its former repository
  name, count by their own entry when their `rev` is v8.1.1 or later.
  A hook whose command statute cannot see (gitleaks' own at an older or
  unreadable `rev`, a hook of another repository, a local hook that runs
  a script), a gitleaks hook without `--redact` or with a file filter,
  another scanner, another hook framework's configuration and YAML
  statute does not read are inconclusive. No hook configuration at all,
  or one whose hooks at the commit stage all show their command and none
  scans the staged changes, is a violation;
- rule 58 (release-first-green-commit) takes a tag on `HEAD` as the
  release, the default branch as the main branch, and as CI the workflow
  holding a check the default branch requires, by branch protection or a
  ruleset; it passes when that workflow's run for the push of `HEAD` to
  the default branch concluded in success on attempt 1, and is violated
  when it did not. Without a tag on `HEAD`, without a required check,
  without such a run (no CI on pushes, or a commit not on the branch) and
  while the run has not finished, it is inconclusive;
- rule 61 (verdict-names-version) reads the verdicts on the pull
  requests of `HEAD`: each comment or review whose text opens with
  `Verdict:`, and each review that approves or asks for changes. A verdict
  without the full SHA of a commit of its pull request is a violation; a
  review also names the commit GitHub records it on;
  verdicts given elsewhere, and whether the SHA named is the version
  reviewed, are left to the reviewer;
- rule 71 (no-automatic-merge) reads the timeline of the pull requests of
  `HEAD`, and is violated when auto-merge was ever enabled on one, even if
  it was disabled again or has merged it; other ways of merging
  automatically, and who enabled it, are left to the reviewer.

The rules 43 and 46 supersede, 3 and 7, keep their modules, which run
only for a rule set that still enables them.

The reviewer runs `statute check`, and CI may run it too. It is not a
commit hook: some checks scan the whole history or
ask the forge over the network and can be slow, and a commit hook holds
only checks that take seconds.

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
rules improve themselves: the few checks a script can run settle what
they can, agents review the rest rule by rule, and what the agents
conclude goes back into the checks.

**Who decides.** Compiling, upgrading, finding conflicts and judging the
compatible range are done by deterministic scripts wherever a script can
decide. What a script cannot decide goes to an agent, through the three
outcomes and the review loop above. People take no part in these
judgments; they appear only where the constitution itself asks for a
person's approval. This follows `automation-first.decide-and-do`.

## Trial rules

When agents doing a piece of work find that no rule, skill or convention
covers it, they start writing the convention down. The first version is a
trial rule (`status = "trial"`): it is used on real work, changed freely
while it is tried, and made a regular rule once it has settled. The rules,
skills and conventions of this repo itself are meant to come about the
same way.

**Becoming active.** Agents decide this, not a person. Once a trial rule
has been used in real work a number of times (5 in the first version)
with no false positive left unresolved in that time, the reviewing agent
sets its `status` to `active`. That changes only the status, not what
counts as compliant, so the rule is edited in place and its revision goes
up by one ([IDs and revisions](#ids-and-revisions)).

## Compilation and the site

**Locally**, the compiler takes the constitution (the version for agents)
and a rule set with its lock file, and writes:

- an `AGENTS.md`, which opens with the repository, the edition and the
  revision of the constitution it rests on, and then has the enabled rules
  in nested blocks along the parent tree, one heading level per level of
  the tree, each domain with an enabled rule below it as a top heading; a
  subtree whose root is not enabled (for a one-of: none of its children
  is) contributes nothing, since none of its rules can be enabled without
  it, and a requirement that crosses subtrees is rendered as a link. By
  default each rule contributes only its statement, to keep the file
  small; the rule set can add the reason or the verification
  ([Rule sets and the lock file](#rule-sets-and-the-lock-file)), and the
  reviewer gets them either way. The file is not split for now. Once the
  compiled file of a repository using it has been measured with
  `count-tokens`, a length threshold is set from that measurement, and a
  file over it is split along domains;
- the skill directories the enabled rules need;
- the version of the check binary to install, and the external commands
  of the enabled rules.

`statute compile` writes the `AGENTS.md` next to `statute.toml`, or to
another file with `--output`, and the other two next to it (below),
and only when the lock file is what `statute lock` would write now. The
`AGENTS.md` holds, in
this order: a comment that the file is generated; a line with the
constitution's edition, revision and source; the constitution's version
for agents in the rule set's language, the file
`constitution/<edition>/generated/constitution.<lang>.agent.md` that the
constitution's build writes, unchanged; and a heading "Rules" with the
enabled rules. Each domain is a heading of level two, and each level of
the tree one level deeper, down to level six, where deeper levels stay;
a one-of none of whose children is enabled does not appear. A node's
heading is its title and its reference as statute shows it (`R42`),
after an anchor of `rule-` and that reference with a colon made a
hyphen: `rule-R42`, `rule-L12`, `rule-team-7`, so the anchors of
different sources never clash. A requirement of a node under another domain is
a line "Requires:" with a link to that anchor; one within the domain is
not rendered. A rule uses its translation into the language when it has
one, and its original otherwise. The citations in a rule's text are
rendered ([IDs](source-formats.md#ids)): `{@rule:<id>}` as the cited
node's title, in the language as its heading is, linked to its anchor
when the node is in the file, and followed by "(rule R42)" otherwise; and
`{@constitution:<id>}` as the provision's label in the language, in plain
text as in the version for agents, such as `Art. IV §1(2)`. The labels
come from the edition's sources, the numbers of the articles' and
sections' names and the items' `number`, by the same rules as in the
constitution's build ([Compilation](compilation.md)); unlike that build,
citations in a row are not shortened (`Art. IV §1, Art. IV §2`). A
citation of a node or provision that does not exist fails the command,
naming the rule and the part. The rules' files and their hashes in the
lock file are unchanged.

In the directory of the `AGENTS.md`, `statute compile` also writes:

- each skill an enabled rule needs ([Rules and skills](#rules-and-skills)),
  copied whole from `skills/<name>/` of the source to
  `.agents/skills/<name>/`, with a file `.statute-generated` added, and a
  link to that copy, `.claude/skills/<name>`, whose target is the
  relative path `../../.agents/skills/<name>`. Each skill has a link of
  its own, never one for the whole directory nor one for `SKILL.md`
  alone; Claude Code reads a repository's skills only from
  `.claude/skills/` (read 2026-10-14). Links are made only on a Unix
  system: elsewhere, a rule set that needs a skill fails the command. A
  skill may hold only directories and regular files;
- `statute-checks.toml`: the version of the statute that compiled it,
  as `statute = "<version>"`, the binary whose built-in checks the
  rules use, and one `[[commands]]` table per enabled rule with
  `check = "command"`, ordered by ID, with its `rule` (`"R42"`),
  `run` and `tools`. The file changes with statute's version.

Each file starts with a comment that statute wrote it (`<!-- ... -->`
in the `AGENTS.md`, `# ...` in `statute-checks.toml`). Before writing anything,
the command checks every place it would write, and when one holds
something it did not write (a file without that first line, a skill
directory without `.statute-generated`, a `.claude/skills/<name>` that
is not its link), it writes nothing at all and fails. When a skill is to
be written, `.agents/skills` and `.claude/skills` must be plain
directories, or absent: a repository that links the whole of
`.claude/skills` to `../.agents/skills` fails the command, which names
the link, since statute writes a link per skill. A skill directory
with `.statute-generated` that the rule set no longer needs is removed,
with its link; nothing else in `.agents/skills/` or `.claude/skills/` is
touched. The same input compiles to the same bytes, links included.
The files are not written all at once: an error midway, such as a full
disk, can leave part of the output, and running the command again
finishes it. The order makes that work: a skill's
`.statute-generated` is written before its files, a link is removed
before its directory, and the `AGENTS.md` is written last.

Compiling, checking and upgrading a rule set are one command
([The command and the first version](#the-command-and-the-first-version)).
The constitution's compiler stays in Python for now
([Compilation](compilation.md)) and moves into that command later.

**The public site** lists every rule, so that people can choose from
the full list, with a filter that leaves superseded and repealed rules
out by default; a superseded rule keeps its page, so links to it keep
working. A rule of this repo is shown as `R42` there too. What the list
and the rule pages show is in [Static site](static-site.md).

## The command and the first version

**The command.** The rule system's command is written in Rust (see
[Why Rust](#why-rust)) and grows in the crate `crates/statute`
([File structure](file-structure.md)). The first version is that one
crate. Its name, `statute`, is held on crates.io by a placeholder release
([Releases and versions](amendments-and-releases.md#releases-and-versions));
it may still change before the first real release, and another name would
be published anew. The rule set is `statute.toml`, the lock file
`statute.lock`, and the commands are:

- `init`: write a new rule set;
- `add` and `remove`: enable a rule or take it out of the rule set, and
  update the lock file;
- `lock`: resolve the rule set into the lock file;
- `compile`: write the compiled output
  ([Compilation and the site](#compilation-and-the-site));
- `check`: list the enabled rules for review, with the outcome of those a
  script checks
  ([Checks](#checks));
- `upgrade`: move the sources and the constitution to newer versions
  ([Rule sets and the lock file](#rule-sets-and-the-lock-file));
- `explain`: say why a rule is enabled and which rules require it.

**The first version** builds the repository level only. The user level,
whose instruction files in the user's home directory the private overlay
installs today, is the second step. The rules of this repo have moved to
the new format, each keeping its earlier word ID as its `slug`. Among the first
rules written in the new format are those for writing a design document,
which try the whole process on real work.

**Built so far**: the commands `init`, `lock`, `explain`, `add`,
`remove`, `upgrade`, `compile` and `check`
(built-in checks for rules 43, 46, 50, 58, 61 and 71, and for 3 and 7,
which 43 and 46 supersede), for one source on disk. Inherited rule sets and path segments are
not built; a git source, more than one source and a reference to
another source, in `requires` or `conflicts` (`"R12"` or `"team:7"`),
are reported as not supported yet. The three kinds of reference are
read and shown as above, but with one source a rule set can use only
the rules of that source: a local rule fails with a note to write
`R12` when the first-party source is the rule set's own directory, and
is reported as not supported yet when it is elsewhere, since the local
rules would then be a second source.

**`init`** writes `statute.toml` in the current directory: the
constitution's source and edition, an empty `rules`, and one source on
disk, the first-party source `R`. `--source <dir>` gives the source's directory, relative
to the current one, `.` by default. `--edition` names the edition;
without it, `init` takes the one directory under the source's
`constitution/` that holds an `index.toml`, and fails when there is none
or more than one, naming those it found. It resolves the rule set before
writing it and writes nothing when that fails. It does not write
`statute.lock`: with no rule enabled there is nothing to pin, and the
first `add` writes it, replacing a `statute.lock` already there. An
existing `statute.toml` is left as it is, and `init` exits with status 1.

**`upgrade`** compares the resolution with `statute.lock`, prints what
changed ([Rule sets and the lock file](#rule-sets-and-the-lock-file)),
or that the lock file is up to date, and writes the lock file as `lock`
does. Without a `statute.lock` there is nothing to compare with, and it
exits with status 1. With `--replace-superseded` it first replaces, in
`rules` and `packs`, each superseded rule with its successor, following
the chain to its end, and keeps the rest of the file as written; when
the successor is listed already, the old entry is only taken out. Where
the chain reaches a rule with more than one successor, the entry is left
as it is and the successors are listed, for the user to choose with
`add` and `remove`. A repealed rule is left for the user to take out. When the changed rule
set does not resolve, neither file is changed.

**Moving over.** Today agents read the user-level instruction files of
the private overlay. These will gain a provision: in a repository that
has a `statute.lock`, the repository's compiled output governs, and the
user-level instruction files are set aside. This repo tries it first. It
switches only once the compiled output covers what those files govern
today that matters most: credentials, the git workflow and review. Until
then the compiled output is built and compared, but not read in their
place. [Rule sources](rule-sources.md#cutover-from-the-hand-written-guide)
replaces this condition with a switch of the whole guide at once, made
when every part of it has become a rule.

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

[Rule file format](#rule-file-format) is implemented, and
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
  on (see [Rule sets and the lock file](#rule-sets-and-the-lock-file)).
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
