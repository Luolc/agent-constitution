# Rules and relaxations

## Rules

One rule per file, `rules/<id>.toml`, so that one proposal is one file and
a diff shows exactly one rule changing.

```toml
id = "credential-storage"
status = "draft"
scope = "all"
authority = ["constitution:safety", "constitution:sensitive-info"]

[title]
en = "<Title>"
zh = "<标题>"

[content]
en = '''
<statement: condition, action, exception, what to do when unsure>
'''
zh = '''
<正文>
'''
```

- `[title]` and `[content]` have one key per language of `index.toml`.
- The text cites the constitution with `{@constitution:<id>}` and another
  rule with `{@rule:<id>}` (see [IDs](source-formats.md#ids)); in the compiled instruction file a
  citation of a rule renders as that rule's title, and the rest of the
  rendering is set in stage B. A rule has no number.
- A rule about messages or notices says whom it covers: agent to agent,
  agent to person, or both.
- `id`: stable, also the file name. Renaming the title never changes it.
- `status`: `draft`, `active` or `repealed`. Only `active` is compiled. A
  repealed rule keeps its file and its ID. While the constitution is a
  draft, every rule is `draft` too.
- Unlike the constitution, a rule may be deleted. Its ID then goes on a line
  of its own in `rules/retired-ids.txt` and is never used again; the build
  rejects a rule whose ID is listed there. A missing file is an empty
  list.
- `scope`: who it addresses: `all`, or one of a short list defined in
  `schema/` (for example `implementer`, `reviewer`, `merger`). The compiler
  renders it as a one-line prefix; it does not filter by it. There is one
  compiled file for every role.
- `authority`: the constitution articles, sections or list items the rule
  rests on, cited as in the text (`constitution:<id>`); none of them
  repealed. A rule with no authority does
  not compile.
- `adjustable` (optional): the values in this rule that a lower-ranking text may
  change under `relaxations.adjustable-rules`, each with its default, for
  example `adjustable = ["wait_hours=24"]`. A rule without it cannot be
  relaxed that way. These are distinct from local parameters (below): a
  local parameter is a value the private overlay fills in; an adjustable
  value is one a repo or a task may change.

More fields (`supersedes`, `verification`, `expires`) are added when a real
rule needs them, not before.

Private rules in the overlay have the same format and share the rules'
namespace. The compiler checks that the union of core and overlay rule IDs
is unique and that none of them is retired in either.

## Choosing rules

The general rules are a default set, not obligations on every jurisdiction
(`division-of-instructions.rules`): one jurisdiction may need a rule that another has
good reason to drop (a repo that stays private on a self-hosted server may
keep credentials in it on purpose). Open, for the person to decide. Recommended:

- A jurisdiction uses every general rule by default. Its overlay leaves a rule
  out only by naming it, with a one-line reason, in `overlay.toml`
  (`exclude = ["credential-storage"]`). The build manifest lists
  what was left out.
- A repo inside a jurisdiction departs from a rule only by a relaxation
  declaration (below), where the constitution permits it. It does not leave
  rules out of the compiled file, because one compiled file serves all of a
  jurisdiction's repos.
- To change a rule rather than drop it, the overlay leaves it out and adds a
  private rule in its place.
- The constitution cannot be left out or relaxed this way.

Reasons: defaulting to all means a new rule reaches every jurisdiction without
anyone remembering to list it, where an opt-in list would go stale as rules
are added. Leaving a rule out is not a relaxation (`division-of-instructions.rules`),
but naming each one, with its reason, keeps it visible and puts everything a
jurisdiction dropped in one place. Exclusion per jurisdiction, not per repo, matches how the output is
built: one file per jurisdiction.

## Relaxation declarations

A lower-ranking text (a repo's `AGENTS.md`, or a task file) relaxes a rule only by
naming the constitution's permission for it:

```markdown
## Relaxations

- relaxes: constitution:relaxations.adjustable-rules
  target: rule:example-rule/wait_hours=4
  scope: <this repo, or this task>
  condition: <how this text meets the condition that item states>
```

One item per relaxation, with all four keys: the permission (an item of
`relaxations`, cited as a rule cites it), what it relaxes (a rule, a
boundary item such as `must-ask.means-of-production`, or under
`relaxations.adjustable-rules` a rule's adjustable value and its new
setting), where it applies, and how the condition of that item is met. A relaxation not declared this
way has no effect, whatever else the file says. The private overlay declares
no relaxations: it leaves rules out instead (see Choosing rules).

## Local parameters

A rule that needs a local value names a parameter instead of the value:
`{{ models.reviewer.default }}`. The overlay's `params.toml` supplies the
values. Parameters are plain values, never rule text, so the private side can
change a model name or a machine list without touching a rule. An
unresolved or unused parameter fails the build.
