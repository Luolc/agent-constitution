"""Check 7: the numbering and the revision against a base commit."""

from pathlib import Path
import re
import subprocess
import tomllib

from constitution_compiler import git_env
from constitution_compiler.sources import (
  ARTICLE_FILE,
  ARTICLE_NAME,
  ARTICLES,
  Doc,
  Errors,
  INDEX,
  name_number,
  RETIRED,
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


def check_base(root: Path, ref: str, src: Sources, errors: Errors) -> str:
  """Check 7: compare the numbering with a base commit.

  Once the base is active: the edition is the base's or a later one.
  While it is the base's: every article,
  section and item of the base still exists, by ID, with the same number,
  a section in the same article; what was repealed is still repealed;
  every rule of the base still exists or is retired, and every retired ID
  stays retired.

  Args:
    root: the checkout.
    ref: the base commit.
    src: the sources.
    errors: where errors go.

  Returns:
    A note when the step is skipped, else "".
  """
  base, listing, note = open_base(root, ref, errors)
  if base is None:
    return note
  stop = check_edition(ref, src, base, errors)
  if stop is not None:
    return stop
  BaseComparison(root, ref, src, errors).compare(listing)
  return ""


def open_base(
  root: Path, ref: str, errors: Errors
) -> tuple[dict | None, set[Path], str]:
  """Read the base's index.toml and listing, if the base is to be compared.

  Args:
    root: the checkout.
    ref: the base commit.
    errors: where errors go.

  Returns:
    The base's index.toml, None when the numbering is not compared; the
    files of the base under the source paths; and a note when the step is
    skipped, else "".
  """
  commit = f"{ref}^{{commit}}"
  try:
    git(root, "rev-parse", "--verify", "--quiet", commit)
  except GitError:
    errors.add(INDEX, 1, 7, f"base `{ref}` is not a commit")
    return None, set(), ""
  listing = base_paths(root, ref, INDEX, ARTICLES, RULES, RETIRED)
  if INDEX not in listing:
    return None, listing, f"base {ref} has no {INDEX}; numbering not compared"
  try:
    base = tomllib.loads(git(root, "show", f"{ref}:{INDEX}"))
  except tomllib.TOMLDecodeError as e:
    errors.add(INDEX, 1, 7, f"{INDEX} at base {ref} is not valid TOML: {e}")
    return None, listing, ""
  if base.get("status") != "active":
    return (
      None,
      listing,
      f"base {ref} is {base.get('status')}; numbering not compared",
    )
  if not any(
    p.name == ARTICLE_FILE and p.parent.parent == ARTICLES for p in listing
  ):
    return (
      None,
      listing,
      f"base {ref} has no {ARTICLES}/<NN>-<id>/{ARTICLE_FILE};"
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
      INDEX,
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
    INDEX,
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
    self.rules = {r.id: r for r in src.rules}
    self.retired = set(src.retired)

  def compare(self, listing: set[Path]) -> None:
    """Compare every rule, article and section file of the base.

    Args:
      listing: the files of the base under the source paths.
    """
    for path in sorted(listing):
      if path.parent == RULES and path.suffix == ".toml":
        self.rule(path)
        continue
      if path.parent.parent != ARTICLES or path.suffix != ".toml":
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
    if RETIRED in listing:
      self.retired_ids()

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

    Args:
      path: the file.
    """
    old = self.toml(path)
    now = self.rules.get(path.stem)
    if old is None:
      return
    if now is None:
      if path.stem not in self.retired:
        self.errors.add(
          path,
          1,
          7,
          f"rule {path.stem} existed at base {self.ref} and is gone"
          f" without being listed in {RETIRED}",
        )
      return
    self.still_repealed(
      now.doc,
      f"rule {now.id}",
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

  def retired_ids(self) -> None:
    """Check that every ID retired at the base is still retired."""
    for value in git(self.root, "show", f"{self.ref}:{RETIRED}").splitlines():
      if value and value not in self.retired:
        self.errors.add(
          RETIRED,
          1,
          7,
          f"{value} was retired at base {self.ref} and is no longer listed",
        )


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


def source_text_changed(root: Path, ref: str) -> bool:
  """Return whether the constitution text differs from the base's.

  The text is the article sources and `index.toml` other than its
  `revision` line. The files git tracks are compared as they are in the
  working tree, so the result does not depend on what is committed.

  Args:
    root: the checkout.
    ref: the base commit.

  Returns:
    True when the text changed.
  """
  drop = re.compile(r'^revision = ".*"\n', re.MULTILINE)
  tracked = git(root, "ls-files", "--", str(ARTICLES))
  paths = {Path(p) for p in tracked.split("\n") if p and (root / p).is_file()}
  if paths != base_paths(root, ref, ARTICLES):
    return True
  if any(
    (root / p).read_text(encoding="utf-8") != git(root, "show", f"{ref}:{p}")
    for p in paths
  ):
    return True
  base_index = git(root, "show", f"{ref}:{INDEX}")
  return drop.sub("", (root / INDEX).read_text(encoding="utf-8")) != drop.sub(
    "", base_index
  )


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
  if INDEX not in base_paths(root, ref, INDEX):
    return
  try:
    base = tomllib.loads(git(root, "show", f"{ref}:{INDEX}"))
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
      INDEX,
      line,
      7,
      f"revision {revision} is neither `0.<n>` nor a whole number from 1",
    )
  elif base_key is None:
    return
  elif source_text_changed(root, ref):
    if key <= base_key:
      errors.add(
        INDEX,
        line,
        7,
        f"the constitution text changed since base {ref}"
        f" but revision {revision} is not greater"
        f" than the base's {base_revision}",
      )
  elif revision != base_revision:
    errors.add(
      INDEX,
      line,
      7,
      f"revision {revision} differs from revision {base_revision}"
      f" at base {ref} but the constitution text did not change",
    )
