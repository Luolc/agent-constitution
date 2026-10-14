"""Render the constitution, in full and for agents, in each language."""

import re

from constitution_languages import (
  LANGUAGES,
)
from constitution_sources import (
  Article,
  Item,
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


def render(lang: str, src: Sources, agent: bool = False) -> str:
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
  return _Renderer(lang, src, agent).text()


class _Renderer:
  """The rendering of one language and version, with its number tables."""

  def __init__(self, lang: str, src: Sources, agent: bool) -> None:
    """Compute the number and the citation label of every provision.

    Args:
      lang: the language.
      src: the sources.
      agent: whether to render the version for agents.
    """
    self.lang, self.src, self.agent = lang, src, agent
    self.lng = lng = LANGUAGES[lang]
    # short: a section's or an item's citation without its article or
    # section; parent: the article or section that short leaves out.
    labels: dict[str, str] = {}
    numbers: dict[str, str] = {}
    short: dict[str, str] = {}
    parent: dict[str, str] = {}
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
    self.labels, self.numbers, self.short, self.parent = (
      labels,
      numbers,
      short,
      parent,
    )
    # citation -> None, in order of first use: the link definitions
    self.cited: dict[str, None] = {}

  def anchor(self, ref: str) -> str:
    """Return the anchor of a provision, none in the version for agents.

    Args:
      ref: the provision's citation.

    Returns:
      The anchor, or "".
    """
    return "" if self.agent else f'<a id="{ref}"></a>'

  def note(self, month: str | None) -> str:
    """Return the note that a provision was repealed in a month.

    Args:
      month: the month, `YYYY-MM`.

    Returns:
      The note.
    """
    # Called only on a repealed provision, which check 1 gave a month.
    assert month is not None
    m = MONTH.fullmatch(month)
    assert m is not None
    return self.lng.repealed_note(int(m.group(1)), int(m.group(2)))

  def refs(self, text: str) -> str:
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
        ref in self.parent
        and self.parent[ref] == previous
        and between in self.lng.connectors
      )
      label = self.short[ref] if joined else self.labels[ref]
      if not self.agent:
        self.cited[ref] = None
        label = f"[{label}][{ref}]"
      out += [between, label]
      previous, end = self.parent.get(ref), m.end()
    return "".join(out) + text[end:]

  def indented(self, first: str, text: str, indent: str) -> list[str]:
    """Return a list line `first` + the text's first line, then the rest.

    The other lines are indented under it.

    Args:
      first: the start of the list line.
      text: the source text.
      indent: the indent of the other lines.

    Returns:
      The lines.
    """
    head, *tail = self.refs(text).split("\n")
    return [first + head, *(indent + t if t else t for t in tail)]

  def item_entry(self, i: Item) -> list[str]:
    """Return the lines of one list item.

    Args:
      i: the item.

    Returns:
      The lines.
    """
    lng = self.lng
    marker = self.anchor(i.ref) + lng.item_marker(*number_key(i.number))
    if i.status == "repealed":
      return [f"{ITEM}{marker.rstrip()} ({self.note(i.repealed)})"]
    if i.omitted and self.agent:
      return [f"{ITEM}{marker}{lng.omitted}"]
    if i.omitted:
      return self.indented(
        f"{ITEM}{marker.rstrip()}{OMITTED_MARK} ",
        i.content[self.lang],
        2 * INDENT,
      )
    return self.indented(f"{ITEM}{marker}", i.content[self.lang], 2 * INDENT)

  def section_entries(self, s: Section) -> list[list[str]]:
    """Return the entries of one section: the section, then its items.

    Args:
      s: the section.

    Returns:
      The lines of each entry.
    """
    number = f"{SECTION}{self.anchor(s.id)}**{self.numbers[s.id]}**"
    if s.status == "repealed":
      return [[f"{number} ({self.note(s.repealed)})"]]
    if s.omitted and self.agent:
      return [[f"{number} {self.lng.omitted}"]]
    mark = OMITTED_MARK if s.omitted else ""
    entries = [self.indented(f"{number}{mark} ", s.content[self.lang], INDENT)]
    for i in sorted(s.items, key=lambda i: number_key(i.number)):
      entries.append(self.item_entry(i))
    return entries

  def article(self, a: Article) -> str:
    """Return one article: its heading, intro and sections.

    Args:
      a: the article.

    Returns:
      The Markdown text.
    """
    lng, anchor, number = self.lng, self.anchor(a.id), self.numbers[a.id]
    if a.repealed:
      return lng.repealed_article_heading.format(
        anchor=anchor, number=number, note=self.note(a.repealed)
      )
    block = [
      lng.article_heading.format(
        anchor=anchor, number=number, title=self.refs(a.title[self.lang])
      )
    ]
    if a.intro is not None:
      block.append(self.refs(a.intro[self.lang]))
    own = sorted(
      (s for s in self.src.sections if s.article == a.id),
      key=lambda s: number_key(s.number),
    )
    body = []
    for s in own:
      for e in self.section_entries(s):
        # An entry of several paragraphs ends with a blank line, or a
        # list line right after it would join its last paragraph.
        body += e + [""] if "" in e else e
    if body and body[-1] == "":
      body.pop()
    if body:
      block.append("\n".join(body))
    return "\n\n".join(block)

  def front(self) -> str:
    """Return the front text, or the note in its place for agents.

    Returns:
      The Markdown text.
    """
    index, lng = self.src.index.data, self.lng
    if self.agent:
      active = index["status"] == "active"
      opening = lng.agent_active if active else lng.agent_draft
      return lng.agent_front.format(opening=opening)
    return FRONT_HEADING.sub(
      lambda m: m.group() + OMITTED_MARK, self.refs(index["front"][self.lang])
    )

  def header(self, front: str) -> str:
    """Return the header line, with the legend when a text is marked.

    Args:
      front: the rendered front text.

    Returns:
      The header.
    """
    index, lng = self.src.index.data, self.lng
    revision = index["revision"]
    if index["status"] == "draft":
      revision = lng.draft_revision.format(revision=revision)
    header = (lng.agent_header if self.agent else lng.header).format(
      revision=revision
    )
    marked = any(
      s.omitted or any(i.omitted for i in s.items) for s in self.src.sections
    )
    if not self.agent and (OMITTED_MARK in front or marked):
      header += lng.legend
    return header

  def text(self) -> str:
    """Return the whole text.

    Returns:
      The Markdown text.
    """
    index = self.src.index.data
    edition = self.lng.edition.format(edition=index["edition"])
    front = self.front()
    title = self.refs(index["title"][self.lang])
    parts = [self.header(front), f"# {title} ({edition})\n\n{front}"]
    for a in sorted(self.src.articles, key=lambda a: number_key(a.number)):
      parts.append(self.article(a))
    if self.cited:
      parts.append("\n".join(f"[{ref}]: #{ref}" for ref in self.cited))
    return "\n\n".join(parts) + "\n"
