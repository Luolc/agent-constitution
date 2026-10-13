// The popup on the counts under the title: open while the pointer is over
// the info button or the popup, and toggled by a click or a tap on the
// button (Enter or Space from the keyboard). Escape closes it and keeps it
// closed under the pointer until the pointer leaves; a click elsewhere
// closes it. Inlined in <head>, so it listens on the document.
(function () {
  function set(info, open) {
    info.classList.toggle("open", open);
    info.querySelector(".count-button").setAttribute("aria-expanded", String(open));
  }
  document.addEventListener("click", function (event) {
    var info = document.querySelector(".count-info");
    if (!info) return;
    if (event.target.closest(".count-button")) {
      set(info, !info.classList.contains("open"));
      info.classList.remove("dismissed");
    } else if (!event.target.closest(".count-popup")) {
      set(info, false);
    }
  });
  document.addEventListener("keydown", function (event) {
    var info = document.querySelector(".count-info");
    if (event.key !== "Escape" || !info) return;
    var inside = info.contains(document.activeElement);
    set(info, false);
    info.classList.add("dismissed");
    if (inside) info.querySelector(".count-button").focus();
  });
  document.addEventListener("mouseout", function (event) {
    var info = document.querySelector(".count-info");
    if (info && info.contains(event.target) && !info.contains(event.relatedTarget)) {
      info.classList.remove("dismissed");
    }
  });
})();
