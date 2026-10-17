// The tip on a parameter's value (the public site's
// web/assets/styles/design.css, the local site's src/site.css): set to
// end at the value's end where it would run out of the window, and hidden
// by Escape until the pointer or the focus comes to a value again.
// Inlined in <head> by both sites, so it listens on the document.
(function () {
  function place(event) {
    var value = event.target.closest && event.target.closest(".param-value");
    var tip = value && value.querySelector(".param-tip");
    if (!tip) return;
    document.documentElement.classList.remove("tips-dismissed");
    value.classList.toggle("tip-end", value.getBoundingClientRect().left + tip.offsetWidth > document.documentElement.clientWidth);
  }
  document.addEventListener("mouseover", place);
  document.addEventListener("focusin", place);
  document.addEventListener("keydown", function (event) {
    if (event.key === "Escape") document.documentElement.classList.add("tips-dismissed");
  });
})();
