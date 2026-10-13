#!/usr/bin/env python3
"""Check the built static site: every in-page link `href="#id"` has its
target `id` on the same page, no id appears twice on a page, and no list
marker leaked into the text as a literal `- §` or `- 第` (a sign that the
Markdown renderer did not see a list item where the build wrote one).

Usage: check_site_anchors.py [<dir>], default site/. Errors print as
`path: message`; the exit status is 1 on any error, also when the
directory holds no HTML page.
"""

import argparse
import re
import sys
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote

LEAKED_MARKER = re.compile(r"(?:^|\s)- (?:§\d|第[〇一二三四五六七八九十百]+款)")


class Page(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids, self.links, self.text = [], set(), []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if attrs.get("id"):
            self.ids.append(attrs["id"])
        href = attrs.get("href") or ""
        if tag == "a" and href.startswith("#") and len(href) > 1:
            # Percent-encoded as browsers decode it: #%e5%ba%8f is #序.
            self.links.add(unquote(href[1:]))

    def handle_data(self, data):
        self.text.append(data)


def check(html: str) -> list[str]:
    page = Page()
    page.feed(html)
    page.close()
    errors = []
    ids = set(page.ids)
    errors += [f"duplicate id {i!r}" for i in sorted(ids) if page.ids.count(i) > 1]
    errors += [f"link #{t} has no target on the page" for t in sorted(page.links - ids)]
    for m in LEAKED_MARKER.finditer("".join(page.text)):
        errors.append(f"literal list marker in the text: {m.group().strip()!r}")
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("dir", nargs="?", default="site", type=Path, help="the built site (default: site/)")
    args = parser.parse_args(argv)
    pages = sorted(args.dir.rglob("*.html"))
    if not pages:
        print(f"{args.dir}: no HTML page", file=sys.stderr)
        return 1
    failed = False
    for path in pages:
        for error in check(path.read_text(encoding="utf-8")):
            print(f"{path}: {error}", file=sys.stderr)
            failed = True
    if not failed:
        print(f"{len(pages)} pages, every in-page link has its target")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
