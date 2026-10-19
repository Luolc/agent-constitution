# Agent Fleet and migration

## Relation to Agent Fleet

The toolchain is plain commands over files: it reads sources and an overlay,
writes an output and a manifest, and exits with a status. It does not call
Agent Fleet, read Agent Fleet's state or depend on how Agent Fleet starts
agents. Agent Fleet (and any other orchestrator) consumes the output the way
any harness does. The only file specific to Agent Fleet is
`.fleet/config.json`, which tells Agent Fleet where this repo's issues live.

## Migrating from the current hand-written guide

Method and order only; the per-rule mapping is its own piece of work.

1. The hand-written guide stays in force until cutover, except in a
   repository that has already switched to its own rule set. Nothing is
   deleted early.
2. The constitution is approved first, because every rule needs an
   `authority` to point at.
3. Classify each current rule into one destination: constitution, general
   rule, private rule, parameter (the value moves to `params.toml`, the rule
   keeps a placeholder), procedure (moves into a skill), provenance (moves to
   the private casebook), or removal.
4. Build `statute` and its checkers
   ([The command and the first version](statute-command.md#the-command-and-the-first-version)),
   then port rules in batches, compiling after each batch.
5. Each repository, in the change that adds its rule set, compares its
   compiled output with the instructions it replaces and records every
   intended difference
   ([Rule sources](rule-sources.md#cutover-from-the-hand-written-guide),
   condition 3).
6. Each existing repo-level override either becomes a `relaxes:`
   declaration or is removed; a person decides which.
7. Cutover: repositories switch one at a time, and once the last one in
   use has switched, the hand-written guide is retired, with no compiled
   file installed in its place.

[Rule sources](rule-sources.md#cutover-from-the-hand-written-guide)
splits the destination "private rule" of step 3 by scope and
visibility, turns step 6 into overrides and one-ofs, and says how a repository
switches in step 7, when the guide may be retired and how the
retirement is checked.
