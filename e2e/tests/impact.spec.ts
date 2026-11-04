import type { Page } from "@playwright/test";

import { expect, live, openStreet, test } from "./fixtures";

// The Impact tab's "Last change" card (#64): what the last edit did to the street's space and checks.

const card = (page: Page) => page.locator("#impact-card");
const body = (page: Page) => page.locator("#impact");

/** Selects the first sidewalk by its cell in the piece list and gives the focus back to the drawing, where the keys edit. */
async function selectSidewalk(page: Page) {
  await page
    .locator("#piece-list .piece-cell")
    .filter({ has: page.locator(".piece-name", { hasText: /^(Sidewalk|Trottoir)$/ }) })
    .first()
    .click();
  await page.locator("#wrap").focus();
}

test.describe("the summary of the last change", () => {
  test.beforeEach(async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#drawing")).toBeVisible();
  });

  test("the first tab is Impact, and the card starts by saying nothing has changed", async ({ page }) => {
    await expect(page.locator("#t-space")).toHaveText("Impact");
    await expect(page.locator("#t-space")).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#h-impact")).toHaveText("Last change");
    await expect(body(page)).toContainText("No change yet.");
    // The width table is still there, under the card.
    await expect(page.locator("#h-space")).toHaveText("Where the width goes");
  });

  test("widening a sidewalk lists the space it took, and the check it broke apart from the ones that pass", async ({
    page,
  }) => {
    await selectSidewalk(page);
    await page.keyboard.press("+");
    // The edit is announced at once, as it always was.
    await expect(live(page)).toContainText("too wide");
    await expect(body(page)).toContainText("Resize sidewalk");
    await expect(body(page).locator(".impact-uses li").first()).toContainText(
      /Walking: [\d.]+ m → [\d.]+ m \(0\.1 m more\)/,
    );
    const failing = body(page).locator("h3", { hasText: "Newly failing" });
    await expect(failing).toBeVisible();
    await expect(body(page).locator("h3", { hasText: "Newly passing" })).toHaveCount(0);
    await expect(body(page).locator("ul.impact-checks li.bad")).toContainText("Fits the street width");
    await expect(body(page).locator(".impact-verdict")).toHaveText("Regressed: 1 check now fails.");
  });

  test("undo shows what undoing changed, as passing checks, and redo shows the edit again", async ({ page }) => {
    await selectSidewalk(page);
    await page.keyboard.press("+");
    await expect(body(page)).toContainText("Newly failing");
    await page.locator("#undo").click();
    await expect(body(page)).toContainText("Undone: Resize sidewalk");
    await expect(body(page).locator("h3", { hasText: "Newly passing" })).toBeVisible();
    await expect(body(page).locator("ul.impact-checks li.ok")).toContainText("Fits the street width");
    await expect(body(page).locator("ul.impact-checks li.bad")).toHaveCount(0);
    await expect(body(page).locator(".impact-verdict")).toHaveText("Improved: 1 check now passes.");
    await expect(body(page).locator(".impact-uses li").first()).toContainText("m less)");
    await page.locator("#redo").click();
    await expect(body(page)).toContainText("Redone: Resize sidewalk");
    await expect(body(page).locator("h3", { hasText: "Newly failing" })).toBeVisible();
  });

  test("start over is summarized as its own change", async ({ page }) => {
    await selectSidewalk(page);
    await page.keyboard.press("+");
    await expect(body(page)).toContainText("Newly failing");
    await page.locator("#reset").click();
    await expect(body(page).locator(".impact-what")).toHaveText("Started over");
    await expect(body(page).locator("h3", { hasText: "Newly passing" })).toBeVisible();
  });

  test("a change to neither space nor checks says so", async ({ page }) => {
    const current = await page.locator("#speed-limit").inputValue();
    await page.locator("#speed-limit").selectOption(current === "30" ? "40" : "30");
    await expect(body(page)).toContainText("No effect on space or checks");
    await expect(body(page).locator("ul")).toHaveCount(0);
  });

  test("a grouped operation, Make pedestrian, is one change that says what the street became", async ({ page }) => {
    await page.locator("#transform-btn").click();
    await page.locator("#transform-menu .ped-item").first().click();
    await expect(body(page).locator(".impact-what")).toHaveText("Made pedestrian");
    await expect(body(page).locator(".impact-uses li", { hasText: "Cars and trucks" })).toBeVisible();
    await expect(body(page).locator(".impact-class")).toContainText("is now a pedestrian street.");
    await page.locator("#undo").click();
    await expect(body(page).locator(".impact-what")).toHaveText("Undone: Made pedestrian");
    await expect(body(page).locator(".impact-class")).toContainText("carries traffic again.");
  });

  test("the card waits for the edits to settle: it does not change while a run of keys goes on", async ({ page }) => {
    await selectSidewalk(page);
    for (let i = 0; i < 4; i++) {
      await page.keyboard.press("+");
      await page.waitForTimeout(100);
      await expect(body(page)).toContainText("No change yet.");
    }
    // Only the last key's change is summarized.
    await expect(body(page).locator(".impact-uses li").first()).toContainText("(0.1 m more)");
  });

  test("the card is a polite live region that no key reaches, so the focus order is the page's own", async ({
    page,
  }) => {
    await expect(card(page)).toHaveAttribute("aria-live", "polite");
    await expect(card(page)).toHaveAttribute("aria-atomic", "true");
    await expect(card(page)).not.toHaveAttribute("aria-live", "assertive");
    await selectSidewalk(page);
    await page.keyboard.press("+");
    await expect(body(page)).toContainText("Newly failing");
    await expect(card(page).locator("a, button, input, select, textarea, [tabindex]")).toHaveCount(0);
    // Tab from the tab strip goes where it would go with no card.
    const order = async () => {
      await page.locator("#t-space").focus();
      const ids: string[] = [];
      for (let i = 0; i < 4; i++) {
        await page.keyboard.press("Tab");
        ids.push(await page.evaluate(() => document.activeElement?.id || document.activeElement?.tagName || ""));
      }
      return ids;
    };
    const withCard = await order();
    await page.evaluate(() => document.getElementById("impact-card")?.remove());
    expect(await order()).toEqual(withCard);
  });

  test("the Impact tab is reached by the tab keys and remembered", async ({ page }) => {
    await page.locator("#t-space").focus();
    await page.keyboard.press("ArrowRight");
    await expect(page.locator("#t-checks")).toHaveAttribute("aria-selected", "true");
    await expect(card(page)).toBeHidden();
    await page.keyboard.press("ArrowLeft");
    await expect(page.locator("#t-space")).toHaveAttribute("aria-selected", "true");
    await expect(card(page)).toBeVisible();
    await page.keyboard.press("End");
    await expect(page.locator("#t-provenance")).toHaveAttribute("aria-selected", "true");
    await page.keyboard.press("Home");
    await expect(page.locator("#t-space")).toHaveAttribute("aria-selected", "true");
    // Remembered across a reload, for the Impact tab as for another.
    await page.locator("#t-checks").click();
    await page.reload();
    await expect(page.locator("#t-checks")).toHaveAttribute("aria-selected", "true");
    await page.locator("#t-space").click();
    await page.reload();
    await expect(page.locator("#t-space")).toHaveAttribute("aria-selected", "true");
    await expect(card(page)).toBeVisible();
  });

  test("in French the tab is Incidence and the card is worded in French", async ({ page }) => {
    await page.locator("#account-btn").click();
    await page.locator('[data-lang="fr-CA"]').click();
    await expect(page.locator("#t-space")).toHaveText("Incidence");
    await expect(page.locator("#h-impact")).toHaveText("Dernière modification");
    await selectSidewalk(page);
    await page.keyboard.press("+");
    await expect(body(page)).toContainText("Nouveaux échecs");
    await expect(body(page).locator(".impact-verdict")).toContainText("Dégradation");
    await expect(body(page).locator(".impact-uses li").first()).toContainText("Marche");
  });
});
