// The color theme: "auto" follows the system, "light" and "dark" are the
// reader's choice, kept in localStorage. Inlined in <head>, so the theme is
// set before the page is drawn; without storage the switch still works for
// the page at hand. The button's title and accessible name follow the
// setting, from its data attributes.
(function () {
  var KEY = "constitution-theme";
  var NEXT = { auto: "light", light: "dark", dark: "auto" };
  var root = document.documentElement;
  var theme = null;
  try { theme = localStorage.getItem(KEY); } catch (e) {}
  root.dataset.theme = NEXT[theme] ? theme : "auto";
  function label() {
    var button = document.querySelector(".theme-toggle");
    if (!button) return;
    var text = button.dataset[root.dataset.theme];
    button.title = text;
    button.setAttribute("aria-label", text);
  }
  document.addEventListener("DOMContentLoaded", label);
  document.addEventListener("click", function (event) {
    if (!event.target.closest(".theme-toggle")) return;
    root.dataset.theme = NEXT[root.dataset.theme] || "auto";
    try { localStorage.setItem(KEY, root.dataset.theme); } catch (e) {}
    label();
  });
})();
