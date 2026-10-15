"""Check 7, numbering: what the base numbered still exists, unmoved.

Also the git access the revision part of check 7 shares.
"""

from pathlib import Path
import re
import subprocess
import tomllib

from constitution_compiler import git_env
from constitution_compiler.model import (
  ARTICLE_FILE,
  ARTICLE_NAME,
  CONSTITUTION,
  Doc,
  Errors,
  INDEX_FILE,
  Layout,
  name_number,
  NODE_NAME,
  PARAMS_FILE,
  RULES,
  SECTION_NAME,
  Sources,
)

# Check 7: numbering against a base commit.


GIT_TIMEOUT = 60  # seconds, for one git command


class GitError(Exception):
  """A git command failed or timed out."""


def git(root: Path, *args: str) -> str:
  """Run git in root.

  Args:
    root: the checkout.
    *args: the git arguments.

  Returns:
    Its output.

  Raises:
    GitError: when git exits non-zero or times out, with its stderr.
  """
  command = f"git {' '.join(args)}"
  try:
    result = subprocess.run(
      ["git", "-C", str(root), *args],
      capture_output=True,
      text=True,
      env=git_env.clean(),
      timeout=GIT_TIMEOUT,
    )
  except subprocess.TimeoutExpired as e:
    raise GitError(f"`{command}` timed out") from e
  if result.returncode != 0:
    raise GitError(f"`{command}` failed: {result.stderr.strip()}")
  return result.stdout


def base_paths(root: Path, ref: str, *paths: Path) -> set[Path]:
  """Return the files under the given paths in the base commit.

  Args:
    root: the checkout.
    ref: the base commit.
    *paths: the files and directories to list.

  Returns:
    The files, relative to the checkout.
  """
  listing = git(
    root, "ls-tree", "-r", "--name-only", ref, "--", *map(str, paths)
  )
  return {Path(p) for p in listing.split("\n") if p}


def base_layout(root: Path, ref: str) -> Layout | None:
  """Return the layout of the base commit.

  Args:
    root: the checkout.
    ref: the base commit.

  Returns:
    The layout, or None when the base has no edition directory.
  """
  return Layout.find(base_paths(root, ref, CONSTITUTION))


def check_base(root: Path, ref: str, src: Sources, errors: Errors) -> str:
  """Check 7: compare the numbering with a base commit.

  Once the base is active: the edition is the base's or a later one.
  While it is the base's: every article,
  section and item of the base still exists, by ID, with the same number,
  a section in the same article; what was repealed is still repealed;
  every node of the rule tree of the base still exists, a rule of the
  older format as the node whose slug is its ID, and what was repealed or
  superseded still is.

  Args:
    root: the checkout.
    ref: the base commit.
    src: the sources.
    errors: where errors go.

  Returns:
    A note when the step is skipped, else "".
  """
  commit = f"{ref}^{{commit}}"
  try:
    git(root, "rev-parse", "--verify", "--quiet", commit)
  except GitError:
    errors.add(src.layout.index, 1, 7, f"base `{ref}` is not a commit")
    return ""
  layout = base_layout(root, ref)
  if layout is None:
    return (
      f"base {ref} has no {CONSTITUTION}/<YYYY-MM>/{INDEX_FILE};"
      " numbering not compared"
    )
  base, listing, note = open_base(root, ref, layout, errors)
  if base is None:
    return note
  stop = check_edition(ref, src, base, errors)
  if stop is not None:
    return stop
  BaseComparison(root, ref, src, errors).compare(layout, listing)
  return ""


def open_base(
  root: Path, ref: str, layout: Layout, errors: Errors
) -> tuple[dict | None, set[Path], str]:
  """Read the base's index.toml and listing, if the base is to be compared.

  Args:
    root: the checkout.
    ref: the base commit.
    layout: the base's layout.
    errors: where errors go.

  Returns:
    The base's index.toml, None when the numbering is not compared; the
    files of the base under the source paths; and a note when the step is
    skipped, else "".
  """
  index, articles = layout.index, layout.articles
  listing = base_paths(root, ref, index, articles, RULES)
  try:
    base = tomllib.loads(git(root, "show", f"{ref}:{index}"))
  except tomllib.TOMLDecodeError as e:
    errors.add(index, 1, 7, f"{index} at base {ref} is not valid TOML: {e}")
    return None, listing, ""
  if base.get("status") != "active":
    return (
      None,
      listing,
      f"base {ref} is {base.get('status')}; numbering not compared",
    )
  if not any(
    p.name == ARTICLE_FILE and p.parent.parent == articles for p in listing
  ):
    return (
      None,
      listing,
      f"base {ref} has no {articles}/<NN>-<id>/{ARTICLE_FILE};"
      " numbering not compared",
    )
  return base, listing, ""


def check_edition(
  ref: str, src: Sources, base: dict, errors: Errors
) -> str | None:
  """Check the status and the edition against an active base.

  Args:
    ref: the base commit.
    src: the sources.
    base: the base's index.toml.
    errors: where errors go.

  Returns:
    None when the numbering is to be compared, else what check_base
    returns: a note when a later edition skips the comparison, or "".
  """
  if src.index.data["status"] != "active":
    errors.add(
      src.layout.index,
      src.index.line("status ="),
      7,
      f"status went back to draft from active at base {ref}",
    )
    return None
  edition, base_edition = src.index.data["edition"], base.get("edition")
  if edition == base_edition:
    return None
  # Both are `YYYY-MM`, so string order is month order.
  if isinstance(base_edition, str) and edition > base_edition:
    return (
      f"base {ref} is edition {base_edition}, this is edition {edition};"
      " numbering not compared"
    )
  errors.add(
    src.layout.index,
    src.index.line("edition ="),
    7,
    f"edition {edition} is not later than edition {base_edition} at base {ref}",
  )
  return ""


class BaseComparison:
  """The comparison of the sources with the files of a base commit."""

  def __init__(self, root: Path, ref: str, src: Sources, errors: Errors):
    """Set up the comparison.

    Args:
      root: the checkout.
      ref: the base commit.
      src: the sources.
      errors: where errors go.
    """
    self.root, self.ref, self.errors = root, ref, errors
    self.articles = {a.id: a for a in src.articles}
    self.sections = {s.id: s for s in src.sections}
    self.nodes = {n.id: n for n in src.rules}
    self.slugs = {n.slug: n for n in src.rules if n.slug is not None}

  def compare(self, layout: Layout, listing: set[Path]) -> None:
    """Compare every rule, article and section file of the base.

    Args:
      layout: the base's layout.
      listing: the files of the base under the source paths.
    """
    for path in sorted(listing):
      if path.parent == RULES and path.suffix == ".toml":
        if path.name != PARAMS_FILE:
          self.rule(path)
        continue
      if path.parent.parent != layout.articles or path.suffix != ".toml":
        continue
      article = ARTICLE_NAME.fullmatch(path.parent.name)
      section = SECTION_NAME.fullmatch(path.stem)
      if not article or (path.name != ARTICLE_FILE and not section):
        continue
      old = self.toml(path)
      if old is None:
        continue
      if path.name == ARTICLE_FILE:
        self.article(path, article, old)
        continue
      assert section is not None
      self.section(path, article, section, old)

  def toml(self, path: Path) -> dict | None:
    """Return a file of the base, parsed.

    Args:
      path: the file.

    Returns:
      The parsed file, or None when it is not valid TOML.
    """
    try:
      return tomllib.loads(git(self.root, "show", f"{self.ref}:{path}"))
    except tomllib.TOMLDecodeError as e:
      self.errors.add(path, 1, 7, f"at base {self.ref} is not valid TOML: {e}")
      return None

  def gone(self, path: Path, what: str) -> None:
    """Record that a provision of the base is gone.

    Args:
      path: its file at the base.
      what: the provision, for the message.
    """
    self.errors.add(
      path,
      1,
      7,
      f"{what} existed at base {self.ref} and is gone; repeal it instead",
    )

  def renumbered(self, doc: Doc, what: str, old: str, new: str) -> None:
    """Record a change of number, if there is one.

    Args:
      doc: the file now.
      what: the provision, for the message.
      old: its number at the base.
      new: its number now.
    """
    if old != new:
      self.errors.add(
        doc.path,
        doc.line(f'number = "{new}"'),
        7,
        f"{what} was renumbered from {old} to {new} since base {self.ref}",
      )

  def still_repealed(
    self, doc: Doc, what: str, was: bool, is_now: bool
  ) -> None:
    """Record a repeal undone, if there is one.

    Args:
      doc: the file now.
      what: the provision, for the message.
      was: whether it was repealed at the base.
      is_now: whether it is repealed now.
    """
    if was and not is_now:
      self.errors.add(
        doc.path,
        doc.line("repealed =", "status ="),
        7,
        f"{what} was repealed at base {self.ref} and is no longer",
      )

  def rule(self, path: Path) -> None:
    """Compare one rule file of the base.

    A rule of the older format, named by a word, may live on as a node of
    the rule tree whose slug is that word. A node of the base is never
    deleted, and stays superseded once superseded.

    Args:
      path: the file.
    """
    old = self.toml(path)
    if old is None:
      return
    if NODE_NAME.fullmatch(path.stem):
      node = self.nodes.get(int(path.stem))
      if node is None:
        self.gone(path, f"{old.get('kind', 'rule')} {path.stem}")
        return
      for status in ("repealed", "superseded"):
        if old.get("status") == status and node.status != status:
          self.errors.add(
            node.doc.path,
            node.doc.line("status ="),
            7,
            f"rule {node.id} was {status} at base {self.ref} and is no longer",
          )
      return
    now = self.slugs.get(path.stem)
    if now is None:
      self.errors.add(
        path,
        1,
        7,
        f"rule {path.stem} existed at base {self.ref} and no node has it as"
        " its slug",
      )
      return
    self.still_repealed(
      now.doc,
      f"rule {path.stem}",
      old.get("status") == "repealed",
      now.status == "repealed",
    )

  def article(self, path: Path, article: re.Match[str], old: dict) -> None:
    """Compare one article file of the base.

    Args:
      path: the file.
      article: the match of its directory name.
      old: the file, parsed.
    """
    aid = article.group(2)
    now = self.articles.get(aid)
    if now is None:
      self.gone(path, f"article {aid}")
      return
    self.renumbered(
      now.doc, f"article {aid}", name_number(article.group(1)), now.number
    )
    self.still_repealed(
      now.doc, f"article {aid}", "repealed" in old, bool(now.repealed)
    )

  def section(
    self, path: Path, article: re.Match[str], section: re.Match[str], old: dict
  ) -> None:
    """Compare one section file of the base, with its items.

    Args:
      path: the file.
      article: the match of its directory name.
      section: the match of its file name.
      old: the file, parsed.
    """
    sid = section.group(3)
    now = self.sections.get(sid)
    if now is None:
      self.gone(path, f"section {sid}")
      return
    if now.article != article.group(2):
      self.errors.add(
        now.doc.path,
        1,
        7,
        f"section {sid} moved from article {article.group(2)}"
        f" to {now.article} since base {self.ref}",
      )
    else:
      self.renumbered(
        now.doc,
        f"section {sid}",
        name_number(section.group(1), section.group(2)),
        now.number,
      )
    self.still_repealed(
      now.doc,
      f"section {sid}",
      old.get("status") == "repealed",
      now.status == "repealed",
    )
    items = {i.id: i for i in now.items}
    for item in old.get("items", []):
      iid = item.get("id")
      what = f"item {sid}.{iid}"
      if iid not in items:
        self.errors.add(
          now.doc.path,
          now.doc.line("[[items]]", "[content]"),
          7,
          f"{what} existed at base {self.ref} and is gone; repeal it instead",
        )
        continue
      self.renumbered(now.doc, what, item.get("number"), items[iid].number)
      self.still_repealed(
        now.doc,
        what,
        item.get("status") == "repealed",
        items[iid].status == "repealed",
      )
