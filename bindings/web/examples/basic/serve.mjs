#!/usr/bin/env node
// 見本を配る小さな静的サーバー (依存なし)。`node serve.mjs [port]`、既定のポートは 8080。
//
// - このディレクトリ (index.html と node_modules/@ugoe/runtime) を配る
// - `/samples/` はリポジトリの assets/samples/ (見本のモデル。MIT) を配る
// - wasm は `application/wasm` で返す (WebAssembly.instantiateStreaming が求める)

import { createReadStream, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, sep } from "node:path";

const here = import.meta.dirname;
const samples = join(here, "..", "..", "..", "..", "assets", "samples");
const port = Number(process.argv[2] ?? process.env.PORT ?? 8080);

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
  ".css": "text/css; charset=utf-8",
  ".ugoe": "application/octet-stream",
  ".ugoemod": "application/octet-stream",
};

/** URL のパスを、`base` の中のファイルのパスにする (外に出るパスは `null`)。 */
function resolveIn(base, path) {
  const file = normalize(join(base, decodeURIComponent(path)));
  return file === base || file.startsWith(base + sep) ? file : null;
}

createServer((req, res) => {
  const { pathname } = new URL(req.url ?? "/", "http://localhost");
  let file = null;
  try {
    // 壊れた `%` の並び (`decodeURIComponent` が投げる) でサーバーを落とさない
    file = pathname.startsWith("/samples/")
      ? resolveIn(samples, pathname.slice("/samples/".length))
      : resolveIn(here, pathname === "/" ? "index.html" : pathname.slice(1));
    if (file && statSync(file).isDirectory()) file = join(file, "index.html");
    if (!file || !statSync(file).isFile()) throw new Error("not a file");
  } catch {
    res.writeHead(404, { "content-type": "text/plain" }).end("not found");
    return;
  }
  res.writeHead(200, {
    "content-type": TYPES[extname(file)] ?? "application/octet-stream",
    "cache-control": "no-store",
  });
  createReadStream(file).pipe(res);
}).listen(port, () => {
  console.log(`http://localhost:${port}/`);
});
