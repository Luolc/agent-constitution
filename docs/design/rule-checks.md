# Rule checks and review

Part of the [Rule system](rule-system.md), whose status and terms
apply here.

## Checks

Most rules cannot be decided by a script: a rule a script could decide is
usually a CI check or a lint already. So a rule is kept mainly through
review, and a check serves the review: it settles the few rules a script
can, and hands the reviewer the rest.

There is no language for writing checks, declarative or otherwise. A small
set of built-in check primitives ("this file exists", "this key in this
TOML file has this value") would keep needing one more case, "or", and
"unless", until it became a language of its own that nobody reads well. A
check is code, of one of two kinds, and there is no third.

**Our rules: Rust.** The checks of the rules in this repo are in the
command's crate
([The command and the first version](statute-command.md#the-command-and-the-first-version)),
with one module per rule, named for the rule's ID (`r42.rs` for
rule 42), the way ruff and clippy implement each lint as a function. The
rule's text and metadata stay in `rules/42.toml`, so the person writing a
rule and the one writing its check each change their own file; the
module's test cases sit next to the check, in the crate. Common
operations, such as reading a key from a TOML file, are ordinary helper
functions in the same crate; an operation becomes a helper on its third
repetition, not before (`simplicity`). The crate builds into one binary, so a
downstream user installs one binary and has every check, with no scripts
to deploy. The crate is published to crates.io with its own version, and
publishing it is a release; the rules are released with tags of the
repository ([Releases and versions](amendments-and-releases.md#releases-and-versions)).

A check that needs the state of GitHub, of a pull request or of a CI
run, reads it through the `gh` on the machine, run in the repository,
where `gh api` finds the repository from its GitHub remote. statute never
reads, stores or prints a token: the credential stays in `gh`'s own
configuration. No `gh` on `PATH`, no login, a repository that is not on
GitHub, a call that fails and one that has not answered within 60 s
(stopped like a command past its time, below) are inconclusive, like any
missing tool. What such a check looks at comes from the repository
itself: the pull requests are those GitHub associates with `HEAD` (the
one that merged it into the default branch, or else the open and merged
ones that hold it), and a release is a tag on `HEAD`. A commit not
pushed yet has no pull request, and a `HEAD` without a tag no release,
and both are inconclusive.

When a rule is superseded, its successor gets a new module, copied from
the old one and changed as needed. The old module stays in the code,
marked superseded, and does not run unless asked to.

**Rules written downstream: external commands.** A rule kept in another
source declares its check as a command (`check = "command"`, with `run`
and `tools`), in any language. The script lives in that source's own
repository and runs with its own toolchain; the rule names the tools it
needs (for example, uv or Go). The contract:

- exit code 0: the rule is met; 1: it is violated; anything else: the
  check could not decide;
- standard output: JSON giving, for each finding, the location and the
  reason, as a list of objects `{"location": ..., "reason": ...}`.

The command runs with `sh -c` in the directory of the rule set; when a
change is checked, `STATUTE_BASE` holds the git ref it starts from. A
tool of `tools` that is not on `PATH`, a command that after 10 minutes
has not exited and closed its output (what it started is stopped with
it), an output longer than 1 MiB, and an exit status of 1 whose output
is not that list are inconclusive.

A rule without an automatic check (`check = "none"`) is allowed; it
goes to the reviewer when its `uses` has `review`
([Where a rule is used](rule-config.md#where-a-rule-is-used)).

## Outcomes and the review loop

A check has three outcomes: pass, violation, and inconclusive. A missing
tool, a timeout and a script that crashes are inconclusive, never a
violation, so a missing toolchain does not turn the checks red.

The review starts from `statute check`: the reviewer runs it and gets a
list of the enabled rules, the few a script settled with their outcome
and the rest for review, each with its reason and verification, which
the compiled `AGENTS.md` leaves out by default
([Compilation and the site](statute-command.md#compilation-and-the-site)). The reviewer
goes through the list domain by domain and rule by rule: the rules for
review, the inconclusive results and the violations. Whether
one reviewer takes all of them or several take a group each, and how the
groups are formed, is set in the rule set. `uses` narrows the rules
for review to those marked `review`
([Where a rule is used](rule-config.md#where-a-rule-is-used)).

In the first version, `statute check` lists the enabled rules of the
repository whose `uses` has `review` for review and runs the checks of those that have one; with
`--base <ref>` it checks the change `git diff <ref>...HEAD`. It groups
the rules by their top domain, a subdomain's rules under its domain,
the domains and the rules in each in order of ID,
under a heading with the domain's reference and title (`## R13
(Security)`). Each rule has a line with its outcome (`pass`,
`violation`, `inconclusive`, or `review` for a rule without an
automatic check whose `uses` has `review`), its reference and title, and a note, with the findings
below; a rule that is inconclusive or for review is followed by its
reason and verification, with their citations rendered as in the
compiled `AGENTS.md`, except that a cited rule is its title and
reference (`Title (rule R42)`), since the reviewer reads it apart from
that file. Before
the domains, a section `## statute.toml` compares the rule set's
`public` with the repository's visibility on GitHub
([Public or not](rule-sources.md#public-or-not)), through `gh repo view
--json visibility`: `public = false` in a repository GitHub shows as
`PUBLIC` is a violation, an answer `gh` cannot give is inconclusive,
and anything else passes. It exits with status 1 when a rule or the
rule set is violated, and 0 otherwise. A
built-in check that covers only part of its rule, finding nothing, is
inconclusive, not a pass, since the rest of the rule is still for the
reviewer. A built-in check belongs to a rule of the first-party source;
a rule of another source with `check = "builtin"` is inconclusive, with
a note that statute has no built-in check for it, so a local `L50` does
not run rule 50's check. The built-in checks:

- rule 46 (instruction-files-apart) tells instruction files, generated
  files and other files apart with the path tables of
  `check-instruction-changes`, and is violated by a change that mixes
  instruction files with other files. statute's files are taken beside
  the rule set, wherever it is in the repository: `statute.toml`,
  `statute.lock` and everything under `.statute/` are instruction files.
  The lock file is one because it pins each git source's commit, and
  `statute upgrade` moves a source to newer rules by rewriting the lock
  file alone, which `statute compile --check` accepts. The `AGENTS.md`
  that statute compiles beside the rule set is a generated file only
  while it equals what statute compiles from the public layer now, which
  is what rule 47 asks of a generated file; edited by hand, not
  recompiled, or when statute cannot compile, it is an instruction file,
  so a hand edit mixed with other files is a violation whether or not the
  repository's CI runs `statute compile --check`. The
  `statute-checks.toml` that statute before 0.1.18 wrote beside the rule
  set, and now deletes, is an instruction file too. A change of generated
  files alone, such as a recompile, mixes nothing. The note names the
  generated files the change touches, for the reviewer of rule 47. Here
  the script differs: in this repository it counts the top `AGENTS.md`
  as generated always, the top `statute.lock` when its TOML differs from
  the base's (at the merge base) only in `statute` and `commands`, which
  statute writes from its own version and the rules' checks, and
  `statute-checks.toml` always, by its rules L31 and L27, since its CI
  runs `statute compile --check`, and it knows statute's files only at the
  top of the repository. Files of agent tools the
  tables do not name, sources of instructions outside them, and paths a
  repository declares in its `AGENTS.md` are left to the reviewer, and
  without `--base` there is no change to check;
- rule 280 (instruction-files-apart-but-migration), which supersedes
  rule 46, runs the same check with its one exception: a change that
  adds `statute.toml` beside the rule set, to a base that has none in
  any directory, is the migration change, which may mix instruction files with other
  files. It is inconclusive, with each other file listed for the
  reviewer, who checks that they only add `statute compile --check` to
  the commit hooks and the CI the repository has at its base. Every
  other change is judged as under rule 46;
- rule 43 (credential-designated-place) runs the gitleaks on the
  machine, with `--redact`, on the commits of the change or on the whole
  history, and is violated by a hit, shown by its place and gitleaks rule
  only; credentials gitleaks does not recognize, and places outside the
  repository, are left to the reviewer, and without gitleaks it is
  inconclusive;
- rule 50 (commit-hook-credential-scan) reads `.pre-commit-config.yaml`
  at the top of the repository (outside git, in the rule set's directory),
  and passes when a hook runs gitleaks with `--staged` and `--redact`,
  at the commit stage and for every commit, with no file filter; the
  hooks gitleaks publishes, under its present or its former repository
  name, count by their own entry when their `rev` is v8.1.1 or later.
  A hook whose command statute cannot see (gitleaks' own at an older or
  unreadable `rev`, a hook of another repository, a local hook that runs
  a script), a gitleaks hook without `--redact` or with a file filter,
  another scanner, another hook framework's configuration and YAML
  statute does not read are inconclusive. No hook configuration at all,
  or one whose hooks at the commit stage all show their command and none
  scans the staged changes, is a violation;
- rule 58 (release-first-green-commit) takes a tag on `HEAD` as the
  release, the default branch as the main branch, and as CI the workflow
  holding a check the default branch requires, by branch protection or a
  ruleset; it passes when that workflow's run for the push of `HEAD` to
  the default branch concluded in success on attempt 1, and is violated
  when it did not. Without a tag on `HEAD`, without a required check,
  without such a run (no CI on pushes, or a commit not on the branch) and
  while the run has not finished, it is inconclusive;
- rule 61 (verdict-names-version) reads the verdicts on the pull
  requests of `HEAD`: each comment or review whose text opens with
  `Verdict:`, and each review that approves or asks for changes. A verdict
  without the full SHA of a commit of its pull request is a violation; a
  review also names the commit GitHub records it on;
  verdicts given elsewhere, and whether the SHA named is the version
  reviewed, are left to the reviewer;
- rule 71 (no-automatic-merge) reads the timeline of the pull requests of
  `HEAD`, and is violated when auto-merge was ever enabled on one, even if
  it was disabled again or has merged it; other ways of merging
  automatically, and who enabled it, are left to the reviewer.

The rules 43, 46 and 280 supersede, 3, 7 and 46, keep their checks,
which run only for a rule set that still enables them.

The reviewer runs `statute check`, and CI may run it too. It is not a
commit hook: some checks scan the whole history or ask the forge over
the network and can be slow, and a commit hook holds only checks that
take seconds.

For each rule it is handed, the reviewer decides whether the rule is met
or violated. A violation is fixed. Where the reviewer's decision differs
from the check's, the check was wrong:

- a false positive: the check flagged something that meets the rule;
- a false negative: the check passed something that violates the rule,
  found while reviewing another result or in later work.

Either leads to a proposed fix to the rule's check. Fixing a check does
not change what counts as compliant, so the rule keeps its ID and its
revision goes up by one (`wrong-checkers.fixing`). People and agents alike
may propose changes to a rule, under `rule-evolution`. This is how the
rules improve themselves: the few checks a script can run settle what
they can, agents review the rest rule by rule, and what the agents
conclude goes back into the checks.

**Who decides.** Compiling, upgrading, finding conflicts and judging the
compatible range are done by deterministic scripts wherever a script can
decide. What a script cannot decide goes to an agent, through the three
outcomes and the review loop above. People take no part in these
judgments; they appear only where the constitution itself asks for a
person's approval. This follows `automation-first.decide-and-do`.

## Trial rules

When agents doing a piece of work find that no rule, skill or convention
covers it, they start writing the convention down. The first version is a
trial rule (`status = "trial"`): it is used on real work, changed freely
while it is tried, and moved to `active` once it is stable. The rules,
skills and conventions of this repo itself are meant to come about the
same way.

**Starting.** While the constitution is a draft, every rule is `draft`.
Once it is active, a new rule starts as `trial`, and the rules written
while it was a draft move from `draft` to `trial` in place
([IDs and revisions](rule-format.md#ids-and-revisions)).

**What the statuses mean.** `trial` and `active` differ as the
experimental and the stable checks of a linter do: an `active` rule is
stable and can be used as it is, a `trial` rule is still experimental and
may change. `active` says that a rule is stable, not that a person
reviewed it. The status of a domain or a one-of is about that node alone,
its title, its applicability and which rules it holds; a domain can be
`active` while some of its rules are still `trial`.

**Becoming active.** A principal decides when a trial rule is stable
enough to move to `active`, on the conditions of R289
(`rules/289.toml`). Most rules cannot be settled by a deterministic check,
so the main use of a rule is in review: the reviewer runs the checks there
are, reads the rule's verification, and judges the change by the rules of
its domain. How a trial rule fares there, how often it was used and
whether it was misapplied, is what the principal has to go on; no count of
uses moves a rule by itself. The move changes only the status, not what
counts as compliant, so the rule is edited in place and its revision goes
up by one ([IDs and revisions](rule-format.md#ids-and-revisions)).
