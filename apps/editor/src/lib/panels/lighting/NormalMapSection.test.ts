// @vitest-environment happy-dom
// 法線マップの節: 画像の PNG は、編集のたびではなく画像の版が変わったときだけ読み直す (T1614)。
import { describe, expect, it, vi } from "vitest";
import type { Model } from "../../ipc";
import { FakeDocument, render } from "../../viewport/tools/test/harness.svelte";
import NormalMapSection from "./NormalMapSection.svelte";

/** 画像の名前 → 版 (`getTextureVersions` の結果)。 */
const versions: Record<string, string> = { Skin: "a", Skin_n: "b" };

vi.mock("../../ipc", () => ({
  ipc: {
    getTexturePng: vi.fn(async (name: string) => new TextEncoder().encode(name)),
    getTextureVersions: vi.fn(async (names: string[]) => names.map((n) => versions[n] ?? null)),
    getNormalMapGroup: vi.fn(async () => null),
  },
}));

const { ipc } = await import("../../ipc");

const model = {
  textures: [{ name: "Skin", normal_map: "Skin_n" }],
  meshes: [{ id: "Skin", name: "Skin", texture: 0 }],
} as unknown as Model;

/** 待っている IPC の約束を流す。 */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("NormalMapSection images", () => {
  it("reads the PNGs again only when an image changes", async () => {
    let next = 0;
    URL.createObjectURL = vi.fn(() => `blob:${++next}`);
    URL.revokeObjectURL = vi.fn();
    const doc = new FakeDocument();
    const { target } = render(NormalMapSection, { doc: doc.asDocument, model, texture: 0 });
    await vi.waitFor(() => expect(target.querySelectorAll("img")).toHaveLength(2));
    expect(vi.mocked(ipc.getTexturePng).mock.calls.map(([n]) => n)).toEqual(["Skin", "Skin_n"]);
    // 画像に関わらない編集の後は読み直さない。
    doc.revision += 1;
    await settle();
    expect(ipc.getTexturePng).toHaveBeenCalledTimes(2);
    expect(URL.revokeObjectURL).not.toHaveBeenCalled();
    // 同じ名前の法線マップを作り直したら、その画像だけを読み直す。
    versions.Skin_n = "c";
    doc.revision += 1;
    await vi.waitFor(() => expect(ipc.getTexturePng).toHaveBeenCalledTimes(3));
    expect(vi.mocked(ipc.getTexturePng).mock.lastCall).toEqual(["Skin_n"]);
    await vi.waitFor(() =>
      expect([...target.querySelectorAll("img")].map((i) => i.getAttribute("src"))).toEqual([
        "blob:1",
        "blob:3",
      ]),
    );
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:2");
  });
});
