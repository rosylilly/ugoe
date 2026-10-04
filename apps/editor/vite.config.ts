/// <reference types="vitest/config" />
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  // テストでは Svelte のブラウザ向けの実装を読む (部品を happy-dom に描くテスト。docs/testing.md#エディタの部品のテスト)。
  resolve: process.env.VITEST ? { conditions: ["browser"] } : undefined,
  // テストは英語の文言で検査する (src/test-setup.ts)。
  test: { setupFiles: ["./src/test-setup.ts"] },
});
