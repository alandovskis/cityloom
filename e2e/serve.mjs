// A static server for ../web that copes with the tests' parallel page loads.
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../web/", import.meta.url));
const types = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm",
  ".woff2": "font/woff2",
  ".pbf": "application/x-protobuf",
  ".pmtiles": "application/octet-stream",
  ".txt": "text/plain; charset=utf-8",
};

/** The bytes a `Range` header asks of a file `size` long: [start, end], or null when it cannot be had. */
function rangeOf(header, size) {
  const m = /^bytes=(\d*)-(\d*)$/.exec(header);
  if (!m || (m[1] === "" && m[2] === "")) return undefined;
  const start = m[1] === "" ? Math.max(0, size - Number(m[2])) : Number(m[1]);
  const end = m[1] === "" || m[2] === "" ? size - 1 : Math.min(Number(m[2]), size - 1);
  return start < size && start <= end ? [start, end] : null;
}

createServer(async (req, res) => {
  let path;
  try {
    path = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname));
  } catch {
    // A malformed escape: answered, rather than left to throw out of the handler and stop the server.
    return res.writeHead(400).end("bad request");
  }
  const file = join(root, path.endsWith("/") ? path + "index.html" : path);
  if (!file.startsWith(root)) return res.writeHead(403).end();
  try {
    const body = await readFile(file);
    const headers = {
      "content-type": types[extname(file)] ?? "application/octet-stream",
      "cache-control": process.env.CACHE_CONTROL ?? "no-store",
      "accept-ranges": "bytes",
    };
    const range = rangeOf(req.headers.range ?? "", body.length);
    if (range === null) return res.writeHead(416, { "content-range": `bytes */${body.length}` }).end();
    if (range === undefined) return res.writeHead(200, headers).end(body);
    const [start, end] = range;
    res
      .writeHead(206, { ...headers, "content-range": `bytes ${start}-${end}/${body.length}` })
      .end(body.subarray(start, end + 1));
  } catch {
    res.writeHead(404).end("not found");
  }
}).listen(Number(process.env.E2E_PORT ?? 8137), process.env.E2E_HOST ?? "127.0.0.1");
