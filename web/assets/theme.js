// The color theme: "auto" follows the system, "light" and "dark" are the
// reader's choice, kept in localStorage. Inlined in <head>, so the theme is
// set before the page is drawn; without storage the switch still works for
// the page at hand.
(function () {
  var KEY = "constitution-theme";
  var NEXT = { auto: "light", light: "dark", dark: "auto" };
  var root = document.documentElement;
  var theme = null;
  try { theme = localStorage.getItem(KEY); } catch (e) {}
  root.dataset.theme = NEXT[theme] ? theme : "auto";
  document.addEventListener("click", function (event) {
    if (!event.target.closest(".theme-toggle")) return;
    root.dataset.theme = NEXT[root.dataset.theme] || "auto";
    try { localStorage.setItem(KEY, root.dataset.theme); } catch (e) {}
  });
})();
