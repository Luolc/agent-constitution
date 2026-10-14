// The rules under a provision (layouts/_partials/rules/cites.html): a
// click or a tap on the button, or Enter or Space from the keyboard, opens
// or closes the list under it. Inlined in <head>, so it listens on the
// document.
(function () {
  document.addEventListener("click", function (event) {
    var button = event.target.closest(".cites-button");
    if (!button) return;
    var open = !button.parentElement.classList.contains("open");
    button.parentElement.classList.toggle("open", open);
    button.setAttribute("aria-expanded", String(open));
  });
})();
