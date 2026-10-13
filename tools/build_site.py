#!/usr/bin/env python3
"""Build the static site from the built constitution with Hugo.

Each constitution/constitution.<lang>.md becomes the home page of its
language: the header line that says the file is generated is dropped, the
title goes into the front matter, the revision and the length of the text
are set under the title, and the text is wrapped in
`<div class="constitution">` for the style sheet. The token count comes
from Anthropic's count_tokens API when CONSTITUTION_TOKEN_COUNT_ACCESS_TOKEN
or CONSTITUTION_TOKEN_COUNT_API_KEY is set; without either, or when the
call fails, the page says the tokens were not counted and the build goes
on; with --require-tokens it fails instead. The pages are written to a
temporary content directory, Hugo builds web/ (the hugo-book theme, a
submodule) into site/, English at site/ and Chinese at site/zh/, and
check_site_anchors.py checks the result. site/ is not committed. The setup is described in
docs/design.md (Static site).
"""

import argparse
import http.client
import json
import os
import re
import subprocess
import sys
import tempfile
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

import check_site_anchors

ROOT = Path(__file__).resolve().parent.parent
HUGO = "hugo-0.167.0"  # the same version as the CI step that builds the site
# `## <a id="general-provisions"></a>Article I. ...`
HEADING_ANCHOR = re.compile(r'^(#{2,6}) <a id="([^"]+)"></a>(.*)$', re.M)
KEY_VARIABLE = "CONSTITUTION_TOKEN_COUNT_API_KEY"  # sent as x-api-key
# A short-lived access token from Workload Identity Federation, sent as
# Authorization: Bearer; it wins over the key when both are set.
TOKEN_VARIABLE = "CONSTITUTION_TOKEN_COUNT_ACCESS_TOKEN"
COUNT_URL = "https://api.anthropic.com/v1/messages/count_tokens"
COUNT_MODEL = "claude-opus-5-5"
COUNT_TIMEOUT = 10  # seconds per connect or read; a stalled API must not hold up the build
# Language: the content file and the labels under the title.
PAGES = {
    "en": {"file": "_index.md", "revision": "Revision",
           "length": "{chars:,} characters · {words:,} words",
           "tokens": "{tokens:,} tokens (counted for {model})", "no_tokens": "tokens not counted"},
    "zh": {"file": "_index.zh.md", "revision": "修订版",
           "length": "{chars:,} 字符",
           "tokens": "{tokens:,} token (按 {model} 计数)", "no_tokens": "token 数未计算"},
}


def counted(markdown: str) -> str:
    """The text whose length the page gives: the built constitution without
    its header line, anchors and link definitions included, since the
    compiler takes that text into an instruction file."""
    return markdown.partition("\n")[2]


def credential(environ: dict[str, str]) -> dict[str, str] | None:
    """The header that authenticates the count, or None without a credential."""
    if token := environ.get(TOKEN_VARIABLE):
        return {"authorization": f"Bearer {token}"}
    if key := environ.get(KEY_VARIABLE):
        return {"x-api-key": key}
    return None


def count_tokens(text: str, auth: dict[str, str] | None) -> int | None:
    """The tokens of text for COUNT_MODEL, or None without a credential or
    when the call fails. A failure prints one line with the HTTP status or
    the exception's class, never the request or the exception, which may
    hold the credential."""
    if not auth:
        return None
    request = urllib.request.Request(
        COUNT_URL,
        data=json.dumps({"model": COUNT_MODEL, "messages": [{"role": "user", "content": text}]}).encode(),
        headers={**auth, "anthropic-version": "2023-06-01", "content-type": "application/json"})
    try:
        with urllib.request.urlopen(request, timeout=COUNT_TIMEOUT) as response:
            return int(json.load(response)["input_tokens"])
    except urllib.error.HTTPError as e:
        print(f"token count: HTTP {e.code}; tokens not counted", file=sys.stderr)
    except (OSError, http.client.HTTPException, ValueError, KeyError, TypeError) as e:
        print(f"token count: {type(e).__name__}; tokens not counted", file=sys.stderr)
    return None


def page(markdown: str, labels: dict[str, str], revision: str, tokens: int | None) -> str:
    """The Hugo page of one built constitution."""
    lines = markdown.split("\n")
    if len(lines) < 3 or "tools/build_constitution.py" not in lines[0] or not lines[2].startswith("# "):
        raise ValueError("does not start with the build's header line, a blank line and the `# ` title")
    front = f"+++\ntitle = {json.dumps(lines[2][2:], ensure_ascii=False)}\n+++"
    text = counted(markdown)
    length = labels["length"].format(chars=len(text), words=len(text.split()))
    if tokens is None:
        length += " · " + labels["no_tokens"]
    else:
        length += " · " + labels["tokens"].format(tokens=tokens, model=COUNT_MODEL)
    revision_line = (f'<p class="revision">{labels["revision"]} {revision}<br>'
                     f'<span class="length">{length}</span></p>')
    body = HEADING_ANCHOR.sub(r"\1 \3 {#\2}", "\n".join(lines[3:]))
    return "\n".join([front, '<div class="constitution">', "", lines[2], "", revision_line,
                      body, "", "</div>", ""])


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=ROOT / "site", help="the output directory (default: site/)")
    parser.add_argument("--require-tokens", action="store_true",
                        help="fail when the tokens of a page are not counted (the token-count workflow)")
    args = parser.parse_args(argv)
    revision = tomllib.loads((ROOT / "constitution/index.toml").read_text(encoding="utf-8"))["revision"]
    auth = credential(os.environ)
    if not auth:
        print(f"neither {TOKEN_VARIABLE} nor {KEY_VARIABLE} is set; tokens not counted", file=sys.stderr)
        if args.require_tokens:
            return 1
    with tempfile.TemporaryDirectory() as content:
        for lang, labels in PAGES.items():
            source = ROOT / f"constitution/constitution.{lang}.md"
            markdown = source.read_text(encoding="utf-8")
            tokens = count_tokens(counted(markdown), auth)
            if tokens is None and args.require_tokens:
                print(f"{source.relative_to(ROOT)}: tokens not counted; --require-tokens fails", file=sys.stderr)
                return 1
            try:
                text = page(markdown, labels, revision, tokens)
            except ValueError as e:
                print(f"{source.relative_to(ROOT)}: {e}", file=sys.stderr)
                return 1
            (Path(content) / labels["file"]).write_text(text, encoding="utf-8")
        build = subprocess.run([HUGO, "--source", str(ROOT / "web"), "--contentDir", content,
                                "--destination", str(args.out.resolve()), "--cleanDestinationDir",
                                "--panicOnWarning"])
    if build.returncode:
        return build.returncode
    return check_site_anchors.main([str(args.out)])


if __name__ == "__main__":
    sys.exit(main())
