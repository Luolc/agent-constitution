# The constitution

## Constitution

The sources of the constitution are in its edition directory,
`constitution/<YYYY-MM>/`, today `constitution/2026-10/`: `index.toml`
and one directory per article under `articles/`. The directory is named
after the edition and keeps its name when the draft is approved; the
status is recorded only in `index.toml`. The tools take the edition
directory to be the one directory under `constitution/` that holds an
`index.toml`, and fail when there is none or more than one, so its name is
written nowhere in them.

The constitution is compiled by `statute constitution build`
([The command and the first version](statute-command.md#the-command-and-the-first-version)),
run here as `cargo run -p statute -- constitution build`, from those
sources. It writes two files for each language in `index.toml` to
the edition directory's `generated/`, `constitution.<lang>.md` (the full
version) and `constitution.<lang>.agent.md` (the version for agents, see
The version for agents), committed and never edited by hand; `--check`
writes nothing and runs the checks under [Checkers](checkers.md#checkers).

```text
constitution/2026-10/
  index.toml
  articles/
    05-boundary-rule/
      article.toml
      01-must-ask.toml
      02-decide-yourself.toml
  generated/
    constitution.en.md  constitution.en.agent.md
    constitution.zh.md  constitution.zh.agent.md
```

`index.toml`:

- `status`: `draft` until a person first approves the text, then
  `active`.
- `edition`: the name of the edition, the month a principal approved it as
  in force, `YYYY-MM`; required. A draft names the edition it prepares,
  the month it is meant to be approved in; if it is approved in another
  month, the edition is renamed then. An edition is a whole version of the
  text, like a constitution named by the year it was adopted. Starting a
  new edition is itself an amendment and needs a principal's approval, and
  at most one edition starts in a month, so no two share a name; within an
  edition, numbers never change (see Numbering), and a new edition may
  renumber freely.
- `revision`: a string, the revision within the edition: a draft is
  `0.<n>`, with `<n>` counting from 1 in every draft (the first
  edition's and each later one's), the first approved text of an edition
  is `1`, each adopted amendment adds one, and a new edition starts again
  at `1`. The build manifest records it. Within an edition it rises
  exactly when the constitution text changes ([Checkers](checkers.md#checkers), check 7).
- `languages`: the language codes, the first being the default. Each
  must have a rendering entry in the build script (one table,
  `LANGUAGES`, holds everything that differs by language: the file header,
  the shapes of headings, numbers and citations, the repealed note). Adding
  a language means adding one entry there and one key to every text table.
- `[title]`: the title, one line per language; the build adds the
  edition after it (see Numbering).
- `[front]`: the Markdown between the title and the first article (draft
  notice, `## Preamble` and its text), per language. It may cite.

An article is a directory `articles/<NN>-<id>/`: its number,
two digits from `01`, a hyphen and its ID. In it, `article.toml` holds
`id` (equal to the directory name without the number), `[title]`, an
optional `[intro]` (the paragraph between the heading and the first
section) and, only when the whole article is repealed,
`repealed = "YYYY-MM"`.

Each section is a file `<NN>[a-z]-<id>.toml` in its article's directory:
its number, two digits from `01` with an optional lowercase letter, a
hyphen, its ID and `.toml`. The section belongs to the article whose
directory holds it; there is no field for it. The file holds `id` (equal
to the file name without the number and `.toml`), `status` (`draft`,
`active` or `repealed`), `repealed = "YYYY-MM"` exactly when the status is
`repealed`, an optional `audience = "maintainers"` (its only value) for a
provision on how the constitution itself is made, arranged and amended,
and `[content]`, the section's text per language: the first
line is the sentence after the section's number, further lines, nested
lists and blank lines follow as they are, and the build indents them
under the list item.

A section with list items has one `[[items]]` table per item, in the
section's file: `id`, `number` (`1`, `2`, `2a`, within the section),
`status`, `repealed` and `audience` as for a section, and
`[items.content]`, the item's text per language. The section's `[content]`
then holds only the lead-in; the build renders the items below it in number
order.

```toml
id = "must-ask"
status = "draft"

[content]
en = '''
**Must go through a principal** whose authority covers the item ({@explicit-yes}).
'''
zh = '''
**必须经过委托人**，即由权限覆盖该事项的委托人同意 ({@explicit-yes})。
'''

[[items]]
id = "capital"
number = "1"
status = "draft"

[items.content]
en = '''
Capital: allocating, spending, ...
'''
zh = '''
资本：资金和其他有价资源的配置……
'''
```

Article, section and item order come from the numbers alone; there is no
separate list. While `index.toml` is `draft`, every section, item and rule
is `draft` and nothing is repealed; once it is `active`, every section and
item is `active` or `repealed`.

## Numbering

Numbers are not derived from position. An article's number is the prefix
of its directory name and a section's the prefix of its file name, written
there and nowhere else; a list item's is its `number` field. A section or
item number is an integer with an optional lowercase letter suffix (`2`,
`2a`), the suffix marking one inserted later between `2` and `3`; articles
are only ever appended and take no suffix. Only after the first approved
revision does an insertion take a suffix (`<NN>a-<id>.toml`); while the
constitution is a draft, a new provision takes the next number in order
and the ones after it are renumbered. A name prefix is that number
with two digits (`02`, `02a`), so the files sort in reading order. Sort
order is by integer, then suffix: `2` < `2a` < `2b` < `3`. Within one
scope (all articles; the sections of one article; the items of one
section) numbers are unique; a draft need not be contiguous.

From the first approved revision on, numbers follow `stable-numbers`
within an edition; the check `--check --base <git-ref>` enforces it
against a base commit of the same edition (see [Checkers](checkers.md#checkers), check 7):
renaming a directory or file to a new number, or moving a section to
another article's directory, is an error. A repealed provision is
rendered as a one-line note in place of its text (table below). While the
constitution is a draft, or when a new edition starts, numbers may
change, directories and files are renamed with them, and the check only
says that it did not compare.

Rendering, with `must-ask` as the first section of the fourth article,
`capital` as its third item and `authority` as the article's ID; `<a id="must-ask"></a>`
is written `⟨must-ask⟩` in the table:

| Item | en | zh |
| --- | --- | --- |
| Title | `# Agent Constitution (2026-10 Edition)`, the `edition` | `# Agent 宪法 (2026-10 版)` |
| Article heading | `## ⟨authority⟩Article IV. <Title>` | `## ⟨authority⟩第四条 <标题>` |
| Section | `-   ⟨must-ask⟩**§1** <content>` | `-   ⟨must-ask⟩**第一款** <content>` |
| List item | `    -   ⟨must-ask.capital⟩(**3**) <content>` | `    -   ⟨must-ask.capital⟩(**三**) <content>` |
| Citation of an article | `[Art. IV][authority]` | `[第四条][authority]` |
| Citation of a section | `[Art. IV §1][must-ask]` | `[第四条第一款][must-ask]` |
| Citation of a list item | `[Art. IV §1(3)][must-ask.capital]` | `[第四条第一款第三项][must-ask.capital]` |
| Suffix | `§1a`, `(3a)`, `Art. IV §1(3a)` | `第一款之一`, `(三之一)`, `第四条第一款第三项之一` (a is `之一`, b is `之二`) |
| Repealed section | `-   ⟨…⟩**§3** (repealed November 2026)` | `-   ⟨…⟩**第三款** (2026 年 11 月废止)` |
| Repealed list item | `    -   ⟨…⟩(**3**) (repealed November 2026)` | `    -   ⟨…⟩(**三**) (2026 年 11 月废止)` |
| Repealed article | `## ⟨…⟩Article IV (repealed November 2026)`, nothing below it | `## ⟨…⟩第四条 (2026 年 11 月废止)` |
| Adjacent item citations | `Art. IV §1(2) or (4)`, `Art. IV §1(1) to (7)`, `Art. IV §1(1), (2) and (4)` | `第四条第一款第二项或第四项`, `第四条第一款第一项至第七项`, `第四条第一款第一项、第二项和第四项` |
| Adjacent section citations | `Art. IV §1 and §2`, `Art. IV §1 to §3`, `Art. IV §1, §2 and §4` | `第四条第一款和第二款`, `第四条第一款至第三款`, `第四条第一款、第二款和第四款` |

A list marker is padded to four characters, and the further lines of a
section or an item, nested lists and paragraphs included, are indented
to its content column: four spaces for a section, eight for an item.
A section or an item whose text has a blank line in it is followed by a
blank line. Python-Markdown nests only what is indented four spaces and
joins a list line to the paragraph just above it; with this layout the
output renders alike there and in CommonMark (GitHub) as long as a
source text's own nested blocks are indented four spaces and a list line
after a paragraph has a blank line before it. A source text that breaks
either still renders wrong in Python-Markdown; the constitution has none
today.

An item citation renders short, as its item number alone (`(4)`, `(2a)`;
`第四项`, `第二项之一`), when the text just before it is exactly one
connector and the text before that is a citation of another item of the
same section. A section citation renders short the same way, as its
section number alone (`§2`, `§1a`; `第二款`, `第一款之一`), after a
connector and a citation of another section of the same article. The
connectors are ` or `, ` and `, `, ` and ` to ` in English and `或`,
`和`, `、` and `至` in Chinese; a range is written in the sources with
` to ` or `至` between its first and last citation. An item of another
section, a section of another article, a citation after a citation one
level up or down (an article and its section, a section and an item),
and a citation with any other text before it keep the full path. The
sources always write the full path.

Every article, section and item starts with an anchor named by its ID,
not its number: `<a id="<article-id>">`, `<a id="<section-id>">` and
`<a id="<section-id>.<item-id>">` ([IDs](source-formats.md#ids)). The anchors are the same in every
language and survive renumbering; article and section IDs are unique in
the whole constitution, so they do not collide. A citation is a link to
its provision's anchor, a short one too: `[Art. IV §1][must-ask] and
[§2][…]`, each number its own link. The links are reference links, with
one definition `[<ref>]: #<ref>` per cited provision at the end of the
file, in order of first citation: limae reads the `)` of an inline link
`[…](#…)` followed by a Chinese character as a parenthesis that needs a
space (limae 0.13.3, read 2026-10). Section numbers are bold; an item
number is bold inside its parentheses, because limae asks for a space
between `**` and a parenthesis.

Each output starts with a plain-text paragraph, in its own language:
generated file, do not edit it, edit the sources; and the `revision`,
followed by `(draft)` (`(草案)`) while `status` is `draft`, and the
language; in the full version, when a part has the
asterisk (a provision marked for maintainers, or the preamble's heading),
also what the asterisk means: the parts marked are background for people,
where the constitution comes from, how it is organized and how it is
amended, and the version for agents omits them. Output is UTF-8 with LF
line endings and a final newline, with no timestamp; the same sources give
byte-identical output.

## The version for agents

Agents read the constitution through the compiled instructions, where
every token counts; the preamble and the provisions on how the
constitution itself is made and amended matter to the people who maintain
it and to an agent drafting an amendment, who reads the full version. So
the build writes a second file per language, `constitution.<lang>.agent.md`,
which differs from the full version in this:

- The header line says it is the version for agents, without the text of
  the provisions written for human readers.
- `[front]` (the draft notice and the preamble) is left out, so in the
  full version each heading in it, `## Preamble`, has the asterisk after
  it. In its place
  is a note of the build script's own, one sentence per language in
  `LANGUAGES`: that this is the version for agents, while `index.toml` is
  `draft` that the constitution is a draft that may be used on a trial
  basis but is not final, once `active`
  that every agent must follow it; that the preamble and the text of the
  provisions written for human readers, on how the constitution is
  organized and amended, are left out, with only a pointer to the full
  version in their place; and that before changing the constitution or the
  tools that build it, an agent reads the full version, named by its path.
  It is not the text of the constitution (see [File structure](file-structure.md)).
- A section or item with `audience = "maintainers"` keeps its number and
  shows a note in place of its text: `-   **§2** (see the full text)`,
  `-   **第二款** (见完整版)`; a repealed one shows its repealed note as
  usual. In the full version the same provision keeps its text and has an
  asterisk after its number, `**§2**<sup class="omitted">*</sup>`, which
  the header line explains.
- There are no anchors and no link definitions: a citation is its label
  as plain text (`Art. VIII §6`, `第八条第六款`), short where the full
  version is short.
