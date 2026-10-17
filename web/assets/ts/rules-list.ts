// The list of the rules (layouts/rules/section.html) with the search and
// the filter the reader chose, read from the address: the values chosen
// of each field as assets/ts/filter.ts writes them, and `q`, what the
// search holds. Without them, an empty search and the default filter, as
// the menus' check boxes were built. The page holds every rule, so this
// only shows the rules of the filter whose rows' text (their IDs, titles
// and statements) holds what is searched, case aside, on the rules'
// items (`.rule-item`, its values in `data-*`), and hides the others;
// and shows an item's group row where the group changes and above the
// first rule shown, each group of its path with its id the first time it
// shows; and writes the number of rules shown, a one-of's options each
// counted. A change applies at once and is written into the address: the
// first change after a menu of the filter opens or the search takes the
// focus adds an entry to the history, the next ones replace it, so back
// returns to the list as it was before. Checking or unchecking a domain
// does the same to the domains under it. Each menu's button says what
// its field chooses: All, None, the one value chosen, or how many of how
// many. Clear filters brings back the default filter and an empty
// search. The legend stays folded or open as the reader left it, kept in
// localStorage. An address of the list's earlier forms, with `filter=1`,
// a page size (`per`) or a page (`page`), shows its filter on the whole
// list and is written again without them. Inlined in <head>.
import { differing, type Fields, type Filter, matches, read, write } from "./filter";

document.addEventListener("DOMContentLoaded", () => {
  const list = document.querySelector<HTMLElement>(".rules-list");
  const bar = document.querySelector<HTMLElement>(".rules-bar");
  if (!list || !bar) return;
  // The controls below are in the partial that made `bar`
  // (layouts/_partials/rules/filter.html), so they are there.
  const find = bar.querySelector<HTMLElement>(".rules-find")!;
  const search = find.querySelector<HTMLInputElement>("input")!;
  const filter = bar.querySelector<HTMLElement>(".rules-filter")!;
  const resetButton = bar.querySelector<HTMLButtonElement>(".rules-filter-reset")!;
  const count = document.querySelector<HTMLElement>(".rules-count")!;
  const none = document.querySelector<HTMLElement>(".rules-none");
  const items = Array.from(list.querySelectorAll<HTMLElement>(".rule-item"));
  // An item's text, that of its rows, not of its group row nor of the
  // tips on its parameters' values.
  const text = (row: Element): string => {
    const copy = row.cloneNode(true) as Element;
    copy.querySelectorAll(".param-tip").forEach((tip) => tip.remove());
    return copy.textContent ?? "";
  };
  const texts = new Map(
    items.map((item) => [
      item,
      Array.from(item.querySelectorAll(".rule-entry"), text)
        .join(" ")
        .toLowerCase(),
    ]),
  );
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

  // Whether this opening of a menu has added its entry to the history.
  let pushed = false;
  const address = (): string => {
    const query = write(chosen(), fields, defaults);
    if (search.value) query.push(`q=${encodeURIComponent(search.value)}`);
    // What the rows show is assets/ts/rules-view.ts's, kept as it is.
    const view = new URLSearchParams(location.search).get("view");
    if (view) query.push(`view=${encodeURIComponent(view)}`);
    return query.length ? `${location.pathname}?${query.join("&")}` : location.pathname;
  };
  const save = (push: boolean): void => {
    if (push) history.pushState(null, "", address());
    else history.replaceState(null, "", address());
  };
  const load = (): void => {
    const query = new URLSearchParams(location.search);
    const choice = read(query, fields, defaults);
    for (const box of boxes()) box.checked = choice.get(box.name)?.has(box.value) ?? false;
    search.value = query.get("q") ?? "";
    if (["filter", "per", "page"].some((p) => query.has(p))) save(false);
  };

  // Shows the rules of the search and the filter.
  const show = (): void => {
    const choice = chosen();
    const typed = search.value.trim().toLowerCase();
    const shown = items.filter(
      (item) =>
        texts.get(item)?.includes(typed) &&
        matches(choice, fields, (field) => (item.dataset[field] ?? "").split(" ")),
    );

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
    const rules = shown.reduce((n, item) => n + Math.max(1, item.querySelectorAll(".one-of-options > li").length), 0);
    count.textContent = ((rules === 1 ? count.dataset.singular : count.dataset.plural) ?? "").replace("{}", String(rules));

    const changedFields = differing(choice, fields, defaults);
    for (const { button, value, group } of menus) {
      const field = group.dataset.field ?? "";
      const on = boxes(field).filter((box) => box.checked);
      const all = fields.get(field)?.length ?? 0;
      if (on.length === all) value.textContent = filter.dataset.every ?? "";
      else if (on.length === 0) value.textContent = filter.dataset.nothing ?? "";
      else if (on.length === 1) value.textContent = on[0]?.parentElement?.textContent?.trim() ?? "";
      else value.textContent = `${on.length}/${all}`;
      button.classList.toggle("changed", changedFields.includes(field));
    }
  };

  const changed = (): void => {
    save(!pushed);
    pushed = true;
    show();
  };
  const checkAll = (checked: boolean, field?: string): void => {
    for (const box of boxes(field)) box.checked = checked;
    changed();
  };
  // Back to the default filter and an empty search, as the address
  // without them.
  const reset = (): void => {
    for (const box of boxes()) box.checked = defaults.get(box.name)?.has(box.value) ?? false;
    search.value = "";
    pushed = false;
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
  search.addEventListener("focus", () => {
    pushed = false;
  });
  search.addEventListener("input", changed);
  resetButton.addEventListener("click", reset);
  const again = document.querySelector<HTMLButtonElement>(".rules-none-reset");
  again?.addEventListener("click", reset);
  if (again) again.hidden = false;

  // The menus, each opened and closed by its button, one open at a
  // time: Escape, a click outside or the focus leaving it closes it,
  // Escape and the button giving the focus back to the button. Closing
  // a menu clears its search.
  type Menu = (typeof menus)[number];
  let current: Menu | undefined;
  const close = (refocus: boolean): void => {
    if (!current) return;
    const { button, group } = current;
    current = undefined;
    button.setAttribute("aria-expanded", "false");
    group.classList.remove("open");
    const search = group.querySelector<HTMLInputElement>("input[type='search']");
    if (search?.value) {
      search.value = "";
      for (const label of group.querySelectorAll<HTMLElement>("label")) label.hidden = false;
    }
    if (refocus) button.focus();
  };
  const openMenu = (menu: Menu): void => {
    close(false);
    current = menu;
    pushed = false;
    menu.button.setAttribute("aria-expanded", "true");
    menu.group.classList.add("open");
    // Under its button, but within the box it is placed in, the bar of
    // the filter, so that it never runs past the page's edge.
    const box = (menu.group.offsetParent ?? filter).getBoundingClientRect();
    const button = menu.button.getBoundingClientRect();
    const room = box.width - menu.group.offsetWidth;
    menu.group.style.setProperty("--menu-start", `${Math.max(0, Math.min(button.left - box.left, room))}px`);
    menu.group.style.setProperty("--menu-top", `${button.bottom - box.top}px`);
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

  window.addEventListener("popstate", () => {
    load();
    show();
  });

  // The legend, open by default, as the reader left it; its button says
  // whether it is open.
  const legend = document.querySelector<HTMLDetailsElement>("details.rules-legend");
  if (legend) {
    const summary = legend.querySelector("summary");
    const expanded = (): void => summary?.setAttribute("aria-expanded", String(legend.open));
    try {
      if (localStorage.getItem("rules-legend") === "closed") legend.open = false;
    } catch {
      // Storage blocked: the legend opens as the page has it.
    }
    expanded();
    legend.addEventListener("toggle", () => {
      expanded();
      try {
        localStorage.setItem("rules-legend", legend.open ? "open" : "closed");
      } catch {
        // Storage blocked: the legend stays as it is until the page reloads.
      }
    });
  }

  load();
  show();
  for (const control of [find, filter, resetButton, count]) control.hidden = false;
});
