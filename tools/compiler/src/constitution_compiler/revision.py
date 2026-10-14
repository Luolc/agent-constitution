"""Check 7, revision: the revision rises exactly when the text changes."""

from pathlib import Path
import re
import tomllib

from constitution_compiler.base import base_layout, base_paths, git, GitError
from constitution_compiler.model import Errors, Layout, Sources


def revision_key(revision: str) -> tuple[int, int] | None:
  """Return the sort key of a revision.

  A draft `0.<n>` sorts by its number, before every approved integer
  revision.

  Args:
    revision: the revision as written.

  Returns:
    The key, or None for any other string.
  """
  if re.fullmatch(r"0\.[0-9]+", revision):
    return 0, int(revision[2:])
  if re.fullmatch(r"[1-9][0-9]*", revision):
    return int(revision), 0
  return None


def source_text_changed(
  root: Path, ref: str, layout: Layout, base: Layout
) -> bool:
  """Return whether the constitution text differs from the base's.

  The text is the article sources and `index.toml` other than its
  `revision` line. The files git tracks are compared as they are in the
  working tree, so the result does not depend on what is committed. Each
  side's files are found by its own layout and compared by their paths
  within it, so moving the sources to another directory alone changes
  nothing.

  Args:
    root: the checkout.
    ref: the base commit.
    layout: the tree's layout.
    base: the base's layout.

  Returns:
    True when the text changed.
  """
  drop = re.compile(r'^revision = ".*"\n', re.MULTILINE)
  tracked = git(root, "ls-files", "--", str(layout.articles))
  paths = {
    Path(p).relative_to(layout.articles)
    for p in tracked.split("\n")
    if p and (root / p).is_file()
  }
  base_files = {
    p.relative_to(base.articles) for p in base_paths(root, ref, base.articles)
  }
  if paths != base_files:
    return True
  if any(
    (root / layout.articles / p).read_text(encoding="utf-8")
    != git(root, "show", f"{ref}:{base.articles / p}")
    for p in paths
  ):
    return True
  base_index = git(root, "show", f"{ref}:{base.index}")
  text = (root / layout.index).read_text(encoding="utf-8")
  return drop.sub("", text) != drop.sub("", base_index)


def check_revision(root: Path, ref: str, src: Sources, errors: Errors) -> None:
  """Check 7: compare the revision with the base's.

  Within one edition the revision is greater than the base's when the
  constitution text changed since it, and equal to it when the text did
  not change. A draft base counts as the same edition whatever edition
  each names, so renaming the edition a draft prepares does not restart
  its revisions, and the approved text's revision is greater than the
  draft's. Not compared: an active base of another edition.

  Args:
    root: the checkout.
    ref: the base commit.
    src: the sources.
    errors: where errors go.
  """
  try:
    git(root, "rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}")
  except GitError:
    return  # check_base reports it
  layout = base_layout(root, ref)
  if layout is None:
    return
  try:
    base = tomllib.loads(git(root, "show", f"{ref}:{layout.index}"))
  except tomllib.TOMLDecodeError:
    return  # check_base reports it
  revision, base_revision = (
    src.index.data["revision"],
    str(base.get("revision")),
  )
  if base.get("status") == "active" and src.index.data.get(
    "edition"
  ) != base.get("edition"):
    return
  line = src.index.line("revision =")
  key, base_key = revision_key(revision), revision_key(base_revision)
  if key is None:
    errors.add(
      src.layout.index,
      line,
      7,
      f"revision {revision} is neither `0.<n>` nor a whole number from 1",
    )
  elif base_key is None:
    return
  elif source_text_changed(root, ref, src.layout, layout):
    if key <= base_key:
      errors.add(
        src.layout.index,
        line,
        7,
        f"the constitution text changed since base {ref}"
        f" but revision {revision} is not greater"
        f" than the base's {base_revision}",
      )
  elif revision != base_revision:
    errors.add(
      src.layout.index,
      line,
      7,
      f"revision {revision} differs from revision {base_revision}"
      f" at base {ref} but the constitution text did not change",
    )
