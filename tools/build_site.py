#!/usr/bin/env python3
"""Build the static site from the built constitution with Hugo.

Each constitution/constitution.<lang>.md becomes the home page of its
language: the header line that says the file is generated is dropped, the
title goes into the front matter, the revision and the length of the
version for agents are set under the title, with a line on the asterisk
when the text marks provisions left out of that version, and the text is
wrapped in `<div class="constitution">` for the style sheet. An info button
after the length opens a popup that says what was counted, gives the
length of the full version and links to the version for agents, which
constitution/constitution.<lang>.agent.md makes into a page of its own,
agent/ under the home page of its language. The token counts come from
Anthropic's count_tokens API when CONSTITUTION_TOKEN_COUNT_ACCESS_TOKEN or
CONSTITUTION_TOKEN_COUNT_API_KEY is set; without either, or when a call
fails, the page says the tokens were not counted and the build goes on;
with --require-tokens it fails instead. The pages are written to a
temporary content directory, Hugo builds web/ (the hugo-book theme, a
submodule) into site/, English at site/ and Chinese at site/zh/, and
check_site_anchors.py checks the result. site/ is not committed. The
setup is described in docs/design.md (Static site).
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
from build_constitution import OMITTED_MARK

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
# Language: the content files, the labels under the title and the text of
# the popup on the counts.
PAGES = {
    "en": {"file": "_index.md", "agent_file": "agent.md", "home": "/", "revision": "Revision",
           "length": "{chars:,} characters · {words:,} words",
           "tokens": "{tokens:,} tokens (counted for {model})", "no_tokens": "tokens not counted",
           "agent_length": "Version for agents: {length}",
           "omitted": ("Sections and items marked {mark} are written for human readers: how this constitution is"
                       " organized and amended. {agent} leaves out their text and keeps only a pointer to this"
                       " full version."),
           "omitted_agent": "The version compiled for agents",
           "info": "About these numbers",
           "counted": "Tokens are counted by Anthropic's token counting API (count_tokens) for {model}.",
           "not_counted": ("Tokens were not counted: the build had no credential for Anthropic's token"
                           " counting API (count_tokens), or the call failed."),
           "agent_note": ("The numbers are those of the version agents read, which leaves out the text of the"
                          " sections and items written for human readers, keeping only a pointer to the full"
                          " version, and has no anchors or links."),
           "full": "The full version on this page: {length}.",
           "source": "Counted at revision {revision}{commit}.",
           "commit": ", commit {commit}",
           "agent_link": "See the version agents read",
           "back": "Back to the full version"},
    "zh": {"file": "_index.zh.md", "agent_file": "agent.zh.md", "home": "/zh/", "revision": "修订号",
           "length": "{chars:,} 字符",
           "tokens": "{tokens:,} token (按 {model} 计数)", "no_tokens": "token 数未计算",
           "agent_length": "agent 版：{length}",
           "omitted": "标 {mark} 的条款写给人读，讲本宪法怎样编排、怎样修改。{agent}不收这些条款的正文，只留一句「见完整版」。",
           "omitted_agent": "给 agent 编译的版本",
           "info": "关于这些数字",
           "counted": "token 数由 Anthropic 的 token 计数接口 (count_tokens) 按 {model} 计算。",
           "not_counted": "token 数未计算：构建时没有 Anthropic token 计数接口 (count_tokens) 的凭证，或者调用失败。",
           "agent_note": "这里的数字是 agent 读的精简版：写给人读的条款不收正文，只留一句「见完整版」，也不含锚点和链接。",
           "full": "本页完整版：{length}。",
           "source": "按修订号 {revision}{commit} 计算。",
           "commit": "、commit {commit}",
           "agent_link": "查看 agent 读到的版本",
           "back": "回到完整版"},
}


def counted(markdown: str) -> str:
    """The text whose length the page gives: the full version of the built
    constitution without its header line, anchors and link definitions
    included."""
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


INFO_ICON = ('<svg viewBox="0 0 16 16" width="1em" height="1em" aria-hidden="true">'
             '<circle cx="8" cy="8" r="7" fill="none" stroke="currentColor" stroke-width="1.5"/>'
             '<circle cx="8" cy="4.75" r="1" fill="currentColor"/>'
             '<path d="M8 7v5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>')

# A heading attribute that tells hugo-book's heading render hook to leave
# out its `#` link.
NO_ANCHOR = " {anchor=false}"


def split(markdown: str) -> tuple[str, list[str]]:
    """The `# ` title of a built constitution and the lines after it."""
    lines = markdown.split("\n")
    if len(lines) < 3 or "tools/build_constitution.py" not in lines[0] or not lines[2].startswith("# "):
        raise ValueError("does not start with the build's header line, a blank line and the `# ` title")
    return lines[2], lines[3:]


def front(title: str, *extra: str) -> str:
    return "\n".join(["+++", f"title = {json.dumps(title[2:], ensure_ascii=False)}", *extra, "+++"])


def length(text: str, labels: dict[str, str], tokens: int | None) -> str:
    """The length of text: characters, in English words, and tokens."""
    line = labels["length"].format(chars=len(text), words=len(text.split()))
    if tokens is None:
        return line + " · " + labels["no_tokens"]
    return line + " · " + labels["tokens"].format(tokens=tokens, model=COUNT_MODEL)


def page(markdown: str, agent_markdown: str, labels: dict[str, str], revision: str, commit: str | None,
         tokens: int | None, agent_tokens: int | None) -> str:
    """The Hugo page of one built constitution, given the version for agents
    whose length it shows and the counts of both."""
    title, rest = split(markdown)
    text = counted(markdown)
    counts = labels["counted"].format(model=COUNT_MODEL)
    if tokens is None or agent_tokens is None:
        counts = labels["not_counted"]
    source = labels["source"].format(revision=revision, commit=labels["commit"].format(commit=commit) if commit else "")
    agent_length = labels["agent_length"].format(length=length(counted(agent_markdown), labels, agent_tokens))
    popup = "".join(f'<span class="count-line">{line}</span>' for line in [
        counts, labels["agent_note"], labels["full"].format(length=length(text, labels, tokens)), source,
        f'<a href="{labels["home"]}agent/">{labels["agent_link"]}</a>'])
    revision_line = (f'<p class="revision">{labels["revision"]} {revision}<br><span class="length">{agent_length}</span>'
                     f'<span class="count-info"><button type="button" class="count-button" aria-expanded="false"'
                     f' aria-controls="count-popup" aria-label="{labels["info"]}">{INFO_ICON}</button>'
                     f'<span class="count-popup" id="count-popup">{popup}</span></span>')
    if OMITTED_MARK in text:
        legend = labels["omitted"].format(mark='<span class="mark">*</span>',
                                          agent=f'<a href="{labels["home"]}agent/">{labels["omitted_agent"]}</a>')
        revision_line += f'<br><span class="legend">{legend}</span>'
    revision_line += "</p>"
    body = HEADING_ANCHOR.sub(r"\1 \3 {#\2}", "\n".join(rest))
    return "\n".join([front(title), '<div class="constitution">', "", title, "", revision_line,
                      body, "", "</div>", ""])


def agent_page(agent_markdown: str, labels: dict[str, str], revision: str) -> str:
    """The Hugo page of the version for agents: its text as agents read it,
    under the revision and a link back to the full version. Hidden from
    the sidebar's list of pages; its headings get no anchor link from the
    theme."""
    title, rest = split(agent_markdown)
    revision_line = (f'<p class="revision">{labels["revision"]} {revision}<br>'
                     f'<span class="back"><a href="{labels["home"]}">{labels["back"]}</a></span></p>')
    rest = [line + NO_ANCHOR if line.startswith("#") else line for line in rest]
    return "\n".join([front(title, "bookHidden = true"), '<div class="constitution">', "", title + NO_ANCHOR, "",
                      revision_line, *rest, "", "</div>", ""])


def short_commit() -> str | None:
    """The abbreviated commit the build runs on, or None outside git."""
    try:
        result = subprocess.run(["git", "-C", str(ROOT), "rev-parse", "--short", "HEAD"],
                                capture_output=True, text=True)
    except OSError:
        return None
    return result.stdout.strip() if result.returncode == 0 else None


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
    commit = short_commit()
    with tempfile.TemporaryDirectory() as content:
        for lang, labels in PAGES.items():
            pages = {}
            for version in ("", ".agent"):
                source = ROOT / f"constitution/constitution.{lang}{version}.md"
                markdown = source.read_text(encoding="utf-8")
                try:
                    split(markdown)
                except ValueError as e:
                    print(f"{source.relative_to(ROOT)}: {e}", file=sys.stderr)
                    return 1
                tokens = count_tokens(counted(markdown), auth)
                if tokens is None and args.require_tokens:
                    print(f"{source.relative_to(ROOT)}: tokens not counted; --require-tokens fails", file=sys.stderr)
                    return 1
                pages[version] = markdown, tokens
            (markdown, tokens), (agent_markdown, agent_tokens) = pages[""], pages[".agent"]
            text = page(markdown, agent_markdown, labels, revision, commit, tokens, agent_tokens)
            (Path(content) / labels["file"]).write_text(text, encoding="utf-8")
            (Path(content) / labels["agent_file"]).write_text(agent_page(agent_markdown, labels, revision),
                                                              encoding="utf-8")
        build = subprocess.run([HUGO, "--source", str(ROOT / "web"), "--contentDir", content,
                                "--destination", str(args.out.resolve()), "--cleanDestinationDir",
                                "--panicOnWarning"])
    if build.returncode:
        return build.returncode
    return check_site_anchors.main([str(args.out)])


if __name__ == "__main__":
    sys.exit(main())
