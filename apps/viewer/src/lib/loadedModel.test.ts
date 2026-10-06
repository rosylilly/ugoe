import { describe, expect, it } from "vitest";
import { LoadedModelWatch } from "./loadedModel";

describe("LoadedModelWatch", () => {
  it("reports each newly opened model once", () => {
    const watch = new LoadedModelWatch();
    expect(watch.changed(null)).toBe(false);
    expect(watch.changed("a.ugoe")).toBe(true);
    // 着せ替えなどの読み直し
    expect(watch.changed("a.ugoe")).toBe(false);
    expect(watch.changed("b.ugoe")).toBe(true);
    // 読み込みの途中を挟めば、同じ名前でも開き直したことになる
    expect(watch.changed(null)).toBe(false);
    expect(watch.changed("b.ugoe")).toBe(true);
  });
});
