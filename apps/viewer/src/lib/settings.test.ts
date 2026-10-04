import { describe, expect, it } from "vitest";
import { LOCALE_STORAGE_KEY, THEME_STORAGE_KEY } from "./settings";

describe("storage keys", () => {
  it("keeps the keys the viewer has always used for the language and the theme", () => {
    // キーを変えると、覚えた設定が読めなくなる (T0925)。index.html の <head> のスクリプトもテーマのキーを読む。
    expect(LOCALE_STORAGE_KEY).toBe("ugoe-viewer.locale");
    expect(THEME_STORAGE_KEY).toBe("ugoe-viewer.theme");
  });
});
