import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

import { expect, PLATEAU_PBF, PLATEAU_TILES, placesOnMap, test } from "./fixtures";

// Run by `just e2e-archive`: the server unpacked the archive `cityloom_prepare pack` made of the fixtures, and the
// fixtures do not route the road data or the basemap (see fixtures.ts), so what the page gets is what the server
// served from it.
test.skip(!process.env.E2E_ARCHIVE, "needs `just e2e-archive`");

test("the server answers the tile index from the unpacked archive", async ({ request }) => {
  const res = await request.get("/data/metro/index.json");
  expect(res.status()).toBe(200);
  expect(res.headers()["cache-control"]).toBe("public, max-age=300, stale-while-revalidate=600");
  const index = await res.json();
  expect(Object.keys(index)).toEqual(["lon0", "lat0", "dlon", "dlat", "tiles", "checksums"]);
  expect(index.tiles).toEqual(["0_0"]);
  const tile = readFileSync(PLATEAU_PBF);
  expect(index.checksums["0_0"]).toBe(createHash("sha256").update(tile).digest("hex").slice(0, 16));
});

test("the tile and the basemap are the fixtures' bytes, the basemap in ranges", async ({ request }) => {
  const tile = await request.get("/data/metro/0_0.osm.pbf");
  expect(Buffer.from(await tile.body()).equals(readFileSync(PLATEAU_PBF))).toBe(true);
  const basemap = readFileSync(PLATEAU_TILES);
  const part = await request.get("/data/basemap/montreal.pmtiles", { headers: { range: "bytes=0-15" } });
  expect(part.status()).toBe(206);
  expect(part.headers()["content-range"]).toBe(`bytes 0-15/${basemap.length}`);
  expect(Buffer.from(await part.body()).equals(basemap.subarray(0, 16))).toBe(true);
});

test("the map draws the Plateau from the archive", async ({ page }) => {
  await page.goto("/map.html");
  const places = await placesOnMap(page);
  expect(places.junctions.length).toBeGreaterThan(0);
  expect(places.streets.length).toBeGreaterThan(0);
});
