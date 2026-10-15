// The sidebar and the table of contents. On a narrow screen each is a
// drawer opened by its button in the top bar (layouts/_partials/docs/
// header.html): the button says whether it is open and which drawer it
// controls; while one is open the rest of the page is inert, and Escape,
// a second click on the button or a click outside closes it, as does a
// link followed in it. Focus goes to the drawer's first link when it
// opens and back to the button when it closes from the keyboard or the
// button. The style sheet (assets/styles/site.css) draws the drawer from
// `data-drawer` on the root. A group of the sidebar that folds, and the
// language switch, have a button (`data-fold`) that shows or hides their
// list. Once the page is parsed, the
// sidebar scrolls its current page into view when the theme's restored
// position left it out. Inlined in <head>, so it listens on the document.
(function () {
  var root = document.documentElement;
  var DRAWERS = { menu: ".book-menu-content", toc: ".book-toc-content" };
  // The widths at which a drawer is one: above them it is a column.
  var NARROW = { menu: "(width <= 56rem)", toc: "(width <= 70rem)" };

  function toggles() { return document.querySelectorAll(".drawer-toggle"); }
  function panel(name) { return document.querySelector(DRAWERS[name]); }

  function setInert(name, inert) {
    document.querySelectorAll(".book-page, .book-menu, .book-toc").forEach(function (part) {
      if (!part.contains(panel(name))) part.inert = inert;
    });
  }

  function open(name) {
    if (root.dataset.drawer) close(false);
    root.dataset.drawer = name;
    toggles().forEach(function (button) {
      button.setAttribute("aria-expanded", String(button.dataset.drawer === name));
    });
    setInert(name, true);
    var first = panel(name).querySelector("a[href], button");
    if (first) first.focus({ preventScroll: true });
  }

  function close(refocus) {
    var name = root.dataset.drawer;
    if (!name) return;
    delete root.dataset.drawer;
    setInert(name, false);
    var button = null;
    toggles().forEach(function (each) {
      each.setAttribute("aria-expanded", "false");
      if (each.dataset.drawer === name) button = each;
    });
    if (refocus && button) button.focus();
  }

  document.addEventListener("click", function (event) {
    var fold = event.target.closest("[data-fold]");
    if (fold) {
      fold.setAttribute("aria-expanded", String(fold.getAttribute("aria-expanded") !== "true"));
      return;
    }
    var button = event.target.closest(".drawer-toggle");
    if (button) {
      if (root.dataset.drawer === button.dataset.drawer) close(true);
      else open(button.dataset.drawer);
      return;
    }
    var name = root.dataset.drawer;
    if (!name) return;
    var inside = panel(name).contains(event.target);
    if (!inside || event.target.closest("a[href]")) close(false);
  });

  document.addEventListener("keydown", function (event) {
    if (event.key === "Escape" && root.dataset.drawer) close(true);
  });

  document.addEventListener("DOMContentLoaded", function () {
    toggles().forEach(function (button) {
      var drawer = panel(button.dataset.drawer);
      if (!drawer) return;
      drawer.id = drawer.id || "drawer-" + button.dataset.drawer;
      button.setAttribute("aria-controls", drawer.id);
    });
    Object.keys(NARROW).forEach(function (name) {
      window.matchMedia(NARROW[name]).addEventListener("change", function () {
        if (root.dataset.drawer === name) close(false);
      });
    });
    var menu = panel("menu");
    var current = menu && menu.querySelector("nav a.active");
    if (!current) return;
    var shown = menu.getBoundingClientRect(), at = current.getBoundingClientRect();
    if (at.top < shown.top || at.bottom > shown.bottom) {
      menu.scrollTop += at.top - shown.top - (shown.height - at.height) / 2;
    }
  });
})();
