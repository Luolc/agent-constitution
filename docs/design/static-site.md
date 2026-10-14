# Static site

The built constitution is also published as static web pages, English at
`site/index.html` and Chinese at `site/zh/index.html`, each with the
version for agents under `agent/` (`site/agent/index.html`,
`site/zh/agent/index.html`), and so is this design, under `design/`
(`site/design/index.html`, `site/zh/design/index.html`), and so are the
rules, under `rules/` (`site/rules/index.html`,
`site/zh/rules/index.html`). Hugo builds them with `hugo-0.167.0 -s
web`, once `uv run site-rules` has written the rules' data (Rules,
below). CI runs four steps on every pull request and push: `uv run
site-rules`, `uv run count-tokens` (the token count, below),
`hugo-0.167.0 --source web --cleanDestinationDir --panicOnWarning`, and
`uv run check-site-anchors site`, the link check. The workflow
`.github/workflows/pages.yml` publishes the site to GitHub Pages at
`https://agent-constitution.luolc.com/`, which it passes to Hugo as
`--baseURL`.

- Publishing: on each push to `main`, and on a manual run, the job
  `build` counts the tokens (below), builds and checks the site as CI
  does and uploads it; the job `deploy` publishes it, but only when the
  repository is public and its variable `PAGES_DEPLOY` is `true`, and is
  skipped otherwise. Publishing is a release (rule 32, `release-must-ask`):
  setting the variable is the person's yes to publish every later push
  to `main`, and deleting it stops that. The custom domain is set in the
  repository's Pages settings, with the source "GitHub Actions"; a
  workflow that publishes ignores a `CNAME` file, so there is none. In
  the published build, links between the pages stay relative
  (`relativeURLs`); the canonical link, the language alternates and
  `og:url` are absolute, on the domain. `web/hugo.toml` keeps `baseURL`
  `/`: with an absolute one, the theme adds `integrity` and `crossorigin`
  to the style sheet link, and a page opened as a file loses its styles.
- Input: the generated `constitution.<lang>.md` in the edition
  directory's `generated/` ([The constitution](constitution.md)), not the
  sources: the citations, numbers and anchors are rendered once, by
  `constitution-build`. `web/hugo.toml` mounts them and the edition
  directory's `index.toml` from `constitution/` as assets, and the partial
  `web/layouts/_partials/constitution/source.html` finds each by name,
  failing the build unless exactly one file matches; Python writes no
  page, only the data of the rules' pages (Rules, below). The content
  adapter `web/content/_content.gotmpl` makes each into the home page of
  its language: the header line is dropped,
  the title becomes the page's, each heading's anchor (`## <a
  id="general-provisions"></a>Article I. ...`) becomes the heading's id,
  so the table of contents links to it, and the page gets the lengths
  (below) as params. It fails the build when a file does not start with
  the header line, a blank line and the `# ` title. The layout
  (`web/layouts/home.html`) sets the title, the list under it and the
  text in `<div class="constitution">`, the root the style sheet hangs
  on. The title (`web/layouts/_partials/constitution/title.html`) has no
  `#` link from the theme and keeps its name and its parenthesis each on
  one line, so a narrow screen breaks it only before the parenthesis; its
  size is chosen so that it fits on one line in a desktop window 1400
  pixels wide. The revision from `index.toml` is followed by `(draft)`
  (`(草案)`) while its `status` is `draft`. Its words, in both languages,
  are in
  `web/i18n/`. Under
  the title is a list without bullets: the revision and the length
  (below), each centered under the title, and, when the text has an
  asterisk, its legend, starting at the column's left edge, smaller and
  fainter than the two lines above it: the asterisk,
  then that the parts marked are background for people (where the
  constitution comes from, how it is organized and how it is amended) and
  that the version for agents, linked, omits them. The asterisk is set small and bold in
  the accent color, not a superscript and without opening up the line,
  lowered until its middle sits a little above the middle of an item
  number's parentheses and its top well under the top of the number. The
  legend and the table of contents show it the same way.
- Version for agents: each generated `constitution.<lang>.agent.md`
  becomes the page `agent/` under the home page of its language, made by
  the same content adapter (layout `web/layouts/page.html`) but with its
  text as is: no anchors, no links, the provisions
  written for human readers only their numbers and a pointer to the full
  version. Its headings carry `{anchor=false}`, so the theme adds no `#`
  link to them. Under the title is a list like the home page's, both lines
  centered: the revision, and a link back to the full version.
  The sidebar, the language switch (to the other language's version for
  agents), the color switch and the table of contents are as on the home
  page, the table of contents linking to the IDs Hugo gives the headings.
- Design: the files of `docs/design/` become the pages under `design/`
  of each language, read where they are: `web/hugo.toml` mounts
  `docs/design/*.md` and `docs/design/zh/*.md` as assets, and the content
  adapter `web/content/design/_content.gotmpl` makes the pages, so no
  copy is kept. `README.md` is the page `design/` itself, each other file
  `design/<name>/`, in the order of the README's list of contents, whose
  link texts name the pages in the sidebar; the build fails when the list
  and the directory do not have the same files. Each file's `# ` title
  becomes the page's. The English pages read `docs/design/`, the Chinese
  ones the translation in `docs/design/zh/`: its first line, the SHA-256
  of the English file (the design translation check in
  [Checkers](checkers.md#checkers)), is dropped, and each heading's
  anchor, `## <a id="checks"></a>检查`, becomes the heading's id, the same
  as the English heading's. While a file has no translation, its Chinese
  page shows the English text, under a line saying that there is no
  translation yet (`本页还没有中文译文，下面是英文原文。`); the build
  never fails for a missing translation, and the Chinese sidebar names
  pages from the Chinese README, or the English one while it has none.
  The layout (`web/layouts/design/all.html`) sets the title and the text
  in `<div class="design">`, in the constitution's fonts, size and column
  and with its colors, but as a document: headings at the left, the
  theme's bullets, tables and code blocks. The table of contents lists
  the level-2 headings, as on the constitution's pages; the design has no
  deeper ones today.
- Links in the design: written for the files as GitHub shows them, they
  are rewritten for the site by the link render hook
  (`web/layouts/_markup/render-link.html`), and the sources stay as they
  are. A relative link is resolved against the directory of the file the
  page read, `docs/design/` or `docs/design/zh/`, so a translation writes
  a link to another design file as the English does (`rules.md#rules`)
  and a link out of `docs/design/` with one more `../`. A link to a design
  file, in either directory, goes to its page in the page's language; to
  a generated constitution,
  `constitution/<edition>/generated/constitution.<lang>.md`, to the home
  page of that language; to any
  other file of the repo, to the file on GitHub (`params.SourceRepo`,
  `/blob/main/`). The fragment is kept. A link to a design file without a
  page, or one that leads out of the repo, fails the build; a link to a
  file that does not exist is left to the design translation check. Links on the
  constitution's pages, and absolute ones and fragments alone everywhere,
  are left as written.
- Rules: `uv run site-rules` (`tools/site`) reads `rules/*.toml` with the
  compiler's reader and writes `web/data/rules.json` (not committed),
  which Hugo reads; the build fails without it, so CI and
  `pages.yml` run it before Hugo. For each language of `index.toml` it
  holds every node of the rule tree along the parent tree, a node's
  children in order of ID, each with its label, `R` and its ID (`R12`),
  made by `rule_label` in `site_rules.py`, the one place that makes it;
  the templates read the node's `label`. For each provision it holds the
  rules that name it in `authority`, and for each article the rules that
  name it or a provision under it, each rule once, in order of ID. An
  `authority` that names a provision and one above it (an item and its
  section or article, a section and its article) counts only the
  narrower one, so a rule is listed under the most specific provisions it
  names, and its page lists only those; `constitution-build` warns of
  such a pair ([Checkers](checkers.md#checkers), check 5). A superseded rule has a page but is in no
  list, not under a provision either. The content adapter
  `web/content/rules/_content.gotmpl` makes the list, `rules/`, and a
  page for each node, `rules/<id>/`: the ID, not the `slug`, since the ID
  never changes. The list (`web/layouts/rules/section.html`) shows the
  rules along the tree, 100 a page unless the reader chooses 25, 50 or
  all of them in the menu above it, each rule's statement only, as plain
  text cut to three lines by the style sheet, after its label and linked
  to its page; a rule under another rule is indented under it. Domains
  and one-ofs are not listed but head the rules below them, and again at
  the top of each page. Hugo builds pages of `params.rulesPerPage` rules
  (`web/hugo.toml`), 100; each holds every rule, those of the other pages
  hidden, and each rule after the first carries its groups' headings once
  more, shown only when it starts a page. `web/assets/rules-list.js`,
  inlined in `<head>`, takes the size and the page from the address
  (`?per=50&page=2`), where the menu puts them, shows that page's rules
  and headings, and makes the links to the pages and the table of
  contents again; without scripts the menu is hidden and Hugo's pages
  stay. The table of contents at the right lists the headings of the
  domains and one-ofs on the page, a one-of's under its domain's. A
  rule's page (`web/layouts/rules/page.html`) gives its label and title,
  for a superseded rule the rules that supersede it, its statement,
  reason and verification, the provisions it cites (`authority`), linked
  to their anchors on the home page, its relations (its parent, its
  requirements and its conflicts, a conflict shown on both rules though
  written on one) and all its metadata: ID, revision, kind, status, slug,
  default level, the revisions of the constitution, the check with its
  command and tools, the language of the original and the translations.
  A domain's or a one-of's page lists the nodes below it. A node named on
  a page shows its label and title; the table of contents lists the
  page's headings. In the texts,
  `{@constitution:...}` becomes a link to the provision's anchor, labeled
  as the full version labels it, in the language of the text, and
  `{@rule:N}` a link to the rule's page ("rule N", 「规定 N」). A page
  shows a node's texts in its own language when the node is written in
  it or has a translation into it whose hash in `[translations]` is that
  of the current original; otherwise it shows the original, under a line
  that names its language (`这条规定还没有中文译文，下面是原文
  (英文)。`), with the text's `lang` set. A translation of an older
  original is treated as no translation: it is not shown, and not listed
  among the translations. `constitution-build --check` fails on such a
  translation, so only a build from sources that fail that check meets
  one. Under each provision of the home page that a listed rule cites,
  the home page's content adapter adds a line: a button that shows the
  rules' labels as a row of boxes, with a ring holding a plus after them,
  and opens a list under it of each rule's label and title, linked to its
  page; a click, or Enter or Space, opens it and closes it again, the
  button's `aria-expanded` saying which
  (`web/layouts/_partials/rules/cites.html`, `web/assets/cites.js`). An
  article that rules cite through a provision under it also gets a
  button with their count, "N rules cite this article or a provision
  under it", which opens the list of them all. A provision lists only the
  rules that name it, not those that name one under it. The line comes
  after an article's heading, or at the end of the first line of a
  section or an item, which the style sheet sets on a line of its own;
  without scripts the lists are open. The lengths are of the text
  without these lines. The rules' pages have the design's layout and
  style (`<div class="design rules">`).
  `params.tokenCount.model` of `web/hugo.toml` by Anthropic's count_tokens
  API (`POST /v1/messages/count_tokens`, standard library only). The
  content adapter counts the characters and words; `count-tokens`
  counts the tokens and writes them to `web/data/lengths.json` (not
  committed), with the SHA-256 of each text it counted and the full SHA
  of the commit it ran on. The site shows a token count, and the commit, only
  when the SHA-256 is that of the text it builds, so a file left from
  another commit shows no stale number; without the file it says the
  tokens were not counted. An info
  button after it opens a popup, on hover or a click or tap; it is a
  disclosure button (`aria-expanded`), reached with the keyboard, and
  Escape closes it (`web/assets/count-info.js`, inlined in `<head>`). The
  popup hangs from the button with no gap, centered under the length and
  at least as wide as it, so the pointer reaches its link without leaving
  it; on a narrow screen it spans the column. The
  popup first says that the length it hangs under is that of the version
  for agents and gives the lengths of both versions in a small table
  (spans with ARIA table roles, so a screen reader names the columns; the
  full version is the generated file without its header line, anchors and
  link definitions included; a token count not made is a dash), the table
  as wide as the popup; under it, fainter than the table, a bulleted list
  (spans with ARIA list roles): what the version for agents omits, which
  API counted the tokens for which model or why they were not counted (the
  model named in words by `params.tokenCount.name` next to its ID), and
  the revision and the commit of the count, abbreviated to seven
  characters and linked to that commit in the source repository, opened
  in a new tab (left out without a count of this text, or outside git);
  last, on a line
  of its own, the link to the page of the version for agents. The text of
  the popup is the size of the legend under the title, the numbers of the
  table a little larger. Each page's two versions are counted
  separately, so a build with a credential makes four calls.
- Credential: only from the environment, never from a file in this repo,
  in one of two variables. `CONSTITUTION_TOKEN_COUNT_API_KEY` holds an API
  key, sent as `x-api-key`. `CONSTITUTION_TOKEN_COUNT_ACCESS_TOKEN` holds a
  short-lived access token that a workflow got through Anthropic's
  Workload Identity Federation (its OIDC token exchanged at
  `POST /v1/oauth/token`), sent as `Authorization: Bearer`; when both are
  set, the token is used, so a key left behind does not shadow the
  federation. The credential never reaches the output, the logs or the
  page; a failure prints one line with the HTTP status or the exception's
  class. Without a credential, or when the call fails or a connect or read
  stalls for 10 seconds, the page says the tokens were not counted and the
  build goes on; with `count-tokens --require-tokens` the count fails
  instead and writes no file. The
  job `build` of `.github/workflows/pages.yml` runs only on `main`
  (the federation rule accepts a run on any branch of this repository, not
  only `main`; the repository chooses to publish only `main`) and in no environment, which would
  change the token's subject: it asks GitHub for the run's
  OIDC token (audience `https://api.anthropic.com`), exchanges it for an
  access token with the IDs of the federation rule, organization, service
  account and workspace, which are repository secrets
  (`ANTHROPIC_FEDERATION_RULE_ID`, `ANTHROPIC_ORGANIZATION_ID`,
  `ANTHROPIC_SERVICE_ACCOUNT_ID`, `ANTHROPIC_WORKSPACE_ID`; identifiers,
  not credentials, but the logs of a public repository are open to anyone
  and the log masks a secret as `***`, where it prints a variable), counts
  the tokens with `--require-tokens`, builds and checks the site as CI
  does, and writes the length line of both pages
  into the job summary and into the log, which,
  unlike the summary, the API can read. A failed exchange or count fails
  the run, and nothing is published. The CI on pull requests gets no credential,
  and neither does a pull request from a fork. A local build that wants the
  count sets the key variable for that one command, reading the key from
  the maintainer's secret store. So the pages of one commit differ with
  and without a credential, in that line and the popup only.
- Generator: Hugo, a single binary pinned by version in the workflows
  and in `tools/site/tests/test_site.py`, which writes to `site/`
  (`publishDir`),
  with the theme hugo-book (MIT license) as a git submodule at
  `web/themes/hugo-book`, pinned to a commit. The theme gives the language
  switch, the light, dark and auto color themes, the sidebar's list of
  pages and the table of contents at the right. The sidebar lists the
  pages, not the articles, in groups named in `web/hugo.toml`'s menus:
  each group a top-level entry, its name in bold after an icon of its own
  from Octicons (`params.BookIcon`, a file in `web/assets/icons/`), its
  pages as children. There are three groups: the constitution, the home
  page, with the scales (`law`); the rules, their list, with a checklist
  (`checklist`) and no pages below it, so the group itself is marked as
  the current page on the list and on each rule's page; and the design,
  its README's page, with an open book (`book`), its files below it as the
  content adapter lists them (Design, above). The design group folds
  (`collapse` in its menu params): it is closed on every page but the
  design's own, where it is open with the current page marked. It uses
  the theme's checkbox, label
  and chevron from its file tree; the chevron points right when closed and
  down when open, and the keyboard toggles it. The constitution group does
  not fold. Under the constitution are its editions, newest first:
  the content adapter adds each page's entry to the menu, named from
  `index.toml` as "2026-10 Edition" / 「2026-10 版」, followed
  by "(draft)" / 「(草案)」 while `status` is `draft`, and weighted by the
  edition's year and month, negated. Today that is the one edition, the
  home page; a page per edition, once there are several, is not built
  yet. The version for agents is not listed, as
  it is not meant for people to read; the home page links it (the legend
  and the popup on the length), and the content adapter hides it from the
  theme's own list with `bookHidden`, as it hides the design's pages,
  which the menu lists, and the rules' pages. The table of contents
  lists the page's level-2 headings, the preamble and the articles
  (`markup.tableOfContents` in `web/hugo.toml`), under a heading of its
  own; on a narrow screen the theme hides it behind a button at the right
  of the top bar, which opens it above the text, as the button at the
  left opens the sidebar. It marks the article the reader is in
  (`aria-current`), set by `web/assets/toc-current.js`, inlined in
  `<head>`: the last heading above a line at 30% of the window's height,
  or the last one once the end of the text is in view, watched with
  `IntersectionObserver`, which every current browser has; nothing is
  read on scroll. CSS alone (`scroll-target-group` and `:target-current`)
  did this only in Chromium when read (2026-10-13). The
  site overrides the theme's sidebar (`layouts/_partials/docs/menu.html`)
  to put a link to the source repository, with GitHub's mark from
  Octicons (MIT license), between the site's title and the language
  switch, and adds, through the theme's inject partials, a switch for the
  color theme at the right end of the language switch's row, and the
  fonts. The color switch is one button that cycles through following the
  system, light and dark, as Docusaurus's does; it shows the current
  setting by an Octicons icon (`device-desktop`, `sun`, `moon`), and its
  title and accessible name give the setting and the next one, from
  `web/i18n`. The choice is kept in `localStorage` and set in `<head>`
  before the page is drawn. A submodule keeps the theme's files out of
  this repo's diffs; a Hugo module would need Go on every machine that
  builds.
- Raw HTML: Goldmark's `unsafe` renderer is on, since the anchors
  (`<a id="...">`) and the asterisks are raw HTML. The content is only this
  repo's own built constitution, design and rules, never outside input.
- Anchors: the ID of the article, section or item: `#general-provisions`,
  `#terms`, `#terms.principal`, the same in both languages and in the
  Markdown. A citation is a reference link to it. `:target` flashes the
  element an anchor starts.
- Style: `web/assets/styles/custom.css`, loaded by the theme after its own.
  Its first part uses only the page's own markup and moves to another
  generator as is; the last part maps it onto hugo-book's variables.
  Centered article headings, bold section and item numbers (bold in the
  Markdown), English text ragged right and Chinese justified, a print
  style without link decoration. Every link in the text, under the title
  and in the popup is underlined like the citations: the text color with
  a light underline in the color of the rules, both turning to the accent
  color under the pointer. The navigation (the sidebar, the table of
  contents and the top bar of a narrow screen) is not underlined, but is
  under the pointer or the keyboard's focus. The sidebar marks the
  current page in the accent color. The table of contents is set apart
  from the text, muted along a thin rule; its current article is bold, in
  the accent color, with a bar over the rule, so it does not rest on the
  color alone. The
  text size grows with the window
  between a floor and a ceiling (`clamp()`), and the column is set in em,
  so a line keeps about 90 characters in English (36em, measured on the
  text's full lines: EB Garamond's letters are narrower than `ch`; the
  usual 75 reads too narrow for this text) and 40 in Chinese where the
  window has room for it. Three columns would leave the text under
  about 65 characters a line in English below 70rem (1120px), so there
  the table of contents goes behind the theme's top-bar button, floating
  at the top right, which opens it over the text at the right edge; the
  sidebar stays down to hugo-book's breakpoint (56rem), below which the
  theme hides it too. The column is centered between the sidebar and the
  table of contents. Fonts: EB Garamond and Noto
  Serif SC (SIL Open Font License 1.1) from Google Fonts as variable
  fonts, falling back to local serif fonts. The colors are one palette
  block, light and dark; contrast of text on the page and the sidebar is
  above WCAG AA in both, the values are in the style sheet.
- The few Chinese strings of the site itself (`web/i18n/zh.yaml`, which
  holds the labels and the popup text, and the language label and menu
  names in `hugo.toml`) are outside limae's reach.
- `site/` is not committed: a generated page under review would make every
  amendment touch it, while an amendment may change only the constitution
  sources and their generated Markdown. Hugo needs a download, so the build
  is not a commit hook, and the tests of the templates
  (`test_site.py`, which builds a made-up constitution and design) are
  skipped without it, except in CI; CI installs Hugo before the tests.
