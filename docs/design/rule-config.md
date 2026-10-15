# Rule configuration

Status: decided; not built yet. What is built is under
[The command and the first version](statute-command.md#the-command-and-the-first-version).
This part of the design says three things a rule set can set beyond
the list of its rules: the directory of the repository's own rules,
the values of parameters that rules leave open (the languages first),
and which domains and rules come in without being listed, through a
switch each of them declares. It builds on
[Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file),
[Rule file format](rule-format.md#rule-file-format) and
[Rule sources](rule-sources.md), and on the global configuration in
the user's statute directory, `~/.statute/config.toml`, which names
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
- [Recommended defaults for the first-party rules](#recommended-defaults-for-the-first-party-rules)
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
occur in it. The local site that `statute site` writes lists a source
at `.` once, as global public, and says in its local public section
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
is, with `kind = "parameter"`:

```toml
id = 130
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
- `type`: `language`, a language code that statute has a name for
  (today `en` and `zh`); `integer` comes with the first number.
- `default`: the value a rule set that sets none gets.
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
- The global configuration may hold the same tables. Its values are
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
| `language.tickets` | issues and other work items |

Commit messages and pull request descriptions are two parameters
because a common setup writes the first in English, for the history
and its tools, and the second in the language of the people who read
the repository. Code itself, its identifiers and the strings that are
part of its interface, has no parameter: it is in English.

`[compile] lang` stays apart from these: it names the language the
compiled `AGENTS.md` is written in, which is a choice about the
agents' reading, while the parameters say what the agents write.

### Values in the lock file and the outputs

**The lock file** has a line for each parameter an enabled rule uses,
as for a rule, with the value the rule set gets:

```toml
[[rules]]
rule = "R130"
revision = 1
sha256 = "<SHA-256 of rules/130.toml>"
value = "zh"
```

A rule set whose value or whose parameter's default changed has a
stale lock file, as for a changed rule, and `upgrade` lists the
parameter with its old and new value.

**The compiled `AGENTS.md`** writes the value in place of the
placeholder, with no mark: a language as its name in the language of
the file (`Chinese`, `中文`), a number as written. A rule's heading
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

- `on` or `off`. A one-of and a parameter have no `default`: a one-of
  is on when one of its children is, and a parameter when a rule uses
  it.
- A domain is `off` when it is for some repositories or some ways of
  working only, as Fleet is for work divided among agents: none of its
  rules comes in until a rule set turns it on. A subdomain's `default`
  applies once its domain is on.
- A rule is `on` only when it is among the most general and the least
  likely to be wrong anywhere. A rule that depends on one person's
  preference or on the environment (a shared machine, a reviewer, a
  CI service, a tool) is `off`.
- At most one child of a one-of is `on`.
- The rules that a source's defaults enable must resolve on their own:
  within the source, the parent of an `on` rule, when that is a rule,
  and every rule it requires are `on`, and an `on` rule requires
  nothing of a domain that is `off` while its own is `on`. The build
  checks it.

Changing a rule's or a domain's `default` does not change what counts
as compliant, so it is a change in place, and the revision goes up by
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
  one-of turns off the child that is `on` by default.
- `domains` turns on a domain that is `off`: its rules follow their
  own `default`. Listing a domain that is `on` draws a warning.
- `exclude` turns off a domain or a rule the defaults enable. Turning
  off a rule turns off its subtree. A rule that an enabled rule
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
on". `statute explain --defaults` prints three lists for the rule set:
the rules on by default that it keeps, those on by default that it
turns off, and those off by default that it turns on.

**The compiled `AGENTS.md`** is unchanged: it holds the enabled rules,
however they came in.

**The public site** marks each domain and rule as on or off by default
in the list and on its page, and the list gets a filter that shows only
the rules on by default, so that a reader sees at a glance what a new
rule set holds. **The local site** shows the same mark beside whether
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
   `rules`.

A rule set written before default switches lists every rule it uses in
`rules` and `packs`; under the defaults it may also get rules it never
listed. The first `lock` after the switch lists them, and the move
puts each in the second list or keeps it. The skill that guides the
migration (not written yet) goes through the three lists in this order.

## Recommended defaults for the first-party rules

The first-party rules do not carry `default` yet. The table below is
the recommendation that the change adding the field follows; from then
on the rule files are the record, and this section goes. It covers the
rules in force on the main branch on 2026-10-14 and those added by the
pull request that splits rules 4, 20 and 82 (112 to 124); R4, R20 and
R82, which it supersedes, are listed with their successors. Rules
superseded before are left out, since they are never enabled. A rule
not listed below under its domain is recommended `on`.

| Domain | Default | Why |
| --- | --- | --- |
| R13 Security | on | Wherever agents work |
| R14 Collaboration | on | Wherever an agent acts for a person; its review and work-item rules are off |
| R15 Verification | on | Wherever agents change a repository with checks |
| R16 Documentation | on | Wherever agents write documents; its design rules are off |
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
| R4, R116 to R121: finding and taking work items | Environment: a place where work items are kept |
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
| R19 to R21, R82, R112 to R114, R122 to R124, and R80 and R81 under the one-of R79: the design | Environment: a repository that keeps a design |
| R69 Nothing in an agent tool's own memory | Preference |
| R50 Scan staged changes for credentials in the hooks | Environment: a scanner installed and hooks set up |
| R84 to R86 Narrow testing rules | Preference about which tests to write |
| R88 Comment only what the code cannot say | Preference about comment density |

The domains that are off (R78, R96) have every rule `on`: a rule set
that turns one on takes it whole.

## What this changes

Not built: everything in this file. statute reads none of
`local-rules`, `domains`, `exclude`, `defaults` or the `[language]`
table, nor `kind = "parameter"`, `default` on a node or a placeholder,
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
  `parameter`, and `default` on domains and rules.
