// Player の着せ替えの組 (覚えた組を尋ねる・戻す・モデルを替えたとき。T0931)。wasm と DOM は偽物 (src/testing/)。
import { describe, expect, it, vi } from "vitest";
import { t } from "./i18n";
import type { FileLocation } from "./modelFile";
import { modelIdentity } from "./modelMemory";
import { bundle, fakePuppet, moduleFile } from "./testing/fakePuppet";
import { moduleRows, ready, setupPlayer } from "./testing/player";

// 画面の部品と MediaPipe まで読み込むと遅いので、Player が使う関数だけにする。
vi.mock("@ugoe/face-tracking", () => import("./testing/faceTracking"));

setupPlayer();

describe("Player dress-up sets", () => {
  const model = bundle("a.ugoe", 1, "path:/a.ugoe");
  const other = bundle("b.ugoe", 2, "path:/b.ugoe");
  const hat = moduleFile("hat", "head", "path:/hat.ugoemod");
  const scarf = moduleFile("scarf", "neck", "path:/scarf.ugoemod");

  /** 前のセッションで `model` に hat を付けていた (localStorage に覚えてある)。 */
  async function rememberHat(): Promise<void> {
    const { p } = await ready();
    await p.load(model, [{ file: hat, attach: true }]);
    expect(moduleRows(p)).toEqual([["hat", true]]);
    expect(p.dressUpRemembered).toBe(true);
    p.detach();
  }

  it("offers the remembered set when the model is opened again and restores it from its location", async () => {
    await rememberHat();
    const reads: FileLocation[] = [];
    const { p } = await ready(
      {},
      {
        readFile: async (location) => {
          reads.push(location);
          if (location === hat.location) return hat;
          throw new Error("missing");
        },
      },
    );
    await p.load(model);
    expect(p.restoreOffer).toEqual({
      modules: [{ name: "hat", file: modelIdentity(hat) }],
      attached: ["hat"],
    });
    expect(p.modules).toEqual([]);
    expect(p.dressUpRemembered).toBe(true);

    await p.restoreDressUp();
    expect(reads).toEqual(["path:/hat.ugoemod"]);
    expect(moduleRows(p)).toEqual([["hat", true]]);
    expect(p.restoreOffer).toBeNull();
    expect(p.dressUpError).toBeNull();
  });

  it("restores without asking when autoRestore is on", async () => {
    await rememberHat();
    const { p } = await ready({}, { readFile: async () => hat });
    p.setAutoRestore(true);
    await p.load(model);
    expect(p.restoreOffer).toBeNull();
    expect(moduleRows(p)).toEqual([["hat", true]]);
  });

  it("keeps the remembered set when a module cannot be found again", async () => {
    const { p: first } = await ready();
    // 場所の無いモジュール (ドロップしたファイル) は、次のセッションでは読み直せない。
    const dropped = moduleFile("ribbon", "head");
    await first.load(model, [{ file: dropped, attach: true }]);
    first.detach();

    const { p } = await ready();
    await p.load(model);
    await p.restoreDressUp();
    expect(p.dressUpError).toBe(
      t("dressUp.restore.noLocation", { module: "ribbon", file: "ribbon.ugoemod" }),
    );
    expect(p.modules).toEqual([]);
    // 覚えている組は変えないので、開き直せばまた尋ねる。
    expect(p.dressUpRemembered).toBe(true);
    await p.load(model);
    expect(p.dressUpRemembered).toBe(true);
    await p.load(other);
    await p.load(model);
    expect(p.restoreOffer?.attached).toEqual(["ribbon"]);
  });

  it("keeps the remembered set when the same model is opened again before it is restored", async () => {
    await rememberHat();
    const { p } = await ready();
    await p.load(model);
    await p.load(model);
    expect(p.restoreOffer?.attached).toEqual(["hat"]);
    expect(p.dressUpRemembered).toBe(true);
    // 今回は戻さないと答えても、着せ替えを操作するまでは覚えている組をそのままにする。
    p.dismissRestore();
    await p.load(model);
    expect(p.dressUpRemembered).toBe(true);
    p.detach();

    const { p: next } = await ready();
    await next.load(model);
    expect(next.restoreOffer?.attached).toEqual(["hat"]);
  });

  it("keeps the modules and asks nothing when the same model is opened again", async () => {
    const { p, log } = await ready();
    await p.load(model, [{ file: hat, attach: true }]);
    await p.load(model);
    expect(moduleRows(p)).toEqual([["hat", true]]);
    expect(p.restoreOffer).toBeNull();
    expect(log).not.toContain("unload hat");
  });

  it("remembers the set again when a module no longer fits the reopened model", async () => {
    const { p, state } = await ready();
    await p.load(model, [
      { file: hat, attach: true },
      { file: scarf, attach: true },
    ]);
    state.unfit = ["scarf"];
    await p.load(model);
    expect(moduleRows(p)).toEqual([
      ["hat", true],
      ["scarf", false],
    ]);
    p.detach();

    const { p: next } = await ready();
    await next.load(model);
    expect(next.restoreOffer?.attached).toEqual(["hat"]);
  });

  it("clears the modules for another model and restores them from this session's files", async () => {
    const { p, log } = await ready();
    await p.load(model, [{ file: moduleFile("hat", "head"), attach: true }]);
    await p.load(other);
    expect(log).toContain("unload hat");
    expect(p.modules).toEqual([]);
    expect(p.restoreOffer).toBeNull();
    expect(p.dressUpRemembered).toBe(false);

    // 戻ると尋ねる。場所の無いモジュールでも、このセッションで読んだ中身から戻せる (readFile は失敗する)。
    await p.load(model);
    expect(p.restoreOffer?.attached).toEqual(["hat"]);
    expect(p.dressUpRemembered).toBe(true);
    await p.restoreDressUp();
    expect(p.dressUpError).toBeNull();
    expect(moduleRows(p)).toEqual([["hat", true]]);
    // other には何も覚えていない。
    await p.load(other);
    expect(p.restoreOffer).toBeNull();
    expect(p.dressUpRemembered).toBe(false);
  });

  it("forgets the offer and remembers the new set when the user changes the dress-up", async () => {
    await rememberHat();
    const { p } = await ready();
    await p.load(model);
    expect(p.restoreOffer).not.toBeNull();
    p.loadModule(scarf);
    expect(p.restoreOffer).toBeNull();
    p.detach();

    const { p: next } = await ready();
    await next.load(model);
    expect(next.restoreOffer?.attached).toEqual(["scarf"]);
  });

  it("keeps the shown model's modules and offer when another file fails to open", async () => {
    const { p, puppet } = await ready();
    await p.load(model, [
      { file: hat, attach: true },
      { file: scarf, attach: false },
    ]);
    expect(moduleRows(p)).toEqual([
      ["hat", true],
      ["scarf", false],
    ]);
    const good = puppet.load_bundle;
    puppet.load_bundle = fakePuppet({ failLoad: true }).puppet.load_bundle;
    await p.load(other);
    expect(p.error).toContain("b.ugoe");
    expect(p.modelName).toBe("a.ugoe");
    // 除いたモジュールは戻り、付けていたものも付け直す。どちらのモデルの組も変えない。
    expect(moduleRows(p)).toEqual([
      ["hat", true],
      ["scarf", false],
    ]);
    puppet.load_bundle = good;
    await p.load(other);
    expect(p.restoreOffer).toBeNull();
    await p.load(model);
    expect(p.restoreOffer).toEqual({
      modules: [
        { name: "hat", file: modelIdentity(hat) },
        { name: "scarf", file: modelIdentity(scarf) },
      ],
      attached: ["hat"],
    });

    // 尋ねている間に別のファイルが開けなくても、尋ねている組は残る (表示しているのは model のまま)。
    puppet.load_bundle = fakePuppet({ failLoad: true }).puppet.load_bundle;
    await p.load(other);
    await p.load({ name: "notes.txt", bytes: new Uint8Array([1, 2, 3]) });
    expect(p.restoreOffer?.attached).toEqual(["hat"]);
  });
});
