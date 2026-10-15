// The list of the rules (layouts/rules/section.html) with the filter and
// the page size the reader chose, read from the address: the values
// chosen of each field as assets/ts/filter.ts writes them, `per` (25,
// 50, 100 or all) and `page`. Without them, the default filter, as the
// panel's check boxes were built, and the page Hugo built. Every page
// holds every rule, so this only shows the rules of the filter and page
// on the rules' items (`.rule-item`, its values in `data-*`), and hides
// the others; shows the group headings where the group changes and on
// top of the page, with their ids; and makes the table of contents and
// the links to the other pages again. A change to the filter, the page
// size or the page applies at once and is written into the address: the
// first change after the panel opens adds an entry to the history, the
// next ones replace it, so back returns to the list as it was before.
// Checking or unchecking a domain does the same to the domains under it.
// Inlined in <head>; it runs before assets/toc-current.js, which then
// finds the headings shown.
import { type Fields, type Filter, filtering, matches, read, write } from "./filter";

const SIZES = ["25", "50", "100", "all"];

document.addEventListener("DOMContentLoaded", () => {
  const list = document.querySelector<HTMLElement>(".rules-list");
  const controls = document.querySelector<HTMLElement>(".rules-controls");
  if (!list || !controls) return;
  const button = controls.querySelector<HTMLButtonElement>(".rules-filter-button")!;
  const badge = controls.querySelector<HTMLElement>(".rules-filter-count")!;
  const panel = controls.querySelector<HTMLElement>(".rules-filter-panel")!;
  const select = controls.querySelector<HTMLSelectElement>("select[name='per']")!;
  const none = document.querySelector<HTMLElement>(".rules-none");
  const pager = document.querySelector<HTMLElement>(".rules-pager");
  const items = Array.from(list.querySelectorAll<HTMLElement>(".rule-item"));
  const boxes = (field?: string): HTMLInputElement[] =>
    Array.from(panel.querySelectorAll<HTMLInputElement>(field ? `input[name='${field}']` : "input[type='checkbox']"));

  // The fields as the panel offers them, and the default filter, as its
  // boxes were built.
  const fields: Fields = new Map(
    Array.from(panel.querySelectorAll<HTMLElement>("[data-field]"), (group) => {
      const field = group.dataset.field ?? "";
      return [field, boxes(field).map((box) => box.value)];
    }),
  );
  const chosen = (): Filter =>
    new Map([...fields.keys()].map((f) => [f, new Set(boxes(f).filter((b) => b.checked).map((b) => b.value))]));
  const defaults = chosen();

  let per = list.dataset.size ?? "100";
  let page = 1;
  // Whether this opening of the panel has added its entry to the history.
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
    const filter = read(query, fields, defaults);
    for (const box of boxes()) box.checked = filter.get(box.name)?.has(box.value) ?? false;
    per = query.get("per") ?? "";
    if (!SIZES.includes(per)) per = list.dataset.size ?? "100";
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
    const filter = chosen();
    const matched = items.filter((item) =>
      matches(filter, fields, (field) => (item.dataset[field] ?? "").split(" ")),
    );
    const size = per === "all" ? Math.max(matched.length, 1) : Number(per);
    const pages = Math.max(1, Math.ceil(matched.length / size));
    page = Math.min(Math.max(Math.floor(page) || 1, 1), pages);
    const shown = matched.slice((page - 1) * size, page * size);

    for (const item of items) item.hidden = true;
    for (const heading of list.querySelectorAll<HTMLElement>(".rules-group")) {
      heading.hidden = true;
      heading.removeAttribute("id");
    }
    let previous: (string | undefined)[] = [];
    for (const item of shown) {
      item.hidden = false;
      const groups = Array.from(item.querySelectorAll<HTMLElement>(".rules-group"));
      let changed = false;
      groups.forEach((heading, j) => {
        changed = changed || previous[j] !== heading.dataset.group;
        if (!changed) return;
        heading.hidden = false;
        heading.id = `group-${heading.dataset.group}`;
      });
      previous = groups.map((heading) => heading.dataset.group);
    }
    if (none) none.hidden = shown.length > 0;

    const toc = document.querySelector("#TableOfContents ul");
    if (toc) {
      toc.textContent = "";
      for (const heading of list.querySelectorAll<HTMLElement>(".rules-group[id]")) {
        const li = document.createElement("li");
        li.style.setProperty("--level", String(Number(heading.tagName.slice(1)) - 2));
        const a = document.createElement("a");
        a.href = `#${heading.id}`;
        a.textContent = (heading.querySelector("a") ?? heading).textContent;
        li.append(a);
        toc.append(li);
      }
    }

    select.value = per;
    const n = filtering(filter, fields).length;
    badge.textContent = String(n);
    badge.hidden = n === 0;

    if (pager) {
      pager.textContent = "";
      const link = (n: number, text: string, label?: string, rel?: string): void => {
        const a = document.createElement("a");
        a.textContent = text;
        if (n === page && !rel) {
          a.setAttribute("aria-current", "page");
        } else {
          a.href = address(n);
          a.addEventListener("click", (event) => {
            event.preventDefault();
            page = n;
            save(true);
            show();
            list.scrollIntoView();
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

  panel.addEventListener("change", (event) => {
    const box = event.target as HTMLInputElement;
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
  panel.addEventListener("click", (event) => {
    const all = (event.target as Element).closest<HTMLElement>("[data-all]");
    if (all) checkAll(all.dataset.all === "true", all.closest<HTMLElement>("[data-field]")?.dataset.field);
  });
  const clear = (): void => checkAll(true);
  panel.querySelector(".rules-filter-clear")?.addEventListener("click", clear);
  const reset = document.querySelector<HTMLButtonElement>(".rules-none-clear");
  reset?.addEventListener("click", clear);
  if (reset) reset.hidden = false;

  // The panel, opened and closed by its button: Escape, a click outside
  // or the focus leaving it closes it, Escape and the button giving the
  // focus back to the button.
  const filter = controls.querySelector<HTMLElement>(".rules-filter")!;
  const open = (): boolean => button.getAttribute("aria-expanded") === "true";
  const toggle = (show: boolean, refocus: boolean): void => {
    button.setAttribute("aria-expanded", String(show));
    panel.classList.toggle("open", show);
    if (show) {
      pushed = false;
      panel.querySelector<HTMLElement>("input, button")?.focus();
    } else if (refocus) {
      button.focus();
    }
  };
  button.addEventListener("click", () => toggle(!open(), true));
  filter.addEventListener("keydown", (event) => {
    if (event.key !== "Escape" || !open()) return;
    event.preventDefault();
    toggle(false, true);
  });
  document.addEventListener("pointerdown", (event) => {
    if (open() && !filter.contains(event.target as Node)) toggle(false, false);
  });
  filter.addEventListener("focusout", (event) => {
    const next = event.relatedTarget as Node | null;
    if (open() && next && !filter.contains(next)) toggle(false, false);
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
