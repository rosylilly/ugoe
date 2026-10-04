// 書き出しの設定の欄 (IPC は偽物)。続けて変えても、前の変更を戻さない。
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ExportSettingsForm } from "./exportSettings.svelte";
import type { ExportSettings, ProjectStatus } from "./ipc";

const { ipc } = vi.hoisted(() => ({
  ipc: { getExportSettings: vi.fn(), setExportSettings: vi.fn() },
}));

vi.mock("./ipc", () => ({ ipc }));

const initial: ExportSettings = { atlas: false, atlas_max_size: 2048, atlas_padding: 2 };
const status: ProjectStatus = { name: "Avatar", path: null, modified: true };

function deferred<T>(): {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (e: unknown) => void;
} {
  let resolve: (value: T) => void = () => {};
  let reject: (e: unknown) => void = () => {};
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

beforeEach(() => {
  ipc.getExportSettings.mockReset().mockResolvedValue(initial);
  ipc.setExportSettings.mockReset();
});

describe("ExportSettingsForm", () => {
  it("sends changes one after another on top of the earlier ones", async () => {
    const onstatus = vi.fn();
    const form = new ExportSettingsForm(onstatus);
    await form.load();
    const first = deferred<ProjectStatus>();
    ipc.setExportSettings.mockReturnValueOnce(first.promise).mockResolvedValueOnce(status);
    const a = form.update({ atlas: true });
    const b = form.update({ atlas_max_size: 4096 });
    // 変えた値はすぐ欄に出す。2 つ目は 1 つ目が終わるまで送らない
    expect(form.settings).toEqual({ atlas: true, atlas_max_size: 4096, atlas_padding: 2 });
    await Promise.resolve();
    expect(ipc.setExportSettings).toHaveBeenCalledOnce();
    first.resolve(status);
    await Promise.all([a, b]);
    expect(ipc.setExportSettings.mock.calls.map((c) => c[0])).toEqual([
      { atlas: true, atlas_max_size: 2048, atlas_padding: 2 },
      { atlas: true, atlas_max_size: 4096, atlas_padding: 2 },
    ]);
    expect(form.settings).toEqual({ atlas: true, atlas_max_size: 4096, atlas_padding: 2 });
    expect(onstatus).toHaveBeenCalledTimes(2);
    expect(form.error).toBeNull();
  });

  it("drops a refused change from the field and keeps the later ones", async () => {
    const form = new ExportSettingsForm(vi.fn());
    await form.load();
    ipc.setExportSettings
      .mockRejectedValueOnce(new Error("too large"))
      .mockResolvedValueOnce(status);
    const a = form.update({ atlas_max_size: 99999 });
    const b = form.update({ atlas: true });
    await Promise.all([a, b]);
    expect(ipc.setExportSettings).toHaveBeenLastCalledWith({
      atlas: true,
      atlas_max_size: 2048,
      atlas_padding: 2,
    });
    expect(form.settings).toEqual({ atlas: true, atlas_max_size: 2048, atlas_padding: 2 });
    // 後の変更が通っても、断られたことは見せる (次に変えたときに消す)
    expect(form.error).toBe("too large");
    ipc.setExportSettings.mockResolvedValueOnce(status);
    await form.update({ atlas_padding: 4 });
    expect(form.error).toBeNull();
  });

  it("does not send a padding out of range", async () => {
    const form = new ExportSettingsForm(vi.fn());
    await form.load();
    await form.setPadding(-1);
    expect(ipc.setExportSettings).not.toHaveBeenCalled();
    expect(form.error).toMatch(/padding/i);
  });
});
