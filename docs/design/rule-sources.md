# Rule sources

Status: draft, not built. This part of the design says which kinds of
source a rule set draws on, which may cite which, how a rule set
replaces one rule with its own (an override), how domains nest, and how
the hand-written guide is moved into rules and retired. It builds on
[Rule system](rule-system.md): the reference forms (`R42`, `L12`,
`team:7`) are those of [Sources and references](rule-system.md#sources-and-references),
and the tree, one-ofs and packs those of
[Dependencies, one-ofs and packs](rule-system.md#dependencies-one-ofs-and-packs).
Each point marked "default, to be decided" is a recommendation that
stands until a person decides otherwise. Where this part differs from
what [Rule system](rule-system.md) says today, the section
[What this changes](#what-this-changes) lists the difference.

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
| Local | The `rules/` next to a public repository's rule set, written `L12` | The `rules/` of a private repository, written `L12`; or, for a public repository, a source outside it (below) |

- **Global public** is the first-party source, `R`, and any public
  source of another team, `team:7`.
- **Global private** is a source the rule set names in `[sources]`, with
  `public = false`: a private git repository, or a directory on disk.
- **Local public** is the repository's own `rules/` when the repository
  is public.
- **Local private** is the repository's own `rules/` when the repository
  is private. A public repository keeps its private rules in a source
  outside it, named with `scope = "local"`, as in
  [Private rules in a public repository](#private-rules-in-a-public-repository).

Every rule, of whichever kind, is self-contained: it cites other rules
only through `requires`, `conflicts`, `overrides` and `{@rule:...}`, and
the constitution only through `authority` and `{@constitution:...}`. A
rule that has to point to the hand-written guide, or to any text that is
not a rule or the constitution, has not been migrated yet
([Cutover](#cutover-from-the-hand-written-guide)).

In `statute.toml`, a source's entry gains two fields (default, to be
decided):

```toml
public = true

[sources]
R = { git = "github.com/Luolc/agent-constitution", tag = "<tag>" }
team = { git = "example.test/our-team/rules", branch = "main", public = true }
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

Visibility is declared, not looked up: the rule set says `public = true`
or `public = false`, for itself and for each source (default, to be
decided). Reading it from the host at build time (GitHub's visibility of
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
[Checks](rule-system.md#checks) do: it fails when the rule set says
`public = false` and the repository's visibility is `PUBLIC`, and says it
could not tell when `gh` cannot answer (default, to be decided).

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

The message names the rule, the field and both sources:

```text
rules/12.toml: error: `requires` names "me:4", a rule of the private source `me`; a public rule may name only rules of public sources
```

The check reads the sources' declarations; it does not need the private
source's files, so it runs in CI on a public repository, where the
private source cannot be read.

## Private rules in a public repository

A public repository's committed files are public, and they include the
rule set, the lock file and the compiled `AGENTS.md`. A private source
named in `statute.toml` would publish its address there, and its rules
compiled into `AGENTS.md` would publish their text. So, in a rule set
with `public = true` (default, to be decided):

- `statute.toml` names only public sources; a source with
  `public = false` fails, with a note to name it in
  `statute.private.toml`.
- Private sources go in `statute.private.toml`, next to `statute.toml`,
  which git ignores. It names `statute.toml` as its base with
  `base = "statute.toml"`, inheriting it as described in
  [Rule sets and the lock file](rule-system.md#rule-sets-and-the-lock-file),
  and adds sources, rules and overrides:

  ```toml
  base = "statute.toml"
  rules = ["me:4", "here:2"]

  [sources]
  me = { path = "../my-rules", public = false }
  here = { path = "../my-rules/repos/example", public = false, scope = "local" }

  [compile]
  output = "AGENTS.private.md"
  ```

- It has its own lock file, `statute.private.lock`, and its own output,
  a whole instruction file with the public rules and the private ones,
  overrides applied. Before writing any of them, `statute compile` checks
  with `git check-ignore` that git ignores each of these files, and
  writes nothing when one is not ignored.

In a private repository (`public = false`) none of this is needed: the
one `statute.toml` names every source, and its outputs are committed as
they are. How an agent in a public repository is brought to read the
private output in place of the committed `AGENTS.md` is open: it is
measured on the agents first, as the instruction files of subdirectories
were ([Rule sets and the lock file](rule-system.md#rule-sets-and-the-lock-file),
Monorepos). Global private rules belong at the user level once it is
built (same section, Where it lives); until then they reach a repository
this way.

## Overrides

A rule set may put a rule of its own in place of a rule of another
source. A public rule says the main checkout of a repository goes in
`~/dev` and its worktrees in `~/wt`; a repository whose machines keep
them elsewhere writes a local rule with the other paths, which overrides
the public one. The compiled file then has only the local rule, at the
place of the one it overrides, with a line that says which one:
"Overrides R12". The public rule is not superseded: it stays in force in
every other rule set.

**Declaring it.** The overriding rule names the rule it replaces, in one
field (default, to be decided):

```toml
id = 3
kind = "rule"
overrides = "R12"
```

`overrides` names exactly one rule, written as `requires` writes it. A
rule that replaces two rules is two requirements, so it is two rules. A
domain or a one-of cannot be overridden: a rule set picks another child
of a one-of instead, and a child the source lacks is proposed to the
source ([Conflicting practices as one-ofs](#conflicting-practices-as-one-ofs)). A rule with
`overrides` has no `parent`, since its place is that of the rule it
overrides; the build fails when it has one.

**Who may override whom.** A source has one of three ranks, and a rule
may override only a rule of a lower rank (default, to be decided):

1. the first-party source, `R`, which overrides nothing;
2. every other global source, public or private: a team's or a person's
   own rules may replace a first-party rule for all of their
   repositories;
3. local rules: the repository's `rules/` and any source with
   `scope = "local"`.

Two rules of the same rank cannot override each other, and within one
source a rule is changed in place or superseded, never overridden. A
chain is allowed: a local rule may override `team:7`, which overrides
`R12`; the compiled file then shows the local rule, and its line says
"Overrides team:7, which overrides R12". The reference direction
([Which rules may cite which](#which-rules-may-cite-which)) applies as well: a public rule
cannot override a private one.

**What the build does.** An override takes effect when the overriding
rule is enabled:

- The overridden rule counts as enabled for every requirement on it and
  for its one-of, so a rule that requires `R12` is satisfied, and `R12`'s
  siblings under a one-of stay excluded. Its own requirements are not
  brought in; the overriding rule's are.
- Its children stay as they are and are rendered under the overriding
  rule's heading.
- Its check does not run; the overriding rule's does.
- Two enabled rules that override the same rule fail the build, naming
  both.
- An overriding rule whose target is not enabled by anything is compiled
  where its target would stand, with a warning that the target is not in
  the rule set.

**The lock file** records the overridden rule as it records any other,
with one more field:

```toml
[[rules]]
rule = "R12"
revision = 2
sha256 = "<SHA-256 of rules/12.toml>"
overridden-by = "L3"
```

So the source's later changes to `R12` still show on `statute upgrade`,
which adds to each such line that `L3` overrides it, for the user to
check that the override still holds.

**Not the same as.**

- Superseding: a source replaces its own rule for every rule set, for
  good ([IDs and revisions](rule-system.md#ids-and-revisions)); an
  override replaces another source's rule in one rule set.
- `conflicts`: two rules that cannot both be enabled, with nothing taking
  either's place ([Dependencies, one-ofs and packs](rule-system.md#dependencies-one-ofs-and-packs));
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
that names a domain (default, to be decided):

```toml
id = 105
kind = "domain"
parent = 104
```

- A domain's `parent`, when present, names a domain with no parent. So
  there are at most two levels; a subdomain under a subdomain fails the
  build, as does a domain whose parent is a rule or a one-of.
- A rule or a one-of may have a subdomain as its parent, as it may a
  domain. A one-of cannot have a domain below it, so subdomains and
  one-ofs never interleave: the tree goes domain, subdomain, then rules
  and one-ofs.
- A rule's `parent` may also name a domain or a subdomain of another
  source, never a rule or a one-of of another source, so that the local
  rules of a repository about Git appear under the general Git heading
  rather than under one of their own. The reference direction applies:
  a public rule's parent is a public domain.
- Enabling a domain as a pack enables its subdomains' rules; a subdomain
  may be the root of a pack of its own.

**Rendering.** In the compiled `AGENTS.md`, a domain is a heading of
level two and a subdomain one of level three; each level of the tree
below is one level deeper, down to level six, as in
[Compilation and the site](rule-system.md#compilation-and-the-site). A
subdomain with no enabled rule below it does not appear, nor does a
domain all of whose subdomains and rules are absent. A requirement is
rendered as a link when it leads out of the requiring rule's nearest
domain, subdomain included. On the site, a subdomain heads the rules
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
whole, in one step, once every part of it has one. There is no
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
4. Load verification ([Checkers](checkers.md)) passes on the compiled
   output.

Then one change on the private side installs the compiled output in
place of the guide and retires the guide.

## What this changes

Once built, this part replaces these statements in
[Rule system](rule-system.md), which stay as they are until the work
that implements it changes them:

- Dependencies, one-ofs and packs, Domains: "Domains do not nest"
  becomes the two levels of [Subdomains](#subdomains).
- Rule file format: `parent` may name a domain of another source, a
  domain may have a `parent`, and a rule may have `overrides` in place of
  `parent`.
- Rule sets and the lock file: the rule set gains `public`, a source
  gains `public` and `scope`, a public repository gains
  `statute.private.toml` with its lock file and output, and a rule of
  the lock file may have `overridden-by`.
- The tool and the first version, Moving over: the switch is the one of
  [Cutover](#cutover-from-the-hand-written-guide), whole, not one topic first.

And in the rest of the design: [Choosing rules](rules.md#choosing-rules),
"to change a rule, the overlay leaves it out and adds a private rule in
its place", becomes an override.
