// The table of contents marks the heading the reader is in: the last one
// above a line at 30% of the window's height, or the last heading once the
// end of the text is in view, since the last articles may be too short to
// reach the line. One observer watches the headings against the window
// extended far upward and cut at that line, so a heading intersects
// exactly while it is above the line; another watches a mark at the end of
// the text. The browser reports each crossing, a jump by a link included,
// and nothing is read on scroll. Inlined in <head>, so it starts once the
// page is parsed; a page without a table of contents is left alone.
(function () {
  if (!("IntersectionObserver" in window)) return;
  document.addEventListener("DOMContentLoaded", function () {
    var links = [], headings = [], above = [], atEnd = false;
    document.querySelectorAll("#TableOfContents a[href^='#']").forEach(function (link) {
      var heading = document.getElementById(decodeURIComponent(link.hash.slice(1)));
      if (!heading) return;
      links.push(link);
      headings.push(heading);
      above.push(false);
    });
    if (!headings.length) return;
    function mark() {
      var current = atEnd ? headings.length - 1 : above.lastIndexOf(true);
      links.forEach(function (link, i) {
        if (i === current) link.setAttribute("aria-current", "location");
        else link.removeAttribute("aria-current");
      });
    }
    var line = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        above[headings.indexOf(entry.target)] = entry.isIntersecting;
      });
      mark();
    }, { rootMargin: "100000px 0px -70% 0px" });
    headings.forEach(function (heading) { line.observe(heading); });
    var end = document.createElement("div");
    headings[0].parentElement.append(end);
    new IntersectionObserver(function (entries) {
      atEnd = entries[entries.length - 1].isIntersecting;
      mark();
    }).observe(end);
  });
})();
