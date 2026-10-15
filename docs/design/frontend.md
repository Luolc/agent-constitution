# Frontend

How the pages of this repo look and behave, and how a change to them is
made and reviewed: the design tokens, the icons, the motion, the
components and their states, and the screenshots a pull request
attaches. It covers both sites that show the constitution and the
rules: [the public site](static-site.md) and
[the local site](statute-command.md#the-local-site) of `statute site`.

Not built yet: this file is written ahead of the code. Today's pages
follow it only where the [Status](#status) section says so; the rest
replaces what they do now as each page is redone. Everything here is
independent of how the sites are built. How the tokens, icons and
components are written in the code (a style sheet, templates, a theme's
variables, a framework's configuration) is left out and will be added
once the sites' stack is chosen; until then a change follows the values
and behavior here in whatever code the site has.

## Contents

- [Design tokens](#design-tokens): what a token is and the one rule
  about values.
- [Color](#color): the two themes and the states.
- [Type](#type): fonts, sizes and weights, and the levels of the rules'
  tree.
- [Space, radius and shadow](#space-radius-and-shadow).
- [Icons](#icons): one library, Lucide, and what replaces today's icons.
- [Motion](#motion): durations, easing and reduced motion.
- [Components](#components): tags, the filter, the tree of rules, the
  one-of, popovers, the sidebar and buttons.
- [Screenshots and review](#screenshots-and-review): what a pull request
  that changes the pages attaches, what CI runs, what the reviewer
  checks.
- [Status](#status): what is built and what this replaces.
- [Open questions](#open-questions).

## Design tokens

A token is a named value: a color, a size, a duration. Each is given
below as a name, its value in the light theme and its value in the dark
theme, one value when the two are the same. The names are written
without a prefix (`text`, `space-4`), so that they become CSS custom
properties (`--text`) or the keys of a framework's theme alike.

The one rule about values: outside the place where the tokens are
defined, a page's styles name a token for every color, font size, font
weight, spacing (margin, padding, gap), corner radius, shadow, duration
and easing, and never write the value itself. The few values allowed as
written are `0`, `auto`, `100%`, `1px` and `2px` borders, the
breakpoints in a media query (a custom property cannot stand there),
and the offset that sets one glyph in its line (the asterisk after an
item's number), with a comment saying so. A value a page needs and no
token has is added here first, as a token.

Two more rules hold for every page in both themes: no state is shown by
color alone (the current page is also bold or has a bar, an error also
has an icon or a word), and every state below is drawn in both themes.

## Color

The palette is today's sage palette, kept; the tokens for the states and
the tags are new. Contrast is WCAG's ratio, computed on the values
below.

| Token | Light | Dark | Use |
| --- | --- | --- | --- |
| `bg-page` | `#dde6d5` | `#1e2520` | The page behind the text. |
| `bg-panel` | `#d1dcc8` | `#252d27` | The sidebar, popovers, the filter's panel. |
| `bg-hover` | `#c7d0bf` | `#343b35` | A row or a button on the page under the pointer. |
| `bg-panel-hover` | `#bcc7b4` | `#3a423b` | An item on a panel under the pointer. |
| `bg-active` | `#b2bdaa` | `#444c45` | An item while it is pressed. |
| `bg-selected` | `#c5c5b0` | `#3e3e33` | The current item: the sidebar's current page, a chosen option. |
| `bg-target` | `#e3d79c` | `#4a4a2a` | The element an anchor led to, for a moment. |
| `text` | `#23301f` | `#d3dccf` | Text and links in the text. |
| `text-muted` | `#4b5946` | `#9cab98` | Labels, secondary text, the table of contents. |
| `text-faint` | `#5a6754` | `#839080` | Notes under notes on the page: the legend under the title. Never on a panel. |
| `accent` | `#7d3a1e` | `#dba37f` | Links under the pointer, the current item's text, the focus ring, a primary button. |
| `accent-hover` | `#6b381e` | `#d9ae8f` | A primary button under the pointer. |
| `text-on-accent` | `#dde6d5` | `#1e2520` | Text on an `accent` background. |
| `border-subtle` | `#b5c4aa` | `#38453a` | Hairlines: the rules between parts, a link's underline, a tag's outline. |
| `border-strong` | `#9ca695` | `#5d655d` | Controls and boxes: inputs, the one-of's box, a tree level's bar. |
| `status-active` | `#245a20` | `#9fd39a` | The text of an Active tag. |
| `status-active-bg` | `#c3d2bc` | `#303d31` | Its background. |
| `status-trial` | `#66460a` | `#e2c07a` | The text of a Trial tag. |
| `status-trial-bg` | `#ccd0b9` | `#393b2d` | Its background. |
| `status-draft` | `#4b5946` | `#9cab98` | The text of a Draft tag. |
| `status-draft-bg` | `#c9d2c1` | `#303831` | Its background. |
| `quadrant-global-public` | `#acc8df` | `#274160` | The background of a source's quadrant tag (with `text`). |
| `quadrant-global-private` | `#e4d098` | `#5e491a` | |
| `quadrant-local-public` | `#a5dcb1` | `#1f5533` | |
| `quadrant-local-private` | `#cdbadf` | `#473360` | |

The derived colors are mixes of the palette, which is how they were
chosen and how they stay in tune if the palette changes: `bg-hover` is
12% `text` into `bg-page`, `bg-panel-hover` 12% `text` into `bg-panel`,
`bg-active` 18% `text` into `bg-panel`, `bg-selected` 14% `accent` into
`bg-panel`, `accent-hover` 20% `text` into `accent`, `border-strong`
35% `text` into `bg-page`, each status background 14% of its text color
into `bg-page`, and each quadrant 30%
of its hue (blue `#3b82f6`, amber `#f59e0b`, green `#22c55e`, purple
`#a855f7`) into `bg-page`.

The contrast the values must keep, in both themes:

- `text`, `text-muted` and `accent` on `bg-page` and on `bg-panel`, and
  `text-faint` on `bg-page`: at least 4.5. The lowest are `text-muted`
  on a panel (5.25 light, 5.86 dark) and `text-faint` on the page (4.67
  in both). `text-faint` on a panel is 4.22, under the floor, so on a
  panel the faintest text is `text-muted`.
- `text-on-accent` on `accent` and on `accent-hover`: at least 4.5 (6.55
  and 7.40 light, 7.13 and 7.76 dark).
- A status tag's text on its background: at least 4.5 (5.19, 5.43 and
  4.78 light; 6.67, 6.56 and 5.01 dark).
- `text` on a quadrant tag: at least 4.5 (the lowest is 6.10).
- A hover background against the surface it sits on: at least 1.2, so
  the change shows in light mode too (`bg-hover` on the page 1.24 light
  and 1.36 dark; `bg-panel-hover` on a panel 1.24 and 1.36).
- `accent` on `bg-selected`: at least 4.5 (4.80 light, 4.92 dark).

The focus ring is `2px` of `accent`, outside the element (offset `2px`),
shown on keyboard focus only (`:focus-visible`), the same on every
control.

## Type

The fonts stay: EB Garamond for English, EB Garamond with Noto Serif SC
for Chinese (SIL Open Font License 1.1), falling back to local serif
fonts, for the text and the controls alike. The size of the text grows
with the window between a floor and a ceiling, and the column is set in
em so that a line keeps its length, as [Static site](static-site.md)
gives them:

| Token | English | Chinese |
| --- | --- | --- |
| `font-size-base` | `clamp(1.125rem, 0.645vw + 0.686rem, 1.375rem)` | `clamp(1rem, 0.548vw + 0.583rem, 1.17rem)` |
| `measure` | `36em` | `40em` |
| `line-height-text` | `1.65` | `1.9` |
| `line-height-ui` | `1.4` | `1.5` |

Every other size is relative to the text's size, in em:

| Token | Size | Use |
| --- | --- | --- |
| `text-xs` | `0.75em` | Tags, counts on a button. |
| `text-sm` | `0.875em` | Controls, the sidebar's group names, notes, the table of contents. |
| `text-md` | `1em` | The text; a rule's line in the list. |
| `text-lg` | `1.25em` | A second-level heading; a domain of the second level. |
| `text-xl` | `1.5em` | A domain of the first level. |
| `text-2xl` | `1.75em` | A page's title. |

| Token | English | Chinese |
| --- | --- | --- |
| `weight-regular` | `450` | `500` |
| `weight-medium` | `600` | `650` |
| `weight-bold` | `750` | `800` |

Noto Serif SC is lighter than EB Garamond at the same weight, hence the
Chinese column. The levels of the rules' tree differ in size and weight
together, so that two neighbouring levels never look alike:

| Level | Size | Weight | Color |
| --- | --- | --- | --- |
| A domain of the first level | `text-xl` | `weight-bold` | `text` |
| A domain of the second level | `text-lg` | `weight-bold` | `text` |
| A domain of the third level or deeper; a one-of's title in its box | `text-md` | `weight-medium` | `text` |
| A rule | `text-md` | `weight-regular` | `text` |
| A node's label (`R13`) | the size of its line | the weight of its line | `text-muted`, tabular figures |

## Space, radius and shadow

Space inside a component and between components is in rem, on this
scale; space that belongs to the text (between paragraphs, around a
heading) stays in em, a multiple of the text size.

| Token | Value |
| --- | --- |
| `space-1` | `0.25rem` |
| `space-2` | `0.5rem` |
| `space-3` | `0.75rem` |
| `space-4` | `1rem` |
| `space-6` | `1.5rem` |
| `space-8` | `2rem` |
| `space-12` | `3rem` |

| Token | Value | Use |
| --- | --- | --- |
| `radius-sm` | `0.25rem` | Tags, inputs, buttons, a row's hover background. |
| `radius-md` | `0.5rem` | Popovers, panels, the one-of's box. |
| `radius-full` | `9999px` | Round things: a count badge, a status pill. |

| Token | Light | Dark | Use |
| --- | --- | --- | --- |
| `shadow-raised` | `0 1px 2px rgb(0 0 0 / 0.08)` | `0 1px 2px rgb(0 0 0 / 0.30)` | A control that stands on the page. |
| `shadow-popover` | `0 0.25rem 0.9rem rgb(0 0 0 / 0.14)` | `0 0.25rem 0.9rem rgb(0 0 0 / 0.45)` | Popovers and the filter's panel. |

The breakpoints are the sites' today, in rem: `22.5rem` (the narrowest
phones), `48rem` (a popover spans the column), `56rem` (the sidebar
goes behind a button) and `70rem` (the table of contents goes behind a
button).

## Icons

Every icon on the pages comes from one library, Lucide (ISC license),
as SVG drawn with `currentColor`, so it takes the color of its text in
both themes. No icon is drawn by hand, and no character stands in for
one (`ⓘ`, `▸`, `×`, `✓`, `+`). The files are copied into the repository
from Lucide's release, with its license and the version they were copied
from beside them; no package is installed for them. Where the files go
and how a page includes them is not decided (Not built yet, above).

The one exception is GitHub's mark on the link to the source: it is
GitHub's logo, not an icon, and stays GitHub's own drawing (Octicons'
`mark-github`, MIT license).

| Token | Size | Stroke | Use |
| --- | --- | --- | --- |
| `icon-sm` | `1em` | `2` | In a line of text or on a tag; grows with the text. |
| `icon-md` | `1.25rem` | `1.75` | On controls and in the sidebar. |

The stroke is in Lucide's units (a 24-unit box). An icon is set on the
middle of its line. An icon that only decorates is hidden from screen
readers; a button that shows only an icon has an accessible name.

What replaces today's icons:

| Today | Where | Lucide |
| --- | --- | --- |
| Octicons `law` | The constitution's group in the sidebar | `scale` |
| Octicons `checklist` | The rules' group | `list-checks` |
| Octicons `book` | The design's group | `book-open` |
| Octicons `device-desktop`, `sun`, `moon` | The color switch | `monitor`, `sun`, `moon` |
| A ring around an `i`, drawn by hand | The info button after the length | `info` |
| A ring holding a plus or a minus, drawn by hand | The button that opens the rules under a provision | `circle-plus`, `circle-minus` |
| The theme's chevron | A group that folds | `chevron-right`, `chevron-down` |
| A text button | The filter | `funnel` |
| None | Closing a popover, removing a condition | `x` |
| None | Adding a condition | `plus` |
| None | A rule under another rule | `corner-down-right` |
| None | An option of a one-of, the default one | `circle`, `circle-dot` |
| None | Enabled by the rule set | `check` |

## Motion

Motion is for three things only: a part that folds open or shut, a
change under the pointer or the keyboard's focus, and a popover that
opens or closes; and, once, the flash on an anchor's target. Nothing
moves on its own, on scroll or while the page loads.

| Token | Value | Reduced | Use |
| --- | --- | --- | --- |
| `duration-fast` | `120ms` | `0ms` | Hover and focus colors, a popover closing. |
| `duration-base` | `200ms` | `0ms` | Folding open or shut, a popover opening, a chevron turning. |
| `duration-flash` | `2000ms` | `0ms` | The anchor's target fading from `bg-target`. |
| `ease-out` | `cubic-bezier(0.2, 0, 0, 1)` | | Something appearing or opening. |
| `ease-in` | `cubic-bezier(0.4, 0, 1, 1)` | | Something closing. |

What moves is color, opacity and transform: a popover fades in and moves
`space-1` down to its place; a chevron turns a quarter. Under
`prefers-reduced-motion: reduce` every duration is the Reduced column's,
so the change is there at once, and the target, instead of flashing,
keeps a bar of `accent` along its start edge until the reader clicks
elsewhere.

## Components

Each component below is given as it looks at rest and in each state:
under the pointer (hover), pressed (active), with the keyboard's focus
(focus, the ring of [Color](#color)), current or chosen (selected), and
turned off (disabled), where it has these.

- **Tags.** Small, `text-xs`, `line-height-ui`, one line, never wrapped.
  Different facts take different forms, so the row of tags after a
  rule is no longer a row of the same boxes:
  - Status: a pill (`radius-full`, padding `0 space-2`) in the status's
    colors, `status-active`, `status-trial`, `status-draft`, with no
    border. Superseded and repealed: `text-muted`, an outline in
    `border-subtle`, no fill.
  - Whether the defaults enable a node: no box, a dot before a word. On
    by default: a filled dot of `accent`, the word in `text-muted`; off
    by default: a ring of `text-muted`, the word in `text-muted`. In the
    dense list the dot alone stands, with the word as its accessible
    name and on hover, and a legend above the list says what the dot
    means.
  - Enabled by the rule set: `check` (`icon-sm`) before the word, in
    `accent`; not enabled: the word alone in `text-muted`.
  - The quadrant of a source: a box with `radius-sm`, filled with the
    quadrant's color, in `text`.
  - In the list the tags stand in a column at the right edge of the
    row, aligned from one row to the next, not right after the title.
  - Tags do not react to the pointer; the row does.
- **The filter**, in the manner of Linear and Notion. Shut, it is one
  button: `funnel` (`icon-md`), the word Filter, and, when any condition
  is set, their number in a badge (`radius-full`, `accent` on
  `bg-selected`). Open, it is a popover under the button, about `28rem`
  wide (the column's width on a narrow screen), holding one row per
  condition: the field (status, defaults, use, domain, and the quadrant
  when there is more than one source, enabled when the site has a rule
  set), the operator (is, is not), the value (one or more values of the
  field, chosen from a list), and `x` to remove the row. The first row
  starts with Where; each later row starts with the connective, and or
  or, a menu on the second row that sets it for the whole filter (one
  connective for all rows, as Notion's simple filter). Under the rows,
  Add condition (`plus`), and Clear, which removes them all. A change
  applies at once and is written into the address, so a filtered list
  can be linked; there is no Apply button. A domain's condition takes
  in the domains under it. Escape or a click outside closes it. A site
  without scripts, as the local site is, shows the same button and
  popover with one row per field holding its values as check boxes,
  joined by and.
- **The tree of rules.** The level of a node shows by its size, weight
  and a bar ([Type](#type)), not by indentation: a domain of the first
  level is a heading with `space-8` above it; one of the second level
  is indented one step (`space-4`) with a bar of `border-strong`,
  `2px`, along its start edge, down the height of its rules; deeper
  domains get no further indent, their bar is `border-subtle`. The
  rules of a domain start at its heading's edge. A rule under another
  rule is not indented again: `corner-down-right` (`icon-sm`,
  `text-faint`) in front of its label shows it. Each rule is one row:
  the label in a column of fixed width, `text-muted`, the title cut to
  the row with an ellipsis, the tags at the right. The whole row is the
  link: hover fills it with `bg-hover`, `radius-sm`, and turns the
  title to `accent`; focus puts the ring around the row.
- **The one-of.** A box (`1px` of `border-strong`, `radius-md`, padding
  `space-3`) holding its options, with a label on its top border, One
  of / 「择一」 (`text-xs`, `weight-medium`, `text-muted`, on the
  surface's color so the border breaks around it), after its label and
  title. Each option is a row as in the tree, led by `circle`, and the
  default option by `circle-dot` in `accent` and its tag, Default option /
  「默认选项」, in `accent` with `check`, so the default shows by shape as
  well as color. A one-of whose options are off has its box in
  `border-subtle`.
- **Popovers** (the filter's panel, the info on the length, the rules
  under a provision when they open over the text): `bg-panel`, `1px`
  of `border-subtle`, `radius-md`, `shadow-popover`, padding `space-3
  space-4`, under the control that opens it. A popover with controls
  opens and closes on a click, Enter or Space, its control's
  `aria-expanded` saying which; focus goes into it and back to the
  control when it closes. A popover that only explains (the info on the
  length) may also open on hover. Escape closes either, and so does a
  click outside. It opens with `duration-base` and `ease-out`, closes
  with `duration-fast` and `ease-in`.
- **The sidebar.** `bg-panel`, with `1px` of `border-subtle` along its
  edge. A group's name: its icon (`icon-md`) and its name in `text-md`,
  `weight-bold`, `text`. A page: `text-md`, `weight-regular`, `text`,
  padding `space-1 space-2`, `radius-sm`. Hover: `bg-panel-hover`.
  Active: `bg-active`. The current page: `bg-selected`, its text in
  `accent` and `weight-medium`. A group that folds has its chevron at
  its end, `chevron-right` shut and `chevron-down` open, the turn taking
  `duration-base`.
- **Buttons and inputs.** A primary button (one per popover at most):
  `accent` with `text-on-accent`, `radius-sm`, padding `space-1
  space-3`; hover darkens it by 10% `text`, disabled is at half
  opacity. A quiet button (most of them: the color switch, the
  filter's, an icon alone): no fill, `text`, hover `bg-hover`, active
  `bg-active`. Inputs and menus: `bg-page`, `1px` of `border-strong`,
  `radius-sm`; focus the ring.

## Screenshots and review

- **Screenshots.** A pull request that changes how a page looks (its
  styles, templates, icons, components or their states) attaches
  screenshots of each page it changes, in the light and the dark theme,
  in English and in Chinese, four for each page. They are made by the
  repository's screenshot script, which [Static site](static-site.md)
  describes; until the script is merged they are made by hand with a
  headless browser. The description says which pages and which states
  they show.
- **CI.** The CI of each pull request builds the site, checks its links
  and runs the screenshot script, and keeps the screenshots as an
  artifact of the run ([Static site](static-site.md)).
- **Review.** The reviewer goes through this file against the change,
  point by point: no value written where a token belongs, icons from
  Lucide only, motion only where [Motion](#motion) allows it and gone
  under reduced motion, each state of each component it touches drawn
  as here, in both themes, and the screenshots there for all four.
  A change that needs a value or a component this file lacks adds it
  here in the same pull request.
- **Visual comparison.** Comparing screenshots against those of `main`
  to find changes nobody meant is not done; it may follow once the
  screenshots are made in CI.

## Status

Built today: the palette (`bg-page`, `bg-panel`, `text`, `text-muted`,
`text-faint`, `accent`, `border-subtle`, `bg-target`, under other names),
the fonts, the text size and the column, and the breakpoints. Not built
yet, and what each replaces:

- The tokens for the states, the tags, the type scale, space, radii,
  shadows and motion replace the values written in the style sheets
  today (the public site's and the local site's).
- The tags replace the boxes of the same look for status, defaults,
  enabled and quadrant.
- The filter replaces the panel of check boxes with its Apply button.
- The tree replaces the indentation by depth, a rule under a rule
  included.
- The list shows a one-of as a box today
  ([Static site](static-site.md)); the label on its border replaces the
  prefix "One of:" (「择一：」) before its title, and the circles before
  its options and the tag of [Components](#components) replace the
  plain tag that marks its default option.
- The icons replace those of the table in [Icons](#icons).
- The screenshot script, and its run in CI, are not built yet.
- How all of this is written in the code waits for the choice of the
  sites' stack.

## Open questions

- How the two sites share one definition of the tokens and one copy of
  the icons, when one is built by a site generator and the other by
  `statute`: decided with the stack.
