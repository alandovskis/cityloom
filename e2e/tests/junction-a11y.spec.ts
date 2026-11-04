import { expect, live, openJunction, test } from "./fixtures";

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
          await expect(field(id)).toBeVisible();
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
