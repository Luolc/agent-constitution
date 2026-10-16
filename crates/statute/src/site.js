// The index of the local site (site.rs). Its style sheet filters the
// rows by the menus' check boxes and shows the view of the radio
// buttons, so the page works without this script; it adds what the style
// sheet cannot do.
//
// The address holds the filter, the search and the view, each as it
// differs from the page's defaults: a field's values joined by commas
// (`status=active,trial`, `status=` choosing none), `q` the search,
// `view=title` or `view=statement`. Back and forward return to what the
// address says, an address without `view` showing the default view; but
// when the index opens at such an address, it shows the view kept from
// the last choice and writes it into that address, and when it opens at
// an address with an unknown view, it shows the default and leaves the
// view out of the address.
//
// Each menu says what it chooses, has All and None, and closes on
// Escape, a click outside it or another menu opening; checking or
// clearing a domain does the same to the domains below it, and the
// domains can be searched. The search shows the rows whose text holds
// what is typed. Clear filters brings back the default filter and an
// empty search. The groups and the sources the filter empties are
// hidden, the number of rules shown is written under the controls, and
// a filter that leaves no row says so and offers to reset it. The legend
// stays folded or open as the reader left it.
(() => {
  const menus = Array.from(document.querySelectorAll("details.menu"));
  const views = Array.from(document.querySelectorAll("input[name='view']"));
  const legend = document.querySelector("details.legend");
  const find = document.querySelector(".find");
  const search = find?.querySelector("input");
  const clear = document.querySelector(".clear");
  const shownLine = document.querySelector(".shown");
  const none = document.querySelector(".none");
  const items = Array.from(document.querySelectorAll(".item"));
  const VIEW = "statute-site-view";
  const LEGEND = "statute-site-legend";
  const fields = menus.map((menu) => menu.dataset.field);
  const boxes = (field) => Array.from(document.querySelectorAll(`.menu input[name='${field}']`));
  const chosen = (field) => boxes(field).filter((box) => box.checked).map((box) => box.value);
  const defaults = new Map(fields.map((f) => [f, boxes(f).filter((b) => b.defaultChecked).map((b) => b.value)]));
  const texts = new Map(items.map((item) => [item, item.textContent.toLowerCase()]));
  // A page opened from disk may refuse to change its address or to keep
  // a value.
  const kept = (key) => {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  };
  const keep = (key, value) => {
    try {
      localStorage.setItem(key, value);
    } catch {}
  };

  // The address with the page's state in place of what it held, its
  // other parameters kept.
  const address = () => {
    const other = new URLSearchParams(location.search);
    for (const key of [...fields, "q", "view"]) other.delete(key);
    const query = other.toString() ? [other.toString()] : [];
    for (const f of fields) {
      const values = chosen(f);
      if (values.join(",") !== defaults.get(f).join(",")) {
        query.push(`${f}=${values.map(encodeURIComponent).join(",")}`);
      }
    }
    if (search?.value) query.push(`q=${encodeURIComponent(search.value)}`);
    const view = views.find((v) => v.checked);
    if (view && view !== views[0]) query.push(`view=${view.value}`);
    return query.length ? `?${query.join("&")}` : location.pathname;
  };
  const save = (push) => {
    try {
      if (push) history.pushState(null, "", address());
      else history.replaceState(null, "", address());
    } catch {}
  };
  // The address's filter, search and view; when `first`, a missing view
  // is the one kept.
  const load = (first) => {
    const query = new URLSearchParams(location.search);
    for (const f of fields) {
      const values = query.has(f) ? query.get(f).split(",") : defaults.get(f);
      for (const box of boxes(f)) box.checked = values.includes(box.value);
    }
    if (search) search.value = query.get("q") ?? "";
    let view = query.get("view");
    if (!view && first) view = kept(VIEW);
    const shown = views.find((v) => v.value === view) ?? views[0];
    if (shown) shown.checked = true;
    if (first && shown && (query.has("view") ? shown.value !== view : shown !== views[0])) save(false);
  };

  // Writes what each menu chooses, hides what the search leaves out and
  // what the filter emptied, and counts the rules shown.
  const show = () => {
    for (const menu of menus) {
      const field = menu.dataset.field;
      const all = boxes(field);
      const on = all.filter((box) => box.checked);
      const value = menu.querySelector(".value");
      if (on.length === all.length) value.textContent = menu.dataset.allText;
      else if (on.length === 0) value.textContent = menu.dataset.noneText;
      else if (on.length === 1) value.textContent = on[0].parentElement.textContent.trim();
      else value.textContent = `${on.length}/${all.length}`;
      menu.classList.toggle("changed", chosen(field).join(",") !== defaults.get(field).join(","));
    }
    const text = (search?.value ?? "").trim().toLowerCase();
    for (const item of items) item.hidden = Boolean(text) && !texts.get(item).includes(text);
    const parts = Array.from(document.querySelectorAll(".block, .source"));
    for (const part of parts) part.hidden = false;
    // An item only carries its rows (`display: contents`): it is shown
    // when its first row is.
    const visible = (item) => (item.firstElementChild?.getClientRects().length ?? 0) > 0;
    for (const part of parts) part.hidden = !Array.from(part.querySelectorAll(".item")).some(visible);
    // An item is a rule, or a one-of that holds its options.
    const count = items
      .filter(visible)
      .reduce((n, item) => n + Math.max(1, item.querySelectorAll(".options > li").length), 0);
    if (shownLine) shownLine.textContent = shownLine.dataset.words.replace("{}", String(count));
    if (none) none.hidden = count > 0;
  };

  // The first change after a menu opens or the search takes the focus
  // adds an entry to the history, the next ones replace it.
  let pushed = false;
  const changed = () => {
    save(!pushed);
    pushed = true;
    show();
  };
  const reset = () => {
    for (const f of fields) for (const box of boxes(f)) box.checked = box.defaultChecked;
    if (search) search.value = "";
    pushed = false;
    changed();
  };

  // Each summary says whether its details are open, kept in step as
  // they open and close.
  const expanded = (details) => {
    details.querySelector("summary").setAttribute("aria-expanded", String(details.open));
  };
  for (const details of document.querySelectorAll("details")) {
    expanded(details);
    details.addEventListener("toggle", () => expanded(details));
  }

  for (const menu of menus) {
    const field = menu.dataset.field;
    for (const hidden of menu.querySelectorAll("[hidden]")) hidden.hidden = false;
    menu.addEventListener("toggle", () => {
      if (!menu.open) return;
      pushed = false;
      for (const other of menus) if (other !== menu) other.open = false;
    });
    menu.addEventListener("change", (event) => {
      const box = event.target;
      if (box.type !== "checkbox") return;
      if (field === "domain") {
        const below = [box.value];
        for (const other of boxes(field)) {
          if (below.includes(other.dataset.above)) {
            other.checked = box.checked;
            below.push(other.value);
          }
        }
      }
      changed();
    });
    menu.addEventListener("click", (event) => {
      const all = event.target.closest("[data-all]");
      if (!all) return;
      for (const box of boxes(field)) box.checked = all.dataset.all === "true";
      changed();
    });
    const within = menu.querySelector(".search");
    within?.addEventListener("input", () => {
      const text = within.value.trim().toLowerCase();
      for (const label of menu.querySelectorAll(".values label")) {
        label.hidden = !label.textContent.toLowerCase().includes(text);
      }
    });
    menu.addEventListener("keydown", (event) => {
      if (event.key !== "Escape" || !menu.open) return;
      event.preventDefault();
      menu.open = false;
      menu.querySelector("summary").focus();
    });
  }
  document.addEventListener("pointerdown", (event) => {
    for (const menu of menus) if (menu.open && !menu.contains(event.target)) menu.open = false;
  });
  if (search) {
    search.addEventListener("focus", () => {
      pushed = false;
    });
    search.addEventListener("input", changed);
    find.hidden = false;
  }
  for (const view of views) {
    view.addEventListener("change", () => {
      keep(VIEW, view.value);
      save(true);
    });
  }
  if (clear) {
    clear.addEventListener("click", reset);
    clear.hidden = false;
  }
  if (shownLine) shownLine.hidden = false;
  document.querySelector(".none .reset")?.addEventListener("click", reset);
  if (legend) {
    if (kept(LEGEND) === "closed") legend.open = false;
    legend.addEventListener("toggle", () => keep(LEGEND, legend.open ? "open" : "closed"));
  }
  window.addEventListener("popstate", () => {
    load(false);
    show();
  });

  load(true);
  show();
})();
