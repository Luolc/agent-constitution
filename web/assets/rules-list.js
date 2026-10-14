// The list of the rules (layouts/rules/section.html) at the page size the
// reader chose: 25, 50, 100 or all, from `?per=` in the address, and the
// page from `?page=`; without them, the page Hugo built. Every page holds
// every rule, so this only shows the rules of the page and hides the
// others, gives the shown group headings their ids, and makes the table
// of contents and the links to the other pages again. Choosing a size in
// the form loads its first page. Inlined in <head>; it runs before
// assets/toc-current.js, which then finds the headings shown.
(function () {
  var SIZES = ["25", "50", "100", "all"];
  document.addEventListener("DOMContentLoaded", function () {
    var list = document.querySelector(".rules-list");
    var form = document.querySelector(".rules-size");
    if (!list || !form) return;
    var items = list.querySelectorAll(".rule-item");
    var query = new URLSearchParams(location.search);
    var per = query.get("per");
    if (SIZES.indexOf(per) < 0) per = list.dataset.size;
    var size = per === "all" ? Math.max(items.length, 1) : Number(per);
    var pages = Math.max(1, Math.ceil(items.length / size));
    var page = query.has("page")
      ? Number(query.get("page"))
      : Math.floor(Number(list.dataset.first) / size) + 1;
    page = Math.min(Math.max(Math.floor(page) || 1, 1), pages);
    var first = (page - 1) * size;

    items.forEach(function (item, i) {
      item.hidden = i < first || i >= first + size;
      item.classList.toggle("first", i === first);
      item.querySelectorAll(".rules-group").forEach(function (heading) {
        var shown = !item.hidden && (i === first || !heading.classList.contains("repeat"));
        if (shown) heading.id = "group-" + heading.dataset.group;
        else heading.removeAttribute("id");
      });
    });

    var toc = document.querySelector("#TableOfContents ul");
    if (toc) {
      toc.textContent = "";
      list.querySelectorAll(".rules-group[id]").forEach(function (heading) {
        var li = document.createElement("li");
        if (heading.tagName !== "H2") li.className = "toc-sub";
        var a = document.createElement("a");
        a.href = "#" + heading.id;
        a.textContent = heading.textContent;
        li.append(a);
        toc.append(li);
      });
    }

    var pager = document.querySelector(".rules-pager");
    if (pager) {
      pager.textContent = "";
      var link = function (n, text, label, rel) {
        var a = document.createElement("a");
        a.textContent = text;
        if (n === page && !rel) a.setAttribute("aria-current", "page");
        else a.href = form.action + "?per=" + per + "&page=" + n;
        if (label) a.setAttribute("aria-label", label);
        if (rel) a.rel = rel;
        pager.append(a);
      };
      if (pages > 1) {
        if (page > 1) link(page - 1, "‹", pager.dataset.previous, "prev");
        for (var n = 1; n <= pages; n++) link(n, String(n));
        if (page < pages) link(page + 1, "›", pager.dataset.next, "next");
      }
    }

    var select = form.querySelector("select");
    select.value = per;
    select.addEventListener("change", function () { form.submit(); });
    form.hidden = false;
  });
})();
