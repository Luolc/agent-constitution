# Rules and relaxations

## Rules

One node of the rule tree per file, `rules/<id>.toml`, so that one
proposal is one file and a diff shows exactly one rule changing. The
format, the IDs and the statuses are in
[Rule file format](rule-format.md#rule-file-format) and
[IDs and revisions](rule-format.md#ids-and-revisions).

- The text cites the constitution with `{@constitution:<id>}` and another
  rule with `{@rule:<number>}` (see [IDs](source-formats.md#ids)); in the
  compiled instruction file a citation of a rule renders as that rule's
  title, and the rest of the rendering is set in stage B.
- A rule about messages or notices says whom it covers: agent to agent,
  agent to person, or both.
- While the constitution is a draft, every rule is `draft` too; once it is
  active, a new rule starts as `trial`
  ([Trial rules](rule-checks.md#trial-rules)).
- `adjustable` (optional): the values in this rule that a lower-ranking text may
  change under `relaxations.adjustable-rules`, each with its default, for
  example `adjustable = ["wait_hours=24"]`. A rule without it cannot be
  relaxed that way. These are distinct from local parameters (below): a
  local parameter is a value the private overlay fills in; an adjustable
  value is one a repo or a task may change.

Private rules in the overlay have the same format, in a source of their
own ([Sources and references](rule-sets.md#sources-and-references)).

## Choosing rules

The general rules are a default set, not obligations on every jurisdiction
(`division-of-instructions.rules`): one jurisdiction may need a rule that another has
good reason to drop (a repo that stays private on a self-hosted server may
keep credentials in it on purpose). Open, for the person to decide. Recommended:

- A jurisdiction uses every general rule by default. Its overlay leaves a rule
  out only by naming it, with a one-line reason, in `overlay.toml`
  (`exclude = [3]`). The build manifest lists
  what was left out.
- A repo inside a jurisdiction departs from a rule only by a relaxation
  declaration (below), where the constitution permits it. It does not leave
  rules out of the compiled file, because one compiled file serves all of a
  jurisdiction's repos.
- To change a rule rather than drop it, a rule of the overlay overrides it
  ([Overrides](rule-sources.md#overrides)). In a public repository the
  private layer only adds rules and overrides none; there the change is a
  public local rule, or a proposal to the rule's source.
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
  target: rule:42/wait_hours=4
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
