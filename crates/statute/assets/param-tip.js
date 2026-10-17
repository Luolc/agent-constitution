// The tip on a parameter's value (the public site's
// web/assets/styles/design.css, the local site's src/site.css): given the
// room from the value's start to the window's end (--tip-room), by which
// the styles keep it inside the window, and hidden by Escape until the
// pointer or the focus comes to a value again. A resize takes the room
// back, as it no longer holds; without it, the hidden tip lies before the
// value, where it widens no page.
// Inlined in <head> by both sites, so it listens on the document.
(function () {
  function place(event) {
    var value = event.target.closest && event.target.closest(".param-value");
    if (!value || !value.querySelector(".param-tip")) return;
    document.documentElement.classList.remove("tips-dismissed");
    var room = document.documentElement.clientWidth - value.getBoundingClientRect().left;
    value.style.setProperty("--tip-room", room + "px");
  }
  document.addEventListener("mouseover", place);
  document.addEventListener("focusin", place);
  document.addEventListener("keydown", function (event) {
    if (event.key === "Escape") document.documentElement.classList.add("tips-dismissed");
  });
  window.addEventListener("resize", function () {
    document.querySelectorAll(".param-value[style]").forEach(function (value) {
      value.style.removeProperty("--tip-room");
    });
  });
})();
