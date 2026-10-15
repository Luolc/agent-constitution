// The index of the local site (site.rs). The style sheet filters the
// rows by the menus' check boxes and shows the view of the radio buttons,
// so the page works without this script; it adds what the style sheet
// cannot do. The filter and the view are written into the address, the
// fields and the view that differ from the page's defaults, each field's
// values joined by commas (`status=active,trial&view=title`, `status=`
// choosing none), so back and forward return to what was shown; the view
// is also kept for the next index opened. Each menu shows how many of
// its values are chosen, has All and None, and closes on Escape, a click
// outside it or another menu opening; checking or clearing a domain does
// the same to the domains below it, and the domains can be searched. The
// groups and the sources the filter empties are hidden, and a filter
// that leaves no row says so and offers to reset it.
(() => {
  const menus = Array.from(document.querySelectorAll("details.menu"));
  const views = Array.from(document.querySelectorAll("input[name='view']"));
  if (!menus.length) return;
  const KEY = "statute-site-view";
  const fields = menus.map((menu) => menu.dataset.field);
  const boxes = (field) => Array.from(document.querySelectorAll(`.menu input[name='${field}']`));
  const chosen = (field) => boxes(field).filter((box) => box.checked).map((box) => box.value);
  const defaults = new Map(fields.map((f) => [f, boxes(f).filter((b) => b.defaultChecked).map((b) => b.value)]));
  const none = document.querySelector(".none");

  const address = () => {
    const query = [];
    for (const f of fields) {
      const values = chosen(f);
      if (values.join(",") !== defaults.get(f).join(",")) {
        query.push(`${f}=${values.map(encodeURIComponent).join(",")}`);
      }
    }
    const view = views.find((v) => v.checked);
    if (view && view !== views[0]) query.push(`view=${view.value}`);
    return query.length ? `?${query.join("&")}` : location.pathname;
  };
  // A page opened from disk may refuse a change to its address.
  const save = (push) => {
    try {
      if (push) history.pushState(null, "", address());
      else history.replaceState(null, "", address());
    } catch {}
  };
  const load = () => {
    const query = new URLSearchParams(location.search);
    for (const f of fields) {
      const values = query.has(f) ? query.get(f).split(",") : defaults.get(f);
      for (const box of boxes(f)) box.checked = values.includes(box.value);
    }
    let view = query.get("view");
    if (!view) {
      try {
        view = localStorage.getItem(KEY);
      } catch {}
    }
    (views.find((v) => v.value === view) ?? views[0]).checked = true;
  };

  // Shows each menu's count, hides what the filter emptied.
  const show = () => {
    for (const menu of menus) menu.querySelector(".count").textContent = chosen(menu.dataset.field).length;
    const parts = Array.from(document.querySelectorAll(".block, .source"));
    for (const part of parts) part.hidden = false;
    const shown = (item) => item.getClientRects().length > 0;
    let any = false;
    for (const part of parts) {
      const some = Array.from(part.querySelectorAll(".item")).some(shown);
      any ||= some;
      part.hidden = !some;
    }
    if (none) none.hidden = any;
  };

  // The first change after a menu opens adds an entry to the history, the
  // next ones replace it.
  let pushed = false;
  const changed = () => {
    save(!pushed);
    pushed = true;
    show();
  };
  for (const menu of menus) {
    const field = menu.dataset.field;
    for (const hidden of menu.querySelectorAll("[hidden]")) hidden.hidden = false;
    menu.querySelector(".count").hidden = false;
    menu.addEventListener("toggle", () => {
      if (!menu.open) return;
      pushed = false;
      for (const other of menus) if (other !== menu) other.open = false;
    });
    menu.addEventListener("change", (event) => {
      const box = event.target;
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
    const search = menu.querySelector(".search");
    search?.addEventListener("input", () => {
      const text = search.value.trim().toLowerCase();
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
  for (const view of views) {
    view.addEventListener("change", () => {
      try {
        localStorage.setItem(KEY, view.value);
      } catch {}
      save(true);
    });
  }
  document.querySelector(".none .reset")?.addEventListener("click", () => {
    for (const f of fields) for (const box of boxes(f)) box.checked = box.defaultChecked;
    changed();
  });
  window.addEventListener("popstate", () => {
    load();
    show();
  });

  load();
  show();
})();
