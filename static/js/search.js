(function () {
  var prefix = window.__PREFIX__ || ".";
  var input = document.getElementById("search-input");
  var results = document.getElementById("search-results");
  if (!input || !results) return;

  var index = null;
  var selectedIndex = -1;

  function loadIndex() {
    if (index !== null) return Promise.resolve(index);
    return fetch(prefix + "/static/search-index.json")
      .then(function (r) { return r.json(); })
      .then(function (data) {
        index = data;
        return data;
      })
      .catch(function (e) {
        console.error("search-index load failed", e);
        index = [];
        return index;
      });
  }

  function score(entry, q) {
    var ql = q.toLowerCase();
    if (entry.title.toLowerCase().indexOf(ql) !== -1) return 3;
    if (entry.tags.some(function (t) { return t.toLowerCase().indexOf(ql) !== -1; })) return 2;
    if (entry.ingredients.some(function (i) { return i.toLowerCase().indexOf(ql) !== -1; })) return 1;
    return 0;
  }

  function escapeHtml(s) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  }

  function render(matches) {
    if (matches.length === 0) {
      results.innerHTML = '<div class="p-4 text-gray-500 text-center">No recipes found</div>';
    } else {
      results.innerHTML = matches.map(function (m) {
        var href = prefix + "/" + m.path;
        return '<a href="' + escapeHtml(href) + '" class="search-result block px-4 py-3 hover:bg-gradient-to-r hover:from-purple-50 hover:to-pink-50 transition-colors border-b border-gray-100 last:border-b-0">' +
          '<div class="font-medium text-gray-800">' + escapeHtml(m.title) + '</div>' +
          '</a>';
      }).join("");
    }
    results.classList.remove("hidden");
  }

  function updateSearchSelection() {
    var items = results.querySelectorAll("a");
    items.forEach(function (item, i) {
      if (i === selectedIndex) {
        item.classList.add("search-selected");
        item.scrollIntoView({ block: "nearest" });
      } else {
        item.classList.remove("search-selected");
      }
    });
  }

  var timeout;
  input.addEventListener("input", function () {
    clearTimeout(timeout);
    var q = this.value.trim();
    selectedIndex = -1;
    if (q.length < 2) {
      results.classList.add("hidden");
      return;
    }
    timeout = setTimeout(function () {
      loadIndex().then(function (idx) {
        var matches = idx
          .map(function (e) { return { e: e, s: score(e, q) }; })
          .filter(function (x) { return x.s > 0; })
          .sort(function (a, b) { return b.s - a.s; })
          .slice(0, 20)
          .map(function (x) { return x.e; });
        render(matches);
      });
    }, 150);
  });

  input.addEventListener("keydown", function (e) {
    var items = results.querySelectorAll("a");
    if (items.length === 0 || results.classList.contains("hidden")) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, items.length - 1);
      updateSearchSelection();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, -1);
      updateSearchSelection();
    } else if (e.key === "Enter") {
      if (selectedIndex >= 0 && selectedIndex < items.length) {
        e.preventDefault();
        items[selectedIndex].click();
      }
    }
  });

  document.addEventListener("click", function (e) {
    if (!input.contains(e.target) && !results.contains(e.target)) {
      results.classList.add("hidden");
      selectedIndex = -1;
    }
  });
})();
