// Recipe picker: a modal that searches the collection and resolves with the
// recipe the user chose. The toolbar's Add recipe (menus) and Recipe
// reference (recipes) buttons turn that choice into Cooklang.

// The same pacing as the search box in the page header.
const SEARCH_DELAY_MS = 300;
const MIN_QUERY_LENGTH = 2;

function normalisePath(path) {
  return String(path).replace(/\\/g, "/");
}

function isMenu(path) {
  return /\.menu$/i.test(path);
}

// `Breakfast/Easy Pancakes.cook` -> `./Breakfast/Easy Pancakes`. Menu and
// recipe references resolve from the collection root, hence the `./`.
export function referencePath(path) {
  return "./" + normalisePath(path).replace(/^\.?\/+/, "").replace(/\.cook$/i, "");
}

// Every recipe in a `GET /api/recipes` tree, as `{ name, path }` with the path
// relative to the collection root, sorted by path. The tree carries absolute
// paths; the root node's own path is the collection root.
export function flattenTree(tree) {
  const root = normalisePath(tree.path || "").replace(/\/+$/, "");
  const recipes = [];
  (function walk(node) {
    if (node.recipe && node.path) {
      const full = normalisePath(node.path);
      const path = root && full.startsWith(root + "/") ? full.slice(root.length + 1) : full;
      recipes.push({ name: node.name || path, path });
    }
    for (const child of Object.values(node.children || {})) walk(child);
  })(tree);
  return recipes.sort((a, b) => a.path.localeCompare(b.path));
}

// Wire the picker markup in `root`. `exclude` is a path (relative, with its
// extension) never offered, such as the file being edited.
export function initRecipePicker(root, { prefix = "", exclude = null } = {}) {
  const search = root.querySelector('[role="combobox"]');
  const list = root.querySelector('[role="listbox"]');
  const status = root.querySelector('[aria-live]');
  const servingsRow = root.querySelector("#recipe-picker-servings");
  const servings = servingsRow.querySelector("input");
  const cancelButton = root.querySelector("#recipe-picker-cancel");
  const insertButton = root.querySelector("#recipe-picker-insert");

  let results = [];
  let active = -1;
  let settle = null;
  let returnFocus = null;
  let searchTimer = null;
  let request = 0;
  let tree = null;
  // Whether this opening offers menus as well as recipes (see `open`).
  let offerMenus = false;

  function optionId(index) {
    return `${list.id}-option-${index}`;
  }

  function setActive(index) {
    active = index;
    list.querySelectorAll('[role="option"]').forEach((option, i) => {
      option.setAttribute("aria-selected", String(i === active));
      if (i === active) option.scrollIntoView({ block: "nearest" });
    });
    if (active >= 0) search.setAttribute("aria-activedescendant", optionId(active));
    else search.removeAttribute("aria-activedescendant");
    insertButton.disabled = active < 0;
  }

  function show(items, message) {
    results = items.filter(item => (offerMenus || !isMenu(item.path)) && item.path !== exclude);
    list.replaceChildren(...results.map((item, index) => {
      const option = document.createElement("li");
      option.id = optionId(index);
      option.setAttribute("role", "option");
      option.className = "picker-option";
      const name = document.createElement("span");
      name.className = "picker-option-name";
      name.textContent = item.name;
      const path = document.createElement("span");
      path.className = "picker-option-path";
      path.textContent = referencePath(item.path);
      option.append(name, path);
      // Keep focus in the search box, so typing carries on after a click.
      option.addEventListener("mousedown", event => event.preventDefault());
      option.addEventListener("click", () => choose(index));
      return option;
    }));
    status.textContent = message ?? (results.length ? "" : root.dataset.noResults);
    setActive(results.length ? 0 : -1);
  }

  // The whole collection, fetched once per page.
  function loadTree() {
    tree ??= fetch(`${prefix}/api/recipes`)
      .then(response => {
        if (!response.ok) throw new Error(response.statusText);
        return response.json();
      })
      .then(flattenTree)
      .catch(error => {
        tree = null;
        throw error;
      });
    return tree;
  }

  async function searchRecipes(query) {
    const response = await fetch(`${prefix}/api/search?q=${encodeURIComponent(query)}`);
    if (!response.ok) throw new Error(response.statusText);
    const hits = await response.json();
    return hits.map(hit => ({ name: hit.name, path: normalisePath(hit.path) }));
  }

  async function refresh() {
    const query = search.value.trim();
    const current = ++request;
    try {
      const items = query.length < MIN_QUERY_LENGTH ? await loadTree() : await searchRecipes(query);
      if (current === request) show(items);
    } catch {
      if (current === request) show([], root.dataset.loadFailed);
    }
  }

  function close(choice) {
    clearTimeout(searchTimer);
    request++;
    root.classList.add("hidden");
    root.classList.remove("flex");
    const done = settle;
    settle = null;
    // A choice hands focus to the editor; a cancel goes back where it was.
    if (!choice && returnFocus?.isConnected) returnFocus.focus();
    returnFocus = null;
    done?.(choice);
  }

  function choose(index) {
    const item = results[index];
    if (!item) return;
    close({
      path: item.path,
      reference: referencePath(item.path),
      servings: servingsRow.hidden ? "" : servings.value.trim()
    });
  }

  function focusable() {
    return [search, servings, cancelButton, insertButton].filter(
      el => !el.disabled && !el.closest("[hidden]")
    );
  }

  search.addEventListener("input", () => {
    clearTimeout(searchTimer);
    if (search.value.trim().length < MIN_QUERY_LENGTH) refresh();
    else searchTimer = setTimeout(refresh, SEARCH_DELAY_MS);
  });

  root.addEventListener("keydown", event => {
    switch (event.key) {
      case "Escape":
        event.preventDefault();
        event.stopPropagation();
        close(null);
        return;
      case "Enter":
        if (event.target === search || event.target === servings) {
          event.preventDefault();
          choose(active);
        }
        return;
      case "ArrowDown":
      case "ArrowUp":
        if (event.target !== search || !results.length) return;
        event.preventDefault();
        setActive(event.key === "ArrowDown"
          ? (active + 1) % results.length
          : (active - 1 + results.length) % results.length);
        return;
      case "Tab": {
        // Keep focus inside the dialog.
        const items = focusable();
        const index = items.indexOf(document.activeElement);
        const next = event.shiftKey
          ? items[(index - 1 + items.length) % items.length]
          : items[(index + 1) % items.length];
        event.preventDefault();
        next.focus();
        return;
      }
    }
  });

  root.addEventListener("click", event => {
    if (event.target === root) close(null);
  });
  cancelButton.addEventListener("click", () => close(null));
  insertButton.addEventListener("click", () => choose(active));

  return {
    // Resolves with `{ path, reference, servings }`, or null when cancelled.
    // `servings` pre-fills the servings field; leave it out to hide the
    // field, for references that take no servings. `menus` offers menus as
    // well, for a menu's meals; their reference keeps the `.menu`.
    open({ servings: initial, menus = false } = {}) {
      if (settle) close(null);
      offerMenus = menus;
      returnFocus = document.activeElement;
      search.value = "";
      servingsRow.hidden = initial === undefined;
      servings.value = initial ?? "";
      show([], "");
      root.classList.remove("hidden");
      root.classList.add("flex");
      search.focus();
      refresh();
      return new Promise(resolve => {
        settle = resolve;
      });
    }
  };
}
