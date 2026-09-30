import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig, type Plugin } from "vite";

const host = process.env.TAURI_DEV_HOST;

/** 同梱する見本 (assets/samples、MIT) の置き場所。ツールバーの「サンプル」で開く (src/lib/samples.ts)。 */
const SAMPLE_DIR = new URL("../../assets/samples/", import.meta.url);

/** 見本のモデルとモジュール (`samples/` からのパス。`/` 区切り)。 */
function sampleFiles(): string[] {
  return readdirSync(SAMPLE_DIR, { recursive: true, encoding: "utf8" })
    .map((path) => path.replaceAll("\\", "/"))
    .filter((path) => path.endsWith(".ugoe") || path.endsWith(".ugoemod"))
    .sort();
}

/** 見本のファイルを、写さずに開発サーバーで `samples/<パス>` に配り、ビルドの出力に入れる。 */
function sampleModels(): Plugin {
  const read = (path: string) => readFileSync(fileURLToPath(new URL(path, SAMPLE_DIR)));
  return {
    name: "ugoe-sample-models",
    configureServer(server) {
      for (const path of sampleFiles()) {
        server.middlewares.use(`/samples/${path}`, (_req, res) => {
          res.setHeader("Content-Type", "application/octet-stream");
          res.end(read(path));
        });
      }
    },
    generateBundle() {
      for (const path of sampleFiles()) {
        this.emitFile({ type: "asset", fileName: `samples/${path}`, source: read(path) });
      }
    },
  };
}

// Tauri と Vite の組み合わせ方は https://v2.tauri.app/start/frontend/vite/ に従う。
// エディタ (1420) と同時に起動できるよう、ポートは 1430 を使う。
export default defineConfig({
  plugins: [svelte(), sampleModels()],
  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1431 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
