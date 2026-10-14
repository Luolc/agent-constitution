# Compilation

Input: one pinned version of this repo (a tag, checked against its commit
SHA) and one overlay directory. The overlay selects the language. The
compiler takes the constitution from the version for agents in that
language, `constitution.<lang>.agent.md` (it does not read the
constitution sources) and each rule's block in that
language. Output: one file, installed by the overlay as
the user-level instruction file that all three harnesses read through
symlinks (a file in the home directory), plus a build
manifest. Nothing else.

Overlay layout:

```text
overlay.toml       core version, language, rules left out, output path,
                   size budget
params.toml        parameter values
rules/<id>.toml    private rules
skills/<name>/     private skills
```

The compiler is a deterministic concatenation, no model involved:

1. Validate every input (schema, references); stop on the first class of
   error, print all errors of that class.
2. Emit a header in plain text, not an HTML comment: generated file, do not
   edit, edit the sources instead; the core version, the constitution
   revision, and the input digest (the SHA-256 of the sorted list of
   per-input hashes).
3. Emit the constitution, then active rules (core and overlay, minus those
   the overlay leaves out) grouped by the article of their first
   `authority` and ordered by ID inside a group.
   Parameters are substituted here.
4. Before each article and rule, emit a source comment holding its ID,
   such as `<!-- constitution:authority -->`, so any paragraph of the output
   can be traced to its file.
5. Measure the result against the budget in `overlay.toml` (the core sets a
   ceiling the overlay cannot raise). Over budget fails the build; nothing is
   silently dropped.
6. Write the output and the manifest: the SHA-256 of each input file (sorted
   by path), core version and commit, compiler version (the core version,
   since the compiler lives here), constitution revision, language, the rules
   left out, input digest, output digest and size. No timestamps; the same
   inputs give byte-identical output.

Roles: there is one output for everyone. The core has no role sections. The
overlay contributes, as a private rule, the one or two sentences that tell
an agent how to tell whether it runs under fleet or under another way of
working, and which skill, if any, that way of working reads.

Wiring on the private side, as currently planned (how the content is
installed is not decided yet): the overlay's apply step fetches the pinned core
version, verifies tag and commit, runs the compiler, runs the checkers that
need private input (the private-term scan), installs the output and the
skills, then runs load verification. The real output is never committed to
this repo.
