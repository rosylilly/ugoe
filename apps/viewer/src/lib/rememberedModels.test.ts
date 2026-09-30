import { describe, expect, it } from "vitest";
import type { DressUpSet } from "./dressUpMemory";
import { defaultLighting } from "./lighting";
import type { ModelIdentity } from "./modelMemory";
import { describeDressUpSet, describeLocation, mergeRemembered } from "./rememberedModels";

const id = (name: string, fp: string, location?: ModelIdentity["location"]): ModelIdentity =>
  location === undefined ? { name, fingerprint: fp } : { name, fingerprint: fp, location };

const set = (attached: string[], extra: string[] = []): DressUpSet => ({
  modules: [...attached, ...extra].map((name) => ({ name, file: id(`${name}.ugoemod`, name) })),
  attached,
});

describe("mergeRemembered", () => {
  it("puts the lighting and the dress-up set of the same model in one row", () => {
    const lit = defaultLighting();
    const rows = mergeRemembered(
      [
        { model: id("b.ugoe", "fb", "sample:b.ugoe"), value: lit },
        { model: id("c.ugoe", "fc"), value: lit },
      ],
      [
        // 場所が同じ (中身は書き出し直して変わった) → b と同じ行。
        { model: id("b.ugoe", "fb2", "sample:b.ugoe"), value: set(["Hair"]) },
        // 指紋が同じ (ドロップで場所なし) → c と同じ行。
        { model: id("c-copy.ugoe", "fc"), value: set([]) },
        { model: id("a.ugoe", "fa"), value: set(["Dress"]) },
      ],
      id("c.ugoe", "fc", "path:/c.ugoe"),
    );
    // 開いているモデルが先頭、あとは名前の順。
    expect(
      rows.map((r) => [r.model.name, r.current, r.lighting !== null, r.dressUp !== null]),
    ).toEqual([
      ["c.ugoe", true, true, true],
      ["a.ugoe", false, false, true],
      ["b.ugoe", false, true, true],
    ]);
  });

  it("is empty when nothing is remembered", () => {
    expect(mergeRemembered([], [], null)).toEqual([]);
  });
});

describe("describe", () => {
  it("shows where the model was", () => {
    expect(describeLocation(id("a", "f", "sample:avatar.ugoe"))).toBe("見本 (avatar.ugoe)");
    expect(describeLocation(id("a", "f", "path:/home/a.ugoe"))).toBe("/home/a.ugoe");
    expect(describeLocation(id("a", "f", "url:https://x/a.ugoe"))).toBe("https://x/a.ugoe");
    expect(describeLocation(id("a", "f"))).toBe("場所なし (中身で見分ける)");
  });

  it("summarizes a dress-up set", () => {
    expect(describeDressUpSet(set(["Hair", "Dress"]))).toBe("Hair・Dress");
    expect(describeDressUpSet(set([], ["Hat"]))).toBe("付けていない (1 個を読み込み)");
    expect(describeDressUpSet(set([]))).toBe("なし");
  });
});
