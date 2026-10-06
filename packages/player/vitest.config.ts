import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

// 文言 (@ugoe/i18n) が `.svelte.ts` の runes を使うので、Svelte のプラグインを通す。部品のテスト (`*.svelte.test.ts` の先頭で
// `@vitest-environment happy-dom`) は mount と $effect を動かすため、svelte をブラウザ向けの版 (`browser` の条件) で解決する。
export default defineConfig({
  plugins: [svelte()],
  resolve: { conditions: ["browser"] },
});
