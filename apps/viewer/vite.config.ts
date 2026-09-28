import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig, type Plugin } from "vite";

const host = process.env.TAURI_DEV_HOST;

/** 同梱する見本のモデル (assets/samples/avatar.ugoe、MIT)。「サンプル」で開く。 */
const SAMPLE_URL = "samples/avatar.ugoe";
const samplePath = fileURLToPath(new URL("../../assets/samples/avatar.ugoe", import.meta.url));

/** 見本のモデルを、写さずに開発サーバーで配り、ビルドの出力に入れる。 */
function sampleModel(): Plugin {
  return {
    name: "ugoe-sample-model",
    configureServer(server) {
      server.middlewares.use(`/${SAMPLE_URL}`, (_req, res) => {
        res.setHeader("Content-Type", "application/octet-stream");
        res.end(readFileSync(samplePath));
      });
    },
    generateBundle() {
      this.emitFile({ type: "asset", fileName: SAMPLE_URL, source: readFileSync(samplePath) });
    },
  };
}

// Tauri と Vite の組み合わせ方は https://v2.tauri.app/start/frontend/vite/ に従う。
// エディタ (1420) と同時に起動できるよう、ポートは 1430 を使う。
export default defineConfig({
  plugins: [svelte(), sampleModel()],
  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1431 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
