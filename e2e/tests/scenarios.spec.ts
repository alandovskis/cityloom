import { expect, live, openJunction, openStreet, test } from "./fixtures";
import type { Page } from "@playwright/test";

// A place's scenarios (#190): named alternative designs kept beside its baseline, made, shown and changed from the
// keyboard alone on the street and junction pages. The page opens on the baseline; the map reads the baseline.

const pick = (page: Page) => page.locator("#scenario-pick");

/** Opens the scenario actions (a disclosure), from the keyboard. */
const openActions = async (page: Page) => {
  const details = page.locator("details.scenario-manage");
  if (await details.evaluate((d: HTMLDetailsElement) => d.open)) return;
  await details.locator("summary").focus();
  await page.keyboard.press("Enter");
  await expect(details).toHaveAttribute("open", "");
};

/** Types `name` in the name field and presses the button `id`, all from the keyboard. */
const act = async (page: Page, id: string, name?: string) => {
  await openActions(page);
  if (name !== undefined) {
    await page.locator("#scenario-name").focus();
    await page.keyboard.press("ControlOrMeta+a");
    await page.keyboard.type(name);
  }
  await page.locator(`#${id}`).focus();
  await page.keyboard.press("Enter");
};

/** Shows the design called `label` (the baseline's option or a scenario's name) with the menu, from the keyboard. */
const show = async (page: Page, label: string) => {
  await pick(page).focus();
  await pick(page).selectOption({ label });
};

const showBaseline = async (page: Page) => {
  await pick(page).focus();
  await pick(page).selectOption("");
};

/** The edits document of the first project, as kept. */
const edits = (page: Page) => page.evaluate(() => JSON.parse(localStorage.getItem("cityloom-edits@first") ?? "null"));

/** The street's first piece's width, as the inspector shows it. */
const firstPieceWidth = async (page: Page) => {
  await page.locator("#wrap").focus();
  await page.keyboard.press("Escape");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  const width = page.locator("#inspector input").first();
  await expect(width).toBeFocused();
  return width.inputValue();
};

test.describe("a street's scenarios", () => {
  test("two named scenarios are made, one is edited, and switching and a reload lose nothing", async ({ page }) => {
    await openStreet(page);
    await expect(pick(page)).toHaveValue("");
    await expect(page.locator("#scenario-mode")).toHaveText("Baseline: edits change the design the map shows.");
    const baseline = await firstPieceWidth(page);

    await act(page, "scenario-new", "Wider bike lane");
    await expect(live(page)).toHaveText("Scenario “Wider bike lane” made from the baseline. Now editing it.");
    await expect(page.locator("#scenario-mode")).toHaveText(
      "Scenario “Wider bike lane”: edits change this scenario only.",
    );
    await expect(page.locator("#scenario-name")).toHaveValue("");
    // an edit made within it
    await page.locator("#wrap").focus();
    await page.keyboard.press("Escape");
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("+");
    await expect(page.locator("#undo")).toBeEnabled();
    const wider = await firstPieceWidth(page);
    expect(wider).not.toEqual(baseline);

    await act(page, "scenario-new", "Parking kept");
    await expect(live(page)).toHaveText("Scenario “Parking kept” made from the baseline. Now editing it.");
    expect(await firstPieceWidth(page)).toEqual(baseline);

    await show(page, "Wider bike lane");
    await expect(live(page)).toHaveText("Now showing scenario “Wider bike lane”.");
    expect(await firstPieceWidth(page)).toEqual(wider);
    await showBaseline(page);
    await expect(live(page)).toHaveText("Now showing the baseline.");
    expect(await firstPieceWidth(page)).toEqual(baseline);

    await page.reload();
    await expect(pick(page)).toHaveValue("", { timeout: 10_000 });
    expect(await firstPieceWidth(page)).toEqual(baseline);
    await expect(pick(page).locator("option")).toHaveText([
      "Baseline (what the map shows)",
      "Wider bike lane",
      "Parking kept",
    ]);
    await show(page, "Wider bike lane");
    expect(await firstPieceWidth(page)).toEqual(wider);
    await show(page, "Parking kept");
    expect(await firstPieceWidth(page)).toEqual(baseline);
    const record = Object.values((await edits(page)).streets)[0] as { scenarios: { name: string }[] };
    expect(record.scenarios.map((s) => s.name)).toEqual(["Wider bike lane", "Parking kept"]);
  });

  test("a scenario is renamed, duplicated and deleted once the delete is confirmed", async ({ page }) => {
    await openStreet(page);
    await expect(page.locator("#scenario-delete")).toBeDisabled();
    await act(page, "scenario-new", "Wider bike lane");
    await act(page, "scenario-rename", "Wider lane");
    await expect(live(page)).toHaveText("Scenario “Wider bike lane” renamed “Wider lane”.");
    await act(page, "scenario-duplicate", "Wider lane, parking kept");
    await expect(live(page)).toHaveText(
      "Scenario “Wider lane” duplicated as “Wider lane, parking kept”. Now editing the copy.",
    );
    await act(page, "scenario-delete");
    await expect(live(page)).toHaveText(
      "Delete scenario “Wider lane, parking kept”? Press Confirm delete to delete it.",
    );
    await expect(page.locator("#scenario-delete")).toHaveAccessibleName("Confirm delete");
    await page.keyboard.press("Enter");
    await expect(live(page)).toHaveText("Scenario “Wider lane, parking kept” deleted. Now showing the baseline.");
    await expect(pick(page)).toHaveValue("");
    await expect(pick(page).locator("option")).toHaveText(["Baseline (what the map shows)", "Wider lane"]);
    // the pressed button is disabled on the baseline: the person goes on from the menu of designs, not the page's start
    await expect(pick(page)).toBeFocused();
  });

  test("asking to confirm a delete moves neither the scenarios nor the toolbar", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openStreet(page);
    await act(page, "scenario-new", "Parking kept, wider sidewalk");
    const boxes = () =>
      Promise.all(["#scenario-slot", ".tools", "#drawing"].map(async (s) => page.locator(s).first().boundingBox()));
    const before = await boxes();
    await act(page, "scenario-delete");
    await expect(page.locator("#scenario-delete")).toHaveAccessibleName("Confirm delete");
    expect(await boxes()).toEqual(before);
    // and it sits beside the title card, not on a row of its own
    const [title, slot] = await Promise.all([
      page.locator(".title-card").boundingBox(),
      page.locator("#scenario-slot").boundingBox(),
    ]);
    expect(slot!.x).toBeGreaterThan(title!.x + title!.width - 1);
  });

  test("in French a name already taken is refused, under the field and in the live region", async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem("cityloom-lang", "fr-CA"));
    await openStreet(page);
    await act(page, "scenario-new", "Parking kept");
    await expect(live(page)).toHaveText(/^Scénario «\u00a0Parking kept\u00a0» créé à partir de la référence\./);
    await act(page, "scenario-new", "parking KEPT");
    // the French quotes hold no-break spaces (OQLF)
    const refused = "Un autre scénario s’appelle déjà «\u00a0parking KEPT\u00a0».";
    await expect(live(page)).toHaveText(refused);
    await expect(page.locator("#scenario-refusal")).toHaveText(refused);
    await expect(page.locator("#scenario-name")).toHaveAttribute("aria-invalid", "true");
    await expect(page.locator("#scenario-name")).toHaveAttribute("aria-describedby", "scenario-refusal");
    await act(page, "scenario-new", " ");
    await expect(page.locator("#scenario-refusal")).toHaveText("Donnez un nom au scénario.");
  });
});

test.describe("a junction's scenarios", () => {
  test("two named scenarios are made, one is edited, and switching and a reload lose nothing", async ({ page }) => {
    await openJunction(page);
    await expect(pick(page)).toHaveValue("");
    await act(page, "scenario-new", "Wider bike lane");
    await expect(live(page)).toHaveText("Scenario “Wider bike lane” made from the baseline. Now editing it.");
    // an edit made within it, from the keyboard on the drawing
    await page.locator("#wrap").focus();
    await page.keyboard.press("Escape");
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Shift+ArrowRight");
    if (!(await page.locator("#undo").isEnabled())) await page.keyboard.press("Shift+ArrowLeft");
    await expect(page.locator("#undo")).toBeEnabled();

    await act(page, "scenario-new", "Parking kept");
    await expect(live(page)).toHaveText("Scenario “Parking kept” made from the baseline. Now editing it.");
    await expect(page.locator("#undo")).toBeDisabled();
    await show(page, "Wider bike lane");
    await expect(live(page)).toHaveText("Now showing scenario “Wider bike lane”.");
    await showBaseline(page);
    await expect(live(page)).toHaveText("Now showing the baseline.");

    await page.reload();
    await expect(pick(page)).toHaveValue("", { timeout: 10_000 });
    await expect(pick(page).locator("option")).toHaveText([
      "Baseline (what the map shows)",
      "Wider bike lane",
      "Parking kept",
    ]);
    const record = Object.values((await edits(page)).junctions)[0] as {
      state: unknown;
      scenarios: { name: string; design: unknown }[];
    };
    const [wider, parking] = record.scenarios;
    expect([wider.name, parking.name]).toEqual(["Wider bike lane", "Parking kept"]);
    expect(parking.design).toEqual(record.state);
    expect(wider.design).not.toEqual(record.state);
  });
});
