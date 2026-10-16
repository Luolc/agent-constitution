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
R = { git = "https://github.com/Luolc/agent-constitution", tag = "<tag>" }

[compile]
lang = "en"
```

The rule set names the constitution's source and edition; the revision
is the one at the commit of that source the lock file pins, and the lock
file records it. `rules` lists the rules chosen directly and `packs` the
roots of the packs enabled, each as a reference
([Sources and references](#sources-and-references)): here `R42` and
`R51` are first-party rules and `12` is one of the repository's own. An
option of a one-of that is a rule is chosen by listing it in `rules`,
here `R51`; one that is a domain, through `domains`
([In the rule set](rule-config.md#in-the-rule-set)). A domain or a
one-of listed in `rules` fails the build, which says to enable the
domain as a pack and the one-of through one of its options.
`lang` in `[compile]` names the language the compiled `AGENTS.md` is
written in, `en` by default. What the file carries of each rule is not
a setting: its reference and its statement, nothing more
([Compilation and the site](statute-command.md#compilation-and-the-site));
`agents-md`, a former key of `[compile]`, fails the build and says so.

[Rule sources](rule-sources.md) adds fields. `public` at the top,
required, says whether the repository is public, and each source but
`R` says `public` of itself, and may have `scope`, `"global"` by
default or `"local"`
([Four kinds of source](rule-sources.md#four-kinds-of-source)). A public
repository names its private sources in a private rule set,
`private.toml` in git's own directory, shared by every worktree of the
clone, which inherits the worktree's `statute.toml` without naming it
and has its own lock file, `private.lock`. That lock file pins only the
private layer's own sources, each by its address, branch or tag and
commit; the two layers together are resolved in memory in each
worktree, from its `statute.lock` and `private.lock`, and never written
down. It compiles to `.statute/AGENTS.full.md`, the
public and private rules in one file
([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)).
[Rule configuration](rule-config.md) adds the directory of the local
rules, parameters and default switches, which it marks where they are
not built yet.

The lock file is TOML, written by statute and never edited by hand. It
records the constitution (its repository, edition, revision and commit),
each source (its short name, address, the branch or tag it follows,
commit and, but for `R`, `public`;
[Git sources](global-configuration.md#git-sources)), and each enabled rule,
whether chosen directly or brought in by a requirement (its reference,
written as the rule set writes it, `R42` or `team:7`, and `L12` for a
local rule; its revision; the SHA-256 of its file; and the SHA-256 of
the rule without its translations, which a changed translation leaves as
it was):

```toml
[constitution]
git = "https://github.com/Luolc/agent-constitution"
edition = "2026-10"
revision = "<revision>"
commit = "<commit>"

[[sources]]
name = "R"
git = "https://github.com/Luolc/agent-constitution"
tag = "<tag>"
commit = "<commit>"

[[rules]]
rule = "R7"
revision = 2
sha256 = "<SHA-256 of rules/7.toml>"
original-sha256 = "<SHA-256 of rule 7 without its translations>"

[[rules]]
rule = "R42"
revision = 1
sha256 = "<SHA-256 of rules/42.toml>"
original-sha256 = "<SHA-256 of rule 42 without its translations>"
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
set when there is one successor. When several rules together take its
place, it leaves the entry to the user, who enables them all, leaving out
any that does not apply; for a rule brought in by another, the warning
names the rule that brings it in. An enabled repealed rule draws a warning too. These are
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
whose original changed while its revision did not draws a warning from
`lock` and a note on its line from `upgrade`, since an edited rule's
revision goes up by one ([IDs and revisions](rule-format.md#ids-and-revisions)).
They compare the rule without its translations: the rule as TOML, less
`[translations]` and the text in every language but the original,
written as JSON with its keys sorted and hashed with SHA-256, so
that a change to the original or to any other field counts, and a change
to a translation or to the file's layout alone does not. A rule whose
translations changed and nothing else draws no warning, and `upgrade`
lists it with its revision unchanged. A lock file written before this
hash was recorded compares the file's SHA-256 instead.

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
The cache of the sources and a global configuration that names
sources for every repository are designed in
[Global configuration and git sources](global-configuration.md); the
personal layer of rules is not.

## Sources and references

A rule is found by its source and its ID. The commit is not part of
that identity.

**Naming a source.** A rule set names each source in a `[sources]`
table, with its address and the tag or branch to follow:

```toml
[sources]
R = { git = "https://github.com/Luolc/agent-constitution", tag = "<tag>" }
team = { git = "example.test/their-team/rules", branch = "main" }
```

The key `R` is reserved for the first-party source: this repo, which
publishes the constitution and the general rules. A rule set declares
it as it declares any other source, with the tag, branch or path to
read, or leaves it to the global configuration and the built-in
default, the first-party repository's `main` branch
([Global configuration and git sources](global-configuration.md#the-global-configuration)). Every other source has a short name the rule
set chooses, of one of two forms: lowercase words joined by hyphens,
such as `team`, or one or two capital letters, such as `P`. A name of
three capital letters or more fails. References to such a source are
capital letters and a number, consistent with `R12`; the project owner
chose the limit of two letters. `L` and `local` are
reserved for the repository's own rules (below), and `ac`, the
first-party source's earlier name, fails with a note to name it `R`.
A source may also be a directory on disk, `R = { path = "." }`,
relative to the rule set's directory: a repository that publishes
rules, as this one does, names itself this way. A source's rules are in
`rules/` at its top, or in the directory `rules-dir` names, relative
to that top, and its constitution in `constitution/<edition>/`:

```toml
[sources]
team = { git = "https://example.test/our-team/tools.git", branch = "main", rules-dir = "agent/rules", public = true }
```

`rules-dir` is for a repository that publishes rules beside other
things and keeps them elsewhere; its parameters' declarations,
`params.toml`, are in the same directory
([Parameters](rule-config.md#parameters)). An absolute path, a path
with `..` and one into `.statute/` fail: a source is read only
within itself, and `.statute/rules/` holds a repository's rules for
itself, which no other repository may use
([Four kinds of source](rule-sources.md#four-kinds-of-source)).

A source may be a git repository as well, cloned into statute's cache
directory and read at the commit the lock file pins ([Git sources](global-configuration.md#git-sources)).
Sources every rule set of a user may use, and the
first-party source's default address, come from the global
configuration, which a rule set's own `[sources]` overrides by name
([Which source a name means](global-configuration.md#which-source-a-name-means));
a rule set that uses only those may leave `[sources]` out.

**Three kinds of reference.** Each kind has a form of its own, so no
reference can be read as another kind:

- a first-party rule: `R` and the ID, `R42`;
- a local rule, one in the repository's own `.statute/rules/`
  ([The directory of local rules](rule-config.md#the-directory-of-local-rules)),
  that is, of the repository that holds the rule set: the ID alone,
  `12`; `L12` and `local:12` mean the same;
- a rule of another source: its short name, a colon and the ID,
  `team:7`; for a source named in capital letters, the letters and the
  ID with no colon, `P7`.

A reference with no colon is read as the capital letters before the
first digit and the ID after them, so `RL12` is rule 12 of source
`RL`, not a first-party or a local rule. `P:7` fails with a note to
write `P7`, and `team7` is no reference.

statute names a rule in one form wherever it shows one: in its
messages, in `statute explain`, in the lock file and in the compiled
`AGENTS.md`. That form is `R42`, `L12`, `P7` or `team:7`: a local rule
is shown as `L12`, so that it never reads like `R12`. The site lists the
first-party rules alone, as `R42`. A rule set keeps each entry as
written, and the commands that add one write `R42`, `12`, `P7` or
`team:7`.

When the first-party source is the rule set's own directory,
`R = { path = "." }`, as in this repo, its `rules/` are the
first-party rules, written and shown as `R12`, and this repo's own
rules are apart from them in `.statute/rules/`. When `local-rules`
names the rules directory of a source on disk, those rules stay the
source's: a local reference to one, such as `12`, fails with a note
to write `R12`, so that one file never has two names.

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
`requires` or `conflicts` as `"R12"`, `"P7"` or `"team:7"`, and in its
text as `{@rule:R12}`, `{@rule:P7}` or `{@rule:team:7}`, with the names
of its own source's
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

- `parent`: every rule has exactly one parent, a one-of or a domain
  (below), never another rule, and every one-of a domain, so the rules
  form a tree under the domains. A rule that overrides a rule of another source
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
service. It sits in the Python domain, requires the uv rule, which it
refines, and requires the one-of "CI service", satisfied by any of its
options, such as a GitHub Actions rule. A rule that refines another is
never its child ([No rule under a rule](rule-config.md#no-rule-under-a-rule)).

**Domains.** A domain is a node at the top of the tree, such as Python,
CI, Git or security. Besides its ID and title it has only the few fields
every node has and an optional applicability, a sentence or two on which
repositories it is for ([Rule file format](rule-format.md#rule-file-format)), and no text or
check; it cannot be enabled on its own. When any rule
below it is enabled, the domain's rules appear in the compiled output,
together, without the domain. A
domain may sit under a domain, as a subdomain, to any depth
([Subdomains](rule-sources.md#subdomains)).

**One-ofs.** A one-of is a node of the parent tree, under a domain, of
which at most one child, an option, is enabled within one rule set (or
one path segment of it). Enabling none of its options is allowed, as in
a repository with no Python at all; "one of" here never means "exactly
one". The one-of "Python package manager" under Python has the options
uv, Poetry and Bazel. The options of a one-of are all rules or all
domains: an option that needs several rules is a domain under the
one-of. A
one-of has an ID, and adding one goes through review like adding a
rule. A rule that requires a one-of is satisfied when any one of its
options is enabled. Every one-of names a default option
([A one-of's default](rule-config.md#in-the-rule-files)), which statute
reads; a pack and a requirement take it where the rule set names
none, in place of contributing nothing or failing as below.

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
along `requires`, directly or through other rules, since
enabling the one would enable the other; the build fails on it. Sibling
rules under one parent may conflict.

The build checks these constraints:

- No cycle, in the tree or in the requirements, within one source or
  across sources.
- A domain has no parent, or, as a subdomain, a domain, or, as an
  option, a one-of; every rule and one-of has one, except a
  rule with `overrides`. A rule's parent is a domain or a one-of, a
  one-of's a domain, and a one-of's options are all rules or all
  domains ([No rule under a rule](rule-config.md#no-rule-under-a-rule)).
- A parent requires nothing: enabling a rule enables no other rule
  along the tree. A one-of counts as enabled when one of its options
  is.
- No redundant requirement: if A requires B and B requires C, A does not
  also list C.
- A requirement that crosses to another subtree should name a one-of, not
  a particular rule. The number of such requirements has no hard limit;
  the build warns about them.
- At most one child of a one-of is enabled in a rule set or path segment,
  and no two enabled rules conflict. A rule that brings in two options of
  one one-of, along `requires`, could never be enabled, and
  the build fails on it; so it does on a rule that requires a one-of with
  no options yet.
- A rule in force that requires a superseded or repealed rule draws a
  warning, which names the successors when there are any. A rule that is
  itself superseded or repealed requires nothing any more, and draws none.
  In the source `statute validate` checks, it is also an error, as is
  any other way a node in force names a superseded or repealed node
  ([Checkers](checkers.md#checkers)).

**Missing requirements are added.** When a rule set enables a rule
without one it requires, statute adds the missing rule, as a package
manager adds a dependency: the rule set lists only the rules chosen
directly, the lock file records the ones brought in, and an upgrade's
diff shows them. Among missing requirements, the one case that fails
is a requirement of a one-of when the rule set has enabled none of its
options and the one-of names no default: statute does not choose for
the user, so it fails and lists the options to choose from. A one-of
that names a default brings in its default option.

Packs: a pack is a subtree, named by its root, and a rule set can enable
it as a whole, by listing the root in `packs`. A one-of in the pack of
which the rule set names no option contributes nothing, like any one-of
with no option enabled; the build fails only when an enabled rule requires
that one-of, as above. When the rule set names an option, the pack
enables it. In the first version that is the only way into a one-of: an
option enabled only because a rule requires it does not take the pack
into it. Where the rule set names none, the pack takes the one-of's
default option. A superseded or repealed rule of the pack is
left out, with the rules below it; a pack whose root is superseded or
repealed enables nothing, and the build warns as it does for such a rule
in `rules`.
