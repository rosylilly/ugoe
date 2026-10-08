import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  Autosave,
  type AutosaveOptions,
  hasUnsavedChanges,
  needsSavePath,
  shouldAutosave,
} from "./autosave";
import type { ReadOnlyReason } from "./profile";

const view = (path: string | null, dirty: boolean, readOnly: ReadOnlyReason | null = null) => ({
  path,
  dirty,
  readOnly,
});

/** `save` を差し替えた自動の保存 (待つ時間は 800 ミリ秒、失敗の後は 30 秒)。 */
function autosaveWith(save: AutosaveOptions["save"]) {
  const onError = vi.fn();
  const onRecovered = vi.fn();
  const autosave = new Autosave({ save, delayMs: 800, retryMs: 30_000, onError, onRecovered });
  return { autosave, onError, onRecovered };
}

describe("Autosave", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("saves a changed profile with a path after the delay", () => {
    const save = vi.fn(async () => {});
    const { autosave } = autosaveWith(save);
    autosave.schedule(view("/p.json", true));
    vi.advanceTimersByTime(799);
    expect(save).not.toHaveBeenCalled();
    // 続けて変えたら待ち直す
    autosave.schedule(view("/p.json", true));
    vi.advanceTimersByTime(799);
    expect(save).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(save).toHaveBeenCalledTimes(1);
  });

  it("does not save profiles that are unchanged, without a path or kept as they are (T0866)", () => {
    const save = vi.fn(async () => {});
    const { autosave } = autosaveWith(save);
    for (const v of [
      view("/p.json", false),
      view(null, true),
      view("/samples/ugoe-sample-profile.json", true, "template"),
      view("/read-only/p.json", true, "notWritable"),
    ]) {
      autosave.schedule(v);
      vi.advanceTimersByTime(10_000);
    }
    expect(save).not.toHaveBeenCalled();
  });

  it("drops the pending save when the profile becomes read-only", () => {
    const save = vi.fn(async () => {});
    const { autosave } = autosaveWith(save);
    autosave.schedule(view("/p.json", true));
    // 保存に失敗して、書けない場所と分かった
    autosave.schedule(view("/p.json", true, "notWritable"));
    vi.advanceTimersByTime(10_000);
    expect(save).not.toHaveBeenCalled();
    // 写しを保存すると、写しを自動で保存する
    autosave.schedule(view("/copy.json", true));
    vi.advanceTimersByTime(800);
    expect(save).toHaveBeenCalledTimes(1);
    autosave.schedule(view("/copy.json", true));
    autosave.cancel();
    vi.advanceTimersByTime(10_000);
    expect(save).toHaveBeenCalledTimes(1);
  });
});

describe("saving", () => {
  it("asks for a path for read-only profiles, new profiles and save as", () => {
    expect(needsSavePath(view("/p.json", true), false)).toBe(false);
    expect(needsSavePath(view("/p.json", true), true)).toBe(true);
    expect(needsSavePath(view(null, true), false)).toBe(true);
    expect(needsSavePath(null, false)).toBe(true);
    expect(needsSavePath(view("/p.json", false, "template"), false)).toBe(true);
    expect(needsSavePath(view("/p.json", true, "notWritable"), false)).toBe(true);
    expect(shouldAutosave(view("/p.json", true))).toBe(true);
  });

  it("treats changes that are not saved automatically as unsaved", () => {
    expect(hasUnsavedChanges(view(null, true))).toBe(true);
    expect(hasUnsavedChanges(view("/p.json", true, "template"))).toBe(true);
    expect(hasUnsavedChanges(view("/p.json", true, "notWritable"))).toBe(true);
    expect(hasUnsavedChanges(view("/p.json", true))).toBe(false);
    expect(hasUnsavedChanges(view("/p.json", false, "template"))).toBe(false);
    expect(hasUnsavedChanges(view(null, false))).toBe(false);
    expect(hasUnsavedChanges(null)).toBe(false);
  });
});

describe("Autosave after failures (T0870)", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  const diskFull = new Error("disk full");

  it("tells a failure once while it lasts and retries after a longer wait", async () => {
    let failure: Error | null = diskFull;
    const changed = view("/p.json", true);
    const save = vi.fn(async () => {
      if (failure === null) return;
      // 保存の後の通知 (変更は残っている) は、保存の失敗より先に届く
      autosave.schedule(changed);
      throw failure;
    });
    const { autosave, onError, onRecovered } = autosaveWith(save);
    autosave.schedule(changed);
    await vi.advanceTimersByTimeAsync(800);
    expect(save).toHaveBeenCalledTimes(1);
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onError).toHaveBeenCalledWith(diskFull);
    expect(autosave.failing).toBe(true);

    // 続けて変えても、すぐには書かない (失敗の後の間隔で 1 度だけ試す)
    autosave.schedule(changed);
    await vi.advanceTimersByTimeAsync(10_000);
    autosave.schedule(changed);
    await vi.advanceTimersByTimeAsync(10_000);
    expect(save).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(10_000);
    expect(save).toHaveBeenCalledTimes(2);
    // 同じ失敗は知らせ直さない
    expect(onError).toHaveBeenCalledTimes(1);
    // 変えなくても、失敗している間は間を空けて試し続ける
    await vi.advanceTimersByTimeAsync(30_000);
    expect(save).toHaveBeenCalledTimes(3);
    expect(onError).toHaveBeenCalledTimes(1);

    // 種類の違う失敗は知らせる
    failure = new Error("busy");
    await vi.advanceTimersByTimeAsync(30_000);
    expect(onError).toHaveBeenCalledTimes(2);
    expect(onRecovered).not.toHaveBeenCalled();

    // 空きができて書けたら、知らせを消し、いつもの間隔に戻る
    failure = null;
    await vi.advanceTimersByTimeAsync(30_000);
    expect(save).toHaveBeenCalledTimes(5);
    expect(onRecovered).toHaveBeenCalledTimes(1);
    expect(autosave.failing).toBe(false);
    autosave.schedule(view("/p.json", false));
    await vi.advanceTimersByTimeAsync(60_000);
    expect(save).toHaveBeenCalledTimes(5);
    autosave.schedule(changed);
    await vi.advanceTimersByTimeAsync(800);
    expect(save).toHaveBeenCalledTimes(6);

    // 同じ失敗でも、一度直った後なら知らせ直す
    failure = diskFull;
    autosave.schedule(changed);
    await vi.advanceTimersByTimeAsync(800);
    expect(onError).toHaveBeenCalledTimes(3);
  });

  it("stops retrying once the profile turns read-only and recovers on a copy", async () => {
    // 権限で失敗した: バックエンドが書き換えないプロファイルにし、その通知が保存の失敗より先に届く
    const save = vi.fn(async () => {
      autosave.schedule(view("/p.json", true, "notWritable"));
      throw diskFull;
    });
    const { autosave, onError, onRecovered } = autosaveWith(save);
    autosave.schedule(view("/p.json", true));
    await vi.advanceTimersByTimeAsync(120_000);
    expect(save).toHaveBeenCalledTimes(1);
    expect(onError).toHaveBeenCalledTimes(1);
    // 写しを保存した (保存先が変わった) ら、知らせを消す
    autosave.schedule(view("/copy.json", false));
    expect(onRecovered).toHaveBeenCalledTimes(1);
  });

  it("drops the failure when another profile is opened or the profile is saved by hand", async () => {
    const save = vi.fn(async () => {
      throw diskFull;
    });
    const { autosave, onError, onRecovered } = autosaveWith(save);
    autosave.schedule(view("/p.json", true));
    await vi.advanceTimersByTimeAsync(800);
    expect(onError).toHaveBeenCalledTimes(1);
    // 手で保存できた (変更が無くなった)
    autosave.schedule(view("/p.json", false));
    expect(onRecovered).toHaveBeenCalledTimes(1);
    autosave.schedule(view("/p.json", true));
    await vi.advanceTimersByTimeAsync(800);
    expect(onError).toHaveBeenCalledTimes(2);
    // 新しいプロファイルにした
    autosave.schedule(view(null, false));
    expect(onRecovered).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(120_000);
    expect(save).toHaveBeenCalledTimes(2);
  });
});
