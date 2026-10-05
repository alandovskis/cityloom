import { expect, test } from "./fixtures";

// PMTiles is read with range requests, so the server the pages are tested on has to answer them.
test.describe("the static server", () => {
  test("answers a range with just those bytes and says where they lie", async ({ request }) => {
    const whole = await (await request.get("/map.html")).body();
    const part = await request.get("/map.html", { headers: { range: "bytes=10-29" } });
    expect(part.status()).toBe(206);
    expect(part.headers()["content-range"]).toBe(`bytes 10-29/${whole.length}`);
    expect(part.headers()["accept-ranges"]).toBe("bytes");
    expect(await part.body()).toEqual(whole.subarray(10, 30));
  });

  test("answers an open-ended range and a range of the last bytes", async ({ request }) => {
    const whole = await (await request.get("/map.html")).body();
    const from = await request.get("/map.html", { headers: { range: "bytes=100-" } });
    expect(from.status()).toBe(206);
    expect(await from.body()).toEqual(whole.subarray(100));
    const last = await request.get("/map.html", { headers: { range: "bytes=-16" } });
    expect(last.status()).toBe(206);
    expect(await last.body()).toEqual(whole.subarray(whole.length - 16));
  });

  test("refuses a range past the end, and serves the whole file when there is no range", async ({ request }) => {
    const whole = await request.get("/map.html");
    expect(whole.status()).toBe(200);
    expect(whole.headers()["accept-ranges"]).toBe("bytes");
    const past = await request.get("/map.html", { headers: { range: "bytes=99999999-" } });
    expect(past.status()).toBe(416);
  });

  test("refuses a path whose escapes are not UTF-8, and keeps serving", async ({ request }) => {
    expect((await request.get("/%E0%A4%A")).status()).toBe(400);
    expect((await request.get("/map.html")).status()).toBe(200);
  });

  test("serves JSON with its content type", async ({ request }) => {
    expect((await request.get("/basemap-light.json")).headers()["content-type"]).toContain("application/json");
  });
});
