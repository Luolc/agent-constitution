# agent-constitution

A constitution for coding agents, a general rule library and skills, and a
planned toolchain that compiles them, together with a private overlay that
holds one user's local values and rules, into the single instruction file
that Claude Code, Codex and Grok read. The repo is at the draft stage: it
holds the constitution draft, the [design](docs/design/README.md) and the script
that builds the constitution; the rule compiler and checkers are designed
but not yet written.

The constitution, the general rules and the general skills are the three
kinds of content the repo provides for installing; a user picks which of
them to install through configuration. How they are installed is not
decided yet.

The constitution is kept in English and Chinese, both built from the same
sources: [English](constitution/constitution.en.md),
[中文](constitution/constitution.zh.md). The sources are TOML files under
`constitution/articles/`, one directory per article and one file per
section, each holding both languages; edit them and rebuild, never the two
generated files. The general rules under `rules/` are TOML files of the
same kind, one per rule. The format, the IDs and the numbering are in the
design. The general skills are under `skills/`, one directory per skill
with its `SKILL.md`, in English.

The tools under `tools/` are a [uv](https://docs.astral.sh/uv/)
workspace of three Python projects: `tools/compiler` (the constitution
build, `uv run constitution-build`), `tools/site` (the token count and
the link check of the static site) and `tools/checks` (the checks this
repo runs on its own changes). The root `pyproject.toml` sets the Python
version and pins limae, which they call, pre-commit and the Python
checkers (ruff, pydoclint, basedpyright), which the commit hooks run;
`uv sync` installs all of it into `.venv` from `uv.lock`. Install the
commit hooks once with `uv run pre-commit install`.

`hugo-0.167.0 -s web` builds the constitution into two static web pages,
English and Chinese, under `site/` (not committed), the tokens not
counted; `uv run check-site-anchors site` checks their links. It
needs Hugo at that version and the theme submodule (`git submodule update
--init`); the setup, and the token count, are in the design (Static
site).
Look at the pages served, for example with `uv run python -m http.server -d
site`: opened as files, the language switch leads to a directory listing.
