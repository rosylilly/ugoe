import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

// Vitest の設定。部品と $effect のテスト (`*.svelte.test.ts` の先頭で `@vitest-environment happy-dom`) は、happy-dom の上で
// ブラウザ向けに変換して走らせる (svelte もブラウザ向けの版を読むため `browser` の条件で解決する。既定の node 環境では
// server 向けになり、mount と $effect が動かない)。アプリのビルドの設定は vite.config.ts。
export default defineConfig({
  plugins: [svelte()],
  resolve: { conditions: ["browser"] },
});
