// The index of the local site (site.rs). Its style sheet shows the view
// of the radio buttons, so the page works without this script, which
// writes the view into the address, `view=title` or `view=statement`
// and nothing for the default, so that back and forward return to it,
// and keeps it for the next index opened. The view shown is the
// address's, and an address without one shows the default; but when the
// index opens at such an address, it shows the view kept and writes it
// into that address.
(() => {
  const views = Array.from(document.querySelectorAll("input[name='view']"));
  if (!views.length) return;
  const KEY = "statute-site-view";
  // A page opened from disk may refuse to change its address or to keep
  // a value.
  const address = (view) => {
    const query = new URLSearchParams(location.search);
    if (view === views[0]) query.delete("view");
    else query.set("view", view.value);
    const search = query.toString();
    return search ? `?${search}` : location.pathname;
  };
  const load = (kept) => {
    let view = new URLSearchParams(location.search).get("view");
    if (!view && kept) {
      try {
        view = localStorage.getItem(KEY);
      } catch {}
    }
    const shown = views.find((v) => v.value === view) ?? views[0];
    shown.checked = true;
    if (kept && shown !== views[0]) {
      try {
        history.replaceState(null, "", address(shown));
      } catch {}
    }
  };
  for (const view of views) {
    view.addEventListener("change", () => {
      try {
        localStorage.setItem(KEY, view.value);
      } catch {}
      try {
        history.pushState(null, "", address(view));
      } catch {}
    });
  }
  window.addEventListener("popstate", () => load(false));
  load(true);
})();
