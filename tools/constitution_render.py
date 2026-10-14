"""Render the constitution, in full and for agents, in each language."""

import re

from constitution_languages import (
  LANGUAGES,
)
from constitution_sources import (
  MARK,
  MONTH,
  number_key,
  Section,
  Sources,
)

# Rendering.

# List markers padded to the indent, so that a section's or an item's
# further lines start at its content column: CommonMark takes any indent
# up to that column, Python-Markdown wants four spaces.
INDENT = " " * 4
SECTION = "-   "
ITEM = INDENT + SECTION

OMITTED_MARK = '<sup class="omitted">*</sup>'
# A heading of the front text (the preamble's): the version for agents
# leaves the front text out, so in the full version it is marked as well.
FRONT_HEADING = re.compile(r"^#{2,6} .+$", re.M)


def render(lang: str, src: Sources, agent: bool = False) -> str:  # noqa: C901
  """Return the constitution in one language.

  The full version, or with `agent` the version for agents, without
  anchors and links, its citations as plain text, a note of its own in
  place of the front text, and each provision marked for maintainers as
  its number and a note.

  Args:
    lang: the language.
    src: the sources.
    agent: whether to render the version for agents.

  Returns:
    The Markdown text.
  """
  lng = LANGUAGES[lang]
  # short: a section's or an item's citation without its article or
  # section; parent: the article or section that short leaves out.
  labels, numbers, short, parent = {}, {}, {}, {}
  for a in src.articles:
    numbers[a.id] = lng.article_number(*number_key(a.number))
    labels[a.id] = lng.article_ref.format(number=numbers[a.id])
  for s in src.sections:
    numbers[s.id] = short[s.id] = lng.section_number(*number_key(s.number))
    labels[s.id] = lng.section_ref.format(
      article=labels[s.article], section=numbers[s.id]
    )
    parent[s.id] = s.article
    for i in s.items:
      short[i.ref] = lng.item_ref(*number_key(i.number))
      labels[i.ref] = labels[s.id] + short[i.ref]
      parent[i.ref] = s.id

  cited = {}  # citation -> None, in order of first use: the link definitions

  def anchor(ref: str) -> str:
    return "" if agent else f'<a id="{ref}"></a>'

  def note(month: str | None) -> str:
    # Called only on a repealed provision, which check 1 gave a month.
    assert month is not None
    m = MONTH.fullmatch(month)
    assert m is not None
    return lng.repealed_note(int(m.group(1)), int(m.group(2)))

  def refs(text: str) -> str:
    """Return the text with each citation rendered.

    Each citation becomes a link to its provision's anchor, its label as
    the link text, except that a section or item citation joined by a
    connector to the citation of another section of the same article,
    or another item of the same section, just before it renders short:
    `Art. IV §1 and §2`, `Art. IV §1(2) or (4)`. Reference links, not
    inline ones: limae reads the `)` of an inline link followed by a
    Chinese character as a parenthesis that needs a space after it.

    Args:
      text: the source text.

    Returns:
      The rendered text.
    """
    out, end, previous = [], 0, None
    for m in MARK.finditer(text):
      ref, between = m.group(2), text[end : m.start()]
      joined = (
        ref in parent and parent[ref] == previous and between in lng.connectors
      )
      label = short[ref] if joined else labels[ref]
      if not agent:
        cited[ref] = None
        label = f"[{label}][{ref}]"
      out += [between, label]
      previous, end = parent.get(ref), m.end()
    return "".join(out) + text[end:]

  def indented(first: str, text: str, indent: str) -> list[str]:
    """Return a list line `first` + the text's first line, then the rest.

    The other lines are indented under it.

    Args:
      first: the start of the list line.
      text: the source text.
      indent: the indent of the other lines.

    Returns:
      The lines.
    """
    head, *tail = refs(text).split("\n")
    return [first + head, *(indent + t if t else t for t in tail)]

  def section_entries(s: Section) -> list[list[str]]:
    number = f"{SECTION}{anchor(s.id)}**{numbers[s.id]}**"
    if s.omitted and agent:
      return [[f"{number} {lng.omitted}"]]
    mark = OMITTED_MARK if s.omitted else ""
    entries = [indented(f"{number}{mark} ", s.content[lang], INDENT)]
    for i in sorted(s.items, key=lambda i: number_key(i.number)):
      marker = anchor(i.ref) + lng.item_marker(*number_key(i.number))
      if i.status == "repealed":
        entries.append([f"{ITEM}{marker.rstrip()} ({note(i.repealed)})"])
      elif i.omitted and agent:
        entries.append([f"{ITEM}{marker}{lng.omitted}"])
      elif i.omitted:
        entries.append(
          indented(
            f"{ITEM}{marker.rstrip()}{OMITTED_MARK} ",
            i.content[lang],
            2 * INDENT,
          )
        )
      else:
        entries.append(indented(f"{ITEM}{marker}", i.content[lang], 2 * INDENT))
    return entries

  index = src.index.data
  edition = lng.edition.format(edition=index["edition"])
  revision = index["revision"]
  if index["status"] == "draft":
    revision = lng.draft_revision.format(revision=revision)
  header = (lng.agent_header if agent else lng.header).format(revision=revision)
  opening = lng.agent_active if index["status"] == "active" else lng.agent_draft
  front = (
    lng.agent_front.format(opening=opening)
    if agent
    else FRONT_HEADING.sub(
      lambda m: m.group() + OMITTED_MARK, refs(index["front"][lang])
    )
  )
  if not agent and (
    OMITTED_MARK in front
    or any(s.omitted or any(i.omitted for i in s.items) for s in src.sections)
  ):
    header += lng.legend
  parts = [header, f"# {refs(index['title'][lang])} ({edition})\n\n{front}"]
  for a in sorted(src.articles, key=lambda a: number_key(a.number)):
    if a.repealed:
      parts.append(
        lng.repealed_article_heading.format(
          anchor=anchor(a.id), number=numbers[a.id], note=note(a.repealed)
        )
      )
      continue
    block = [
      lng.article_heading.format(
        anchor=anchor(a.id), number=numbers[a.id], title=refs(a.title[lang])
      )
    ]
    if a.intro is not None:
      block.append(refs(a.intro[lang]))
    own = sorted(
      (s for s in src.sections if s.article == a.id),
      key=lambda s: number_key(s.number),
    )
    body = []
    for s in own:
      if s.status == "repealed":
        entries = [
          [f"{SECTION}{anchor(s.id)}**{numbers[s.id]}** ({note(s.repealed)})"]
        ]
      else:
        entries = section_entries(s)
      for e in entries:
        # An entry of several paragraphs ends with a blank line, or a
        # list line right after it would join its last paragraph.
        body += e + [""] if "" in e else e
    if body and body[-1] == "":
      body.pop()
    if body:
      block.append("\n".join(body))
    parts.append("\n\n".join(block))
  if cited:
    parts.append("\n".join(f"[{ref}]: #{ref}" for ref in cited))
  return "\n\n".join(parts) + "\n"
