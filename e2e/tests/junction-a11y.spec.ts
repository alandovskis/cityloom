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
          await expect(page.locator("#jn-fold-calls")).toContainText("Appels piétons");
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

test.describe("the junction's folds are reached by heading (CLM-50)", () => {
  // Each signal option's fold, by its id, and the name its heading gives it.
  const SIGNAL_FOLDS: [string, string][] = [
    ["jn-fold-plans", "Time-of-day plans"],
    ["jn-fold-calls", "Pedestrian calls"],
    ["jn-fold-actuation", "Vehicle actuation"],
    ["jn-fold-bikes", "Bicycle detection"],
    ["jn-fold-priority", "Transit signal priority"],
    ["jn-fold-preemption", "Emergency vehicle pre-emption"],
  ];

  test("each signal option's fold is named by a level-3 heading, open or closed, and opens with Enter or Space", async ({
    page,
  }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    await page.locator('select[aria-labelledby="i-h-control"]').selectOption({ label: "Traffic signal" });
    await page.locator("#jn-go-plan").click();
    await expect(page.locator("#signal-plan-state")).toBeVisible();
    const inspector = page.locator("#inspector");
    const heading = (name: string) => inspector.getByRole("heading", { level: 3, name, exact: true });
    await expect(heading("Signal timing")).toHaveCount(1);
    // every fold closed: each still named by a heading, inside its summary
    await expect(inspector.locator('details.jn-fold[id^="jn-fold-"][open]')).toHaveCount(0);
    const drawn: [string, string][] = [];
    for (const [id, name] of SIGNAL_FOLDS) {
      const fold = page.locator(`#${id}`);
      // transit signal priority is drawn only where some street has transit; the others on this junction always
      if (id !== "jn-fold-priority") await expect(fold, id).toHaveCount(1);
      if ((await fold.count()) === 0) continue;
      drawn.push([id, name]);
      await expect(heading(name), `${id} closed`).toHaveCount(1);
      await expect(fold.locator(":scope > summary").getByRole("heading", { level: 3, name, exact: true })).toHaveCount(
        1,
      );
    }
    // the keys still open a fold: Space on the calls', Enter on the pre-emption's
    await page.locator("#jn-fold-calls > summary").focus();
    await page.keyboard.press("Space");
    await expect(page.locator("#jn-fold-calls")).toHaveAttribute("open");
    await expect(page.locator("#signal-calls")).toBeVisible();
    await page.locator("#jn-fold-preemption > summary").focus();
    await page.keyboard.press("Enter");
    await expect(page.locator("#jn-fold-preemption")).toHaveAttribute("open");
    await expect(page.locator("#signal-preemption")).toBeVisible();
    // every fold open: each name is still one level-3 heading, not two
    await openSignalFolds(page);
    await expect(heading("Signal timing")).toHaveCount(1);
    for (const [id, name] of drawn) {
      await expect(heading(name), `${id} open`).toHaveCount(1);
      await expect(page.locator(`#${id} > summary`).getByRole("heading", { level: 3, name, exact: true })).toHaveCount(
        1,
      );
    }
  });

  test("a street's folds are named by level-3 headings, open or closed", async ({ page }) => {
    await openJunction(page);
    await expect(page.locator("#drawing")).toBeVisible();
    // The first street, as the keys reach it.
    await page.locator("#wrap").focus();
    await page.keyboard.press("ArrowRight");
    await expect(page.locator("#jn-fold-transit")).toHaveCount(1);
    const inspector = page.locator("#inspector");
    const heading = (name: string) => inspector.getByRole("heading", { level: 3, name, exact: true });
    for (const [id, name] of [
      ["jn-fold-transit", "Transit priority"],
      ["jn-fold-position", "Position"],
    ]) {
      await expect(page.locator(`#${id}`)).not.toHaveAttribute("open");
      await expect(heading(name), `${id} closed`).toHaveCount(1);
      await expect(page.locator(`#${id} > summary`).getByRole("heading", { level: 3, name, exact: true })).toHaveCount(
        1,
      );
    }
    await page.locator("#jn-fold-transit > summary").focus();
    await page.keyboard.press("Enter");
    await expect(page.locator("#jn-fold-transit")).toHaveAttribute("open");
    await page.locator("#jn-fold-position > summary").focus();
    await page.keyboard.press("Space");
    await expect(page.locator("#jn-fold-position")).toHaveAttribute("open");
    await expect(heading("Transit priority")).toHaveCount(1);
    await expect(heading("Position")).toHaveCount(1);
  });
});

// ---- the details as a label column, as the street's (CLM-51) ------------------------------------------------------------

/** What does not fit the details' label column: a box or a word past the column's right edge, a setting's name that runs into
 *  its control, a button under 24 px. What scrolls or is cut inside a box of its own is that box's. */
async function misfits(page: Page) {
  return page.evaluate(() => {
    const column = document.querySelector("#inspector > .insp-columns");
    if (!column) return ["no label column"];
    const found: string[] = [];
    const edge = column.getBoundingClientRect().right + 0.5;
    const own = (el: Element) => {
      for (let e: Element | null = el; e && e !== column; e = e.parentElement) {
        if (getComputedStyle(e).overflowX !== "visible") return true;
      }
      return false;
    };
    const shown = (r: DOMRect) => r.width > 0 && r.height > 0;
    const words = document.createTreeWalker(column, NodeFilter.SHOW_TEXT);
    for (let n = words.nextNode(); n; n = words.nextNode()) {
      const parent = n.parentElement!;
      if (parent.closest("option, svg") || own(parent)) continue;
      const range = document.createRange();
      range.selectNodeContents(n);
      const said = n.textContent!.trim();
      const within = parent.closest(".note-h, button, li, p") ?? column;
      const limit = Math.min(edge, within.getBoundingClientRect().right + 0.5);
      for (const r of range.getClientRects()) {
        if (shown(r as DOMRect) && r.right > limit) found.push(`"${said}" ends at ${r.right}, past ${limit}`);
      }
    }
    for (const el of column.querySelectorAll("*")) {
      const r = el.getBoundingClientRect();
      if (!shown(r) || el.closest("option, svg") || (el.parentElement !== column && own(el.parentElement!))) continue;
      if (r.right > edge) found.push(`${el.tagName}.${el.className} ends at ${r.right}, past ${edge}`);
      if (el.matches("button:enabled, select, input, summary") && r.height < 24) {
        found.push(`${el.tagName} "${el.textContent}" is ${r.height} high`);
      }
    }
    // A number can be read in its field.
    const pen = document.createElement("canvas").getContext("2d")!;
    // The shorthand `font` can be empty (a variant it cannot say), and the pen would keep its own 10 px.
    const wide = (said: string, css: CSSStyleDeclaration) => {
      pen.font = `${css.fontWeight} ${css.fontSize} ${css.fontFamily}`;
      if (!pen.font.includes(css.fontSize)) found.push(`"${said}" cannot be measured in ${pen.font}`);
      return pen.measureText(said).width;
    };
    for (const input of column.querySelectorAll<HTMLInputElement>(".stepper input")) {
      const css = getComputedStyle(input);
      if (!shown(input.getBoundingClientRect())) continue;
      const room = input.clientWidth - parseFloat(css.paddingLeft) - parseFloat(css.paddingRight);
      const needs = wide(input.value, css);
      if (needs > room) found.push(`"${input.value}" needs ${needs} in its field's ${room}`);
    }
    // What a list is set to can be read in it, beside its arrow (a bus's route is two streets' names, as long as they are).
    for (const select of column.querySelectorAll<HTMLSelectElement>('select:not([aria-labelledby="i-h-bus"])')) {
      const css = getComputedStyle(select);
      if (!shown(select.getBoundingClientRect())) continue;
      const room = select.clientWidth - parseFloat(css.paddingLeft) - parseFloat(css.paddingRight) - 16;
      const value = select.selectedOptions[0]?.textContent ?? "";
      const needs = wide(value, css);
      if (needs > room) found.push(`"${value}" needs ${needs} in its list's ${room}`);
    }
    // What is said under a control is right under it, however many lines the setting's name takes.
    for (const section of column.querySelectorAll(".insp-sec:not(.insp-span)")) {
      const parts = [...section.children].filter((c) => !c.matches(".note-h") && shown(c.getBoundingClientRect()));
      parts.slice(1).forEach((c, i) => {
        const gap = c.getBoundingClientRect().top - parts[i].getBoundingClientRect().bottom;
        if (gap > 12) found.push(`"${c.textContent}" is ${gap} under what it is about`);
      });
    }
    if (document.documentElement.scrollWidth > window.innerWidth) found.push("the page scrolls sideways");
    return found;
  });
}

for (const lang of ["en", "fr-CA"]) {
  for (const width of [1440, 280]) {
    test(`each level of the details is a label column that fits the panel (${lang}, ${width} px)`, async ({ page }) => {
      await page.addInitScript((l) => localStorage.setItem("cityloom-lang", l), lang);
      await page.setViewportSize({ width, height: 900 });
      await openJunction(page);
      const column = page.locator("#inspector > .insp-columns");
      const up = page.locator("#jn-path-up-0");
      const control = page.locator('select[aria-labelledby="i-h-control"]');
      for (const option of ["0", "1", "2", "4", "3"]) {
        await control.selectOption(option);
        await expect(control).toHaveValue(option);
        expect(await misfits(page), `the junction, control ${option}`).toEqual([]);
      }
      await expect(column.locator("#jn-go-plan")).toBeVisible();
      // A street whose lanes come in: one that only leads out has no lane to go to.
      const streets = column.locator(".jn-goes .btn");
      for (let i = 0; i < (await streets.count()); i++) {
        await streets.nth(i).click();
        await expect(column.locator("#i-h-lanes")).toBeVisible();
        if ((await column.locator(".lane-pick").count()) && (await column.locator("#i-cross-go").count())) break;
        await up.click();
        await expect(streets.first()).toBeVisible();
      }
      await expect(column.locator(".lane-pick").first()).toBeVisible();
      for (const fold of await column.locator(".jn-fold > summary").all()) await fold.click();
      await expect(column.locator("#i-h-bearing")).toBeVisible();
      expect(await misfits(page), "a street").toEqual([]);
      await column.locator("#i-cross-go").click();
      await expect(column.locator("#i-h-setback")).toBeVisible();
      expect(await misfits(page), "a crossing").toEqual([]);
      await page.locator("#jn-path-up-1").click();
      await column.locator(".lane-pick").first().click();
      await expect(column.locator("#i-h-serves")).toBeVisible();
      expect(await misfits(page), "a lane").toEqual([]);
      for (let i = 0; i < 40 && !(await column.locator("#i-h-corner").isVisible()); i++) {
        await page.locator("#part-next").click();
      }
      await expect(column.locator("#i-h-corner")).toBeVisible();
      expect(await misfits(page), "a corner").toEqual([]);
      await up.click();
      await control.selectOption("4");
      const track = column.locator("section:has(#i-h-cycle) .opt");
      await track.click();
      await expect(column.locator("#i-h-cycle-width")).toBeVisible();
      expect(await misfits(page), "a roundabout").toEqual([]);
      await column.locator('select[aria-labelledby="i-h-bus"]').selectOption({ index: 1 });
      const parts = [
        ["bus", lang === "en" ? "Bus lane" : "Voie d’autobus"],
        ["cycle", lang === "en" ? "Cycle track" : "Piste cyclable"],
      ];
      for (const [part, name] of parts) {
        for (let i = 0; i < 40 && !(await page.locator("#jn-path-current").innerText()).startsWith(name); i++) {
          await page.locator("#part-next").click();
        }
        await expect(page.locator("#jn-path-current")).toContainText(name);
        await expect(column.locator(".insp-actions .btn.danger")).toBeVisible();
        expect(await misfits(page), `the ${part}`).toEqual([]);
      }
    });
  }
}

test.describe("the details' label column on a touch screen", () => {
  test.use({ hasTouch: true });

  test("the buttons of a level are 44 px high, as every other button is there", async ({ page }) => {
    await openJunction(page);
    expect(await page.evaluate(() => matchMedia("(any-pointer: coarse)").matches)).toBe(true);
    const column = page.locator("#inspector > .insp-columns");
    await column.locator(".jn-goes .btn").first().click();
    const heights = (name: string) =>
      column.locator(name).evaluateAll((all) => all.map((b) => b.getBoundingClientRect().height));
    const link = await heights(".insp-sec > a.btn");
    await column.locator("#i-cross-go").click();
    await expect(column.locator("#i-h-setback")).toBeVisible();
    const remove = await heights(".insp-actions .btn");
    expect(link.length).toBe(1);
    expect(remove.length).toBe(1);
    expect([...link, ...remove].filter((h) => h < 44)).toEqual([]);
  });
});
