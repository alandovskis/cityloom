import { readFileSync } from "node:fs";

import type { Download, Page } from "@playwright/test";
// The legacy build runs on Node without a canvas.
// @ts-expect-error pdfjs-dist ships its Node build without declarations for this path
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";

import { expect, forgive, live, mapReady, openJunction, openStreet, test } from "./fixtures";

// The design review report: the map page's Share menu writes a PDF in the browser (a cover, a summary, a page per changed
// place and the sources pages; the `cityloom_report`
// module, loaded on first use by `web/report.js`) and saves it. The file is read back with pdf.js.

const VERSION = readFileSync(new URL("../../crates/cityloom_editor/Cargo.toml", import.meta.url), "utf8").match(
  /^version = "(.+)"$/m,
)![1];

/** Downloads from the map page that is open already. */
async function downloadHere(page: Page): Promise<{ file: Download; bytes: Buffer }> {
  await mapReady(page);
  await page.locator("#share-btn").click();
  const pending = page.waitForEvent("download");
  await page.locator("#report").click();
  const file = await pending;
  return { file, bytes: readFileSync(await file.path()) };
}

/** The alternative texts of the figures, from the tagged structure of every page. */
async function figureAlts(bytes: Buffer): Promise<string[]> {
  const doc = await getDocument({ data: new Uint8Array(bytes), useSystemFonts: false }).promise;
  const alts: string[] = [];
  const walk = (node: any) => {
    if (node.role === "Figure" && node.alt) alts.push(node.alt);
    for (const child of node.children ?? []) walk(child);
  };
  for (let n = 1; n <= doc.numPages; n++) {
    const tree = await (await doc.getPage(n)).getStructTree();
    if (tree) walk(tree);
  }
  return alts;
}

/** Changes the first piece's width on the street page, then the control of a junction on its page. Says the
 *  street's name as its page shows it. */
async function editAStreetAndAJunction(page: Page, roundabout = "Roundabout"): Promise<string> {
  await openStreet(page);
  await page.locator("#wrap").focus();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  const width = page.locator("#inspector input").first();
  await expect(width).toBeFocused();
  const shown = await width.inputValue();
  const was = Number(shown.replace(",", "."));
  // The field reads the decimal mark of the page's language.
  await width.fill(String(Math.round((was - 0.3) * 10) / 10).replace(".", shown.includes(",") ? "," : "."));
  await width.press("Enter");
  await expect(page.locator("#undo")).toBeEnabled();
  const name = (await page.locator("#tb-street").innerText()).trim();
  await openJunction(page);
  await page.locator("#inspector select").first().selectOption({ label: roundabout });
  await expect(page.locator("#undo")).toBeEnabled();
  return name;
}

async function download(page: Page): Promise<{ file: Download; bytes: Buffer }> {
  await page.goto("/map.html");
  await mapReady(page);
  await page.locator("#share-btn").click();
  const pending = page.waitForEvent("download");
  await page.locator("#report").click();
  const file = await pending;
  const path = await file.path();
  return { file, bytes: readFileSync(path) };
}

async function read(bytes: Buffer): Promise<{ pages: number; text: string }> {
  const doc = await getDocument({ data: new Uint8Array(bytes), useSystemFonts: false }).promise;
  let text = "";
  for (let n = 1; n <= doc.numPages; n++) {
    const content = await (await doc.getPage(n)).getTextContent();
    text += content.items.map((i: any) => i.str).join(" ") + "\n";
  }
  return { pages: doc.numPages, text };
}

test("the Share menu downloads the report as a PDF", async ({ page }) => {
  const { file, bytes } = await download(page);
  expect(file.suggestedFilename()).toMatch(/^cityloom-.*\.pdf$/);
  expect(bytes.subarray(0, 4).toString("latin1")).toBe("%PDF");
  const { pages, text } = await read(bytes);
  expect(pages).toBeGreaterThanOrEqual(5);
  expect(text).toContain("Plateau");
  // A line break in the PDF is a run of spaces in the text, so these are read with whitespace collapsed.
  const flat = text.replace(/\s+/g, " ");
  expect(flat).toContain("Room for the transition");
  expect(flat).toContain("Regional Municipality of York");
  // The last cited rule and its source: a page cut short would lose them before the closing notice.
  expect(flat).toContain("Planting clear of the corners at junctions");
  expect(flat).toContain("Distance de visibilité");
  expect(flat).toContain("Every other rule and recommendation");
  expect(text).toContain(VERSION);
  expect(text).toMatch(/\bPass\b/);
  expect(text).toMatch(/\bFail\b/);
  await file.saveAs(process.env.REPORT_SAMPLE ?? test.info().outputPath("report.pdf"));
});

test("the report downloads in French when the page is French", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("cityloom-lang", "fr-CA"));
  await page.goto("/map.html");
  await mapReady(page);
  await page.locator("#share-btn").click();
  await expect(page.locator("#report")).toHaveText("Télécharger le rapport (PDF)");
  const pending = page.waitForEvent("download");
  await page.locator("#report").click();
  const file = await pending;
  const { pages, text } = await read(readFileSync(await file.path()));
  expect(pages).toBeGreaterThanOrEqual(5);
  expect(text.replace(/\s+/g, " ")).toContain("Place pour la transition");
  expect(text).toContain("Réussite");
  expect(text).toContain("Échec");
  expect(text).toContain(VERSION);
});

test("a report that cannot be written saves nothing", async ({ page, errors }) => {
  await page.route("**/cityloom_report*", (route) => route.fulfill({ status: 404, body: "gone" }));
  await page.goto("/map.html");
  await mapReady(page);
  let saved = false;
  page.on("download", () => (saved = true));
  await page.locator("#share-btn").click();
  await page.locator("#report").click();
  await expect(live(page)).toContainText("The report could not be made", { timeout: 5000 });
  const notice = page.locator("#report-notice");
  await expect(notice).toBeVisible();
  await expect(notice).toHaveText(/The report could not be made\. Try again\./);
  await expect(notice).toHaveAttribute("role", "status");
  await page.getByRole("button", { name: "Dismiss" }).click();
  await expect(notice).toHaveCount(0);
  await page.waitForTimeout(2000);
  expect(saved).toBe(false);
  forgive(errors, /404|Failed to load|cityloom_report/i);
});

test("the editors' Share menu still prints", async ({ page }) => {
  await openStreet(page);
  await page.locator("#share-btn").click();
  await expect(page.locator("#share-menu #print")).toBeVisible();
  await expect(page.locator("#report")).toHaveCount(0);
});

test("with no edits the report has the review pages and the appendix but no place pages", async ({ page }) => {
  const { bytes } = await download(page);
  const { pages, text } = await read(bytes);
  // The cover, the summary, the network effects and area rules, the limits of the data and the appendix.
  expect(pages).toBeGreaterThanOrEqual(5);
  for (const heading of [
    "Network effects",
    "Area rules",
    "Limits of the OpenStreetMap data",
    "Appendix",
    "Glossary",
    "Credits",
  ]) {
    expect(text).toContain(heading);
  }
  // The glossary names Before and After, but no place page draws them.
  expect(await figureAlts(bytes)).toEqual([]);
});

test("an edited street and an edited junction each get a page in the report", async ({ page }) => {
  const name = await editAStreetAndAJunction(page);
  await page.goto("/map.html");
  const { file, bytes } = await downloadHere(page);
  const { pages, text } = await read(bytes);
  // Five pages without edits, and two places changed.
  expect(pages).toBeGreaterThanOrEqual(7);
  expect(text).toContain("Before");
  expect(text).toContain("After");
  expect(text).toContain(name);
  expect(text.indexOf("Before")).toBeLessThan(text.indexOf("Network effects"));
  const alts = await figureAlts(bytes);
  expect(alts.length).toBe(4);
  for (const alt of alts) expect(alt.length).toBeGreaterThan(20);
  await file.saveAs(process.env.REPORT_SAMPLE_PLACES ?? test.info().outputPath("report-places.pdf"));
});

test("the place pages follow the language", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("cityloom-lang", "fr-CA"));
  await editAStreetAndAJunction(page, "Carrefour giratoire");
  await page.goto("/map.html");
  const { bytes } = await downloadHere(page);
  const { pages, text } = await read(bytes);
  expect(pages).toBeGreaterThanOrEqual(7);
  expect(text).toContain("Avant");
  expect(text).toContain("Après");
  for (const heading of [
    "Effets sur le réseau",
    "Règles de zone",
    "Limites des données OpenStreetMap",
    "Annexe",
    "Glossaire",
    "Crédits",
  ]) {
    expect(text).toContain(heading);
  }
  expect(text).not.toContain("Network effects");
  expect(text).not.toMatch(/\bBefore\b/);
  const alts = await figureAlts(bytes);
  expect(alts.length).toBe(4);
});
