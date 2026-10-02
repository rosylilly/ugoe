import { fileURLToPath } from "node:url";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;
const page = (name: string) => fileURLToPath(new URL(name, import.meta.url));

// Tauri と Vite の組み合わせ方は https://v2.tauri.app/start/frontend/vite/ に従う。
// エディタ (1420)・ビューア (1430) と同時に起動できるよう、ポートは 1440 を使う。
// 操作ウィンドウ (index.html) と出力ウィンドウ (output.html) は別のページ (src-tauri/tauri.conf.json の windows)。
// ブラウザソースのページ (source.html) は、スタジオの待ち受けが `/source/<トークン>/` の下で出す (T0808)。
// その下でもスクリプトと wasm を読めるよう、ページからの相対のパスで出力する (`base: "./"`)。
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  base: "./",
  build: {
    rollupOptions: {
      input: {
        control: page("index.html"),
        output: page("output.html"),
        source: page("source.html"),
      },
    },
  },
  server: {
    port: 1440,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1441 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
