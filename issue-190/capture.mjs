// Evidence capture for issue #190: drives street.html and intersection.html from the keyboard, as
// e2e/tests/scenarios.spec.ts does, and screenshots each step. Usage:
//   node capture.mjs <worktree> <outDir> <port>
import { createRequire } from "node:module";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { spawn } from "node:child_process";
import path from "node:path";

const [worktree, outDir, port] = process.argv.slice(2);
const require = createRequire(path.join(worktree, "e2e", "package.json"));
const { chromium } = require("@playwright/test");
mkdirSync(outDir, { recursive: true });

const b64 = (byte, length) => Buffer.alloc(length, byte).toString("base64");
const base = `http://127.0.0.1:${port}`;
const server = spawn(path.join(worktree, "target/e2e/cityloom_server"), [], {
  cwd: path.join(worktree, "e2e"),
  env: {
    ...Object.fromEntries(Object.entries(process.env).filter(([k]) => k !== "DATABASE_URL")),
    WEB_ROOT: "../web",
    E2E_PORT: port,
    BASE_URL: base,
    COOKIE_KEY: b64(7, 64),
    DATA_KEYS: `1:${b64(1, 32)}`,
    DATA_KEY_CURRENT: "1",
    LOOKUP_KEY: b64(3, 32),
  },
  stdio: "ignore",
});

const plateau = readFileSync(path.join(worktree, "e2e/fixtures/generated/plateau.json"));
const FIRST = {
  id: "first",
  name: "First project",
  area: "area=45.5261,-73.5978&name=Plateau%20Mont-Royal%2C%20Montr%C3%A9al",
  created_at: "1970-01-01T00:00:00.000Z",
};
const log = [];
let n = 0;

async function waitForServer() {
  for (let i = 0; i < 100; i++) {
    try {
      if ((await fetch(base + "/index.html")).ok) return;
    } catch {}
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error("server did not start");
}

async function newPage(browser, lang) {
  const context = await browser.newContext({ baseURL: base, locale: "en-CA", viewport: { width: 1440, height: 900 } });
  await context.addInitScript(
    ([first, lang]) => {
      if (localStorage.getItem("cityloom-lang") === null) localStorage.setItem("cityloom-lang", lang);
      if (localStorage.getItem("cityloom-projects") === null) {
        localStorage.setItem("cityloom-projects", JSON.stringify({ version: 1, projects: [first] }));
        localStorage.setItem("cityloom-project", first.id);
      }
    },
    [FIRST, lang],
  );
  const page = await context.newPage();
  await page.route("**/data/metro/index.json", (r) =>
    r.fulfill({ json: { lon0: -73.6078, lat0: 45.5161, dlon: 0.0257, dlat: 0.018, tiles: ["0_0"] } }),
  );
  await page.route("**/data/metro/0_0.json", (r) => r.fulfill({ contentType: "application/json", body: plateau }));
  return page;
}

async function hrefs(page) {
  await page.goto("/map.html");
  await page.waitForFunction(
    () => {
      const m = window.cityloomMap;
      if (!m || !m.getSource("places") || !m.areTilesLoaded()) return false;
      const hots = m.querySourceFeatures("places").map((f) => String(f.properties.hot));
      return hots.some((h) => h.startsWith("j-")) && hots.some((h) => h.startsWith("s-"));
    },
    undefined,
    { timeout: 20_000 },
  );
  return page.evaluate(() => {
    const hots = new Set(window.cityloomMap.querySourceFeatures("places").map((f) => f.properties.hot));
    const pick = (k, p, q) =>
      [...hots]
        .filter((h) => h.startsWith(k + "-"))
        .map((h) => Number(h.slice(2)))
        .sort((a, b) => a - b)
        .map((u) => `${p}?${q}=${u}`);
    return { street: pick("s", "street.html", "street")[0], junction: pick("j", "intersection.html", "junction")[0] };
  });
}

/** One evidence step: a screenshot of the scenario panel's area plus what the live region and panel say. */
async function shot(page, caption) {
  n += 1;
  const file = `${String(n).padStart(2, "0")}.png`;
  const state = await page.evaluate(() => ({
    live: document.querySelector("#live")?.textContent ?? "",
    mode: document.querySelector("#scenario-mode")?.textContent ?? "",
    options: [...document.querySelectorAll("#scenario-pick option")].map((o) => (o.selected ? "▶ " : "") + o.text),
    refusal: document.querySelector("#scenario-refusal")?.textContent ?? "",
    focused: document.activeElement?.id || document.activeElement?.tagName,
  }));
  const slot = page.locator("#scenario-slot");
  await slot.scrollIntoViewIfNeeded();
  await page.screenshot({ path: path.join(outDir, file) });
  log.push({ step: n, caption: `${n}. ${caption}`, file, state });
  console.log(`${n}. ${caption}\n   ${JSON.stringify(state)}`);
}

async function openActions(page) {
  const details = page.locator("details.scenario-manage");
  if (await details.evaluate((d) => d.open)) return;
  await details.locator("summary").focus();
  await page.keyboard.press("Enter");
}
async function act(page, id, name) {
  await openActions(page);
  if (name !== undefined) {
    await page.locator("#scenario-name").focus();
    await page.keyboard.press("ControlOrMeta+a");
    await page.keyboard.type(name);
  }
  await page.locator(`#${id}`).focus();
  await page.keyboard.press("Enter");
  await page.waitForTimeout(150);
}
async function show(page, label) {
  await page.locator("#scenario-pick").focus();
  if (label === null) await page.locator("#scenario-pick").selectOption("");
  else await page.locator("#scenario-pick").selectOption({ label });
  await page.waitForTimeout(150);
}
async function firstPieceWidth(page) {
  await page.locator("#wrap").focus();
  await page.keyboard.press("Escape");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  await page.waitForTimeout(100);
  return page.locator("#inspector input").first().inputValue();
}
const kept = (page) => page.evaluate(() => JSON.parse(localStorage.getItem("cityloom-edits@first") ?? "null"));
/** The paths at which two JSON values differ, with both values (the junction's edit, read from the kept record). */
function diff(a, b, at = "") {
  if (JSON.stringify(a) === JSON.stringify(b)) return [];
  if (a && b && typeof a === "object" && typeof b === "object") {
    const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
    return [...keys].flatMap((k) => diff(a[k], b[k], `${at}.${k}`));
  }
  return [`${at}: ${JSON.stringify(a)} → ${JSON.stringify(b)}`];
}
/** Waits (up to 5 s) until the kept scenario `name` differs from the baseline (`differs`) or not, and returns the differences. */
async function settled(page, name, differs) {
  let d = [];
  for (let i = 0; i < 50; i++) {
    const r = Object.values((await kept(page)).junctions ?? {})[0];
    const sc = r?.scenarios?.find((s) => s.name === name);
    if (sc) {
      d = diff(r.state, sc.design);
      if ((d.length > 0) === differs) return d;
    }
    await page.waitForTimeout(100);
  }
  return d;
}
const junctionRecord = async (page) => Object.values((await kept(page)).junctions)[0];
const designNow = async (page, name) => {
  const r = await junctionRecord(page);
  return name ? r.scenarios.find((s) => s.name === name).design : r.state;
};

try {
  await waitForServer();
  const browser = await chromium.launch();

  // Street, English
  let page = await newPage(browser, "en");
  const places = await hrefs(page);
  await page.goto("/" + places.street);
  await page.locator("#scenario-pick").waitFor();
  const baseline = await firstPieceWidth(page);
  await shot(page, `Open a street (${places.street}): it opens on the baseline; first piece ${baseline} m wide`);
  await act(page, "scenario-new", "Wider bike lane");
  await shot(page, "Type “Wider bike lane” and press New from baseline (keyboard only): the scenario is made and shown, the live region announces it");
  await page.locator("#wrap").focus();
  await page.keyboard.press("Escape");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("+");
  const wider = await firstPieceWidth(page);
  const undo1 = await page.locator("#undo").isEnabled();
  await shot(page, `Widen the first piece with + inside “Wider bike lane”: now ${wider} m (baseline ${baseline} m); Undo enabled: ${undo1}`);
  await page.locator("#undo").focus();
  await page.keyboard.press("Enter");
  const undone = await firstPieceWidth(page);
  await shot(page, `Undo inside the scenario: first piece back to ${undone} m`);
  await page.locator("#redo").focus();
  await page.keyboard.press("Enter");
  const redone = await firstPieceWidth(page);
  await shot(page, `Redo inside the scenario: first piece ${redone} m again`);
  await act(page, "scenario-new", "Parking kept");
  await shot(page, "Make “Parking kept” from the baseline: shown and announced");
  const parking = await firstPieceWidth(page);
  await shot(page, `In “Parking kept” the first piece is ${parking} m: the baseline's width, not the other scenario's`);
  await show(page, "Wider bike lane");
  await shot(page, `Show “Wider bike lane” with the picker: announced; Undo enabled right after the switch: ${await page.locator("#undo").isEnabled()}`);
  const back = await firstPieceWidth(page);
  await shot(page, `In “Wider bike lane” the first piece is ${back} m: its edit is kept`);
  await show(page, null);
  await shot(page, "Show the baseline with the picker: announced");
  const base2 = await firstPieceWidth(page);
  await shot(page, `On the baseline the first piece is ${base2} m: untouched by the scenario edit`);
  await page.reload();
  await page.locator("#scenario-pick").waitFor();
  await page.waitForTimeout(500);
  const afterReload = await firstPieceWidth(page);
  const rec = Object.values((await kept(page)).streets)[0];
  await shot(page, `Reload: the page opens on the baseline (${afterReload} m) and lists both scenarios; the kept record holds ${JSON.stringify(rec.scenarios.map((s) => s.name))}`);
  await show(page, "Wider bike lane");
  const afterReloadWider = await firstPieceWidth(page);
  await shot(page, `After the reload, “Wider bike lane” still has ${afterReloadWider} m; Undo enabled: ${await page.locator("#undo").isEnabled()}`);
  await page.reload();
  await page.locator("#scenario-pick").waitFor();
  await page.waitForTimeout(500);
  const reopened = await firstPieceWidth(page);
  const recKeys = Object.keys(Object.values((await kept(page)).streets)[0]);
  await shot(page, `Reload while “Wider bike lane” is shown: the page opens on the baseline (first piece ${reopened} m); the kept street record's keys are ${JSON.stringify(recKeys)}, with no active-scenario field`);
  await show(page, "Wider bike lane");
  await act(page, "scenario-rename", "Wider lane");
  await shot(page, "Rename the shown scenario to “Wider lane”: renamed in the picker and announced");
  await act(page, "scenario-duplicate", "Wider lane, parking kept");
  await shot(page, "Duplicate it as “Wider lane, parking kept”: the copy is shown and announced");
  await act(page, "scenario-delete");
  await shot(page, "Press Delete: it asks once, the button now reads Confirm delete");
  await page.keyboard.press("Enter");
  await page.waitForTimeout(150);
  await shot(page, "Press Confirm delete: the copy is deleted, the baseline is shown, announced");
  const delDisabled = await page.locator("#scenario-delete").isDisabled();
  const renDisabled = await page.locator("#scenario-rename").isDisabled();
  await shot(page, `On the baseline, Delete disabled: ${delDisabled}, Rename disabled: ${renDisabled}`);
  await act(page, "scenario-new", "PARKING kept");
  await shot(page, "In English, try “PARKING kept” (taken, case ignored): refused under the field and in the live region");
  await act(page, "scenario-new", "  ");
  await shot(page, "In English, try a blank name: refused");
  await act(page, "scenario-new", "y".repeat(61));
  await shot(page, "In English, try a 61-character name: refused, name too long");
  for (let k = 3; k <= 8; k++) await act(page, "scenario-new", `Option ${k}`);
  const count = await page.locator("#scenario-pick option").count();
  await shot(page, `Make scenarios up to the limit: the picker lists the baseline and ${count - 1} scenarios`);
  await act(page, "scenario-new", "Option 9");
  await shot(page, "In English, try a ninth scenario: refused, the place is full");
  await page.context().close();

  // Street, French refusals
  page = await newPage(browser, "fr-CA");
  await page.goto("/" + places.street);
  await page.locator("#scenario-pick").waitFor();
  await act(page, "scenario-new", "Parking kept");
  await shot(page, "In fr-CA, make “Parking kept”: announced in French");
  await act(page, "scenario-new", "parking KEPT");
  await shot(page, "In fr-CA, try “parking KEPT”: refused (name taken, case ignored) under the field and in the live region, aria-invalid set");
  await act(page, "scenario-new", " ");
  await shot(page, "In fr-CA, try a blank name: refused, “Donnez un nom au scénario.”");
  await act(page, "scenario-new", "x".repeat(200));
  await shot(page, "In fr-CA, try a 200-character name: refused, name too long");
  for (let k = 2; k <= 8; k++) await act(page, "scenario-new", `Option ${k}`);
  await act(page, "scenario-new", "Option 9");
  await shot(page, "In fr-CA, try a ninth scenario: refused, the place is full");
  await page.context().close();

  // Junction, English
  page = await newPage(browser, "en");
  await page.goto("/" + places.junction);
  await page.locator("#scenario-pick").waitFor();
  await shot(page, `Open a junction (${places.junction}): it opens on the baseline`);
  await act(page, "scenario-new", "Wider bike lane");
  await shot(page, "Make “Wider bike lane” on the junction (keyboard only): shown and announced");
  await page.locator("#wrap").focus();
  await page.keyboard.press("Escape");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Shift+ArrowRight");
  if (!(await page.locator("#undo").isEnabled())) await page.keyboard.press("Shift+ArrowLeft");
  const edit = await settled(page, "Wider bike lane", true);
  await shot(page, `Turn the selected street with Shift+Arrow inside the scenario: Undo enabled: ${await page.locator("#undo").isEnabled()}; the scenario now differs from the baseline at ${edit.join("; ")}`);
  await page.locator("#undo").focus();
  await page.keyboard.press("Enter");
  const afterUndo = await settled(page, "Wider bike lane", false);
  await shot(page, `Undo inside the junction scenario: differences from the baseline left: ${afterUndo.length ? afterUndo.join("; ") : "none"}`);
  await page.locator("#redo").focus();
  await page.keyboard.press("Enter");
  const afterRedo = await settled(page, "Wider bike lane", true);
  await shot(page, `Redo: the scenario differs from the baseline again at ${afterRedo.join("; ")}`);
  await act(page, "scenario-new", "Parking kept");
  await shot(page, `Make “Parking kept”: shown and announced; Undo enabled: ${await page.locator("#undo").isEnabled()}`);
  await show(page, "Wider bike lane");
  await shot(page, `Show “Wider bike lane” with the picker: announced; Undo enabled: ${await page.locator("#undo").isEnabled()}; the turned street is still turned: ${diff(await designNow(page, null), await designNow(page, "Wider bike lane")).length > 0}`);
  await show(page, null);
  await shot(page, "Show the baseline with the picker: announced");
  const baseBefore = JSON.stringify(await designNow(page, null));
  await page.reload();
  await page.locator("#scenario-pick").waitFor();
  await page.waitForTimeout(500);
  const j = await junctionRecord(page);
  const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
  await shot(page, `Reload the junction: baseline shown and unchanged by the reload: ${JSON.stringify(j.state) === baseBefore}; both scenarios listed; “Parking kept” equals the baseline: ${same(j.scenarios[1].design, j.state)}`);
  await show(page, "Wider bike lane");
  await shot(page, `After the reload, show “Wider bike lane”: announced; its edit is kept: ${diff(j.state, j.scenarios[0].design).join("; ")}`);
  await page.reload();
  await page.locator("#scenario-pick").waitFor();
  await page.waitForTimeout(500);
  const jKeys = Object.keys(await junctionRecord(page));
  await shot(page, `Reload while “Wider bike lane” is shown: the junction opens on the baseline; the kept junction record's keys are ${JSON.stringify(jKeys)}, with no active-scenario field`);
  await show(page, "Wider bike lane");
  await act(page, "scenario-rename", "Turned arm");
  await shot(page, "Rename it to “Turned arm” on the junction: renamed in the picker and announced");
  await act(page, "scenario-duplicate", "Turned arm, copy");
  await shot(page, "Duplicate it as “Turned arm, copy”: the copy is shown and announced");
  await act(page, "scenario-new", "parking KEPT");
  await shot(page, "Try “parking KEPT” on the junction: refused under the field and in the live region");
  await act(page, "scenario-delete");
  await shot(page, "Press Delete on the junction: it asks once, the button reads Confirm delete");
  await page.keyboard.press("Enter");
  await page.waitForTimeout(150);
  await shot(page, "Press Confirm delete: the copy is deleted, the baseline is shown, announced");
  await page.context().close();

  // Start over on a street scenario: the baseline is edited first, so "as imported" and "the baseline" differ.
  page = await newPage(browser, "en");
  await page.goto("/" + places.street);
  await page.locator("#scenario-pick").waitFor();
  const imported = await firstPieceWidth(page);
  await page.locator("#wrap").focus();
  await page.keyboard.press("Escape");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("+");
  const edited = await firstPieceWidth(page);
  await shot(page, `Start over, street: on the baseline, widen the first piece from ${imported} m (as imported) to ${edited} m`);
  await act(page, "scenario-new", "Wider still");
  await page.locator("#wrap").focus();
  await page.keyboard.press("Escape");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("+");
  const still = await firstPieceWidth(page);
  await act(page, "scenario-new", "Untouched");
  await show(page, "Wider still");
  await shot(page, `Make “Wider still” from the baseline and widen it to ${still} m, then make “Untouched” (${edited} m) and show “Wider still” again`);
  await page.locator("#reset").focus();
  await page.keyboard.press("Enter");
  await page.waitForTimeout(150);
  await shot(page, "Press Start over in “Wider still”: the live region announces it");
  const reset = await firstPieceWidth(page);
  await shot(page, `After the Start over, the first piece of “Wider still” is ${reset} m: the street as imported`);
  await page.locator("#undo").focus();
  await page.keyboard.press("Enter");
  const undoneReset = await firstPieceWidth(page);
  await shot(page, `Undo the Start over: “Wider still” is back to ${undoneReset} m`);
  await page.locator("#reset").focus();
  await page.keyboard.press("Enter");
  await page.waitForTimeout(150);
  const scenarioAfterReset = await firstPieceWidth(page);
  await shot(page, `Start over in “Wider still” again: its first piece is ${scenarioAfterReset} m (as imported), shown in the inspector`);
  await show(page, null);
  const baseAfterReset = await firstPieceWidth(page);
  await shot(page, `Then show the baseline: its first piece is still ${baseAfterReset} m (the baseline's own edit), shown in the inspector`);
  await show(page, "Untouched");
  const untouched = await firstPieceWidth(page);
  await shot(page, `Then show “Untouched”: its first piece is still ${untouched} m, shown in the inspector; only the design shown was started over`);
  await page.context().close();

  // A scenario deleted in another tab of the same browser while this one shows it: the Missing refusal.
  const ctx = (await newPage(browser, "en")).context();
  const tabA = ctx.pages()[0];
  await tabA.goto("/" + places.street);
  await tabA.locator("#scenario-pick").waitFor();
  await act(tabA, "scenario-new", "Doomed");
  await shot(tabA, "Missing, tab A: make and show “Doomed”");
  const tabB = await ctx.newPage();
  await tabB.route("**/data/metro/index.json", (r) =>
    r.fulfill({ json: { lon0: -73.6078, lat0: 45.5161, dlon: 0.0257, dlat: 0.018, tiles: ["0_0"] } }),
  );
  await tabB.route("**/data/metro/0_0.json", (r) => r.fulfill({ contentType: "application/json", body: plateau }));
  await tabB.goto("/" + places.street);
  await tabB.locator("#scenario-pick").waitFor();
  await show(tabB, "Doomed");
  await act(tabB, "scenario-delete");
  await tabB.keyboard.press("Enter");
  await tabB.waitForTimeout(300);
  await shot(tabB, "Missing, tab B: show “Doomed” and delete it, confirmed");
  await tabA.bringToFront();
  await tabA.waitForTimeout(500);
  await shot(tabA, "Missing, back in tab A without reloading: what it shows after the other tab deleted “Doomed”");
  await act(tabA, "scenario-rename", "Doomed again");
  await shot(tabA, "Missing, tab A: try to rename the shown design to “Doomed again”");
  await ctx.close();

  // The same in fr-CA: the Missing refusal in French.
  const ctxFr = (await newPage(browser, "fr-CA")).context();
  const frA = ctxFr.pages()[0];
  await frA.goto("/" + places.street);
  await frA.locator("#scenario-pick").waitFor();
  await act(frA, "scenario-new", "Condamné");
  await shot(frA, "Missing en fr-CA, onglet A : créer et afficher « Condamné »");
  const frB = await ctxFr.newPage();
  await frB.route("**/data/metro/index.json", (r) =>
    r.fulfill({ json: { lon0: -73.6078, lat0: 45.5161, dlon: 0.0257, dlat: 0.018, tiles: ["0_0"] } }),
  );
  await frB.route("**/data/metro/0_0.json", (r) => r.fulfill({ contentType: "application/json", body: plateau }));
  await frB.goto("/" + places.street);
  await frB.locator("#scenario-pick").waitFor();
  await show(frB, "Condamné");
  await act(frB, "scenario-delete");
  await frB.keyboard.press("Enter");
  await frB.waitForTimeout(300);
  await shot(frB, "Missing en fr-CA, onglet B : afficher « Condamné » et le supprimer, confirmé");
  await frA.bringToFront();
  await frA.waitForTimeout(500);
  await act(frA, "scenario-rename", "Condamné encore");
  await shot(frA, "Missing en fr-CA, onglet A : renommer la conception affichée est refusé, sous le champ et dans la zone d’annonces");
  await ctxFr.close();

  // fr-CA: every action and announcement on the street
  page = await newPage(browser, "fr-CA");
  await page.goto("/" + places.street);
  await page.locator("#scenario-pick").waitFor();
  await act(page, "scenario-new", "Voie élargie");
  await shot(page, "fr-CA, rue : créer « Voie élargie », annoncé");
  await show(page, null);
  await shot(page, "fr-CA, rue : afficher la référence avec le menu, annoncé");
  await show(page, "Voie élargie");
  await shot(page, "fr-CA, rue : afficher « Voie élargie » avec le menu, annoncé");
  await act(page, "scenario-rename", "Voie cyclable large");
  await shot(page, "fr-CA, rue : renommer en « Voie cyclable large », annoncé");
  await act(page, "scenario-duplicate", "Voie cyclable large, copie");
  await shot(page, "fr-CA, rue : dupliquer en « Voie cyclable large, copie », annoncé");
  await act(page, "scenario-delete");
  await shot(page, "fr-CA, rue : Supprimer demande une confirmation, annoncée");
  await page.keyboard.press("Enter");
  await page.waitForTimeout(150);
  await shot(page, "fr-CA, rue : confirmer la suppression, annoncée, la référence est affichée");
  await page.context().close();

  // fr-CA: every action and announcement on the junction
  page = await newPage(browser, "fr-CA");
  await page.goto("/" + places.junction);
  await page.locator("#scenario-pick").waitFor();
  await shot(page, "fr-CA, intersection : la page s’ouvre sur la référence");
  await act(page, "scenario-new", "Virage protégé");
  await shot(page, "fr-CA, intersection : créer « Virage protégé », annoncé");
  await show(page, null);
  await shot(page, "fr-CA, intersection : afficher la référence avec le menu, annoncé");
  await show(page, "Virage protégé");
  await shot(page, "fr-CA, intersection : afficher « Virage protégé » avec le menu, annoncé");
  await act(page, "scenario-rename", "Virage sûr");
  await shot(page, "fr-CA, intersection : renommer en « Virage sûr », annoncé");
  await act(page, "scenario-duplicate", "Virage sûr, copie");
  await shot(page, "fr-CA, intersection : dupliquer en « Virage sûr, copie », annoncé");
  await act(page, "scenario-new", "virage SÛR");
  await shot(page, "fr-CA, intersection : « virage SÛR » est refusé, nom déjà pris");
  await act(page, "scenario-delete");
  await shot(page, "fr-CA, intersection : Supprimer demande une confirmation, annoncée");
  await page.keyboard.press("Enter");
  await page.waitForTimeout(150);
  await shot(page, "fr-CA, intersection : confirmer la suppression, annoncée, la référence est affichée");
  await page.context().close();

  // Start over on a junction scenario: the baseline is turned first.
  page = await newPage(browser, "en");
  await page.goto("/" + places.junction);
  await page.locator("#scenario-pick").waitFor();
  const turn = async () => {
    await page.locator("#wrap").focus();
    await page.keyboard.press("Escape");
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Shift+ArrowRight");
  };
  const bearing = async (name) => {
    for (let i = 0; i < 30; i++) await page.waitForTimeout(100);
    const r = await junctionRecord(page);
    const d = name === null ? r?.state : r?.scenarios?.find((s) => s.name === name)?.design;
    return d?.arms?.[0]?.bearing;
  };
  await turn();
  await shot(page, `Start over, junction: on the baseline, turn the first street: the kept baseline's bearing is ${await bearing(null)}°`);
  await act(page, "scenario-new", "Turned more");
  await turn();
  await shot(page, `Make “Turned more” and turn the street again: the scenario's bearing is ${await bearing("Turned more")}°, the baseline's ${await bearing(null)}°`);
  await page.locator("#reset").focus();
  await page.keyboard.press("Enter");
  await page.waitForTimeout(150);
  const jl = await page.locator("#live").textContent();
  await shot(page, `Press Start over in “Turned more”: “${jl}”; the scenario's bearing is ${await bearing("Turned more")}° (as imported), the baseline's still ${await bearing(null)}°; Undo enabled: ${await page.locator("#undo").isEnabled()}`);
  await page.locator("#undo").focus();
  await page.keyboard.press("Enter");
  await shot(page, `Undo the Start over: the scenario's bearing is ${await bearing("Turned more")}° again, the baseline's ${await bearing(null)}°`);
  /** Selects the first street and opens its Position fold, so its bearing field is on screen; returns the field's value. */
  const shownBearing = async () => {
    await page.locator("#wrap").focus();
    await page.keyboard.press("Escape");
    await page.keyboard.press("ArrowRight");
    const fold = page.locator("#jn-fold-position");
    await fold.waitFor();
    if (!(await fold.evaluate((d) => d.open))) await fold.locator("summary").click();
    const field = page.locator('#inspector input[aria-labelledby="i-h-bearing"]');
    await field.waitFor();
    return field.inputValue();
  };
  await page.locator("#reset").focus();
  await page.keyboard.press("Enter");
  await page.waitForTimeout(150);
  const scenarioField = await shownBearing();
  await shot(page, `Start over in “Turned more” again: the first street's bearing field reads ${scenarioField}° (as imported)`);
  await show(page, null);
  const baseField = await shownBearing();
  await shot(page, `Then show the baseline: the first street's bearing field reads ${baseField}°, the baseline's own turn, untouched by the scenario's Start over`);
  await page.context().close();
  await browser.close();
} finally {
  writeFileSync(path.join(outDir, "steps.json"), JSON.stringify(log, null, 2));
  server.kill();
}
