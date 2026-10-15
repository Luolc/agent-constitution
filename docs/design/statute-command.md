# The statute command

Part of the [Rule system](rule-system.md), whose status and terms
apply here.

## Compilation and the site

**Locally**, the compiler takes the constitution (the version for agents)
and a rule set with its lock file, and writes:

- an `AGENTS.md`, which opens with the repository, the edition and the
  revision of the constitution it rests on, and then has the enabled
  rules as one flat list, each rule an item of its reference and its
  statement and nothing more (below). The file is not split for now. Once the
  compiled file of a repository using it has been measured with
  `count-tokens`, a length threshold is set from that measurement, and a
  file over it is split along domains;
- the skill directories the enabled rules need;
- the version of the check binary to install, and the external commands
  of the enabled rules.

The `AGENTS.md` holds only the enabled rules whose `uses` has
`instructions` ([Where a rule is used](rule-config.md#where-a-rule-is-used));
once parameters are built, each placeholder is replaced by its value
([Parameters](rule-config.md#parameters)).

`statute compile` writes the `AGENTS.md` next to `statute.toml`, or to
another file with `--output`, and the other two next to it (below),
and only when the lock file is what `statute lock` would write now. The
`AGENTS.md` holds, in
this order: a comment that the file is generated; a line with the
constitution's edition, revision and source; the constitution's version
for agents in the rule set's language, the file
`constitution/<edition>/generated/constitution.<lang>.agent.md` that the
constitution's build writes, unchanged; and a heading "Rules" with the
enabled rules. In a public repository it ends with one more line, for
an agent that finds no link to the full text the private layer compiles
to ([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)):
"If `.statute/AGENTS.full.md` exists, read it and follow it: it is this
file with the repository's private rules added." (in Chinese
「如果 `.statute/AGENTS.full.md` 存在，读它并照做：它是本文件加上本仓库的私有规定。」).

`AGENTS.md` is for agents, as is the private full text,
`.statute/AGENTS.full.md`, built the same way, and each is always a
flat list. Each enabled
rule is one item, `- <reference> <statement>`, such as
`- R22 Keep a credential out of every message.`: the rule's reference
as statute shows it (`R42`, `L12`, `team:7`), a space, and its
statement, a line after the first indented by two spaces to continue
the item. Nothing else goes in: no title, reason or verification of a
rule, no domain, subdomain or one-of, neither its title nor its
applicability, and no anchor or link. The relations and settings
between the nodes, `requires`, `conflicts`, one-ofs, `overrides`, the
defaults, `uses` and `parent`, are metadata: statute checks them when it
locks and compiles, and once the rule set compiles they have done their
work and stay out of the file. Titles, reasons, verifications and the
domains' applicability are for people, on the site (below); the
reviewer gets a rule's reason and verification from `statute check`.
The references stay, at the start of each item, so that a review's
findings and a citation can name the rule.

The items follow the tree, a node before its children, so that related
rules stay together: the first-party source's domains first, then those
of the other sources by the sources' names, then the repository's own;
below a node, the first-party source's children first, then those of
the other sources by name, then the repository's own, each source's by
ID. A one-of contributes the rules of its enabled option, the option's
own rules when it is a domain; a subtree with no enabled rule
contributes nothing. An overriding rule
([Overrides](rule-sources.md#overrides)) stands at the place of the rule
it overrides, which is not in the file. A rule uses its translation
into the language when it has one, and its original otherwise. The
citations in a statement are rendered ([IDs](source-formats.md#ids)):
`{@rule:<id>}` as the cited rule's reference, `R42`, whether the rule is
in the file or not, and as the rule in its place when it is overridden;
and `{@constitution:<id>}` as the provision's label in the language, in
plain text as in the version for agents, such as `Art. IV §1(2)`. The labels
come from the edition's sources, the numbers of the articles' and
sections' names and the items' `number`, by the same rules as in the
constitution's build ([Compilation](compilation.md)); unlike that build,
citations in a row are not shortened (`Art. IV §1, Art. IV §2`). A
citation of a node or provision that does not exist fails the command,
naming the rule and the part. The rules' files and their hashes in the
lock file are unchanged.

In the directory of the `AGENTS.md`, `statute compile` also writes:

- each skill an enabled rule needs ([Rules and skills](rule-format.md#rules-and-skills)),
  copied whole from `skills/<name>/` of the rule's source (for the
  repository's own rules, the `skills/` at the top of the repository,
  unless a source is that directory, as with `R = { path = "." }`: those
  skills are then the source's, and the repository's own rules have none) to
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
  `run` and `tools`. The file changes with statute's version. It stays
  at the top of the repository rather than in `.statute/`: it is
  committed, CI reads it, and a change to it is reviewed beside the
  `AGENTS.md` it comes from.

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

A `CLAUDE.md` next to the `AGENTS.md` written (the top of the
repository, unless `--output` names another place), a file or a link to
`AGENTS.md`, draws a warning on every `statute compile`, with or without
a private layer. The warning asks whether to delete it, and says why in
a few short sentences: Claude Code reads `AGENTS.md` itself when there
is no `CLAUDE.md`
([Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file),
Monorepos), so a `CLAUDE.md` is not needed to point to it; with the
private output, where `CLAUDE.local.md` gives Claude Code the full text
in place of `AGENTS.md`, Claude Code reads a `CLAUDE.md` as well, so its
content comes on top of the rules, the public rules twice for a link to
`AGENTS.md` ([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)),
a sentence the warning leaves out without a private layer; and the
migration ([statute-migrate](../../skills/statute-migrate/SKILL.md),
step e) renames a `CLAUDE.md` that is a file to a legacy copy, and
asks the user whether to delete one that is a link to `AGENTS.md`.
statute neither deletes nor renames it. Not built yet: the warning
still says that a migration renames either kind.

Compiling, checking and upgrading a rule set are one command
([The command and the first version](#the-command-and-the-first-version)).
The same command builds the constitution, `statute constitution
build`.

**The public site** lists every rule, so that people can choose from
the full list, with a filter that leaves superseded and repealed rules
out by default; a superseded rule keeps its page, so links to it keep
working. A rule of this repo is shown as `R42` there too. What the list
and the rule pages show is in [Static site](static-site.md).

## The command and the first version

**The command.** The rule system's command is written in Rust (see
[Why Rust](#why-rust)) and grows in the crate `crates/statute`
([File structure](file-structure.md)). The first version is that one
crate. Its name is `statute`: on crates.io, 0.0.1 is a placeholder that
held the name, and the command starts with 0.1.0
([Releases and versions](amendments-and-releases.md#releases-and-versions)).
The rule set is `statute.toml`, the lock file
`statute.lock`, and the commands are:

- `init`: write a new rule set; with `--global`, the global
  configuration ([Global configuration and git sources](global-configuration.md));
- `source add`: name a source in the rule set, its private layer or
  the global configuration
  ([Adding a source](global-configuration.md#adding-a-source));
- `add` and `remove`: enable a rule or take it out of the rule set, and
  update the lock file; with `--private`, of the private rule set
  ([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository));
- `lock`: resolve the rule set into the lock file;
- `compile`: write the compiled output
  ([Compilation and the site](#compilation-and-the-site)); with
  `--private-only`, only the private output, for a git hook in a new
  worktree, without the network and never failing
  ([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository),
  A new worktree);
- `check`: list the enabled rules for review by domain, with the outcome
  of those a script checks ([Checks](rule-checks.md#checks));
- `upgrade`: move the sources and the constitution to newer versions
  ([Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file));
- `explain`: say why a rule is enabled and which rules require it;
- `site`: write a local site of the rules the rule set can draw on
  ([The local site](#the-local-site));
- `export`: print the constitution and the rules of a directory as JSON,
  for a program that reads them ([Export](#export));
- `constitution build`: build the constitution of the repository it runs
  in from its sources, or check it (below);
- `validate <dir>`: check a directory of rule files on its own, as one
  source, without a rule set (below).

**The first version** builds the repository level only. The user level,
whose instruction files in the user's home directory the private overlay
installs today, is the second step. The rules of this repo have moved to
the new format, each keeping its earlier word ID as its `slug`. Among the first
rules written in the new format are those for writing a design document,
which try the whole process on real work.

**Built so far**: the commands `init`, `lock`, `explain`, `add`,
`remove`, `upgrade`, `compile` and `check`
(built-in checks for rules 43, 46, 50, 58, 61 and 71, and for 3 and 7,
which 43 and 46 supersede), `site`, `constitution build` and
`validate`, for sources on disk and git
sources, as many as
`[sources]` names, and `export`. The repository's own rules, `L12`, are read from
`.statute/rules/`, or the directory `local-rules` names
([The directory of local rules](rule-config.md#the-directory-of-local-rules));
when that is a source's rules directory, as with `local-rules = "rules"`
and `R = { path = "." }`, they are that source's, and a local reference
fails with a note to write `R12`. A rule names a rule of another source in
`requires`, `conflicts` and `{@rule:...}` as `R12` or `team:7`, with
the names of the rule set's `[sources]`: a source's own `[sources]`
table ([Sources and references](rule-sets.md#sources-and-references))
is not read yet. Of [Rule sources](rule-sources.md), `public` is built,
with which rules may cite which in `requires`, `conflicts`,
`overrides`, `parent` and `{@rule:...}`, and the check of `public` against GitHub
([Checks](rule-checks.md#checks)). Overrides are built, with `scope`
and the ranks, and so are subdomains, a rule's or a domain's `parent`
in another source, and the private layer with its lock file in git's
directory, its output, `.statute/.gitignore` and
`compile --private-only`.
The global configuration, `init --global`, git sources and `source add`
are built ([Global configuration and git sources](global-configuration.md)),
except that `upgrade` does not show the text of a changed rule yet.
Of [Rule configuration](rule-config.md), the rule files' `default`,
`uses` and `check = "none"` are read and checked, with the tree of
[No rule under a rule](rule-config.md#no-rule-under-a-rule), and
`uses` decides what `compile` writes and `check` lists, and the
default switches of a rule set are built, with `domains`, `exclude`,
`local-rules` and a source entry's `defaults`, and what `add`,
`remove`, `explain` (`--defaults` included) and the lock file do with
them; `default` is required where the rule files need it, and
`check = "review"` fails; parameters are built, with their values in
the lock file, `upgrade`, `explain <name>` and both outputs; the marks
of both sites, the public site's pages of parameters, the one-of's place
in the compiled `AGENTS.md` and the one-of as a box on the sites are
built.
Inherited rule sets and path segments are not built.

**`init`** writes `statute.toml` in the current directory: the
constitution's source and edition, `public = true` or `public = false`,
an empty `rules`, and, with `--source`, one source on disk, the
first-party source `R`.
`public` is what `--public` or `--private` says (one of them at most),
or else what `gh`
shows for the repository, as in
[Public or not](rule-sources.md#public-or-not). `--source <dir>` gives the source's directory, relative
to the current one. Without it, `init` writes no `[sources]`,
and `R` is the global configuration's or the built-in default
([Which source a name means](global-configuration.md#which-source-a-name-means)),
read at the head of the branch it follows. `--edition` names the edition;
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

**`validate <dir>`** reads the rule files of one directory as the
first-party source, with its parameters in `params.toml`, and runs
every check that needs no rule set ([Checkers](checkers.md#checkers)):
the format of each node and parameter, the tree, the targets of its
references, the translations, citations and placeholders, and the
defaults. Each finding goes to standard error on a
line of its own; an error makes it exit with status 1, and a clean run
prints one line to standard output with the number of nodes. When
`<dir>` has a `constitution/` beside it, it also reads that constitution
and checks what the constitution decides of the rules: each node's
status against `index.toml`'s, each `{@constitution:...}` citation and
each `authority` entry, with the warning on an `authority` that names a
provision and one above it; a constitution that does not load is one
error, and its own errors are left to `constitution build`. With
`--base <git-ref>` it also runs the rules part of check 7: when the
numbering is compared, every node of the base still exists, a rule of
the older format as the node whose slug is its name, and what was
repealed or superseded still is; it skips the comparison, with the
note, where check 7 does, and without a `constitution/` beside `<dir>`
it fails. These findings are `rules/7.toml:12: error: <message>`.
This repo runs `statute validate rules` as a commit hook, and CI runs
it with `--base` too.

**`constitution build`** checks the sources of the one edition under
`constitution/` and writes the full version and the version for agents
in each language to its `generated/`, whose first line names the
command that wrote them, `cargo run -p statute -- constitution build`;
with `--check` it writes nothing and fails when a committed file
differs from a fresh build; with `--check --base <git-ref>` it also
compares the numbering and the revision with that commit; and `--root
<dir>` runs it on another checkout than the current directory. Its
checks are checks 1 to 7 of [Checkers](checkers.md#checkers) as far as
they concern the constitution. It does not read `rules/`: what the
rules say of the constitution, and the rules a base commit had, are
`validate`'s (above). This repo runs `constitution build --check` as a
commit hook, and CI runs it with `--base` too.

**Moving over.** Today agents read the user-level instruction files of
the private overlay. These will gain a provision: in a repository that
has a `statute.lock`, the repository's compiled output governs, and the
user-level instruction files are set aside. Every repository they
govern switches at once, when every part of them has become a rule and
the conditions of
[Cutover from the hand-written guide](rule-sources.md#cutover-from-the-hand-written-guide)
hold; no topic switches first. Until then the compiled output is built
and compared, but not read in their place.

## The local site

`statute site` writes a site of the rules a rule set can draw on into
`.statute/site/` next to `statute.toml`, to be opened in a browser from
disk; it is never published, and it is not
[the public site](static-site.md). It reads `statute.toml` and, when it
is there, the private rule set, both layers resolved in memory, with
each git source at the commit the lock file records
([Git sources](global-configuration.md#git-sources)); it writes no lock
file. It prints the path of `index.html`.

- **Files.** Plain HTML with one inline style sheet, which follows
  [Frontend](frontend.md) with the public site's tokens, and its icons
  inlined: no script, no Hugo, no dependency beyond those statute
  has. Each run builds the site
  anew: it removes the old `.statute/site/` first, but only when that
  directory holds statute's `.gitignore` (below), and otherwise writes
  nothing and fails. Nothing is written outside `.statute/site/` but
  `.statute/.gitignore`.
- **Ignored by git.** statute writes `.statute/site/.gitignore`
  holding `*`, so git ignores every file there, that one included,
  and it marks the directory as statute's. It also writes
  `.statute/.gitignore`, which lists `/site/`, as every command that
  writes a generated file into `.statute/` does
  ([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)).
  Before anything, in a git repository, it asks `git ls-files` about
  `.statute/site/`, and when git tracks a file there it writes nothing
  and fails, naming the file and the way out,
  `git rm -r --cached .statute/site`. When git cannot answer, because
  it does not start or fails, statute writes nothing either and fails
  with git's error. Outside a git repository there is nothing to commit
  and nothing to ask.
- **The index** has the four kinds of source
  ([Four kinds of source](rule-sources.md#four-kinds-of-source)) as
  sections: global public, global private, local public and local
  private. Each lists its sources, and each source its tree of domains
  with every node, enabled or not: its reference, title and status, and
  whether this rule set enables it, overrides it or leaves it out; a
  one-of is a box that holds its options, the default marked. A
  filter by kind of source, one that hides the rules the rule set does
  not enable, and one that shows superseded and repealed nodes, hidden
  by default, work through check boxes and the style sheet alone.
- **The kind of a source** follows its two properties: local for the
  repository's own rules (`L`) and a source with `scope = "local"`,
  global for any other, the global configuration's included; public as
  the source declares it, and for `L` as the rule set does. A source
  at `.`, as `R = { path = "." }` in this repo, is first-party and so
  global public. When `local-rules` names its rules directory, its
  rules are the repository's own as well: it is listed once, as global
  public, and the local public section says so and links there;
  otherwise the repository's own rules are listed in the local section
  of their kind. Not built yet: the site still treats a source at `.`
  as the repository's own rules whatever `local-rules` names.
  Every node, in the index and on its page, is marked with the kind of
  its source: its name, in the language of the pages, on one of four
  background colors. On the node's page the mark also names the source,
  as in `Global private · me`.
- **One page per node**, `<source>/<id>.html`: the statement, reason,
  verification and applicability; `requires`, `conflicts`, `overrides`
  and the successors; the path of parents and the children. A domain's
  page is the list of its nodes. A citation of a rule is a link to its
  page, and one of the constitution the provision's label, as in the
  compiled `AGENTS.md`. The pages are in the language of
  `[compile] lang`, the kind of each node included; a status is shown
  as the rule file writes it, `draft` or `trial`.
- **The marks of [Rule configuration](rule-config.md).** Every domain
  and rule, in the index and on its page, says whether it is on or off
  by default, and an option of a one-of whether it is the one-of's
  default. A rule's page says where it is used, `AGENTS.md`, review or
  both, and two filters of the index hide the rules for either.
  A placeholder shows the value this rule set gives the parameter, the
  person's own where there is one, marked, with the parameter's name
  on hover and a link to the page of its source's parameters,
  `<source>/parameters.html`. That page lists each parameter's title,
  name, type, scope and default, its value and where the value comes
  from, and the enabled rules that use it.

## Export

`statute export [<dir>]` prints the constitution and the rules of a
directory, `.` by default, as one JSON value, for a program that reads
them; this repo's site does
([Static site](static-site.md), Rules). It reads no rule set: the
directory is a source, holding `constitution/`, `rules/` or both.

- `constitution`: `null` without `constitution/`; otherwise the
  `edition`, `revision`, `status` and `languages` of the `index.toml` of
  its one edition (with none or more than one, the export fails), and its
  `provisions` in the order of the text: each article, section and item
  with its `id` as a citation names it, its `parent` (`null` for an
  article) and its `labels` by language, as the compiled `AGENTS.md`
  writes them.
- `nodes`: each node of `rules/` in order of ID, with its `file` and
  its `fields` as the file writes them, texts included. A program gets
  each value as written and a field statute learns later without a
  change to the export; it applies its own defaults. One value is
  statute's own reckoning, so that no program repeats it:
  `on-by-default`, whether the source's defaults enable the node
  ([Default switches](rule-config.md#default-switches)), never for a
  superseded or repealed rule, and `null` when that depends on a node
  of another source.
- `params`: the parameters of `rules/params.toml`
  ([Parameters](rule-config.md#parameters)) in the order of the file,
  each with its `name`, `type`, `values` (a `choice`'s, else `null`),
  `scope`, `default` and `title` by language, and `shown`: the default
  as the outputs write it, by each language a title of the source is
  written in (a language by its name, "Chinese", 「中文」); empty without
  that file.

Before it prints anything, statute checks the rules as it checks any
source ([Dependencies, one-ofs and packs](rule-sets.md#dependencies-one-ofs-and-packs));
when a check fails, it prints the errors alone and exits with status 1.
A translation that follows an older original fails that check, so every
translation a node lists follows its current original. JSON is the one
format, so the command has no option to choose one.

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
