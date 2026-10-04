import { describe, expect, it } from "vitest";
import { LOCALE_KEY, systemLocale } from "./locale";

describe("locale preference", () => {
  it("keeps the key the editor has always used", () => {
    // キーを変えると、覚えた設定が読めなくなる (T0925)。
    expect(LOCALE_KEY).toBe("ugoe-editor.locale");
  });

  it("leaves the OS language to navigator.languages outside Tauri", async () => {
    await expect(systemLocale()).resolves.toBeNull();
  });
});
