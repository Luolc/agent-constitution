# The statute command

Part of the [Rule system](rule-system.md), whose status and terms
apply here.

## Compilation and the site

**Locally**, the compiler takes the constitution (the version for agents)
and a rule set with its lock file, and writes:

- an `AGENTS.md`, which opens with the repository, the edition and the
  revision of the constitution it rests on, and then has the enabled rules
  in nested blocks along the parent tree, one heading level per level of
  the tree, each domain with an enabled rule below it as a top heading,
  followed by the domain's applicability when it has one; a
  subtree whose root is not enabled (for a one-of: none of its children
  is) contributes nothing, since none of its rules can be enabled without
  it, and a requirement that leads out of the requiring rule's nearest
  domain, a subdomain when the rule is under one, is rendered as a link. By
  default each rule contributes only its statement, to keep the file
  small; the rule set can add the reason or the verification
  ([Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file)), and the
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
enabled rules. In a public repository it ends with one more line, which
tells an agent to read `.statute/AGENTS.full.md` when it exists, the
public and private rules together, which the private layer compiles to
([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository);
not built yet). Each domain is a heading of level two, a subdomain one of
level three ([Subdomains](rule-sources.md#subdomains); not built yet),
and each level of the tree one level deeper, down to level six, where
deeper levels stay;
a one-of none of whose children is enabled does not appear. The
first-party source's domains come first, then those of the other
sources by the sources' names, then the repository's own. A node's
heading is its title and its reference as statute shows it (`R42`),
after an anchor of `rule-` and that reference with a colon made a
hyphen: `rule-R42`, `rule-L12`, `rule-team-7`, so the anchors of
different sources never clash. An overriding rule
([Overrides](rule-sources.md#overrides)) stands at the place of the rule
it overrides, with the anchors of every rule of the chain before its
heading, so that a link to `#rule-R2` lands on it, and the children of
every rule of the chain below it. Under its heading, a line names what
it overrides: `Overrides R2.`, or along a chain
`Overrides team:7, which overrides R2.` (in Chinese
`覆盖 team:7，后者覆盖 R2。`). A requirement on an overridden rule, and a
citation of one, render as the rule in its place. A requirement of a node outside the
requiring rule's nearest domain, or subdomain, is a line "Requires:"
with a link to that anchor; one within it is not rendered. A rule uses its translation into the language when it has
one, and its original otherwise. A domain's applicability, when it has
one ([Rule file format](rule-format.md#rule-file-format)), is a paragraph
under its heading, in the same language as the heading, its citations
rendered as in a rule's text. The citations in a rule's text are
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

- each skill an enabled rule needs ([Rules and skills](rule-format.md#rules-and-skills)),
  copied whole from `skills/<name>/` of the rule's source to
  `.agents/skills/<name>/` (two needed skills of one name, from two
  sources, fail the command), with a file `.statute-generated` added, and a
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
in the `AGENTS.md`, `# ...` in `statute-checks.toml`). Before writing
anything, the command checks every place it would write, and when one holds
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

A `CLAUDE.md` at the top of the repository, a file or a link to
`AGENTS.md`, draws a warning on every `statute compile`, with or without
a private layer: it is no longer needed, since Claude Code reads
`AGENTS.md` when there is no `CLAUDE.md`
([Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file),
Monorepos), and with the private output it makes Claude Code read the
public rules twice
([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)).
The warning says to delete it, and that a migration will rename it to a
legacy copy, asking the user first. statute neither deletes nor renames
it. Not built yet.

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
- `check`: list the enabled rules for review by domain, with the outcome
  of those a script checks ([Checks](rule-checks.md#checks));
- `upgrade`: move the sources and the constitution to newer versions
  ([Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file));
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
which 43 and 46 supersede), for sources on disk, as many as
`[sources]` names. The `rules/` next to `statute.toml` are the
repository's own rules, `L12`, when no source is at `.`; with
`R = { path = "." }` they are first-party, and a local reference fails
with a note to write `R12`. A rule names a rule of another source in
`requires`, `conflicts` and `{@rule:...}` as `R12` or `team:7`, with
the names of the rule set's `[sources]`: a source's own `[sources]`
table ([Sources and references](rule-sets.md#sources-and-references))
is not read yet. Of [Rule sources](rule-sources.md), `public` is built,
with which rules may cite which in `requires`, `conflicts`, `overrides`
and `{@rule:...}`, and the check of `public` against GitHub
([Checks](rule-checks.md#checks)). Overrides are built, with `scope`
and the ranks, except the check of the private layer of a public
repository, which comes with `statute.private.toml`. Inherited rule
sets, path segments and git sources are not built, nor
`statute.private.toml` and its output, and subdomains.

**`init`** writes `statute.toml` in the current directory: the
constitution's source and edition, `public = true` or `public = false`,
an empty `rules`, and one source on disk, the first-party source `R`.
It takes `--public` or `--private`, one of them and not both. `--source <dir>` gives the source's directory, relative
to the current one, `.` by default. `--edition` names the edition;
without it, `init` takes the one directory under the source's
`constitution/` that holds an `index.toml`, and fails when there is none
or more than one, naming those it found. It resolves the rule set before
writing it and writes nothing when that fails. It does not write
`statute.lock`: with no rule enabled there is nothing to pin, and the
first `add` writes it, replacing a `statute.lock` already there. An
existing `statute.toml` is left as it is, and `init` exits with status 1.

**`upgrade`** compares the resolution with `statute.lock`, prints what
changed ([Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file)),
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
user-level instruction files are set aside. Every repository they
govern switches at once, when every part of them has become a rule and
the conditions of
[Cutover from the hand-written guide](rule-sources.md#cutover-from-the-hand-written-guide)
hold; no topic switches first. Until then the compiled output is built
and compared, but not read in their place.

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
