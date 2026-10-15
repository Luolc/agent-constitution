// The list of the rules (layouts/rules/section.html) with the filter and
// the page size the reader chose, read from the address: `filter` once the
// form was sent, with the checked `status`, `uses`, `domain` and, when
// the site has more than one source, `quadrant` values, `default=on` for
// only the rules the defaults enable, `per` (25,
// 50, 100 or all) and `page`. Without them, the default filter, as the
// form's checkboxes were built, and the page Hugo built. Every page holds
// every rule, so this only shows the rules of the filter and page, and
// hides the others; shows the group headings where the group changes and
// on top of the page, with their ids; and makes the table of contents and
// the links to the other pages again. Sending the form loads the first
// page of its choice: the page size sends it at once, the filter with its
// button. Checking or unchecking a domain does the same to the domains
// under it. Inlined in <head>; it runs before assets/toc-current.js, which
// then finds the headings shown.
(function () {
  var SIZES = ["25", "50", "100", "all"];
  // The filter's checkboxes; a name the form has none of filters nothing.
  // A rule has one value of each, but may have both `uses`; the one
  // `default` box, unchecked, filters nothing.
  var NAMES = ["status", "uses", "domain", "quadrant", "default"];
  document.addEventListener("DOMContentLoaded", function () {
    var list = document.querySelector(".rules-list");
    var form = document.querySelector(".rules-controls");
    if (!list || !form) return;
    var query = new URLSearchParams(location.search);
    var boxes = function (name) {
      return Array.prototype.slice.call(form.querySelectorAll("input[name='" + name + "']"));
    };
    if (query.has("filter")) {
      NAMES.forEach(function (name) {
        var checked = query.getAll(name);
        boxes(name).forEach(function (box) { box.checked = checked.indexOf(box.value) >= 0; });
      });
    }
    var checked = function (name) {
      return boxes(name).filter(function (box) { return box.checked; }).map(function (box) { return box.value; });
    };
    var filters = NAMES.filter(function (name) {
      return boxes(name).length && (name !== "default" || checked(name).length);
    }).map(function (name) {
      return { name: name, values: checked(name) };
    });
    var matched = Array.prototype.filter.call(list.querySelectorAll(".rule-item"), function (item) {
      return filters.every(function (f) {
        return item.dataset[f.name].split(" ").some(function (v) { return f.values.indexOf(v) >= 0; });
      });
    });

    var per = query.get("per");
    if (SIZES.indexOf(per) < 0) per = list.dataset.size;
    var size = per === "all" ? Math.max(matched.length, 1) : Number(per);
    var pages = Math.max(1, Math.ceil(matched.length / size));
    var page = query.has("page")
      ? Number(query.get("page"))
      : Math.floor(Number(list.dataset.first) / size) + 1;
    page = Math.min(Math.max(Math.floor(page) || 1, 1), pages);
    var shown = matched.slice((page - 1) * size, page * size);

    list.querySelectorAll(".rule-item").forEach(function (item) { item.hidden = true; });
    list.querySelectorAll(".rules-group").forEach(function (heading) {
      heading.hidden = true;
      heading.removeAttribute("id");
    });
    var previous = [];
    shown.forEach(function (item) {
      item.hidden = false;
      var groups = item.querySelectorAll(".rules-group"), changed = false;
      groups.forEach(function (heading, j) {
        changed = changed || previous[j] !== heading.dataset.group;
        if (!changed) return;
        heading.hidden = false;
        heading.id = "group-" + heading.dataset.group;
      });
      previous = Array.prototype.map.call(groups, function (heading) { return heading.dataset.group; });
    });
    document.querySelector(".rules-none").hidden = shown.length > 0;

    var toc = document.querySelector("#TableOfContents ul");
    if (toc) {
      toc.textContent = "";
      list.querySelectorAll(".rules-group[id]").forEach(function (heading) {
        var li = document.createElement("li");
        li.style.setProperty("--level", Number(heading.tagName.slice(1)) - 2);
        var a = document.createElement("a");
        a.href = "#" + heading.id;
        a.textContent = (heading.querySelector("a") || heading).textContent;
        li.append(a);
        toc.append(li);
      });
    }

    var select = form.querySelector("select");
    select.value = per;
    var pager = document.querySelector(".rules-pager");
    if (pager) {
      pager.textContent = "";
      var link = function (n, text, label, rel) {
        var a = document.createElement("a");
        a.textContent = text;
        if (n === page && !rel) {
          a.setAttribute("aria-current", "page");
        } else {
          var data = new URLSearchParams(new FormData(form));
          data.set("page", n);
          a.href = form.action + "?" + data;
        }
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

    form.addEventListener("change", function (event) {
      var box = event.target;
      if (box === select) {
        form.submit();
      } else if (box.name === "domain") {
        var below = [box.value];
        boxes("domain").forEach(function (other) {
          if (below.indexOf(other.dataset.parent) >= 0) {
            other.checked = box.checked;
            below.push(other.value);
          }
        });
      }
    });
    form.hidden = false;
  });
})();
