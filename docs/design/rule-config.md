# Rule configuration

Status: decided; partly built. What is built is under
[The command and the first version](statute-command.md#the-command-and-the-first-version),
and [What this changes](#what-this-changes) lists what is not.
This part of the design says three things a rule set can set beyond
the list of its rules: the directory of the repository's own rules,
the values of parameters that rules leave open (the languages first),
and which domains and rules come in without being listed, through a
switch each of them declares; and one thing each rule says of itself,
whether it goes to the agents at work, to the reviewer or to both. It
also settles the shape of the rule tree: no rule hangs under a rule. It
builds on
[Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file),
[Rule file format](rule-format.md#rule-file-format) and
[Rule sources](rule-sources.md), and on
[the global configuration](global-configuration.md#the-global-configuration)
in the user's configuration directory, `$XDG_CONFIG_HOME/statute/config.toml`
(`~/.config/statute/config.toml` by default), which names
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
- [No rule under a rule](#no-rule-under-a-rule)
- [What this changes](#what-this-changes)

## The directory of local rules

The repository's own rules, `L12`, are in `.statute/rules/` next to
`statute.toml` ([Four kinds of source](rule-sources.md#four-kinds-of-source)).
`local-rules` at the top of the rule set names another directory,
relative to that of `statute.toml`:

```toml
local-rules = "policy/agents"
```

- The default is `".statute/rules"`. The `rules/` at the top of a
  repository is not read as its own rules: it is a source's, read only
  when a source is at that directory, as `R = { path = "." }`.
  `rules/` is too common a name for statute to claim in a repository
  that uses it, and in `.statute/` the rules sit with statute's other
  files, as a tool's committed files sit in `.claude/` or
  `.github/workflows/`. `.statute/rules/` is committed; `.statute/`
  lists only the files statute generates as ignored
  ([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)).
- Only `statute.toml` sets it. It describes the repository's layout,
  which every machine and CI share, so the global configuration has no
  such key, and the private rule set neither, since the private layer
  only adds.
- A source on disk, as `R = { path = "." }`, is still read from
  `<path>/rules/`, or the directory its `rules-dir` names
  ([Sources and references](rule-sets.md#sources-and-references)):
  those are the source's rules, not local ones. When `local-rules`
  names that same directory, the rules belong to the source: a local
  reference to one fails with a note to write `R12`.
- The directory may hold `params.toml`, the declarations of its
  parameters ([Parameters](#parameters)); statute does not read it as
  a rule.

This repo is the first-party source and holds rules for its own
development as well, rules that bind this repository and are no
general rule for others. Its rule set, once it has one, names
`R = { path = "." }` for the general rules in `rules/` and keeps its
own, `L12`, in `.statute/rules/` with the default. It then has local
rules beside the first-party ones, and all four kinds of source can
occur in it. [The local site](statute-command.md#the-local-site)
lists the first-party rules as global public and this repo's own rules
in its local public section.

## Parameters

Built, but for what the public site shows of parameters (below).

A rule that states a principle leaves its concrete values to the
layers below (`rules/101.toml`). Some values are the same kind of
choice in every repository: the language of commit messages, of
comments in code, of replies to a person. For those, a rule names a
parameter in its text, and the rule set gives the value, as a linter's
configuration gives `line-length`. The first parameters are the
languages and a few names on an issue tracker. A parameter is not an
`adjustable` value ([Rules](rules.md#rules)): the rule set chooses a
parameter's value for every agent that reads its output, while an
adjustable value is one that a repository's or a task's text may relax
under the constitution's permission.

### Declaring a parameter

Each source declares its parameters in one file, `params.toml`, in the
directory of its rules: `rules/params.toml` in this repo, and for the
repository's own rules, `params.toml` in the directory of local rules
([The directory of local rules](#the-directory-of-local-rules)).
statute does not read the file as a rule, and a source without one
declares no parameter. Each parameter is a table of the array
`parameter`:

```toml
[[parameter]]
name = "language.commits"
type = "language"
scope = "repo"
default = "en"

[parameter.title]
en = "Language of commit messages and pull request titles"
zh = "commit message 与 PR 标题的语言"

[parameter.translations]
zh = "<SHA-256 of the English title>"
```

- `name`: two words joined by a dot, each lowercase letters and digits
  joined by hyphens, unique among the parameters of the source. The
  first word is the table of the rule set that sets it (below), so it
  may not be a key the rule set already has: `constitution`, `public`,
  `rules`, `packs`, `sources`, `compile`, `domains`, `exclude`,
  `local-rules` or `base`. The name is what a rule's text and a rule set
  write, so that both read as what they mean.
- `type`: one of
  - `language`: a language code that statute has a name for (today
    `en` and `zh`);
  - `string`: any text that is not empty, on one line, with no space
    at either end and no backtick. A string is a name, so a rule writes
    its placeholder inside a code span itself
    (`` `{$work-items.label-approved}` ``), and statute fills in the
    value with no mark of its own;
  - `choice`: one of the strings that `values` lists, which is
    required with this type and only with it, not empty, with no value
    twice, each value a `string` as above. A `language` is a `choice`
    whose values statute knows.

  A number type comes with the first parameter that needs one.
  statute checks the `default` and every value a rule set sets
  against the type, and fails naming the parameter and the value.
  It does not check that a string names anything that exists, such as
  a label on an issue tracker.
- `scope`: `repo` or `personal`, required; it says who sets the value
  ([Setting a value](#setting-a-value)).
- `default`: the value a rule set that sets none gets. The key means
  the same as on a node of the tree, what a rule set gets when it says
  nothing: here a value, on a domain or a rule its switch, on a one-of
  its option ([Default switches](#default-switches)).
- `[title]` says what the value governs. A parameter has no `lang`: its
  original is the one language of `[title]` that `[translations]` does
  not name, and there must be exactly one. `[translations]` holds the
  SHA-256 of the original title, as for a domain without an
  applicability ([Rule file format](rule-format.md#rule-file-format)).

Any other key fails. A parameter that takes a value from a fixed list:

```toml
[[parameter]]
name = "python.quote-style"
type = "choice"
values = ["double", "single"]
scope = "repo"
default = "double"

[parameter.title]
en = "Quotes around strings in Python code"
```

A parameter has no ID, revision, status or parent: it is no node of
the rule tree, is never enabled on its own and has no line among the
lock file's rules. It is in a rule set when an enabled rule uses it.
Changing its title or its default touches no rule file. A new default
does change what an agent is told, but only in a rule set that has not
set the value; the lock file records the value, so `upgrade` shows the
change (below).

One file per source was chosen over three other places. As a node of
the rule tree, a parameter would get an ID, a revision, a status and a
parent domain, all of which serve a rule that is enabled, cited and
superseded, while a parameter is none of these; and its domain would be
one place in the tree, while the languages are used across domains:
commit messages under Git, replies under Collaboration. In each rule
that uses it, the default would be written once per rule and could
differ between them. In a domain's file, it would sit in one domain
again. One file holds a source's parameters together, where a reader
finds them all.

### A parameter or a one-of

Two ways of doing something that differ only in a value that fills
one sentence are one rule with a parameter; two that differ in the
requirement itself, its sentences, its practice or its check, are the
options of a one-of, each a whole rule (`rules/230.toml`). A parameter
only fills in a value: a rule's text never branches on it ("if A, do
this; if B, do that"), and statute has no conditions or expressions in
a rule's text. A text that would branch is split into a one-of.

The type `choice` follows linters that keep a rule apart from its
settings: ruff selects rules by code and sets `flake8-quotes.inline-quotes`
apart, and ESLint gives a rule its options, `quotes: ["error", "double"]`.
A rule says what must be so; a setting says which of a few values it
takes.

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
  source, by name alone. A name that both declare fails, naming both.
- A placeholder may stand in a rule's title and the three parts of its
  text, and nowhere else: one in a domain's title or applicability, a
  one-of's title or a parameter's title fails. One whose name does not
  have the shape of a name, or names no parameter, fails, as a citation
  of a missing rule does.
- Each translation holds the same placeholders as the original, each
  as many times; statute checks it as it checks citations. A
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

A parameter's `scope` says who sets it:

- **`repo`**: what goes into the repository, the same for everyone who
  works in it, such as the language of its documents or its commit
  messages. Only `statute.toml` sets it. The global configuration and
  the private rule set, `private.toml`
  ([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)),
  may not: a value of this kind set there draws a warning that names
  the parameter and the file, and is ignored.
- **`personal`**: what concerns only the person an agent works for,
  today only the language in which an agent writes to that person.
  `statute.toml` gives the repository's value, and
  [the global configuration](global-configuration.md#the-global-configuration)
  and the private rule set may each set the person's own, which the
  private output takes (below).

The value a `repo` parameter gets is the first of: `statute.toml`, the
parameter's `default`. A `personal` parameter's value in the private
output is the first of: the private rule set, the global
configuration, `statute.toml`, the `default`; in the committed
`AGENTS.md` it is the first of the last two.

- A key that names no parameter of the rule set's sources fails, and
  so does a value of the wrong type or a language code statute has no
  name for; the global configuration serves every repository, so a key
  there that names no parameter of this rule set's sources is left
  alone. A value set for a parameter no enabled rule uses draws a
  warning, so that a value stays in place while its rules are turned
  off.
- `statute init` copies no value from the global configuration: a new
  rule set starts from the defaults, and a person's own values reach
  the private output from the global configuration directly.

The committed files do not change with who compiles them: the
committed `AGENTS.md` and `statute.lock` hold the values of
`statute.toml` and the defaults only, so CI, which has no global
configuration, builds the same file, and everyone reads the same
rules. What only one person wants stays out of them.

**The language parameters.** Six, each `en` by default:

| Name | Scope | Governs |
| --- | --- | --- |
| `language.communication` | `personal` | what an agent writes to a person: replies, questions, reports |
| `language.code-comments` | `repo` | comments in code |
| `language.docs` | `repo` | the repository's documents: Markdown, the design, other prose files |
| `language.commits` | `repo` | commit messages and pull request titles |
| `language.pull-requests` | `repo` | pull request descriptions, their comments and reviews |
| `language.work-items` | `repo` | issues and other work items |

Commit messages and pull request descriptions are two parameters
because a common setup writes the first in English, for the history
and its tools, and the second in the language of the people who read
the repository. Code itself, its identifiers, file names and the
strings that are part of its interface, has no parameter: it is in
English, which is decided.

**Names on the issue tracker.** The rules about work items, in the
domain R175 (`tickets`), name the tracker's labels and statuses, which
differ from one tracker or team to another. Four `string` parameters of
scope `repo` name them, each with a common name as its default:

| Name | Default | Governs |
| --- | --- | --- |
| `work-items.label-needs-decision` | `needs-user` | the label of an item that waits for a person to look or decide |
| `work-items.label-approved` | `approved` | the label of an item a person has approved |
| `work-items.status-done` | `Done` | the status of an attempt that ended as done |
| `work-items.status-abandoned` | `Abandoned` | the status of an attempt that was given up |

The ten parameters are declared in `rules/params.toml`. The table
`work-items` and the key `language.work-items` do not collide: one is
a table of the rule set, the other a key in the table `language`.

`[compile] lang` stays apart from the language parameters: it names the language the
compiled `AGENTS.md` is written in, which is a choice about the
agents' reading, while the parameters say what the agents write.

### Values in the lock file and the outputs

**The lock file** has a line for each parameter an enabled rule uses,
apart from the rules, with the value the committed `AGENTS.md` gets:

```toml
[[parameters]]
name = "language.commits"
value = "zh"
```

A rule set whose value or whose parameter's default changed has a
stale lock file, as for a changed rule, and `upgrade` lists the
parameter with its old and new value. A person's own values, from the
global configuration and the private rule set, are in no lock file:
statute reads them each time it compiles the private output.

**The compiled `AGENTS.md`** writes the value in place of the
placeholder, with no mark: a language as its name in the language of
the file (`Chinese`, `中文`), a string as written. A rule's heading
and statement read as if the value had been written in the rule.

**The private output**, `.statute/AGENTS.full.md`, writes the person's
own values of the `personal` parameters, and is otherwise the same:
with no value of the person's own, the two files say the same. In a
public repository, a `personal` value that differs from the committed
one makes `statute compile` write the private output even when there
is no private rule set, and `statute compile --private-only` writes it
then too. A private repository (`public = false`) has no private
output, so a person's own value there is not used, and
`statute compile` says so in a note, naming the parameter.

**`statute explain <name>`** says the value, where it came from
(`statute.toml`, the default, and for a `personal` parameter the
private rule set or the global configuration, with both values when
they differ) and which enabled rules use it.

**The public site** (not built yet) has a page of parameters for each
source: each parameter's title, name, type, scope, default (and
values, for a `choice`) and the rules that use it. A placeholder shows
its default, marked as a value, with the parameter's name on hover,
linked to its entry on that page. **The local site** shows the value
of its rule set in the same way, and on the page of parameters where
each value came from.

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

- `on` or `off`. The children of a one-of have no `default`; the
  one-of names one of them as its default (below).
- A rule with `overrides` has no `default` either: it takes the place
  of the rule it overrides
  ([Overrides](rule-sources.md#overrides)), whose place decides whether
  it comes in, and when that rule is an option of a one-of, a switch of
  its own would contradict the one-of's default.
- A domain is `off` when it is for some repositories or some ways of
  working only, as Fleet is for work divided among agents: none of its
  rules comes in until a rule set turns it on. A subdomain's `default`
  applies once its domain is on.
- A rule is `on` only when it is among the most general and the least
  likely to be wrong anywhere. A rule that depends on one person's
  preference or on the environment (a shared machine, a reviewer, a
  CI service, a tool) is `off`.
- The rules that a source's defaults enable must resolve on their own,
  and statute checks it. A rule in force (neither superseded nor
  repealed) is enabled by default when its own `default` is `on`, every
  domain above it is `on`, and every one-of on the way names as its
  default the option that leads to it. Each rule of the same source
  that such a rule requires must be enabled by default too; a required
  one-of counts as met.

statute reads `default`, fails on a node that lacks it or has it where
the list above says it has none, and makes the checks above and below;
a one-of with no option yet has no `default` to name. Every first-party
rule, domain and one-of carries it: the rule files are the record of the first-party
defaults, and `statute explain --defaults` lists what a rule set gets
from them (below).

**A one-of's default.** Every one-of names one of its options, its
children, as its default, in a required field:

```toml
id = 79
kind = "one-of"
parent = 232
default = 81
```

- `default` is the ID of a child of the one-of; statute fails when it
  names any other node.
- A one-of is on whenever its domain is: it then enables its default
  option, unless the rule set picks another option or turns the one-of
  off (below). So a repository that turns a domain on gets one working
  choice at every one-of without having to make it, and changes only
  the choices it cares about. An option that is a domain brings in its
  rules each by its own `default`, as a domain that `domains` turns on
  does ([No rule under a rule](#no-rule-under-a-rule)).
- A one-of that only some repositories need belongs in a domain or
  subdomain that is `off`, which those repositories turn on.
- The rules the default options bring in must resolve on their own, as
  `on` rules must (above).

Changing the `default` of a rule, a domain or a one-of does not change
what counts as compliant, so it is a change in place, and the revision goes up by
one ([IDs and revisions](rule-format.md#ids-and-revisions)). `upgrade`
lists a rule that comes in or drops out through it, as it lists one a
requirement brings in.

### In the rule set

Built, but for the marks of the sites (below).

The rule set lists only what differs from the defaults:

```toml
rules = ["R57", "R62"]
domains = ["R78"]
exclude = ["R83", "R66"]
packs = ["R96"]
```

- `rules`, as today, turns on a rule. For a rule that is `off`, or in
  a domain that is `off`, that is what turns it on; listing a rule that
  the defaults already enable draws a warning. Listing an option of a
  one-of that is a rule picks it in place of the one-of's default.
- `domains` turns on a domain that is `off`: its rules follow their
  own `default`. Listing a domain that is `on` draws a warning.
  Listing an option of a one-of that is a domain picks it, as `rules`
  picks one that is a rule.
- `exclude` turns off a domain, a one-of or a rule the defaults
  enable. Turning off a node turns off its subtree; a one-of turned off
  enables none of its options. A rule that an enabled rule
  requires cannot be turned off: the build fails, naming the rule that
  requires it. An option of a one-of, the default or any other, cannot
  be listed in `exclude`: the build fails, with a note to list another
  option in `rules` or `domains`, or to exclude the whole one-of. A node
  listed both in `exclude` and in `rules` or `packs` fails as well,
  with a note to keep one of the two.
- `packs`, as today, turns on a whole subtree, every rule in it,
  whatever its `default`, but at a one-of only one option: the one the
  rule set names in `rules` or `domains`, or else the default, and an
  option that is a domain with every rule in it. `exclude` turns off a
  part of it.
- A rule that requires a one-of of which the rule set enables no option
  brings in the default option, as statute brings in any missing
  requirement, instead of failing.
- `statute add` writes a domain to `domains` and a rule to `rules`;
  `statute remove` takes an entry out of `rules`, `domains` or `packs`,
  and when the rule or domain is still enabled by default, writes it to
  `exclude`.
- `private.toml` may have `rules` and `domains`, which only
  add, and no `exclude`: the private layer leaves out no rule of the
  public one
  ([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)).

**Whose defaults.** A rule set takes the defaults of the first-party
source, `R`, wherever it comes from, and of each source its own
`[sources]` names: in `statute.toml`, or in `private.toml` for
the private layer, whose defaults count for that layer alone. The
repository's own rules, `L`, count as such a source: their `default`
holds as the first-party rules' does. A source that only the global configuration names
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
on". For a one-of it names the default option, and the option the rule
set picked when that is another. `statute explain --defaults` prints
three lists for the rule set:
the rules on by default that it keeps, those on by default that it
turns off, and those off by default that it turns on.

**The compiled `AGENTS.md`** is unchanged: it holds the enabled rules,
however they came in.

**The public site** marks each domain and rule as on or off by default
in the list and on its page, and each one-of's default option, and the list gets a filter that shows only
the rules on by default, so that a reader sees at a glance what a new
rule set holds. **The local site** shows the same marks beside whether
its rule set enables the rule. The public site does not show them yet.

### Moving a repository over

A repository moving onto statute, from the hand-written guide or from
a rule set written before default switches, starts from the defaults
and goes through three lists, which `statute explain --defaults`
prints once the rule set is written:

1. the rules on by default that the repository keeps: nothing to do;
2. those on by default that it must turn off: `exclude`, each with a
   comment that says why;
3. the domains and rules off by default that it needs: `domains` and
   `rules`, and each one-of where it picks another option than the
   default: `rules` or `domains`.

A rule set written before default switches lists every rule it uses in
`rules` and `packs`; under the defaults it may also get rules it never
listed. The first `lock` after the switch lists them, and the move
puts each in the second list or keeps it. The skill that guides the
migration, [statute-migrate](../../skills/statute-migrate/SKILL.md),
goes through the three lists in this order.

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
  writing it. An empty list fails, as does a value other than these
  two or one listed twice. Domains and one-ofs have no `uses`, nor do
  parameters, which are no nodes.

`uses` is apart from `check`, which says how a rule is checked by a
program: `builtin`, `command`, or `none` for no program at all
([Rule file format](rule-format.md#rule-file-format)). `statute check`
runs the program of every enabled rule that has one, whatever its
`uses`, and lists for review only the rules whose `uses` has `review`.
`none` was called `review` before `uses` existed; statute fails on
`check = "review"` and says to write `none`.
Changing `uses` does not change what counts as compliant, so the rule
is changed in place and its revision goes up by one.

**Which to choose.** Rule R233 (`rules/233.toml`) says it: mark a
rule `review` only when a reviewer can decide from the diff and the
pull request description alone whether it was broken, and
`instructions` only when an agent needs to know it while it works.

**What follows from it.** statute reads `uses`, and the compiled
`AGENTS.md`, `statute check`, `statute explain` and the local site
follow it; what the public site shows of it is not built yet.

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

## No rule under a rule

The tree of
[Dependencies, one-ofs and packs](rule-sets.md#dependencies-one-ofs-and-packs)
has this shape, and statute checks it when it reads a source:

- A rule's parent is a domain or a one-of, never a rule. What a rule
  under a rule used to say, that it refines the other, is written as
  `requires`.
- A one-of's parent is a domain.
- A one-of's options are its children, all rules or all domains, never
  some of each. An option that is one rule is that rule, with no domain
  around it; an option that needs several rules is a domain whose
  parent is the one-of, holding them; when it is the option enabled,
  its rules come in each by its own `default`, as in a domain that
  `domains` turns on. Domains still nest at most two levels, counting
  such a domain as one: a one-of in a subdomain has rules as its
  options.
- Every one-of names one of its options as its default
  ([A one-of's default](#in-the-rule-files)).
- An option names its one-of in its `parent`; the one-of lists no
  `options`. The tree stays recorded in `parent` alone
  ([Rule file format](rule-format.md#rule-file-format)), and an option
  is added or moved by changing one file. The one-of's own file does
  not show its options; `statute explain` and the sites list them.
- In the compiled `AGENTS.md`, a domain's rules stand one level below
  its heading, and a one-of's enabled option stands at the one-of's
  place, with a line that names the one-of ("One of: Keep the design in
  step with the code."); an option that is a domain keeps its heading.
  On the sites, a one-of is a box in its domain's list that names each
  option and marks the default; an option appears in that box only, not
  again in the list. The compiled `AGENTS.md` is built so; the box on
  the sites is not built yet.

**Why.** A reader of the compiled file or the site finds every rule
directly under a domain or a one-of, with no rule nested under a rule,
and an option of one rule needs no domain of its own. The special case
of a rule's parent that counted as a requirement is gone, and so is a
requirement passing through a one-of to the one-of's own parent, since
that parent is always a domain. What is lost is the indentation that
showed a rule refining another; `requires` keeps the relation, and the
site shows it on both rules.

**What it changed.** One rule moved: R94, whose parent was R93, now
hangs under R13 and requires R93. statute no longer counts a parent as
a requirement; the lock file did not change.

## What this changes

Built: statute reads `default` on domains, rules and one-ofs, required
where the rule files need it, `uses` and `check = "none"`, fails on
`check = "review"`, makes the checks of the defaults and of the tree
above, and follows `uses` in the compiled `AGENTS.md` and in
`statute check`; `statute validate`
([The command and the first version](statute-command.md#the-command-and-the-first-version))
runs these checks on a directory of rules. The first-party rules carry
`default` and `uses`, and `rules/params.toml` declares their
parameters.

The default switches of a rule set are built too: `domains`,
`exclude`, `local-rules` and a source entry's `defaults`, the defaults
taking effect, `by = "default"` in the lock file, and what `statute
add`, `remove` and `explain` (`--defaults` included) do with them.

Parameters are built: `params.toml` and the placeholders are read and
checked, the rule set, the private rule set and the global
configuration set values, and the lock file, `upgrade`, `explain
<name>` and both outputs follow them.

The local site marks the default switches, each one-of's default
option, `uses` and the values of parameters
([The local site](statute-command.md#the-local-site)).

The compiled `AGENTS.md` puts each one-of's enabled option in the
one-of's place, with the line that names the one-of.

Not built yet: the one-of as a box on the sites, and what the public
site shows of these.

The parts of the design this one changes:

- [Choosing rules](rules.md#choosing-rules): its recommendation that a
  jurisdiction uses every general rule and leaves rules out by name
  gives way to the default switches.
- [Local parameters](rules.md#local-parameters): the parameter is
  declared in its source's `params.toml`, its placeholder is
  `{$<name>}`, and its value comes from the rule set, or for a
  `personal` one from the person, not from the overlay's `params.toml`.
- [Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file):
  the rule set gains `local-rules`, `domains`, `exclude` and the
  parameters' tables, a source entry `defaults`, and the lock file
  `by` and `[[parameters]]`.
- [Rule file format](rule-format.md#rule-file-format): `default` on
  domains, rules and one-ofs, `uses` on rules, `check = "none"`, and
  the parents a node may have.
- [Outcomes and the review loop](rule-checks.md#outcomes-and-the-review-loop):
  `statute check` lists for review only the rules whose `uses` has
  `review`, and runs every rule's program as before.
- [Compilation and the site](statute-command.md#compilation-and-the-site):
  the compiled `AGENTS.md` holds only the rules whose `uses` has
  `instructions`.
- [Dependencies, one-ofs and packs](rule-sets.md#dependencies-one-ofs-and-packs):
  no rule under a rule, and a one-of enables its default option when
  the rule set picks none. Its "Missing requirements are added", where
  a requirement of a one-of with no option enabled fails, and its
  "Packs", where a one-of of which the rule set names no option
  contributes nothing, both take the default option instead.
