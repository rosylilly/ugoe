#!/usr/bin/env node
// 計測のページを配る小さな静的サーバー (依存なし)。`node serve.mjs [port]`、既定のポートは 8081。
//
// - このディレクトリ (index.html と、`--write` で書き出した生成のモデル generated.ugoe) を配る
// - `/node_modules/@ugoe/runtime/` は bindings/web (パッケージ。`just web-runtime` でビルドした dist/) を配る
//   (tgz を作って入れなくてよい。入れたものではなく、手元のビルドをそのまま測る)
// - `/samples/` はリポジトリの assets/samples/ (見本のモデル。MIT) を配る
// - `performance.now()` の分解能を上げるため (Chrome は 100µs → 5µs)、COOP と COEP を付けて cross-origin isolated にする
// - wasm は `application/wasm` で返す (WebAssembly.instantiateStreaming が求める)

import { createReadStream, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, sep } from "node:path";

const here = import.meta.dirname;
const runtime = join(here, "..", "..");
const samples = join(here, "..", "..", "..", "..", "assets", "samples");
const RUNTIME_PREFIX = "/node_modules/@ugoe/runtime/";

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
  ".ugoe": "application/octet-stream",
};

/** URL のパスを、`base` の中のファイルのパスにする (外に出るパスは `null`)。 */
function resolveIn(base, path) {
  const file = normalize(join(base, decodeURIComponent(path)));
  return file === base || file.startsWith(base + sep) ? file : null;
}

/** URL のパスを配るファイルのパスにする。無ければ `null`。 */
function resolve(pathname) {
  if (pathname.startsWith(RUNTIME_PREFIX)) {
    return resolveIn(runtime, pathname.slice(RUNTIME_PREFIX.length));
  }
  if (pathname.startsWith("/samples/")) {
    return resolveIn(samples, pathname.slice("/samples/".length));
  }
  return resolveIn(here, pathname === "/" ? "index.html" : pathname.slice(1));
}

/** サーバーを `port` (0 なら空いているポート) で開き、開いたサーバーを返す。 */
export function serve(port) {
  const server = createServer((req, res) => {
    const { pathname } = new URL(req.url ?? "/", "http://localhost");
    let file = null;
    try {
      // 壊れた `%` の並び (`decodeURIComponent` が投げる) でサーバーを落とさない
      file = resolve(pathname);
      if (file && statSync(file).isDirectory()) file = join(file, "index.html");
      if (!file || !statSync(file).isFile()) throw new Error("not a file");
    } catch {
      res.writeHead(404, { "content-type": "text/plain" }).end("not found");
      return;
    }
    res.writeHead(200, {
      "content-type": TYPES[extname(file)] ?? "application/octet-stream",
      "cache-control": "no-store",
      "cross-origin-opener-policy": "same-origin",
      "cross-origin-embedder-policy": "require-corp",
    });
    createReadStream(file).pipe(res);
  });
  return new Promise((done) => server.listen(port, "127.0.0.1", () => done(server)));
}

if (process.argv[1] === import.meta.filename) {
  const port = Number(process.argv[2] ?? process.env.PORT ?? 8081);
  const server = await serve(port);
  console.log(`http://localhost:${server.address().port}/`);
}
