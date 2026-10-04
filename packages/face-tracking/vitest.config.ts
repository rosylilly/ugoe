import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

// FaceTracker (`.svelte.ts`) と文言 (@ugoe/i18n) が runes を使うので、Svelte のプラグインを通す。
export default defineConfig({
  plugins: [svelte()],
});
