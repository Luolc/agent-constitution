# AGENTS.md

Repo-specific notes for coding agents working on agent-constitution.

This repo holds the constitution for coding agents, the general rule
library and skills, and (later) the toolchain that compiles them, together
with a private overlay, into the one instruction file the agents read. It is
in draft: today it has the constitution draft and the design, no tools yet.

## Language

Everything in this repo is English: code, comments, docs, commit messages,
PR titles and bodies.

## Layout and docs

- `constitution/constitution.md`: the constitution. Changes to it follow
  its own amendment procedure. Several other paths are protected
  foundations under C8 of the constitution; the list is in `docs/design.md`
  (File structure).
- `docs/design.md`: the one design doc. A PR that changes how the repo
  works (layout, source formats, compilation, checkers, releases) updates
  the section it touches in the same PR. There are no other design or
  decision files.
- The full directory plan, with what belongs to which stage, is in
  `docs/design.md`. Do not create a reserved directory before its stage.
- Never commit a compiled instruction file for a real user. The only
  compiled output here is the golden output of the fictional example
  overlay.

## Content rules

This repo will be made public. Write every file, commit message and PR text
as if it already were:

- No private information: no machine or host names, ports, secret-store
  paths or account names, issue-tracker numbers, chat channels or user IDs,
  people's names or email addresses, names of private repos (call the
  private side "the private overlay"), names of agent instances or
  sessions, and no "who said what, when, relayed by whom". Provenance stays
  on the private side.
- Self-contained: no pointer to rules, files or documents outside this repo
  that a public reader cannot see. A rule that is needed is written here; one
  that is not, is left out.
- One fact in one place; elsewhere, link to it. Do not write values that go
  stale (versions, model names, external tool behavior) unless needed, and
  then with the date they were read.
- Test data and examples use made-up values only: `example.test` domains,
  documentation address ranges (192.0.2.0/24, 198.51.100.0/24,
  203.0.113.0/24, 2001:db8::/32), fictional names.
- Public projects may be named, for example fleet and herdr.

Before a push, search the diff for each class above and check that the
search reads content: run the same search on a file where it must match.

## Checks

- `uvx pre-commit run --all-files`: gitleaks on the staged diff. Install
  the hook once after cloning with `uvx pre-commit install`.
- `gitleaks git --redact --no-banner --verbose .`: the full-history scan
  that CI runs. Keep `--redact`; without it a hit prints the secret.

## CI

`.github/workflows/ci.yml` runs on pull requests and on pushes to `main`. Its
one job, `check`, scans the full history with gitleaks (pinned by version and
SHA-256) and then runs `pre-commit run --all-files` with the gitleaks hook
skipped. `check` is the required status check on `main`: a merge waits for
it to pass. Runners are pinned to `ubuntu-24.04`, and actions by commit SHA.

## Fleet

For leads and workers that `fleet` starts in this repo.

- Open your own worktree with `fleet worktree agent-constitution --branch
  <type>/<short-desc>`, or work in the one your lead names. A reviewer's
  checkout of a PR head is `fleet worktree agent-constitution --name <name>
  --detach <head-sha>`, never a bare `git worktree add`: fleet removes what
  it recorded at `fleet job end`.
- `constitution/` changes in a task of its own; do not mix it with other
  files in one PR.
- Worker report (`fleet done --report-file`): what was done, the PR and its
  merge commit, the checks run and CI's `run_attempt`, why done or
  abandoned, and follow-ups left.
- Done means the PR is merged. Abandoned means you stop without a merge and
  say why. The lead reads the reports and decides what comes next.
- This repo has no natural key for a job. Before starting one, look at
  `fleet job list` and the open issues, and ask in the thread when unsure.
