# Rule sets, references and dependencies

Part of the [Rule system](rule-system.md), whose status and terms
apply here.

## Rule sets and the lock file

A rule set lists rule IDs (and packs, below). Its lock file records, for
each enabled rule, the revision in use and the commit of the source it came
from, so the same rule set and lock file always build the same output.

The rule set is `statute.toml` and the lock file `statute.lock`, at the
root of the repository (named for the command,
[The command and the first version](statute-command.md#the-command-and-the-first-version)). The
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
parts in as well ([Compilation and the site](statute-command.md#compilation-and-the-site)).
`lang` in `[compile]` names the language it is written in, `en` by
default.

[Rule sources](rule-sources.md) adds fields. `public` at the top,
required, says whether the repository is public, and each source but
`R` says `public` of itself, and may have `scope`, `"global"` by
default or `"local"`
([Four kinds of source](rule-sources.md#four-kinds-of-source)); these are
built. Not built yet: a public repository names its private sources in `statute.private.toml`,
which git ignores, inherits `statute.toml` through
`base = "statute.toml"` and has its own lock file,
`statute.private.lock`. It compiles to `.statute/AGENTS.full.md`, the
public and private rules in one file
([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)).

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
anew. A rule that an enabled rule overrides keeps its line, with one
more field, `overridden-by = "L3"`, naming the rule that overrides it
directly (in a chain, the next link), so that `upgrade` still shows its
changes ([Overrides](rule-sources.md#overrides)). A rule that only an
override brings in, which nothing else enables, has its line as well.

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
revision goes up by one ([IDs and revisions](rule-format.md#ids-and-revisions)). The
two cannot tell a changed translation from a changed original, so either
draws it.

**Bound to one constitution.** A rule set names the constitution it
rests on: the repository it comes from, the edition and the revision.
The lock file pins that revision as it pins a source's commit, and one
compilation uses exactly one revision of one constitution. Rule sets
that rest on different constitutions are never combined: at least the
repository and the edition must agree, and the revision is checked
against each rule (see
[Tracing rules to the constitution](rule-format.md#tracing-rules-to-the-constitution)).
When the constitution is amended, a rule set pinned to the older
revision does not change; moving it to the new revision is an explicit
upgrade, like any other. A rule set that inherits a base rule set may pin
another revision than the base does. The build then takes the newer of
the two, and fails if some enabled rule's range cannot hold it.

**Per repository, with inheritance.** A rule set belongs to a repository,
and it may inherit a shared base rule set, named in its `base` field
(not built yet), so the rules every repository
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
([The command and the first version](statute-command.md#the-command-and-the-first-version)).

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
`constitution/<edition>/`. statute reads sources on disk only so far,
and reports a `git` source as not supported yet.

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
`requires` or `conflicts` as `"R12"` or `"team:7"`, and in its text as
`{@rule:R12}` or `{@rule:team:7}`, with the names of its own source's
`[sources]` table; statute resolves them with the rule set's names so
far. A rule of the first-party source
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

## Dependencies, one-ofs and packs

Rules are related in two ways, and the tree has two more kinds of node,
domains and one-ofs:

- `parent`: every rule has exactly one parent, either another rule, a
  one-of or a domain (below), so the rules form a tree, each domain
  refined level by level. A rule that overrides a rule of another source
  has none and stands in that rule's place
  ([Overrides](rule-sources.md#overrides)). The tree decides where a rule stands in the
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
every node has and an optional applicability, a sentence or two on which
repositories it is for ([Rule file format](rule-format.md#rule-file-format)), and no text or
check; it cannot be enabled on its own. When any rule
below it is enabled, the domain appears in the compiled output as a heading. A
domain may sit under a domain that has no parent, as a subdomain, so
domains nest two levels deep and no deeper
([Subdomains](rule-sources.md#subdomains); not built yet); a finer
grouping is a rule or a one-of under the domain.

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
[IDs and revisions](rule-format.md#ids-and-revisions). When both rules are sound and
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

- No cycle, in the tree or in the requirements, within one source or
  across sources.
- A domain has no parent, or, as a subdomain, a domain that has none;
  every rule and one-of has one, except a rule with `overrides`.
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
