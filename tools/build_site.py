#!/usr/bin/env python3
"""Build the static site from the built constitution with Hugo.

Each constitution/constitution.<lang>.md becomes the home page of its
language: the header line that says the file is generated is dropped, the
title goes into the front matter, the revision is set under the title, and
the text is wrapped in `<div class="constitution">` for the style sheet.
The pages are written to a temporary content directory, Hugo builds web/
(the hugo-book theme, a submodule) into site/, English at site/ and Chinese
at site/zh/, and check_site_anchors.py checks the result. site/ is not
committed. The setup is described in docs/design.md (Static site).
"""

import argparse
import json
import re
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

import check_site_anchors

ROOT = Path(__file__).resolve().parent.parent
HUGO = "hugo-0.167.0"  # the same version as the CI step that builds the site
# `## <a id="general-provisions"></a>Article I. ...`
HEADING_ANCHOR = re.compile(r'^(#{2,6}) <a id="([^"]+)"></a>(.*)$', re.M)
# Language: (content file, revision label).
PAGES = {
    "en": ("_index.md", "Revision"),
    "zh": ("_index.zh.md", "修订版"),
}


def page(markdown: str, label: str, revision: str) -> str:
    """The Hugo page of one built constitution."""
    lines = markdown.split("\n")
    if len(lines) < 3 or "tools/build_constitution.py" not in lines[0] or not lines[2].startswith("# "):
        raise ValueError("does not start with the build's header line, a blank line and the `# ` title")
    front = f"+++\ntitle = {json.dumps(lines[2][2:], ensure_ascii=False)}\n+++"
    revision_line = f'<p class="revision">{label} {revision}</p>'
    body = HEADING_ANCHOR.sub(r"\1 \3 {#\2}", "\n".join(lines[3:]))
    return "\n".join([front, '<div class="constitution">', "", lines[2], "", revision_line,
                      body, "", "</div>", ""])


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=ROOT / "site", help="the output directory (default: site/)")
    args = parser.parse_args(argv)
    revision = tomllib.loads((ROOT / "constitution/index.toml").read_text(encoding="utf-8"))["revision"]
    with tempfile.TemporaryDirectory() as content:
        for lang, (name, label) in PAGES.items():
            source = ROOT / f"constitution/constitution.{lang}.md"
            try:
                text = page(source.read_text(encoding="utf-8"), label, revision)
            except ValueError as e:
                print(f"{source.relative_to(ROOT)}: {e}", file=sys.stderr)
                return 1
            (Path(content) / name).write_text(text, encoding="utf-8")
        build = subprocess.run([HUGO, "--source", str(ROOT / "web"), "--contentDir", content,
                                "--destination", str(args.out.resolve()), "--cleanDestinationDir",
                                "--panicOnWarning"])
    if build.returncode:
        return build.returncode
    return check_site_anchors.main([str(args.out)])


if __name__ == "__main__":
    sys.exit(main())
