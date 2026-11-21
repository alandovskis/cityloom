import AxeBuilder from "@axe-core/playwright";
import type { Page } from "@playwright/test";

import { expect, live, openJunction, openStreet, test } from "./fixtures";

// The scenario comparison page (compare.html): two designs of one place side by side, and what differs in the second.
// Opened from a street's or junction's scenario row; it reads the place's edits and writes nothing.

/** Opens the scenario actions (a disclosure), from the keyboard. */
const openActions = async (page: Page) => {
  const details = page.locator("details.scenario-manage");
  if (await details.evaluate((d: HTMLDetailsElement) => d.open)) return;
  await details.locator("summary").focus();
  await page.keyboard.press("Enter");
  await expect(details).toHaveAttribute("open", "");
};

/** Makes a scenario called `name` from the shown design; it is then the one shown. */
const makeScenario = async (page: Page, name: string) => {
  await openActions(page);
  await page.locator("#scenario-name").focus();
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.type(name);
  await page.locator("#scenario-new").focus();
  await page.keyboard.press("Enter");
  await expect(live(page)).toHaveText(`Scenario “${name}” made from the baseline. Now editing it.`);
};

/** Widens the street's first piece, in the scenario being edited. */
const widenStreetPiece = async (page: Page) => {
  await page.locator("#wrap").focus();
  await page.keyboard.press("Escape");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("+");
  await expect(page.locator("#undo")).toBeEnabled();
};

/** Changes the junction's design, in the scenario being edited. */
const changeJunction = async (page: Page) => {
  await page.locator("#wrap").focus();
  await page.keyboard.press("Escape");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Shift+ArrowRight");
  if (!(await page.locator("#undo").isEnabled())) await page.keyboard.press("Shift+ArrowLeft");
  await expect(page.locator("#undo")).toBeEnabled();
};

const chooseFrench = async (page: Page) => {
  await page.locator("#account-btn").click();
  await page.locator('[data-lang="fr-CA"]').click();
  await expect(page.locator("html")).toHaveAttribute("lang", "fr-CA");
};

/** The id of the scenario `#compare-b` shows, from the address. */
const scenarioId = (page: Page) => new URL(page.url()).searchParams.get("b")!;

const noSidewaysScroll = (page: Page, width: number) =>
  page.evaluate(() => document.documentElement.scrollWidth).then((w) => expect(w).toBeLessThanOrEqual(width));

test.describe("comparing a street's designs", () => {
  test("the baseline and a scenario are drawn side by side and what differs in B is said", async ({ page }) => {
    await openStreet(page);
    await makeScenario(page, "Wider bike lane");
    await widenStreetPiece(page);
    await page.locator("#scenario-compare").click();

    await expect(page).toHaveURL(/compare\.html/);
    await expect(page.locator("#compare-h")).toBeVisible();
    await expect(page.locator("#compare-a")).toHaveValue("base");
    const id = scenarioId(page);
    await expect(page.locator("#compare-b")).toHaveValue(id);
    await expect(page.locator("#compare-b option:checked")).toHaveText("Wider bike lane");
    await expect(page.locator(".compare-cols > section.compare-col")).toHaveCount(2);
    await expect(page.locator("figure.compare-iso")).toHaveCount(2);
    await expect(page.locator("figure.compare-plan")).toHaveCount(2);
    for (const figure of await page.locator("figure.compare-iso, figure.compare-plan").all()) {
      await expect(figure).toHaveAttribute("role", "img");
      await expect(figure).toHaveAttribute("aria-label", /\S/);
    }
    const summary = page.locator("#compare-summary");
    await expect(summary.locator("h2")).toHaveText("What differs in B");
    await expect(summary.locator("p.compare-headline")).toContainText("Wider bike lane");
    await expect(summary.locator("h3", { hasText: "Space" })).toBeVisible();
    await expect(summary.locator("ul li").first()).toBeVisible();
    // a change of pick is announced: the headline, which the summary opens with
    const headline = await summary.locator("p.compare-headline").innerText();
    await page.locator("#compare-b").selectOption("base");
    await expect(summary).toContainText("Baseline and Baseline are the same.");
    await page.locator("#compare-b").selectOption(id);
    await expect(live(page)).toHaveText(headline);
    await expect(summary.locator("p.compare-headline")).toHaveText(headline);
  });

  test("picking the same design twice says they are the same, and the address keeps the picks", async ({ page }) => {
    await openStreet(page);
    await makeScenario(page, "Wider bike lane");
    await widenStreetPiece(page);
    await page.locator("#scenario-compare").click();
    await expect(page.locator("#compare-b")).not.toHaveValue("base");
    const id = scenarioId(page);

    await page.locator("#compare-a").selectOption(id);
    await expect.poll(() => new URL(page.url()).searchParams.get("a")).toBe(id);
    expect(new URL(page.url()).searchParams.get("b")).toBe(id);
    await expect(live(page)).toHaveText(/Wider bike lane and Wider bike lane are the same\./);
    await expect(page.locator("#compare-summary")).toHaveText(/Wider bike lane and Wider bike lane are the same\./);
    await expect(page.locator("#compare-summary h3")).toHaveCount(0);
  });

  test("a scenario that is no longer there is refused, and the select shows what the page shows", async ({ page }) => {
    await openStreet(page);
    await makeScenario(page, "Wider bike lane");
    await page.locator("#scenario-compare").click();
    await expect(page).toHaveURL(/compare\.html/);
    const url = new URL(page.url());
    url.searchParams.set("b", "99");
    await page.goto(url.toString());
    await expect(page.locator("#compare-b")).toHaveValue("base");
    await expect(page.locator("#compare-refusal")).toHaveText("That scenario is no longer here.");
  });

  test("the page has no axe violations", async ({ page }) => {
    await openStreet(page);
    await makeScenario(page, "Wider bike lane");
    await widenStreetPiece(page);
    await page.locator("#scenario-compare").click();
    await expect(page.locator("figure.compare-iso")).toHaveCount(2);
    const scan = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
      .analyze();
    expect(scan.passes.length, "the scan reached the page").toBeGreaterThan(0);
    expect(scan.violations).toEqual([]);
  });

  test("at 280 px wide neither the street's scenario row nor the comparison scrolls sideways", async ({ page }) => {
    await page.setViewportSize({ width: 280, height: 800 });
    await openStreet(page);
    await makeScenario(page, "Wider bike lane");
    await expect(page.locator("#scenario-compare")).toBeVisible();
    await noSidewaysScroll(page, 280);
    await widenStreetPiece(page);
    await page.locator("#scenario-compare").click();
    await expect(page.locator("figure.compare-iso")).toHaveCount(2);
    await noSidewaysScroll(page, 280);
  });

  test("in French the page's words follow the language switch", async ({ page }) => {
    await openStreet(page);
    await makeScenario(page, "Wider bike lane");
    await widenStreetPiece(page);
    await page.locator("#scenario-compare").click();
    await expect(page.locator("#compare-summary h2")).toHaveText("What differs in B");
    await chooseFrench(page);
    await expect(page.locator("#compare-h")).toContainText("Comparer les conceptions");
    await expect(page.locator("#compare-summary h2")).toHaveText("Ce qui diffère dans B");
  });
});

test.describe("comparing a junction's designs", () => {
  test("each column draws the junction once", async ({ page }) => {
    await openJunction(page);
    await makeScenario(page, "Wider bike lane");
    await changeJunction(page);
    await page.locator("#scenario-compare").click();

    await expect(page).toHaveURL(/compare\.html/);
    await expect(page.locator("#compare-a")).toHaveValue("base");
    await expect(page.locator("#compare-b")).toHaveValue(scenarioId(page));
    await expect(page.locator(".compare-cols > section.compare-col")).toHaveCount(2);
    await expect(page.locator("figure.compare-iso")).toHaveCount(2);
    for (const column of await page.locator("section.compare-col").all()) {
      await expect(column.locator("figure.compare-iso")).toHaveCount(1);
    }
    await expect(page.locator("figure.compare-plan")).toHaveCount(0);
    await expect(page.locator("#compare-summary h2")).toHaveText("What differs in B");
  });
});
