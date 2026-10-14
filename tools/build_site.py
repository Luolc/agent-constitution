#!/usr/bin/env python3
"""Build the static site from the built constitution with Hugo.

Each constitution/constitution.<lang>.md becomes the home page of its
language: the header line that says the file is generated is dropped, the
title goes into the front matter, a list under the title gives the
revision, the length of the version for agents and, when the text marks
parts left out of that version, what the asterisk means, and the text is
wrapped in `<div class="constitution">` for the style sheet. An info button
after the length opens a popup that gives the lengths of both versions,
says how they were counted and links to the version for agents, which
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
# Language: the content files, the lines under the title and the text of
# the popup on the counts.
PAGES = {
    "en": {"file": "_index.md", "agent_file": "agent.md", "home": "/", "revision": "Revision",
           "length": "{chars:,} characters · {words:,} words",
           "tokens": "{tokens:,} tokens", "no_tokens": "tokens not counted",
           "legend": ("Parts marked {mark} are about how this constitution came about, is organized and is"
                      " amended, and are shown to people by default; {agent} leaves out their text by default."),
           "legend_agent": "the version compiled for agents",
           "info": "About these numbers",
           "summary": "The length under the title is that of the version for agents.",
           "columns": ("characters", "words", "tokens"), "agent_row": "For agents", "full_row": "Full, this page",
           "counted": "Tokens counted by Anthropic's token counting API (count_tokens) for {model}.",
           "not_counted": ("Tokens are counted by Anthropic's token counting API (count_tokens) for {model};"
                           " this build had no credential for it, or the call failed, so they were not counted."),
           "agent_note": ("The version for agents leaves out the preamble, keeps only a pointer to the full"
                          " version for the other parts marked *, and has no anchors or links."),
           "source": "Counted at revision {revision}{commit}.",
           "commit": ", commit {commit}",
           "agent_link": "See the version agents read",
           "back": "Back to the full version"},
    "zh": {"file": "_index.zh.md", "agent_file": "agent.zh.md", "home": "/zh/", "revision": "修订号",
           "length": "{chars:,} 字符",
           "tokens": "{tokens:,} token", "no_tokens": "token 数未计算",
           "legend": "标 {mark} 的部分讲本宪法的由来、编排和修改，默认只对人可见；{agent}默认不收录它们的正文。",
           "legend_agent": "给 agent 编译的版本",
           "info": "关于这些数字",
           "summary": "标题下的数字是 agent 版的长度。",
           "columns": ("字符", "token"), "agent_row": "agent 版", "full_row": "完整版 (本页)",
           "counted": "token 数由 Anthropic 的 token 计数接口 (count_tokens) 按 {model} 计算。",
           "not_counted": "token 数按 Anthropic 的 token 计数接口 (count_tokens) 以 {model} 计数；这次构建没有凭证，或者调用失败，未计算。",
           "agent_note": "agent 版不含序言，其余标 * 的部分只留一句「见完整版」，也不含锚点和链接。",
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
             '<circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" stroke-width="1"/>'
             '<circle cx="8" cy="5" r=".9" fill="currentColor"/>'
             '<path d="M8 7.25v4.25" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>')

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
    return line + " · " + (labels["no_tokens"] if tokens is None else labels["tokens"].format(tokens=tokens))


def numbers(text: str, labels: dict[str, str], tokens: int | None) -> list[str]:
    """The cells of the popup's table for text, one per column; a dash for
    tokens not counted, which the note under the table explains."""
    cells = [f"{len(text):,}", f"{len(text.split()):,}"][:len(labels["columns"]) - 1]
    return cells + ["—" if tokens is None else f"{tokens:,}"]


def page(markdown: str, agent_markdown: str, labels: dict[str, str], revision: str, commit: str | None,
         tokens: int | None, agent_tokens: int | None) -> str:
    """The Hugo page of one built constitution, given the version for agents
    whose length it shows and the counts of both. Under the title, a list:
    the revision, the length of the version for agents with the info
    button, and the legend of the asterisk when the text has one. The
    popup puts the lengths of both versions in a table, the notes on how
    they were counted under it, and the link to the version for agents
    last."""
    title, rest = split(markdown)
    text = counted(markdown)
    agent_length = length(counted(agent_markdown), labels, agent_tokens)
    counts = labels["counted" if tokens is not None and agent_tokens is not None else "not_counted"]
    source = labels["source"].format(revision=revision, commit=labels["commit"].format(commit=commit) if commit else "")
    # Spans with ARIA table roles, since a <table> cannot sit in the <span>
    # the popup is; a screen reader then names each number's column.
    rows = "".join(
        f'<span class="count-row" role="row"><span class="count-label" role="{head}">{label}</span>'
        + "".join(f'<span class="count-cell" role="{role}">{cell}</span>' for cell in cells) + "</span>"
        for head, role, label, cells in (
            ("columnheader", "columnheader", "", labels["columns"]),
            ("rowheader", "cell", labels["agent_row"], numbers(counted(agent_markdown), labels, agent_tokens)),
            ("rowheader", "cell", labels["full_row"], numbers(text, labels, tokens))))
    notes = "".join(f'<span class="count-note">{note}</span>'
                    for note in (counts.format(model=COUNT_MODEL), labels["agent_note"], source))
    popup = (f'<span class="count-summary">{labels["summary"]}</span><span class="count-table" role="table">{rows}</span>'
             f'{notes}<a class="count-link" href="{labels["home"]}agent/">{labels["agent_link"]}</a>')
    items = [f'<li class="number">{labels["revision"]} {revision}</li>',
             f'<li class="length">{agent_length}<span class="count-info"><button type="button"'
             f' class="count-button" aria-expanded="false" aria-controls="count-popup"'
             f' aria-label="{labels["info"]}">{INFO_ICON}</button>'
             f'<span class="count-popup" id="count-popup">{popup}</span></span></li>']
    if OMITTED_MARK in text:
        legend = labels["legend"].format(mark='<span class="mark">*</span>',
                                         agent=f'<a href="{labels["home"]}agent/">{labels["legend_agent"]}</a>')
        items.append(f'<li class="legend">{legend}</li>')
    revision_list = f'<ul class="revision">{"".join(items)}</ul>'
    body = HEADING_ANCHOR.sub(r"\1 \3 {#\2}", "\n".join(rest))
    return "\n".join([front(title), '<div class="constitution">', "", title, "", revision_list,
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
