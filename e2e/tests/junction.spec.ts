import { readFileSync } from "node:fs";

import {
  expect,
  live,
  openJunction,
  openStreet,
  placesOnMap,
  splitInto,
  streetLengthM,
  test,
  TURBO_PBF,
} from "./fixtures";

import type { Locator, Page } from "@playwright/test";

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
    // Each top is in its material's colour and pattern, the pattern laid on the ground.
    await expect(page.locator("#drawing .iso-top").first()).toHaveClass(/mat-\w+/);
    await expect(page.locator("#drawing pattern#iso-m-asphalt")).toHaveAttribute("patternTransform", /^matrix\(/);
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
    // waited for before the second is tried, or the two could cancel. The status is emptied before it says
    // anything (so the same words are heard twice), so "changed" is not enough: wait for the answer itself,
    // the new bearing or the refusal.
    await page.keyboard.press("Shift+ArrowRight");
    await expect(live(page)).toHaveText(/bearing:|too close/);
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

test.describe("a junction of a street made pedestrian", () => {
  test("loses the crossing of that street's arm", async ({ page }) => {
    await openStreet(page);
    const end = page.locator('#street-sub a[href^="intersection.html"]').first();
    const href = (await end.getAttribute("href")) ?? "";
    expect(href).not.toBe("");
    await end.click();
    await expect(page.locator("#drawing")).toBeVisible();
    const crossings = await page.locator("#drawing path.crossing").count();
    expect(crossings).toBeGreaterThan(0);
    await page.goBack();
    await page.locator("#transform-btn").click();
    await page.locator("#transform-menu .ped-item").first().click();
    // leaving the page keeps what was waiting to be kept
    await page.locator('#street-sub a[href^="intersection.html"]').first().click();
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator("#drawing path.crossing")).toHaveCount(crossings - 1);
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

  test("each phase is drawn, the Suggested label goes with an edit and returns with Restore, and a drawing and its street select each other", async ({
    page,
  }) => {
    const cards = page.locator("#inspector .signal-phases .phase-card");
    await expect(cards).toHaveCount(2);
    await expect(page.locator("#signal-plan-state")).toHaveText("Suggested plan");
    await expect(cards.first()).toContainText("Phase 1:");
    await expect(cards.first()).toContainText(/Cars go: /);
    await expect(cards.first().locator("svg.phase-svg")).toHaveAttribute("aria-hidden", "true");
    await expect(cards.first()).toHaveAccessibleName(/^Phase 1: .*Cars go: /);
    await expect(cards.first()).not.toHaveAttribute("aria-current");

    // Pressing a drawing selects its street in the inspector, and the drawing is pressed there.
    await cards.nth(1).focus();
    await page.keyboard.press("Enter");
    await expect(page.locator('#inspector input[aria-labelledby="i-h-green"]')).toBeVisible();
    await expect(cards.nth(1)).toHaveAttribute("aria-current", "true");
    await expect(cards.first()).not.toHaveAttribute("aria-current");
    // The panel swapped to the street's, and the focus is on the same card in it, not lost to the page.
    await expect(cards.nth(1)).toBeFocused();
    // The other drawing selects the other street, and keeps the focus too.
    await page.keyboard.press("Shift+Tab");
    await expect(cards.first()).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(cards.first()).toBeFocused();
    await cards.first().click();
    await expect(cards.first()).toHaveAttribute("aria-current", "true");
    await expect(cards.nth(1)).not.toHaveAttribute("aria-current");

    // Editing the plan drops the label; Restore brings it back.
    await page.locator("#wrap").focus();
    await page.keyboard.press("Escape");
    await expect(page.locator("#signal-plan-state")).toBeVisible();
    await page.locator("#sig-remove-1").click();
    await expect(page.locator("#signal-plan-state")).toHaveText("Edited plan");
    await expect(page.locator("#inspector .signal-phases .phase-card")).toHaveCount(1);
    await page.locator("#signal-restore").click();
    await expect(page.locator("#signal-plan-state")).toHaveText("Suggested plan");
    await expect(page.locator("#inspector .signal-phases .phase-card")).toHaveCount(2);
  });

  test("pressing the later card of a street that has two phases keeps the focus on that card", async ({ page }) => {
    const cards = page.locator("#inspector .signal-phases .phase-card");
    const before = await cards.count();
    await page
      .getByRole("button", { name: /^Add a phase for / })
      .first()
      .focus();
    await page.keyboard.press("Enter");
    await expect(live(page)).toContainText("added for");
    await expect(cards).toHaveCount(before + 1);
    const last = cards.nth(before);
    await last.focus();
    await page.keyboard.press("Enter");
    await expect(page.locator('#inspector input[aria-labelledby="i-h-green"]')).toBeVisible();
    // The street has two cards; only its first is marked, and the focus stays on the one that was pressed.
    await expect(last).toBeFocused();
    await expect(last).not.toHaveAttribute("aria-current");
    await expect(page.locator("#inspector .signal-phases .phase-card[aria-current='true']")).toHaveCount(1);
  });

  test("a phase is given a ready-made pattern from a radio group with one tab stop, and undo and redo follow", async ({
    page,
  }) => {
    const group = page.getByRole("radiogroup", { name: /^Pattern for phase 1, / });
    const radios = group.getByRole("radio");
    const names = await radios.allInnerTexts();
    expect(names.map((n) => n.trim())).toEqual([
      "Through only",
      "All movements",
      "Protected turn across traffic, then through",
      "Pedestrian scramble",
    ]);
    await expect(radios.last()).toHaveText(/Pedestrian scramble/);
    expect(await radios.count()).toBeGreaterThanOrEqual(2);
    // The suggestion is all of the street's movements; the checked radio is the one tab stop.
    await expect(group.locator('[role=radio][aria-checked="true"]')).toHaveCount(1);
    await expect(group.locator('[role=radio][tabindex="0"]')).toHaveCount(1);
    await expect(group.locator('[role=radio][aria-checked="true"]')).toHaveAttribute("tabindex", "0");
    const cards = page.locator("#inspector .signal-phases .phase-card");
    const second = await cards.nth(1).innerText();
    await expect(cards.first()).toContainText("Cars go:");
    await expect(group).toContainText("Every movement of the street goes");

    // End chooses the last pattern: nobody drives, everybody crosses. It is announced, the drawing's card follows, the
    // other card does not, and the focus stays on the group (on the chosen radio).
    await group.locator('[role=radio][aria-checked="true"]').focus();
    await page.keyboard.press("End");
    const scramble = group.getByRole("radio", { name: /Pedestrian scramble/ });
    await expect(scramble).toHaveAttribute("aria-checked", "true");
    await expect(scramble).toBeFocused();
    await expect(live(page)).toContainText(/Phase 1 for .* set to pedestrian scramble/);
    await expect(cards.first()).not.toContainText("Cars go:");
    await expect(cards.first()).toContainText("People cross:");
    expect(await cards.nth(1).innerText()).toBe(second);
    await expect(page.locator("#signal-plan-state")).toHaveText("Edited plan");

    // Home chooses the first; Arrow keys wrap. Each is one edit.
    await page.keyboard.press("Home");
    await expect(radios.first()).toHaveAttribute("aria-checked", "true");
    await expect(radios.first()).toBeFocused();
    await expect(cards.first()).toContainText("Cars go:");
    await page.keyboard.press("ArrowLeft");
    await expect(scramble).toHaveAttribute("aria-checked", "true");
    await expect(scramble).toBeFocused();
    await page.keyboard.press("ArrowRight");
    await expect(radios.first()).toHaveAttribute("aria-checked", "true");
    // Pressing a key that chooses the pattern already chosen says nothing more (Home on the first). The last
    // choice's announcement is waited for before the live region is emptied, or it could land after the emptying.
    await expect(live(page)).toContainText("set to through only");
    await live(page).evaluate((el) => (el.textContent = ""));
    await page.keyboard.press("Home");
    await expect(live(page)).toHaveText("");

    // Undo and redo step back through the choices.
    await page.locator("#undo").click();
    await expect(scramble).toHaveAttribute("aria-checked", "true");
    await page.locator("#undo").click();
    await expect(radios.first()).toHaveAttribute("aria-checked", "true");
    await page.locator("#redo").click();
    await expect(scramble).toHaveAttribute("aria-checked", "true");
    await expect(cards.first()).toContainText("People cross:");
  });

  test("a chosen pattern is kept across a reload, and the group is in French", async ({ page }) => {
    const group = page.getByRole("radiogroup", { name: /^Pattern for phase 1, / });
    await group.getByRole("radio", { name: /Pedestrian scramble/ }).click();
    await expect(page.locator("#signal-plan-state")).toHaveText("Edited plan");
    // The crossings are checked and cannot be changed; the group says why, and is described by it.
    const walks = page.getByRole("group", { name: /^People walking in phase 1, / });
    await expect(walks.getByRole("checkbox").first()).toBeDisabled();
    await expect(walks).toContainText(
      "This phase is a pedestrian scramble: everyone crosses every street. Choose another pattern to set crossings one by one.",
    );
    await expect(walks).toHaveAccessibleDescription(
      "This phase is a pedestrian scramble: everyone crosses every street. Choose another pattern to set crossings one by one.",
    );
    // The reason comes first, on a line of its own above the boxes.
    const noteBox = await walks.locator(".phase-moves-note").boundingBox();
    const firstBox = await walks.getByRole("checkbox").first().boundingBox();
    expect(noteBox!.y + noteBox!.height).toBeLessThanOrEqual(firstBox!.y);
    await page.reload();
    await expect(page.locator("#drawing")).toBeVisible();
    const again = page.getByRole("radiogroup", { name: /^Pattern for phase 1, / });
    await expect(again.getByRole("radio", { name: /Pedestrian scramble/ })).toHaveAttribute("aria-checked", "true");
    await expect(page.locator("#inspector .signal-phases .phase-card").first()).not.toContainText("Cars go:");
  });

  test("a phase whose movements match no pattern shows Custom, checked and not chosen, until a pattern is chosen", async ({
    page,
  }) => {
    const group = page.getByRole("radiogroup", { name: /^Pattern for phase 1, / });
    await group.getByRole("radio", { name: /Protected turn/ }).click();
    const custom = group.getByRole("radio", { name: /^Custom/ });
    await expect(custom).toHaveCount(0);
    // Switch on a crossing the pattern does not walk: the people the phase lets cross are no longer the pattern's.
    const off = page
      .getByRole("group", { name: /^People walking in phase 1, / })
      .locator("input:not(:checked)")
      .first();
    await off.focus();
    await page.keyboard.press("Space");
    // The edit says that the phase it changed no longer follows its pattern.
    await expect(live(page)).toContainText(/Phase 1 no longer follows a ready-made pattern\./);
    await expect(custom).toHaveCount(1);
    await expect(custom).toHaveAttribute("aria-checked", "true");
    await expect(custom).toHaveAttribute("aria-disabled", "true");
    await expect(group.locator('[role=radio][tabindex="0"]')).toHaveCount(1);
    await expect(group.getByRole("radio", { name: /Through only/ })).toHaveAttribute("aria-checked", "false");
    // Pressing Custom chooses nothing; an arrow key from it goes to a real pattern.
    await custom.click({ force: true });
    await expect(custom).toHaveCount(1);
    await custom.focus();
    await page.keyboard.press("Home");
    await expect(custom).toHaveCount(0);
    await expect(group.getByRole("radio").first()).toHaveAttribute("aria-checked", "true");
  });

  test("the pattern group speaks French and announces a choice in French", async ({ page }) => {
    await page.evaluate(() => localStorage.setItem("cityloom-lang", "fr-CA"));
    await page.reload();
    await expect(page.locator("#drawing")).toBeVisible();
    const group = page.getByRole("radiogroup", { name: /^Modèle de la phase 1, / });
    await expect(group).toBeVisible();
    await group.getByRole("radio", { name: /Phase piétonne exclusive/ }).click();
    await expect(live(page)).toContainText(/Phase 1 de .*\s: phase piétonne exclusive/);
  });

  test("a crossing is switched in a phase from a checklist, kept even where cars conflict, shows Custom, can be off everywhere and fails the check", async ({
    page,
  }) => {
    const walks = (n: number) => page.getByRole("group", { name: new RegExp(`^People walking in phase ${n}, `) });
    const boxes = walks(1).getByRole("checkbox");
    expect(await boxes.count()).toBeGreaterThanOrEqual(1);
    const card = page.locator("#inspector .signal-phases .phase-card").first().locator("svg.phase-svg");
    const patterns = page.getByRole("radiogroup", { name: /^Pattern for phase 1, / });
    await expect(patterns.getByRole("radio", { name: /^Custom/ })).toHaveCount(0);

    // The street whose own cars go in phase 1 has its crossing off there; switch it on with Space (a conflict: kept as chosen).
    const off = walks(1).locator("input:not(:checked)").first();
    await expect(off).toBeVisible();
    const id = await off.getAttribute("id");
    await expect(off).toHaveAccessibleName(/^People walk across /);
    const drawn = await card.innerHTML();
    await off.focus();
    await page.keyboard.press("Space");
    const box = page.locator(`#${id}`);
    await expect(box).toBeChecked();
    await expect(box).toBeFocused();
    await expect(live(page)).toContainText("now walk across");
    await expect(live(page)).toContainText(/Phase 1 no longer follows a ready-made pattern\./);
    await expect(page.locator("#signal-plan-state")).toHaveText("Edited plan");
    expect(await card.innerHTML()).not.toBe(drawn);
    await expect(patterns.getByRole("radio", { name: /^Custom/ })).toHaveAttribute("aria-checked", "true");

    // It is kept across a reload; undo and redo follow.
    await page.reload();
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator(`#${id}`)).toBeChecked();
    await page.locator("#undo").click();
    await expect(page.locator(`#${id}`)).not.toBeChecked();
    await expect(card).toBeVisible();
    await page.locator("#redo").click();
    await expect(page.locator(`#${id}`)).toBeChecked();

    // Every crossing off in every phase: the Checks list says the time to cross is short.
    const phases = await page.locator("#inspector .signal-phases .phase-card").count();
    for (let n = 1; n <= phases; n++) {
      const on = walks(n).locator("input:checked");
      while ((await on.count()) > 0) {
        await on.first().focus();
        await page.keyboard.press("Space");
      }
      await expect(walks(n).locator("input:checked")).toHaveCount(0);
    }
    await expect(live(page)).toContainText("no longer walk across");
    await page.locator("#t-checks").click();
    await expect(page.locator("#p-checks")).toContainText("Not enough time to cross at");
  });

  test("movements that meet in a phase are warned about on the card, the drawing and the checks, said aloud, never refused, and the warning goes with its cause", async ({
    page,
  }) => {
    const walks = page.getByRole("group", { name: /^People walking in phase 1, / });
    const card = page.locator("#inspector .signal-phases .phase-card").first();
    const warning = page.locator("#inspector .phase-conflicts");
    const meetCheck = page.locator("#p-checks li", { hasText: "Movements that meet" });

    // The suggested plan meets nothing: no list, no mark, a passing check.
    await expect(warning).toHaveCount(0);
    await expect(card.locator("svg.phase-svg .ph-move.warn")).toHaveCount(0);
    await page.locator("#t-checks").click();
    await expect(meetCheck).toHaveClass(/ok/);
    await expect(meetCheck).toContainText("No phase puts movements together that meet");

    // Switch on the crossing of the street whose own cars go in phase 1: allowed (the box stays checked), and warned about.
    const off = walks.locator("input:not(:checked)").first();
    const id = await off.getAttribute("id");
    await off.focus();
    await page.keyboard.press("Space");
    await expect(page.locator(`#${id}`)).toBeChecked();
    await expect(live(page)).toContainText(/Warning: /);
    await expect(live(page)).toContainText(/meet people walking across on the/);
    await expect(warning.first()).toContainText("Phase 1: cars from the");
    await expect(page.getByRole("list", { name: "Warnings for phase 1" })).toBeVisible();
    // The movement is dashed as well as coloured.
    const marked = card.locator("svg.phase-svg .ph-move.warn .ph-line");
    expect(await marked.count()).toBeGreaterThanOrEqual(1);
    await expect(marked.first()).toHaveAttribute("stroke-dasharray", /\d/);
    await page.locator("#t-checks").click();
    await expect(meetCheck).not.toHaveClass(/ok/);
    await expect(meetCheck).toContainText("meet people walking across");

    // A later edit that leaves the warning alone does not say it again.
    await page.locator("#undo").click();
    await page.locator("#redo").click();
    await page.evaluate(() => {
      document.querySelector("#live")!.textContent = "";
    });
    const other = page
      .getByRole("group", { name: /^People walking in phase 2, / })
      .locator("input:checked")
      .first();
    await other.focus();
    await page.keyboard.press("Space");
    await expect(live(page)).toContainText("no longer walk across");
    await expect(live(page)).not.toContainText("Warning");
    await page.keyboard.press("Space");

    // Removing the cause removes the warning.
    await page.locator(`#${id}`).focus();
    await page.keyboard.press("Space");
    await expect(page.locator(`#${id}`)).not.toBeChecked();
    await expect(warning).toHaveCount(0);
    await expect(card.locator("svg.phase-svg .ph-move.warn")).toHaveCount(0);
    await page.locator("#t-checks").click();
    await expect(meetCheck).toHaveClass(/ok/);
  });

  test("the warning about movements that meet is in French", async ({ page }) => {
    await page.evaluate(() => localStorage.setItem("cityloom-lang", "fr-CA"));
    await page.reload();
    await expect(page.locator("#drawing")).toBeVisible();
    const walks = page.getByRole("group", { name: /^Piétons en marche à la phase 1, / });
    const off = walks.locator("input:not(:checked)").first();
    await off.focus();
    await page.keyboard.press("Space");
    await expect(live(page)).toContainText(/Attention\s:\s/);
    await expect(live(page)).toContainText("rencontrent les piétons qui traversent du côté");
    await expect(page.getByRole("list", { name: "Avertissements de la phase 1" })).toBeVisible();
    await page.locator("#t-checks").click();
    await expect(page.locator("#p-checks li", { hasText: "Mouvements qui se rencontrent" })).toContainText(
      "rencontrent les piétons",
    );
  });

  test("the walking checklist is in French and announces a switch in French", async ({ page }) => {
    await page.evaluate(() => localStorage.setItem("cityloom-lang", "fr-CA"));
    await page.reload();
    await expect(page.locator("#drawing")).toBeVisible();
    const group = page.getByRole("group", { name: /^Piétons en marche à la phase 1, / });
    await expect(group).toBeVisible();
    await group.locator("input:not(:checked)").first().focus();
    await page.keyboard.press("Space");
    await expect(live(page)).toContainText(/traversent maintenant/);
    await expect(live(page)).toContainText(/La phase 1 ne suit plus un modèle prêt à l’emploi\./);
  });

  test("a phase drawing is small, and two cards share a row where the inspector is wide enough", async ({ page }) => {
    const cards = page.locator("#inspector .signal-phases .phase-card");
    await expect(cards).toHaveCount(2);
    const svg = await cards.first().locator("svg.phase-svg").boundingBox();
    expect(svg!.height).toBeLessThanOrEqual(160);
    expect(svg!.width).toBeLessThanOrEqual(160);
    const [a, b] = [await cards.nth(0).boundingBox(), await cards.nth(1).boundingBox()];
    const room = (await page.locator("#inspector .signal-phases").boundingBox())!.width;
    expect(room).toBeGreaterThanOrEqual(2 * 120 + 8); // the e2e window is wide enough for two
    expect(Math.abs(a!.y - b!.y)).toBeLessThan(2);
  });

  test("the row of drawings fits a 900 px window and works in forced colours", async ({ page }) => {
    await page.setViewportSize({ width: 900, height: 800 });
    await page.emulateMedia({ forcedColors: "active" });
    const cards = page.locator("#inspector .signal-phases .phase-card");
    await expect(cards).toHaveCount(2);
    await cards.first().click();
    await expect(cards.first()).toHaveAttribute("aria-current", "true");
    await expect(cards.first()).toHaveCSS("outline-style", "solid");
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth),
    ).toBe(true);
    await page.setViewportSize({ width: 360, height: 800 });
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth),
    ).toBe(true);
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

  test("stepping a time with the keyboard keeps the focus on the button, so it can be stepped again", async ({
    page,
  }) => {
    await page.locator("#inspector .signal-list button").first().click();
    const green = page.locator("#inspector section", { has: page.locator("#i-h-green") });
    const longer = green.getByRole("button", { name: "Longer by 1 s" });
    await longer.focus();
    await page.keyboard.press("Enter");
    await expect(green.locator("input")).toHaveValue("26");
    await expect(longer).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(green.locator("input")).toHaveValue("27");
  });

  test("phases are added, moved, removed and restored from the keyboard, with undo, redo, announcements and a reload", async ({
    page,
  }) => {
    const rows = page.locator("#inspector .signal-list > li");
    const state = page.locator("#signal-plan-state");
    // The city's junction has two streets that traffic enters from, so two phases.
    await expect(rows).toHaveCount(2);
    await expect(state).toHaveText("Suggested plan");
    await expect(page.locator("#signal-restore")).toBeDisabled();
    const firstStreet = (await rows.first().locator("button").first().innerText()).split(":")[0];
    const lastStreet = (await rows.last().locator("button").first().innerText()).split(":")[0];

    // Move the first phase later with the keyboard alone; the focus follows it.
    await page.locator("#sig-later-0").focus();
    await page.keyboard.press("Enter");
    await expect(live(page)).toContainText("moved later, now 2 of 2");
    await expect(page.locator("#sig-earlier-1")).toBeFocused();
    await expect(state).toHaveText("Edited plan");
    await expect(rows.nth(1).locator("button").first()).toContainText(firstStreet);
    await page.keyboard.press("Enter");
    await expect(live(page)).toContainText("moved earlier, now 1 of 2");
    await expect(page.locator("#sig-later-0")).toBeFocused();
    await expect(rows.first().locator("button").first()).toContainText(firstStreet);
    // The first phase cannot go earlier, nor the last later: those buttons are off.
    await expect(page.locator("#sig-earlier-0")).toBeDisabled();
    await expect(page.locator("#sig-later-1")).toBeDisabled();

    // Remove the last one: the street it served is flagged, and a phase for it can be added back.
    await page.locator("#sig-remove-1").focus();
    await page.keyboard.press("Enter");
    await expect(rows).toHaveCount(1);
    await expect(live(page)).toContainText("removed");
    await expect(page.locator("#sig-remove-0")).toBeFocused();
    await page.locator("#t-checks").click();
    await expect(page.locator("#p-checks")).toContainText("No phase serves the traffic entering from");
    await page
      .getByRole("button", { name: new RegExp(`^Add a phase for ${lastStreet.replace(/[()]/g, "\\$&")}`) })
      .focus();
    await page.keyboard.press("Enter");
    await expect(rows).toHaveCount(2);
    await expect(live(page)).toContainText("added for");

    // Undo and redo take the steps back and forward.
    await page.locator("#undo").click();
    await expect(rows).toHaveCount(1);
    await page.locator("#redo").click();
    await expect(rows).toHaveCount(2);

    // A reload keeps the edited plan.
    await page.reload();
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator("#signal-plan-state")).toHaveText("Edited plan");
    await expect(page.locator("#inspector .signal-list > li")).toHaveCount(2);

    // Restore the suggestion.
    await page.locator("#signal-restore").focus();
    await page.keyboard.press("Enter");
    await expect(live(page)).toContainText("restored to the suggestion");
    await expect(page.locator("#signal-plan-state")).toHaveText("Suggested plan");
    await expect(page.locator("#signal-restore")).toBeDisabled();
    // The button that held the focus is disabled now, so the focus moves to the plan's state, not to the body.
    await expect(state).toBeFocused();
  });

  test("the last phase stays, and the phase buttons speak French", async ({ page }) => {
    await page.locator("#account-btn").click();
    await page.getByRole("button", { name: "Français" }).click();
    await expect(page.locator("#signal-plan-state")).toHaveText("Plan suggéré");
    await page.locator("#sig-later-0").focus();
    await page.keyboard.press("Enter");
    await expect(live(page)).toContainText("reculée, maintenant 2 sur 2");
    await expect(page.locator("#signal-plan-state")).toHaveText("Plan modifié");
    await page.locator("#sig-remove-0").focus();
    await page.keyboard.press("Enter");
    await expect(page.locator("#inspector .signal-list > li")).toHaveCount(1);
    await page.locator("#sig-remove-0").focus();
    await page.keyboard.press("Enter");
    await expect(live(page)).toContainText("garde au moins une phase");
    await page.locator("#signal-restore").focus();
    await page.keyboard.press("Enter");
    await expect(live(page)).toContainText("rétabli à la suggestion");
  });

  test("the timing is French in French", async ({ page }) => {
    await page.locator("#account-btn").click();
    await page.getByRole("button", { name: "Français" }).click();
    await expect(page.locator("#inspector")).toContainText("Réglage des feux");
    await expect(page.locator("#inspector")).toContainText(/Cycle\s*:\s*\d+\s*s/);
  });
});

// Gives the one crossing of `fixtures/turbo.osm.pbf` (four streets with room for two entry lanes each) the basic turbo
// layout from its details. The Plateau's own junctions have no room for it, so the tile is served in their place.
async function basicTurbo(page: Page) {
  await page.route("**/data/metro/0_0.osm.pbf", (route) => route.fulfill({ body: readFileSync(TURBO_PBF) }));
  await openJunction(page);
  await expect(page.locator("#arm-count")).toHaveText("4 streets");
  await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Roundabout" });
  await page.getByRole("radio", { name: "Basic", exact: true }).click();
}

test("a roundabout is given the basic turbo layout from its details and drawn with its painted divider", async ({
  page,
}) => {
  await basicTurbo(page);
  // The divider is cut where each street meets the ring, so it is drawn in several open pieces (more than one; the count depends on the crossing).
  const dividers = page.locator("#drawing .turbo-divider");
  await expect(dividers.first()).toBeAttached();
  expect(await dividers.count()).toBeGreaterThan(1);
  expect(await dividers.evaluateAll((els) => els.every((e) => !/Z/.test(e.getAttribute("d")!)))).toBe(true);
  await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /Basic turbo roundabout, \w+ size\.$/);
  await expect(page.getByRole("radio", { name: "Basic", exact: true })).toHaveAttribute("aria-checked", "true");
  await expect(page.getByText("Turbo size")).toBeVisible();
  await page.locator("#undo").click();
  await expect(page.locator("#drawing .turbo-divider")).toHaveCount(0);
});

test("a turbo roundabout shows its painted dividers and its name in 2.5D", async ({ page }) => {
  await basicTurbo(page);
  await page.locator("#view-iso").click();
  const paint = page.locator("#drawing .iso-paint.white").first();
  await expect(paint).toBeAttached();
  await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /Basic turbo roundabout, \w+ size\.$/);
  // In the light colour scheme the paint carries its dark edge.
  await page.emulateMedia({ colorScheme: "light" });
  const stroke = await paint.evaluate((el) => getComputedStyle(el).stroke);
  expect(stroke).not.toMatch(/^(none|transparent|rgba\(0, 0, 0, 0\))$/);
});

test("the turbo layouts are a radio group: one tab stop, the arrow keys choose, re-choosing says nothing", async ({
  page,
}) => {
  await basicTurbo(page);
  const basic = page.getByRole("radio", { name: "Basic", exact: true });
  await basic.focus();
  await expect(basic).toBeFocused();
  await expect(basic).toHaveAttribute("aria-checked", "true");

  // Choosing the layout already chosen changes nothing: the live region is unchanged, and the one Undo that follows
  // goes back to the plain roundabout, so the choice added no history step.
  // (The live region fills 40 ms after a change, so wait for the first choice's words and again before comparing.)
  await expect(live(page)).toContainText("basic turbo");
  const said = await live(page).innerText();
  await page.keyboard.press("Enter");
  await page.waitForTimeout(250);
  await expect(basic).toHaveAttribute("aria-checked", "true");
  expect(await live(page).innerText()).toBe(said);
  await page.locator("#undo").click();
  await expect(page.locator("#drawing .turbo-divider")).toHaveCount(0);
  await expect(basic).toHaveAttribute("aria-checked", "false");

  await basic.click();
  await expect(basic).toHaveAttribute("aria-checked", "true");
  await expect(basic).toBeFocused();

  await page.keyboard.press("ArrowDown");
  await expect(basic).toHaveAttribute("aria-checked", "false");
  const checked = page.locator('[role="radio"][aria-checked="true"]');
  await expect(checked).toHaveCount(1);
  await expect(checked).toBeFocused();
  await expect(page.locator('[role="radio"][tabindex="0"]')).toHaveCount(1);

  await page.keyboard.press("ArrowUp");
  await expect(basic).toHaveAttribute("aria-checked", "true");
  await expect(basic).toBeFocused();
  await expect(page.locator('[role="radio"][tabindex="0"]')).toHaveCount(1);
});

// ---- approaches: what lies beyond an arm's end zone ---------------------------------------------------------

test.describe("the approach of an arm whose street is divided", () => {
  const options = (page: Page) => page.locator('#zone-strip [role="option"]');

  /** Finds a junction with a street of `minM` to `maxM` metres leaving it, by the first arm's "Open the cross-section"
   *  link (and at most `maxM`, so that half of it is inside the 36 m an arm is drawn). Leaves the page on that street, and says the junction's address and the street's. */
  const junctionWithStreet = async (page: Page, minM: number, maxM: number) => {
    await page.goto("/map.html");
    for (const junction of (await placesOnMap(page)).junctions) {
      await page.goto("/" + junction);
      await expect(page.locator("#drawing")).toBeVisible();
      await page.locator("#wrap").focus();
      await page.keyboard.press("ArrowRight"); // the first street
      const link = page.locator('#inspector a.btn[href^="street.html?street="]');
      await expect(link).toBeVisible();
      const street = await link.getAttribute("href");
      await link.click();
      await expect(page.locator("#zone-0")).toBeVisible();
      const length = await streetLengthM(page);
      if (length >= minM && length <= maxM) return { junction, street: street! };
    }
    throw new Error(`no junction has a street of ${minM} to ${maxM} m`);
  };

  /** Zone 2 must differ from zone 1 in its section. Removing the first piece (a sidewalk) does that in one exact step:
   *  the zone is left with width to use, which `Street::beyond` draws (only a zone wider than the street draws none).
   *  The fit text is awaited, not the key presses counted. */
  const changeZone = async (page: Page) => {
    const fit = page.locator("#fit");
    await page.locator("#wrap").focus();
    await page.keyboard.press("Escape");
    await page.keyboard.press("ArrowRight"); // the first piece
    await page.keyboard.press("Delete");
    await expect(fit).toContainText(/m of the street is still unused\./);
    await expect(fit).not.toContainText(/too wide/);
  };

  const checkRow = (page: Page) => page.locator("#checks li", { hasText: "Approaches" });

  test("draws the transition and its marks, passes the check, then fails it when the end zone is short", async ({
    page,
  }) => {
    // It looks for a junction with a street of the right length, then goes between the street and the junction pages
    // up to six times: about 30 s on CI's runner, the default timeout.
    test.setTimeout(120_000);
    // Halved, the street has an end zone of 25 to 33 m: the boundary lies inside the 36 m an arm is drawn to.
    const { junction, street } = await junctionWithStreet(page, 50, 66);
    await splitInto(page, 2);
    // Zone 2 loses its first piece, so its section differs from zone 1's.
    await page.locator("#zone-1").click();
    await changeZone(page);
    await page.waitForTimeout(500); // the debounced write (250 ms) has nothing observable to await

    await page.goto("/" + junction);
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator("#drawing .piece.taper").first()).toBeAttached();
    await expect(page.locator("#drawing .approach-mark")).toHaveCount(2);
    await page.locator("#t-checks").click();
    await expect(checkRow(page)).toBeVisible();
    await expect(checkRow(page)).not.toHaveClass(/bad/);
    await expect(checkRow(page)).toContainText("Every end zone reaches past its stop line");

    // The first press selects the street we divided: the arm says where its end zone reaches.
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await expect(live(page)).toContainText(
      /Its end zone reaches .* from the centre, then a transition of .* into Zone/,
    );
    const arm = (await live(page).innerText()).split(" (")[0];

    // Cut the end zone short from the street page: push the boundary one way, then the other, until the zone at the
    // junction is under what its stop line and arrows need.
    let failed = false;
    for (const [key, presses] of [
      ["Shift+Alt+ArrowRight", 12],
      ["Shift+Alt+ArrowLeft", 24],
    ] as const) {
      await page.goto("/" + street);
      await expect(options(page)).toHaveCount(2);
      await page.locator("#zone-strip").focus();
      await page.keyboard.press("ArrowRight"); // zone 2
      const before = await options(page).first().getAttribute("aria-label");
      for (let i = 0; i < presses; i++) await page.keyboard.press(key);
      // The presses are handled synchronously; the zone's label (its length) shows they took effect.
      await expect(options(page).first()).not.toHaveAttribute("aria-label", before!);
      await page.waitForTimeout(500); // the debounced write (250 ms) has nothing observable to await
      await page.goto("/" + junction);
      await expect(page.locator("#drawing")).toBeVisible();
      await page.locator("#t-checks").click();
      if (await checkRow(page).evaluate((e) => e.classList.contains("bad"))) {
        failed = true;
        await expect(checkRow(page)).toContainText(arm);
        await expect(checkRow(page)).toContainText(/ends .* from the centre, short of the/);
        break;
      }
    }
    expect(failed, "a short end zone fails the Approaches check").toBe(true);
  });
});

// ---- transit stops on an arm ---------------------------------------------------------------------------------

test.describe("a street's transit stop on the arm that meets it", () => {
  const STOP_M = 12;
  const clear = "Bus stops clear of the junction";
  const lanes = "Buses at a stop are not trapped in a turning lane";

  /** Finds a junction whose first arm has two entering lanes (a stop's lane can then be trapped in a turning lane)
   *  and a street of 50 m or more with a Stops row on the side where traffic enters the junction. Traffic keeps
   *  right, so a bus enters on the street's left half where the junction is at the end the stops are measured from,
   *  and on its right half at the other end. The end is read from the street page: it says it measures from the
   *  junction's own name when the junction lies at that end. Says where the stop goes. Leaves the page on the street. */
  const nearSideArm = async (page: Page) => {
    await page.goto("/map.html");
    for (const junction of (await placesOnMap(page)).junctions) {
      await page.goto("/" + junction);
      await expect(page.locator("#drawing")).toBeVisible();
      const name = (await page.locator("#street-name").innerText()).trim();
      await page.locator("#wrap").focus();
      await page.keyboard.press("ArrowRight"); // the first street
      const link = page.locator('#inspector a.btn[href^="street.html?street="]');
      await expect(link).toBeVisible();
      if ((await page.locator("#inspector .lane-row").count()) < 2) continue;
      await link.click();
      await expect(page.locator("#zone-0")).toBeVisible();
      const length = await streetLengthM(page);
      const from = (
        await page
          .getByText(/^Measured from /)
          .first()
          .innerText()
      ).trim();
      const origin = from === `Measured from ${name}`;
      const side = origin ? "left" : "right";
      if (length >= 50 && (await page.locator(`#stop-strip-${side}`).count()) === 1) {
        return { junction, length, origin, side };
      }
    }
    throw new Error("no junction has an arm with two entering lanes and a street with a stop row on its entering side");
  };

  /** Adds a stop of `STOP_M` metres on the street page's `side` row, `near` metres from the junction's centre to its
   *  near edge. The stop is added at the street's origin, then moved a step at a time, awaiting the label each time. */
  const placeStop = async (page: Page, arm: { length: number; origin: boolean; side: string }, near: number) => {
    const strip = page.locator(`#stop-strip-${arm.side}`);
    const used = strip.locator('[role="option"].stop-cell:not(.gap)');
    await page.locator(`#stop-${arm.side}-0`).click();
    await page.locator("#stop-length").fill(String(STOP_M));
    await page.locator("#stop-add").click();
    await expect(used).toHaveCount(1);
    await expect(used).toHaveAttribute("aria-label", new RegExp(`, 0\\.0 m to ${STOP_M}\\.0 m,`));
    await used.click();
    await strip.focus();
    await moveStop(page, strip, 0, arm.origin ? near : Math.floor(arm.length - STOP_M - near));
  };

  /** Moves the stop in `strip` from `at` to `target` metres along the street, by 10 m (Shift+Alt) and 1 m (Alt). */
  const moveStop = async (page: Page, strip: Locator, at: number, target: number) => {
    const used = strip.locator('[role="option"].stop-cell:not(.gap)');
    while (at !== target) {
      const d = target - at;
      const big = Math.abs(d) >= 10;
      const step = Math.sign(d) * (big ? 10 : 1);
      await page.keyboard.press(`${big ? "Shift+" : ""}Alt+Arrow${step > 0 ? "Right" : "Left"}`);
      at += step;
      await expect(used).toHaveAttribute("aria-label", new RegExp(`, ${at}\\.0 m to ${at + STOP_M}\\.0 m,`));
    }
  };

  /** Waits until the document in storage holds a stop that starts at `fromM` metres (the write is debounced). */
  const kept = (page: Page, fromM: number) =>
    expect
      .poll(() =>
        page.evaluate((m) => (localStorage.getItem("cityloom-edits") ?? "").includes(`"from_mm":${m * 1000}`), fromM),
      )
      .toBe(true);

  const checkRow = (page: Page, label: string) => page.locator("#checks li").filter({ hasText: label });

  test("a stop too near the junction is drawn and fails the checks, and passes 20 m out", async ({ page }) => {
    // It looks through the junctions for an arm, then goes between the street and the junction pages four times.
    test.setTimeout(120_000);
    const arm = await nearSideArm(page);
    const street = page.url();

    // No stop on the street: nothing drawn on the arm.
    await page.goto("/" + arm.junction);
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator("#drawing .stop-pad")).toHaveCount(0);

    // A stop 6 m from the centre.
    await page.goto(street);
    await expect(page.locator("#zone-0")).toBeVisible();
    await placeStop(page, arm, 6);
    const near = arm.origin ? 6 : Math.floor(arm.length - STOP_M - 6);
    await kept(page, near);

    await page.goto("/" + arm.junction);
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator('#drawing g[data-role="arm"] .stop-pad')).toHaveCount(1);
    await expect(page.locator('#drawing g[data-role="arm"] .stop-dwell')).toHaveCount(1);
    await page.locator("#t-checks").click();
    await expect(checkRow(page, clear)).toHaveClass("bad");
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight"); // the street
    await expect(live(page)).toContainText(/from the centre/);
    const name = (await live(page).innerText()).split(" (")[0];
    await expect(checkRow(page, clear)).toContainText(name);

    // The street page reads the junctions when it opens: with the stop kept, it now judges it, and the same stop
    // 20 m out is clear on both pages.
    await page.goto(street);
    await expect(page.locator("#zone-0")).toBeVisible();
    await page.locator("#t-checks").click();
    await expect(checkRow(page, "Stops clear of the junction")).toHaveClass("bad");
    const strip = page.locator(`#stop-strip-${arm.side}`);
    await strip.locator('[role="option"].stop-cell:not(.gap)').click();
    await strip.focus();
    const to = arm.origin ? 20 : Math.floor(arm.length - STOP_M - 20);
    await moveStop(page, strip, near, to);
    await kept(page, to);
    await expect(checkRow(page, "Stops clear of the junction")).toHaveClass("ok");

    await page.goto("/" + arm.junction);
    await expect(page.locator('#drawing g[data-role="arm"] .stop-pad')).toHaveCount(1);
    await page.locator("#t-checks").click();
    await expect(checkRow(page, clear)).not.toHaveClass(/bad/);
    await expect(checkRow(page, lanes)).not.toHaveClass(/bad/);
  });

  test("a bus stopped in a lane that only turns fails, and passes when the lane goes straight on", async ({ page }) => {
    test.setTimeout(120_000);
    const arm = await nearSideArm(page);
    await placeStop(page, arm, 20);
    await kept(page, arm.origin ? 20 : Math.floor(arm.length - STOP_M - 20));

    await page.goto("/" + arm.junction);
    await expect(page.locator('#drawing g[data-role="arm"] .stop-pad')).toHaveCount(1);
    await page.locator("#t-checks").click();
    await expect(checkRow(page, lanes)).not.toHaveClass(/bad/);

    // The curb lane is the last: it turns left only, then goes straight on again.
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    const curb = page.locator("#inspector .lane-row").last();
    await expect(curb.locator("button.dest")).toHaveCount(2);
    const [turn, ahead] = [
      curb.getByRole("button", { name: /, left$/ }),
      curb.getByRole("button", { name: /, straight on$/ }),
    ];
    await turn.click();
    await expect(turn).toHaveAttribute("aria-pressed", "true");
    await ahead.click();
    await expect(ahead).toHaveAttribute("aria-pressed", "false");
    await expect(checkRow(page, lanes)).toHaveClass(/bad/);
    await expect(checkRow(page, lanes)).toContainText("only turns left");

    await ahead.click();
    await expect(ahead).toHaveAttribute("aria-pressed", "true");
    await turn.click();
    await expect(turn).toHaveAttribute("aria-pressed", "false");
    await expect(checkRow(page, lanes)).not.toHaveClass(/bad/);
  });
});

// ---- curbside uses on an arm ---------------------------------------------------------------------------------

test.describe("a street's curbside use on the arm that meets it", () => {
  const PARKING = "0";
  const USE_M = 6;
  const label = "Curbside clear of crossings";

  /** Finds a junction whose first street is measured from the junction itself (so a use placed at the street's
   *  origin stands at the junction's centre) and has a curbside strip. Leaves the page on the street. */
  const originArm = async (page: Page) => {
    await page.goto("/map.html");
    for (const junction of (await placesOnMap(page)).junctions) {
      await page.goto("/" + junction);
      await expect(page.locator("#drawing")).toBeVisible();
      const name = (await page.locator("#street-name").innerText()).trim();
      await page.locator("#wrap").focus();
      await page.keyboard.press("ArrowRight");
      const link = page.locator('#inspector a.btn[href^="street.html?street="]');
      await expect(link).toBeVisible();
      await link.click();
      await expect(page.locator("#zone-0")).toBeVisible();
      const from = (
        await page
          .getByText(/^Measured from /)
          .first()
          .innerText()
      ).trim();
      if (
        from === `Measured from ${name}` &&
        (await streetLengthM(page)) >= 50 &&
        (await page.locator(".curb-strip").count()) > 0
      ) {
        return { junction, street: page.url() };
      }
    }
    throw new Error("no junction has a first street measured from it with a curbside lane");
  };

  const kept = (page: Page, fromM: number) =>
    expect
      .poll(() =>
        page.evaluate((m) => (localStorage.getItem("cityloom-edits") ?? "").includes(`"from_mm":${m * 1000}`), fromM),
      )
      .toBe(true);

  const checkRow = (page: Page) => page.locator("#checks li").filter({ hasText: label });

  test("parking at the centre fails the check, and drawn 30 m out it passes", async ({ page }) => {
    test.setTimeout(120_000);
    const arm = await originArm(page);

    await page.goto("/" + arm.junction);
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator("#drawing .curb-use")).toHaveCount(0);

    // A parking use at the street's origin: it stands on the crossing, which the junction says.
    await page.goto(arm.street);
    await expect(page.locator("#zone-0")).toBeVisible();
    const strip = page.locator(".curb-strip").first();
    const cells = strip.locator('[role="option"]');
    await cells.nth(0).click();
    await page.locator("#curb-use").selectOption(PARKING);
    await page.locator("#curb-length").fill(String(USE_M));
    await page.locator("#curb-add").click();
    await expect(cells).toHaveCount(2);
    await kept(page, 0);

    await page.goto("/" + arm.junction);
    await expect(page.locator("#drawing")).toBeVisible();
    await page.locator("#t-checks").click();
    await expect(checkRow(page)).toHaveClass("bad");

    // Moved 30 m out, it is drawn on the arm and clear.
    await page.goto(arm.street);
    await expect(page.locator("#zone-0")).toBeVisible();
    await cells.nth(0).click();
    await strip.focus();
    for (let i = 0; i < 3; i++) await page.keyboard.press("Shift+Alt+ArrowRight");
    await kept(page, 30);

    await page.goto("/" + arm.junction);
    await expect(page.locator("#drawing")).toBeVisible();
    await expect(page.locator('#drawing g[data-role="arm"] .curb-use.use-parking')).toHaveCount(1);
    await page.locator("#t-checks").click();
    await expect(checkRow(page)).not.toHaveClass(/bad/);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await expect(live(page)).toContainText(/Parking .*30\.0 m from the centre/);
  });
});
