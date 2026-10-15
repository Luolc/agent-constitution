// The rules under a provision (layouts/_partials/rules/cites.html): a
// click or a tap on the button, or Enter or Space from the keyboard, shows
// or hides the rules that cite only a provision under it; Escape within
// the row hides them and leaves the focus on the button. Inlined in
// <head>, so it listens on the document.
(function () {
  function show(row, open) {
    row.classList.toggle("open", open);
    row.querySelector(".cites-button").setAttribute("aria-expanded", String(open));
  }
  document.addEventListener("click", function (event) {
    var button = event.target.closest(".cites-button");
    if (button) show(button.parentElement, !button.parentElement.classList.contains("open"));
  });
  document.addEventListener("keydown", function (event) {
    if (event.key !== "Escape" || !document.activeElement) return;
    var row = document.activeElement.closest(".cites.open");
    if (!row) return;
    show(row, false);
    row.querySelector(".cites-button").focus();
  });
})();
