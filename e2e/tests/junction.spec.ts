import { expect, live, openJunction, openStreet, test } from "./fixtures";

import type { Page } from "@playwright/test";

const turns = (page: Page) => page.locator("#turns button.turn");

// The editor is reached from the map: a junction of the Plateau, the first the map lists.
test.describe("the junction editor on a junction of the city", () => {
  test.beforeEach(async ({ page }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
  });

  test("is named for the junction, links back to the map, and has no streets to add or pages to go to", async ({
    page,
  }) => {
    const name = (await page.locator("#street-name").innerText()).trim();
    expect(name).not.toBe("");
    await expect(page.locator("a.back")).toHaveAttribute("href", "map.html");
    await expect(page).toHaveTitle(`${name} · CityLoom`);
    await expect(page.locator(".lower")).toHaveCount(0);
    await expect(page.locator('.surface[href="street.html"]')).toHaveCount(0);
  });

  test("a raised table is switched on from the details and drawn with its ramps", async ({ page }) => {
    await expect(page.locator("#drawing .raised-ramp")).toHaveCount(0);
    await page.getByRole("checkbox", { name: "Raise the whole junction to sidewalk height" }).check();
    await expect(page.locator("#drawing .raised-ramp").first()).toBeVisible();
  });

  test("a street's crossing is made a continuous sidewalk from its details and drawn with ramps", async ({ page }) => {
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await expect(page.locator("#drawing .continuous-ramp")).toHaveCount(0);
    await page.getByRole("checkbox", { name: "Sidewalk runs on across the street" }).check();
    await expect(page.locator("#drawing .continuous-ramp").first()).toBeVisible();
  });

  test("a street with a bike lane is given a bike box from its details and it is drawn", async ({ page }) => {
    await page.locator("#wrap").focus();
    await expect(page.locator("#drawing .bike-box")).toHaveCount(0);
    const box = page
      .locator("#inspector")
      .getByRole("checkbox", { name: /^(Waiting box for cyclists ahead of the cars|Bike box \()/ });
    // Each street takes three presses (the street, its crossing, the corner): find the first with a bike lane to reach a box from.
    await page.keyboard.press("ArrowRight");
    for (let i = 0; i < 16 && !(await box.isEnabled({ timeout: 500 }).catch(() => false)); i++)
      await page.keyboard.press("ArrowRight");
    await box.check();
    await expect(page.locator("#drawing .bike-box").first()).toBeVisible();
  });

  test("a corner can be protected from its details, or says why it cannot", async ({ page }) => {
    await page.locator("#wrap").focus();
    const protect = page
      .locator("#inspector")
      .getByRole("checkbox", { name: /^(Bike track round a corner island|Protected corner \()/ });
    // The panel of a corner has the switch: arrow through the streets, crossings and corners until one shows.
    for (let i = 0; i < 16 && !(await protect.isVisible().catch(() => false)); i++)
      await page.keyboard.press("ArrowRight");
    await expect(protect).toBeVisible();
    if (await protect.isEnabled()) {
      await protect.check();
      await expect(page.locator("#drawing .corner-island").first()).toBeVisible();
    } else {
      await expect(protect).toHaveAccessibleName(/^Protected corner \(/);
    }
  });

  test("starts as the city lays it out and says how many streets meet and whether it works", async ({ page }) => {
    await expect(page.locator("#arm-count")).toHaveText(/^[3-5] streets$/);
    await expect(page.locator("#fit")).toContainText(/^[3-5] streets, /);
    await expect(page.locator("#drawing")).toHaveAttribute(
      "aria-label",
      /^Plan of the junction, north up\. [3-5] streets/,
    );
    await expect(page.locator("#undo")).toBeDisabled();
    await expect(page.locator("#redo")).toBeDisabled();
    await expect(page.locator("#reset")).toBeDisabled();
  });

  test("the control is changed from the details, announced, and can be undone and redone", async ({ page }) => {
    const before = await page.locator("#fit").innerText();
    await page.locator("#inspector select").first().selectOption({ label: "Roundabout" });
    await expect(page.locator("#fit")).toHaveText(/roundabout/i);
    await expect(live(page)).toContainText("roundabout");
    await expect(page.locator("#undo")).toBeEnabled();
    await expect(page.locator("#reset")).toBeEnabled();
    await page.locator("#undo").click();
    await expect(live(page)).toHaveText("Undone.");
    await expect(page.locator("#fit")).toHaveText(before);
    await expect(page.locator("#redo")).toBeEnabled();
    await page.locator("#redo").click();
    await expect(page.locator("#fit")).toHaveText(/roundabout/i);
  });

  test("start over puts the junction back as it is today and leaves nothing to undo", async ({ page }) => {
    const before = await page.locator("#fit").innerText();
    await page.locator("#inspector select").first().selectOption({ label: "Roundabout" });
    await page.locator("#reset").click();
    await expect(live(page)).toHaveText("Started over from the junction as it is today.");
    await expect(page.locator("#fit")).toHaveText(before);
    await expect(page.locator("#reset")).toBeDisabled();
    await expect(page.locator("#undo")).toBeDisabled();
  });

  test("a turn is banned and allowed again from the table, and Ctrl+Z undoes it", async ({ page }) => {
    const turn = turns(page).first();
    await expect(turn).toHaveClass(/\bon\b/);
    await turn.click();
    await expect(turn).not.toHaveClass(/\bon\b/);
    await expect(live(page)).not.toBeEmpty();
    await page.keyboard.press("Control+z");
    await expect(turn).toHaveClass(/\bon\b/);
  });

  test("the arrow keys select a street, which the details then show", async ({ page }) => {
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await expect(live(page)).toHaveText(/^.+ \(.+\), \d+ degrees$/);
    await expect(page.locator("#inspector .insp-empty")).toHaveCount(0);
    await page.keyboard.press("Escape");
    await expect(page.locator("#inspector .insp-empty")).toBeVisible();
  });

  test("Shift and an arrow turn the selected street, and Delete cannot take it away", async ({ page }) => {
    const count = await page.locator("#arm-count").innerText();
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    // A street turns only where it keeps clear of its neighbours: one way or the other.
    await page.keyboard.press("Shift+ArrowRight");
    if (!/bearing/.test(await live(page).innerText())) await page.keyboard.press("Shift+ArrowLeft");
    await expect(live(page)).toContainText("bearing");
    await page.keyboard.press("Delete");
    await expect(page.locator("#arm-count")).toHaveText(count);
  });

  test("the 2.5D view is looked at, follows an edit, and is remembered across a reload", async ({ page }) => {
    const plan = page.locator("#view-plan");
    const iso = page.locator("#view-iso");
    await expect(page.locator("#drawing .iso")).toHaveCount(0);
    await expect(plan).toHaveAttribute("aria-pressed", "true");

    await iso.click();
    await expect(page.locator("#drawing .iso")).toHaveCount(1);
    await expect(page.locator("#drawing .iso-top").first()).toBeVisible();
    await expect(page.locator("#drawing .iso-name").first()).toBeVisible();
    await expect(page.locator("#drawing [data-role^='grip']")).toHaveCount(0);
    await expect(iso).toHaveAttribute("aria-pressed", "true");
    await expect(live(page)).toHaveText("Showing the junction in 2.5D.");

    // The keyboard still edits the junction, and the picture follows.
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await expect(page.locator("#drawing .sel-box")).toHaveCount(1);
    const selected = await page.locator("#drawing").innerHTML();
    // A street turns only where it keeps clear of its neighbours: one way or the other. The first turn is
    // waited for before the second is tried, or the two could cancel.
    const said = await live(page).innerText();
    await page.keyboard.press("Shift+ArrowRight");
    await expect(live(page)).not.toHaveText(said);
    if (!/bearing/.test(await live(page).innerText())) await page.keyboard.press("Shift+ArrowLeft");
    await expect(live(page)).toContainText("bearing");
    await expect.poll(async () => page.locator("#drawing").innerHTML()).not.toBe(selected);
    await expect(page.locator("#drawing .iso")).toHaveCount(1);

    // The choice is kept.
    await page.reload();
    await expect(page.locator("#drawing .iso")).toHaveCount(1);
    await expect(page.locator("#view-iso")).toHaveAttribute("aria-pressed", "true");

    // V goes back to the plan from anywhere on the page.
    await page.keyboard.press("v");
    await expect(page.locator("#drawing .iso")).toHaveCount(0);
    await expect(page.locator("#drawing g.arm").first()).toBeVisible();
    await expect(page.locator("#view-plan")).toHaveAttribute("aria-pressed", "true");
    await expect(live(page)).toHaveText("Showing the plan.");
  });

  test("a street is pressed on the plan to select it, and its handle is dragged to turn it", async ({ page }) => {
    const arm = page.locator("#drawing g.arm").first();
    const uid = await arm.getAttribute("data-uid");
    await arm.click();
    await expect(page.locator("#inspector .insp-empty")).toHaveCount(0);
    const grip = page.locator(`#drawing [data-role="grip-arm"][data-uid="${uid}"]`);
    const box = (await grip.boundingBox())!;
    const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x + 120, y + 40, { steps: 8 });
    await page.mouse.up();
    await expect(live(page)).toContainText("bearing");
    await expect(page.locator("#undo")).toBeEnabled();
  });

  test("the notes list the checks, the changes made and where the junction came from", async ({ page }) => {
    await page.locator("#t-checks").click();
    await expect(page.locator("#checks li").first()).toBeVisible();
    await page.locator("#inspector select").first().selectOption({ label: "Roundabout" });
    await page.locator("#t-changes").click();
    await expect(page.locator("#revs tbody tr")).toHaveCount(2);
    await page.locator("#t-provenance").click();
    await expect(page.locator("#p-provenance")).toBeVisible();
  });
});

// Each junction measure of the Atlas: set on an arm from the inspector, it shows its code on the plan.
const JUNCTION_MEASURES = [
  { code: "G1", list: "At the approach", name: "Offset queue-jump lane", busLane: false },
  { code: "G2", list: "At the approach", name: "Curbside queue-jump lane", busLane: false },
  { code: "G3", list: "At the approach", name: "Virtual queue-jump lane", busLane: false },
  { code: "H1", list: "At the approach", name: "Signal-controlled bus gate", busLane: true },
  { code: "H2", list: "At the approach", name: "Yield-controlled bus gate", busLane: true },
  { code: "M1", list: "Bus stop", name: "Bus bulb", busLane: false },
  { code: "M2", list: "Bus stop", name: "Signal-protected on-street platform", busLane: false },
  { code: "L1", list: "Turns", name: "Indirect left turn via alternative itinerary", busLane: false },
  { code: "L2", list: "Turns", name: "Indirect left turn within the intersection", busLane: false },
  { code: "L3", list: "Turns", name: "Right-in/right-out", busLane: false },
  { code: "L4", list: "Turns", name: "Dead-ending of a lateral street", busLane: false },
  { code: "N1", list: null, name: "Transit modal filter", busLane: false },
];

test.describe("the junction measures of the Atlas", () => {
  for (const m of JUNCTION_MEASURES) {
    test(`${m.code} ${m.name} is set on an arm and its code is on the plan`, async ({ page }) => {
      await openJunction(page);
      await page.locator("#drawing g.arm").first().click();
      const tag = page.locator("#drawing text.measure-tag", { hasText: new RegExp(`^${m.code}$`) });
      await expect(tag).toHaveCount(0);
      if (m.busLane) {
        await page.getByRole("checkbox", { name: "Bus lane along the way in" }).click();
      }
      if (m.list) {
        await page.getByLabel(m.list, { exact: true }).selectOption({ label: `${m.code} ${m.name}` });
      } else {
        await page.getByRole("checkbox", { name: `${m.code} ${m.name}` }).click();
      }
      await expect(tag.first()).toBeVisible();
    });
  }
});

test.describe("what the junction editor keeps in the city", () => {
  test("says which OpenStreetMap nodes it was made from, and whether it has been changed", async ({ page }) => {
    await openJunction(page);
    await page.locator("#t-provenance").click();
    const block = page.locator("#p-provenance");
    await expect(block.locator('a[href^="https://www.openstreetmap.org/node/"]').first()).toHaveText(
      /^node \d+( v\d+)?$/,
    );
    await expect(block.locator("#tb-state")).toHaveText("As imported");
    await page.locator("#t-space").click();
    await turns(page).first().click();
    await page.locator("#t-provenance").click();
    await expect(block.locator("#tb-state")).toHaveText("Edited");
    await page.reload();
    await page.locator("#t-provenance").click();
    await expect(block.locator("#tb-state")).toHaveText("Edited");
  });

  test("what is changed is kept across a reload and shown on the map", async ({ page }) => {
    await openJunction(page);
    const turn = turns(page).first();
    await expect(turn).toHaveClass(/\bon\b/);
    await turn.click();
    await expect(turn).not.toHaveClass(/\bon\b/);
    await page.reload();
    await expect(turns(page).first()).not.toHaveClass(/\bon\b/);
    await page.goto("/map.html");
    await page.locator("#t-changes").click();
    await expect(page.locator("#changes")).not.toBeEmpty();
  });

  test("a change is kept even when the page is left at once", async ({ page }) => {
    const href = await openJunction(page);
    await turns(page).first().click();
    await page.goto("/map.html");
    await page.goto(`/${href}`);
    await expect(turns(page).first()).not.toHaveClass(/\bon\b/);
  });

  test("a junction that is not in the city goes back to the map", async ({ page }) => {
    await page.goto("/intersection.html?junction=999");
    await expect(page).toHaveURL(/map\.html$/);
  });

  test("the page without a junction goes back to the map", async ({ page }) => {
    await page.goto("/intersection.html");
    await expect(page).toHaveURL(/map\.html$/);
  });

  test("a street change made elsewhere shows in the junction at its end", async ({ page }) => {
    await openStreet(page);
    const end = (await page.locator('#street a[href^="intersection.html"]').first().getAttribute("href"))!;
    const street = page.url();
    await page.goto(`/${end}`);
    await expect(page.locator("#drawing")).toBeVisible();
    const before = await page.locator("#drawing").innerHTML();
    await page.goto(street);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("-");
    await expect(live(page)).toContainText("Resize");
    await page.goto(`/${end}`);
    await expect(page.locator("#drawing")).toBeVisible();
    expect(await page.locator("#drawing").innerHTML()).not.toBe(before);
  });
});

// A number field takes the decimal mark of the page's language, and a point too.
const cycleWidth = async (page: Page, control: string, track: RegExp) => {
  await page.locator("#inspector select").first().selectOption({ label: control });
  await page.getByRole("checkbox", { name: track }).click();
  return page.locator('#inspector input[aria-labelledby="i-h-cycle-width"]');
};

test.describe("a length typed in a junction number field", () => {
  test("is read with a decimal comma in French and shown with one", async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem("cityloom-lang", "fr-CA"));
    await openJunction(page);
    const width = await cycleWidth(page, "Carrefour giratoire", /Piste tout autour/);
    await expect(width).toHaveAttribute("inputmode", "decimal");
    await expect(width).toHaveAttribute("type", "text");
    await width.fill("2,5");
    await width.press("Enter");
    await expect(width).toHaveValue("2,5");
    await width.press("ArrowDown");
    await expect(width).toHaveValue("2,0");
    await width.press("ArrowUp");
    await expect(width).toHaveValue("2,5");
    await width.fill("abc");
    await width.press("Enter");
    await expect(width).toHaveValue("2,5");
    await width.fill("9,5");
    await width.press("Enter");
    await expect(width).toHaveValue("2,5");
  });

  test("is read with a point in English", async ({ page }) => {
    await openJunction(page);
    const width = await cycleWidth(page, "Roundabout", /Track around the outside/);
    await width.fill("2.5");
    await width.press("Enter");
    await expect(width).toHaveValue("2.5");
    await width.press("ArrowDown");
    await expect(width).toHaveValue("2.0");
  });
});

test.describe("signal timing on a junction of the city", () => {
  test.beforeEach(async ({ page }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Traffic signal" });
  });

  test("a street's green is typed, the cycle follows, and a reload keeps it", async ({ page }) => {
    const cycle = page.locator("#inspector").getByText(/^Cycle: \d+ s$/);
    await expect(cycle).toBeVisible();
    const before = Number((await cycle.innerText()).match(/\d+/)![0]);
    await page.locator("#inspector .signal-list button").first().click();
    const green = page.locator('#inspector input[aria-labelledby="i-h-green"]');
    await expect(green).toHaveValue("25");
    await green.fill("30");
    await green.press("Enter");
    await expect(green).toHaveValue("30");
    // (wait for the edit to be kept the way the neighbouring persistence test does, then:)
    await page.reload();
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator("#inspector").getByText(`Cycle: ${before + 5} s`)).toBeVisible();
  });

  test("a bad typed time changes nothing and a long cycle fails its check", async ({ page }) => {
    await page.locator("#inspector .signal-list button").first().click();
    const green = page.locator('#inspector input[aria-labelledby="i-h-green"]');
    await green.fill("abc");
    await green.press("Enter");
    await expect(green).toHaveValue("25");
    await green.fill("120");
    await green.press("Enter");
    await expect(green).toHaveValue("120");
    await page.locator("#t-checks").click();
    await expect(page.locator("#p-checks")).toContainText("Signal cycle");
    await expect(page.locator("#p-checks")).toContainText("over the 120 s");
  });

  test("the timing is French in French", async ({ page }) => {
    await page.locator("#account-btn").click();
    await page.getByRole("button", { name: "Français" }).click();
    await expect(page.locator("#inspector")).toContainText("Réglage des feux");
    await expect(page.locator("#inspector")).toContainText(/Cycle\s*:\s*\d+\s*s/);
  });
});
