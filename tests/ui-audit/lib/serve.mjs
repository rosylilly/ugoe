// ビルドしたアプリ (dist) を配る小さな静的サーバ。空いているポート (127.0.0.1) で待ち受ける。
import { createReadStream, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, sep } from "node:path";

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".wasm": "application/wasm",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
  ".woff2": "font/woff2",
  ".task": "application/octet-stream",
  ".tflite": "application/octet-stream",
};

/** 拡張子から Content-Type を決める (知らないものはバイト列)。 */
export function contentType(path) {
  return TYPES[extname(path).toLowerCase()] ?? "application/octet-stream";
}

/** URL のパスを `root` の中のファイルのパスに。`root` の外を指すなら null。 */
export function resolvePath(root, urlPath) {
  let path;
  try {
    path = decodeURIComponent(urlPath.split("?")[0]);
  } catch {
    return null;
  }
  const full = normalize(join(root, path));
  const base = normalize(root.endsWith(sep) ? root : root + sep);
  if (full !== normalize(root) && !full.startsWith(base)) return null;
  return full;
}

/** `root` を配るサーバを起動し、{url, close} を返す。ディレクトリは index.html を返す。 */
export async function serve(root) {
  const server = createServer((req, res) => {
    let path = resolvePath(root, req.url ?? "/");
    if (path === null) {
      res.writeHead(403).end();
      return;
    }
    try {
      if (statSync(path).isDirectory()) path = join(path, "index.html");
      const size = statSync(path).size;
      res.writeHead(200, { "Content-Type": contentType(path), "Content-Length": size });
      createReadStream(path).pipe(res);
    } catch {
      res.writeHead(404).end();
    }
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  const { port } = server.address();
  return {
    url: `http://127.0.0.1:${port}`,
    close: () => new Promise((done) => server.close(done)),
  };
}
