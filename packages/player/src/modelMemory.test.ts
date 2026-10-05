import { MemoryStorage } from "@ugoe/i18n/testing";
import { describe, expect, it } from "vitest";
import type { ModelFile } from "./modelFile";
import {
  fingerprint,
  type ModelIdentity,
  ModelMemory,
  modelIdentity,
  parseIdentity,
  sameModel,
} from "./modelMemory";
import { PlayerStorage, STORAGE_NAMES } from "./storage";

const bytes = (...b: number[]) => new Uint8Array(b);
const file = (name: string, content: Uint8Array, location?: ModelFile["location"]): ModelFile =>
  location === undefined ? { name, bytes: content } : { name, bytes: content, location };
const numberValue = (v: unknown) => (typeof v === "number" ? v : null);

describe("fingerprint", () => {
  it("is stable, and differs by content and length", () => {
    expect(fingerprint(bytes(1, 2, 3))).toBe(fingerprint(bytes(1, 2, 3)));
    expect(fingerprint(bytes(1, 2, 3))).not.toBe(fingerprint(bytes(1, 2, 4)));
    expect(fingerprint(bytes(1, 2, 3))).not.toBe(fingerprint(bytes(3, 2, 1)));
    expect(fingerprint(bytes(0))).not.toBe(fingerprint(bytes(0, 0)));
    expect(fingerprint(bytes())).toMatch(/^0-[0-9a-f]{16}$/);
  });
});

describe("model identity", () => {
  const a = modelIdentity(file("model.ugoe", bytes(1), "path:/a/model.ugoe"));

  it("keeps the location and ignores the name when comparing", () => {
    expect(a.location).toBe("path:/a/model.ugoe");
    // 書き出し直して中身が変わっても、場所が同じなら同じモデル。
    expect(sameModel(a, modelIdentity(file("x.ugoe", bytes(2), "path:/a/model.ugoe")))).toBe(true);
    // ドロップ (場所なし) でも、中身が同じなら同じモデル。
    expect(sameModel(a, modelIdentity(file("model.ugoe", bytes(1))))).toBe(true);
    // 同じ名前の別のモデル。
    expect(sameModel(a, modelIdentity(file("model.ugoe", bytes(9))))).toBe(false);
    expect(sameModel(a, modelIdentity(file("model.ugoe", bytes(9), "path:/b/model.ugoe")))).toBe(
      false,
    );
  });

  it("parses saved keys and rejects broken ones", () => {
    expect(parseIdentity(JSON.parse(JSON.stringify(a)))).toEqual(a);
    expect(parseIdentity({ fingerprint: "1-x", location: "ftp:/x" })).toEqual({
      name: "",
      fingerprint: "1-x",
    });
    expect(parseIdentity({ name: "a" })).toBeNull();
    expect(parseIdentity("a")).toBeNull();
    expect(parseIdentity(null)).toBeNull();
  });
});

describe("ModelMemory", () => {
  const NAME = STORAGE_NAMES.dressUp;
  const KEY = "p.dress-up";
  const store = (storage: Storage) => new PlayerStorage("p", storage);
  const id = (n: number, location?: string): ModelIdentity =>
    location === undefined
      ? { name: `m${n}`, fingerprint: `fp${n}` }
      : { name: `m${n}`, fingerprint: `fp${n}`, location: `path:${location}` };

  it("remembers values per model, most recent first, up to the limit", () => {
    const storage = new MemoryStorage();
    const memory = new ModelMemory(store(storage), NAME, numberValue, { limit: 3 });
    for (let i = 1; i <= 4; i++) memory.set(id(i), i * 10);
    expect(memory.get(id(1))).toBeNull();
    expect(memory.get(id(2))).toBe(20);
    memory.set(id(2), 21);
    memory.set(id(5), 50);
    // 2 は使い直したので残り、3 が消える。
    expect(memory.models().map((m) => m.name)).toEqual(["m5", "m2", "m4"]);
    memory.delete(id(2));
    expect(memory.get(id(2))).toBeNull();
    // 別の入れ物からも読める (localStorage に書いている)。
    expect(new ModelMemory(store(storage), NAME, numberValue, { limit: 3 }).get(id(5))).toBe(50);
  });

  it("lists entries and forgets everything (T0714)", () => {
    const storage = new MemoryStorage();
    let now = 1000;
    const memory = new ModelMemory(store(storage), NAME, numberValue, {
      limit: 3,
      now: () => now++,
    });
    memory.set(id(1), 10);
    memory.set(id(2, "/b.ugoe"), 20);
    storage.setItem("other", "keep");
    expect(memory.entries()).toEqual([
      { model: id(2, "/b.ugoe"), value: 20, at: 1001 },
      { model: id(1), value: 10, at: 1000 },
    ]);
    memory.clear();
    expect(memory.entries()).toEqual([]);
    expect(storage.getItem(KEY)).toBeNull();
    // ほかのキーには触らない。
    expect(storage.getItem("other")).toBe("keep");
  });

  it("remembers when each entry was set, and reads older values without dates (T0717)", () => {
    const storage = new MemoryStorage();
    // T0717 より前の保存値 (日時なし) と、壊れた日時。
    storage.setItem(
      KEY,
      JSON.stringify({
        version: 1,
        entries: [
          { model: id(1), value: 10 },
          { model: id(2), value: 20, at: "yesterday" },
        ],
      }),
    );
    const memory = new ModelMemory(store(storage), NAME, numberValue, {
      limit: 10,
      now: () => 5000,
    });
    expect(memory.entries()).toEqual([
      { model: id(1), value: 10 },
      { model: id(2), value: 20 },
    ]);
    expect(memory.get(id(2))).toBe(20);
    // 覚え直すと日時が付き、ほかの項目はそのまま (日時なし) で残る。
    memory.set(id(2), 21);
    expect(memory.entries()).toEqual([
      { model: id(2), value: 21, at: 5000 },
      { model: id(1), value: 10 },
    ]);
  });

  it("prefers the entry with the same location, then the same fingerprint", () => {
    const memory = new ModelMemory(store(new MemoryStorage()), NAME, numberValue, { limit: 10 });
    memory.set(id(1), 1);
    memory.set(id(2, "/x.ugoe"), 2);
    // 場所が /x.ugoe で中身が 1 と同じ: 場所の方を優先する。
    expect(memory.get({ name: "x", fingerprint: "fp1", location: "path:/x.ugoe" })).toBe(2);
    expect(memory.get({ name: "x", fingerprint: "fp1" })).toBe(1);
    // 場所の分かったモデルを覚え直すと、同じモデルの古い項目は置き換わり、鍵に場所が付く。
    memory.set({ name: "m1", fingerprint: "fp1", location: "path:/one.ugoe" }, 3);
    expect(memory.models().filter((m) => m.fingerprint === "fp1")).toHaveLength(1);
    expect(memory.get({ name: "?", fingerprint: "changed", location: "path:/one.ugoe" })).toBe(3);
  });

  it("survives broken or foreign values", () => {
    const storage = new MemoryStorage();
    const memory = new ModelMemory(store(storage), NAME, numberValue, { limit: 10 });
    for (const text of ["{not json", "null", "[]", '{"version":99,"entries":[]}']) {
      storage.setItem(KEY, text);
      expect(memory.get(id(1))).toBeNull();
    }
    storage.setItem(
      KEY,
      JSON.stringify({
        version: 1,
        entries: [
          null,
          { model: { name: "x" }, value: 1 },
          { model: id(1), value: "not a number" },
          { model: id(2), value: 2 },
        ],
      }),
    );
    expect(memory.get(id(1))).toBeNull();
    expect(memory.get(id(2))).toBe(2);
    memory.set(id(3), 3);
    expect(memory.models().map((m) => m.name)).toEqual(["m3", "m2"]);
  });

  it("keeps values a newer version wrote, and remembers only for this session (T0937)", () => {
    const storage = new MemoryStorage();
    const newer = JSON.stringify({ version: 2, entries: [{ model: id(1), value: { v2: 10 } }] });
    storage.setItem(KEY, newer);
    const memory = new ModelMemory(store(storage), NAME, numberValue, { limit: 10 });
    // 読めない (形を知らない) ので覚えていないものとして動く。
    expect(memory.get(id(1))).toBeNull();
    expect(memory.entries()).toEqual([]);
    // 覚える・忘れる・すべて忘れるは、このセッションの間だけ効き、新しい版の値は書き換えない。
    memory.set(id(2), 20);
    memory.set(id(3), 30);
    expect(memory.get(id(2))).toBe(20);
    expect(memory.models().map((m) => m.name)).toEqual(["m3", "m2"]);
    memory.delete(id(3));
    expect(memory.models().map((m) => m.name)).toEqual(["m2"]);
    expect(storage.getItem(KEY)).toBe(newer);
    memory.clear();
    expect(memory.entries()).toEqual([]);
    expect(storage.getItem(KEY)).toBe(newer);
    // 起動し直すと (別の入れ物)、このセッションで覚えたものは無い。新しい版の値は残る。
    expect(new ModelMemory(store(storage), NAME, numberValue, { limit: 10 }).get(id(2))).toBeNull();
  });

  it("overwrites broken values and versions that are not newer", () => {
    // 壊れた値・版の無い値・整数でない版は、前と同じく次に覚えるときに書き直す。
    for (const text of [
      "{not json",
      "[]",
      '{"entries":[]}',
      '{"version":"2","entries":[]}',
      '{"version":1.5}',
    ]) {
      const storage = new MemoryStorage();
      storage.setItem(KEY, text);
      new ModelMemory(store(storage), NAME, numberValue, { limit: 10 }).set(id(1), 1);
      expect(JSON.parse(storage.getItem(KEY) ?? "null")).toMatchObject({ version: 1 });
    }
  });

  it("skips entries whose parser throws", () => {
    const storage = new MemoryStorage();
    const throwing = new ModelMemory(
      store(storage),
      NAME,
      (v) => {
        if (v === 1) throw new Error("bad");
        return numberValue(v);
      },
      { limit: 10 },
    );
    storage.setItem(
      KEY,
      JSON.stringify({
        version: 1,
        entries: [
          { model: id(1), value: 1 },
          { model: id(2), value: 2 },
        ],
      }),
    );
    expect(throwing.get(id(2))).toBe(2);
    expect(throwing.get(id(1))).toBeNull();
  });

  it("works without a usable storage", () => {
    const storage = new MemoryStorage();
    storage.getItem = () => {
      throw new Error("denied");
    };
    storage.setItem = () => {
      throw new Error("quota");
    };
    const memory = new ModelMemory(store(storage), NAME, numberValue, { limit: 10 });
    expect(() => memory.set(id(1), 1)).not.toThrow();
    expect(memory.get(id(1))).toBeNull();
    expect(() => memory.delete(id(1))).not.toThrow();
  });
});
