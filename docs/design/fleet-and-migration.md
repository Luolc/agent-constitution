# Fleet and migration

## Relation to fleet

The toolchain is plain commands over files: it reads sources and an overlay,
writes an output and a manifest, and exits with a status. It does not call
fleet, read fleet's state or depend on how fleet starts agents. Fleet (and
any other orchestrator) consumes the output the way any harness does. The
only fleet-specific file is `.fleet/config.json`, which tells fleet where
this repo's issues live.

## Migrating from the current hand-written guide

Method and order only; the per-rule mapping is its own piece of work.

1. The hand-written guide stays in force until cutover. Nothing is deleted
   early.
2. The constitution is approved first, because every rule needs an
   `authority` to point at.
3. Classify each current rule into one destination: constitution, general
   rule, private rule, parameter (the value moves to `params.toml`, the rule
   keeps a placeholder), procedure (moves into a skill), provenance (moves to
   the private casebook), or removal.
4. Build the compiler and the schema and reference checkers, then port rules
   in batches, compiling after each batch.
5. Compare the compiled output with the hand-written guide, section by
   section, and record every intended difference.
6. Each existing repo-level override either becomes a `relaxes:`
   declaration or is removed; a person decides which.
7. Cutover: the overlay's apply step installs the compiled file in place of
   the hand-written one; load verification passes; the hand-written source
   is retired.
