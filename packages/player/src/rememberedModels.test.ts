import { describe, expect, it } from "vitest";
import type { DressUpSet } from "./dressUpMemory";
import { defaultLighting } from "./lighting";
import type { ModelIdentity } from "./modelMemory";
import {
  describeDressUpSet,
  describeLocation,
  describeRememberedAt,
  loadRememberedOrder,
  mergeRemembered,
  REMEMBERED_ORDER_STORAGE_KEY,
  rememberedCount,
  saveRememberedOrder,
} from "./rememberedModels";

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
      {
        lighting: [
          { model: id("b.ugoe", "fb", "sample:b.ugoe"), value: lit },
          { model: id("c.ugoe", "fc"), value: lit },
        ],
        dressUp: [
          // 場所が同じ (中身は書き出し直して変わった) → b と同じ行。
          { model: id("b.ugoe", "fb2", "sample:b.ugoe"), value: set(["Hair"]) },
          // 指紋が同じ (ドロップで場所なし) → c と同じ行。
          { model: id("c-copy.ugoe", "fc"), value: set([]) },
          { model: id("a.ugoe", "fa"), value: set(["Dress"]) },
        ],
        idle: [],
      },
      id("c.ugoe", "fc", "path:/c.ugoe"),
      "name",
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

  it("sorts by the newest date, with undated (older) entries last (T0717)", () => {
    const lit = defaultLighting();
    const rows = mergeRemembered(
      {
        lighting: [
          { model: id("a.ugoe", "fa"), value: lit, at: 100 },
          { model: id("b.ugoe", "fb"), value: lit },
          { model: id("c.ugoe", "fc"), value: lit, at: 300 },
          { model: id("z.ugoe", "fz"), value: lit },
        ],
        dressUp: [
          // a の組は光源より新しい → a の行は新しい方の日時。
          { model: id("a.ugoe", "fa"), value: set([]), at: 500 },
          { model: id("d.ugoe", "fd"), value: set([]), at: 200 },
        ],
        idle: [],
      },
      id("z.ugoe", "fz"),
    );
    expect(rows.map((r) => [r.model.name, r.rememberedAt])).toEqual([
      // 開いているモデルは先頭のまま。
      ["z.ugoe", null],
      ["a.ugoe", 500],
      ["c.ugoe", 300],
      ["d.ugoe", 200],
      ["b.ugoe", null],
    ]);
  });

  it("adds the idle motion choice to the same row (T0722)", () => {
    const lit = defaultLighting();
    const rows = mergeRemembered(
      {
        lighting: [{ model: id("a.ugoe", "fa", "path:/a.ugoe"), value: lit, at: 100 }],
        dressUp: [],
        idle: [
          // 場所が同じ → a の行。待機の方が新しい。
          { model: id("a.ugoe", "fa2", "path:/a.ugoe"), value: { motion: "idle" }, at: 400 },
          // 待機だけを覚えたモデル (「(なし)」を選んだ)。
          { model: id("b.ugoe", "fb"), value: { motion: null }, at: 300 },
        ],
      },
      null,
    );
    expect(
      rows.map((r) => [
        r.model.name,
        r.lighting !== null,
        r.idle,
        r.rememberedAt,
        rememberedCount(r),
      ]),
    ).toEqual([
      ["a.ugoe", true, { motion: "idle" }, 400, 2],
      ["b.ugoe", false, { motion: null }, 300, 1],
    ]);
    // 行の鍵は光源の項目のもの。
    expect(rows[0]?.model.fingerprint).toBe("fa");
  });

  it("is empty when nothing is remembered", () => {
    expect(mergeRemembered({ lighting: [], dressUp: [], idle: [] }, null)).toEqual([]);
  });
});

describe("describe", () => {
  it("shows where the model was", () => {
    expect(describeLocation(id("a", "f", "sample:avatar.ugoe"))).toBe("見本 (avatar.ugoe)");
    expect(describeLocation(id("a", "f", "path:/home/a.ugoe"))).toBe("/home/a.ugoe");
    expect(describeLocation(id("a", "f", "url:https://x/a.ugoe"))).toBe("https://x/a.ugoe");
    expect(describeLocation(id("a", "f"))).toBe("場所なし (中身で見分ける)");
  });

  it("shows when it was remembered", () => {
    expect(describeRememberedAt(new Date(2026, 9, 1, 9, 5).getTime())).toBe("2026/10/01 09:05");
    expect(describeRememberedAt(null)).toBe("日時なし");
  });

  it("summarizes a dress-up set", () => {
    expect(describeDressUpSet(set(["Hair", "Dress"]))).toBe("Hair・Dress");
    expect(describeDressUpSet(set([], ["Hat"]))).toBe("付けていない (1 個を読み込み)");
    expect(describeDressUpSet(set([]))).toBe("なし");
  });
});

describe("remembered order", () => {
  /** 最小の Storage (getItem と setItem だけ使う)。 */
  const storage = (items: Record<string, string> = {}) =>
    ({
      getItem: (key: string) => items[key] ?? null,
      setItem: (key: string, value: string) => {
        items[key] = value;
      },
    }) as unknown as Storage;

  it("remembers the chosen order (T0718)", () => {
    const s = storage();
    expect(loadRememberedOrder(s)).toBe("recent");
    saveRememberedOrder("name", s);
    expect(loadRememberedOrder(s)).toBe("name");
    saveRememberedOrder("recent", s);
    expect(loadRememberedOrder(s)).toBe("recent");
  });

  it("falls back to the newest first for unknown values and a broken storage", () => {
    expect(loadRememberedOrder(storage({ [REMEMBERED_ORDER_STORAGE_KEY]: "size" }))).toBe("recent");
    const broken = {
      getItem: () => {
        throw new Error("denied");
      },
      setItem: () => {
        throw new Error("quota");
      },
    } as unknown as Storage;
    expect(loadRememberedOrder(broken)).toBe("recent");
    expect(() => saveRememberedOrder("name", broken)).not.toThrow();
  });
});
