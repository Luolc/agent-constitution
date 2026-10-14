# Rule format

Part of the [Rule system](rule-system.md), whose status and terms
apply here.

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
  superseded, names the rule that supersedes it (or the rules, when it
  is split into several), and stays in the source
  for good. Only a rule of the same source can supersede it (see
  [Sources and references](rule-sets.md#sources-and-references)).

The ID of a rule therefore always means the same requirement. The revision
is a number per rule, counted from 1; it has nothing to do with the
revision of the constitution. A rule's status is one of `draft`,
`trial`, `active`, `superseded` or `repealed`. `draft` is what every rule
is while the constitution is a draft; once the constitution is active, a
new rule starts as `trial`, a rule being tried on real work before it
settles (see [Trial rules](rule-checks.md#trial-rules)), and `draft` stays only for a
rule that is not meant for real work yet, such as the canary rule in
[Checkers](checkers.md). Moving a rule
from `draft` to `trial` changes only its status, not what counts as
compliant, so the rule is edited in place and its revision goes up by one,
as for any such change above. A rule
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
  this one, or a list of IDs when several rules together take its place,
  `superseded-by = [22, 23]`; present only when `status` is
  `superseded`. Each names another rule, and none twice; a list is never
  empty.
- `lang`: the language the rule was written in, its original, as a
  language code such as `"en"` or `"zh"`.
- `parent`: the ID of the parent node, a rule, a one-of or a domain of the
  same source. Every rule and one-of has one; a domain has none.
- `requires` (optional): the rules or one-ofs that enabling this rule
  requires. A node of the same source is written as a number, one of
  another source as a string of the source's short name, a colon and the
  ID (`"team:7"`), the name coming from the source's `[sources]` table
  ([Sources and references](rule-sets.md#sources-and-references)).
- `conflicts` (optional): the rules this rule cannot be used with,
  written like `requires` and on one side only
  ([Dependencies, one-ofs and packs](rule-sets.md#dependencies-one-ofs-and-packs)).
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
  ([Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file)).
- `check`: how compliance is checked ([Checks](rule-checks.md#checks)). `builtin`: the
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

## Rules and skills

A rule says what must be so; a skill says how to get there: steps,
templates, commands, known pitfalls (`skills`). What someone can point to
and say "you broke this" belongs in a rule; what only helps an agent get
it right belongs in a skill.

A skill declares which rules it implements, and it is installed only when
at least one of them is in the rule set. It declares them in the
`metadata` map of its front matter, under the key `statute-rules`: the
IDs of rules of its own source, separated by spaces.

```yaml
---
name: example-skill
description: How to do the thing that rules 42 and 51 require.
metadata:
  statute-rules: "42 51"
---
```

The [Agent Skills specification](https://agentskills.io/specification)
allows only `name`, `description`, `license`, `compatibility`,
`metadata` and `allowed-tools` in the front matter, and only strings as
the values of `metadata` (read 2026-10-14); hence a string under
`metadata` and not a key of its own. statute reads the declaration only
from a block mapping, as above, the key on its own line below
`metadata:`; a `metadata` in flow style (`metadata: {statute-rules:
"42"}`) that names the key fails the compile, naming the skill and the
line. A skill that declares no rules is
never installed, and one that names a rule its source does not have
fails the compile. A repository's `AGENTS.md` keeps
only what is specific to that repository (what it is, its language, its
layout, its check commands) and names the rule set it uses
(`division-of-instructions.project-instructions`). A rule set is chosen
per repository ([Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file)).

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
version specifier such as `">=1"` or `">=1,<4"`. Revisions are whole
numbers, so only the part of the specifier syntax that compares whole
numbers is accepted: one or more clauses joined by commas, each one of
`>=`, `<=`, `>`, `<`, `==` or `!=` followed by a revision, with spaces
allowed around them. `~=1`, `===1`, `==1.*`, `>=1.0`, a bare `1`, an empty
string and a trailing comma are errors. The lowest revision is
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
