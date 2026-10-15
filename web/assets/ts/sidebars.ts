// The sidebar and the table of contents. On a narrow screen each is a
// drawer opened by its button in the top bar (layouts/_partials/docs/
// header.html): the button says whether it is open and which drawer it
// controls; while one is open the rest of the page is inert, and Escape,
// a second click on the button or a click outside closes it, as does a
// link followed in it. Focus goes to the drawer's first link shown when
// it opens and back to the button when it closes from the keyboard or the
// button. The style sheet (assets/styles/site.css) draws the drawer from
// `data-drawer` on the root. A group of the sidebar that folds, and the
// language switch, have a button (`data-fold`) that shows or hides their
// list. Once the page is parsed, the sidebar scrolls its current page
// into view when the theme's restored position left it out. Inlined in
// <head>, so it listens on the document.

type Drawer = "menu" | "toc";

const DRAWERS: Record<Drawer, string> = { menu: ".book-menu-content", toc: ".book-toc-content" };
// The widths at which a drawer is one: above them it is a column.
const NARROW: Record<Drawer, string> = { menu: "(width <= 56rem)", toc: "(width <= 70rem)" };

const root = document.documentElement;

const toggles = (): HTMLButtonElement[] =>
  Array.from(document.querySelectorAll<HTMLButtonElement>(".drawer-toggle"));
const panel = (name: Drawer): HTMLElement | null => document.querySelector<HTMLElement>(DRAWERS[name]);
// The drawer open now, if any; the root's `data-drawer` holds it.
const opened = (): Drawer | undefined => root.dataset.drawer as Drawer | undefined;

function setInert(name: Drawer, inert: boolean): void {
  const drawer = panel(name);
  document.querySelectorAll<HTMLElement>(".book-page, .book-menu, .book-toc").forEach((part) => {
    if (!drawer || !part.contains(drawer)) part.inert = inert;
  });
}

function open(name: Drawer): void {
  if (opened()) close(false);
  root.dataset.drawer = name;
  for (const button of toggles()) {
    button.setAttribute("aria-expanded", String(button.dataset.drawer === name));
  }
  setInert(name, true);
  // The first link or button shown: the style sheet hides some, as the
  // sidebar's copy of the site's title.
  const targets = panel(name)?.querySelectorAll<HTMLElement>("a[href], button") ?? [];
  Array.from(targets)
    .find((each) => each.getClientRects().length > 0)
    ?.focus({ preventScroll: true });
}

function close(refocus: boolean): void {
  const name = opened();
  if (!name) return;
  delete root.dataset.drawer;
  setInert(name, false);
  let button: HTMLButtonElement | undefined;
  for (const each of toggles()) {
    each.setAttribute("aria-expanded", "false");
    if (each.dataset.drawer === name) button = each;
  }
  if (refocus) button?.focus();
}

document.addEventListener("click", (event) => {
  if (!(event.target instanceof Element)) return;
  const target = event.target;
  const fold = target.closest("[data-fold]");
  if (fold) {
    fold.setAttribute("aria-expanded", String(fold.getAttribute("aria-expanded") !== "true"));
    return;
  }
  const button = target.closest<HTMLElement>(".drawer-toggle");
  if (button) {
    const name = button.dataset.drawer as Drawer;
    if (opened() === name) close(true);
    else open(name);
    return;
  }
  const name = opened();
  if (!name) return;
  const inside = panel(name)?.contains(target) ?? false;
  if (!inside || target.closest("a[href]")) close(false);
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && opened()) close(true);
});

document.addEventListener("DOMContentLoaded", () => {
  for (const button of toggles()) {
    const drawer = panel(button.dataset.drawer as Drawer);
    if (!drawer) continue;
    drawer.id ||= `drawer-${button.dataset.drawer}`;
    button.setAttribute("aria-controls", drawer.id);
  }
  for (const name of Object.keys(NARROW) as Drawer[]) {
    window.matchMedia(NARROW[name]).addEventListener("change", () => {
      if (opened() === name) close(false);
    });
  }
  const menu = panel("menu");
  const current = menu?.querySelector("nav a.active");
  if (!menu || !current) return;
  const shown = menu.getBoundingClientRect();
  const at = current.getBoundingClientRect();
  if (at.top < shown.top || at.bottom > shown.bottom) {
    menu.scrollTop += at.top - shown.top - (shown.height - at.height) / 2;
  }
});
