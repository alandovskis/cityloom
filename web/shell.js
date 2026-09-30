// The parts of the sheet both pages share: the account menu (units, region,
// theme, drawing style), the details and notes sidebars, the notes tabs and
// print. Each page passes in what differs: how to announce, and what to redraw.

const $ = (id) => document.getElementById(id);
const root = document.documentElement;

export const typing = (t) => t instanceof Element && t.closest("input, select, textarea");
export const engineering = () => root.dataset.drawing === "engineering";

const remember = (key, value) => {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Storage can be blocked; the choice then lasts for this visit only.
  }
};
const recall = (key) => {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
};

// Units: the m | ft toggle. The page owns the value and redraws.
export function initUnits(onChange) {
  for (const b of document.querySelectorAll(".unit[data-unit]")) {
    b.addEventListener("click", () => {
      for (const o of document.querySelectorAll(".unit[data-unit]")) o.setAttribute("aria-pressed", String(o === b));
      onChange(b.dataset.unit);
    });
  }
}

// Region: which side of the road traffic keeps to; remembered. `apply(index)`
// tells the model and says whether it accepted; `current()` is the region id
// the model now holds.
export function initRegion({ regions, apply, current, onChange, say }) {
  const pick = $("region");
  pick.innerHTML = regions.map((r) => `<option value="${r.id}">${r.name} (${r.drive_side})</option>`).join("");
  const set = (id) => {
    const i = regions.findIndex((r) => r.id === id);
    if (i >= 0 && apply(i)) onChange();
    pick.value = current();
  };
  set(recall("cityloom-region"));
  pick.addEventListener("change", () => {
    set(pick.value);
    remember("cityloom-region", pick.value);
    const r = regions.find((x) => x.id === current());
    say(`${r.name}: traffic keeps ${r.drive_side}.`);
  });
}

// Theme: follow the system until the person picks one; the pick is remembered.
export function initTheme(say) {
  const dark = matchMedia("(prefers-color-scheme: dark)");
  const theme = () => root.dataset.theme || (dark.matches ? "dark" : "light");
  const sync = () => {
    for (const b of document.querySelectorAll(".theme")) b.setAttribute("aria-pressed", String(b.dataset.themeSet === theme()));
  };
  for (const b of document.querySelectorAll(".theme")) {
    b.addEventListener("click", () => {
      root.dataset.theme = b.dataset.themeSet;
      remember("cityloom-theme", b.dataset.themeSet);
      sync();
      say(`${b.textContent} theme.`);
    });
  }
  dark.addEventListener("change", sync);
  sync();
}

// Drawing style: the same drawing as plain engineering line work; remembered.
export function initDrawingStyle(say, onChange) {
  const sync = () => {
    for (const b of document.querySelectorAll(".drawing-mode")) {
      b.setAttribute("aria-pressed", String((b.dataset.drawingSet === "engineering") === engineering()));
    }
  };
  for (const b of document.querySelectorAll(".drawing-mode")) {
    b.addEventListener("click", () => {
      if (b.dataset.drawingSet === "engineering") root.dataset.drawing = "engineering";
      else delete root.dataset.drawing;
      remember("cityloom-drawing", b.dataset.drawingSet);
      sync();
      onChange();
      say(`${b.textContent} drawing.`);
    });
  }
  sync();
}

// Account menu: a disclosure from the avatar that holds the settings.
export function initAccountMenu() {
  const btn = $("account-btn");
  const menu = $("account-menu");
  const setOpen = (open, refocus = false) => {
    menu.hidden = !open;
    btn.setAttribute("aria-expanded", String(open));
    if (!open && refocus) btn.focus();
  };
  btn.addEventListener("click", () => setOpen(menu.hidden));
  document.addEventListener("pointerdown", (e) => {
    if (!menu.hidden && !menu.contains(e.target) && !btn.contains(e.target)) setOpen(false);
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !menu.hidden) {
      e.stopPropagation();
      setOpen(false, true);
    }
  });
  menu.addEventListener("focusout", (e) => {
    if (e.relatedTarget && !menu.contains(e.relatedTarget) && e.relatedTarget !== btn) setOpen(false);
  });
}

// A sidebar that collapses from a header button or a key; remembered.
function initSidebar({ dataKey, storeKey, button, label, key, shown, hidden, saidShown, saidHidden, say }) {
  const btn = $(button);
  const text = $(label);
  const sync = () => {
    const open = root.dataset[dataKey] !== "closed";
    btn.setAttribute("aria-expanded", String(open));
    text.textContent = open ? hidden : shown;
  };
  const toggle = () => {
    const open = root.dataset[dataKey] === "closed";
    if (open) delete root.dataset[dataKey];
    else root.dataset[dataKey] = "closed";
    remember(storeKey, open ? "open" : "closed");
    sync();
    say(open ? saidShown : saidHidden);
  };
  btn.addEventListener("click", toggle);
  document.addEventListener("keydown", (e) => {
    if (e.key !== key || e.metaKey || e.ctrlKey || e.altKey || typing(e.target)) return;
    e.preventDefault();
    toggle();
  });
  sync();
}

// Notes tabs: arrow keys move between tabs; the choice is remembered.
function initTabs() {
  const tabs = [...document.querySelectorAll(".notes .tab")];
  const show = (tab, focus) => {
    for (const t of tabs) {
      const on = t === tab;
      t.setAttribute("aria-selected", String(on));
      t.tabIndex = on ? 0 : -1;
      $(t.getAttribute("aria-controls")).hidden = !on;
    }
    if (focus) tab.focus();
    remember("cityloom-notes-tab", tab.id);
  };
  const saved = recall("cityloom-notes-tab");
  show(tabs.find((t) => t.id === saved) || tabs[0], false);
  tabs.forEach((t, i) => {
    t.addEventListener("click", () => show(t, false));
    t.addEventListener("keydown", (e) => {
      const to = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: tabs.length - 1 }[e.key];
      if (to === undefined) return;
      e.preventDefault();
      show(tabs[(to + tabs.length) % tabs.length], true);
    });
  });
}

// The panels around the drawing. `detailsWord` names what the left sidebar
// holds on this page ("piece details", "details").
export function initPanels({ say, detailsWord = "piece details" }) {
  initSidebar({
    dataKey: "inspector",
    storeKey: "cityloom-inspector",
    button: "inspector-toggle",
    label: "inspector-label",
    key: "[",
    shown: `Show ${detailsWord}`,
    hidden: `Hide ${detailsWord}`,
    saidShown: `${cap(detailsWord)} shown.`,
    saidHidden: `${cap(detailsWord)} hidden.`,
    say,
  });
  initTabs();
  initSidebar({
    dataKey: "notes",
    storeKey: "cityloom-notes",
    button: "notes-toggle",
    label: "notes-label",
    key: "]",
    shown: "Show notes",
    hidden: "Hide notes",
    saidShown: "Notes shown.",
    saidHidden: "Notes hidden.",
    say,
  });
  // Print: the print stylesheet lays the sheet out; this is only the trigger.
  $("print").addEventListener("click", () => window.print());
}

const cap = (s) => s.charAt(0).toUpperCase() + s.slice(1);
