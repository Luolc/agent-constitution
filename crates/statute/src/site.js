// The index of the local site (site.rs). Its style sheet shows the view
// of the radio buttons, so the page works without this script, which
// writes the view into the address, `view=title` or `view=statement`
// and nothing for the default, so that back and forward return to it,
// and keeps it for the next index opened.
(() => {
  const views = Array.from(document.querySelectorAll("input[name='view']"));
  if (!views.length) return;
  const KEY = "statute-site-view";
  // A page opened from disk may refuse to change its address or to keep
  // a value.
  const load = () => {
    let view = new URLSearchParams(location.search).get("view");
    if (!view) {
      try {
        view = localStorage.getItem(KEY);
      } catch {}
    }
    (views.find((v) => v.value === view) ?? views[0]).checked = true;
  };
  for (const view of views) {
    view.addEventListener("change", () => {
      const query = new URLSearchParams(location.search);
      if (view === views[0]) query.delete("view");
      else query.set("view", view.value);
      const search = query.toString();
      try {
        localStorage.setItem(KEY, view.value);
      } catch {}
      try {
        history.pushState(null, "", search ? `?${search}` : location.pathname);
      } catch {}
    });
  }
  window.addEventListener("popstate", load);
  load();
})();
