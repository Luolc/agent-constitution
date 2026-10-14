#!/usr/bin/env python3
"""Check the built static site's links, ids and list markers.

Every in-page link `href="#id"` has its target `id` on the same page,
every relative link to another page leads to a file of the site, no id
appears twice on a page, and no list marker leaked into the text as a
literal `- §` or `- 第` (a sign that the Markdown renderer did not see a
list item where the build wrote one).

Usage: check_site_anchors.py [<dir>], default site/. Errors print as
`path: message`; the exit status is 1 on any error, also when the
directory holds no HTML page.
"""

import argparse
from html.parser import HTMLParser
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit

LEAKED_MARKER = re.compile(r"(?:^|\s)- (?:§\d|第[〇一二三四五六七八九十百]+款)")


class Page(HTMLParser):
  """The ids, links and text of one HTML page."""

  def __init__(self):
    """Start with no ids, links or text."""
    super().__init__()
    self.ids, self.links, self.pages, self.text = [], set(), set(), []

  def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]):
    """Record the tag's id and, for a link, its target.

    Args:
      tag: The tag's name.
      attrs: The tag's attributes.
    """
    values = dict(attrs)
    if tag_id := values.get("id"):
      self.ids.append(tag_id)
    href = values.get("href") or ""
    if tag == "a" and href.startswith("#") and len(href) > 1:
      # Percent-encoded as browsers decode it: #%e5%ba%8f is #序.
      self.links.add(unquote(href[1:]))
    elif (
      tag == "a"
      and href
      and not urlsplit(href).scheme
      and not href.startswith(("#", "//"))
    ):
      self.pages.add(unquote(urlsplit(href).path))

  def handle_data(self, data: str):
    """Record the text.

    Args:
      data: Text between tags.
    """
    self.text.append(data)


def check(html: str, path: Path | None = None) -> list[str]:
  """Check one page.

  With its path, its relative links to other pages are resolved from there
  and must lead to a file, `dir/` to `dir/index.html`.

  Args:
    html: The page.
    path: Where the page is, or None to skip the links to other pages.

  Returns:
    The errors of the page.
  """
  page = Page()
  page.feed(html)
  page.close()
  errors = []
  ids = set(page.ids)
  errors += [
    f"duplicate id {i!r}" for i in sorted(ids) if page.ids.count(i) > 1
  ]
  errors += [
    f"link #{t} has no target on the page" for t in sorted(page.links - ids)
  ]
  if path:
    for href in sorted(page.pages):
      target = path.parent / href
      if href.endswith("/") or target.is_dir():
        target /= "index.html"
      if not target.is_file():
        errors.append(f"link {href} leads to no file of the site")
  for m in LEAKED_MARKER.finditer("".join(page.text)):
    errors.append(f"literal list marker in the text: {m.group().strip()!r}")
  return errors


def main(argv: list[str] | None = None) -> int:
  """Check every page of the site and print the errors.

  Args:
    argv: The command-line arguments, or None for sys.argv.

  Returns:
    The exit status: 1 on any error, else 0.
  """
  parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
  parser.add_argument(
    "dir",
    nargs="?",
    default="site",
    type=Path,
    help="the built site (default: site/)",
  )
  args = parser.parse_args(argv)
  pages = sorted(args.dir.rglob("*.html"))
  if not pages:
    print(f"{args.dir}: no HTML page", file=sys.stderr)
    return 1
  failed = False
  for path in pages:
    for error in check(path.read_text(encoding="utf-8"), path):
      print(f"{path}: {error}", file=sys.stderr)
      failed = True
  if not failed:
    print(f"{len(pages)} pages, every link has its target")
  return 1 if failed else 0


if __name__ == "__main__":
  sys.exit(main())
