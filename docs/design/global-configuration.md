# Global configuration and git sources

Status: decided; not built yet. What is built is under
[The command and the first version](statute-command.md#the-command-and-the-first-version).
This part of the design says where statute keeps its own files in the
user's home directory, how a global configuration names sources for
every repository, how a git source is cloned, pinned and read, and how
a source is added with one command. It builds on
[Rule sets and the lock file](rule-sets.md#rule-sets-and-the-lock-file)
and [Rule sources](rule-sources.md).

## Contents

- [The statute directory](#the-statute-directory)
- [The global configuration](#the-global-configuration)
- [Which source a name means](#which-source-a-name-means)
- [Git sources](#git-sources)
- [Adding a source](#adding-a-source)

## The statute directory

statute keeps its own files in `~/.statute/`, as Claude Code keeps
its own in `~/.claude/`:

```text
~/.statute/
  config.toml                       the global configuration
  repos/<name>-<hash>/              a bare clone of each git source
  snapshots/<commit>/               the files of one commit, written once
```

- `repos/` holds one bare clone per address: `<name>` is the last part
  of the address without `.git` (`agent-constitution`), and `<hash>` the
  first 12 hex digits of the SHA-256 of the address as written, so two
  addresses never share a clone. A bare clone has no work tree, so
  nothing statute does there changes files anyone works in.
- `snapshots/<commit>/` holds the files of one commit, which statute
  reads as it reads a source on disk. It is written into a temporary
  directory next to it and renamed into place, and never changed
  after; a commit names its content, so the same snapshot serves every
  clone that has the commit.

Everything under `repos/` and `snapshots/` can be deleted at any time:
statute clones and writes it again when it needs it. The home
directory is the one the system reports (`HOME` on Unix).

`statute init --global` writes `config.toml` with the built-in default
written out (below) and a commented example of a private source, then
clones or fetches each git source it names, so that a missing network
or account shows at once. An existing `config.toml` is left as it is,
and the command exits with status 1.

## The global configuration

`config.toml` names sources that every rule set of the user may use,
in the shape of a rule set's `[sources]`:

```toml
[sources]
R = { git = "https://github.com/Luolc/agent-constitution", branch = "main" }
me = { git = "git@example.test:me/rules.git", branch = "main", public = false }
```

- Each source is a git source: `git` with `branch` or `tag`, as in a
  rule set ([Git sources](#git-sources)). A committed lock file records
  a git source by its address and commit, which every machine can read
  again; a directory on one machine is read as it is, and stays in the
  rule set that names it with `path`.
- `clone = "<dir>"` beside `git` and `branch` or `tag` uses a clone
  the user keeps, in place of one under `repos/`. statute clones nothing
  then and fetches nothing into it: it reads the commit of
  `refs/remotes/origin/<branch>`, or of the tag, as the user's last
  fetch left it, and fails when that clone's `origin` is not the address
  `git` names. `~` at the start of the directory is the home directory.
- `public` is required on every source but `R`, as in a rule set; there
  is no `scope`, since a source of the global configuration is global
  by definition ([Four kinds of source](rule-sources.md#four-kinds-of-source)).
- The configuration names sources only. It enables no rule: each rule
  set lists the rules and packs it uses, as before, so that the
  committed rule set and lock file alone say what a repository's
  compiled output holds. A source of the global configuration is
  available to every rule set by its name, as if the rule set named it.

**The built-in default** is the one source statute knows by itself:
`R = { git = "https://github.com/Luolc/agent-constitution", branch = "main" }`.
The first-party source publishes no tagged releases of its rules yet;
once it does, the default follows the newest tag.

**Which layer a global source joins.** A public source of the global
configuration joins the rule set of `statute.toml`. A private one joins
it in a private repository (`public = false`), and in a public one only
the private layer of `statute.private.toml`
([Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)):
the committed files of a public repository never name it. Enabling a rule
of a private source in a public `statute.toml` fails, naming two ways:
enable it in the private layer, `statute add --private me:4`, or leave
it out.

## Which source a name means

For each name in `statute.toml`, the first of these that names it gives
the source:

1. `statute.toml`;
2. the global configuration;
3. the lock file, for `compile`, `check`, `explain` and `site` (below);
4. the built-in default, for `R`.

Resolving `statute.toml`, its lock file and its `AGENTS.md` never reads
`statute.private.toml`. The private layer only adds names, as
[Private rules in a public repository](rule-sources.md#private-rules-in-a-public-repository)
says: `statute.private.toml` may not name a source the public layer
has, whichever of the four gave it, and a name it gives is otherwise
looked up in it first, then in the global configuration's private
sources the layer holds, then in `statute.private.lock`.

A rule set that names a source replaces the global configuration's
entry whole: fields are not merged across layers. `name = false` in a
rule set's `[sources]` takes a source of the global configuration out
of that rule set, as if it were not there; the built-in `R` is taken out
only by naming another `R`. Once this is built, `[sources]` is optional
in `statute.toml`: a rule set that uses only the first-party source at
its default names none.

The lock file comes third so that a machine without the user's global
configuration, CI or another person's, still builds what the lock file
pins: it records each git source's address, the branch or tag it
follows, the commit and, for every source but `R`, `public`
([Git sources](#git-sources)), so that the checks of which rules may
cite which run there as well. `lock` and
`upgrade` do not read it as a source: they resolve the rule set from
the configuration as it is now, so a source taken out of the global
configuration drops out at the next `lock`.

## Git sources

A source in a rule set or in the global configuration may be a git
repository:

```toml
[sources]
team = { git = "https://example.test/our-team/rules.git", branch = "main", public = true }
```

- `git` is an address that `git clone` takes, as written:
  `https://...`, `ssh://...`, `git@host:path`, or `file://...` and an
  absolute path for a repository on the machine, which is what the
  tests use, with no network.
- One of `branch` and `tag` says what to follow. `path` and `git` in one
  entry fail, as before.
- Its rules are in `rules/` at the top of the repository, its
  constitution in `constitution/<edition>/`, and its skills in
  `skills/<name>/`, as in a source on disk.

**Pinning.** The lock file records a git source's address, its branch
or tag, and the commit:

```toml
[constitution]
git = "https://github.com/Luolc/agent-constitution"
commit = "<commit>"
edition = "2026-10"
revision = "<revision>"

[[sources]]
name = "R"
git = "https://github.com/Luolc/agent-constitution"
branch = "main"
commit = "<commit>"

[[sources]]
name = "team"
git = "https://example.test/our-team/rules.git"
branch = "main"
commit = "<commit>"
public = true
```

Every source but `R` has its `public` there, a source on disk as well; the lock file statute writes today records a source on disk by its `path` alone.

- `lock` keeps the commit the lock file records when the source's
  address and branch or tag are those recorded, so that nothing changes
  under a rule set until it upgrades. For a new source, or one whose
  address or branch changed, it fetches and takes the head of the branch,
  or the tag's commit.
- `upgrade` fetches every git source and takes the newest commit of
  each, then shows what changed, as it does today. The text of a
  changed rule can now be shown, since the old commit can be read; that
  is not built yet.
- `compile`, `check`, `explain` and `site` read each git source at the
  commit the lock file records; without a lock file or a line for the
  source there, they take the head as `lock` would. A source whose
  address or branch or tag is not the one the lock file records makes
  the lock file stale: they fail, with a note to run `statute lock`, and
  read no commit. They fetch only when the commit is missing from the
  clone, and fail when it is missing after the fetch, naming the
  source, the commit and the address; with `clone`, they fetch nothing
  and fail at once, asking the user to fetch in that clone.

**Reading.** statute clones a source once, `git clone --bare`, into
`repos/` ([The statute directory](#the-statute-directory)), and fetches
into that clone, `git fetch` with the branches and tags of the remote,
whenever a command above needs a commit the clone lacks or the newest
one. It reads a commit through its snapshot, writing the commit's files
into `snapshots/<commit>/` with git's own commands against a temporary
index, so that no clone's work tree, `HEAD` or index ever changes. The
same commit therefore always gives the same files, and what is compiled
changes only when the lock file does.

A file of a git source is named in messages by the source and the
commit, `R@1a2b3c4d:rules/7.toml`.

## Adding a source

`statute source add <name> <address>` names a source:

- `<address>` is a git address, or a directory on disk. In a rule set a
  directory becomes `path`, relative to the directory of `statute.toml`;
  with `--global`, the directory must be a clone, which becomes
  `clone`, its `origin` the `git`.
- `--branch` or `--tag` says what to follow; without either, the branch
  the remote's `HEAD` names, written out in the entry.
- The source is cloned, or the directory read, and the rule set
  resolved with it before anything is written; nothing is written when
  that fails.

**Public or not.** `--public` or `--private` decides. Without either,
statute asks `gh repo view --json visibility` about the address, or,
for a directory, about the repository that holds it:

| GitHub shows | `public` written |
| --- | --- |
| `PUBLIC` | `true` |
| `PRIVATE` | `false` |
| `INTERNAL`, or no answer (no `gh`, no login, an address GitHub does not hold, a directory in no repository) | `false`, with a note that says why and that `--public` makes it public |

The side that cannot leak is the reverse of `init`'s: there a wrong
`public = true` only makes a rule set stricter, while here a wrong
`public = true` lets a public rule cite the source's rules and put
their text into public files
([Which rules may cite which](rule-sources.md#which-rules-may-cite-which)).

**Where it is written.**

| | Written to |
| --- | --- |
| `--global` | `~/.statute/config.toml` |
| a public source, or any source of a private repository | `statute.toml` |
| a private source of a public repository | `statute.private.toml`, which is created with `base = "statute.toml"` when it is not there, after the check that git ignores it |

A private source in a public repository goes to the private layer
rather than stopping the command: nothing in that layer reaches a
committed file, and a private overlay is the usual reason to add a
private source. The note says where it went and that `--public` writes
it to `statute.toml`. A name the file being written already holds fails
the command, and so does a name the public layer holds when the source
goes to `statute.private.toml`, since the private layer only adds. A
name that only the global configuration holds is replaced in
`statute.toml` or `statute.private.toml`, as
[Which source a name means](#which-source-a-name-means) says, and the
note says so. When a lock file is there, it is written again, as `add`
writes it.
