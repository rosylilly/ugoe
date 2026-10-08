import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Autosave, hasUnsavedChanges, needsSavePath, shouldAutosave } from "./autosave";
import type { ReadOnlyReason } from "./profile";

const view = (path: string | null, dirty: boolean, readOnly: ReadOnlyReason | null = null) => ({
  path,
  dirty,
  readOnly,
});

describe("Autosave", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("saves a changed profile with a path after the delay", () => {
    const save = vi.fn();
    const autosave = new Autosave(save, 800);
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
    const save = vi.fn();
    const autosave = new Autosave(save, 800);
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
    const save = vi.fn();
    const autosave = new Autosave(save, 800);
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
