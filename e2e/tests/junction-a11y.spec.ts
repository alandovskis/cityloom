import { expect, live, openJunction, openSignalFolds, test } from "./fixtures";

import type { Page } from "@playwright/test";

// The junction page's one drawing, the 2.5D, as a keyboard, small-window and touch user meets it (WCAG 2.2 AA, #140).
//
// Target size (2.5.8, 24 by 24 px): a part of the drawing is as big as the drawing makes it, and the drawing is a picture of a
// junction at one scale, so a lane or a corner can be thinner than a finger. Each part that is pressed is grown by an unpainted
// stroke to 24 px where it is thinner (`target_growth_px`); where its neighbours leave it less, what meets the criterion is
// the equivalent control: the two buttons under the drawing, which step through every part as the arrow keys do, and the
// keys. What the old grips dragged is in the inspector's fields (`tests.rs` and the 2.5D spec of junction.spec.ts).

/** Where each pressable part of the drawing can be pressed, as the pointer finds it: the page is asked what is on top at points of
 *  the drawing, a grid 3 px apart, and for each part (its role, uid and lane) the side of the largest square of those points
 *  that are all that part's. That is how big a finger's target is, whatever is drawn over a part's shape. */
async function targetSides(page: Page) {
  return page.evaluate(() => {
    const svg = document.querySelector("#drawing")!;
    const box = svg.getBoundingClientRect();
    const step = 3;
    const cols = Math.floor(box.width / step);
    const rows = Math.floor(box.height / step);
    const keys: (string | null)[][] = [];
    for (let y = 0; y < rows; y++) {
      const row: (string | null)[] = [];
      for (let x = 0; x < cols; x++) {
        const el = document.elementFromPoint(box.left + x * step + step / 2, box.top + y * step + step / 2);
        const part = el?.closest("#drawing [data-role]");
        row.push(
          part
            ? [
                part.getAttribute("data-role"),
                part.getAttribute("data-uid") ?? "",
                part.getAttribute("data-lane") ?? "",
              ].join("|")
            : null,
        );
      }
      keys.push(row);
    }
    // The largest square of one part's points: each cell holds the side of the largest square ending at it.
    const best = new Map<string, number>();
    const side: number[][] = keys.map((r) => r.map(() => 0));
    for (let y = 0; y < rows; y++) {
      for (let x = 0; x < cols; x++) {
        const k = keys[y][x];
        if (!k) continue;
        const near = (dy: number, dx: number) =>
          y + dy >= 0 && x + dx >= 0 && keys[y + dy][x + dx] === k ? side[y + dy][x + dx] : 0;
        side[y][x] = 1 + Math.min(near(-1, 0), near(0, -1), near(-1, -1));
        best.set(k, Math.max(best.get(k) ?? 0, side[y][x]));
      }
    }
    return [...best.entries()].map(([key, cells]) => ({ key: key.split("|")[0], side: cells * step }));
  });
}

/** What the keys say as they step through every part of the junction, once round. */
async function stepThrough(page: Page, press: () => Promise<void>, times: number) {
  const said: string[] = [];
  for (let i = 0; i < times; i++) {
    const before = await live(page).innerText();
    await press();
    // The status is emptied before it says what is selected: wait for the new words, not for the gap.
    await expect(live(page)).not.toHaveText(before);
    await expect(live(page)).toHaveText(/\S/);
    said.push(await live(page).innerText());
  }
  return said;
}

const KINDS: [string, RegExp][] = [
  ["a street", /degrees$/],
  ["a lane", /^Lane \d+ of \d+, /],
  ["a crossing", /^(Crossing on|Continuous sidewalk across) /],
  ["a corner", /^(Protected corner|Corner) after /],
];

test.describe("the junction's parts are reached without a pointer, and with one that is not exact", () => {
  test("the arrow keys step through every kind of part, and the inspector opens the fields the old grips dragged", async ({
    page,
  }) => {
    await openJunction(page);
    await page.locator("#wrap").focus();
    const said = await stepThrough(page, () => page.keyboard.press("ArrowRight"), 24);
    for (const [name, pattern] of KINDS)
      expect(
        said.some((t) => pattern.test(t)),
        `the keys reach ${name}`,
      ).toBe(true);
    // A street, a corner and a crossing each show the field that sets what its grip dragged.
    const field = (id: string) => page.locator(`#inspector input[aria-labelledby="${id}"]`);
    for (const [pattern, id] of [
      [KINDS[0][1], "i-h-bearing"],
      [KINDS[3][1], "i-h-corner"],
      [KINDS[2][1], "i-h-setback"],
    ] as const) {
      let found = false;
      for (let i = 0; i < 24 && !found; i++) {
        const [t] = await stepThrough(page, () => page.keyboard.press("ArrowRight"), 1);
        if (pattern.test(t)) {
          // a street's direction is in its Position fold
          if (id === "i-h-bearing") await page.locator("#jn-fold-position summary").click();
          await expect(field(id)).toBeVisible();
          await page.locator("#wrap").focus();
          found = true;
        }
      }
      expect(found, id).toBe(true);
    }
  });

  test("a roundabout's cycle track and bus lane are reached by the keys too", async ({ page }) => {
    await openJunction(page);
    await page.locator("#inspector select").first().selectOption({ label: "Roundabout" });
    await page.getByRole("checkbox", { name: /Track around the outside/ }).click();
    await page.getByLabel("Bus lane through the middle").selectOption({ index: 1 });
    await page.locator("#wrap").focus();
    const said = await stepThrough(page, () => page.keyboard.press("ArrowRight"), 24);
    expect(said.some((t) => /^Cycle track, /.test(t))).toBe(true);
    expect(said.some((t) => t === "Bus lane across the middle")).toBe(true);
  });

  test("the two buttons under the drawing step through the same parts, forwards and back", async ({ page }) => {
    await openJunction(page);
    const next = page.locator("#part-next");
    const previous = page.locator("#part-previous");
    await expect(next).toHaveText("Next part");
    const forward = await stepThrough(page, () => next.click(), 24);
    for (const [name, pattern] of KINDS)
      expect(
        forward.some((t) => pattern.test(t)),
        `the button reaches ${name}`,
      ).toBe(true);
    // Back goes to what the last step left.
    const back = await stepThrough(page, () => previous.click(), 1);
    expect(back[0]).toBe(forward[forward.length - 2]);
    await expect(page.locator("#drawing .iso-sel")).not.toHaveCount(0);
  });

  test("a key pressed on a step button is the button's, not the drawing's: it turns, resizes and removes nothing", async ({
    page,
  }) => {
    await openJunction(page);
    await page.locator("#part-next").click();
    await expect(live(page)).toHaveText(/degrees$/);
    await expect(page.locator("#part-next")).toBeFocused();
    for (const key of [
      "Shift+ArrowRight",
      "Shift+ArrowLeft",
      "+",
      "-",
      "Delete",
      "Backspace",
      "ArrowRight",
      "Escape",
    ]) {
      await page.keyboard.press(key);
    }
    await page.waitForTimeout(250);
    // Nothing was edited, and what is selected is still the street the button chose.
    await expect(page.locator("#undo")).toBeDisabled();
    await expect(page.locator("#drawing .iso-sel")).toHaveCount(1);
    // Enter and Space still press the button.
    const before = await live(page).innerText();
    await page.keyboard.press("Enter");
    await expect(live(page)).not.toHaveText(before);
    await page.keyboard.press("Space");
    await expect(page.locator("#undo")).toBeDisabled();
    // On the drawing itself the keys edit, as ever.
    await page.locator("#wrap").focus();
    await page.keyboard.press("Escape");
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Shift+ArrowRight");
    if (!(await page.locator("#undo").isEnabled())) await page.keyboard.press("Shift+ArrowLeft");
    await expect(page.locator("#undo")).toBeEnabled();
  });

  for (const width of [1440, 1050, 600, 280]) {
    test(`at ${width} px the step buttons are 24 px or more and each kind of part has a place that is nearly as big`, async ({
      page,
    }) => {
      await page.setViewportSize({ width, height: 900 });
      await openJunction(page);
      await expect(page.locator("#drawing .iso-junction")).toHaveCount(1);
      for (const id of ["#part-previous", "#part-next"]) {
        await page.locator(id).scrollIntoViewIfNeeded();
        const b = (await page.locator(id).boundingBox())!;
        expect(b.width, `${id} wide at ${width}`).toBeGreaterThanOrEqual(24);
        expect(b.height, `${id} high at ${width}`).toBeGreaterThanOrEqual(24);
      }
      await page.locator("#drawing").scrollIntoViewIfNeeded();
      const best = new Map<string, number>();
      for (const { key, side } of await targetSides(page)) best.set(key, Math.max(best.get(key) ?? 0, side));
      // A regression floor, not the criterion: the largest place of a kind (the others are covered by the buttons and keys).
      for (const kind of ["arm", "lane", "crossing", "corner"]) {
        expect(best.get(kind) ?? 0, `${kind} at ${width}`).toBeGreaterThanOrEqual(width >= 600 ? 18 : 9);
      }
    });
  }

  test("at 280 px the page does not scroll sideways and the drawing fits the window", async ({ page }) => {
    await page.setViewportSize({ width: 280, height: 700 });
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await expect
      .poll(async () => {
        const b = (await page.locator("#drawing").boundingBox())!;
        return b.x >= -0.5 && b.x + b.width <= 280.5;
      })
      .toBe(true);
    // The buttons are in the window too.
    for (const id of ["#part-previous", "#part-next"]) {
      await page.locator(id).scrollIntoViewIfNeeded();
      const b = (await page.locator(id).boundingBox())!;
      expect(b.x).toBeGreaterThanOrEqual(0);
      expect(b.x + b.width).toBeLessThanOrEqual(280.5);
    }
  });
});

test.describe("vehicle actuation in the signal plan (CLM-12)", () => {
  test("each street's mode is a radio group named by its heading and described by its state, the arrow keys choose it, its settings are labelled fields, and at 280 px nothing scrolls sideways", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 280, height: 700 });
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Traffic signal" });
    await page.locator("#jn-go-plan").click();
    await openSignalFolds(page);
    const street = page.locator(".act-street").first();
    const name = (await street.locator(".act-name").innerText()).trim();
    await expect(page.getByRole("group", { name: `Phase of ${name}` })).toHaveCount(1);
    const mode = street.getByRole("radiogroup", { name: "Phase" });
    await expect(mode).toHaveAccessibleDescription(/^Phase on recall: runs every cycle at its green/);
    const recall = mode.getByRole("radio", { name: "On recall" });
    const actuated = mode.getByRole("radio", { name: "Actuated" });
    await expect(recall).toBeChecked();
    await recall.focus();
    await page.keyboard.press("ArrowDown");
    await expect(actuated).toBeChecked();
    await expect(actuated).toBeFocused();
    await expect(live(page)).toContainText(": actuated.");
    await expect(mode).toHaveAccessibleDescription(/^Actuated: vehicles detected on its approach call its phase/);
    for (const [label, range] of [
      ["Minimum green", "5 to 30 s (placeholder bound)"],
      ["Gap", "1 to 10 s (placeholder bound)"],
      ["Maximum green", "10 to 120 s (placeholder bound)"],
      ["Vehicles an hour", "0 to 1800 vehicles an hour (placeholder bound)"],
    ]) {
      await expect(street.getByRole("textbox", { name: label, exact: true })).toHaveAccessibleDescription(range);
    }
    const min = street.getByRole("textbox", { name: "Minimum green", exact: true });
    await street.getByRole("button", { name: "Minimum green, 1 more" }).click();
    await expect(min).toHaveValue("8");
    await min.focus();
    await page.keyboard.press("ArrowDown");
    await expect(min).toHaveValue("7");
    await expect(min).toBeFocused();
    await expect(page.getByRole("region", { name: "Actuated phases in one hour" })).toHaveAttribute("tabindex", "0");
    await expect(page.getByRole("region", { name: "Vehicles in one hour, by approach" })).toHaveAttribute(
      "tabindex",
      "0",
    );
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
});

test.describe("bicycle detection in the signal plan (CLM-13)", () => {
  test("each bike movement's mode is a radio group named by its heading and described by its state, the arrow keys choose it, its cyclists are a labelled field, and at 280 px nothing scrolls sideways", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 280, height: 700 });
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Traffic signal" });
    await page.locator("#jn-go-plan").click();
    await openSignalFolds(page);
    const movement = page.locator(".bike-movement").first();
    const name = (await movement.locator(".bike-name").innerText()).trim();
    await expect(page.getByRole("group", { name, exact: true })).toHaveCount(1);
    const mode = movement.getByRole("radiogroup", { name: "Bikes" });
    await expect(mode).toHaveAccessibleDescription(/^On recall: runs with every phase that lets it go/);
    const recall = mode.getByRole("radio", { name: "On recall" });
    const detection = mode.getByRole("radio", { name: "On detection" });
    await expect(recall).toBeChecked();
    await recall.focus();
    await page.keyboard.press("ArrowDown");
    await expect(detection).toBeChecked();
    await expect(detection).toBeFocused();
    await expect(live(page)).toContainText(": on detection.");
    await expect(mode).toHaveAccessibleDescription(
      /^On detection: a cyclist detected on the approach calls the movement/,
    );
    await expect(movement.getByRole("textbox", { name: "Cyclists an hour", exact: true })).toHaveAccessibleDescription(
      "0 to 120 cyclists an hour (placeholder bound)",
    );
    await expect(page.getByRole("region", { name: "Cyclists' calls in one hour, by bike movement" })).toHaveAttribute(
      "tabindex",
      "0",
    );
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
});

test.describe("transit signal priority in the signal plan (CLM-14)", () => {
  test("a street's priority is a radio group named by its heading and described by its state, the arrow keys choose it, its buses are a labelled field, and at 280 px nothing scrolls sideways", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 280, height: 700 });
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Traffic signal" });
    // A bus lane gives the first street with a phase of its own transit.
    await page.locator("#jn-go-plan").click();
    await openSignalFolds(page);
    const uid = (await page.locator("#signal-actuation .act-mode").first().getAttribute("id"))!.replace(
      "act-mode-",
      "",
    );
    await page.locator("#jn-path-up-0").click();
    await page.locator(`#jn-go-${uid}`).click();
    await page.locator("#jn-fold-transit summary").click();
    await page.getByRole("checkbox", { name: "Bus lane along the way in" }).click();
    await page.locator("#jn-path-up-0").click();
    await page.locator("#jn-go-plan").click();
    await openSignalFolds(page);
    const street = page.locator(".transit-street").first();
    const name = (await street.locator(".transit-name").innerText()).trim();
    await expect(page.getByRole("group", { name, exact: true })).toHaveCount(1);
    const mode = street.getByRole("radiogroup", { name: "Priority" });
    await expect(mode).toHaveAccessibleDescription(/^No priority: a bus waits/);
    const none = mode.getByRole("radio", { name: "None" });
    const early = mode.getByRole("radio", { name: "Early green" });
    await expect(none).toBeChecked();
    await none.focus();
    await page.keyboard.press("ArrowDown");
    await expect(early).toBeChecked();
    await expect(early).toBeFocused();
    await expect(live(page)).toContainText(": early green.");
    await expect(mode).toHaveAccessibleDescription(/^Early green: a bus coming while the street is red/);
    await page.keyboard.press("ArrowDown");
    await expect(mode.getByRole("radio", { name: "Green extension" })).toBeChecked();
    await expect(mode).toHaveAccessibleDescription(/^Green extension: a bus coming within 10 s/);
    await expect(street.getByRole("textbox", { name: "Buses an hour", exact: true })).toHaveAccessibleDescription(
      "0 to 60 buses an hour (placeholder bound)",
    );
    await expect(page.getByRole("region", { name: "Buses in one hour, by street" })).toHaveAttribute("tabindex", "0");
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
});

test.describe("emergency vehicle pre-emption in the signal plan (CLM-15)", () => {
  test("a street's pre-emption is a radio group named by its heading and described by its state, the arrow keys choose it, its calls are a labelled field, and at 280 px nothing scrolls sideways", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 280, height: 700 });
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Traffic signal" });
    await page.locator("#jn-go-plan").click();
    await openSignalFolds(page);
    const street = page.locator(".preempt-street").first();
    const name = (await street.locator(".preempt-name").innerText()).trim();
    await expect(page.getByRole("group", { name, exact: true })).toHaveCount(1);
    const mode = street.getByRole("radiogroup", { name: "Pre-emption" });
    await expect(mode).toHaveAccessibleDescription(/^Off: an emergency vehicle waits/);
    const off = mode.getByRole("radio", { name: "Off" });
    const on = mode.getByRole("radio", { name: "On" });
    await expect(off).toBeChecked();
    await off.focus();
    await page.keyboard.press("ArrowDown");
    await expect(on).toBeChecked();
    await expect(on).toBeFocused();
    await expect(live(page)).toContainText(": pre-emption on.");
    await expect(mode).toHaveAccessibleDescription(/^On, the highest priority: an emergency call on this street/);
    await expect(
      street.getByRole("textbox", { name: "Emergency calls an hour", exact: true }),
    ).toHaveAccessibleDescription("0 to 12 emergency calls an hour (placeholder bound)");
    await expect(page.getByRole("region", { name: "Emergency calls in one hour, by street" })).toHaveAttribute(
      "tabindex",
      "0",
    );
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
});

test.describe("time-of-day plans in the signal plan (CLM-16)", () => {
  test("the plan shown is a labelled choice described by what it changes, each control of the plans and the schedule is named by its plan, day and time, each day's table is a region named by its caption, which plan runs is a status, and at 280 px nothing scrolls sideways", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 280, height: 700 });
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Traffic signal" });
    await page.locator("#jn-go-plan").click();
    await openSignalFolds(page);
    const chooser = page.getByRole("combobox", { name: "Plan shown and edited", exact: true });
    await expect(chooser).toHaveAccessibleDescription(/^The signal plan below, its summary of the hour and its checks/);
    await page.getByRole("textbox", { name: "Name of a new plan", exact: true }).fill("Morning peak");
    await page.getByRole("textbox", { name: "Name of a new plan", exact: true }).press("Enter");
    await expect(live(page)).toContainText("Showing plan “Morning peak”");
    await expect(chooser).toHaveValue("1");
    await expect(page.getByRole("list", { name: "Plans" })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "Name of plan “Morning peak”", exact: true })).toHaveValue(
      "Morning peak",
    );
    await expect(page.getByRole("button", { name: "Remove plan “Morning peak”", exact: true })).toBeVisible();
    for (const [day, caption, on] of [
      ["weekday", "Weekdays (Monday to Friday)", "on weekdays"],
      ["saturday", "Saturday", "on Saturdays"],
      ["sunday", "Sunday", "on Sundays"],
    ]) {
      await expect(page.getByRole("region", { name: caption, exact: true })).toHaveAttribute("tabindex", "0");
      await expect(page.locator(`#schedule-${day}`)).toBeVisible();
      await expect(
        page.getByRole("combobox", { name: `Plan of the period from 00:00 ${on}`, exact: true }),
      ).toHaveValue("0");
      const group = page.getByRole("group", { name: `New period ${on}`, exact: true });
      await expect(group.getByLabel(`Start of a new period ${on}`, { exact: true })).toBeVisible();
      await expect(group.getByRole("combobox", { name: `Plan of a new period ${on}`, exact: true })).toBeVisible();
      await expect(group.getByRole("button", { name: `Add period ${on}`, exact: true })).toBeVisible();
    }
    // A period added by the keys alone: its start and plan named by the day and its time.
    await page.getByLabel("Start of a new period on Saturdays", { exact: true }).fill("07:00");
    await page
      .getByRole("combobox", { name: "Plan of a new period on Saturdays", exact: true })
      .selectOption({ label: "Morning peak" });
    await page.getByRole("button", { name: "Add period on Saturdays", exact: true }).press("Enter");
    await expect(live(page)).toContainText("“Morning peak” from 07:00 on Saturdays.");
    await expect(page.getByLabel("Start of the period from 07:00 on Saturdays", { exact: true })).toHaveValue("07:00");
    await expect(
      page.getByRole("combobox", { name: "Plan of the period from 07:00 on Saturdays", exact: true }),
    ).toHaveValue("1");
    await expect(
      page.getByRole("button", { name: "Remove the period from 07:00 on Saturdays", exact: true }),
    ).toBeVisible();
    // A start changed with the keyboard keeps the focus in that period's start field, wherever the row is drawn again.
    const start = page.getByLabel("Start of the period from 07:00 on Saturdays", { exact: true });
    await start.focus();
    await page.keyboard.press("ArrowUp");
    await expect(live(page)).toContainText("Period from 07:00 on Saturdays now starts at 08:00.");
    const moved = page.getByLabel("Start of the period from 08:00 on Saturdays", { exact: true });
    await expect(moved).toHaveValue("08:00");
    await expect(moved).toBeFocused();
    await expect(page.locator("#period-start-saturday-480")).toBeFocused();
    await page.keyboard.press("ArrowUp");
    await expect(page.getByLabel("Start of the period from 09:00 on Saturdays", { exact: true })).toBeFocused();
    // A period's plan changed keeps the focus on its choice of plan, though the row is drawn again.
    const choice = page.getByRole("combobox", { name: "Plan of the period from 09:00 on Saturdays", exact: true });
    await choice.focus();
    await choice.selectOption("0");
    await expect(choice).toHaveValue("0");
    await expect(page.locator("#period-plan-saturday-540")).toBeFocused();
    await choice.selectOption("1");
    await expect(choice).toHaveValue("1");
    await expect(page.locator("#period-plan-saturday-540")).toBeFocused();
    // Which plan runs: a day and a time, both labelled, and the answer a status.
    const runs = page.getByRole("group", { name: "Which plan runs", exact: true });
    await runs.getByRole("combobox", { name: "Day", exact: true }).selectOption("saturday");
    await runs.getByLabel("Time", { exact: true }).fill("09:30");
    await expect(page.getByRole("status").filter({ hasText: "At 09:30" })).toHaveText(
      "At 09:30 on Saturdays, plan “Morning peak” runs.",
    );
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
});

test.describe("pedestrian calls in the signal plan (#60)", () => {
  test("each crossing's box is named by its words and its group, described by its state, and at 280 px the tables of calls do not make the page scroll sideways", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 280, height: 700 });
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Traffic signal" });
    await page.locator("#jn-go-plan").click();
    await openSignalFolds(page);
    const crossing = page.locator(".call-crossing").first();
    const name = (await crossing.locator(".call-name").innerText()).trim();
    await expect(page.getByRole("group", { name: `Crossing of ${name}` })).toHaveCount(1);
    const box = crossing.getByRole("checkbox", { name: "On call", exact: true });
    await expect(box).toHaveAccessibleDescription(/^On recall: walked every time its phases run/);
    // Space puts it on call; its rate is a labelled field, and the description says what a call needs of the green.
    await box.focus();
    await page.keyboard.press("Space");
    await expect(box).toBeChecked();
    await expect(box).toBeFocused();
    await expect(live(page)).toContainText("on call.");
    await expect(box).toHaveAccessibleDescription(/^On call: walk 7 s \+ clearance \d+ s = \d+ s of green\.$/);
    await expect(crossing.getByRole("textbox", { name: "Calls an hour" })).toHaveAccessibleDescription(
      "0 to 60 calls an hour (placeholder bound)",
    );
    // Its detection is a radio group named by its heading and described by what the chosen detection does; the arrow
    // keys choose passive detection, and the focus stays on the radio.
    const detection = crossing.getByRole("radiogroup", { name: "Detection" });
    await expect(detection).toHaveAccessibleDescription(
      /^A pedestrian pushes the button to call the walk; the call stays placed/,
    );
    const push = detection.getByRole("radio", { name: "Push button" });
    const passive = detection.getByRole("radio", { name: "Passive detection" });
    await expect(push).toBeChecked();
    await push.focus();
    await page.keyboard.press("ArrowDown");
    await expect(passive).toBeChecked();
    await expect(passive).toBeFocused();
    await expect(live(page)).toContainText(": passive detection.");
    await expect(live(page)).toContainText(/Passive detection: \d+ calls? cancelled, \d+ walks? skipped\./);
    await expect(detection).toHaveAccessibleDescription(
      /one who leaves after waiting 30 s cancels it, and a walk with nobody left waiting is skipped\. Walk and clearance are never shortened\.$/,
    );
    await page.locator("#calls-rows summary").click();
    await expect(page.locator("#calls-rows tbody tr").first()).toBeVisible();
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });

  /** How each table of the calls sits in the details panel: whether its box stays inside the panel, whether the table
   *  is wider than its box and, if so, whether the box scrolls and is a labelled region the keys reach; whether each
   *  column comes into view as the box is scrolled to it; and how wide the row headers (the crossings' names) are. */
  async function callTables(page: Page) {
    return page.evaluate(() =>
      [...document.querySelectorAll<HTMLTableElement>("#signal-calls table")].map((table) => {
        const panel = document.querySelector("#details-panel")!.getBoundingClientRect();
        const box = (table.closest(".calls-table") as HTMLElement | null) ?? table.parentElement!;
        const scrolls = box.scrollWidth > box.clientWidth + 1;
        const overflow = getComputedStyle(box).overflowX;
        const named = !!(box.getAttribute("aria-label") || box.getAttribute("aria-labelledby"));
        const reachable = box.getAttribute("tabindex") === "0" && box.getAttribute("role") === "region" && named;
        const b = box.getBoundingClientRect();
        const inPanel = b.left >= panel.left - 0.5 && b.right <= panel.right + 0.5;
        const unseen: string[] = [];
        for (const th of table.querySelectorAll<HTMLElement>("thead th")) {
          th.scrollIntoView({ block: "nearest", inline: "nearest" });
          const r = th.getBoundingClientRect();
          const c = box.getBoundingClientRect();
          if (r.left < c.left - 0.5 || r.right > c.right + 0.5) unseen.push(th.innerText);
        }
        // Scrolled to its far end, the crossing's name stays at the box's left edge (its column is sticky).
        box.scrollLeft = box.scrollWidth;
        const nameAt = table.querySelector<HTMLElement>("tbody th")!.getBoundingClientRect().left;
        const stuck = !scrolls || Math.abs(nameAt - box.getBoundingClientRect().left) <= 1;
        box.scrollLeft = 0;
        const rowHeader = table.querySelector<HTMLElement>("tbody th")!.getBoundingClientRect().width;
        return {
          id: table.id || "rows",
          inPanel,
          scrolls,
          scrollable: overflow === "auto" || overflow === "scroll",
          reachable,
          unseen,
          rowHeader,
          stuck,
        };
      }),
    );
  }

  for (const width of [1440, 280]) {
    test(`at ${width} px every column of the calls' tables can be read in the panel, in English and French`, async ({
      page,
    }) => {
      await page.setViewportSize({ width, height: 900 });
      // The fourth junction: four crossings, one too long to serve, so the totals have four rows and a finding.
      await openJunction(page, 3);
      await expect(page.locator("#drawing")).toBeVisible();
      await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Traffic signal" });
      await page.locator("#jn-go-plan").click();
      await openSignalFolds(page);
      for (const box of await page.locator(".call-crossing input[type=checkbox]").all()) await box.click();
      await expect(page.locator("#calls-totals tbody tr")).not.toHaveCount(0);
      await page.locator("#calls-rows summary").click();
      for (const lang of ["English", "Français"]) {
        if (lang === "Français") {
          await page.locator("#account-btn").click();
          await page.getByRole("button", { name: "Français" }).click();
          await expect(page.locator("#signal-calls")).toContainText("Appels piétons");
          await page.keyboard.press("Escape");
        }
        const tables = await callTables(page);
        expect(tables.length, lang).toBe(2);
        for (const t of tables) {
          expect(t.inPanel, `${lang} ${t.id}: its box stays inside the panel`).toBe(true);
          expect(t.unseen, `${lang} ${t.id}: columns that cannot be brought into view`).toEqual([]);
          if (t.scrolls) {
            expect(
              t.scrollable && t.reachable,
              `${lang} ${t.id}: wider than its box, so the box scrolls and is a labelled region the keys reach`,
            ).toBe(true);
          }
          expect(t.rowHeader, `${lang} ${t.id}: a crossing's name is not one word a line`).toBeGreaterThanOrEqual(110);
          expect(t.stuck, `${lang} ${t.id}: scrolled to its end, the crossing's name stays in view`).toBe(true);
        }
        await expect
          .poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth))
          .toBe(true);
      }
    });
  }
});

test.describe("the drawing's label, focus and colours", () => {
  test("the drawing is an image with a label that says it is the junction in 2.5D and how many streets, and the group says how to use it", async ({
    page,
  }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toHaveAttribute("role", "img");
    await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /^2\.5D view of the junction\. [3-5] streets/);
    await expect(page.locator("#wrap")).toHaveAttribute("aria-label", "Junction editor");
    await expect(page.locator("#wrap")).toHaveAttribute("aria-describedby", "keys");
    // A roundabout is said in it too.
    await page.locator("#inspector select").first().selectOption({ label: "Roundabout" });
    await expect(page.locator("#drawing")).toHaveAttribute("aria-label", /\. Roundabout\.$/);
  });

  test("the group shows its focus in the accent ring and the selected part is outlined in two tones", async ({
    page,
  }) => {
    await openJunction(page);
    await page.locator("#wrap").focus();
    const ring = await page.locator("#wrap").evaluate((el) => {
      const s = getComputedStyle(el);
      const probe = document.createElement("span");
      probe.style.color = "var(--accent)";
      document.body.append(probe);
      const accent = getComputedStyle(probe).color;
      probe.remove();
      return { style: s.outlineStyle, width: parseFloat(s.outlineWidth), color: s.outlineColor, accent };
    });
    expect(ring.style).not.toBe("none");
    expect(ring.width).toBeGreaterThanOrEqual(2);
    expect(ring.color).toBe(ring.accent);
    await page.keyboard.press("ArrowRight");
    await expect(page.locator("#drawing .iso-sel")).toHaveCount(1);
    await expect(page.locator("#drawing .iso-sel-casing")).toHaveCount(1);
  });

  test("in forced colours the marks and the outline are drawn in the system colours", async ({ page }) => {
    await page.emulateMedia({ forcedColors: "active" });
    await openJunction(page);
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    const stroke = (selector: string) =>
      page
        .locator(selector)
        .first()
        .evaluate((el) => getComputedStyle(el).stroke);
    // The outline is Highlight over Canvas; nothing is left in the page's own colours that the system cannot see.
    const outline = await stroke("#drawing .iso-sel");
    const casing = await stroke("#drawing .iso-sel-casing");
    expect(outline).not.toBe(casing);
    expect(await stroke("#drawing .iso-casing")).toBe(casing);
    const zebra = await page
      .locator("#drawing .iso-zebra")
      .first()
      .evaluate((el) => getComputedStyle(el).fill);
    expect(zebra).not.toMatch(/^(none|transparent)$/);
  });

  test("a zebra and a stop line are white paint with a dark edge, not the ink that was 2.9:1 on the asphalt", async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "light" });
    await openJunction(page);
    const paint = await page
      .locator("#drawing .iso-zebra")
      .first()
      .evaluate((el) => {
        const s = getComputedStyle(el);
        const probe = document.createElement("span");
        probe.style.color = "var(--iso-paint-white)";
        document.body.append(probe);
        const white = getComputedStyle(probe).color;
        probe.remove();
        return { fill: s.fill, white, stroke: s.stroke };
      });
    expect(paint.fill).toBe(paint.white);
    expect(paint.stroke).not.toMatch(/^(none|transparent)$/);
  });
});
