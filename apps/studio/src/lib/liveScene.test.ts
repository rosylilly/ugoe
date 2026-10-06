import { describe, expect, it } from "vitest";
import { LiveSceneWatch } from "./liveScene";

describe("LiveSceneWatch", () => {
  it("reports a switch to another scene but not the first value", () => {
    const watch = new LiveSceneWatch();
    expect(watch.changed("a")).toBe(false);
    expect(watch.changed("a")).toBe(false);
    expect(watch.changed("b")).toBe(true);
    // 出すシーンが無くなったときは読まない (次に出したときに読む)
    expect(watch.changed(null)).toBe(false);
    expect(watch.changed("b")).toBe(true);
  });

  it("does not report a profile that starts empty and then gets its first scene", () => {
    const watch = new LiveSceneWatch();
    expect(watch.changed(null)).toBe(false);
    expect(watch.changed("a")).toBe(true);
  });
});
