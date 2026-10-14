# Rule system

Status: draft. The rules of this repo are written in the
[Rule file format](#rule-file-format), which `constitution-build` checks,
and the tool reads them and a rule set and writes the lock file
([The tool and the first version](#the-tool-and-the-first-version)); the
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
- [The tool and the first version](#the-tool-and-the-first-version)
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
  this one; present only when `status` is `superseded`.
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
root of the repository (named for the tool,
[The tool and the first version](#the-tool-and-the-first-version)). The
rule set lists only the rules chosen directly:

```toml
constitution = { source = "ac", edition = "2026-10" }
rules = ["ac:42", "ac:51"]
packs = ["ac:3"]

[sources]
ac = { git = "github.com/Luolc/agent-constitution", tag = "<tag>" }

[compile]
agents-md = ["statement"]
```

The rule set names the constitution's source and edition; the revision
is the one at the commit of that source the lock file pins, and the lock
file records it. `rules` lists the rules chosen directly and `packs` the
roots of the packs enabled, each as a reference
([Sources and references](#sources-and-references)). A child of a one-of
is chosen by listing it in `rules`, here `ac:51`; there is no other
field for it. A domain or a one-of listed in `rules` fails the build,
which says to enable the domain as a pack and the one-of through one of
its children. `[compile]` sets what the compiled `AGENTS.md` carries of each rule:
by default only `statement`; adding `reason` or `verification` puts those
parts in as well ([Compilation and the site](#compilation-and-the-site)).

The lock file is TOML, written by the tool and never edited by hand. It
records the constitution (its repository, edition, revision and commit),
each source (its short name, address and commit), and each enabled rule,
whether chosen directly or brought in by a requirement (its source, ID,
revision and the SHA-256 of its file):

```toml
[constitution]
repository = "github.com/Luolc/agent-constitution"
edition = "2026-10"
revision = "<revision>"
commit = "<commit>"

[[sources]]
name = "ac"
address = "github.com/Luolc/agent-constitution"
commit = "<commit>"

[[rules]]
source = "ac"
id = 42
revision = 1
sha256 = "<SHA-256 of rules/42.toml>"

[[rules]]
source = "ac"
id = 7
revision = 2
sha256 = "<SHA-256 of rules/7.toml>"
```

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
name = "ac"
path = "."
```

Upgrading is an explicit step that rewrites the lock file and shows the
diff of every rule whose revision moved, and of every rule a requirement
brought in or dropped. When the rule set enables a superseded rule, the
build warns and names its successor. For a rule listed in `rules`, one
command replaces the old ID with the new one in the rule set; for a rule
brought in by another, the warning names the rule that brings it in. An
enabled repealed rule draws a warning too. These are warnings, not
errors, since the rule set keeps working as it did. Downstream users do not have to
follow the supersession chain by hand, and nothing changes under them
until they upgrade.

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
on its own schedule. The first version builds the repository level only
([The tool and the first version](#the-tool-and-the-first-version)).

## Sources and references

A rule is found by its source and its ID. The commit is not part of
that identity.

**Naming a source.** A rule set gives each source a short local name, in
a `[sources]` table, with its address and the tag or branch to follow:

```toml
[sources]
ac = { git = "github.com/Luolc/agent-constitution", tag = "<tag>" }
team = { git = "example.test/their-team/rules", branch = "main" }
```

A source may also be a directory on disk, `ac = { path = "." }`, relative
to the rule set's directory: a repository that holds its own rules, as
this one does, names itself this way. Its rules are in `rules/` below
that directory and its constitution in `constitution/<edition>/`. The
first version reads only one source, on disk, and reports a `git`
source as not supported yet.

A reference is the name, a colon and the rule's ID: `ac:42`, `team:7`.
The name means something only inside the file that defines it, so two
rule sets may use `ac` for different sources. When a source moves to
another address, one line of the rule set changes and no reference does.
A source that references rules of other sources has its own `[sources]`
table in the same shape, and its names need not agree with those of any
rule set that uses it. The build matches sources by address, not by
name: the `ac` of one file and the `core` of another are the same source
when both point to the same address.

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
another under `requires`, such as `"ac:12"`. The source of the requiring
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
at least one of them is in the rule set. A repository's `AGENTS.md` keeps
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
one's own side anyway. The tool derives the other side, and both the
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
  which names the successor when there is one.

**Missing requirements are added.** When a rule set enables a rule
without one it requires, the tool adds the missing rule, as a package
manager adds a dependency: the rule set lists only the rules chosen
directly, the lock file records the ones brought in, and an upgrade's
diff shows them. This applies equally to a parent that counts as a
requirement. Among missing requirements, the one case that fails is a
requirement of a one-of when the rule set has enabled none of its
children: the tool cannot choose for
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
version specifier such as `">=1"` or `">=1,<4"`. The lowest revision is
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

There is no language for writing checks, declarative or otherwise. A small
set of built-in check primitives ("this file exists", "this key in this
TOML file has this value") would keep needing one more case, "or", and
"unless", until it became a language of its own that nobody reads well. A
check is code, of one of two kinds, and there is no third.

**Our rules: Rust.** The checks of the rules in this repo are in the
tool's crate ([The tool and the first version](#the-tool-and-the-first-version)),
with one module per rule, named for the rule's ID (`r42.rs` for
rule 42), the way ruff and clippy implement each lint as a function. The
rule's text and metadata stay in `rules/42.toml`, so the person writing a
rule and the one writing its check each change their own file; the
module's test cases sit next to the check, in the crate. Common
operations, such as reading a key from a TOML file, are ordinary helper
functions in the same crate; an operation becomes a helper on its third
repetition, not before (`simplicity`). The crate builds into one binary,
released with the rules, so a downstream user installs one binary and has
every check, with no scripts to deploy.

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
  reason.

A rule without an automatic check (`check = "review"`) is allowed; it
goes straight to the reviewer (below).

## Outcomes and the review loop

A check has three outcomes: pass, violation, and inconclusive. A missing
tool, a timeout and a script that crashes are inconclusive, never a
violation, so a missing toolchain does not turn the checks red.

The automatic checks run first, because they cost least. Then the reviewer
looks at the violations, the inconclusive results and the rules that have
no automatic check. The material the reviewer is given holds each rule's
reason and verification, which the compiled `AGENTS.md` leaves out by
default ([Compilation and the site](#compilation-and-the-site)). Whether
one reviewer takes all of them or several take a group each, and how the
groups are formed, is set in the rule set.

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
  of the enabled rules;
- the hook configuration that runs the checks.

Compiling, checking and upgrading a rule set are one command-line tool
([The tool and the first version](#the-tool-and-the-first-version)).
The constitution's compiler stays in Python for now
([Compilation](compilation.md)) and moves into that tool later.

**The public site** lists every rule that is not superseded, so that
people can choose from the full list. Its "Rules" section is a list,
ordered along the parent tree, of 10 to 20 rules a page, each showing
only its statement, cut short when it is long. Each rule has a page of
its own with its reason and verification, its check, its requirements and
conflicts (both sides of each conflict), the provisions of the
constitution it cites, and all its metadata. A superseded rule keeps its
page, so links to it keep working, but it is left out of the lists by
default.

## The tool and the first version

**The tool.** The rule system's tool is written in Rust (see
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
- `check`: run the checks of the enabled rules
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

**Built so far**: the commands `lock`, `explain`, `add` and `remove`,
for one source on disk; the other commands exit with status 2 and say
they are not implemented yet. Inherited rule sets and path segments are
not built; a git source, more than one source and a reference to
another source, in `requires` or `conflicts` (`"team:7"`), are reported
as not supported yet.

**Moving over.** Today agents read the user-level instruction files of
the private overlay. These will gain a provision: in a repository that
has a `statute.lock`, the repository's compiled output governs, and the
user-level instruction files are set aside. This repo tries it first. It
switches only once the compiled output covers what those files govern
today that matters most: credentials, the git workflow and review. Until
then the compiled output is built and compared, but not read in their
place.

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
  directories, checks and hook configuration, and the compiled file opens
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
- Path segments for monorepos, settled once it has been measured whether
  each supported agent reads the instruction file of a subdirectory it
  works in.
- Rules at the user level: where a rule whose default level is the user
  level is chosen, where it is locked and where it is compiled to.
- The tool's final name, chosen before its first real release. `statute`
  is held by a placeholder release; another name would be published
  anew.
