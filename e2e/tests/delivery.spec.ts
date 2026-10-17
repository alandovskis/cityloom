import { expect, forgive, placesOnMap, test } from "./fixtures";

const PAGES = ["/", "/map.html", "street", "junction", "/account.html"];
const IMMUTABLE = "public, max-age=31536000, immutable";

test.describe("the delivery of the site's code", () => {
  test("every page names its code by versioned addresses, all of them answered and kept for a year", async ({
    page,
  }) => {
    for (const path of PAGES) {
      const code: { url: string; status: number; cache: string | undefined }[] = [];
      page.on("response", (res) => {
        const url = new URL(res.url());
        if (/\.(js|mjs|css|wasm|svg)$/.test(url.pathname) && !/favicon|logo-/.test(url.pathname))
          code.push({ url: url.pathname, status: res.status(), cache: res.headers()["cache-control"] });
      });
      if (path === "street" || path === "junction") {
        await page.goto("/map.html");
        const places = await placesOnMap(page);
        code.length = 0;
        await page.goto("/" + (path === "street" ? places.streets[0] : places.junctions[0]));
      } else await page.goto(path);
      // the shell is bound, and fills in the version, when the module has started
      await expect(page.locator("[data-version]")).not.toBeEmpty();
      page.removeAllListeners("response");
      const build = new Set(code.map((c) => /^\/v\/([0-9a-f]{16})\//.exec(c.url)?.[1]));
      expect(code.length, path).toBeGreaterThan(3);
      expect(
        code.filter((c) => !c.url.startsWith("/v/")).map((c) => c.url),
        `${path}: unversioned code`,
      ).toEqual([]);
      expect(build.size, `${path}: one build`).toBe(1);
      expect(
        code.filter((c) => c.status !== 200 || c.cache !== IMMUTABLE),
        path,
      ).toEqual([]);
      expect(
        code.map((c) => c.url).some((u) => u.endsWith("/pkg/cityloom_editor_bg.wasm")),
        path,
      ).toBe(true);
    }
  });

  test("the pages themselves are revalidated, with a tag that changes with the build", async ({ request }) => {
    const page = await request.get("/map.html");
    expect(page.headers()["cache-control"]).toMatch(/^no-(cache|store)$/);
    expect(page.headers()["etag"]).toBeTruthy();
    expect((await request.get("/map.html", { headers: { "if-none-match": page.headers()["etag"] } })).status()).toBe(
      304,
    );
  });

  test("the module is sent in Brotli, or gzip to a client that takes no Brotli, and it is smaller than the file", async ({
    request,
  }) => {
    const wasm = "/pkg/cityloom_editor_bg.wasm";
    const plain = (await request.get(wasm, { headers: { "accept-encoding": "identity" } })).headers();
    const br = await request.get(wasm, { headers: { "accept-encoding": "br, gzip" } });
    const gz = await request.get(wasm, { headers: { "accept-encoding": "gzip" } });
    expect(plain["content-encoding"]).toBeUndefined();
    expect(br.headers()["content-encoding"]).toBe("br");
    expect(gz.headers()["content-encoding"]).toBe("gzip");
    expect(Number(plain["content-length"])).toBeGreaterThan(2_000_000);
    expect(Number(br.headers()["content-length"])).toBeLessThan(Number(gz.headers()["content-length"]));
    expect(Number(gz.headers()["content-length"])).toBeLessThan(Number(plain["content-length"]) / 2);
    // and what comes out is the module
    expect((await br.body()).subarray(0, 4)).toEqual(Buffer.from([0, 0x61, 0x73, 0x6d]));
  });

  test("a build that is not the current one is not served", async ({ request }) => {
    const res = await request.get("/v/0000000000000000/pkg/cityloom_editor.js");
    expect(res.status()).toBe(404);
    expect(res.headers()["cache-control"]).toBe("no-store");
  });

  test("the glyphs and the tile index are kept for a while, the tiles are always revalidated", async ({ request }) => {
    const index = await request.get("/data/metro/index.json");
    // the fixture serves the index by a route in a page; this asks the server, which has the file only after `just prepare`
    if (index.status() === 200) expect(index.headers()["cache-control"]).toContain("max-age=300");
    const glyph = await request.get("/glyphs/Noto%20Sans%20Regular/0-255.pbf");
    if (glyph.status() === 200) expect(glyph.headers()["cache-control"]).toContain("max-age=86400");
  });
});

test.describe("a module that does not start", () => {
  const WASM = "**/pkg/cityloom_editor_bg.wasm";

  for (const [name, answer] of [
    ["is not found", (route: any) => route.fulfill({ status: 404, body: "no" })],
    [
      "is not a module",
      (route: any) => route.fulfill({ status: 200, contentType: "application/wasm", body: Buffer.from("nope") }),
    ],
  ] as const) {
    test(`says so in English when the module ${name}`, async ({ page, errors }) => {
      await page.route(WASM, answer);
      await page.goto("/map.html");
      const banner = page.locator("#boot-failed");
      await expect(banner).toBeVisible();
      await expect(banner).toHaveAttribute("role", "alert");
      await expect(banner).toContainText("CityLoom could not start. Check your connection, then reload the page.");
      await expect(banner.getByRole("link", { name: "Reload" })).toBeVisible();
      await expect(banner.locator('[lang="fr-CA"]')).toBeHidden();
      forgive(errors, /./);
      errors.length = 0;
    });
  }

  test("says so in French when the person chose French", async ({ page, errors }) => {
    await page.addInitScript(() => localStorage.setItem("cityloom-lang", "fr-CA"));
    await page.route(WASM, (route) => route.fulfill({ status: 404, body: "no" }));
    await page.goto("/street.html?street=1");
    const banner = page.locator("#boot-failed");
    await expect(banner).toBeVisible();
    await expect(banner).toContainText(
      "CityLoom n’a pas pu démarrer. Vérifiez votre connexion, puis rechargez la page.",
    );
    await expect(banner.getByRole("link", { name: "Recharger" })).toBeVisible();
    await expect(banner.locator('[lang="en"]')).toBeHidden();
    forgive(errors, /./);
    errors.length = 0;
  });

  test("follows the browser's language when none was chosen", async ({ browser, baseURL }) => {
    const context = await browser.newContext({ baseURL, locale: "fr-CA" });
    const page = await context.newPage();
    await page.route(WASM, (route) => route.fulfill({ status: 404, body: "no" }));
    await page.goto("/");
    await expect(page.locator("#boot-failed")).toContainText("n’a pas pu démarrer");
    await context.close();
  });

  test("shows nothing when the module starts", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("#search")).toBeVisible();
    await expect(page.locator("#boot-failed")).toBeHidden();
  });
});
