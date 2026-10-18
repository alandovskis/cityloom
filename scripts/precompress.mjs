// Compresses what the server sends as it is, once, at build time: a `.br` (Brotli, the best level) and a `.gz` (gzip,
// the best level) beside each WebAssembly module, script, stylesheet and drawing under the directories it is given.
// The server (cityloom_server/src/static_files.rs) sends the one the browser takes, so it spends no time compressing
// the largest files per request and they are smaller than any on-the-fly level makes them. A copy that would not be
// smaller is not kept, and the copies of a file that is gone are removed.
//
//   node scripts/precompress.mjs web/pkg web/vendor
import { brotliCompressSync, constants, gzipSync } from "node:zlib";
import { readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const COMPRESSIBLE = /\.(wasm|js|mjs|css|svg)$/;

function* files(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) yield* files(path);
    else yield path;
  }
}

let kept = 0;
for (const dir of process.argv.slice(2)) {
  const all = [...files(dir)];
  const present = new Set(all);
  for (const path of all) {
    if (/\.(br|gz)$/.test(path) && !present.has(path.slice(0, -3))) rmSync(path);
  }
  for (const path of all.filter((p) => COMPRESSIBLE.test(p))) {
    const raw = readFileSync(path);
    const copies = {
      br: brotliCompressSync(raw, {
        params: { [constants.BROTLI_PARAM_QUALITY]: 11, [constants.BROTLI_PARAM_SIZE_HINT]: raw.length, [constants.BROTLI_PARAM_LGWIN]: 24 },
      }),
      gz: gzipSync(raw, { level: 9 }),
    };
    for (const [extension, bytes] of Object.entries(copies)) {
      const target = `${path}.${extension}`;
      if (bytes.length < raw.length) {
        writeFileSync(target, bytes);
        kept += 1;
      } else {
        rmSync(target, { force: true });
      }
    }
    console.log(`${path}: ${raw.length} B, br ${copies.br.length} B, gzip ${copies.gz.length} B`);
  }
}
console.log(`${kept} compressed copies`);
