"""Take screenshots of the pages of a built static site with headless Chrome.

Serves the directory on a local port and takes, for each page path, one
screenshot per color scheme (light, dark) and width (desktop, mobile),
one Chrome run each, one after the other. The script knows nothing of
the generator that built the site: its input is the directory and the
paths, and a language is just another path (`/zh/rules/`).

Usage: uv run site-screenshots [<dir>] [--out <dir>] [--page <path> ...],
default site/ and site-screenshots/ of the repo this script is in, and
the pages of DEFAULT_PAGES; a path without its leading `/` gets one.
The file of a page is named by its path, the scheme and the width:
`zh-rules-79.dark.mobile.png`. The exit
status is 1 when the site has no page at a path, Chrome is missing or a
screenshot is not written.
"""

import argparse
from collections.abc import Iterator, Sequence
import contextlib
import functools
import http.server
import itertools
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
from typing import NamedTuple

ROOT = Path(__file__).resolve().parents[4]

DEFAULT_PAGES = (
  "/",
  "/rules/",
  "/rules/79/",
  "/rules/132/",
  "/zh/",
  "/zh/rules/",
  "/zh/rules/79/",
  "/zh/rules/132/",
)

# The light and dark schemes are the browser's preferred color scheme:
# Chrome reports light by default and dark with --force-dark-mode. The
# site's theme follows it while the reader has not chosen a theme, which
# a fresh Chrome profile never has. A site whose theme does not follow
# the preferred scheme needs another switch here.
SCHEMES = {"light": (), "dark": ("--force-dark-mode",)}

WIDTHS = {"desktop": (1280, 900), "mobile": (390, 844)}


class Shot(NamedTuple):
  """One screenshot: a page path, a color scheme and a width.

  Attributes:
    path: The page path, as `/rules/79/`.
    scheme: A key of SCHEMES.
    width: A key of WIDTHS.
  """

  path: str
  scheme: str
  width: str

  @property
  def name(self) -> str:
    """The file name: the path's parts joined by `-`, `index` for `/`.

    Returns:
      The name, as `rules-79.dark.mobile.png`.
    """
    page = "-".join(part for part in self.path.split("/") if part) or "index"
    return f"{page}.{self.scheme}.{self.width}.png"


def shots(paths: Sequence[str]) -> list[Shot]:
  """Return the screenshots to take of the pages, in order.

  Args:
    paths: The page paths.

  Returns:
    One shot per path, scheme and width.
  """
  return [Shot(*shot) for shot in itertools.product(paths, SCHEMES, WIDTHS)]


def page_file(site: Path, path: str) -> Path:
  """Return the file a static server sends for a page path.

  Args:
    site: The built site.
    path: The page path.

  Returns:
    The file, `index.html` for a path that ends in `/`.
  """
  file = site / path.lstrip("/")
  return file / "index.html" if path.endswith("/") else file


def chrome_command(
  chrome: str, profile: Path, shot: Shot, url: str, out: Path
) -> list[str]:
  """Return the Chrome command that takes one screenshot.

  Args:
    chrome: The Chrome executable.
    profile: A fresh user data directory.
    shot: The screenshot to take.
    url: The page's URL.
    out: The PNG file to write.

  Returns:
    The command.
  """
  width, height = WIDTHS[shot.width]
  return [
    chrome,
    "--headless",
    "--no-proxy-server",
    "--hide-scrollbars",
    f"--user-data-dir={profile}",
    f"--window-size={width},{height}",
    *SCHEMES[shot.scheme],
    f"--screenshot={out}",
    url,
  ]


@functools.cache
def find_chrome() -> str | None:
  """Return the first Chrome executable on the PATH.

  Returns:
    Its path, or None when there is none.
  """
  names = ("google-chrome", "google-chrome-stable", "chromium")
  return next(filter(None, map(shutil.which, names)), None)


class QuietHandler(http.server.SimpleHTTPRequestHandler):
  """A static file handler that logs nothing."""

  def log_message(self, format: str, *args: object) -> None:  # noqa: A002
    """Log nothing.

    Args:
      format: Unused.
      *args: Unused.
    """


@contextlib.contextmanager
def serve(site: Path) -> Iterator[str]:
  """Serve the site on a free local port while the context is open.

  Args:
    site: The built site.

  Yields:
    The base URL, without a trailing `/`.
  """
  handler = functools.partial(QuietHandler, directory=str(site))
  with http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
      yield f"http://127.0.0.1:{server.server_address[1]}"
    finally:
      server.shutdown()
      thread.join()


def page_path(arg: str) -> str:
  """Return a page path of the command line, starting with `/`.

  The URL of a page is the server's base and the path, so a path
  without its `/` would run into the port.

  Args:
    arg: The path as given, `rules/79/` or `/rules/79/`.

  Returns:
    The path with one leading `/`.
  """
  return "/" + arg.lstrip("/")


def parser() -> argparse.ArgumentParser:
  """Return the parser of the command line.

  Returns:
    The parser.
  """
  parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
  parser.add_argument(
    "dir",
    nargs="?",
    default=ROOT / "site",
    type=Path,
    help="the built site (default: site/ of the repo this script is in)",
  )
  parser.add_argument(
    "--out",
    default=ROOT / "site-screenshots",
    type=Path,
    help="the directory for the screenshots (default: site-screenshots/)",
  )
  parser.add_argument(
    "--page",
    action="append",
    dest="pages",
    type=page_path,
    help="a page path, as /rules/79/; repeat for more (default: a set list)",
  )
  return parser


def main(argv: list[str] | None = None) -> int:
  """Take the screenshots and print the file of each.

  Args:
    argv: The command-line arguments, or None for sys.argv.

  Returns:
    The exit status: 1 on any error, else 0.
  """
  args = parser().parse_args(argv)
  paths = args.pages or DEFAULT_PAGES
  missing = [p for p in paths if not page_file(args.dir, p).is_file()]
  for path in missing:
    print(f"{args.dir}: no page at {path}", file=sys.stderr)
  chrome = find_chrome()
  if chrome is None:
    print("no Chrome on the PATH", file=sys.stderr)
  if missing or chrome is None:
    return 1
  args.out.mkdir(parents=True, exist_ok=True)
  failed = False
  with tempfile.TemporaryDirectory() as tmp, serve(args.dir) as base:
    for n, shot in enumerate(shots(paths)):
      out = args.out / shot.name
      out.unlink(missing_ok=True)
      command = chrome_command(
        chrome, Path(tmp, str(n)), shot, base + shot.path, out
      )
      result = subprocess.run(
        command, capture_output=True, text=True, timeout=120, check=False
      )
      if result.returncode or not out.is_file():
        print(f"{out}: not written\n{result.stderr}", file=sys.stderr)
        failed = True
      else:
        print(out)
  return 1 if failed else 0


if __name__ == "__main__":
  sys.exit(main())
