// The list of the rules (layouts/rules/section.html) with the filter and
// the page size the reader chose, read from the address: the values
// chosen of each field as assets/ts/filter.ts writes them, `per` (25,
// 50, 100 or all) and `page`. Without them, the default filter, as the
// menus' check boxes were built, the page size Hugo built the pages
// with, all by default, and the page Hugo built. Every page
// holds every rule, so this only shows the rules of the filter and page
// on the rules' items (`.rule-item`, its values in `data-*`), and hides
// the others; shows an item's group row where the group changes and on
// top of the page, each group of its path with its id the first time it
// shows on the page; and makes the links to the other pages again. A
// change to the filter, the page size or the page applies at once and
// is written into the address: the first change after a menu of the
// filter opens adds an entry to the history, the next ones replace it,
// so back returns to the list as it was before. Checking or unchecking a domain does the same to the
// domains under it. Each menu's button says what its field chooses, and
// Reset filter shows while a field differs from the default filter,
// with their number.
// Inlined in <head>.
import { differing, type Fields, type Filter, matches, read, write } from "./filter";

const SIZES = ["25", "50", "100", "all"];

document.addEventListener("DOMContentLoaded", () => {
  const list = document.querySelector<HTMLElement>(".rules-list");
  const controls = document.querySelector<HTMLElement>(".rules-controls");
  if (!list || !controls) return;
  // The controls below are in the partial that made `controls`
  // (layouts/_partials/rules/filter.html), so they are there.
  const filter = controls.querySelector<HTMLElement>(".rules-filter")!;
  const badge = controls.querySelector<HTMLElement>(".rules-filter-count")!;
  const resetButton = controls.querySelector<HTMLButtonElement>(".rules-filter-reset")!;
  const select = controls.querySelector<HTMLSelectElement>("select[name='per']")!;
  const none = document.querySelector<HTMLElement>(".rules-none");
  const pager = document.querySelector<HTMLElement>(".rules-pager");
  const icons = document.querySelector<HTMLTemplateElement>("template.rules-pager-icons");
  const table = list.closest<HTMLElement>(".rules-table") ?? list;
  const items = Array.from(list.querySelectorAll<HTMLElement>(".rule-item"));
  const boxes = (field?: string): HTMLInputElement[] =>
    Array.from(filter.querySelectorAll<HTMLInputElement>(field ? `input[name='${field}']` : "input[type='checkbox']"));
  // Each field's menu: its button, the button's value and its group of
  // boxes, which the button controls.
  const menus = Array.from(filter.querySelectorAll<HTMLElement>(".rules-menu"), (menu) => ({
    menu,
    button: menu.querySelector<HTMLButtonElement>(".rules-menu-button")!,
    value: menu.querySelector<HTMLElement>(".rules-menu-value")!,
    group: menu.querySelector<HTMLElement>("[data-field]")!,
  }));

  // The fields as the menus offer them, and the default filter, as their
  // boxes were built.
  const fields: Fields = new Map(
    Array.from(filter.querySelectorAll<HTMLElement>("[data-field]"), (group) => {
      const field = group.dataset.field ?? "";
      return [field, boxes(field).map((box) => box.value)];
    }),
  );
  const chosen = (): Filter =>
    new Map([...fields.keys()].map((f) => [f, new Set(boxes(f).filter((b) => b.checked).map((b) => b.value))]));
  const defaults = chosen();

  let per = list.dataset.size ?? "all";
  let page = 1;
  // Whether this opening of a menu has added its entry to the history.
  let pushed = false;
  const address = (n: number): string => {
    const query = write(chosen(), fields, defaults);
    if (per !== list.dataset.size) query.push(`per=${per}`);
    if (n > 1 || list.dataset.first !== "0") query.push(`page=${n}`);
    return query.length ? `${location.pathname}?${query.join("&")}` : location.pathname;
  };
  const save = (push: boolean): void => {
    if (push) history.pushState(null, "", address(page));
    else history.replaceState(null, "", address(page));
  };
  const load = (): void => {
    const query = new URLSearchParams(location.search);
    const choice = read(query, fields, defaults);
    for (const box of boxes()) box.checked = choice.get(box.name)?.has(box.value) ?? false;
    per = query.get("per") ?? "";
    if (!SIZES.includes(per)) per = list.dataset.size ?? "all";
    const size = per === "all" ? 0 : Number(per);
    page = query.has("page")
      ? Number(query.get("page"))
      : size
        ? Math.floor(Number(list.dataset.first) / size) + 1
        : 1;
    // An address of the old form is written in the new one.
    if (query.has("filter")) save(false);
  };

  // Shows the rules of the filter and the page.
  const show = (): void => {
    const choice = chosen();
    const matched = items.filter((item) =>
      matches(choice, fields, (field) => (item.dataset[field] ?? "").split(" ")),
    );
    const size = per === "all" ? Math.max(matched.length, 1) : Number(per);
    const pages = Math.max(1, Math.ceil(matched.length / size));
    page = Math.min(Math.max(Math.floor(page) || 1, 1), pages);
    const shown = matched.slice((page - 1) * size, page * size);

    for (const item of items) item.hidden = true;
    for (const heading of list.querySelectorAll<HTMLElement>(".rules-group")) heading.hidden = true;
    for (const part of list.querySelectorAll<HTMLElement>(".group-part")) part.removeAttribute("id");
    let previous: string | undefined;
    const ids = new Set<string>();
    for (const item of shown) {
      item.hidden = false;
      const heading = item.querySelector<HTMLElement>(".rules-group");
      const group = heading?.dataset.group;
      if (heading && group && group !== previous) {
        heading.hidden = false;
        for (const part of heading.querySelectorAll<HTMLElement>(".group-part")) {
          const key = part.dataset.group ?? "";
          if (ids.has(key)) continue;
          part.id = `group-${key}`;
          ids.add(key);
        }
      }
      previous = group;
    }
    if (none) none.hidden = shown.length > 0;

    select.value = per;
    const changedFields = differing(choice, fields, defaults);
    for (const { button, value, group } of menus) {
      const field = group.dataset.field ?? "";
      const n = choice.get(field)?.size ?? 0;
      const all = fields.get(field)?.length ?? 0;
      value.textContent =
        n === all ? (filter.dataset.every ?? "") : n === 0 ? (filter.dataset.nothing ?? "") : String(n);
      button.classList.toggle("changed", changedFields.includes(field));
    }
    const n = changedFields.length;
    badge.textContent = String(n);
    badge.hidden = n === 0;
    if (n === 0 && document.activeElement === resetButton) menus[0]?.button.focus();
    resetButton.hidden = n === 0;

    if (pager) {
      pager.textContent = "";
      const link = (n: number, text: string, label?: string, rel?: string): void => {
        const a = document.createElement("a");
        // The previous and next pages' links show the icons the page
        // keeps in a template, the others their number.
        const icon = rel ? icons?.content.querySelector(`[data-rel='${rel}']`) : null;
        if (icon) a.append(icon.cloneNode(true));
        else a.textContent = text;
        if (n === page && !rel) {
          a.setAttribute("aria-current", "page");
        } else {
          a.href = address(n);
          a.addEventListener("click", (event) => {
            event.preventDefault();
            page = n;
            save(true);
            show();
            table.scrollIntoView();
          });
        }
        if (label) a.setAttribute("aria-label", label);
        if (rel) a.rel = rel;
        pager.append(a);
      };
      if (pages > 1) {
        if (page > 1) link(page - 1, "‹", pager.dataset.previous, "prev");
        for (let i = 1; i <= pages; i++) link(i, String(i));
        if (page < pages) link(page + 1, "›", pager.dataset.next, "next");
      }
    }
  };

  // A change to the filter: the first page of the new list.
  const changed = (): void => {
    page = 1;
    save(!pushed);
    pushed = true;
    show();
  };
  const checkAll = (checked: boolean, field?: string): void => {
    for (const box of boxes(field)) box.checked = checked;
    changed();
  };
  // Back to the default filter, as the address without it.
  const reset = (): void => {
    for (const box of boxes()) box.checked = defaults.get(box.name)?.has(box.value) ?? false;
    changed();
  };

  // The menus' changes come from their check boxes and search boxes.
  filter.addEventListener("change", (event) => {
    const box = event.target as HTMLInputElement;
    if (box.type !== "checkbox") return;
    if (box.name === "domain") {
      const below = [box.value];
      for (const other of boxes("domain")) {
        if (below.includes(other.dataset.parent ?? "")) {
          other.checked = box.checked;
          below.push(other.value);
        }
      }
    }
    changed();
  });
  filter.addEventListener("click", (event) => {
    const all = (event.target as Element).closest<HTMLElement>("[data-all]");
    if (all) checkAll(all.dataset.all === "true", all.closest<HTMLElement>("[data-field]")?.dataset.field);
  });
  // A search box shows the boxes whose label holds what is typed.
  filter.addEventListener("input", (event) => {
    const search = event.target as HTMLInputElement;
    if (search.type !== "search") return;
    const typed = search.value.trim().toLowerCase();
    for (const label of search.closest("[data-field]")?.querySelectorAll<HTMLElement>("label") ?? []) {
      label.hidden = !(label.textContent ?? "").toLowerCase().includes(typed);
    }
  });
  resetButton.addEventListener("click", reset);
  const again = document.querySelector<HTMLButtonElement>(".rules-none-reset");
  again?.addEventListener("click", reset);
  if (again) again.hidden = false;

  // The menus, each opened and closed by its button, one open at a
  // time: Escape, a click outside or the focus leaving it closes it,
  // Escape and the button giving the focus back to the button.
  type Menu = (typeof menus)[number];
  let current: Menu | undefined;
  const close = (refocus: boolean): void => {
    if (!current) return;
    const { button, group } = current;
    current = undefined;
    button.setAttribute("aria-expanded", "false");
    group.classList.remove("open");
    if (refocus) button.focus();
  };
  const openMenu = (menu: Menu): void => {
    close(false);
    current = menu;
    pushed = false;
    menu.button.setAttribute("aria-expanded", "true");
    menu.group.classList.add("open");
    (menu.group.querySelector<HTMLElement>("input[type='search']") ?? menu.group.querySelector<HTMLElement>("input, button"))?.focus();
  };
  for (const menu of menus) {
    menu.button.addEventListener("click", () => (current === menu ? close(true) : openMenu(menu)));
  }
  filter.addEventListener("keydown", (event) => {
    if (event.key !== "Escape" || !current) return;
    event.preventDefault();
    close(true);
  });
  // A pointer or focus event's target and related target are nodes.
  document.addEventListener("pointerdown", (event) => {
    if (current && !current.menu.contains(event.target as Node)) close(false);
  });
  filter.addEventListener("focusout", (event) => {
    const next = event.relatedTarget as Node | null;
    if (current && next && !current.menu.contains(next)) close(false);
  });

  select.addEventListener("change", () => {
    per = select.value;
    page = 1;
    save(true);
    show();
  });
  window.addEventListener("popstate", () => {
    load();
    show();
  });

  load();
  show();
  controls.hidden = false;
});
