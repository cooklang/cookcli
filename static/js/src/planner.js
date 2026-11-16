// Changing a meal plan from its calendar (templates/menu.html): add a recipe
// to a day's meal with the recipe picker, and remove, move or copy a line
// from its menu or by dragging it. Every change goes to
// `POST /api/plans/{path}`, which edits the `.menu` text on the server and
// refuses a change made to an older version than the one on disk; the page
// then reloads to show the plan as the server now reads it.

import { initRecipePicker } from "./picker.js";

// Where focus goes, and what is announced, once the page has reloaded.
const AFTER_RELOAD = "cookcli-plan-after-reload";

function encodePath(path) {
  return path.split("/").map(encodeURIComponent).join("/");
}

export function initPlanner(root, { prefix = "", path, version, servings, strings }) {
  const picker = initRecipePicker(document.getElementById("recipe-picker"), { prefix });
  const menu = document.getElementById("plan-line-menu");
  const target = document.getElementById("plan-target");
  const targetForm = target.querySelector("form");
  const targetHeading = document.getElementById("plan-target-heading");
  const targetDay = document.getElementById("plan-target-day");
  const targetMeal = document.getElementById("plan-target-meal");
  const targetConfirm = targetForm.querySelector('button[type="submit"]');
  const targetCancel = targetForm.querySelector("[data-plan-cancel]");
  const error = document.getElementById("plan-error");
  const status = document.getElementById("plan-status");
  let busy = false;

  // --- Talking to the server ---

  function showError(message) {
    error.querySelector("p").textContent = message;
    error.hidden = false;
  }

  // Sends one change; on success reloads, focusing `focus` afterwards.
  async function change(op, focus) {
    if (busy) return;
    busy = true;
    try {
      const response = await fetch(`${prefix}/api/plans/${encodePath(path)}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ version, ...op })
      });
      if (response.ok) {
        try {
          sessionStorage.setItem(AFTER_RELOAD, JSON.stringify(focus));
        } catch {
          // Private windows may refuse; the change is saved either way.
        }
        location.reload();
        return;
      }
      if (response.status === 409) {
        showError(strings.changed);
      } else {
        const body = await response.json().catch(() => ({}));
        showError(body.error ? `${strings.failed} ${body.error}` : strings.failed);
      }
    } catch {
      showError(strings.failed);
    }
    busy = false;
  }

  error.querySelector("[data-plan-reload]").addEventListener("click", () => location.reload());

  // --- Where a line or meal is ---

  function slotOf(el) {
    const meal = el.closest(".plan-meal[data-meal]");
    const day = el.closest(".plan-day[data-date]");
    return meal && day ? { date: day.dataset.date, meal: meal.dataset.meal } : null;
  }

  function lineOf(el) {
    const line = el.closest(".plan-line");
    const slot = line && slotOf(line);
    return slot && { ...slot, index: Number(line.dataset.index), text: line.dataset.text };
  }

  function addButton({ date, meal }) {
    return [...root.querySelectorAll(`.plan-day[data-date="${date}"] .plan-meal[data-meal]`)]
      .find(el => el.dataset.meal === meal)
      ?.querySelector("[data-plan-add]");
  }

  // --- Add ---

  root.addEventListener("click", async event => {
    const button = event.target.closest("[data-plan-add]");
    if (!button) return;
    const slot = slotOf(button);
    if (!slot) return;
    const choice = await picker.open(servings ? { servings } : {});
    if (!choice) return;
    const amount = Number(choice.servings);
    await change(
      {
        op: "add",
        ...slot,
        recipe: choice.path,
        servings: choice.servings && amount > 0 ? amount : null
      },
      slot
    );
  });

  // --- The line menu: Move to…, Copy to…, Remove ---

  const items = [...menu.querySelectorAll('[role="menuitem"]')];
  let menuButton = null;

  function closeMenu({ focusButton = true } = {}) {
    if (!menuButton) return;
    menu.hidden = true;
    menuButton.setAttribute("aria-expanded", "false");
    if (focusButton) menuButton.focus();
    menuButton = null;
  }

  function openMenu(button) {
    if (menuButton === button) {
      closeMenu();
      return;
    }
    closeMenu({ focusButton: false });
    menuButton = button;
    button.setAttribute("aria-expanded", "true");
    menu.hidden = false;
    const rect = button.getBoundingClientRect();
    const width = menu.offsetWidth;
    const left = Math.max(8, Math.min(rect.right - width, window.innerWidth - width - 8));
    menu.style.top = `${rect.bottom + window.scrollY + 4}px`;
    menu.style.left = `${left + window.scrollX}px`;
    items[0].focus();
  }

  root.addEventListener("click", event => {
    const button = event.target.closest(".plan-line-button");
    if (button) openMenu(button);
  });

  menu.addEventListener("keydown", event => {
    const index = items.indexOf(document.activeElement);
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        items[(index + 1) % items.length].focus();
        break;
      case "ArrowUp":
        event.preventDefault();
        items[(index - 1 + items.length) % items.length].focus();
        break;
      case "Home":
        event.preventDefault();
        items[0].focus();
        break;
      case "End":
        event.preventDefault();
        items[items.length - 1].focus();
        break;
      case "Escape":
        event.preventDefault();
        closeMenu();
        break;
      case "Tab":
        closeMenu({ focusButton: false });
        break;
    }
  });

  document.addEventListener("click", event => {
    if (menuButton && !menu.contains(event.target) && !menuButton.contains(event.target)) {
      closeMenu({ focusButton: false });
    }
  });

  menu.addEventListener("click", event => {
    const item = event.target.closest("[data-plan-action]");
    if (!item || !menuButton) return;
    const button = menuButton;
    const line = lineOf(button);
    closeMenu({ focusButton: false });
    if (!line) return;
    if (item.dataset.planAction === "remove") {
      change({ op: "remove", ...line }, { date: line.date, meal: line.meal });
    } else {
      openTarget(item.dataset.planAction, line, button);
    }
  });

  // --- Move / Copy dialog ---

  let pending = null;

  function openTarget(op, line, returnTo) {
    pending = { op, line, returnTo };
    targetHeading.textContent = targetHeading.dataset[op];
    targetConfirm.textContent = targetConfirm.dataset[op];
    targetDay.value = line.date;
    if ([...targetMeal.options].some(option => option.value === line.meal)) {
      targetMeal.value = line.meal;
    }
    target.classList.remove("hidden");
    target.classList.add("flex");
    targetDay.focus();
  }

  function closeTarget() {
    target.classList.add("hidden");
    target.classList.remove("flex");
    const returnTo = pending?.returnTo;
    pending = null;
    if (returnTo?.isConnected) returnTo.focus();
  }

  targetForm.addEventListener("submit", event => {
    event.preventDefault();
    if (!pending) return;
    const { op, line } = pending;
    const to = { date: targetDay.value, meal: targetMeal.value };
    target.classList.add("hidden");
    target.classList.remove("flex");
    pending = null;
    change({ op, ...line, to }, to);
  });
  targetCancel.addEventListener("click", closeTarget);
  target.addEventListener("click", event => {
    if (event.target === target) closeTarget();
  });
  target.addEventListener("keydown", event => {
    if (event.key === "Escape") {
      event.preventDefault();
      closeTarget();
    } else if (event.key === "Tab") {
      // Keep focus inside the dialog.
      const focusable = [targetDay, targetMeal, targetCancel, targetConfirm];
      const index = focusable.indexOf(document.activeElement);
      event.preventDefault();
      focusable[(index + (event.shiftKey ? -1 : 1) + focusable.length) % focusable.length].focus();
    }
  });

  // --- Drag and drop: a line onto another meal; Alt, Ctrl or ⌘ copies ---

  let dragged = null;
  const copying = event => event.altKey || event.ctrlKey || event.metaKey;

  root.addEventListener("dragstart", event => {
    const line = lineOf(event.target);
    if (!line) return;
    dragged = line;
    event.dataTransfer.effectAllowed = "copyMove";
    event.dataTransfer.setData("text/plain", line.text);
  });

  root.addEventListener("dragend", () => {
    dragged = null;
    root.querySelectorAll(".plan-drop").forEach(el => el.classList.remove("plan-drop"));
  });

  root.addEventListener("dragover", event => {
    const meal = dragged && event.target.closest(".plan-meal[data-meal]");
    if (!meal || !slotOf(meal)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = copying(event) ? "copy" : "move";
    root.querySelectorAll(".plan-drop").forEach(el => el !== meal && el.classList.remove("plan-drop"));
    meal.classList.add("plan-drop");
  });

  root.addEventListener("dragleave", event => {
    const meal = event.target.closest(".plan-meal");
    if (meal && !meal.contains(event.relatedTarget)) meal.classList.remove("plan-drop");
  });

  root.addEventListener("drop", event => {
    const meal = event.target.closest(".plan-meal[data-meal]");
    const to = meal && slotOf(meal);
    const line = dragged;
    if (!to || !line) return;
    event.preventDefault();
    meal.classList.remove("plan-drop");
    const op = copying(event) ? "copy" : "move";
    if (op === "move" && to.date === line.date && to.meal === line.meal) return;
    change({ op, ...line, to }, to);
  });

  // --- After a reload: back to where the change was made ---

  let after = null;
  try {
    after = JSON.parse(sessionStorage.getItem(AFTER_RELOAD) || "null");
    sessionStorage.removeItem(AFTER_RELOAD);
  } catch {
    after = null;
  }
  if (after) {
    const button = addButton(after);
    if (button) {
      button.focus({ preventScroll: true });
      button.closest(".plan-day")?.scrollIntoView({ block: "nearest" });
    }
    // Set after focus so the status is read, not drowned by the focus move.
    setTimeout(() => {
      status.textContent = strings.saved;
    }, 100);
  }
}
