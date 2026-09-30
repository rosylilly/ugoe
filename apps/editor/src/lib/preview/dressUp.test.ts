// プレビューの着せ替え (T0649): 説明の文言・スロットの一覧と、DressUp の付け外し (WebPuppet と IPC は偽物)。
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { WebPuppet } from "../viewport/wasm";
import { describeProblem, fileStem, isModuleFile, sameNames, slotRows } from "./dressUp";
import { DressUp } from "./dressUp.svelte";

const saved: unknown[] = [];
const files: Record<string, string> = {};

vi.mock("../ipc", () => ({
  ipc: {
    getPreviewModules: vi.fn(async () => [
      { path: "/work/LongHair.ugoemod", attached: true },
      { path: "/work/missing.ugoemod", attached: false },
    ]),
    setPreviewModules: vi.fn(async (modules: unknown) => {
      saved.push(modules);
      return { name: "p", path: null, modified: true };
    }),
    readModuleFile: vi.fn(async (path: string) => {
      const name = files[path];
      if (name === undefined) throw `${path}: No such file or directory`;
      return new TextEncoder().encode(name);
    }),
  },
}));

/** ugoe-wasm の DressUpError と同じ形のエラー。 */
function dressUpError(code: string, fields: Record<string, string>): Error {
  return Object.assign(new Error(`${code} happened`), { name: "DressUpError", code, ...fields });
}

/**
 * 着せ替えの WebPuppet の偽物。バイト列はモジュールの名前の文字列で、`slots` にスロットを持つ。
 * `broken` のスロットのモジュールは付けられない (付けた後にスロットが消えた、などの代わり)。
 */
function fakePuppet(slotOf: Record<string, string>) {
  const loaded: string[] = [];
  let attached: string[] = [];
  const broken = new Set<string>();
  /** 直前の `reload` で付け直せなかったもの (`detached_modules_json`)。 */
  let detached: unknown[] = [];
  const check = (names: string[]) => {
    for (const name of names) {
      if (!loaded.includes(name)) throw dressUpError("not_loaded", { module: name });
      const slot = slotOf[name] ?? "?";
      if (broken.has(slot)) throw dressUpError("unknown_slot", { module: name, slot });
    }
  };
  const puppet = {
    has_model: () => true,
    load_module(bytes: Uint8Array) {
      const name = new TextDecoder().decode(bytes);
      if (!(name in slotOf)) throw dressUpError("invalid_file", {});
      if (!loaded.includes(name)) loaded.push(name);
      return name;
    },
    unload_module(name: string) {
      attached = attached.filter((n) => n !== name);
      const i = loaded.indexOf(name);
      if (i >= 0) loaded.splice(i, 1);
    },
    attach_module(name: string) {
      check([name]);
      attached = [...attached.filter((n) => slotOf[n] !== slotOf[name] && n !== name), name];
    },
    detach_module(name: string) {
      if (!attached.includes(name)) throw dressUpError("not_attached", { module: name });
      attached = attached.filter((n) => n !== name);
    },
    set_modules(names: string[]) {
      check(names);
      attached = [...names];
    },
    attached_modules: () => [...attached],
    detached_modules_json: () => JSON.stringify(detached),
    modules_json: () =>
      JSON.stringify(
        loaded.map((name) => ({
          name,
          slot: slotOf[name],
          requires: {},
          attached: attached.includes(name),
          textures: 1,
          warnings: [],
        })),
      ),
  };
  /** モデルの読み直し (T0123): 付けていたものを 1 つずつ付け直し、付けられないものを外して一覧に残す。 */
  const reload = () => {
    const before = attached;
    attached = [];
    detached = [];
    for (const name of before) {
      try {
        check([name]);
        attached.push(name);
      } catch (e) {
        const { code, module, slot } = e as Error & Record<string, string>;
        detached.push({
          module: name,
          slot: slotOf[name],
          error: { code, message: (e as Error).message, module, slot },
          warning: `module \`${name}\` was detached: ${(e as Error).message}`,
        });
      }
    }
  };
  return { puppet: puppet as unknown as WebPuppet, broken, reload, attached: () => attached };
}

function setup() {
  const fake = fakePuppet({ LongHair: "Hair", ShortHair: "Hair", Dress: "Outfit" });
  let active = true;
  const changed = vi.fn();
  const dressUp = new DressUp({ puppet: () => fake.puppet, active: () => active, changed });
  return {
    dressUp,
    fake,
    changed,
    setActive: (a: boolean) => {
      active = a;
    },
  };
}

const bytes = (name: string) => new TextEncoder().encode(name);
const entry = (d: DressUp, name: string) => d.entries.find((e) => e.name === name);

beforeEach(() => {
  saved.length = 0;
  for (const k of Object.keys(files)) delete files[k];
});

describe("dressUp helpers", () => {
  it("recognizes module files and names them by the file", () => {
    expect(isModuleFile("C:\\mods\\Hair.UGOEMOD")).toBe(true);
    expect(isModuleFile("avatar.ugoe")).toBe(false);
    expect(fileStem("/work/Long Hair.ugoemod")).toBe("Long Hair");
    expect(fileStem("C:\\mods\\hat.ugoemod")).toBe("hat");
  });

  it("explains DressUpError codes in plain words", () => {
    const p = describeProblem(
      dressUpError("parameter_not_exposed", {
        module: "Dress",
        slot: "Outfit",
        parameter: "Secret",
      }),
    );
    expect(p.code).toBe("parameter_not_exposed");
    expect(p.summary).toContain('parameter "Secret"');
    expect(p.summary).toContain('slot "Outfit"');
    expect(p.summary).toContain("Slots tab");
    expect(p.detail).toBe("parameter_not_exposed happened");
    expect(
      describeProblem(dressUpError("unknown_slot", { module: "Hat", slot: "Head" })).summary,
    ).toMatch(/^Hat goes in slot "Head", but this model has no such slot/);
    // DressUpError でなければ元の文言のまま。
    expect(describeProblem("disk full")).toEqual({
      summary: "disk full",
      detail: "disk full",
      code: null,
    });
  });

  it("lists the slots with the loaded modules for each", () => {
    const rows = slotRows(
      [{ id: "Hair" }, { id: "Outfit" }, { id: "Hat" }],
      [
        { name: "LongHair", slot: "Hair", attached: false },
        { name: "ShortHair", slot: "Hair", attached: true },
        { name: "Cape", slot: "Back", attached: false },
      ],
    );
    expect(rows).toEqual([
      { id: "Hair", attached: "ShortHair", candidates: ["LongHair", "ShortHair"] },
      { id: "Outfit", attached: null, candidates: [] },
      { id: "Hat", attached: null, candidates: [] },
    ]);
    expect(sameNames(["a", "b"], ["b", "a"])).toBe(true);
    expect(sameNames(["a"], ["a", "b"])).toBe(false);
  });
});

describe("DressUp", () => {
  it("loads, puts on and swaps modules in the same slot, and keeps the list in the project", async () => {
    const { dressUp, fake, changed } = setup();
    await dressUp.add([{ path: "/work/LongHair.ugoemod", bytes: bytes("LongHair") }]);
    expect(fake.attached()).toEqual(["LongHair"]);
    expect(entry(dressUp, "LongHair")).toMatchObject({
      slot: "Hair",
      attached: true,
      wanted: true,
    });
    expect(changed).toHaveBeenCalled();
    // 同じスロットの別のモジュールは差し替わる。
    await dressUp.add([{ path: "/work/ShortHair.ugoemod", bytes: bytes("ShortHair") }]);
    expect(fake.attached()).toEqual(["ShortHair"]);
    expect(entry(dressUp, "LongHair")).toMatchObject({ attached: false, wanted: false });
    dressUp.setSlot("Hair", "LongHair");
    expect(fake.attached()).toEqual(["LongHair"]);
    dressUp.setSlot("Hair", null);
    expect(fake.attached()).toEqual([]);
    await vi.waitFor(() => expect(saved.length).toBeGreaterThanOrEqual(4));
    expect(saved.at(-1)).toEqual([
      { path: "/work/LongHair.ugoemod", attached: false },
      { path: "/work/ShortHair.ugoemod", attached: false },
    ]);
    dressUp.remove("ShortHair");
    expect(dressUp.entries.map((e) => e.name)).toEqual(["LongHair"]);
  });

  it("shows why a module cannot be put on, and a bad file as an error", async () => {
    const { dressUp, fake } = setup();
    fake.broken.add("Outfit");
    await dressUp.add([
      { path: null, bytes: bytes("Dress") },
      { path: "/work/junk.ugoemod", bytes: bytes("junk") },
    ]);
    expect(entry(dressUp, "Dress")).toMatchObject({ attached: false, wanted: false, path: null });
    expect(entry(dressUp, "Dress")?.problem?.code).toBe("unknown_slot");
    expect(dressUp.error?.summary).toMatch(/^junk: This is not a \.ugoemod file/);
    expect(dressUp.entries).toHaveLength(1);
    // パスの分からない (ドロップした) ものはプロジェクトに残さない。
    await vi.waitFor(() => expect(saved.at(-1)).toEqual([]));
  });

  it("reads the kept list from the project and puts the modules on when the preview starts", async () => {
    const { dressUp, fake } = setup();
    files["/work/LongHair.ugoemod"] = "LongHair";
    await dressUp.load();
    expect(dressUp.entries.map((e) => [e.name, e.loaded, e.wanted])).toEqual([
      ["LongHair", false, true],
      ["missing", false, false],
    ]);
    await dressUp.refresh();
    expect(fake.attached()).toEqual(["LongHair"]);
    expect(entry(dressUp, "missing")?.problem?.summary).toContain("No such file");
  });

  it("takes modules off outside the preview and tells when a reload drops one", async () => {
    const { dressUp, fake, setActive } = setup();
    await dressUp.add([
      { path: "/work/LongHair.ugoemod", bytes: bytes("LongHair") },
      { path: "/work/Dress.ugoemod", bytes: bytes("Dress") },
    ]);
    expect(fake.attached()).toEqual(["LongHair", "Dress"]);
    dressUp.takeOff();
    setActive(false);
    expect(fake.attached()).toEqual([]);
    expect(entry(dressUp, "Dress")?.wanted).toBe(true);
    // 入り直すと付け直す。
    setActive(true);
    dressUp.loaded(fake.puppet);
    expect(fake.attached()).toEqual(["LongHair", "Dress"]);
    // 編集でスロットが無くなった後に読み直すと、WebPuppet が外したもの (detached_modules_json) が知らされる。
    fake.broken.add("Outfit");
    fake.reload();
    const setModules = vi.spyOn(fake.puppet, "set_modules");
    dressUp.loaded(fake.puppet);
    expect(fake.attached()).toEqual(["LongHair"]);
    // 付け直しを試し直さない (理由は WebPuppet の code から)。
    expect(setModules).not.toHaveBeenCalled();
    expect(dressUp.notices).toHaveLength(1);
    expect(dressUp.notices[0]).toMatch(/^Dress was taken off: Dress goes in slot "Outfit"/);
    expect(entry(dressUp, "Dress")).toMatchObject({ wanted: false, attached: false });
    expect(entry(dressUp, "Dress")?.problem).toMatchObject({
      code: "unknown_slot",
      detail: "unknown_slot happened",
    });
    // 同じ読み込みの一覧をもう一度渡されても (読み込みを伴わない loaded)、付け直したものは知らせない。
    fake.broken.delete("Outfit");
    dressUp.attach("Dress");
    dressUp.loaded(fake.puppet);
    expect(dressUp.notices).toHaveLength(1);
    expect(fake.attached()).toEqual(["LongHair", "Dress"]);
    dressUp.dismissNotice(0);
    expect(dressUp.notices).toEqual([]);
    // プレビューに入り直したとき (WebPuppet が付け直していない) は、1 つずつ試した理由で知らせる。
    dressUp.takeOff();
    fake.broken.add("Outfit");
    dressUp.loaded(fake.puppet);
    expect(fake.attached()).toEqual(["LongHair"]);
    expect(dressUp.notices[0]).toMatch(/^Dress was taken off: Dress goes in slot "Outfit"/);
  });

  it("loads the kept bytes again into a new WebPuppet", async () => {
    const { dressUp } = setup();
    await dressUp.add([{ path: "/work/LongHair.ugoemod", bytes: bytes("LongHair") }]);
    const other = fakePuppet({ LongHair: "Hair" });
    dressUp.loaded(other.puppet);
    expect(other.attached()).toEqual(["LongHair"]);
  });
});
