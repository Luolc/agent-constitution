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
from collections.abc import Mapping
import http.client
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import tomllib
import urllib.error
import urllib.request

from build_constitution import OMITTED_MARK
import check_site_anchors
import git_env

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
COUNT_MODEL_NAME = "Claude Opus 5.5"  # COUNT_MODEL as the popup names it
COUNT_TIMEOUT = (
  10  # seconds per connect or read; a stalled API must not hold up the build
)
# Language: the content files, the lines under the title and the text of
# the popup on the counts.
PAGES = {
  "en": {
    "file": "_index.md",
    "agent_file": "agent.md",
    "home": "/",
    "revision": "Revision",
    "length": "{chars:,} characters · {words:,} words",
    "tokens": "{tokens:,} tokens",
    "no_tokens": "tokens not counted",
    "legend": (
      "{mark} Background for people: where this constitution comes from,"
      " how it is organized and how it is amended. {agent} omits these parts."
    ),
    "legend_agent": "The agent version",
    "info": "About these numbers",
    "summary": "The length under the title is that of the agent version.",
    "columns": ("characters", "words", "tokens"),
    "agent_row": "Agent version",
    "full_row": "Full version, this page",
    "agent_note": (
      "The agent version omits the background for people, the parts marked"
      " *: the preamble entirely, the others but for a pointer to the full"
      " version. It has no anchors or links either."
    ),
    "counted": "Tokens counted by Anthropic's token counting API for {model}.",
    "not_counted": (
      "Tokens are counted by Anthropic's token counting API for {model};"
      " this build had no credential for it, or the call failed, so they"
      " were not counted."
    ),
    "source": "Counted at revision {revision}{commit}.",
    "commit": ", commit {commit}",
    "agent_link": "See the version agents read",
    "back": "Back to the full version",
  },
  "zh": {
    "file": "_index.zh.md",
    "agent_file": "agent.zh.md",
    "home": "/zh/",
    "revision": "修订号",
    "length": "{chars:,} 字符",
    "tokens": "{tokens:,} token",
    "no_tokens": "token 数未计算",
    "legend": (
      "{mark} 写给人看的背景：本宪法的由来、结构和修改方式。"
      "{agent}不收录这些部分。"
    ),
    "legend_agent": "agent 版",
    "info": "关于这些数字",
    "summary": "标题下的数字是 agent 版的长度。",
    "columns": ("字符", "token"),
    "agent_row": "agent 版",
    "full_row": "完整版 (本页)",
    "agent_note": (
      "agent 版不收录写给人看的背景，即标 * 的部分："
      "序言整段不收录，其余只留一句「见完整版」。"
      "agent 版也不含锚点和链接。"
    ),
    "counted": "token 数由 Anthropic 的 token 计数接口按 {model} 计算。",
    "not_counted": (
      "token 数由 Anthropic 的 token 计数接口按 {model} 计算；"
      "这次构建没有凭证，或者调用失败，所以没有算。"
    ),
    "source": "按修订号 {revision}{commit} 计算。",
    "commit": "、commit {commit}",
    "agent_link": "查看 agent 读到的版本",
    "back": "回到完整版",
  },
}


def counted(markdown: str) -> str:
  """Return the text whose length the page gives.

  That is the full version of the built constitution without its header
  line, anchors and link definitions included.

  Args:
    markdown: A built constitution.

  Returns:
    The text after the header line.
  """
  return markdown.partition("\n")[2]


def credential(environ: Mapping[str, str]) -> dict[str, str] | None:
  """Return the header that authenticates the count.

  Args:
    environ: The environment to read the credential from.

  Returns:
    The header, or None without a credential.
  """
  if token := environ.get(TOKEN_VARIABLE):
    return {"authorization": f"Bearer {token}"}
  if key := environ.get(KEY_VARIABLE):
    return {"x-api-key": key}
  return None


def count_tokens(text: str, auth: dict[str, str] | None) -> int | None:
  """Count the tokens of text for COUNT_MODEL.

  A failure prints one line with the HTTP status or the exception's class,
  never the request or the exception, which may hold the credential.

  Args:
    text: The text to count.
    auth: The header from credential(), or None.

  Returns:
    The tokens, or None without a credential or when the call fails.
  """
  if not auth:
    return None
  request = urllib.request.Request(
    COUNT_URL,
    data=json.dumps(
      {"model": COUNT_MODEL, "messages": [{"role": "user", "content": text}]}
    ).encode(),
    headers={
      **auth,
      "anthropic-version": "2023-06-01",
      "content-type": "application/json",
    },
  )
  try:
    with urllib.request.urlopen(request, timeout=COUNT_TIMEOUT) as response:
      return int(json.load(response)["input_tokens"])
  except urllib.error.HTTPError as e:
    print(f"token count: HTTP {e.code}; tokens not counted", file=sys.stderr)
  except (
    OSError,
    http.client.HTTPException,
    ValueError,
    KeyError,
    TypeError,
  ) as e:
    print(
      f"token count: {type(e).__name__}; tokens not counted", file=sys.stderr
    )
  return None


INFO_ICON = (
  '<svg viewBox="0 0 16 16" width="1em" height="1em" aria-hidden="true">'
  '<circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor"'
  ' stroke-width="1"/>'
  '<circle cx="8" cy="5" r=".9" fill="currentColor"/>'
  '<path d="M8 7.25v4.25" stroke="currentColor" stroke-width="1.2"'
  ' stroke-linecap="round"/></svg>'
)

# A heading attribute that tells hugo-book's heading render hook to leave
# out its `#` link.
NO_ANCHOR = " {anchor=false}"


def split(markdown: str) -> tuple[str, list[str]]:
  """Split a built constitution into its `# ` title and the lines after it.

  Args:
    markdown: A built constitution.

  Returns:
    The title line and the lines after it.

  Raises:
    ValueError: When the text does not start as the build writes it.
  """
  lines = markdown.split("\n")
  if (
    len(lines) < 3
    or "tools/build_constitution.py" not in lines[0]
    or not lines[2].startswith("# ")
  ):
    raise ValueError(
      "does not start with the build's header line, a blank line and the"
      " `# ` title"
    )
  return lines[2], lines[3:]


def front(title: str, *extra: str) -> str:
  """Return the TOML front matter of a page.

  Args:
    title: The `# ` title line.
    *extra: Further lines of front matter.

  Returns:
    The front matter, between its `+++` lines.
  """
  return "\n".join(
    [
      "+++",
      f"title = {json.dumps(title[2:], ensure_ascii=False)}",
      *extra,
      "+++",
    ]
  )


def length(text: str, labels: dict[str, str], tokens: int | None) -> str:
  """Give the length of text: characters, in English words, and tokens.

  Args:
    text: The text.
    labels: The labels of the page's language.
    tokens: The tokens of the text, or None when not counted.

  Returns:
    The length line.
  """
  line = labels["length"].format(chars=len(text), words=len(text.split()))
  return (
    line
    + " · "
    + (
      labels["no_tokens"]
      if tokens is None
      else labels["tokens"].format(tokens=tokens)
    )
  )


def numbers(text: str, labels: dict[str, str], tokens: int | None) -> list[str]:
  """Return the cells of the popup's table for text, one per column.

  Tokens not counted get a dash, which the note under the table explains.

  Args:
    text: The text.
    labels: The labels of the page's language.
    tokens: The tokens of the text, or None when not counted.

  Returns:
    The cells.
  """
  cells = [f"{len(text):,}", f"{len(text.split()):,}"][
    : len(labels["columns"]) - 1
  ]
  return cells + ["—" if tokens is None else f"{tokens:,}"]


def page(
  markdown: str,
  agent_markdown: str,
  labels: dict[str, str],
  revision: str,
  commit: str | None,
  tokens: int | None,
  agent_tokens: int | None,
) -> str:
  """Return the Hugo page of one built constitution.

  Under the title, a list: the revision, the length of the version for
  agents with the info button, and the legend of the asterisk when the
  text has one. The popup puts the lengths of both versions in a table,
  under it a list of notes (what the version for agents leaves out, how
  the tokens were counted, at which revision), and the link to the
  version for agents last.

  Args:
    markdown: The built constitution, full version.
    agent_markdown: The version for agents, whose length the page shows.
    labels: The labels of the page's language.
    revision: The revision of the constitution.
    commit: The abbreviated commit of the build, or None.
    tokens: The tokens of the full version, or None when not counted.
    agent_tokens: The tokens of the version for agents, or None.

  Returns:
    The page, front matter included.
  """
  title, rest = split(markdown)
  text = counted(markdown)
  agent_length = length(counted(agent_markdown), labels, agent_tokens)
  counts = labels[
    "counted"
    if tokens is not None and agent_tokens is not None
    else "not_counted"
  ]
  source = labels["source"].format(
    revision=revision,
    commit=labels["commit"].format(commit=commit) if commit else "",
  )
  # Spans with ARIA table roles, since a <table> cannot sit in the <span>
  # the popup is; a screen reader then names each number's column.
  rows = "".join(
    '<span class="count-row" role="row">'
    f'<span class="count-label" role="{head}">{label}</span>'
    + "".join(
      f'<span class="count-cell" role="{role}">{cell}</span>' for cell in cells
    )
    + "</span>"
    for head, role, label, cells in (
      ("columnheader", "columnheader", "", labels["columns"]),
      (
        "rowheader",
        "cell",
        labels["agent_row"],
        numbers(counted(agent_markdown), labels, agent_tokens),
      ),
      ("rowheader", "cell", labels["full_row"], numbers(text, labels, tokens)),
    )
  )
  # A list as spans with list roles, for the same reason as the table.
  notes = "".join(
    f'<span class="count-note" role="listitem">{note}</span>'
    for note in (
      labels["agent_note"],
      counts.format(model=COUNT_MODEL_NAME),
      source,
    )
  )
  popup = (
    f'<span class="count-summary">{labels["summary"]}</span>'
    f'<span class="count-table" role="table">{rows}</span>'
    f'<span class="count-notes" role="list">{notes}</span>'
    f'<a class="count-link" href="{labels["home"]}agent/">'
    f"{labels['agent_link']}</a>"
  )
  items = [
    f'<li class="number">{labels["revision"]} {revision}</li>',
    f'<li class="length">{agent_length}<span class="count-info">'
    '<button type="button" class="count-button" aria-expanded="false"'
    ' aria-controls="count-popup"'
    f' aria-label="{labels["info"]}">{INFO_ICON}</button>'
    f'<span class="count-popup" id="count-popup">{popup}</span></span></li>',
  ]
  if OMITTED_MARK in text:
    legend = labels["legend"].format(
      mark='<sup class="omitted">*</sup>',
      agent=f'<a href="{labels["home"]}agent/">{labels["legend_agent"]}</a>',
    )
    items.append(f'<li class="legend">{legend}</li>')
  revision_list = f'<ul class="revision">{"".join(items)}</ul>'
  body = HEADING_ANCHOR.sub(r"\1 \3 {#\2}", "\n".join(rest))
  return "\n".join(
    [
      front(title),
      '<div class="constitution">',
      "",
      title,
      "",
      revision_list,
      body,
      "",
      "</div>",
      "",
    ]
  )


def agent_page(
  agent_markdown: str, labels: dict[str, str], revision: str
) -> str:
  """Return the Hugo page of the version for agents.

  Its text as agents read it, under a list like the home page's: the
  revision and a link back to the full version. Hidden from the theme's
  list of pages, since web/hugo.toml names it in the sidebar's menu; its
  headings get no anchor link from the theme.

  Args:
    agent_markdown: The version for agents.
    labels: The labels of the page's language.
    revision: The revision of the constitution.

  Returns:
    The page, front matter included.
  """
  title, rest = split(agent_markdown)
  revision_list = (
    '<ul class="revision">'
    f'<li class="number">{labels["revision"]} {revision}</li>'
    f'<li class="back"><a href="{labels["home"]}">{labels["back"]}</a></li>'
    "</ul>"
  )
  rest = [line + NO_ANCHOR if line.startswith("#") else line for line in rest]
  return "\n".join(
    [
      front(title, "bookHidden = true"),
      '<div class="constitution">',
      "",
      title + NO_ANCHOR,
      "",
      revision_list,
      *rest,
      "",
      "</div>",
      "",
    ]
  )


def short_commit() -> str | None:
  """Return the abbreviated commit the build runs on.

  Returns:
    The commit, or None outside git.
  """
  try:
    result = subprocess.run(
      ["git", "-C", str(ROOT), "rev-parse", "--short", "HEAD"],
      capture_output=True,
      text=True,
      env=git_env.clean(),
    )
  except OSError:
    return None
  return result.stdout.strip() if result.returncode == 0 else None


def main(argv: list[str] | None = None) -> int:
  """Build the site and check it.

  Args:
    argv: The command-line arguments, or None for sys.argv.

  Returns:
    The exit status: 0 on success, else non-zero.
  """
  parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
  parser.add_argument(
    "--out",
    type=Path,
    default=ROOT / "site",
    help="the output directory (default: site/)",
  )
  parser.add_argument(
    "--require-tokens",
    action="store_true",
    help="fail when the tokens of a page are not counted"
    " (the token-count workflow)",
  )
  args = parser.parse_args(argv)
  revision = tomllib.loads(
    (ROOT / "constitution/index.toml").read_text(encoding="utf-8")
  )["revision"]
  auth = credential(os.environ)
  if not auth:
    print(
      f"neither {TOKEN_VARIABLE} nor {KEY_VARIABLE} is set; tokens not counted",
      file=sys.stderr,
    )
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
          print(
            f"{source.relative_to(ROOT)}: tokens not counted;"
            " --require-tokens fails",
            file=sys.stderr,
          )
          return 1
        pages[version] = markdown, tokens
      (markdown, tokens), (agent_markdown, agent_tokens) = (
        pages[""],
        pages[".agent"],
      )
      text = page(
        markdown, agent_markdown, labels, revision, commit, tokens, agent_tokens
      )
      (Path(content) / labels["file"]).write_text(text, encoding="utf-8")
      (Path(content) / labels["agent_file"]).write_text(
        agent_page(agent_markdown, labels, revision), encoding="utf-8"
      )
    build = subprocess.run(
      [
        HUGO,
        "--source",
        str(ROOT / "web"),
        "--contentDir",
        content,
        "--destination",
        str(args.out.resolve()),
        "--cleanDestinationDir",
        "--panicOnWarning",
      ]
    )
  if build.returncode:
    return build.returncode
  return check_site_anchors.main([str(args.out)])


if __name__ == "__main__":
  sys.exit(main())
