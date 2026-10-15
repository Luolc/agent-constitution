# Frontend

How the pages of this repo look and behave, and how a change to them is
made and reviewed: the design tokens, the icons, the motion, the
components and their states, and the screenshots a pull request
attaches. It covers both sites that show the constitution and the
rules: [the public site](static-site.md) and
[the local site](statute-command.md#the-local-site) of `statute site`.

Not built yet: parts of this file are written ahead of the code.
Today's pages follow it where the [Status](#status) section says so;
the rest replaces what they do now as each page is redone. How the
tokens, the icons and the scripts are written in the code is in
[In the code](#in-the-code).

## Contents

- [Design tokens](#design-tokens): what a token is and the one rule
  about values.
- [Color](#color): the two themes and the states.
- [Type](#type): fonts, sizes and weights, and the rows of the list of
  the rules.
- [Space, radius and shadow](#space-radius-and-shadow).
- [Icons](#icons): one library, Lucide, and what replaces today's icons.
- [Motion](#motion): durations, easing and reduced motion.
- [Interaction and accessibility](#interaction-and-accessibility):
  target sizes, hover on devices that have it, names and states for
  assistive technology, pages that work without scripts.
- [Components](#components): marks and tags, the legend, the filter,
  the view of the list, the table of rules, the one-of, popovers, the sidebar and buttons.
- [Not done](#not-done): what the pages never do.
- [Screenshots and review](#screenshots-and-review): what a pull request
  that changes the pages attaches, what CI runs, what the reviewer
  checks.
- [In the code](#in-the-code): where the tokens, the style sheets and
  the icons are, and how the scripts are built.
- [Status](#status): what is built and what this replaces.
- [Open questions](#open-questions).
- [Sources](#sources): the outside guidelines this file draws on.

## Design tokens

A token is a named value: a color, a size, a duration. Each is given
below as a name, its value in the light theme and its value in the dark
theme, one value when the two are the same. The names are written
without a prefix (`text`, `space-4`); in the style sheets each is a
custom property with the prefix `--t-` (`--t-text`,
[In the code](#in-the-code)).

The one rule about values: outside the place where the tokens are
defined, a page's styles name a token for every color, font size, font
weight, spacing (margin, padding, gap), corner radius, shadow, duration
and easing, and never write the value itself. The few values allowed as
written are `0`, `auto`, `100%`, `1px` and `2px` borders, the
breakpoints in a media query (a custom property cannot stand there),
the offset that sets one glyph in its line (the asterisk after an
item's number, with a comment saying so), the term in `vw` that makes a
space grow with the window between two tokens in a `clamp()`, and the
values of the print style, which has no theme (white paper, a size in
points). A value a page needs and no token has is added here first, as
a token.

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
| `bg-panel` | `#d1dcc8` | `#252d27` | The sidebar, popovers, the filter's menus. |
| `bg-hover` | `#c7d0bf` | `#343b35` | A row or a button on the page under the pointer. |
| `bg-panel-hover` | `#bcc7b4` | `#3a423b` | An item on a panel under the pointer. |
| `bg-active` | `#b2bdaa` | `#444c45` | An item while it is pressed. |
| `bg-selected` | `#c5c5b0` | `#3e3e33` | The current item: the sidebar's current group, a chosen option. |
| `bg-target` | `#e3d79c` | `#4a4a2a` | The element an anchor led to, for a moment. |
| `text` | `#23301f` | `#d3dccf` | Text and links in the text. |
| `text-muted` | `#4b5946` | `#9cab98` | Labels, secondary text, the sidebar's pages, the table of contents. |
| `text-faint` | `#5a6754` | `#839080` | Notes under notes on the page: the legend under the title. Never on a panel. |
| `accent` | `#7d3a1e` | `#dba37f` | Links under the pointer, the current item's text, the focus ring, a primary button. |
| `accent-hover` | `#6b381e` | `#d9ae8f` | A primary button under the pointer. |
| `text-on-accent` | `#dde6d5` | `#1e2520` | Text on an `accent` background. |
| `border-subtle` | `#b5c4aa` | `#38453a` | Hairlines: the rules between parts, a link's underline, a tag's outline. |
| `border-strong` | `#6d7968` | `#717970` | Controls and boxes: inputs, the one-of's box, the line under the head of the list of the rules. |
| `status-active` | `#245a20` | `#9fd39a` | The icon of Active in the list of the rules; the text of an Active tag. |
| `status-active-bg` | `#c3d2bc` | `#303d31` | Its background. |
| `status-trial` | `#66460a` | `#e2c07a` | The icon of Trial; the text of a Trial tag. |
| `status-trial-bg` | `#ccd0b9` | `#393b2d` | Its background. |
| `status-draft` | `#4b5946` | `#9cab98` | The icon of Draft; the text of a Draft tag. |
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
60% `text` into `bg-page` in the light theme and 46% in the dark one
(the least that keeps the contrast below), each status background 14% of its text color
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
- A status's icon in the list on the page and on `bg-hover`: at least
  4.5, above the 3 an icon needs (Active 6.39 and 5.16 light, 9.14 and
  6.72 dark; Trial 6.69 and 5.40, 8.99 and 6.61; Draft 5.81 and 4.69,
  6.48 and 4.77).
- `text` on a group row's band: at least 4.5 (on `bg-active` 7.10
  light, 6.30 dark; on `bg-panel-hover` 7.90 and 7.37).
- A hover background against the surface it sits on: at least 1.2, so
  the change shows in light mode too (`bg-hover` on the page 1.24 light
  and 1.36 dark; `bg-panel-hover` on a panel 1.24 and 1.36).
- `accent` on `bg-selected`: at least 4.5 (4.80 light, 4.92 dark).
- `border-strong`, the edge of a control (an unchecked box, an input, a
  menu), on `bg-page` and on `bg-panel`: at least 3, the floor of WCAG
  for the boundary of a control (3.57 and 3.22 light, 3.48 and 3.15
  dark).
- A state never lowers the contrast of the text it holds below 4.5.
  `text-muted` keeps it only on `bg-hover` (4.69 light, 4.77 dark); on
  `bg-panel-hover`, `bg-active` and `bg-selected` it falls to between
  3.67 and 4.48, so there a muted label or a tag's word turns to
  `text`. `accent` falls under 4.5 on `bg-active` (4.31 light, 4.04
  dark), so a pressed item's text is `text`.

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
| `font` | EB Garamond, then local serif fonts | EB Garamond, Noto Serif SC, then local serif fonts |
| `font-size-base` | `clamp(1.125rem, 0.645vw + 0.686rem, 1.375rem)` | `clamp(1rem, 0.548vw + 0.583rem, 1.17rem)` |
| `measure` | `36em` | `40em` |
| `line-height-text` | `1.65` | `1.9` |
| `line-height-ui` | `1.4` | `1.5` |

The text is never smaller than 16px: the floors above are 18px in
English and 16px in Chinese. A line of English holds about 90
characters, above the 45 to 75 most guides give; [Static
site](static-site.md) says why this text reads better so. Numbers that
are compared, in a column or from one row to the next, use tabular
figures, and in a table they are aligned to the end.

Every other size is relative to the text's size, in em:

| Token | Size | Use |
| --- | --- | --- |
| `text-xs` | `0.75em` | Tags, counts on a button. |
| `text-sm` | `0.875em` | Controls, the sidebar, notes, the table of contents. |
| `text-md` | `1em` | The text; every row of the list of the rules. |
| `text-lg` | `1.25em` | A second-level heading. |
| `text-xl` | `1.5em` | A first-level heading of the design's pages. |
| `text-2xl` | `1.75em` | A page's title. |

| Token | English | Chinese |
| --- | --- | --- |
| `weight-regular` | `450` | `500` |
| `weight-medium` | `600` | `650` |
| `weight-bold` | `750` | `800` |

Noto Serif SC is lighter than EB Garamond at the same weight, hence the
Chinese column. The rows of the list of the rules are all one size, so
that they line up as a table; a domain's row differs from a rule's by
its weight and its band ([Components](#components)):

| Row | Size | Weight | Color |
| --- | --- | --- | --- |
| A domain's group row, its label and its path | `text-md` | `weight-bold` | `text`, on `bg-active` at the first level and `bg-panel-hover` below it |
| A one-of's own row in its box | `text-md` | `weight-medium` | `text` |
| A rule's row | `text-md` | `weight-regular` | `text` |
| A rule's label (`R13`) | the size of its row | the weight of its row | `text-muted`, tabular figures, aligned to the end |
| A rule's statement, under its title | `text-sm` | `weight-regular` | `text-muted`; `text-md` in `text` when it stands alone, in place of the title |
| The head of the table | `text-sm` | `weight-medium` | `text-muted` |

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
| `width-popover` | `28rem` | The greatest width of a popover that holds controls, a menu of the filter. |
| `height-popover` | `20rem` | The greatest height of a long list in a popover, the domains in the filter, which scrolls. |
| `width-label` | `3.2em` | The column of the labels in the list of the rules, at least. |
| `width-mark` | `1.1em` | Each of the four places of a row's marks in that list. |
| `size-dot` | `0.6em` | The dot of on or off by default. |

| Token | Value | Use |
| --- | --- | --- |
| `radius-sm` | `0.25rem` | Tags, inputs, buttons, a row's hover background. |
| `radius-md` | `0.5rem` | Popovers, panels, the one-of's box. |
| `radius-full` | `9999px` | Round things: a count badge, the dot of the marks, a status pill. |

| Token | Light | Dark | Use |
| --- | --- | --- | --- |
| `shadow-raised` | `0 1px 1px rgb(0 0 0 / 0.06), 0 1px 3px rgb(0 0 0 / 0.08)` | `0 1px 1px rgb(0 0 0 / 0.25), 0 1px 3px rgb(0 0 0 / 0.30)` | A control that stands on the page. |
| `shadow-popover` | `0 1px 2px rgb(0 0 0 / 0.08), 0 0.25rem 0.9rem rgb(0 0 0 / 0.14)` | `0 1px 2px rgb(0 0 0 / 0.30), 0 0.25rem 0.9rem rgb(0 0 0 / 0.45)` | Popovers and the filter's menus. |

Each shadow has two layers, a close one that sets the edge and a wide
one that lifts it. A box that rests on the page has a border or a
shadow, not both; a popover has both, as it lies over text of the same
colors. A rounded box inside another has a radius no larger than the
outer one's.

What lies over other parts of a page stacks in layers, lowest first,
each a token for `z-index`: a popover lies over the head and the group
rows of the list of the rules that stick, and the top bar and the
drawers of a narrow screen over all that scrolls under them. A style
sheet writes no other `z-index`, but for a control raised one above
its own popover, as the info on the length's button is above the
bridge to its popover.

| Token | Value | Use |
| --- | --- | --- |
| `z-row` | `10` | A group row of the list of the rules, which sticks. |
| `z-head` | `20` | The head of that list, which sticks over the group rows. |
| `z-popover` | `30` | A popover: the filter's menus, the info on the length. |
| `z-bar` | `40` | The top bar of a narrow screen, and the button of the table of contents that floats at the top right between `56rem` and `70rem`. |
| `z-drawer` | `50` | The drawers of a narrow screen. |

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
from beside them; no package is installed for them. Where the files are
and how a page includes them is in [In the code](#in-the-code).

The one exception is GitHub's mark on the link to the source: it is
GitHub's logo, not an icon, and stays GitHub's own drawing (Octicons'
`mark-github`, MIT license).

| Token | Value | Use |
| --- | --- | --- |
| `icon-sm` | `1em` | In a line of text or on a tag; grows with the text. |
| `icon-md` | `1.25rem` | On controls and in the sidebar. |
| `icon-stroke` | `1.75` | Every icon, in Lucide's units (a 24-unit box). |

Every icon has the one stroke width and is drawn in outline, as Lucide
draws them; none is filled. An icon is set on the middle of its line,
between 16px and 24px. All icons are kept in one directory, each named
by its source and name, `lucide:funnel`, never written out in a page's
markup where it is used. An icon that only decorates is hidden from
screen readers (`aria-hidden`); a button that shows only an icon has a
name for them (`aria-label`).

What replaces today's icons:

| Today | Where | Lucide |
| --- | --- | --- |
| Octicons `law` | The constitution's group in the sidebar | `scale` |
| Octicons `checklist` | The rules' group | `list-checks` |
| Octicons `book` | The design's group | `book-open` |
| Octicons `device-desktop`, `sun`, `moon` | The color switch | `monitor`, `sun`, `moon` |
| A ring around an `i`, drawn by hand | The info button after the length | `info` |
| A ring holding a plus or a minus, drawn by hand | The button that shows more of the rules under a provision | `circle-plus`, `circle-minus` |
| The theme's chevron | A group that folds, the language switch | `chevron-right`, `chevron-down` |
| The theme's `translate` | The language switch, and a text not in the page's language | `languages` |
| The theme's `menu` and `toc` | The buttons of a narrow screen's top bar | `menu`, `table-of-contents` |
| A text button | The local site's filter | `funnel` |
| A status's word in a tag | A rule's status in the list and the legend: active, trial, draft, superseded, repealed | `circle-check`, `flask-conical`, `pencil`, `arrow-right-left`, `circle-slash` |
| None | A rule's uses in the list and the legend: instructions, review | `scroll-text`, `scan-eye` |
| None | Between the domains of a group row's path | `chevron-right` |
| None | Closing a popover | `x` |
| None | A rule under another rule | `corner-down-right` |
| None | An option of a one-of, the default one | `circle`, `circle-dot` |
| None | Enabled by the rule set | `check` |

## Motion

Motion is for three things only: a part that folds open or shut, a
change under the pointer or the keyboard's focus, and a popover that
opens or closes; and, once, the flash on an anchor's target. Nothing
moves on its own, on scroll or while the page loads. What is done often
does not move at all: a change to the filter, and anything done from the keyboard (a shortcut, Escape) take
effect at once.

| Token | Value | Use |
| --- | --- | --- |
| `duration-fast` | `120ms` | Hover and focus colors, a popover or a part closing. |
| `duration-base` | `200ms` | Folding open, a popover opening, a chevron turning. |
| `duration-flash` | `2000ms` | The anchor's target fading from `bg-target`. |
| `delay-tip` | `300ms` | A tip, or a popover that only explains, waiting under the pointer before it opens. |
| `ease-out` | `cubic-bezier(0.23, 1, 0.32, 1)` | Every motion, opening and closing alike. |

Every motion the reader starts lasts at most 300 ms, the flash aside,
which the reader does not wait for. A closing is shorter than the
opening it undoes (`duration-fast` against `duration-base`), and both
use `ease-out`: no motion of the interface starts slowly. What moves is
opacity and transform, and color for the states; a transition names the
properties it moves, never `all`. A popover enters from `scale(0.95)`
and `opacity: 0`, never from `scale(0)`, its `transform-origin` at the
control that opened it; a chevron turns a quarter.

Under `prefers-reduced-motion: reduce` nothing moves or changes size:
the scale and the turn are dropped, and a popover or a part appears in
place. What shows a change of state stays, as a fade: a popover still
fades in and out, colors still change over `duration-fast`, and the
target still fades from `bg-target`.

## Interaction and accessibility

- **Targets.** Everything that can be clicked is at least 24px by 24px,
  its padding counted, and at least 44px by 44px where the pointer is
  coarse (`pointer: coarse`, a touch screen), the token `target`; a
  small icon gets the size from its padding, not from a larger drawing.
  A link within a line of text is exempt: it is as tall as the line.
- **Hover.** Hover styles apply only where there is a fine pointer that
  can hover (`@media (hover: hover) and (pointer: fine)`), so a tap on a
  touch screen does not leave one behind.
- **Names and states.** A control is the element that is one (a
  `button`, a link, a `details`, a `select`), and ARIA is added only for
  what that element does not say itself. A control that opens or folds
  something says whether it is open (`aria-expanded`) and what it opens
  (`aria-controls`). A control's name says what it acts on, not the
  words of a link beside it: the button that folds a group of the
  sidebar is named for the group's pages ("Pages of Design"), as the
  group's link already says its name. A popover with controls takes the
  focus when it opens and gives it back to its control when it closes; Escape closes
  it.
- **Tooltips.** A tip shown on hover appears after `delay-tip`; once one
  is shown, the next one under the pointer appears at once. A tip never
  holds what the page says nowhere else: what it says is also the
  element's accessible name or in a legend.
- **State in the address.** The filter of the list and what its rows
  show are in the address, so a list can be linked, and back and
  forward return to the list as it was.
- **Without scripts.** The content is visible by default: a page whose
  scripts fail or never run still shows all its text, with the parts
  that scripts fold open left open.
- **Empty and loading.** A list that the filter leaves empty says so and
  offers the next step, Reset filter. Should a page ever load something
  while the reader waits, its indicator appears only after about 150 to
  300 ms and then stays at least 300 to 500 ms, so it does not flicker;
  the sites load nothing that way today.

| Token | Value | Use |
| --- | --- | --- |
| `target` | `24px`; `44px` under `pointer: coarse` | The least width and height of what can be clicked. |

## Components

Each component below is given as it looks at rest and in each state:
under the pointer (hover), pressed (active), with the keyboard's focus
(focus, the ring of [Color](#color)), current or chosen (selected), and
turned off (disabled), where it has these.

- **Marks.** In the list of the rules, a row's marks stand at its
  end in four places of `width-mark` each, about four characters in
  all, in this order: the status, whether the defaults enable the node,
  instructions and review, its `uses`. A place without its mark stays
  empty, so that the marks line up from one row to the next; the column
  has no head, the legend says what each place holds. Each mark is an
  icon (`icon-sm`) or a dot, named for assistive technology and on
  hover by its word (`role="img"`, `aria-label` and `title`):
  - Status: `circle-check` for Active in `status-active`,
    `flask-conical` for Trial in `status-trial`, `pencil` for Draft in
    `status-draft`, `arrow-right-left` for Superseded and
    `circle-slash` for Repealed in `text-muted`.
  - Whether the defaults enable a node: a dot of `size-dot`, filled
    with `accent` when they do, a ring (a `2px` border) of
    `text-muted` when they do not; the place stays empty when that depends on another source.
  - Uses: `scroll-text` for instructions and `scan-eye` for review, in
    `text-muted`.
  - Marks do not react to the pointer; the row does.
- **Tags.** Small, `text-xs`, `line-height-ui`, one line, never wrapped,
  for the facts a row's four places do not hold and on a rule's page:
  - Whether the defaults enable a node, on its page: the dot of the
    marks before the word, in `text-muted`.
  - Enabled by the rule set: `check` (`icon-sm`) before the word, in
    `accent`; not enabled: the word alone in `text-muted`.
  - The quadrant of a source: a box with `radius-sm`, filled with the
    quadrant's color, in `text`.
  - A node with a text not in the page's language, on its page: no
    box, `languages` (`icon-sm`) before the words Not translated /
    「无中文」, in `text-muted`. The text itself carries its own
    language's `lang`.
  - In the list, enabled and the quadrant, when the site has them,
    follow the four places of the marks, in columns of their own.
- **The legend**, above the list of the rules, as Ruff's list of rules
  has one: a box with `1px` of `border-subtle` and `radius-md`, `text-sm`,
  a row per kind of mark (a description list), its name at the start in
  a column of its own, `weight-medium` in `text-muted`, and its marks
  after it on one line, each drawn as the list draws it and followed by
  its word. The rows: Status, each status's icon and word, with no
  meaning written out; Default, on and off by default; Uses,
  instructions and review, each with a short meaning after its word in
  `weight-medium`; One of, what a one-of is, then the default option's
  `circle-dot` in `accent` and another option's `circle`; when the site
  has them, Rule set, enabled and not enabled, and Source, a quadrant;
  last, Row end, the four places of the marks and their order, in
  `text-muted`. On a narrow screen a row's marks wrap under each other,
  its name staying at its start.
- **The filter**, as far as Ruff's list of rules goes, no further: no
  conditions to combine. It is a row of menus, one per field: the
  status, the defaults (on, off), the use, the domain, and the quadrant
  when there is more than one source; the row wraps on a narrow screen.
  Each menu is a quiet button with `1px` of `border-strong`: the field's
  name, what it chooses, and `chevron-down` (`icon-sm`), turned while
  the menu is open. What it chooses is All, None or the number of values
  chosen, `text-xs` with tabular figures, in `text-muted` while the
  field is as the default filter has it and in a badge (`radius-full`,
  `accent` on `bg-selected`) while it differs. The button opens a
  popover under it, as wide as its content up to `width-popover`, moved
  toward the start as far as it must to stay within the filter's bar
  (the column's width on a narrow screen), so that it never leaves the
  screen, holding the field's name, All and None, quiet buttons, and
  the field's check boxes. The domains' boxes are indented by their
  depth; their list, which is long, scrolls inside the popover, at most
  `height-popover` tall, under a search box that shows only the domains
  whose label or name holds what is typed; closing the menu clears the
  search. One menu is open at a time:
  opening one closes another. Within a field a rule needs one of the
  values checked; across fields it needs that in each, and a field with
  every value checked filters nothing. Checking or unchecking a domain
  does the same to the domains under it. At the end of the row, while a
  field differs from the default filter, Reset filter, in `accent`,
  with the number of such fields in a badge; it brings back the default
  filter, the list as its address without a filter shows it; to see
  every status, the reader checks All in its menu. A change applies at
  once and is written into the address, so a filtered list can be
  linked; there is no Apply button. Escape, a click outside or the focus
  leaving a menu closes it. On the local site, which must work
  without its script, the check boxes apply at once through the style
  sheet, with no primary button, a domain's box holding only the rules
  right in that domain; the script adds the rest.
- **The view of the list**, after the filter in its bar: what each
  row of the table of rules shows. A segmented control, the word Show /
  「显示」 in `text-sm` and `text-muted`, then three segments in one box
  (`1px` of `border-strong`, `radius-sm`), parted by `1px` of
  `border-subtle`: Title and statement / 「标题和要求」, the default;
  Title / 「标题」; Statement / 「要求」. They are radio buttons of one
  group, a `fieldset` named by its `legend`, so the arrow keys move
  between them and Tab enters and leaves the group; each segment is the
  button's label, `text-sm`, padding `0 space-3`, at least a target
  tall, the button itself not drawn. At rest `text`; under the pointer
  `bg-hover`; pressed `bg-active` with `text`; chosen `bg-selected` with
  its text in `accent` and `weight-medium`; focus the ring around the
  segment. The rows change at once. The choice is in the address as
  `view`, `title` or `statement`, left out for the default; arriving
  at an address without it, the reader's last choice, kept in the
  browser, applies and is written into the address, and back and
  forward return to the view of their address, the default where it has
  none. Without scripts the style sheet still applies the choice; the
  address does not. On a narrow screen the control takes a row of its
  own, its segments sharing it.
- **The table of rules**, as ESLint's list of rules is a table: a
  head, then one row a rule, three columns, the label (`width-label`,
  or the widest label where that is wider, `text-muted`, aligned to the end), the title, cut to the row with an
  ellipsis, and the marks; the head names the first two, `text-sm`,
  `weight-medium`, `text-muted`, over a line of `border-strong`. Every
  row is at least as tall as a target, with a hairline of
  `border-subtle` under it, and a row that shows only its title is the
  same height as the others on a wide screen. A
  rule under another rule is not indented: `corner-down-right`
  (`icon-sm`, `text-faint`) in front of its title shows it. The whole
  row is the link: hover fills it with `bg-hover` and turns the title
  to `accent`, active fills it with `bg-active` and turns the title,
  the label and the statement to `text`; focus puts the ring around the
  row. Under its title a rule's row shows its statement, as the view of
  the list says: wrapped, never cut, lined up with the title's text,
  after the icon before it; the row is then as tall as it needs, its
  label, its icon and its marks on the title's first line. With the
  statement alone, it takes the title's place, as large and dark as a
  title, and turns to `accent` under the pointer; a one-of's own row,
  which has no statement, keeps its title, and an option's Default
  option stays on its first line. A domain is not a row of
  its own but a group row over its rules, a band across the table:
  `bg-active` for a domain of the first level, `bg-panel-hover` below
  it, the domain's label in the label column and the path of its
  domains after it, joined by `chevron-right` in `text-muted`
  (`Documentation › Design`), all in `text` and `weight-bold`; the path
  is the whole link to the domain's page, underlined under the pointer,
  as `accent` would fall under 4.5 on the band, and named for screen
  readers by the label and the domains joined by commas. A group row shows where
  the nearest domain changes from the rule shown before it and above
  the first rule shown, so the list coming back to a domain from one under
  it shows that domain's row again. The head sticks to the top of the
  window, under a narrow screen's top bar, and each group row sticks
  under the head until the next one covers it, so that the domain of
  the rows in view is always named; only the nearest domain's row
  sticks, as its path names the ones above it. On a narrow screen a
  title takes as many lines as it needs, the head and the group rows
  one, and a row's label, the icon before its title and its marks line
  up with the title's first line. The table is not text to read on, so
  its line has no measure: on the list's page, which has no table of
  contents, the table, the legend and the row of the filter fill the
  page beside the sidebar, with `space-4` on each side and no greatest
  width, and only the intro above them keeps the text's measure. The
  label's column and the marks' are as wide as their tokens make them,
  and the title's takes the rest; a title that still does not fit is
  cut with an ellipsis, and the whole title shows under the pointer, as
  the row link's `title`; the row's text holds it whole for screen
  readers.
- **The one-of.** A box (`1px` of `border-strong`, `radius-md`) around
  its own row and its options', their columns lined up with the
  table's, with a label on its top border, One of / 「择一」 (`text-xs`,
  `weight-medium`, `text-muted`, on the surface's color so the border
  breaks around it); the box is a group (`role="group"`) that the label
  names for screen readers (`aria-labelledby`), so that they announce
  its rows as One of. Its own row's title is `weight-medium`. Each option
  is a row as in the table, its title led by `circle` in `text-muted`,
  and the default option's by `circle-dot` in `accent` and followed by
  its tag, Default option / 「默认选项」, in `accent` with `check`, so the
  default shows by shape as well as color. A one-of whose options are
  off has its box in `border-subtle`.
- **The path of a node.** Two nodes under different parents may have
  one title (a domain Tiers under Credentials and another under
  Release, say), so a node is named by the path of the nodes above it
  where a title alone could be either:
  - On a node's page, above its title, a breadcrumb: a `nav` named
    Breadcrumb / 「所在位置」 holding an ordered list, each node above
    it, one-ofs included, outermost first, by its title alone and as a
    link, then the node itself, its title in `text`, not a link,
    marked `aria-current="page"`. `text-sm`, `line-height-ui`,
    `text-muted`, the items joined by `chevron-right` (`icon-sm`) and
    wrapping on a narrow screen; each link at least `target` tall,
    turning to `accent` under the pointer as the text's links do. A
    node with nothing above it has no breadcrumb. The local site's
    pages keep their line of the path, which starts at the index.
  - The title of a node's page, in the browser's tab and in a shared
    link (`og:title`): its label, then the titles of the path joined by
    ` › ` (`R137 Security › Credentials › Secret manager`). Past 70 characters the
    titles above its parent are left out from the farthest, an ellipsis
    in their place (`R22 … › Credentials › Confirm a credential is in
    place by its presence only`); the parent's title and the node's own
    stay whole.
  - Where a node's title stands alone (in a page's relations, the rules
    of a parameter, the filter's domains, the local site's links and
    its menu of domains), a title that another node of the site has in
    that language is followed by the parent's title in parentheses, in
    `text-muted` (`Tiers (Credentials)`); a title no other node has
    stands alone. A breadcrumb or a path does not add it, as it names
    the parents.
- **Popovers** (the filter's menus, the info on the length): `bg-panel`,
  `1px` of `border-subtle`, `radius-md`, `shadow-popover`, padding
  `space-3 space-4`, under the control that opens it. A popover with controls
  opens and closes on a click, Enter or Space, its control's
  `aria-expanded` and `aria-controls` saying whether and what; focus
  goes into it and back to the control when it closes. A popover that
  only explains (the info on the length) may also open on hover, after
  the delay of a tooltip. Escape closes either, and so does a click
  outside. It opens over `duration-base` from `scale(0.95)` and
  transparent, and closes over `duration-fast`, both `ease-out`
  ([Motion](#motion)).
- **The rules under a provision.** One row under an article's heading
  or a provision's first line, `text-sm` in `text-muted`, wrapping as it
  needs. First a link to each rule that names the provision: its label
  in a pill (`text-xs`, `line-height-ui`, `radius-full`, padding
  `0 space-2`, `1px` of `border-subtle`, tabular figures, never
  wrapped), the link at least a target wide and tall. Under the pointer the pill
  fills with `bg-hover`, its border turns to `border-strong` and its
  label to `accent`; pressed, `bg-active`. When rules name only a
  provision under it, a quiet button of their count ends the row,
  `circle-plus` before it: it shows their links in the same row before
  it, with `circle-minus` while they are shown, and hides them again on
  a click or on Escape from within the row, the focus left on the
  button. The links appear and go at once, with no motion. Without
  scripts every link is shown and the button hidden.
- **The sidebar.** After the sidebars of Tailwind CSS's documentation
  ([Sources](#sources)), in this file's colors and fonts. `bg-panel`,
  with `1px` of `border-subtle` along its edge, its groups `space-4`
  apart. A group's name: its icon (`icon-md`) and its name in `text-md`,
  `weight-bold`, `text`. Its pages stand under it along a hairline, `1px`
  of `border-subtle` that starts under the middle of the icon. A page:
  `text-md`, `weight-regular`, `text-muted`, padding `space-1 space-2`,
  its text in line with the group's name, with a `2px` bar over the
  hairline, transparent at rest. A row, a group's or a page's, under the
  pointer: `bg-panel-hover`, `radius-sm` but on the hairline's side; a
  page also turns to `text`, its bar to `border-strong`. Active:
  `bg-active`. The current page: its bar and its text in `accent`, the
  text in `weight-medium`, with no fill. A group with no pages, when
  current, has no hairline to mark: it takes `bg-selected`, its text in
  `accent`. A group whose current page is listed under it marks that
  page alone. When the page loads, the sidebar scrolls its current page
  into view if it is out of it. A group that folds has a quiet button at
  the end of its row, its chevron `chevron-right` shut and
  `chevron-down` open, the turn taking `duration-base`; that button and
  the language switch say whether their list is shown
  (`aria-expanded`, `aria-controls`).
  - The table of contents, at the right, on every page but the list of
    the rules, whose group rows that stick name the domain in view:
    `text-sm`, under its heading in
    `text-muted`; its headings along a hairline as the sidebar's pages
    are, `text-muted`, a heading of a lower level indented by `space-4`
    with its bar still on the hairline. Under the pointer: `text`, the
    bar `border-strong`. The heading the reader is in: the bar and the
    text in `accent`, `weight-bold`. It follows the scroll, so its marks
    change at once, without a transition.
  - The drawers of a narrow screen. At `56rem` and under the sidebar,
    and at `70rem` and under the table of contents where a page has one,
    go behind a quiet button of the top bar, `menu` and `table-of-contents`, each showing
    `x` while its drawer is open and saying whether it is and which
    drawer it opens (`aria-expanded`, `aria-controls`). At `56rem` and
    under the top bar stays at the top of the window, `bg-page` with
    `1px` of `border-subtle` under it. A drawer opens under the top bar
    from its own edge, the sidebar from the start and the table of
    contents from the end: `bg-panel`, `1px` of `border-subtle` and
    `shadow-popover`, as it lies over the text. It slides in over
    `duration-base` and out over `duration-fast`, both `ease-out`, and
    the page behind it fades to a quarter of its opacity; under reduced
    motion it fades in and out in place. While a drawer is open the
    rest of the page is inert: the focus goes to the drawer's first
    link shown, and Escape, its button, a click outside it or a link followed
    in it closes it, the focus going back to the button when the
    keyboard or the button closed it. The sidebar does not repeat the
    site's title, which the top bar shows. Without scripts there are no
    drawers: the sidebar stands above the text and the table of
    contents below it, every list open: the buttons that fold the
    sidebar's groups are hidden, and the language switch's chevron is
    turned to open and its button does nothing on a click.
- **The local site's language switch.** The local site has two
  languages and no sidebar: each page has, at the end of its first
  line, a link to itself in the other language, a quiet button
  (padding `0 space-2`, `radius-sm`, `target` high) of `languages`
  (`icon-sm`) and that language's name in itself, 中文 or English,
  `text-sm` in `text-muted`; under the pointer `bg-hover` and `text`,
  active `bg-active`. It names its language (`hreflang`, `lang`).
- **Buttons and inputs.** A primary button (one per popover at most):
  `accent` with `text-on-accent`, `radius-sm`, padding
  `space-1 space-3`; hover `accent-hover`, disabled at half opacity. A
  quiet button (most of them: the color switch, the
  filter's, an icon alone): no fill, `text`, hover `bg-hover`, active
  `bg-active`. Inputs and menus: `bg-page`, `1px` of `border-strong`,
  `radius-sm`, its border `text-muted` under the pointer; focus the
  ring. No native control keeps the browser's own surface, in either
  theme: a menu (`select`) shows `chevron-down` at its end, and its
  list, where the browser lets a page draw it, is a popover whose
  options are rows, the chosen one in `bg-selected` with its text in
  `accent` and `weight-medium`; where the browser draws the list itself,
  only its colors follow the theme. Check boxes and radio buttons:
  `icon-sm` boxes of `bg-page` with `1px` of `border-strong`, the check
  box `radius-sm`, the radio button round; hover turns the border to
  `text-muted`; checked, the box fills with `accent` (`accent-hover`
  under the pointer), a check box's check `check` in `text-on-accent`,
  a radio button's dot a ring of `bg-page` `space-1` wide inside it;
  disabled at half opacity. A `summary`
  shows `chevron-right`, turned a quarter while it is open, in place of
  the browser's marker.

## Not done

- One accent color, `accent`; no second one for emphasis.
- No gradient text, no decorative blur or glass, no glow.
- No small upper-case label over a heading (an eyebrow), no numbers
  such as 01, 02, 03 on what has no order.
- No icon from a second library, no emoji or character as an icon, no
  filled icon among outlined ones.
- No `transition: all`, no entrance from `scale(0)`, no `ease-in` on
  the interface, no motion on load or on scroll.
- No hover style on a device that cannot hover.

## Screenshots and review

- **Screenshots.** A pull request that changes how a page looks (its
  styles, templates, icons, components or their states) attaches
  screenshots of each page it changes, in the light and the dark theme,
  in English and in Chinese, four for each page. They are made by the
  repository's screenshot script, which [Static site](static-site.md)
  describes. The description says which pages and which states they
  show.
- **CI.** The CI of each pull request builds the site, checks its links
  and runs the screenshot script, and keeps the screenshots as an
  artifact of the run ([Static site](static-site.md)).
- **Review.** The reviewer goes through this file against the change,
  point by point: no value written where a token belongs, icons from
  Lucide only, motion only where [Motion](#motion) allows it and only
  fades under reduced motion, the rules of
  [Interaction and accessibility](#interaction-and-accessibility) and
  [Not done](#not-done), each state of each component it touches drawn
  as here, in both themes, and the screenshots there for all four.
  A change that needs a value or a component this file lacks adds it
  here in the same pull request.
- **Visual comparison.** Comparing screenshots against those of `main`
  to find changes nobody meant is not done; it may follow once the
  screenshots are made in CI.

## In the code

- **Tokens.** One style sheet, `crates/statute/assets/tokens.css`,
  defines every token of this file, each as a CSS custom property of its
  name with the prefix `--t-` (`--t-text`, `--t-space-4`); the prefix
  keeps them apart from the theme's own properties, which include a
  `--space-1` of another value. A color is written once with both its
  values, `light-dark(<light>, <dark>)`, and follows the page's
  `color-scheme`: the system's setting, or the reader's choice where the
  page's root has a `data-theme`. The values that differ between English
  and Chinese are set again under `:root:lang(zh)`, and `target` under
  `pointer: coarse`. The file is in the crate so that `statute` can
  build it into the local site with `include_str!` and still be
  published, which takes nothing from outside the crate's directory; the
  public site mounts it from there (`web/hugo.toml`).
- **Style sheets.** The public site's are in `web/assets/styles/`, one
  for each part of the pages, so that changes to different parts touch
  different files: `site.css` (the theme's variables, links, the focus
  ring, icons, the sidebar, the table of contents, the top bar),
  `controls.css` (the native controls wherever they stand),
  `constitution.css`, `design.css` (the design and a rule's page),
  `rules-list.css` (the list of the rules, its marks and the tags, and
  the width of its page),
  `filter.css` (the filter above the list),
  `legend.css` (the legend above them) and `view.css` (the view of the
  list and the statements in its rows), whose markup is in
  `web/layouts/_partials/rules/`. Hugo joins them, after
  the theme's and `tokens.css`, into one file, in the order of
  `web/assets/styles/index.yaml`. The local site's own is
  `crates/statute/src/site.css`, which `statute` writes into each page
  after `tokens.css`; it draws the check boxes as `controls.css` does,
  and holds nothing of the public site's other sheets, which style the
  theme's markup.
- **Icons.** The SVG files are in `crates/statute/assets/icons/`, one
  directory per source, `lucide/` and `octicons/`, unchanged from the
  release, with the license in each and the versions in its
  `README.md`. A page includes an icon by its name,
  `partial "icon.html" "lucide:funnel"`, which inlines the file's SVG on
  one line with the class `icon`, hidden from screen readers; the class
  sets its stroke, `icon-stroke`, and its size, `icon-sm` unless the
  component makes it `icon-md`. A chevron that turns is `chevron-right`
  turned a quarter, which draws `chevron-down`. The local site inlines
  the files it needs in the same way, built into `statute` with
  `include_str!`.
- **Scripts.** A script that holds more than a few lines of logic, the
  list of the rules, its filter and its view, and the sidebar and its
  drawers today, is written in TypeScript in
  `web/assets/ts/`, the logic that needs no page (the filter's choice
  and the address) in a file of its own. Hugo's `js.Build`, the esbuild
  that Hugo carries, bundles each one, with no Node and no npm package,
  for the target `es2018`, minified, and the page inlines it in its
  head as it does the other scripts; there is no source map, as the
  script is small and its source is in the repository. The short
  scripts stay JavaScript in `web/assets/`, minified by Hugo. The local
  site has no build step: the index's one script,
  `crates/statute/src/site.js`, is JavaScript that `statute` inlines
  with `include_str!`, and the page works without it. Read on
  2026-10-15 with Hugo 0.167.0: `js.Build` strips the types and
  type-only imports, bundles the imports between the files, lowers `?.`
  and `??` to the target, and fails the build on a syntax error.
- **Types and tests.** esbuild does not check types: a type error
  builds, and the build passes. Neither the machines nor CI have `tsc`,
  which would need Node and an npm package the site does not otherwise
  use, so no tool checks the types today; the reviewer reads them. The
  behavior is tested on the built page instead: a test of `tools/site`
  builds the site from made-up rules, loads the list in headless Chrome
  with each of a set of addresses and reads the page Chrome leaves
  (`--dump-dom`), skipped without Chrome except in CI. Clicks and keys,
  which `--dump-dom` cannot send, are checked by hand the same way, with
  a script added to a copy of the page that sends them.

## Status

Built today on the public site: every token, in `tokens.css`, and every
color, font size, weight, spacing, radius, shadow, duration and easing
of its style sheets named by a token, but in the parts below that are
not built yet; the icons of the table in [Icons](#icons) that the pages
have today; the motion of [Motion](#motion), reduced motion included,
on what moves today (the popover on the length, the chevrons, the
flash on a target, the drawers of a narrow screen); hover only on fine
pointers; the sizes of targets; the sidebar, the table of contents and
the drawers of [Components](#components); the native controls; the
fonts, the text size and the column, and the breakpoints; the rows of
the rules under a provision; the filter, the legend, the table of rules
with its marks and its group rows and head that stick, and the one-of;
the layers; the view of the list and the statements in its rows; the
path of a node; the
filter and the view in the address, pages readable without
scripts, tabular figures, and `aria-expanded` with `aria-controls` on
the info button, the filter's menus, the buttons of the rules under a
provision, the sidebar's groups and language switch and the top bar's
buttons. Built today on the local site: every token, and every value
of its style sheet named by one; the table of rules with its marks and
its group rows and head that stick, the one-of and the legend of
[Components](#components); the filter's groups, with All and None,
closing on Escape or a click outside; the check boxes; hover only on
fine pointers, the sizes of targets, the focus ring, and the motion of
the filter's popovers; the language switch and the tag of a text not
in the page's language; the parent's title after a title two nodes
have; the view of the list of [Components](#components), its choice
in the address as `view`, and kept for the next index opened only when
it opens at an address without one. Not built yet, and what each replaces:

- On the public site, the tags of a rule's page, and enabled and the
  quadrant in the list, are boxes of one look; the tags of
  [Components](#components) replace them. A one-of whose options are
  off keeps its box in `border-strong`.
- On the public site the column of the labels is `width-label` wide,
  however wide its labels.
- The local site's pages load no font and set the text in the local
  serif fonts.

## Open questions

None now.

## Sources

Several points above come from published guidelines for interfaces,
read on 2026-10-15 at the commits named:

- emil-design-eng, Emil Kowalski's skill on motion
  (<https://github.com/emilkowalski/skills>, `2d228630`): what is done
  often does not move, `ease-out` for the interface and shorter
  closings, the curve of `ease-out`, popovers from `scale(0.95)`,
  reduced motion as less motion rather than none, hover only on fine
  pointers, the delay of tooltips.
- Vercel's Web Interface Guidelines
  (<https://github.com/vercel-labs/web-interface-guidelines>,
  `434b7f91`): target sizes, state in the address, shadows of two
  layers, nested radii, tabular figures, the timing of a loading
  indicator, states that raise contrast, no `transition: all`.
- UI Skills (<https://github.com/ibelick/ui-skills>, `c628a591`):
  names, `aria-expanded` and `aria-controls`, native elements before
  ARIA (fixing-accessibility); transform and opacity only
  (fixing-motion-performance); the next step in an empty state
  (baseline-ui).
- Impeccable (<https://github.com/pbakaus/impeccable>, `a231cc5b`):
  content visible without scripts, one icon library, the floor of the
  text size and the length of a line, part of the list of what is not
  done.
- Anthropic's frontend-design skill
  (<https://github.com/anthropics/skills>, `dbd4588f`): one accent and
  part of the list of what is not done.
- UI Design Brain (<https://github.com/carmahhawwari/ui-design-brain>,
  `38f04c5a`): a border or a shadow, numbers aligned to the end in
  tables, icons between 16px and 24px without mixing outline and fill,
  tips that hold nothing the page lacks.
- Better Icons (<https://github.com/better-auth/better-icons>,
  `033316ec`): icons named by source and kept in one place.
- Tailwind CSS's documentation (<https://tailwindcss.com/docs>, the
  pages as served on 2026-10-15, which have no commit to name): the
  sidebar's and the table of contents' hairline with a bar for the
  current item, muted items that darken under the pointer, the current
  heading following the scroll, the sidebar scrolling its current page
  into view, and a top bar that stays while a drawer opens under it.
