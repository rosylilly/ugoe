// `@ugoe/runtime/element` を DOM の無い環境 (SSR) で import しても落ちないこと。
import { expect, it, vi } from "vitest";

// wasm の生成物はテストでは無いので、読み込みの補助を差し替える
vi.mock("./runtime.js", () => ({ createPuppet: vi.fn(), loadModel: vi.fn() }));

it("can be imported without customElements (server-side rendering)", async () => {
  expect(globalThis.customElements).toBeUndefined();
  const element = await import("./element.js");
  expect(element.UgoeAvatarElement).toBeTypeOf("function");
});
