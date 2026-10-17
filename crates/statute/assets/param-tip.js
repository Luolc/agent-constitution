// The tip on a parameter's value (the public site's
// web/assets/styles/design.css, the local site's src/site.css): set to
// start at the value's start where it fits in the window, and otherwise
// left at its end, where a hidden tip never widens the page; hidden by
// Escape until the pointer or the focus comes to a value again.
// Inlined in <head> by both sites, so it listens on the document.
(function () {
  function place(event) {
    var value = event.target.closest && event.target.closest(".param-value");
    var tip = value && value.querySelector(".param-tip");
    if (!tip) return;
    document.documentElement.classList.remove("tips-dismissed");
    value.classList.toggle("tip-start", value.getBoundingClientRect().left + tip.offsetWidth <= document.documentElement.clientWidth);
  }
  document.addEventListener("mouseover", place);
  document.addEventListener("focusin", place);
  document.addEventListener("keydown", function (event) {
    if (event.key === "Escape") document.documentElement.classList.add("tips-dismissed");
  });
})();
