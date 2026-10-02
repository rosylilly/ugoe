import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

// 文言 (@ugoe/i18n) が `.svelte.ts` の runes を使うので、Svelte のプラグインを通す。
export default defineConfig({
  plugins: [svelte()],
});
