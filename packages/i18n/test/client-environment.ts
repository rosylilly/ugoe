// Vitest の環境: Node のまま、モジュールをブラウザ向け (Vite の client) に変換する。Svelte の runes が
// client 向けにコンパイルされ、$effect が動く (DOM は持たない)。
import type { Environment } from "vitest/runtime";

export default {
  name: "svelte-client",
  viteEnvironment: "client",
  setup() {
    return { teardown() {} };
  },
} satisfies Environment;
