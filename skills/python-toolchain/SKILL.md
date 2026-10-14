---
name: python-toolchain
description: How to set up the check toolchain of a Python repo with uv and pre-commit, so that the commit hooks and CI run the same tool versions, taken from uv.lock. Use when creating a Python repo, or when adding or changing the Python checks in a repo's pre-commit configuration or CI.
---

# Python check toolchain

One way to wire the Python checks of a repo (ruff, pydoclint, basedpyright
and the like) into pre-commit and CI so that each tool's version is
written in one place, the repo's `uv.lock`, and the hooks never change the
environment behind your back.

Scope: the Python checks, and (point 5) how Python scripts are run.
Hooks for other files (secret scanners, Markdown linters, prettier and
other hooks from external repos) stay as they are.

## The setup

1. Each Python check is a local hook that runs the tool from the
   project's environment:

   - `repo: local`, `language: system`;
   - `entry: uv run --frozen --no-sync <tool> ...`.

   The tools are in the project's dev dependency group in
   `pyproject.toml`, so their versions are in `uv.lock` and nowhere else;
   no `rev:` of a tool's own pre-commit repo repeats them. `--frozen`
   keeps uv from touching the lock; `--no-sync` keeps it from installing
   into `.venv`, so a hook whose tool is missing from `.venv` fails
   instead of quietly installing it. That holds only while no other copy
   of the tool is on `PATH`: when the tool is not in `.venv`, `uv run`
   falls back to `PATH`, so another active virtual environment (one that
   direnv activates, say) or a copy installed elsewhere runs instead, and
   the hook may pass. CI syncs first, and a synced `.venv` comes before
   `PATH`. A `.venv` that is out of date is not
   caught: after a change to the dependencies, run `uv sync`, or the
   hooks run the old versions still installed.

2. The `uv-lock` hook from astral-sh/uv-pre-commit keeps `uv.lock`
   consistent with `pyproject.toml`. When they disagree, it rewrites
   `uv.lock` and the hook fails; stage the new lock and commit again.

3. CI installs the environment from the lock, then runs the same hooks:
   `uv sync --locked` (in a uv workspace `uv sync --locked
   --all-packages`) on every run, then `pre-commit run --all-files`.
   `--locked` fails when the lock is stale. Pin the runner to a fixed
   image (not `-latest`) and the actions by commit SHA, as the repo's
   other workflows do.

4. Several Python projects in one repo: make them one uv workspace with
   the dev tools in the workspace root, so the whole repo has one version
   of each tool. A project that really needs other tool versions, and so
   is not a workspace member, is set up as a second project beside the
   first; write down why in the repo's `AGENTS.md`. It gets:

   - its own Python hooks, `entry: uv run --project <dir> --frozen
     --no-sync <tool> ...` with `files: ^<dir>/`. `--project` keeps the
     working directory at the root, so a tool that reads its settings
     from there must be pointed at `<dir>`: `basedpyright -p <dir>` with a
     `[tool.basedpyright]` section in `<dir>/pyproject.toml` (without
     that section it finds no source files and passes), and `pydoclint
     --config <dir>/pyproject.toml` with a `[tool.pydoclint]` section
     there. ruff finds the nearest `pyproject.toml` by itself;
   - its own `uv-lock` hook, `args: [--project, <dir>]` with
     `files: ^<dir>/(pyproject\.toml|uv\.lock)$`;
   - `exclude: ^<dir>/` on the root's hooks, so the two sets of tools do
     not both check its files. A root hook that checks the whole tree
     rather than the file names it is given (basedpyright with
     `pass_filenames: false`) ignores `exclude:`; leave `<dir>` out in
     that tool's own configuration as well, for basedpyright `exclude`
     under `[tool.basedpyright]`;
   - its own `uv sync --locked --project <dir>` in CI.

5. Call scripts with `uv run <path>`. Do not give a Python script a
   shebang such as `#!/usr/bin/env python3` or the executable bit: with
   them `./script.py` runs outside uv's environment. When you move a repo
   to this setup, check for both and remove them (`.py` files with a
   shebang, or mode 100755 in `git ls-files -s`).

The first time in a fresh clone, run `uv sync` (`--all-packages` in a
workspace, and `uv sync --project <dir>` for a separate project) before
the hooks: with `--no-sync` they do not create the environment.

## Templates

`<version>`, `<sha>` and the tool arguments are placeholders: take the
current versions when you copy them, and the arguments from the repo's
own configuration.

`pyproject.toml`, the dev group:

```toml
[dependency-groups]
dev = [
  "basedpyright==<version>",
  "pre-commit==<version>",
  "pydoclint==<version>",
  "ruff==<version>",
]
```

`.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/astral-sh/uv-pre-commit
    rev: <version>
    hooks:
      - id: uv-lock

  - repo: local
    hooks:
      - id: ruff-check
        name: ruff check
        entry: uv run --frozen --no-sync ruff check --force-exclude
        language: system
        types_or: [python, pyi]
      - id: ruff-format
        name: ruff format
        entry: uv run --frozen --no-sync ruff format --check --force-exclude
        language: system
        types_or: [python, pyi]
      - id: pydoclint
        name: pydoclint
        entry: uv run --frozen --no-sync pydoclint --quiet
        language: system
        types: [python]
      - id: basedpyright
        name: basedpyright
        entry: uv run --frozen --no-sync basedpyright
        language: system
        types_or: [python, pyi]
        pass_filenames: false
```

`--force-exclude` keeps ruff's `exclude` settings in force for the file
names that pre-commit passes; without it ruff checks them anyway.

The `uv-lock` hook's default `files:` matches only the root
`pyproject.toml`, `uv.toml` and `uv.lock`. In a workspace, widen it so a
member's `pyproject.toml` triggers it too:

```yaml
      - id: uv-lock
        files: (^|/)(pyproject\.toml|uv\.toml)$|^uv\.lock$
```

CI (GitHub Actions), with pre-commit in the dev group:

```yaml
jobs:
  check:
    runs-on: ubuntu-<version>
    steps:
      - uses: actions/checkout@<sha> # <version>
      - uses: astral-sh/setup-uv@<sha> # <version>
      - run: uv sync --locked # in a uv workspace: --locked --all-packages
      - run: uv run --frozen --no-sync pre-commit run --all-files
```

## Checking that it works

Run these when you set this up or change it, in a clean checkout.

- Happy path: `uv sync` (`--all-packages` in a workspace), then
  `pre-commit run --all-files` passes.
- `--no-sync` takes effect: delete `.venv`, check that `command -v
  <tool>` prints nothing for each tool (otherwise the hook runs that copy,
  see point 1; deactivate the other environment or take its directory
  off `PATH`), and run pre-commit without syncing first. Each Python hook
  fails with `Failed to spawn: <tool>`,
  and afterwards `.venv/bin/<tool>` does not exist. `uv run --no-sync`
  still creates an empty `.venv`, so "`.venv` does not exist" is not the
  test; the tool missing from it is. The other arm: the same command
  with `--no-sync` taken out of the hooks passes, and `.venv/bin/<tool>`
  is there. In a workspace, a bare `uv run` syncs only the root project
  and what it depends on, so a hook that needs the dependency of a member
  the root does not depend on fails without `--no-sync` too; use
  `--all-packages` in place of `--no-sync` for this arm.
- `uv-lock` takes effect: add a dependency to `pyproject.toml` without
  running `uv lock`. The `uv-lock` hook fails with `files were modified
  by this hook` and `uv.lock` is rewritten; stage it and the next run
  passes.
- The widened `uv-lock` `files:` takes effect (workspace): with
  `--all-files` the root `pyproject.toml` is always in the file set, so
  that run cannot tell. Add a dependency to a member's `pyproject.toml`,
  change nothing else, and run
  `pre-commit run uv-lock --files <member>/pyproject.toml`: the widened
  hook reports `Failed`, the default one `(no files to check)Skipped`.

A trap in the second check: if pre-commit is started with `uv run
pre-commit ...`, that outer `uv run` syncs `.venv` before any hook runs,
and the check passes whether `--no-sync` works or not. Start pre-commit
in a way that does not sync: `uvx pre-commit ...` or a `pre-commit`
installed on its own (with no `.venv` yet, `uv run --no-sync pre-commit`
finds no pre-commit to run). For the same reason, a repo
whose docs tell people to run `uv run pre-commit ...` gets its
environment synced by that command; one that runs pre-commit any other
way should say to run `uv sync` first.
