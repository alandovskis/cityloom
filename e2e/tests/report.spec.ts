import { readFileSync } from "node:fs";

import type { Download, Page } from "@playwright/test";
// The legacy build runs on Node without a canvas.
// @ts-expect-error pdfjs-dist ships its Node build without declarations for this path
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";

import { expect, forgive, live, mapReady, openStreet, test } from "./fixtures";

// The design review report: the map page's Share menu writes a PDF in the browser (a cover, a summary and the sources pages) (the `cityloom_report`
// module, loaded on first use by `web/report.js`) and saves it. The file is read back with pdf.js.

const VERSION = readFileSync(new URL("../../crates/cityloom_editor/Cargo.toml", import.meta.url), "utf8").match(
  /^version = "(.+)"$/m,
)![1];

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
  expect(pages).toBeGreaterThanOrEqual(3);
  expect(text).toContain("Plateau");
  // A line break in the PDF is a run of spaces in the text, so these are read with whitespace collapsed.
  const flat = text.replace(/\s+/g, " ");
  expect(flat).toContain("Room for the transition");
  expect(flat).toContain("Federal Highway Administration");
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
  expect(pages).toBeGreaterThanOrEqual(3);
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
