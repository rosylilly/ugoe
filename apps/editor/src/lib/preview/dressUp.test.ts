// プレビューの着せ替え (T0649): 説明の文言・スロットの一覧と、DressUp の付け外し (WebPuppet と IPC は偽物)。
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ModuleDifference } from "../ipc";
import type { WebPuppet } from "../viewport/wasm";
import {
  describeProblem,
  differenceItemCount,
  editingName,
  fileStem,
  filterDifferences,
  isModuleFile,
  parseModules,
  sameNames,
  slotRows,
} from "./dressUp";
import { DressUp } from "./dressUp.svelte";

const saved: unknown[] = [];
/** `comparePreviewModule` の結果 (バイト列はモジュールの名前)。 */
let compared: (bytes: Uint8Array) => ModuleDifference[] | Promise<ModuleDifference[]> = () => [];
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
    comparePreviewModule: vi.fn(async (bytes: Uint8Array) => compared(bytes)),
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
 * `state.lost` の間は GPU を失った WebPuppet で、読み込みと付け外しが `device_lost` を投げる (ugoe-wasm と同じ)。
 */
function fakePuppet(slotOf: Record<string, string>) {
  const loaded: string[] = [];
  let attached: string[] = [];
  const broken = new Set<string>();
  const state = { lost: false };
  /** 直前の `reload` で付け直せなかったもの (`detached_modules_json`)。 */
  let detached: unknown[] = [];
  const lost = () => {
    if (state.lost) throw dressUpError("device_lost", {});
  };
  const check = (names: string[]) => {
    lost();
    for (const name of names) {
      if (!loaded.includes(name)) throw dressUpError("not_loaded", { module: name });
      const slot = slotOf[name] ?? "?";
      if (broken.has(slot)) throw dressUpError("unknown_slot", { module: name, slot });
    }
  };
  const puppet = {
    has_model: () => true,
    is_lost: () => state.lost,
    load_module(bytes: Uint8Array) {
      lost();
      const name = new TextDecoder().decode(bytes);
      if (!(name in slotOf)) throw dressUpError("invalid_file", {});
      if (!loaded.includes(name)) loaded.push(name);
      return name;
    },
    unload_module(name: string) {
      lost();
      attached = attached.filter((n) => n !== name);
      const i = loaded.indexOf(name);
      if (i >= 0) loaded.splice(i, 1);
    },
    attach_module(name: string) {
      check([name]);
      attached = [...attached.filter((n) => slotOf[n] !== slotOf[name] && n !== name), name];
    },
    detach_module(name: string) {
      lost();
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
  return {
    puppet: puppet as unknown as WebPuppet,
    broken,
    state,
    reload,
    attached: () => attached,
    loaded: () => [...loaded],
  };
}

const SLOTS = { LongHair: "Hair", ShortHair: "Hair", Dress: "Outfit" };

function setup() {
  let fake = fakePuppet(SLOTS);
  let active = true;
  const changed = vi.fn();
  const dressUp = new DressUp({ puppet: () => fake.puppet, active: () => active, changed });
  return {
    dressUp,
    get fake() {
      return fake;
    },
    changed,
    setActive: (a: boolean) => {
      active = a;
    },
    /** GPU を失った WebPuppet を作り直す (ViewportController の `#setUp` → `PreviewMode.loaded`)。 */
    recreate() {
      fake = fakePuppet(SLOTS);
      dressUp.loaded(fake.puppet);
      return fake;
    },
  };
}

const bytes = (name: string) => new TextEncoder().encode(name);
const entry = (d: DressUp, name: string) => d.entries.find((e) => e.name === name);

beforeEach(() => {
  saved.length = 0;
  for (const k of Object.keys(files)) delete files[k];
});

describe("filterDifferences", () => {
  const item = (id: string, text: string) => ({
    kind: "mesh" as const,
    id,
    text,
    element: null,
    meshes: [],
    parts: [],
  });
  const lines: ModuleDifference[] = [
    {
      text: "Meshes: 2 → 3 (added: Band)",
      items: [item("Band", "added"), item("Ears", "vertices 12 → 14")],
    },
    { text: "Textures: image changed: Ears", items: [] },
    { text: "Deformers: 1 → 1", items: [item("EarWarp", "keyforms: 1 of 2 changed")] },
  ];

  it("keeps everything for an empty query, and counts the element rows", () => {
    expect(filterDifferences(lines, "  ")).toEqual(lines);
    expect(differenceItemCount(lines)).toBe(3);
  });

  it("keeps a matching line whole, and only the matching elements of other lines", () => {
    expect(filterDifferences(lines, "ears").map((l) => [l.text, l.items.map((i) => i.id)])).toEqual(
      [
        ["Meshes: 2 → 3 (added: Band)", ["Ears"]],
        ["Textures: image changed: Ears", []],
      ],
    );
    // 語のすべてを含むもの (大文字と小文字は区別しない)
    expect(
      filterDifferences(lines, "KEYFORMS earwarp").map((l) => l.items.map((i) => i.id)),
    ).toEqual([["EarWarp"]]);
    expect(filterDifferences(lines, "meshes")[0]?.items).toHaveLength(2);
    expect(filterDifferences(lines, "nothing")).toEqual([]);
  });
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
      { id: "Hair", attached: "ShortHair", candidates: ["LongHair", "ShortHair"], published: true },
      { id: "Outfit", attached: null, candidates: [], published: true },
      { id: "Hat", attached: null, candidates: [], published: true },
    ]);
    expect(sameNames(["a", "b"], ["b", "a"])).toBe(true);
    expect(sameNames(["a"], ["a", "b"])).toBe(false);
  });

  it("marks the slots that the avatar profile does not publish (T0655)", () => {
    const slots = [{ id: "Hair" }, { id: "Outfit" }];
    const published = (list: readonly string[] | null) =>
      slotRows(slots, [], list).map((r) => r.published);
    // プロファイルが無ければ公開の区別が無い。
    expect(published(null)).toEqual([true, true]);
    expect(published(["Hair"])).toEqual([true, false]);
    expect(published([])).toEqual([false, false]);
  });

  it("reads modules_json and words the module warnings like the load warnings (T0652)", () => {
    const ktx = "texture 0 is KTX2, which is not supported yet; using a white placeholder";
    const json = JSON.stringify([
      {
        name: "Hat",
        slot: "Headwear",
        requires: {},
        attached: true,
        textures: 2,
        warnings: [ktx, "an old warning without details"],
        warning_details: [{ code: "texture_ktx2", warning: ktx, texture: 0 }],
      },
      { name: "Cape", slot: "Back", requires: {}, attached: false, textures: 1, warnings: [] },
    ]);
    expect(parseModules(json)).toEqual([
      {
        name: "Hat",
        slot: "Headwear",
        requires: {},
        attached: true,
        textures: 2,
        warnings: [
          "Texture 0 is a KTX2 image in a format the preview can't read yet (such as ETC2, ASTC or Basis), so it is drawn white.",
          "an old warning without details",
        ],
      },
      { name: "Cape", slot: "Back", requires: {}, attached: false, textures: 1, warnings: [] },
    ]);
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
    // 外れたことと理由は、その行に 1 回だけ出す (パネルの頭に同じ理由を重ねない。T1644)。
    expect(entry(dressUp, "Dress")).toMatchObject({ wanted: false, attached: false });
    expect(entry(dressUp, "Dress")?.problem).toMatchObject({
      code: "unknown_slot",
      detail: "unknown_slot happened",
    });
    expect(entry(dressUp, "Dress")?.problem?.summary).toMatch(
      /^Dress was taken off: Dress goes in slot "Outfit"/,
    );
    expect(dressUp).not.toHaveProperty("notices");
    // 同じ読み込みの一覧をもう一度渡されても (読み込みを伴わない loaded)、付け直したものは知らせない。
    fake.broken.delete("Outfit");
    dressUp.attach("Dress");
    dressUp.loaded(fake.puppet);
    expect(fake.attached()).toEqual(["LongHair", "Dress"]);
    expect(entry(dressUp, "Dress")?.problem).toBeNull();
    // プレビューに入り直したとき (WebPuppet が付け直していない) は、1 つずつ試した理由で知らせる。
    dressUp.takeOff();
    fake.broken.add("Outfit");
    dressUp.loaded(fake.puppet);
    expect(fake.attached()).toEqual(["LongHair"]);
    expect(entry(dressUp, "Dress")?.problem?.summary).toMatch(
      /^Dress was taken off: Dress goes in slot "Outfit"/,
    );
  });

  it("loads the kept bytes again into a new WebPuppet", async () => {
    const { dressUp } = setup();
    await dressUp.add([{ path: "/work/LongHair.ugoemod", bytes: bytes("LongHair") }]);
    const other = fakePuppet({ LongHair: "Hair" });
    dressUp.loaded(other.puppet);
    expect(other.attached()).toEqual(["LongHair"]);
  });
});

describe("DressUp when the GPU is lost during a load (T1644)", () => {
  it("loads a module given while the GPU is lost into the new WebPuppet, and puts it on", async () => {
    const t = setup();
    const { dressUp } = t;
    await dressUp.add([{ path: "/work/LongHair.ugoemod", bytes: bytes("LongHair") }]);
    t.fake.state.lost = true;
    await dressUp.add([
      { path: "/work/ShortHair.ugoemod", bytes: bytes("ShortHair") },
      { path: null, bytes: bytes("Dress"), name: "Dress.ugoemod" },
    ]);
    // 失敗の文を出さない (作り直せば読める)。
    expect(dressUp.error).toBeNull();
    expect(dressUp.entries.map((e) => [e.name, e.problem])).toEqual([["LongHair", null]]);
    // 作り直した WebPuppet に、前から持っていたものと、失っている間に渡されたものを読んで付ける
    // (同じスロットは後から渡されたものに差し替わる)。
    const fresh = t.recreate();
    expect(fresh.loaded()).toEqual(["LongHair", "ShortHair", "Dress"]);
    expect(fresh.attached()).toEqual(["ShortHair", "Dress"]);
    expect(dressUp.entries.map((e) => [e.name, e.attached, e.wanted, e.problem])).toEqual([
      ["LongHair", false, false, null],
      ["ShortHair", true, true, null],
      ["Dress", true, true, null],
    ]);
    await vi.waitFor(() =>
      expect(saved.at(-1)).toEqual([
        { path: "/work/LongHair.ugoemod", attached: false },
        { path: "/work/ShortHair.ugoemod", attached: true },
      ]),
    );
  });

  it("keeps the wanted set when putting on or taking off while the GPU is lost", async () => {
    const t = setup();
    const { dressUp } = t;
    await dressUp.add([
      { path: "/work/LongHair.ugoemod", bytes: bytes("LongHair") },
      { path: "/work/ShortHair.ugoemod", bytes: bytes("ShortHair") },
      { path: "/work/Dress.ugoemod", bytes: bytes("Dress") },
    ]);
    expect(t.fake.attached()).toEqual(["ShortHair", "Dress"]);
    t.fake.state.lost = true;
    dressUp.setSlot("Outfit", null);
    // 同じスロットのものは、付けたい組でも差し替える。
    dressUp.attach("LongHair");
    expect(dressUp.entries.map((e) => e.problem)).toEqual([null, null, null]);
    const fresh = t.recreate();
    expect(fresh.attached()).toEqual(["LongHair"]);
    expect(dressUp.entries.map((e) => [e.name, e.wanted, e.problem])).toEqual([
      ["LongHair", true, null],
      ["ShortHair", false, null],
      ["Dress", false, null],
    ]);
  });

  it("reads a file again while the GPU is lost and loads it after the WebPuppet is made again", async () => {
    const t = setup();
    const { dressUp } = t;
    files["/work/LongHair.ugoemod"] = "LongHair";
    await dressUp.load();
    t.fake.state.lost = true;
    await dressUp.refresh();
    expect(entry(dressUp, "LongHair")).toMatchObject({ problem: null, wanted: true });
    const fresh = t.recreate();
    expect(fresh.attached()).toEqual(["LongHair"]);
    expect(entry(dressUp, "missing")?.problem?.summary).toContain("No such file");
  });

  it("does not take modules off when the new WebPuppet is lost again while putting them on", async () => {
    const t = setup();
    const { dressUp } = t;
    await dressUp.add([{ path: "/work/Dress.ugoemod", bytes: bytes("Dress") }]);
    const again = fakePuppet(SLOTS);
    again.state.lost = true;
    dressUp.loaded(again.puppet);
    expect(entry(dressUp, "Dress")).toMatchObject({ wanted: true, problem: null });
    expect(t.recreate().attached()).toEqual(["Dress"]);
  });
});

describe("DressUp in a module project", () => {
  it("does not put another module on the project's own slot", async () => {
    const fake = fakePuppet({ LongHair: "Hair", Dress: "Outfit" });
    const dressUp = new DressUp({
      puppet: () => fake.puppet,
      active: () => true,
      changed: vi.fn(),
      ownSlot: () => "Hair",
    });
    await dressUp.add([
      { path: null, bytes: bytes("LongHair"), name: "LongHair.ugoemod" },
      { path: null, bytes: bytes("Dress"), name: "Dress.ugoemod" },
    ]);
    expect(fake.attached()).toEqual(["Dress"]);
    const hair = entry(dressUp, "LongHair");
    expect(hair?.attached).toBe(false);
    expect(hair?.wanted).toBe(false);
    expect(hair?.problem?.code).toBe("own_slot");
    expect(hair?.problem?.summary).toContain('module for slot "Hair"');
    // 付けたい組に残っていても、読み直しでは付けない
    if (hair) hair.wanted = true;
    dressUp.loaded(fake.puppet);
    expect(fake.attached()).toEqual(["Dress"]);
  });

  it("puts the current edits on the own slot and swaps them with other modules (T0651)", async () => {
    // 本体だけのモデルに付けるとき (PreviewMode が比べている間) は、自分のスロットを除かない。
    const fake = fakePuppet({ Hat: "Hair", LongHair: "Hair", Dress: "Outfit" });
    const dressUp = new DressUp({
      puppet: () => fake.puppet,
      active: () => true,
      changed: vi.fn(),
      ownSlot: () => null,
    });
    // 初めは記録だけで、本体を読み込んだ後の loaded で読み込んで付ける。
    dressUp.setOwn(bytes("Hat"), "Hat", false);
    expect(fake.attached()).toEqual([]);
    dressUp.loaded(fake.puppet);
    expect(fake.attached()).toEqual(["Hat"]);
    expect(dressUp.entries[0]).toMatchObject({
      name: "Hat",
      own: true,
      attached: true,
      slot: "Hair",
    });
    // 同じスロットのモジュールに差し替え、戻す。
    await dressUp.add([{ path: "/work/LongHair.ugoemod", bytes: bytes("LongHair") }]);
    expect(fake.attached()).toEqual(["LongHair"]);
    expect(entry(dressUp, "Hat")?.wanted).toBe(false);
    dressUp.setSlot("Hair", "Hat");
    expect(fake.attached()).toEqual(["Hat"]);
    // 今の編集はプロジェクトの一覧に残さない。
    expect(saved.at(-1)).toEqual([{ path: "/work/LongHair.ugoemod", attached: false }]);
    // 編集の後に取り直すと、付けたまま新しい版に替わる。
    dressUp.setOwn(bytes("Hat"), "Hat", true);
    expect(fake.attached()).toEqual(["Hat"]);
    expect(dressUp.entries.filter((e) => e.own)).toHaveLength(1);
    // 同じ名前のファイル (前に書き出した版) は、今の編集に替えて付けない。
    await dressUp.add([{ path: "/work/Hat.ugoemod", bytes: bytes("Hat") }]);
    expect(dressUp.error?.code).toBe("same_name");
    expect(entry(dressUp, "Hat")?.own).toBe(true);
    expect(fake.attached()).toEqual(["Hat"]);
    // 比べる前 (自分のスロットを除いている間) に揃えても、今の編集を付けたい組は残る。
    let comparing = false;
    const later = new DressUp({
      puppet: () => fake.puppet,
      active: () => true,
      changed: vi.fn(),
      ownSlot: () => (comparing ? null : "Hair"),
    });
    later.setOwn(bytes("Hat"), "Hat", false);
    later.loaded(fake.puppet);
    expect(entry(later, "Hat")?.wanted).toBe(true);
    comparing = true;
    later.loaded(fake.puppet);
    expect(fake.attached()).toEqual(["Hat"]);
    // 比べるのをやめると、行を除く。
    dressUp.setOwn(null, "", true);
    expect(dressUp.entries.some((e) => e.own)).toBe(false);
    expect(fake.attached()).toEqual([]);
  });

  it("compares the current edits with an exported file of the same name (T0653)", async () => {
    const editing = editingName("Hat");
    expect(editing).toBe("Hat (editing)");
    const fake = fakePuppet({ [editing]: "Hair", Hat: "Hair" });
    const dressUp = new DressUp({
      puppet: () => fake.puppet,
      active: () => true,
      changed: vi.fn(),
      ownSlot: () => null,
    });
    dressUp.setOwn(bytes(editing), editing, false, "Hat");
    dressUp.loaded(fake.puppet);
    expect(fake.attached()).toEqual([editing]);
    // 一覧には今の編集をモジュールの名前で出す。
    expect(dressUp.entries[0]).toMatchObject({ name: editing, label: "Hat", own: true });
    // 前に書き出した同じ名前のファイルも読み込めて、同じスロットで差し替えて比べられる。
    await dressUp.add([{ path: "/work/Hat.ugoemod", bytes: bytes("Hat") }]);
    expect(dressUp.error).toBeNull();
    expect(entry(dressUp, "Hat")?.problem).toBeNull();
    expect(fake.attached()).toEqual(["Hat"]);
    dressUp.setSlot("Hair", editing);
    expect(fake.attached()).toEqual([editing]);
    dressUp.setSlot("Hair", "Hat");
    expect(fake.attached()).toEqual(["Hat"]);
    // 取り直しても、ファイルの行はそのまま残る。
    dressUp.setOwn(bytes(editing), editing, true, "Hat");
    expect(fake.attached()).toEqual(["Hat"]);
    expect(dressUp.entries.map((e) => e.name)).toEqual([editing, "Hat"]);
  });

  it("shows how the exported file differs from the current edits (T0654)", async () => {
    const editing = editingName("Hat");
    const fake = fakePuppet({ [editing]: "Hair", Hat: "Hair", Cap: "Hair" });
    const dressUp = new DressUp({
      puppet: () => fake.puppet,
      active: () => true,
      changed: vi.fn(),
      ownSlot: () => null,
    });
    const brim = (name: string): ModuleDifference => ({
      text: `Meshes: 2 → 3 (added: ${name}Brim)`,
      items: [
        {
          kind: "mesh",
          id: `${name}Brim`,
          text: "added",
          element: `${name}/${name}Brim`,
          meshes: [`${name}Brim`],
          parts: [],
        },
      ],
    });
    compared = (b) => [brim(new TextDecoder().decode(b))];
    dressUp.setOwn(bytes(editing), editing, false, "Hat");
    dressUp.loaded(fake.puppet);
    await dressUp.add([{ path: "/work/Hat.ugoemod", bytes: bytes("Hat") }]);
    await vi.waitFor(() => expect(entry(dressUp, "Hat")?.differences).toEqual([brim("Hat")]));
    // ほかの名前のファイルと今の編集の行は比べない。
    await dressUp.add([{ path: "/work/Cap.ugoemod", bytes: bytes("Cap") }]);
    expect(entry(dressUp, "Cap")?.differences).toBeUndefined();
    expect(dressUp.entries[0]?.differences).toBeUndefined();
    // 今の編集を取り直すと比べ直す。
    compared = () => [];
    dressUp.setOwn(bytes(editing), editing, true, "Hat");
    expect(entry(dressUp, "Hat")?.differences).toBeNull();
    await vi.waitFor(() => expect(entry(dressUp, "Hat")?.differences).toEqual([]));
    // 比べるのをやめると消える。
    dressUp.setOwn(null, "", false);
    expect(entry(dressUp, "Hat")?.differences).toBeUndefined();
  });

  it("keeps the highlighted difference while comparing again (T0657)", async () => {
    const editing = editingName("Hat");
    const fake = fakePuppet({ [editing]: "Hair", Hat: "Hair" });
    const dressUp = new DressUp({
      puppet: () => fake.puppet,
      active: () => true,
      changed: vi.fn(),
      ownSlot: () => null,
    });
    const brim = (text: string): ModuleDifference => ({
      text: "Meshes: 2 → 2 (changed: Brim)",
      items: [{ kind: "mesh", id: "Brim", text, element: "Hat/Brim", meshes: ["Brim"], parts: [] }],
    });
    compared = () => [brim("vertices 4 → 6")];
    dressUp.setOwn(bytes(editing), editing, false, "Hat");
    dressUp.loaded(fake.puppet);
    await dressUp.add([{ path: "/work/Hat.ugoemod", bytes: bytes("Hat") }]);
    await vi.waitFor(() => expect(entry(dressUp, "Hat")?.differences).toHaveLength(1));
    const item = entry(dressUp, "Hat")?.differences?.[0]?.items[0];
    if (!item) throw new Error("no item");
    dressUp.toggleHighlight("Hat", item);
    expect(dressUp.highlight).toEqual({ module: "Hat", item });
    // 比べ直すと、同じ要素の新しい中身に替わる。
    compared = () => [brim("vertices 4 → 7")];
    dressUp.setOwn(bytes(editing), editing, true, "Hat");
    await vi.waitFor(() => expect(dressUp.highlight?.item.text).toBe("vertices 4 → 7"));
    // もう一度押すとやめる。
    dressUp.toggleHighlight("Hat", item);
    expect(dressUp.highlight).toBeNull();
    // 無くなった要素の強調はやめる。
    dressUp.toggleHighlight("Hat", item);
    compared = () => [];
    dressUp.setOwn(bytes(editing), editing, true, "Hat");
    await vi.waitFor(() => expect(dressUp.highlight).toBeNull());
    // 比べるのをやめても消える。
    dressUp.toggleHighlight("Hat", item);
    dressUp.setOwn(null, "", false);
    expect(dressUp.highlight).toBeNull();
  });

  it("drops the comparison in progress when the project is opened again (T1614)", async () => {
    const editing = editingName("Hat");
    const fake = fakePuppet({ [editing]: "Hair", Hat: "Hair" });
    const dressUp = new DressUp({
      puppet: () => fake.puppet,
      active: () => true,
      changed: vi.fn(),
      ownSlot: () => null,
    });
    const brim = (text: string): ModuleDifference => ({
      text: "Meshes: 2 → 2 (changed: Brim)",
      items: [{ kind: "mesh", id: "Brim", text, element: "Hat/Brim", meshes: ["Brim"], parts: [] }],
    });
    compared = () => [brim("vertices 4 → 6")];
    dressUp.setOwn(bytes(editing), editing, false, "Hat");
    dressUp.loaded(fake.puppet);
    await dressUp.add([{ path: "/work/Hat.ugoemod", bytes: bytes("Hat") }]);
    await vi.waitFor(() => expect(entry(dressUp, "Hat")?.differences).toHaveLength(1));
    const item = entry(dressUp, "Hat")?.differences?.[0]?.items[0];
    if (!item) throw new Error("no item");
    dressUp.toggleHighlight("Hat", item);
    // 比べ直している途中でプロジェクトを開き直す。
    let finish: (d: ModuleDifference[]) => void = () => {};
    compared = () =>
      new Promise((resolve) => {
        finish = resolve;
      });
    dressUp.setOwn(bytes(editing), editing, true, "Hat");
    await dressUp.load();
    expect(dressUp.highlight).toBeNull();
    // 遅れて届いた古い結果は、強調を書き換えない (同じ名前の行を強調していても)。
    dressUp.toggleHighlight("Hat", item);
    finish([brim("vertices 4 → 7")]);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(dressUp.highlight).toEqual({ module: "Hat", item });
  });
});
