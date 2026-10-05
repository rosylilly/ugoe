// @vitest-environment happy-dom
// 読み込み中のシーン (sceneLoading.svelte.ts)。
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LOADING_DELAY_MS, SceneLoading } from "./sceneLoading.svelte";

/** 終わらせる関数を外に出した切り替え。 */
function deferred() {
  let finish = () => {};
  let fail = (_e: unknown) => {};
  const promise = new Promise<void>((resolve, reject) => {
    finish = resolve;
    fail = reject;
  });
  return { promise, finish: () => finish(), fail: (e: unknown) => fail(e) };
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("SceneLoading", () => {
  it("shows the scene being loaded after a short delay until it finishes", async () => {
    const loading = new SceneLoading();
    const work = deferred();
    const done = loading.track("a", () => work.promise);
    // すぐに終わる切り替えではちらつかせない。
    expect(loading.active).toBe(false);
    vi.advanceTimersByTime(LOADING_DELAY_MS);
    expect(loading.active).toBe(true);
    expect(loading.scene).toBe("a");
    work.finish();
    await done;
    expect(loading.active).toBe(false);
    expect(loading.scene).toBeNull();
  });

  it("does not show a switch that finishes before the delay", async () => {
    const loading = new SceneLoading();
    await loading.track("a", async () => {});
    vi.advanceTimersByTime(LOADING_DELAY_MS * 2);
    expect(loading.active).toBe(false);
  });

  it("keeps showing the later switch when the earlier one finishes", async () => {
    const loading = new SceneLoading();
    const first = deferred();
    const second = deferred();
    const a = loading.track("a", () => first.promise);
    vi.advanceTimersByTime(LOADING_DELAY_MS);
    const b = loading.track("b", () => second.promise);
    vi.advanceTimersByTime(LOADING_DELAY_MS);
    expect(loading.scene).toBe("b");
    // 先の切り替えが後で終わっても (バックエンドは捨てる)、後の切り替えを出し続ける。
    first.finish();
    await a;
    expect(loading.active).toBe(true);
    expect(loading.scene).toBe("b");
    second.finish();
    await b;
    expect(loading.active).toBe(false);
  });

  it("stops showing when the switch fails and passes the error on", async () => {
    const loading = new SceneLoading();
    const work = deferred();
    const done = loading.track(null, () => work.promise);
    vi.advanceTimersByTime(LOADING_DELAY_MS);
    // プロファイルを開くときなど、シーンが分からなければ `null`。
    expect(loading.active).toBe(true);
    expect(loading.scene).toBeNull();
    work.fail(new Error("missing"));
    await expect(done).rejects.toThrow("missing");
    expect(loading.active).toBe(false);
  });
});
