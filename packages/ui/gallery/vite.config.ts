import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

// 部品の見本の画面: `pnpm --filter @ugoe/ui gallery` (開発) / `gallery:build` (dist に書き出す)。
export default defineConfig({
  root: import.meta.dirname,
  base: "./",
  plugins: [svelte()],
  clearScreen: false,
  build: { outDir: "dist", emptyOutDir: true },
  server: { port: 1430 },
});
