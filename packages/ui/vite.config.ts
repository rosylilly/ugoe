import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

// Vitest の設定 (部品は happy-dom の上で svelte の mount で描く)。見本の画面の設定は gallery/vite.config.ts。
export default defineConfig({
  plugins: [svelte()],
  resolve: { conditions: ["browser"] },
  test: {
    environment: "happy-dom",
    include: ["src/**/*.test.ts"],
  },
});
