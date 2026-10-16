// What the rows of the list of the rules show
// (layouts/_partials/rules/view.html): the value of the address's
// `view` (title or statement), else the one the reader chose last,
// kept in localStorage, else title. The style sheet does the showing
// from the checked radio button; this checks it, and writes a change
// into the address as an entry of the history, leaving out title, the
// default, so that back and forward return to the view as it was.
// The address is rewritten in place to say what the page shows: a view
// taken from localStorage is written into it, the default and an
// unknown view are left out. The other parameters of the address are
// kept. Inlined in <head>.

const VIEWS = ["title", "statement"];
const DEFAULT = "title";
const KEY = "rules-view";

// The view saved, or null where storage is blocked or holds no view.
function saved(): string | null {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

function keep(view: string): void {
  try {
    localStorage.setItem(KEY, view);
  } catch {
    // Storage blocked: the view lasts as long as the address.
  }
}

// The address with the view, or without it for the default; the other
// parameters as they are written, the filter's commas unescaped.
function address(view: string): string {
  const params = location.search
    .slice(1)
    .split("&")
    .filter((param) => param && !param.startsWith("view="));
  if (view !== DEFAULT) params.push(`view=${view}`);
  return `${location.pathname}${params.length ? `?${params.join("&")}` : ""}${location.hash}`;
}

document.addEventListener("DOMContentLoaded", () => {
  const control = document.querySelector<HTMLElement>(".rules-view");
  if (!control) return;
  const radios = Array.from(control.querySelectorAll<HTMLInputElement>("input[name='view']"));
  const check = (view: string): void => {
    for (const radio of radios) radio.checked = radio.value === view;
  };
  const load = (stored: boolean): void => {
    const given = new URLSearchParams(location.search).get("view");
    let view = given && VIEWS.includes(given) ? given : DEFAULT;
    if (!given && stored) {
      const last = saved();
      if (last && VIEWS.includes(last)) view = last;
    }
    check(view);
    const written = address(view);
    if (written !== location.pathname + location.search + location.hash) {
      history.replaceState(history.state, "", written);
    }
  };

  control.addEventListener("change", (event) => {
    const view = (event.target as HTMLInputElement).value;
    keep(view);
    history.pushState(history.state, "", address(view));
  });
  window.addEventListener("popstate", () => load(false));
  load(true);
});
