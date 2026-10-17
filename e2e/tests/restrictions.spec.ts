import type { Page } from "@playwright/test";

import { expect, live, mapReady, mapStill, test } from "./fixtures";

// Area restrictions on the map page of the Plateau: declared by pointer and by keyboard, the street inside flagged
// on the map and in the street editor, kept over a reload, deleted and brought back, and a corner dragged. What the
// pointer does on the map (MapLibre in a WebGL canvas, `web/basemap.js`) is covered only here.

type Spot = { x: number; y: number };

/** Four spots in page pixels, a square `half` either way of a wide street that lies wholly inside it, each spot on
 *  the open map (not under a panel). */
const squareAroundAStreet = (page: Page, half = 40) =>
  page.evaluate((half) => {
    const map = (window as any).cityloomMap;
    const r = map.getCanvas().getBoundingClientRect();
    const open = (x: number, y: number) => document.elementFromPoint(x, y) === map.getCanvas();
    for (const f of map.querySourceFeatures("places")) {
      if (f.geometry.type !== "LineString" || f.properties.width_m < 6) continue;
      const pts = f.geometry.coordinates.map((c: number[]) => map.project(c));
      const xs = pts.map((p: any) => p.x);
      const ys = pts.map((p: any) => p.y);
      const cx = r.left + (Math.min(...xs) + Math.max(...xs)) / 2;
      const cy = r.top + (Math.min(...ys) + Math.max(...ys)) / 2;
      const fits = Math.max(...xs) - Math.min(...xs) < half * 1.6 && Math.max(...ys) - Math.min(...ys) < half * 1.6;
      const corners = [
        [cx - half, cy - half],
        [cx + half, cy - half],
        [cx + half, cy + half],
        [cx - half, cy + half],
      ];
      if (fits && corners.every(([x, y]) => open(x, y))) return corners.map(([x, y]) => ({ x, y }));
    }
    throw new Error("no wide street on the map with room around it");
  }, half);

/** A junction the pointer can reach, in page pixels, the only one drawn where it is, with room east and south of
 *  it for a triangle of side `side` on the open map; none within `away` pixels of a spot of `avoid`. */
const junctionSpot = (page: Page, side = 80, avoid: Spot[] = [], away = 30) =>
  page.evaluate(
    ({ side, avoid, away }) => {
      const map = (window as any).cityloomMap;
      const r = map.getCanvas().getBoundingClientRect();
      const open = (x: number, y: number) => document.elementFromPoint(x, y) === map.getCanvas();
      for (const f of map.querySourceFeatures("places")) {
        if (f.geometry.type !== "Point") continue;
        const p = map.project(f.geometry.coordinates);
        const x = r.left + p.x;
        const y = r.top + p.y;
        if (![open(x, y), open(x + side, y), open(x + side, y + side)].every(Boolean)) continue;
        if (avoid.some((a) => Math.hypot(a.x - x, a.y - y) < away)) continue;
        const box = [
          [p.x - 8, p.y - 8],
          [p.x + 8, p.y + 8],
        ];
        const there = map.queryRenderedFeatures(box, { layers: ["places-junction"] });
        if (there.length === 1 && there[0].properties.hot === f.properties.hot)
          return { x, y, junction: Number(String(f.properties.hot).slice(2)) };
      }
      throw new Error("no junction on the map the pointer can reach");
    },
    { side, avoid, away },
  );

/** The middle of the map in page pixels, where Enter places a corner. */
const middle = (page: Page) =>
  page.evaluate(() => {
    const r = (window as any).cityloomMap.getCanvas().getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
  });

/** Where the corners of the selected area are, in page pixels, those on the open map only. */
const areaCorners = (page: Page) =>
  page.evaluate(() => {
    const map = (window as any).cityloomMap;
    const r = map.getCanvas().getBoundingClientRect();
    return map
      .querySourceFeatures("restrictions")
      .filter((f: any) => f.geometry.type === "Point")
      .sort((a: any, b: any) => a.properties.i - b.properties.i)
      .map((f: any) => {
        const p = map.project(f.geometry.coordinates);
        return { i: f.properties.i as number, x: r.left + p.x, y: r.top + p.y };
      })
      .filter((s: any) => document.elementFromPoint(s.x, s.y) === map.getCanvas());
  });

/** Where the first corner of the outline being drawn is, in page pixels. */
const firstDraftCorner = (page: Page) =>
  page.evaluate(() => {
    const map = (window as any).cityloomMap;
    const r = map.getCanvas().getBoundingClientRect();
    const f = map.querySourceFeatures("draft").find((f: any) => f.properties.first === true);
    const p = map.project(f.geometry.coordinates);
    return { x: r.left + p.x, y: r.top + p.y };
  });

/** How many features of `layer` are drawn at a spot of the page. */
const drawnAt = (page: Page, layer: string, s: Spot) =>
  page.evaluate(
    ({ layer, x, y }) => {
      const map = (window as any).cityloomMap;
      const r = map.getCanvas().getBoundingClientRect();
      return map.queryRenderedFeatures([x - r.left, y - r.top], { layers: [layer] }).length;
    },
    { layer, ...s },
  );

/** The Area rules panel is on the left and open at first; this says so. */
const openPanel = async (page: Page) => {
  await expect(page.locator("#areas-panel")).toBeVisible();
  await expect(page.locator("#inspector-toggle")).toHaveAttribute("aria-expanded", "true");
};

const kept = (page: Page) => page.evaluate(() => localStorage.getItem("cityloom-restrictions"));
/** The corners of the area this browser keeps (its id is random; the tests declare one). */
const keptRing = async (page: Page) =>
  Object.values(JSON.parse((await kept(page)) ?? "{}").restrictions ?? {}).map((r: any) => r.ring)[0] as
    [number, number][] | undefined;
/** A spot of the earth, in page pixels as the map is now. */
const onPage = (page: Page, at: [number, number]) =>
  page.evaluate((at) => {
    const map = (window as any).cityloomMap;
    const r = map.getCanvas().getBoundingClientRect();
    const p = map.project(at);
    return { x: r.left + p.x, y: r.top + p.y };
  }, at);
const near = (a: Spot, b: Spot, px = 2) => Math.hypot(a.x - b.x, a.y - b.y) <= px;

/** How many areas the map draws: an outline over several tiles comes once from each, so they are told by id. */
const polygons = (page: Page) =>
  page.evaluate(
    () =>
      new Set(
        (window as any).cityloomMap
          .querySourceFeatures("restrictions")
          .filter((f: any) => f.geometry.type === "Polygon")
          .map((f: any) => f.properties.area),
      ).size,
  );
const anyConflict = (page: Page) =>
  page.evaluate(() =>
    (window as any).cityloomMap.querySourceFeatures("places").some((f: any) => f.properties.conflict === true),
  );
const centre = (page: Page) =>
  page.evaluate(() => {
    const c = (window as any).cityloomMap.getCenter();
    return [c.lng, c.lat];
  });

/** Records every navigation the page starts (the Navigation API says so as it starts; it is
 *  Chromium-only, and Chromium is the one browser these specs run in), and has the editors answer
 *  with nothing, so the map page stays and can be asked. */
async function recordNavigations(page: Page) {
  await page.route(/\/(intersection|street)\.html\?/, (route) => route.fulfill({ status: 204 }));
  await page.evaluate(() => {
    (window as any).navigations = [];
    (window as any).navigation.addEventListener("navigate", (e: any) =>
      (window as any).navigations.push(new URL(e.destination.url).pathname + new URL(e.destination.url).search),
    );
  });
}
const navigations = (page: Page) => page.evaluate(() => (window as any).navigations as string[]);

/** Starts drawing from Declare, with the keyboard, and leaves the focus on the map. */
async function startByKeyboard(page: Page) {
  await openPanel(page);
  await page.locator("#declare-area").focus();
  await page.keyboard.press("Enter");
  await expect(page.locator("#basemap")).toHaveAttribute("data-drawing", "");
  await expect(page.locator("#basemap")).toBeFocused();
}

/** Places three corners from the keyboard: at the middle of the map, moved east, then south, between them. */
async function threeCornersByKeyboard(page: Page) {
  await page.keyboard.press("Enter");
  await expect(live(page)).toHaveText("Corner 1 placed.");
  await page.keyboard.press("ArrowRight");
  await mapStill(page);
  await page.keyboard.press("Enter");
  await expect(live(page)).toHaveText("Corner 2 placed.");
  await page.keyboard.press("ArrowDown");
  await mapStill(page);
  await page.keyboard.press("Enter");
  await expect(live(page)).toHaveText("Corner 3 placed.");
}

/** The closed outline's form is shown wherever the person had gone, and asks for its rule first. */
async function formShown(page: Page) {
  await expect(page.locator("#area-save")).toBeVisible();
  await expect(page.locator("#inspector-toggle")).toHaveAttribute("aria-expanded", "true");
  await expect(page.locator("#area-rule")).toBeFocused();
  await expect(page.locator("#basemap")).not.toHaveAttribute("data-drawing", "");
}

/** Declares an area from the keyboard alone: three corners, Shift+Enter, then its name typed and Enter. */
async function declareByKeyboard(page: Page, name: string) {
  await startByKeyboard(page);
  await threeCornersByKeyboard(page);
  await page.keyboard.press("Shift+Enter");
  await formShown(page);
  await page.keyboard.press("Tab"); // from the rule to the name
  await expect(page.locator("#area-name")).toBeFocused();
  await page.keyboard.type(name);
  await page.keyboard.press("Enter"); // submits the form
  await expect(live(page)).toHaveText(`“${name}” saved.`);
  await expect(page.locator("#areas > li")).toHaveCount(1);
}

/** Declares an area by clicking its corners, then clicking the first, and saves it with `rule` and `name`. */
async function declareByClicks(page: Page, spots: Spot[], name: string, rule = "car_free") {
  for (const [n, s] of spots.entries()) {
    await page.mouse.click(s.x, s.y);
    await expect(live(page)).toHaveText(`Corner ${n + 1} placed.`);
  }
  await page.mouse.click(spots[0].x, spots[0].y);
  await formShown(page);
  await page.locator("#area-rule").selectOption(rule);
  await page.locator("#area-name").fill(name);
  await page.locator("#area-save").click();
  await expect(live(page)).toHaveText(`“${name}” saved.`);
}

test.describe("area restrictions", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/map.html");
    await mapReady(page);
    await mapStill(page);
  });

  test("an area clicked out on the map is listed, drawn, flags the street inside, and the street editor says so", async ({
    page,
  }) => {
    await openPanel(page);
    await expect(page.locator("#areas-lead")).toHaveText(
      "No area rules yet. Declare one to check the streets inside it.",
    );
    await expect(page.locator("#fit")).toHaveText(/ places\. Every check passes\.$/);
    expect(await anyConflict(page)).toBe(false);
    await page.locator("#declare-area").click();
    await expect(page.locator("#basemap")).toHaveAttribute("data-drawing", "");
    const spots: Spot[] = await squareAroundAStreet(page);
    for (const [n, s] of spots.entries()) {
      await page.mouse.click(s.x, s.y);
      await expect(live(page)).toHaveText(`Corner ${n + 1} placed.`);
    }
    // the first corner is drawn before it is pressed to close the outline
    await expect.poll(() => drawnAt(page, "draft-vertex", spots[0])).toBeGreaterThan(0);
    await page.mouse.click(spots[0].x, spots[0].y);
    await formShown(page);
    await page.locator("#area-rule").selectOption("car_free");
    await page.locator("#area-name").fill("Test zone");
    await page.locator("#area-save").click();
    await expect(live(page)).toHaveText("“Test zone” saved.");
    await expect(page.locator("#areas > li")).toHaveCount(1);
    await expect(page.locator("#areas > li")).toContainText("Test zone");
    await expect(page.locator("#areas > li")).toContainText(/Car-free · (One street|\d+ streets) inside; /);
    await expect(page.locator("#areas-lead")).toHaveText("One area rule. Select one to see the streets inside it.");
    await expect.poll(() => polygons(page)).toBe(1);
    // the street inside is tinted, and the status line and the Checks tab count it
    await expect.poll(() => anyConflict(page)).toBe(true);
    await expect(page.locator("#fit")).toHaveText(
      / places\. (One street conflicts|\d+ streets conflict) with an area rule\.$/,
    );
    await expect(page.locator("#checks-lead")).toHaveText(
      /^(One street conflicts|\d+ streets conflict) with an area rule\. The Area rules panel lists (it|them)\.$/,
    );
    await expect(page.locator("#plan-key")).toContainText("Conflicts with an area rule");
    // the saved area is selected: its streets are listed under it, the one that breaks the rule marked
    await expect(page.locator("[id^='area-row-']")).toHaveAttribute("aria-expanded", "true");
    const flagged = page.locator("[id^='area-details-'] li.bad a").first();
    const href = await flagged.getAttribute("href");
    expect(href).toMatch(/^street\.html\?street=\d+$/);

    await page.goto("/" + href);
    await expect(page.locator("#checks li.bad", { hasText: "Area rule: Test zone" })).toHaveCount(1);
    await expect(page.locator("#checks")).toContainText("Car-free: the street does not keep to it.");

    // kept: the map shows it again
    await page.goto("/map.html");
    await mapReady(page);
    await openPanel(page);
    await expect(page.locator("#areas > li")).toHaveCount(1);
    await expect(page.locator("#areas")).toContainText("Test zone");
    await expect.poll(() => polygons(page)).toBe(1);
    await expect.poll(() => anyConflict(page)).toBe(true);
  });

  test("a speed limit tints no street, and the street editor shows it for information only", async ({ page }) => {
    await openPanel(page);
    await page.locator("#declare-area").click();
    const spots = await squareAroundAStreet(page);
    for (const [n, s] of spots.entries()) {
      await page.mouse.click(s.x, s.y);
      await expect(live(page)).toHaveText(`Corner ${n + 1} placed.`);
    }
    await page.mouse.click(spots[0].x, spots[0].y);
    await formShown(page);
    await page.locator("#area-rule").selectOption("speed_limit");
    await page.locator("#area-speed").selectOption("30");
    await page.locator("#area-name").fill("Slow zone");
    await page.locator("#area-save").click();
    await expect(live(page)).toHaveText("“Slow zone” saved.");
    await expect(page.locator("#areas > li")).toContainText(
      /Speed limit of 30 km\/h · (One street|\d+ streets) inside\./,
    );
    await expect.poll(() => polygons(page)).toBe(1);
    expect(await anyConflict(page)).toBe(false);
    await expect(page.locator("#fit")).toHaveText(/ places\. Every check passes\.$/);
    await expect(page.locator("[id^='area-details-'] li.bad")).toHaveCount(0);
    const href = await page.locator("[id^='area-details-'] li.ok a").first().getAttribute("href");
    expect(href).toMatch(/^street\.html\?street=\d+$/);

    await page.goto("/" + href);
    const row = page.locator("#checks li", { hasText: "Area rule: Slow zone" });
    await expect(row).toHaveCount(1);
    await expect(row).toHaveClass("ok");
    await expect(row).toContainText("Speed limit of 30 km/h: streets carry no speed, so this is for information only.");
    await expect(row.locator(".sr-only")).toHaveText(": for information only");
  });

  test("an area is declared from the keyboard alone; Backspace takes a corner back and Escape cancels", async ({
    page,
  }) => {
    await declareByKeyboard(page, "Keyboard zone");
    await expect(page.locator("#areas")).toContainText("Keyboard zone");
    await expect.poll(() => polygons(page)).toBe(1);
    // Backspace and Escape while drawing
    await page.locator("#declare-area").focus();
    await page.keyboard.press("Enter");
    await expect(page.locator("#basemap")).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(live(page)).toHaveText("Corner 1 placed.");
    await page.keyboard.press("Backspace");
    await expect(live(page)).toHaveText("Corner removed. 0 corners left.");
    await page.keyboard.press("Escape");
    await expect(live(page)).toHaveText("Drawing cancelled.");
    await expect(page.locator("#basemap")).not.toHaveAttribute("data-drawing", "");
    await expect(page.locator("#areas > li")).toHaveCount(1);
    await expect.poll(() => polygons(page)).toBe(1);
  });

  test("closing from the keyboard with the left panel hidden shows the Area rules form", async ({ page }) => {
    await startByKeyboard(page);
    await threeCornersByKeyboard(page);
    await page.locator("#inspector-toggle").click();
    await expect(page.locator("#inspector-toggle")).toHaveAttribute("aria-expanded", "false");
    await expect(page.locator("#areas-panel")).toBeHidden();
    await page.locator("#basemap").focus();
    await page.keyboard.press("Shift+Enter");
    // Showing the panel is the shell's own toggle, which says so over the outline's message; the map says it again.
    await expect(live(page)).toHaveText("The outline is closed. Choose its rule and name, then save it.");
    await formShown(page);
  });

  test("closing on the first corner with the left panel collapsed opens it on the Area rules form", async ({
    page,
  }) => {
    await openPanel(page);
    await page.locator("#declare-area").click();
    const spots = await squareAroundAStreet(page);
    for (const [n, s] of spots.slice(0, 3).entries()) {
      await page.mouse.click(s.x, s.y);
      await expect(live(page)).toHaveText(`Corner ${n + 1} placed.`);
    }
    await page.keyboard.press("[");
    await expect(page.locator("#inspector-toggle")).toHaveAttribute("aria-expanded", "false");
    // the map may have grown into the room the panel left: the first corner is where the map now draws it
    await mapStill(page);
    const first = await firstDraftCorner(page);
    expect(await page.evaluate(({ x, y }) => document.elementFromPoint(x, y)?.tagName, first)).toBe("CANVAS");
    await page.mouse.click(first.x, first.y);
    await formShown(page);
  });

  test("the Notes keep Checks and Changes only, and the area rules are in the panel on the left", async ({ page }) => {
    await expect(page.locator("#areas-panel")).toBeVisible();
    await expect(page.locator("#areas-panel #declare-area")).toBeVisible();
    await expect(page.locator("#t-areas")).toHaveCount(0);
    await expect(page.locator("#notes .tab")).toHaveCount(2);
    await expect(page.locator("#notes #declare-area")).toHaveCount(0);
  });

  test("a hidden panel stays hidden after a reload, and the toggle says so", async ({ page }) => {
    await page.locator("#inspector-toggle").click();
    await page.reload();
    await mapReady(page);
    await expect(page.locator("html")).toHaveAttribute("data-inspector", "closed");
    await expect(page.locator("#areas-panel")).toBeHidden();
    await expect(page.locator("#inspector-toggle")).toHaveAttribute("aria-expanded", "false");
  });

  test("the [ key types into the area name field and leaves the panel open", async ({ page }) => {
    await declareByKeyboard(page, "Zone[1");
    await expect(page.locator("#areas-panel")).toBeVisible();
    await expect(page.locator("#areas")).toContainText("Zone[1");
  });

  test("Finish and Cancel, while drawing, have a gap between them", async ({ page }) => {
    await page.locator("#declare-area").click();
    const [finish, cancel] = [
      await page.locator("#area-finish").boundingBox(),
      await page.locator("#area-cancel").boundingBox(),
    ];
    expect(cancel!.x - (finish!.x + finish!.width)).toBeGreaterThanOrEqual(6);
  });

  test("on a 1100 px window the status line stays a short pill and the Notes start closed", async ({ page }) => {
    await page.setViewportSize({ width: 1100, height: 800 });
    await page.reload();
    await mapReady(page);
    await expect(page.locator("html")).toHaveAttribute("data-notes", "closed");
    expect((await page.locator("#fit").boundingBox())!.height).toBeLessThan(90);
  });

  test("at phone width the panel stacks below the map", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 800 });
    const map = (await page.locator("#basemap").boundingBox())!;
    const panel = (await page.locator("#areas-panel").boundingBox())!;
    expect(panel.y).toBeGreaterThanOrEqual(map.y + map.height - 1);
    expect(map.height).toBeGreaterThan(200);
  });

  test("on a small outline a press on the first corner closes it, though a later corner is there too", async ({
    page,
  }) => {
    await openPanel(page);
    await page.locator("#declare-area").click();
    await expect(page.locator("#basemap")).toBeFocused();
    // The third corner goes at the middle of the map from the keyboard, 9 px from the first, so that a press on
    // the first finds both; the second is well away.
    const m = await middle(page);
    const first = { x: m.x - 9, y: m.y - 9 };
    const second = { x: m.x + 70, y: m.y - 9 };
    for (const s of [first, second])
      expect(await page.evaluate(({ x, y }) => document.elementFromPoint(x, y)?.tagName, s)).toBe("CANVAS");
    await page.mouse.click(first.x, first.y);
    await expect(live(page)).toHaveText("Corner 1 placed.");
    await page.mouse.click(second.x, second.y);
    await expect(live(page)).toHaveText("Corner 2 placed.");
    await page.locator("#basemap").focus();
    await page.keyboard.press("Enter");
    await expect(live(page)).toHaveText("Corner 3 placed.");
    // both corners are under a press on the first
    const both = await page.evaluate(({ x, y }) => {
      const map = (window as any).cityloomMap;
      const r = map.getCanvas().getBoundingClientRect();
      const p = [x - r.left, y - r.top];
      return map
        .queryRenderedFeatures(
          [
            [p[0] - 6, p[1] - 6],
            [p[0] + 6, p[1] + 6],
          ],
          { layers: ["draft-vertex"] },
        )
        .map((f: any) => f.properties.i)
        .sort();
    }, first);
    expect(both).toEqual([0, 2]);
    await page.mouse.click(first.x, first.y);
    await formShown(page);
  });

  test("a deleted area goes from the list and the map, comes back with Undo, and a reload keeps what is left", async ({
    page,
  }) => {
    await declareByKeyboard(page, "Short-lived");
    await page.locator("#area-delete").click();
    await expect(live(page)).toHaveText("“Short-lived” deleted.");
    await expect(page.locator("#areas > li")).toHaveCount(0);
    await expect.poll(() => polygons(page)).toBe(0);
    await expect(page.locator("#area-undo")).toBeFocused();
    await page.locator("#area-undo").click();
    await expect(live(page)).toHaveText("“Short-lived” restored.");
    await expect(page.locator("#areas > li")).toHaveCount(1);
    await expect(page.locator("[id^='area-row-']")).toBeFocused();
    await expect.poll(() => polygons(page)).toBe(1);
    await page.reload();
    await mapReady(page);
    await openPanel(page);
    await expect(page.locator("#areas")).toContainText("Short-lived");
    await expect.poll(() => polygons(page)).toBe(1);
    await page.locator("[id^='area-row-']").click();
    await expect(page.locator("[id^='area-row-']")).toHaveAttribute("aria-expanded", "true");
    await page.locator("#area-delete").click();
    await expect(page.locator("#areas > li")).toHaveCount(0);
    await page.reload();
    await mapReady(page);
    await openPanel(page);
    await expect(page.locator("#areas > li")).toHaveCount(0);
    await expect(page.locator("#areas-lead")).toHaveText(
      "No area rules yet. Declare one to check the streets inside it.",
    );
    expect(await polygons(page)).toBe(0);
  });

  test("Ctrl+Z brings a deleted area back, but not while an outline is drawn", async ({ page }) => {
    await declareByKeyboard(page, "Undone");
    await page.locator("#area-delete").click();
    await expect(page.locator("#areas > li")).toHaveCount(0);
    // while drawing, Ctrl+Z leaves the deleted area where it is
    await page.locator("#declare-area").click();
    await expect(page.locator("#basemap")).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(live(page)).toHaveText("Corner 1 placed.");
    await page.keyboard.press("Control+z");
    await expect(page.locator("#basemap")).toHaveAttribute("data-drawing", "");
    await page.keyboard.press("Escape");
    await expect(live(page)).toHaveText("Drawing cancelled.");
    await expect(page.locator("#areas > li")).toHaveCount(0);
    expect(Object.keys(JSON.parse((await kept(page))!).restrictions)).toEqual([]);
    // back in the list it does
    await page.locator("#basemap").focus();
    await page.keyboard.press("Control+z");
    await expect(live(page)).toHaveText("“Undone” restored.");
    await expect(page.locator("#areas > li")).toHaveCount(1);
    await expect.poll(() => polygons(page)).toBe(1);
  });

  test("a corner of the selected area is dragged and kept, and the map does not pan under it", async ({ page }) => {
    await declareByKeyboard(page, "Drag me");
    await mapStill(page);
    const before = await keptRing(page);
    const corners = await areaCorners(page);
    expect(corners.length).toBeGreaterThan(0);
    const corner = corners[0];
    const at = await centre(page);
    await page.mouse.move(corner.x, corner.y);
    await page.mouse.down();
    await page.mouse.move(corner.x - 15, corner.y - 15, { steps: 5 });
    await page.mouse.move(corner.x - 30, corner.y - 30, { steps: 5 });
    await page.mouse.up();
    await expect(live(page)).toHaveText("Corner moved.");
    expect(await centre(page)).toEqual(at);
    const after = (await keptRing(page))!;
    expect(after).not.toEqual(before);
    expect(near(await onPage(page, after[corner.i]), { x: corner.x - 30, y: corner.y - 30 })).toBe(true);
    // the other corners stay
    after.forEach((c, i) => i === corner.i || expect(c).toEqual(before![i]));
    // the next press is the map's again: a place pressed opens
    await recordNavigations(page);
    const spot = await junctionSpot(page, 0, await areaCorners(page));
    await page.mouse.click(spot.x, spot.y);
    await expect.poll(() => navigations(page)).toEqual([`/intersection.html?junction=${spot.junction}`]);
  });

  test("a click on a corner of the selected area moves nothing and opens nothing under it", async ({ page }) => {
    await recordNavigations(page);
    // an area with a corner on a junction
    await openPanel(page);
    await page.locator("#declare-area").click();
    const j = await junctionSpot(page);
    await declareByClicks(
      page,
      [
        { x: j.x, y: j.y },
        { x: j.x + 80, y: j.y },
        { x: j.x + 80, y: j.y + 80 },
      ],
      "Corner zone",
    );
    const before = await kept(page);
    await expect.poll(async () => (await areaCorners(page)).some((c) => near(c, j))).toBe(true);
    await page.mouse.click(j.x, j.y);
    // let the page's frame pass, so a navigation or a write the click caused would have happened before it is looked for
    await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
    expect(await navigations(page)).toEqual([]);
    await expect(live(page)).toHaveText("“Corner zone” saved.");
    expect(await kept(page)).toEqual(before);
    await expect(page.locator("[id^='area-row-']")).toHaveAttribute("aria-expanded", "true");
    // the same click, with no corner there once the area is let go, opens the junction
    await page.locator("#basemap").focus();
    await page.keyboard.press("Escape");
    await expect(page.locator("[id^='area-row-']")).toHaveAttribute("aria-expanded", "false");
    await expect.poll(() => drawnAt(page, "areas-vertex", j)).toBe(0);
    await page.mouse.click(j.x, j.y);
    await expect.poll(() => navigations(page)).toEqual([`/intersection.html?junction=${j.junction}`]);
  });

  test("a right button let go during a corner drag does not end it", async ({ page }) => {
    await declareByKeyboard(page, "Right button");
    await mapStill(page);
    const corner = (await areaCorners(page))[0];
    await page.mouse.move(corner.x, corner.y);
    await page.mouse.down();
    await page.mouse.move(corner.x - 15, corner.y - 15, { steps: 5 });
    await page.evaluate(
      ({ x, y }) =>
        window.dispatchEvent(
          new MouseEvent("mouseup", { button: 2, buttons: 1, clientX: x, clientY: y, bubbles: true }),
        ),
      { x: corner.x - 15, y: corner.y - 15 },
    );
    await page.mouse.move(corner.x - 30, corner.y - 30, { steps: 5 });
    await page.mouse.up();
    await expect(live(page)).toHaveText("Corner moved.");
    expect(near(await onPage(page, (await keptRing(page))![corner.i]), { x: corner.x - 30, y: corner.y - 30 })).toBe(
      true,
    );
  });

  test("a corner drag whose release was lost ends where the corner last was", async ({ page }) => {
    await declareByKeyboard(page, "Lost release");
    await mapStill(page);
    const corner = (await areaCorners(page))[0];
    const at = await centre(page);
    await page.mouse.move(corner.x, corner.y);
    await page.mouse.down();
    await page.mouse.move(corner.x - 20, corner.y - 20, { steps: 5 });
    // a move with no button down: the release happened where the page did not hear it
    await page.evaluate(
      ({ x, y }) =>
        window.dispatchEvent(new MouseEvent("mousemove", { buttons: 0, clientX: x, clientY: y, bubbles: true })),
      { x: corner.x - 40, y: corner.y },
    );
    await expect(live(page)).toHaveText("Corner moved.");
    await page.mouse.up();
    expect(await centre(page)).toEqual(at);
    expect(near(await onPage(page, (await keptRing(page))![corner.i]), { x: corner.x - 20, y: corner.y - 20 })).toBe(
      true,
    );
  });

  test("a corner drag ends where the corner was when the window loses focus", async ({ page }) => {
    await declareByKeyboard(page, "Blurred");
    await mapStill(page);
    const corner = (await areaCorners(page))[0];
    const at = await centre(page);
    await page.mouse.move(corner.x, corner.y);
    await page.mouse.down();
    await page.mouse.move(corner.x - 20, corner.y - 20, { steps: 5 });
    await page.evaluate(() => window.dispatchEvent(new Event("blur")));
    await expect(live(page)).toHaveText("Corner moved.");
    // the button still down, the pointer goes on: the corner does not follow, nor does the map pan
    await page.mouse.move(corner.x - 40, corner.y, { steps: 5 });
    await page.mouse.up();
    expect(await centre(page)).toEqual(at);
    expect(near(await onPage(page, (await keptRing(page))![corner.i]), { x: corner.x - 20, y: corner.y - 20 })).toBe(
      true,
    );
  });
});
