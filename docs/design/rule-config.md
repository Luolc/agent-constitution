# Rule configuration

Status: decided; not built yet. What is built is under
[The command and the first version](statute-command.md#the-command-and-the-first-version).
This part of the design says three things a rule set can set beyond
the list of its rules: the directory of the repository's own rules,
the values of parameters that rules leave open (the languages first),
and which domains and rules come in without being listed, through a
switch each of them declares; and one thing each rule says of itself,
whether it goes to the agents at work, to the reviewer or to both. It
builds on
[Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file),
[Rule file format](rule-format.md#rule-file-format) and
[Rule sources](rule-sources.md), and on
[the global configuration](global-configuration.md#the-global-configuration)
in the user's statute directory, `~/.statute/config.toml`, which names
sources for every rule set of the user and enables no rule.

The aim behind all three: a repository that adopts statute gets a
small set of the most general rules, which are unlikely to be wrong
anywhere, and opts in to the rest; what concerns one person or one
environment is a value it sets or a rule it turns on, not a rule it
has to take out.

## Contents

- [The directory of local rules](#the-directory-of-local-rules)
- [Parameters](#parameters)
- [Default switches](#default-switches)
- [Where a rule is used](#where-a-rule-is-used)
- [Recommended defaults for the first-party rules](#recommended-defaults-for-the-first-party-rules)
- [Proposed: no rule under a rule](#proposed-no-rule-under-a-rule)
- [What this changes](#what-this-changes)

## The directory of local rules

The repository's own rules are in `rules/` next to `statute.toml`
today ([Sources and references](rule-sets.md#sources-and-references)).
`local-rules` at the top of the rule set names another directory,
relative to that of `statute.toml`:

```toml
local-rules = ".repo-rules"
```

- The default is `"rules"`, so a rule set without the key reads what
  it reads today.
- Only `statute.toml` sets it. It describes the repository's layout,
  which every machine and CI share, so the global configuration has no
  such key, and `statute.private.toml` neither, since the private layer
  only adds.
- When the directory is the `rules/` of a source on disk, as in
  `R = { path = "." }` with the default, those rules belong to that
  source, as they do today: a local reference to one fails with a note
  to write `R12`.

This repo is the first-party source and will hold rules for its own
development as well, rules that bind this repository and are no
general rule for others. Its rule set, once it has one, names
`R = { path = "." }` for the general rules in `rules/` and
`local-rules = ".repo-rules"` for its own, `L12`. It then has local
rules beside the first-party ones, and all four kinds of source
([Four kinds of source](rule-sources.md#four-kinds-of-source)) can
occur in it. [The local site](statute-command.md#the-local-site)
lists a source at `.` once, as global public, and says in its local public section
that the repository's own rules are there; with `.repo-rules/`, the
local public section lists the repository's own rules itself.

## Parameters

A rule that states a principle leaves its concrete values to the
layers below (`rules/101.toml`). Some values are the same kind of
choice in every repository: the language of commit messages, of
comments in code, of replies to a person. For those, a rule names a
parameter in its text, and the rule set gives the value, as a linter's
configuration gives `line-length`. The first parameters are the
languages; a number, such as a count of review rounds, works the same
way later. A parameter is not an `adjustable` value
([Rules](rules.md#rules)): the rule set chooses a parameter's value
for every agent that reads its output, while an adjustable value is
one that a repository's or a task's text may relax under the
constitution's permission.

### Declaring a parameter

A parameter is a node of the rule tree, one file in `rules/` as a rule
is, with `kind = "parameter"` (the ID here is made up):

```toml
id = 9001
slug = "language-commits"
revision = 1
kind = "parameter"
status = "trial"
lang = "en"
parent = 77
name = "language.commits"
type = "language"
default = "en"

[title]
en = "Language of commit messages and pull request titles"
```

- `name`: two words joined by a dot, each lowercase words joined by
  hyphens, unique among the parameters of the source. The first word is
  the table of the rule set that sets it (below), so it may not be a
  key the rule set already has (`rules`, `packs`, `sources`,
  `compile` and the others). The name, not the ID, is what a rule's
  text and a rule set write, so that both read as what they mean.
- `type`: one of
  - `language`: a language code that statute has a name for (today
    `en` and `zh`);
  - `string`: any text that is not empty, on one line, with no space
    at either end and no backtick. A string is a name, so a rule writes
    its placeholder inside a code span itself
    (`` `{$work-items.label-approved}` ``), and statute fills in the
    value with no mark of its own;
  - `integer`, which comes with the first number.

  The build checks the `default` and every value a rule set sets
  against the type, and fails naming the parameter and the value.
  statute does not check that a string names anything that exists,
  such as a label on an issue tracker.
- `default`: the value a rule set that sets none gets. The key means
  the same on every kind of node, what a rule set gets when it says
  nothing: here a value, on a domain or a rule its switch, on a one-of
  its child ([Default switches](#default-switches)).
- `parent`: a domain, where the parameter is listed on the sites. A
  parameter belongs to no domain otherwise: rules of any domain may use
  it.
- `[title]` says what the value governs, and is translated as a
  domain's title is, with its hash in `[translations]`
  ([Rule file format](rule-format.md#rule-file-format)).

A parameter has no text, check, `requires` or default switch. It is
never enabled on its own: it is in a rule set when an enabled rule uses
it.

A parameter is changed in place when its title or its default changes,
and its revision goes up by one. A new default does change what an
agent is told, but only in a rule set that has not set the value; the
lock file records the value, so `upgrade` shows the change (below).

Declaring each parameter once, in its own file, was chosen over two
other places. In each rule that uses it, the default would be written
once per rule and could differ between them. In a domain, it would sit
in one domain, while the languages are used across domains: commit
messages under Git, replies under Collaboration.

### Placeholders

A rule's text names a parameter as `{$<name>}`:

```toml
[text.statement]
en = "Write commit messages and pull request titles in {$language.commits}."
zh = "commit message 与 PR 标题用{$language.commits}写。"
```

- The form joins that of citations, `{@...}`
  ([IDs](source-formats.md#ids)), and does not clash with `{{ }}`,
  which Hugo templates use and which the site's data passes through.
- A rule uses parameters of its own source and of the first-party
  source, by name alone. A name that two sources of one rule set declare
  fails the build, naming both.
- A placeholder may stand in a rule's title and any part of its text,
  and nowhere else. One that names no parameter fails the build, as a
  citation of a missing rule does.
- Each translation holds the same placeholders as the original, each
  as many times; the build checks it as it checks citations. A
  placeholder is never translated: the text around it is.
- The hash in `[translations]` covers the original text as written,
  placeholders included. Setting a value touches no rule file, so it
  changes no hash.

### Setting a value

A rule set sets a parameter in the table its name begins with:

```toml
[language]
communication = "zh"
docs = "zh"
```

- `statute.toml` alone sets values. `statute.private.toml` sets none:
  a value it set would change the text of a public rule in the full
  text, against the rule as the committed `AGENTS.md` writes it, and
  the private layer only adds
  ([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)).
- A key that names no parameter of the rule set's sources fails, and
  so does a value of the wrong type or a language code statute has no
  name for. A value set for a parameter no enabled rule uses draws a
  warning, so that a value stays in place while its rules are turned
  off.
- [The global configuration](global-configuration.md#the-global-configuration)
  may hold the same tables. Its values are
  not read when a rule set is compiled: `statute init` writes them into
  the new `statute.toml`, and they will be the values of the user-level
  output once that is built
  ([Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file),
  Where it lives). So the committed rule set and lock file alone still
  say what the compiled `AGENTS.md` holds, and CI, which has no global
  configuration, builds the same file.

The value a rule set gets is the first of: its `statute.toml`, the
parameter's `default`.

**The language parameters.** Six, each `en` by default:

| Name | Governs |
| --- | --- |
| `language.communication` | what an agent writes to a person: replies, questions, reports |
| `language.code-comments` | comments in code |
| `language.docs` | the repository's documents: Markdown, the design, other prose files |
| `language.commits` | commit messages and pull request titles |
| `language.pull-requests` | pull request descriptions, their comments and reviews |
| `language.work-items` | issues and other work items |

Commit messages and pull request descriptions are two parameters
because a common setup writes the first in English, for the history
and its tools, and the second in the language of the people who read
the repository. Code itself, its identifiers and the strings that are
part of its interface, has no parameter: it is in English.

**Names on the issue tracker.** The rules about work items name the
tracker's labels and statuses, which differ from one tracker or team
to another. Four `string` parameters are planned, under the domain
R14, each with a common name as its default:

| Name | Default | Governs |
| --- | --- | --- |
| `work-items.label-needs-decision` | `needs-user` | the label of an item that waits for a person to look or decide |
| `work-items.label-approved` | `approved` | the label of an item a person has approved |
| `work-items.status-done` | `Done` | the status of an attempt that ended as done |
| `work-items.status-abandoned` | `Abandoned` | the status of an attempt that was given up |

Until parameters are built, a rule that uses one writes the name and
the default in plain text, "(parameter `work-items.label-needs-decision`,
default `needs-user`)", and the change that builds them replaces that
with the placeholder. The table `work-items` and the key
`language.work-items` do not collide: one is a table of the rule set, the
other a key in the table `language`.

`[compile] lang` stays apart from the language parameters: it names the language the
compiled `AGENTS.md` is written in, which is a choice about the
agents' reading, while the parameters say what the agents write.

### Values in the lock file and the outputs

**The lock file** has a line for each parameter an enabled rule uses,
as for a rule, with the value the rule set gets:

```toml
[[rules]]
rule = "R9001"
revision = 1
sha256 = "<SHA-256 of rules/9001.toml>"
value = "zh"
```

A rule set whose value or whose parameter's default changed has a
stale lock file, as for a changed rule, and `upgrade` lists the
parameter with its old and new value.

**The compiled `AGENTS.md`** writes the value in place of the
placeholder, with no mark: a language as its name in the language of
the file (`Chinese`, `中文`), a string or a number as written. A rule's heading
and statement read as if the value had been written in the rule.

**`statute explain <name>`** says the value, where it came from (the
rule set or the default) and which enabled rules use it.

**The public site** shows a placeholder as its default, marked as a
value, with the parameter's name on hover, linked to the parameter's
page: its title, name, type, default and the rules that use it. The
list of rules shows parameters under their domains as it shows rules.
**The local site** shows the value of its rule set in the same way, and
on the parameter's page where the value came from.

## Default switches

Every domain and every rule declares whether it is on by default. A
rule set gets, without listing anything, the rules that are on in the
domains that are on, and lists only where it departs from that.

### In the rule files

`default` is required on every domain and rule:

```toml
id = 78
kind = "domain"
default = "off"
```

```toml
id = 44
kind = "rule"
default = "on"
```

- `on` or `off`. A parameter's `default` is its value
  ([Declaring a parameter](#declaring-a-parameter)), and a parameter is
  in a rule set when a rule uses it. The children of a one-of have no
  `default`; the one-of names one of them as its default (below).
- A domain is `off` when it is for some repositories or some ways of
  working only, as Fleet is for work divided among agents: none of its
  rules comes in until a rule set turns it on. A subdomain's `default`
  applies once its domain is on.
- A rule is `on` only when it is among the most general and the least
  likely to be wrong anywhere. A rule that depends on one person's
  preference or on the environment (a shared machine, a reviewer, a
  CI service, a tool) is `off`.
- The rules that a source's defaults enable must resolve on their own:
  within the source, the parent of an `on` rule, when that is a rule,
  and every rule it requires are `on`, and an `on` rule requires
  nothing of a domain that is `off` while its own is `on`. The build
  checks it.

**A one-of's default.** Every one-of names one of its children as its
default, in a required field:

```toml
id = 79
kind = "one-of"
parent = 16
default = 81
```

- `default` is the ID of a child of the one-of; the build fails when it
  names any other node.
- A one-of is on whenever its parent is, a domain that is on or a rule
  that is enabled: it then enables its default child, unless the rule set picks another child or turns
  the one-of off (below). So a repository that turns a domain on gets
  one working choice at every one-of without having to make it, and
  changes only the choices it cares about.
- A one-of that only some repositories need belongs in a domain or
  subdomain that is `off`, which those repositories turn on.
- The rules the default children bring in must resolve on their own, as
  `on` rules must (above).

Changing the `default` of a rule, a domain or a one-of does not change
what counts as compliant, so it is a change in place, and the revision goes up by
one ([IDs and revisions](rule-format.md#ids-and-revisions)). `upgrade`
lists a rule that comes in or drops out through it, as it lists one a
requirement brings in.

### In the rule set

The rule set lists only what differs from the defaults:

```toml
rules = ["R57", "R62"]
domains = ["R78"]
exclude = ["R83", "R66"]
packs = ["R96"]
```

- `rules`, as today, turns on a rule. For a rule that is `off`, or in
  a domain that is `off`, that is what turns it on; listing a rule that
  the defaults already enable draws a warning. Listing a child of a
  one-of picks it in place of the one-of's default.
- `domains` turns on a domain that is `off`: its rules follow their
  own `default`. Listing a domain that is `on` draws a warning.
- `exclude` turns off a domain, a one-of or a rule the defaults
  enable. Turning off a node turns off its subtree; a one-of turned off
  enables none of its children. A rule that an enabled rule
  requires cannot be turned off: the build fails, naming the rule that
  requires it.
- `packs`, as today, turns on a whole subtree, every rule in it,
  whatever its `default`; `exclude` turns off a part of it.
- `statute add` writes a domain to `domains` and a rule to `rules`;
  `statute remove` takes an entry out of `rules`, `domains` or `packs`,
  and when the rule or domain is still enabled by default, writes it to
  `exclude`.
- `statute.private.toml` may have `rules` and `domains`, which only
  add, and no `exclude`: the private layer leaves out no rule of the
  public one
  ([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)).

**Whose defaults.** A rule set takes the defaults of the first-party
source, `R`, wherever it comes from, and of each source its own
`[sources]` names: in `statute.toml`, or in `statute.private.toml` for
the private layer. A source that only the global configuration names
brings no default rule in; its rules are listed, or the rule set names
the source to take its defaults. Otherwise a source added to the
global configuration would enable rules in every repository with
nothing in their rule sets saying so. A source entry with
`defaults = false` takes none of that source's defaults:

```toml
[sources]
team = { git = "https://example.test/our-team/rules.git", branch = "main", public = true, defaults = false }
```

`defaults` is a key of a rule set's source entry only: in the global
configuration it fails, since that file enables no rule.

`statute init` writes no `rules`, `domains` or `exclude`: the new rule
set has the first-party defaults.

### Defaults in the lock file and the outputs

**The lock file** records each enabled rule as today. A rule the
defaults alone enable has `by = "default"` on its line, so that the
lock file says which rules came in without being chosen, and `upgrade`
lists those that a changed `default` brings in or drops.

**`statute explain <rule>`** says, for a rule or domain, whether it is
on by default and why the rule set has it or not: "R44 is on by
default, and its domain R13 is on"; "R66 is on by default and taken
out by `exclude`"; "R78 is off by default; `statute add R78` turns it
on". For a one-of it names the default child, and the child the rule
set picked when that is another. `statute explain --defaults` prints
three lists for the rule set:
the rules on by default that it keeps, those on by default that it
turns off, and those off by default that it turns on.

**The compiled `AGENTS.md`** is unchanged: it holds the enabled rules,
however they came in.

**The public site** marks each domain and rule as on or off by default
in the list and on its page, and each one-of's default child, and the list gets a filter that shows only
the rules on by default, so that a reader sees at a glance what a new
rule set holds. **The local site** shows the same marks beside whether
its rule set enables the rule.

### Moving a repository over

A repository moving onto statute, from the hand-written guide or from
a rule set written before default switches, starts from the defaults
and goes through three lists, which `statute explain --defaults`
prints once the rule set is written:

1. the rules on by default that the repository keeps: nothing to do;
2. those on by default that it must turn off: `exclude`, each with a
   comment that says why;
3. the domains and rules off by default that it needs: `domains` and
   `rules`, and each one-of where it picks another child than the
   default: `rules`.

A rule set written before default switches lists every rule it uses in
`rules` and `packs`; under the defaults it may also get rules it never
listed. The first `lock` after the switch lists them, and the move
puts each in the second list or keeps it. The skill that guides the
migration (not written yet) goes through the three lists in this order.

## Where a rule is used

A rule serves two readers: the agent at work, which reads the compiled
`AGENTS.md`, and the reviewer, which goes through the list that
`statute check` prints ([Outcomes and the review loop](rule-checks.md#outcomes-and-the-review-loop)).
Not every rule serves both. "Pass no credential from one agent to
another" (`rules/45.toml`) is something an agent must know while it
works, but a reviewer who has the diff and the pull request
description cannot tell whether it was broken. A requirement such as
"the head named in the description is the head reviewed" is only of
use to the reviewer. Most rules need not be reviewed at all.

**In the rule file**, `uses` lists where a rule goes, a non-empty
subset of two values:

```toml
uses = ["instructions", "review"]
```

- `instructions`: `statute compile` writes the rule into the compiled
  `AGENTS.md`.
- `review`: `statute check` lists the rule for the reviewer, with its
  reason and verification.
- Without `uses`, a rule has `["instructions"]`; `review` is added by
  writing it. An empty list fails the build, as does a value other
  than these two. Domains, one-ofs and parameters have no `uses`.

`uses` is apart from `check`, which says how a rule is checked by a
program: `builtin`, `command`, or `review` for no program at all
([Rule file format](rule-format.md#rule-file-format)). `statute check`
runs the program of every enabled rule that has one, whatever its
`uses`, and lists for review only the rules whose `uses` has `review`.
`check = "review"` then means only "no automatic check"; renaming it to
`none` would say so, and is left to the change that builds `uses`.
Changing `uses` does not change what counts as compliant, so the rule
is changed in place and its revision goes up by one.

**Which to choose.** A rule for domain R96 is proposed, to be written
with the change that adds `uses` to the rules: mark a rule `review`
only when a reviewer can decide from the diff and the pull request
description alone whether it was broken, and `instructions` only when
an agent needs to know it while it works.

**What follows from it.**

- The compiled `AGENTS.md` holds the enabled rules whose `uses` has
  `instructions`. A domain whose enabled rules are all review-only has
  no heading there, and a requirement or a citation of a review-only
  rule renders as one of a rule outside the file, its title followed
  by "(rule R42)".
- `statute check` lists, in its domains, the enabled rules whose
  `uses` has `review` and the outcome of every program it ran.
- `statute explain <rule>` says where an enabled rule goes: "R43 is
  compiled into `AGENTS.md` and listed for review".
- The lock file records nothing new: `uses` is part of the rule file,
  whose SHA-256 it pins.
- The sites show `uses` on a rule's page, and the list of rules gets a
  filter by it.

## Recommended defaults for the first-party rules

The first-party rules do not carry `default` yet. The table below is
the recommendation that the change adding the field follows; from then
on the rule files are the record, and this section goes; the same
holds for `uses`. It covers the
rules on the main branch on 2026-10-14, up to R126; superseded rules
are left out, since they are never enabled. A rule not listed below
under its domain is recommended `on`.

| Domain | Default | Why |
| --- | --- | --- |
| R13 Security | on | Wherever agents work |
| R14 Collaboration | on | Wherever an agent acts for a person; its review and work-item rules are off |
| R15 Verification | on | Wherever agents change a repository with checks |
| R16 Documentation | on | Wherever agents write documents |
| Design, a new subdomain of R16 | off | Only for a repository that keeps a design (below) |
| R77 Git | on | Nearly every repository agents commit to is a Git repository |
| R78 Fleet | off | Only for work divided among agents |
| R83 Code | on | Wherever agents write code; its narrower testing rules are off |
| R96 Writing rules | off | Only for a repository that writes rules |

Rules recommended `off` in a domain that is `on`, by the test above
(a preference of one person, or one environment):

| Rule | Why off |
| --- | --- |
| R30 No load generator on a shared machine | Environment: a shared machine |
| R31 Heavy-load problems in an isolated environment | Environment: a shared machine |
| R52 A reviewer reads what goes to a public repository | Environment: a reviewer agent |
| R93 Text through a shell only from a quoted heredoc | Preference about how to call commands |
| R94 Check sent text against its file | Its parent, R93, is off |
| R116 to R121: finding and taking work items | Environment: a place where work items are kept |
| R57 Changes only through reviewed pull requests | Preference: a solo repository may commit directly |
| R58 Release only a commit whose CI passed first time | Environment: CI on the main branch |
| R59 Read CI on the merge commit | Environment: CI on the main branch |
| R62 Never merge your own change | Preference about who merges |
| R63 Hold a review in the change's comments | Preference about where reviews live |
| R64 Three review rounds | Preference: the number of rounds |
| R65 Install nothing when a check needs a tool | Environment: a machine others share or manage |
| R71 Never set a change to merge automatically | Preference |
| R95 Keep merging while CI on main fails | Preference |
| R25 to R27 Fixtures for provisions | Environment: a repository that keeps fixtures for the constitution |
| R46 to R49 Instruction files in changes of their own | Preference about how instruction changes are reviewed |
| R56 Read the description back against the head | Preference about how a pull request is kept |
| R72 Cite the CI run, not a full local run | Preference about running the full suite locally |
| R69 Nothing in an agent tool's own memory | Preference |
| R50 Scan staged changes for credentials in the hooks | Environment: a scanner installed and hooks set up |
| R84 to R86 Narrow testing rules | Preference about which tests to write |
| R88 Comment only what the code cannot say | Preference about comment density |

The domains that are off (R78, R96) have every rule `on`: a rule set
that turns one on takes it whole.

**A subdomain for the design.** The rules about a repository's design
(R19, R21, R112 to R114, R122 to R125, and the one-of R79 with its
children R80 and R81) concern only a repository that keeps a design.
Recommended: a new subdomain "Design" under R16, `off`, holding them
all, each `on` inside it; a repository that keeps a design turns on
the subdomain alone. Moving them is a change of their `parent`, made
with the change that adds `default`.

**One-ofs.** R79, "Keep the design in step with the code", is the only
one-of so far. Recommended default: R81, the code and the design change
apart and every difference is marked, since it holds for a design
written before or after the code, while R80 asks every change to carry
its design along.

**Uses.** The rules recommended `["instructions", "review"]`, by the
proposed rule above: R1, R19, R21, R43, R46, R47, R50, R51, R56, R72,
R75, R80, R81, R84 to R89, R92, R97 to R106, R108 to R114, R122 to
R126. None is recommended review-only: each of these is also something
an agent needs while it works. The rest keep the default,
`["instructions"]`.

## Proposed: no rule under a rule

Open, for the person to decide. Until it is decided, the tree of
[Dependencies, one-ofs and packs](rule-sets.md#dependencies-one-ofs-and-packs)
and [Rule file format](rule-format.md#rule-file-format) stands, and the
default switches above are written for it; once the proposal is
accepted, a change to those two parts of the design carries it.

**The proposal.**

- A rule's parent is a domain or a one-of, never a rule. What a rule
  under a rule says today, that it refines the other, is written as
  `requires`.
- A one-of's parent is a domain.
- A one-of's options are its children, all rules or all domains, never
  some of each; the build checks it. An option that is one rule is that
  rule, with no domain around it; an option that needs several rules
  is a domain whose parent is the one-of, holding them. Domains still
  nest at most two levels, counting such a domain as one: a one-of in a
  subdomain has rules as its options.
- Every one-of names one of its options as its default
  ([A one-of's default](#in-the-rule-files)), as decided above.
- An option names its one-of in its `parent`, as a child of a one-of
  does today; the one-of lists no `options`. The tree stays recorded in
  `parent` alone ([Rule file format](rule-format.md#rule-file-format)),
  an option is added or moved by changing one file, and the rules of
  today need no change for it. The one-of's own file does not show its
  options; `statute explain` and the sites list them.
- In the compiled `AGENTS.md`, a domain's rules stand one level below
  its heading, and a one-of's enabled option stands at the one-of's
  place, with a line that names the one-of ("One of: Keep the design in
  step with the code."); an option that is a domain keeps its heading.
  On the sites, a one-of is a box in its domain's list that names each
  option and marks the default; an option appears in that box only, not
  again in the list.

**Why it is recommended.** A reader of the compiled file or the site
finds every rule directly under a domain or a one-of, with no rule
nested under a rule, and an option of one rule needs no domain of its
own. The special case of a rule's parent that counts as a requirement
goes, and so does a requirement passing through a one-of to the
one-of's own parent, since that parent is now always a domain. What is
lost is the indentation that showed a rule refining another; `requires`
keeps the relation, and the site shows it on both rules.

**What it changes.** One rule moves: R94, whose parent is R93; its
parent becomes R13 and it requires R93. R80 and R81, the children of
the one-of R79, stay as they are. `constitution-build` checks that no
rule's parent is a rule, that a one-of's parent is a domain and that
its children are all rules or all domains. statute drops the parent as
a requirement. The lock file does not change. Nothing of this is
built.

## What this changes

Not built: everything in this file. statute reads none of
`local-rules`, `domains`, `exclude`, `defaults` or the `[language]`
table, nor `kind = "parameter"`, `default` on a node, `uses` or a
placeholder,
and `constitution-build` checks none of them; the rule set and the
rule files fail on these keys today. The first-party rules have no
parameter and no `default` yet.

The parts of the design this one changes:

- [Choosing rules](rules.md#choosing-rules): its recommendation that a
  jurisdiction uses every general rule and leaves rules out by name
  gives way to the default switches.
- [Local parameters](rules.md#local-parameters): the parameter is a
  node of the source, its placeholder `{$<name>}`, and its value comes
  from the rule set, not from the overlay's `params.toml`.
- [Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file):
  the rule set gains `local-rules`, `domains`, `exclude` and the
  parameters' tables, a source entry `defaults`, and the lock file
  `by` and `value`.
- [Rule file format](rule-format.md#rule-file-format): the kind
  `parameter`, `default` on domains, rules and one-ofs, and `uses` on
  rules.
- [Outcomes and the review loop](rule-checks.md#outcomes-and-the-review-loop):
  `statute check` lists for review only the rules whose `uses` has
  `review`, and runs every rule's program as before.
- [Compilation and the site](statute-command.md#compilation-and-the-site):
  the compiled `AGENTS.md` holds only the rules whose `uses` has
  `instructions`.
- [Dependencies, one-ofs and packs](rule-sets.md#dependencies-one-ofs-and-packs):
  a one-of enables its default child when the rule set picks none.
