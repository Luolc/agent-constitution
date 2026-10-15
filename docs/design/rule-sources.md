# Rule sources

Status: decided; built in part, as
[The command and the first version](statute-command.md#the-command-and-the-first-version)
says. This part of the design says which kinds of
source a rule set draws on, which may cite which, how a rule set
replaces one rule with its own (an override), how domains nest, and how
the hand-written guide is moved into rules and retired. It builds on
[Rule system](rule-system.md): the reference forms (`R42`, `L12`,
`team:7`) are those of [Sources and references](rule-sets.md#sources-and-references),
and the tree, one-ofs and packs those of
[Dependencies, one-ofs and packs](rule-sets.md#dependencies-one-ofs-and-packs).
The other parts of the design that this one changes already describe it,
each marking what is not built yet; [What this changes](#what-this-changes)
lists them, and what the tools do today that building the rest
replaces.

## Contents

- [Four kinds of source](#four-kinds-of-source)
- [Public or not](#public-or-not)
- [Which rules may cite which](#which-rules-may-cite-which)
- [Private rules in a public repository](#private-rules-in-a-public-repository)
- [Overrides](#overrides)
- [Subdomains](#subdomains)
- [Conflicting practices as one-ofs](#conflicting-practices-as-one-ofs)
- [Cutover from the hand-written guide](#cutover-from-the-hand-written-guide)
- [What this changes](#what-this-changes)

## Four kinds of source

A rule has two properties that matter here: how far it reaches, and
whether anyone may read it. How far it reaches is its scope: global, for
every repository of a person or a team, or local, for one repository.
Whether anyone may read it is its visibility: public or private. The two
give four kinds:

| | Public | Private |
| --- | --- | --- |
| Global | This repo's rules, written `R42` | A source of the person's or team's own, such as a private repository: the names of their bots and machines, how their secret store is laid out, where directories go on their machines |
| Local | The `.statute/rules/` of a public repository, written `L12` | The `.statute/rules/` of a private repository, written `L12`; or, for a public repository, a source outside it (below) |

- **Global public** is the first-party source, `R`, and any public
  source of another team, `team:7`.
- **Global private** is a source the rule set names in `[sources]`, with
  `public = false`: a private git repository, or a directory on disk.
- **Local public** is the repository's own rules, in `.statute/rules/`
  ([The directory of local rules](rule-config.md#the-directory-of-local-rules)),
  when the repository is public.
- **Local private** is the repository's own rules when the repository
  is private. A public repository keeps its private rules in a source
  outside it, named with `scope = "local"`, as in
  [Private rules in a public repository](#private-rules-in-a-public-repository).

**A source and a repository's own rules** are two different things, and
a repository may have both:

- A source publishes rules for other repositories to use. Its rules are
  in `rules/` at its top, or in the directory its entry names with
  `rules-dir` ([Sources and references](rule-sets.md#sources-and-references)),
  and a rule set that wants them names the source in `[sources]`, by a
  git address or a path, and cites them as `R42` or `team:7`.
- A repository's own rules bind that repository alone. They are in
  `.statute/rules/`, cited as `L12`, and compiled into that
  repository's output only. No other repository can name them: there
  is no reference form for another repository's own rules, and
  `rules-dir` may not lead into `.statute/`.

This repo is both: it publishes the first-party rules in `rules/`, and
keeps rules for its own development in `.statute/rules/`. A rule that
another repository should follow belongs in a source; one that only
makes sense here is a local rule.

Every rule, of whichever kind, is self-contained: it cites other rules
only through `parent`, `requires`, `conflicts`, `overrides` and
`{@rule:...}`, and
the constitution only through `authority` and `{@constitution:...}`. A
rule that has to point to the hand-written guide, or to any text that is
not a rule or the constitution, has not been migrated yet
([Cutover](#cutover-from-the-hand-written-guide)).

In `statute.toml`, the rule set gains one field and a source's entry
two:

```toml
public = true

[sources]
R = { git = "https://github.com/Luolc/agent-constitution", tag = "<tag>" }
team = { git = "https://example.test/our-team/rules.git", branch = "main", public = true }
```

- `public` at the top of the rule set: whether the repository that holds
  it is public. It is required; there is no default. It sets the
  visibility of the local rules and decides what the committed rule set
  may name ([Private rules in a public repository](#private-rules-in-a-public-repository)).
- `public` on a source: required on every source but `R`, which is always
  public; `R = { ..., public = false }` fails.
- `scope` on a source: `"global"`, the default, or `"local"`. It matters
  only for overrides ([Overrides](#overrides)).

## Public or not

Visibility is declared, not looked up when the rule set is read: it says `public = true`
or `public = false`, for itself and for each source. Reading it from the host at build time (GitHub's visibility of
the repository) would make the build need the network and an account,
and give two machines different results for the same inputs; guessing
from where a source lives (`R` public, everything else private) would
make a public team source private and forbid its rules to be cited by a
public one.

A wrong declaration is dangerous in one direction only. A private
repository that declares `public = true` gets stricter checks than it
needs, and nothing leaks. A public repository that declares
`public = false` could write private rules into files anyone can read.
So a built-in check of `statute check` compares the declaration with the
host where it can, through `gh` as the built-in checks of
[Checks](rule-checks.md#checks) do: it fails when the rule set says
`public = false` and the repository's visibility is `PUBLIC`, and says it
could not tell when `gh` cannot answer.

The declaration is filled in once, when it is written, and then read
only from `statute.toml`. `statute init` asks `gh` for the visibility of
the repository it writes the rule set in, unless `--public` or
`--private` decides, and takes the side that cannot leak where GitHub
gives no clear answer:

| GitHub shows | `public` written |
| --- | --- |
| `PUBLIC` | `true` |
| `PRIVATE` | `false` |
| `INTERNAL`, which everyone in the enterprise can read | `true`, with a warning that says how to make it private |
| no answer (no `gh`, no login, no GitHub remote) | `true`, with a warning that names why and says how to make it private |

When a source is added, `statute source add` asks `gh` in the same
way, with the safe side reversed: `INTERNAL`, no answer and a directory
in no GitHub repository count as private, and a private source of a
public repository is written to the private rule set rather than
stopping the command
([Adding a source](global-configuration.md#adding-a-source)).

## Which rules may cite which

A private rule may cite a public one; a public rule may not cite a
private one. A public rule's text is published, and a citation of a
rule nobody can read is a dangling link at best and a leak at worst (its
source's name and address).

The build fails when a rule of a public source (`R`, a public source, or
the local rules of a public repository) names a rule of a private source
in any of these places:

- `requires` or `conflicts`;
- `overrides` ([Overrides](#overrides));
- `parent`, where it may name a domain of another source
  ([Subdomains](#subdomains));
- a `{@rule:...}` citation in its title or any part of its text.

The message names the rule, the field, the kind of node it names and
both sources:

```text
rules/12.toml: error: `requires` names "me:4", a rule of the private source `me`; a public rule may name only rules of public sources
rules/21.toml: error: `parent` names "me:1", a domain of the private source `me`; a public rule may name only rules of public sources
```

The check reads the sources' declarations; it does not need the private
source's files, so it runs in CI on a public repository, where the
private source cannot be read.

## Private rules in a public repository

A public repository's committed files are public, and they include the
rule set, the lock file and the compiled `AGENTS.md`. A private source
named in `statute.toml` would publish its address there, and its rules
compiled into `AGENTS.md` would publish their text. So, in a rule set
with `public = true`:

- `statute.toml` names only public sources; a source with
  `public = false` fails, with a note to name it in the private rule
  set.
- A private source holds only what is private. It copies no public rule:
  it cites one, as any rule cites another.
- Private sources go in the private rule set, `private.toml`, which
  lives in git's own directory, not in the work tree:

  ```text
  $(git rev-parse --git-path common/statute)/
    private.toml          the private rule set
    private.lock          its lock file
  ```

  In a clone that is `.git/common/statute/`, from the main checkout
  and from each linked worktree alike. The private rule set builds on
  the `statute.toml` of the worktree statute runs in, which it inherits
  as [Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file)
  describes, and adds sources and rules:

  ```toml
  rules = ["me:4", "here:2"]

  [sources]
  me = { path = "/home/me/my-rules", public = false }
  here = { path = "~/my-rules/repos/example", public = false, scope = "local" }
  ```

  It may list `packs` as `statute.toml` does, rooted in a node of any
  source of either layer, so that a private source's new rules are
  enabled with it; a pack may enable public rules, which the private
  layer alone then enables, as a rule it lists may. It has no
  `[compile]`, since its output's place is fixed (below), and no
  `base`, since its base is always the `statute.toml` beside which the
  command runs. A `path` in it is absolute or starts with `~/`; a
  relative one fails, since the file serves every worktree and a
  relative path would lead somewhere else from each. `statute source
  add` writes the absolute path. Its sources' names may not repeat those
  of `statute.toml`, nor its rules and packs those `statute.toml` lists.
  In a private repository (`public = false`) the file fails: there
  `statute.toml` names everything. Outside a git repository there is
  no private layer, and a command with `--private` fails. `statute add
  --private` and `statute remove --private` change it, writing a
  temporary file in the same directory first and renaming it into
  place.

- This private layer only adds to the public one. It may not override a
  rule of `statute.toml`, nor leave one out
  ([Overrides](#overrides)): the committed `AGENTS.md` holds that rule
  whatever the private layer does, and an agent that read both files
  would find two rules that disagree.
- It has its own lock file, `private.lock`, which pins the private
  layer's own sources and nothing else: each as `statute.lock` records a
  source, a git source by its address, its branch or tag and the commit,
  with `public` and `scope`. It records no rule and nothing of the
  public layer. `statute lock` writes it with `statute.lock`, and the
  commands that change the private rule set or its sources write it
  again. The two layers together are resolved in memory each time a
  command needs them, from the `statute.lock` of the worktree and
  `private.lock`, and that resolution is never written down. So every
  worktree reads the private sources at the same commits, while each
  reads the public layer its own branch pins; a lock file of both
  layers together, shared by worktrees on different branches, would be
  stale in all but one of them.
- git never tracks a file in its own directory, so the private rule set
  and its lock file cannot be committed, and need no `.gitignore` and no
  check. What the private layer writes in the work tree is the private
  output under `.statute/` and two links at the top (below); before
  writing the file under `.statute/`, statute checks with
  `git check-ignore` that git ignores it, and the links are checked as
  below.
  A path whose last matching pattern is a negation (`!`) counts as not
  ignored. It writes no file at all, public or private, when one is not
  ignored or when git cannot tell.
- The skills and the `check = "command"` checks of the rules only the
  private layer enables are left out of the committed files,
  `.agents/skills/` and `statute-checks.toml`
  ([Compilation and the site](statute-command.md#compilation-and-the-site));
  `statute check` runs those checks all the same. How a private rule's
  skills are installed is an open question.

The place in git's directory is chosen so that every worktree of a
clone shares one private rule set. git shares everything under
`common/` of its common directory between the worktrees of a clone
(gitrepository-layout(5)), and `git rev-parse --git-path common/...`
names that place from any worktree, where a path git does not know,
such as `--git-path statute`, would name a directory of each worktree
alone. A file in the work tree does not come with `git worktree add`, which checks out only what git
tracks, so each new worktree would need a copy that then drifts. A
directory in the home directory keyed by the repository's path would
also be shared, but it is orphaned when the clone moves or is deleted,
and needs an encoding of paths into names; one keyed by the remote's
address would join two clones of one remote that are meant to differ,
and has no key for a repository without one. The cost of git's
directory is that the files are out of sight: the commands that write
them print their full path. A clone made anew, or copied without git,
starts without them.

**The names stay `private`.** `local` already means a scope in statute
(`scope = "local"`, `L12` and the local kinds of source), so a name
such as `statute.local.toml`, as other tools name a machine's own
settings, would read as the configuration of the local rules. What
sets these files apart is that they must not be published, which
`private` says.

**The private output** is one file, `.statute/AGENTS.full.md`: the
committed `AGENTS.md` with the rules only the private layer brings in
(its own rules and the public rules that only it enables), each at its
place in the tree. Two links lead to the file:

```text
.statute/
  .gitignore              written by statute, committed: the generated files below
  rules/                  the repository's own rules, committed
  AGENTS.full.md          the public rules and the private ones
  site/                   the local site
CLAUDE.local.md -> .statute/AGENTS.full.md
AGENTS.override.md -> .statute/AGENTS.full.md
```

- Claude Code reads `CLAUDE.local.md`, and where it exists no longer
  reads `AGENTS.md`, so it reads the full text alone. Nor does it read
  the `AGENTS.md` of a subdirectory, below. This holds with its default
  setting; with the setting that reads `AGENTS.md` beside `CLAUDE.md`,
  it reads both.
- Codex and Pi read `AGENTS.override.md` in place of the `AGENTS.md` of
  the same directory, so they read the full text alone.
- Grok reads both `AGENTS.md` and `CLAUDE.local.md`, so it reads the
  public rules twice. They are the same rules and do not contradict
  each other; the repetition is accepted.
- A repository with a `CLAUDE.md` makes Claude Code read it as well as
  `CLAUDE.local.md`, so its content comes on top of the full text: the
  public rules twice when it is a link to `AGENTS.md` or a copy of it,
  and otherwise text that may not agree with the rules. Grok counts a
  `CLAUDE.md` and an `AGENTS.md` that lead to the same file as one, so
  it reads the public rules twice with such a link as without it.
  `statute compile` warns of a `CLAUDE.md`
  ([Compilation and the site](statute-command.md#compilation-and-the-site)).
- OpenCode is not supported yet.
- The committed `AGENTS.md` does not point to `.statute/AGENTS.full.md`:
  an agent that reads a link would read the full text twice, and one
  that finds neither link gets the public rules.

The two links are ignored through `.git/info/exclude`, never through a
`.gitignore`: Grok skips an instruction file that a `.gitignore`
matches, judging by the link's own path, and does not read
`.git/info/exclude`. So `statute compile` checks with `git check-ignore
--verbose` that each link is ignored by a pattern outside every
`.gitignore`, and writes nothing otherwise, naming the lines to add to
`.git/info/exclude`. A file at either path that is not such a link is
left as it is, and the compile fails.

**`.statute/` holds what is committed and what is not.** The
repository's own rules in `.statute/rules/` are committed; the
private output and the local site
([The local site](statute-command.md#the-local-site)) are not. So the
repository's `.gitignore` does not name `.statute/`: statute writes
`.statute/.gitignore` itself, as uv does in the `.venv` it creates and
ruff in its `.ruff_cache`, and it lists the generated files by name:

```gitignore
# written by statute
/AGENTS.full.md
/site/
```

A file statute adds later is then tracked until the list names it,
which is seen in `git status`, where a pattern of `*` with exceptions
would hide it. Every command that writes a generated file into
`.statute/` writes `.statute/.gitignore` first when it is missing, and
rewrites it when it starts with statute's line and differs; a
`.statute/.gitignore` that does not start with that line is the
repository's own, and the command writes nothing and fails. The file is
committed, so that a fresh clone ignores the generated files before
statute first runs there. The check with `git check-ignore` above
still runs on each file before it is written.

Codex reads a project's instruction files, from the repository's root
down to the working directory, up to the number of bytes set in its
own configuration, `project_doc_max_bytes`, and cuts off the rest;
statute sets no limit on the full text.

Where `CLAUDE.local.md` exists, Claude Code loads no `AGENTS.md` of a
subdirectory either, neither at the start in that subdirectory nor when
it touches a file there, as it does without it. The rules of a
subdirectory are lost to it, unless the model happens to read the file.
So with the private output, a repository keeps such rules in its rule
set, not in `AGENTS.md` files of subdirectories. When it compiles the
private layer, `statute compile` warns of each `AGENTS.md` that git
tracks below the directory of `statute.toml` (the top of the
repository, as a rule, and where the links are written), saying that
Claude Code does not load it and that its rules belong in the rule
set. statute does not compile their content into
the full text.

**A new worktree.** The private rule set, its lock file and
`.git/info/exclude` are shared by the worktrees of a clone, but the
private output and the two links are files of one work tree, and
`git worktree add` checks out only what git tracks. A git
`post-checkout` hook writes them. git runs it after `git worktree add`
(unless with `--no-checkout`) and after each
`git switch` or `git checkout` of a branch, in the top of the work tree
(githooks(5)); the hooks are in the common directory, so one hook
serves every worktree of the clone. The hook runs one command,
`statute compile --private-only`:

```sh
#!/bin/sh
[ "$3" = 1 ] && statute compile --private-only; exit 0
```

`$3` is 1 for a checkout of a branch and 0 for one of files, so the
private output follows the branch, whose `statute.toml` may differ.
The hook's path is what `git rev-parse --git-path hooks/post-checkout`
prints, which follows `core.hooksPath`; a repository whose hook
manager owns that file adds the line there, and pre-commit can install
a hook of this type. statute does not install the hook: the user does,
once per clone. `--private-only`:

- writes only `.statute/.gitignore`, `.statute/AGENTS.full.md` and the
  two links, never `AGENTS.md`, the skills or `statute-checks.toml`;
- reads `statute.toml` and `statute.lock` of the worktree and the
  private rule set and its lock file, and needs no network: a git
  source is read from statute's cache
  ([The statute directories](global-configuration.md#the-statute-directories)),
  and nothing is fetched;
- does nothing and exits 0, silently, when there is no `statute.toml`,
  the repository is private (`public = false`) or there is no private
  rule set;
- otherwise writes nothing, prints a warning that names the cause and
  the command to run, and exits 0, whenever the plain `statute compile`
  would fail or need the network: a lock file that is stale, a commit
  missing from the cache, a link not ignored through
  `.git/info/exclude`, a rule set that does not resolve. A hook that
  fails would make `git worktree add` and `git switch` exit with its
  status (githooks(5)), so it never does.

The links are relative and lead to the worktree's own `.statute/`.

The private rules only add to the public ones and never contradict
them, as the private layer only adds rules (above): Grok reads the
public rules apart from the full text, and an agent that read two files
in addition gave no reliable priority to either when they disagreed.

The agents' behavior was measured on 2026-10-14 in non-interactive mode
on Claude Code 2.1.296, Codex 0.159.3 and Grok 1.0.50, with Grok's
folder-trust check turned off for each run (without it, Grok loads no
project instructions in non-interactive mode), and on Pi 1.2.0 in one
run per case; Codex's `project_doc_max_bytes` is from
its documentation and source. In runs of one small task, Codex and Grok
read the file a fallback line with an `@` path names, two runs each,
and Claude Code loaded it as an import; with a line of the same kind
but other wording than statute's, the path in backticks and no `@`,
Claude Code read the full text in two runs when nothing else loaded
it, each time the model's own read. Every run had
the user's own instruction files and skills in place, which also urge
an agent to find its instructions. The layout itself, with a stand-in
for the full text:

- Grok loaded the link `CLAUDE.local.md` ignored through
  `.git/info/exclude` in 2 runs of 2, and skipped it ignored through a
  `.gitignore` in 2 runs of 2.
- With `CLAUDE.local.md` at the root, Claude Code loaded no
  `sub/AGENTS.md` in 4 runs of 4, two with the working directory in
  `sub/` and two at the root changing a file in `sub/`; without it, it
  loaded the file in both control runs, once at the start and once
  after reading a file in `sub/`. In one of the four runs the
  model read the file itself.
- With `CLAUDE.md -> AGENTS.md` and `CLAUDE.local.md`, Claude Code loaded
  the public rules twice in one run, against once with
  `CLAUDE.md -> AGENTS.md` alone. That Grok counts the two as one comes
  from the files `grok inspect` lists, with no model run.
- Pi loaded the full text alone through `AGENTS.override.md`, the link
  ignored through `.git/info/exclude` or through a `.gitignore`, and not
  `AGENTS.md`.
- Claude Code, in the full layout with its default setting, loaded the
  full text alone, and neither the root's nor a subdirectory's
  `AGENTS.md`.

Not measured: interactive mode; the fallback line where no user-level
instructions or skills are present; a
`CLAUDE.md -> AGENTS.md` in a subdirectory, and whether it
brings that subdirectory's rules back to Claude Code; Claude Code's
setting that reads `AGENTS.md` beside `CLAUDE.md`, for subdirectories;
and Grok's reading of a subdirectory's `AGENTS.md` under this layout.
Each negative result holds only for the runs counted; the others are
one run each.

The private layer is built as described here, except what this
section marks as not built yet.

In a private repository (`public = false`) none of this is needed: the
one `statute.toml` names every source, and its outputs are committed as
they are. Global private rules belong at the user level once it is
built ([Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file),
Where it lives); until then they reach a repository
this way, their source named once in the global configuration
([Global configuration and git sources](global-configuration.md)).

## Overrides

A rule set may put a rule of its own in place of a rule of another
source. Most departures need none. A public rule states the principle
and leaves the concrete values to the layers below, which add them as
rules of their own: a public rule says that worktrees live outside the
main checkout, in a directory of their own, and a global private rule
names the directories, `~/dev` for main checkouts and `~/wt` for
worktrees. The two rules hold together, so the second adds to the first
and overrides nothing. A public rule is written so that it can be added
to this way.

An override is for a rule that cannot hold together with what a rule set
needs. A public rule says that pull request descriptions are written in
English, a value the source chose rather than leaving it to the layers
below; a repository whose readers all read Chinese writes a local rule
that its descriptions are in Chinese, which overrides the public one.
The lasting fix is to propose to the source the rule in its
principle-only form.
The compiled file then has only the local rule, at the place of the one
it overrides, with a line that says which one, `Overrides R200.`
([Compilation and the site](statute-command.md#compilation-and-the-site)). The
public rule is not superseded: it stays in force in every other rule
set.

**Declaring it.** The overriding rule names the rule it replaces, in one
field:

```toml
id = 3
kind = "rule"
overrides = "R200"
```

`overrides` names exactly one rule, written as `requires` writes it. A
rule that replaces two rules is two requirements, so it is two rules. A
domain or a one-of cannot be overridden: a rule set picks another child
of a one-of instead, and a child the source lacks is proposed to the
source ([Conflicting practices as one-ofs](#conflicting-practices-as-one-ofs)). A rule with
`overrides` has no `parent`, since its place is that of the rule it
overrides; the build fails when it has one.

**Who may override whom.** A source has one of three ranks, and a rule
may override only a rule of a lower rank:

1. the first-party source, `R`, which overrides nothing;
2. every other global source, public or private: a team's or a person's
   own rules may replace a first-party rule for all of their
   repositories (a private source's rules, in a public repository, only
   a rule that the private layer alone enables, below);
3. local rules: the repository's own, in `.statute/rules/`, and any
   source with `scope = "local"`.

Two rules of the same rank cannot override each other, and within one
source a rule is changed in place or superseded, never overridden. A
chain is allowed: a local rule may override `team:7`, which overrides
`R200`; the compiled file then shows the local rule, and its line says
"Overrides team:7, which overrides R200". The reference direction
([Which rules may cite which](#which-rules-may-cite-which)) applies as well: a public rule
cannot override a private one.

**Only within one output.** An override is made only between rules
compiled into the same output. In a private repository every layer is
compiled into one file, so any rule may override one of a lower rank:
the overridden rule does not appear, and the overriding rule says which
one it overrides. In a public repository, the private layer only adds
rules ([Private rules in a public repository](#private-rules-in-a-public-repository)):
a rule it enables fails when it overrides a rule of the public layer,
one that `statute.toml` enables, directly or through a requirement:

```text
error: `here:2` overrides "R200", a rule of the public rule set `statute.toml`; in a public repository (`public = true`), `private.toml` may only add rules
```

Within the private layer, a rule may override any rule of a lower rank
that only the private layer enables, since both are compiled into the
private output; within the public layer, a rule may override a public
rule of a lower rank. Both work as in a private repository.

**What the build does.** An override takes effect when the overriding
rule is enabled:

- The overridden rule counts as enabled for every requirement on it and
  for its one-of, so a rule that requires `R200` is satisfied, and `R200`'s
  siblings under a one-of stay excluded. Its own requirements are not
  brought in; the overriding rule's are. Its parent is the exception:
  when that is a rule, directly or through a one-of, it still counts as
  a requirement of the overriding rule, which stands in its place.
- Conflicts declared on the overridden rule, or by other rules against
  it, do not apply; the overriding rule's conflicts do.
- Its children stay as they are.
- Its check does not run; the overriding rule's does.
- An override applies where the overriding rule is enabled: in a rule
  set split into path segments, in the segments that enable it, and
  nowhere else.
- Two enabled rules that override the same rule fail the build, naming
  both.
- An overriding rule whose target is not enabled by anything is compiled
  where its target would stand, with a warning that the target is not in
  the rule set.

**The lock file** records the overridden rule as it records any other,
with one more field:

```toml
[[rules]]
rule = "R200"
revision = 2
sha256 = "<SHA-256 of rules/200.toml>"
original-sha256 = "<SHA-256 of rule 200 without its translations>"
overridden-by = "L3"
```

So the source's later changes to `R200` still show on `statute upgrade`,
which adds to each such line that `L3` overrides it, for the user to
check that the override still holds.

**Not the same as.**

- Superseding: a source replaces its own rule for every rule set, for
  good ([IDs and revisions](rule-format.md#ids-and-revisions)); an
  override replaces another source's rule in one rule set.
- `conflicts`: two rules that cannot both be enabled, with nothing taking
  either's place ([Dependencies, one-ofs and packs](rule-sets.md#dependencies-one-ofs-and-packs));
  an override puts one rule in the other's place.
- A one-of: alternatives the source foresaw, each a rule of its own, of
  which each rule set picks one; an override is a departure the source
  did not foresee. When two rule sets override the same rule the same
  way, the departure is proposed to the source as a child of a one-of.
- Leaving a rule out and adding one: the rule set loses the tie to the
  original. An override keeps it: its place in the tree, the
  requirements on it, and the notice when the original changes.
- A relaxation ([Relaxation declarations](rules.md#relaxation-declarations)):
  a relaxation lifts part of a rule under a permission of the
  constitution; an override is a choice of which rules a jurisdiction
  adopts (`division-of-instructions.rules`), and needs no permission.

## Subdomains

A domain may sit under another domain, as a subdomain: Git under
Workflow, say, or Rust under Languages. It is a domain with a `parent`
that names a domain:

```toml
id = 105
kind = "domain"
parent = 104
```

- A domain's `parent`, when present, is a domain, at any depth: of
  the same source, written as its number, or of another source,
  written as `requires` writes one (`"R78"`). It may also be a one-of
  of the same source, of which the domain is an option
  ([No rule under a rule](rule-config.md#no-rule-under-a-rule)).
  Domains nest to any depth; a domain whose parent is a rule fails the
  build, as do parents that form a cycle, within one source or across
  sources. The reference direction applies: a domain of a public
  source hangs only under a public domain.
- A rule or a one-of may have a subdomain as its parent, as it may a
  domain; a rule's parent is never a rule.
- A rule's `parent` may also name a domain or a subdomain of another
  source, never a rule or a one-of of another source, so that the local
  rules of a repository about Git appear under the general Git heading
  rather than under one of their own. The reference direction applies:
  a public rule's parent is a public domain.
- A domain of one source may hang under a domain of another for the
  same reason: a private source's domain about the user's fleet of
  agents appears as a subdomain of the first-party domain about agents,
  with its rules below it. The first-party source's own domains are the
  roots: in it, a `parent` in another source is accepted on a rule only
  ([Checkers](checkers.md#checkers)).
- Enabling a domain as a pack enables its subdomains' rules; a subdomain
  may be the root of a pack of its own. A pack is a subtree of one
  source: it does not reach the rules or subdomains of another source
  hung under its domains, which a rule set enables by listing them in
  `rules`, or a subdomain of another source as a pack of its own.

**Rendering.** The compiled `AGENTS.md` shows no domain or subdomain:
its rules are a flat list in the order of the tree, as in
[Compilation and the site](statute-command.md#compilation-and-the-site),
so a subdomain's rules come at its place in its domain. Under a domain,
its own nodes come first, then those of other sources by the sources'
names, as the domains themselves are ordered. On the site, a subdomain heads the rules
below it as a domain does, nested under its domain's heading, and its
page lists the nodes below it ([Static site](static-site.md)).

## Conflicting practices as one-ofs

When repositories have good reasons to work in incompatible ways, the
ways are written as children of a one-of, and each rule set picks one.
No rule is written as "do A, except when ..." and patched each time a
repository needs the exception.

How the design keeps up with the code is such a pair. A one-of "Keep
the design in step with the code", under a documentation domain, has two
children (the IDs are placeholders here; the rules are written in
`rules/`, not in this design):

- `<A>`: the design changes in the same pull request as the code it
  describes.
- `<B>`: the code may land before or after the design, and every
  mismatch is marked: a part of the design not built yet says so, and a
  pull request that leaves the design behind says so in its description
  and leaves a follow-up.

The reasons for B: designing before writing code is normal; a project
that wants to move fast lets code go first; and when the design is in
one place, parallel changes to it conflict on merge. A rule set enables
one of them in `rules`, as with any one-of; one that enables `<B>` has
nothing to relax and nothing to override, and a monorepo may pick
differently per path segment. The rules that a design is descriptive,
not an instruction, and that it is split into files by topic so
parallel changes rarely meet, are separate rules beside the one-of, not
conditions inside either child.

## Cutover from the hand-written guide

The hand-written guide is replaced by rules of the four kinds as a
whole, in one step, once every part of it has one and the four
conditions below hold; every repository the guide governs switches
then. There is no
transition clause: no text that keeps part of the guide in force after
the switch, and no switch of one topic (credentials, say) before the
others. What is specific to one environment was never a candidate for
the first-party source and does not wait in the guide either: it is a
global private or a local rule from the start.

**The inventory.** Each item of the guide gets exactly one destination,
recorded in a mapping on the private side, as in
[Migrating from the current hand-written guide](fleet-and-migration.md#migrating-from-the-current-hand-written-guide),
step 3, with its "private rule" split three ways:

- the constitution;
- a first-party rule (`R`), when any repository could use it as written;
- a global private rule, when it holds for all of the person's
  repositories but names their machines, accounts, bots or layout;
- a local rule of one repository, public or private as the repository
  is;
- a parameter, a skill (a procedure), provenance (to the private
  casebook), or removal.

A repository's own departure from the guide (that guide's step 6)
becomes an override, a child of a one-of, or is removed.

**Done when** all of these hold, checked on the private side:

1. Every item of the guide has a destination in the mapping, and each
   destination that is a rule gives its reference (`R42`, `me:4`, `L12`).
   No item is pending.
2. No rule of any kind names the guide, its file or the numbers of its
   items. The private-term scan ([Checkers](checkers.md#checkers)) runs
   with the guide's file name and the pattern of its item numbers as
   terms, each pattern first run on a line that must match.
3. Every repository the guide governs has a rule set, and `statute lock`
   and `statute compile` succeed for it, the private rule set included.
   The compiled output has been compared with the guide section by
   section, and every difference is recorded as intended.
4. [Load verification](checkers.md#load-verification) passes on the compiled
   output.

Then one change on the private side installs the compiled output in
place of the guide and retires the guide.

## What this changes

The parts of the design this one changes describe it already, each
marking what is not built:

- [Rule file format](rule-format.md#rule-file-format): a rule's `parent`
  may name a domain of another source, a domain may have a `parent`, in
  its own source or another, and
  a rule may have `overrides` in place of `parent`.
- [Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file):
  `public`, `scope`, the private rule set and `overridden-by` in the
  lock file.
- [Dependencies, one-ofs and packs](rule-sets.md#dependencies-one-ofs-and-packs):
  domains nested to any depth.
- [Compilation and the site](statute-command.md#compilation-and-the-site):
  where a subdomain's rules come in the flat list.
- [The command and the first version](statute-command.md#the-command-and-the-first-version):
  what is built so far, and moving over by the cutover above.
- [Checkers](checkers.md#checkers): what `statute validate` checks of
  the new fields within one source and what statute checks across
  sources.
- [Choosing rules](rules.md#choosing-rules): a rule is changed by an
  override.
