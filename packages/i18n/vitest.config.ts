import { fileURLToPath } from "node:url";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

// `.svelte.ts` の runes を使うので Svelte のプラグインを通す。$effect を動かすため、ブラウザ向けに変換し、
// svelte もブラウザ向けの版を読む (既定の node 環境では server 向けになり、$effect が動かない)。
export default defineConfig({
  plugins: [svelte()],
  resolve: { conditions: ["browser"] },
  test: {
    environment: fileURLToPath(new URL("./test/client-environment.ts", import.meta.url)),
  },
});
