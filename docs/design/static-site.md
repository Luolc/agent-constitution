# Static site

The built constitution is also published as static web pages, English at
`site/index.html` and Chinese at `site/zh/index.html`, each with the
version for agents under `agent/` (`site/agent/index.html`,
`site/zh/agent/index.html`) and the full version again under
`editions/<edition>/` (`site/editions/2026-10/index.html`,
`site/zh/editions/2026-10/index.html`), and so is this design, under `design/`
(`site/design/index.html`, `site/zh/design/index.html`), and so are the
rules, under `rules/` (`site/rules/index.html`,
`site/zh/rules/index.html`). Hugo builds them with `hugo-0.167.0 -s
web`, once `uv run site-rules` has written the rules' data (Rules,
below). CI runs four steps on every pull request and push: `uv run
site-rules`, `uv run count-tokens` (the token count, below),
`hugo-0.167.0 --source web --cleanDestinationDir --panicOnWarning`, and
`uv run check-site-anchors site`, the link check, and then takes the
screenshots (Screenshots, below). The workflow
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
  `statute constitution build`. `web/hugo.toml` mounts them and the edition
  directory's `index.toml` from `constitution/` as assets, and the partial
  `web/layouts/_partials/constitution/source.html` finds each by name,
  failing the build unless exactly one file matches; Python writes no
  page, only the data of the rules' pages (Rules, below). The content
  adapter `web/content/_content.gotmpl` makes each into the home page of
  its language: the full version's header line is dropped (the version
  for agents has none), the title becomes the page's, each heading's anchor (`## <a
  id="general-provisions"></a>Article I. ...`) becomes the heading's id,
  so the table of contents links to it, and the page gets the lengths
  (below) as params. It fails the build when the full version does not
  start with the header line, a blank line and the `# ` title, or the
  version for agents with the `# ` title. The layout
  (`web/layouts/_partials/constitution/full.html`) sets the title, the list under it and the
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
- The edition's page: the full version is also the page
  `editions/<edition>/` of its language, an address that keeps this
  edition when a later one takes the home page; a link to an edition from
  outside the site goes there. The content adapter adds it with the home
  page's text, title and params, and its layout,
  `web/layouts/editions/page.html`, sets them as the home page's does,
  both through `web/layouts/_partials/constitution/full.html`. It has no
  entry in the sidebar, `bookHidden` hides it from the theme's own list,
  and the section `editions/` gets no page (`build` with `render` and
  `list` `never`). As the home page has the same text, the edition's page
  carries `<meta name="robots" content="noindex">`
  (`web/layouts/_partials/docs/inject/head.html`), so search engines
  index the text once, on the home page. The links in the rows of the
  rules under the provisions start at the site's root (`relLangURL`),
  which `relativeURLs` makes relative to each page, so they lead to the
  rules' pages from both.
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
- Rules: `uv run site-rules` (`tools/site`) reads the constitution and
  `rules/*.toml` through `statute export`
  ([The statute command](statute-command.md#export)), which it runs from
  this checkout with `cargo run`, and writes `web/data/rules.json` (not committed),
  which Hugo reads; the build fails without it, so CI and
  `pages.yml` run it before Hugo. For each language of `index.toml` it
  holds every node of the rule tree along the parent tree, a node's
  children in order of ID, each with its label, `R` and its ID (`R12`),
  made by `rule_label` in `site_rules.py`, the one place that makes it;
  the templates read the node's `label`. With `--source
  <name>:<quadrant>:<dir>`, once for each, it also reads the rules of other
  sources, `<dir>/rules/*.toml`, through the same export, where `R12`
  names this repo's rule 12, as `requires` writes it
  ([Rule sources](rule-sources.md)); their nodes follow this repo's,
  source by source in the order of the quadrants (global public, global
  private, local public, local private), each labeled `<name>:<id>`
  (`team:7`), and a node of another source that a node names, in a field
  or a citation (`{@rule:R12}`, `{@rule:team:7}`), links to its page.
  Each node holds its source and the source's quadrant; with more than
  one source, its groups start with its source. With `--enabled
  <statute.lock>`, once for each, each rule says whether one of those
  lock files lists it. The private overlay's review site uses both; the
  public site uses neither, so its pages show no quadrant and nothing
  enabled. For each provision it holds the
  rules that name it in `authority`, and for each article or section
  with a provision under it the rules that name it or a provision under
  it, each rule once, in order of source and ID. An
  `authority` that names a provision and one above it (an item and its
  section or article, a section and its article) counts only the
  narrower one, so a rule is listed under the most specific provisions it
  names, and its page lists only those; `statute validate rules` warns of
  such a pair ([Checkers](checkers.md#checkers), check 5). A
  superseded or repealed rule has a page and is in the list, but not
  under a provision. The content adapter
  `web/content/rules/_content.gotmpl` makes the list, `rules/`, and a
  page for each node, `rules/<id>/`, or `rules/<name>-<id>/` for a node
  of another source: the ID, not the `slug`, since the ID never changes.
  The list (`web/layouts/rules/section.html`) is a table under a head,
  one row a rule, along the tree: each rule's label, its title, cut to
  the row by the style sheet, and its marks, linked to its page: its
  status, whether the source's defaults enable it (on or off, as
  `statute export` reckons it), and its `uses`, each an icon or a dot in
  a place of its own ([Frontend](frontend.md#components)); then, with
  `--enabled`, whether it is enabled, and with more than one source, the
  quadrant of its source, each a small tag. A rule under another rule
  has an arrow before its title. A one-of stands in the table as a box
  ([No rule under a rule](rule-config.md#no-rule-under-a-rule)), "One
  of" on its border: its own row, then a row for each option, the
  default marked; a rule that is an option stands in that box only. An
  option that is a domain is a row of the box too, and its rules stay in
  the list under its group row, a level below its one-of's domain: no
  one-of heads a group. The filter treats the box as one entry, with the
  one-of's status, domain and switch and the `uses` of its rule
  options. Domains are not rows of their own: each rule carries a group
  row for its nearest domain, with that domain's label (`R13`) and the
  path of its domains (`Documentation › Design`), shown where the group
  changes from the rule shown before it and again at the top of each
  page, so that the list coming back to a domain from one under it shows
  the domain's row again; with more than one source, the path starts
  with the source, as its quadrant and name. The head and the group rows
  stick to the top of the window as it scrolls, a group row under the
  head until the next one covers it, with the style sheet alone. Above
  the list, a legend gives each mark a rule carries there and what it
  means, in the order of the marks' places: the statuses, on and off by
  default, the two `uses`, then the default option, and, when the site
  has them, enabled or not and the quadrant. Below it, the filter, a
  button that opens a panel of check boxes ([Frontend](frontend.md#components)), a
  group for each field: each status, on and off by default, each value
  of `uses` and each domain, and each quadrant when there is more than
  one source, each group with All and None. A rule is shown when, in
  every field, one of its values is checked: its status, whether the
  defaults enable it (a node whose answer depends on another source,
  whose place stays empty, counts as off), its `uses`
  (`["instructions"]` when its file leaves it out), its nearest domain
  and its quadrant; checking or unchecking a domain does the same to the
  domains under it. A change applies at once. A node whose `parent` is
  in another source, and a
  rule with `overrides`, has no domain on the site: it is at the top
  of the tree, with no group above it. When its parent is a domain of a
  source the site reads, the filter goes by that domain. Otherwise the
  filter shows it only while every domain is checked; without scripts
  it appears under the group row before it. This repo has no such node
  yet; the pull request that first adds one fixes the list, and the
  first-party source overrides nothing. By default the filter checks
  the statuses of `params.rulesStatuses` in `web/hugo.toml` and every
  other value; superseded, repealed and draft rules are in the list but
  not checked. Beside the filter, a menu sets the page size: 100 by
  default, `params.rulesPerPage`, or 25, 50 or all; the rules are
  filtered first and then split into pages. Hugo builds the pages of
  the default filter; each holds every rule, each rule with its group
  row, those not shown hidden.
  `web/assets/ts/rules-list.ts`, bundled by Hugo
  ([Frontend](frontend.md#in-the-code)) and inlined in `<head>`, takes the
  filter, the size and the page from the address, shows that page's
  rules and group rows, makes the links to the pages and the table of
  contents again, and writes each change back into the address. The
  address holds only the fields whose check boxes differ from the
  default, each as the values checked, keys of `[a-z0-9-]` joined by
  commas: `?status=draft,trial&uses=review&domain=14,15&per=50&page=2`,
  with `status=` for no status. An address of the earlier form, with
  `filter=1` and one parameter per box checked, a field without one
  having none checked
  (`?filter=1&status=draft&uses=review&default=on&domain=14`), is read
  as the same filter and rewritten in the new form. Without scripts the
  controls are hidden and Hugo's pages stay.
  The table of
  contents at the right lists the domains of the group rows on the page,
  each once, indented by its level. A
  rule's page (`web/layouts/rules/page.html`) gives its label and title,
  under it whether the defaults enable it, as in the list, with more
  than one source, the quadrant and name of its
  source ("Global private · team"), and with `--enabled` whether it is
  enabled, for a superseded rule the rules that supersede it, its statement,
  reason and verification, the provisions it cites (`authority`), linked
  to their anchors on the home page, its relations (its parent, or for a
  rule with `overrides` "Overrides" and the rule it overrides, its
  requirements and its conflicts, a conflict shown on both rules though
  written on one) and all its metadata: ID, revision, kind, status, slug,
  default level, the revisions of the constitution, the check with its
  command and tools, its `default`, its `uses`, the language of the
  original and the translations.
  A domain's or a one-of's page lists the nodes below it, a one-of's
  marking its default option, which its metadata names as its
  `default`, and a domain's metadata gives its `default`; a domain's
  page gives its applicability, when it has one, under its title, its
  citations linked as in a rule's text. A node named on
  a page shows its label and title; the table of contents lists the
  page's headings. In the texts,
  `{@constitution:...}` becomes a link to the provision's anchor, labeled
  as the full version labels it, in the language of the text, and
  `{@rule:N}` a link to the rule's page ("rule N", 「规定 N」). A
  placeholder, `{$language.docs}`, becomes its parameter's default as
  the outputs write it in the text's language ("English", 「英文」),
  marked as a value: a link to the parameter's entry on its source's
  page of parameters, the parameter's name as its title, shown on hover;
  in a code span the span becomes HTML around the link. A title stands
  inside a link in the list, in a one-of's box and in a node named on a
  page, so there the value is marked as a span with the parameter's name as its
  title, escaped, and not linked; on the rule's own page it is marked
  and linked as in the text, and elsewhere the value stands alone. A
  title's code spans show as code wherever it is shown in the page;
  Hugo renders the title in the list and on the rule's own page, and
  `site-rules` gives each node named in a rule's data and each rule
  under a provision its title as HTML as well (`titleHtml`: the text
  escaped once, each code span a `<code>`), since the home page's
  content adapter cannot render Markdown; the name of a rule's link under
  a provision is that HTML without its tags. A page's own title, in the
  browser's tab, is its title as text, without the backticks. Each
  source with parameters has
  that page, `rules/params/` for this repo's, `rules/<name>-params/` for
  another's (`web/layouts/_partials/rules/params.html`), linked above
  the list: each parameter under a heading of its title, anchored by its
  name, with its name, type, values for a `choice`, scope, default and
  the rules whose texts name it. A placeholder of another source's rule
  names that source's parameter, or else this repo's. A page
  shows a node's texts in its own language when the node is written in
  it or has a translation into it; otherwise it shows the original, under a line
  that names its language (`这条规定还没有中文译文，下面是原文
  (英文)。`), with the text's `lang` set. `statute export` fails on a
  translation of an older original, and `site-rules` with it, so every
  translation the pages get follows its current original. Under each provision of the home page that a listed rule cites,
  the home page's content adapter adds a row: a link to each rule that
  names it, its label in a pill; then, when rules name only a provision
  under it, a button of their count, "+N citing provisions under it" after
  rules that name it and "N rules cite provisions under it" in a row
  without them, with a ring holding a plus before it. A click, or Enter
  or Space, shows the links to those rules in the same row, before the
  button, and the ring holds a minus; a click again, or Escape from
  within the row, hides them, leaving the focus on the button; its
  `aria-expanded` says which, and its `aria-controls` names them
  (`web/layouts/_partials/rules/cites.html`, `web/assets/cites.js`). The
  row wraps as it needs. A link's accessible name is the rule's label
  and title. The row comes after an article's heading, or at the end of
  the first line of a section or an item, which the style sheet sets on
  a line of its own; without scripts every link is shown and the button
  hidden. The lengths are of the text
  without these rows. The rules' pages have the design's layout and
  style (`<div class="design rules">`).
- Length: the second item of the list gives the length of the version
  for agents, bare, which is what takes room in an agent's context: the
  generated file from its `# ` title on, in characters in both
  languages, words split on white space (ASCII white space, the only kind
  the files hold) in English, and tokens as counted for the model
  `params.tokenCount.model` of `web/hugo.toml` by Anthropic's count_tokens
  API (`POST /v1/messages/count_tokens`, standard library only). The content adapter counts
  the characters and words; `count-tokens`
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
  full version is the generated file from its `# ` title on, anchors and
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
  from Lucide (`params.BookIcon`, named as
  [Frontend](frontend.md#in-the-code) says), its pages as children. There
  are three groups: the constitution, the home page, with the scales
  (`scale`); the rules, their list, with a checklist (`list-checks`) and
  no pages below it, so the group itself is marked as the current page
  on the list and on each rule's page; and the design, its README's
  page, with an open book (`book-open`), its files below it as the
  content adapter lists them (Design, above). The design group folds
  (`collapse` in its menu params): it is closed on every page but the
  design's own, where it is open with the current page marked. It uses
  the theme's checkbox and label from its file tree, with Lucide's
  chevron; the chevron points right when closed and
  down when open, and the keyboard toggles it. The constitution group does
  not fold. Under the constitution are its editions, newest first:
  the content adapter adds each page's entry to the menu, named from
  `index.toml` as "2026-10 Edition" / 「2026-10 版」, followed
  by "(draft)" / 「(草案)」 while `status` is `draft`, and weighted by the
  edition's year and month, negated. Today that is the one edition, whose
  entry links the home page; the edition's page is not listed. A site of
  several editions, an older one on its own page alone and indexed there,
  is not built yet: the site reads one edition (Input, above). The
  version for agents is not listed, as
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
  setting by a Lucide icon (`monitor`, `sun`, `moon`), and its
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
- Style: the style sheets of `web/assets/styles/`, one for each part of
  the pages, joined after the theme's own and the design tokens
  ([Frontend](frontend.md#in-the-code)). `constitution.css` uses only
  the page's own markup and the tokens and moves to another generator as
  is; `site.css` maps the tokens onto hugo-book's variables.
  Centered article headings, bold section and item numbers (bold in the
  Markdown), English text ragged right and Chinese justified, a print
  style without link decoration. Every link in the text, under the title
  and in the popup is underlined like the citations: the text color with
  a light underline in the color of the rules, both turning to the accent
  color under the pointer. The table of contents and the top bar of a
  narrow screen are not underlined, but are under the pointer or the
  keyboard's focus; the sidebar's links are rows, which
  [Frontend](frontend.md#components) describes, the current page marked
  by its background, color and weight. The table of contents is set apart
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
  fonts, falling back to local serif fonts. The colors are the tokens of
  [Frontend](frontend.md#color), which gives their contrast.
- The few Chinese strings of the site itself (`web/i18n/zh.yaml`, which
  holds the labels and the popup text, and the language label and menu
  names in `hugo.toml`) are outside limae's reach.
- `site/` is not committed: a generated page under review would make every
  amendment touch it, while an amendment may change only the constitution
  sources and their generated Markdown. Hugo needs a download, so the build
  is not a commit hook, and the tests of the templates
  (`test_site.py`, which builds a made-up constitution and design) are
  skipped without it, except in CI; CI installs Hugo before the tests.
- Screenshots: `uv run site-screenshots [<dir>] [--out <dir>] [--page
  <path> ...]` (`tools/site`) serves a built site, `site/` by default,
  on a free local port and takes, with headless Google Chrome, one
  screenshot of each page path per color scheme, light and dark, and
  width, desktop (1280 by 900 pixels) and mobile (390 by 844): one
  Chrome run each, one after the other, each with a fresh profile and
  `--no-proxy-server`, the window's view only, not the whole page. The
  file of a page is named by its path, the scheme and the width
  (`zh-rules-79.dark.mobile.png`, `index.light.desktop.png` for `/`),
  in `site-screenshots/` by default, which is not committed. The script
  knows nothing of Hugo: its input is a directory and paths, a language
  is a path prefix (`/zh/`), and without `--page` it takes the
  constitution, the list of rules and two rules (rule 79, a one-of, and
  rule 132, with code in its title), in English and Chinese. Dark is
  Chrome's `--force-dark-mode`, which makes the preferred color scheme
  dark; it works because the theme switch (`web/assets/theme.js`)
  follows that scheme until the reader picks a theme, which a fresh
  profile never has. This is the one place the script depends on the
  site's theme, written in `SCHEMES` in the script. CI takes the
  screenshots after the link check, with the runner's Google Chrome
  (listed in the ubuntu-24.04 image, read 2026-10-14), and uploads them
  as the artifact `site-screenshots` of the run. They are for a person
  or a reviewer to look at; nothing compares them with earlier ones.
