// @vitest-environment happy-dom
// プレビューの着せ替えの節 (DressUpSection.svelte) が、GPU を失っている間と WebPuppet の作り直しの途中の付け外しを、すぐ行の
// チェックとスロットの選択に出し、待っていることを文字で添えること (T1666。docs/testing.md#エディタの部品のテスト)。
// WebPuppet と IPC は偽物、DressUp は本物。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import { choose, render, settle } from "../viewport/tools/test/harness.svelte";
import DressUpSection from "./DressUpSection.svelte";
import { DressUp } from "./dressUp.svelte";
import { fakePuppet } from "./test/fakeModulePuppet";

vi.mock("../ipc", () => ({
  ipc: {
    setPreviewModules: vi.fn(async () => ({ name: "p", path: null, modified: true })),
    comparePreviewModule: vi.fn(async () => []),
  },
}));

const SLOTS = { LongHair: "Hair", ShortHair: "Hair", Dress: "Outfit" };
const bytes = (name: string) => new TextEncoder().encode(name);

/** 3 つのモジュールを読み込み (ShortHair と Dress が付く)、節を描く。 */
async function draw() {
  let fake = fakePuppet(SLOTS);
  /** 作り直しの途中 (`host.puppet()` が `null`)。 */
  let gone = false;
  const dressUp = new DressUp({
    puppet: () => (gone ? null : fake.puppet),
    active: () => true,
    changed: () => {},
  });
  await dressUp.add(
    ["LongHair", "ShortHair", "Dress"].map((name) => ({
      path: `/work/${name}.ugoemod`,
      bytes: bytes(name),
    })),
  );
  const { target } = render(DressUpSection, {
    dressUp,
    slots: [
      { id: "Hair", name: "Hair" },
      { id: "Outfit", name: "Outfit" },
    ] as never,
  });
  await settle();
  return {
    dressUp,
    target,
    get fake() {
      return fake;
    },
    lose() {
      fake.state.lost = true;
    },
    drop() {
      fake.state.lost = true;
      gone = true;
    },
    /** 作り直しを終える (`broken` のスロットには付けられない)。 */
    async recreate(broken: string[] = []) {
      gone = false;
      fake = fakePuppet(SLOTS);
      for (const slot of broken) fake.broken.add(slot);
      dressUp.loaded(fake.puppet);
      await settle();
    },
  };
}

/** 一覧の行: 名前・チェック・待っている付け外しの文 (無ければ `null`)。 */
function rows(target: HTMLElement) {
  return [...target.querySelectorAll(".modules > li")].map((li) => {
    const box = li.querySelector<HTMLElement>("[role=checkbox]");
    const describedBy = box?.getAttribute("aria-describedby");
    const note = describedBy ? li.querySelector(`[id="${describedBy}"]`) : null;
    return [
      li.querySelector("label")?.textContent?.trim(),
      box?.getAttribute("aria-checked") === "true",
      note?.textContent?.trim() ?? null,
    ];
  });
}

/** スロットの行: スロット・選んでいるもの。 */
function slots(target: HTMLElement) {
  return [...target.querySelectorAll(".slot")].map((row) => [
    row.querySelector(".slot-id")?.textContent?.trim(),
    row.querySelector(".ugoe-select__value")?.textContent?.trim(),
  ]);
}

function checkbox(target: HTMLElement, name: string): HTMLElement {
  const li = [...target.querySelectorAll(".modules > li")].find(
    (l) => l.querySelector("label")?.textContent?.trim() === name,
  );
  const box = li?.querySelector<HTMLElement>("[role=checkbox]");
  if (!box) throw new Error(`no checkbox: ${name}`);
  return box;
}

function slotTrigger(target: HTMLElement, slot: string): HTMLElement {
  const row = [...target.querySelectorAll(".slot")].find(
    (r) => r.querySelector(".slot-id")?.textContent?.trim() === slot,
  );
  const trigger = row?.querySelector<HTMLElement>(".ugoe-select__trigger");
  if (!trigger) throw new Error(`no slot: ${slot}`);
  return trigger;
}

const ON = "Puts this on when the preview is back.";
const OFF = "Takes this off when the preview is back.";

describe("DressUpSection while waiting for the WebPuppet (T1666)", () => {
  for (const [when, cut] of [
    ["the GPU is lost", "lose"],
    ["the WebPuppet is being made again", "drop"],
  ] as const) {
    it(`shows a change made while ${when} at once, and keeps it after the recreation`, async () => {
      const t = await draw();
      expect(rows(t.target)).toEqual([
        ["LongHair", false, null],
        ["ShortHair", true, null],
        ["Dress", true, null],
      ]);
      t[cut]();
      checkbox(t.target, "Dress").click();
      flushSync();
      await choose(slotTrigger(t.target, "Hair"), "LongHair");
      await settle();
      expect(rows(t.target)).toEqual([
        ["LongHair", true, ON],
        ["ShortHair", false, OFF],
        ["Dress", false, OFF],
      ]);
      expect(slots(t.target)).toEqual([
        ["Hair", "LongHair"],
        ["Outfit", "(none)"],
      ]);

      await t.recreate();
      expect(t.fake.attached()).toEqual(["LongHair"]);
      expect(rows(t.target)).toEqual([
        ["LongHair", true, null],
        ["ShortHair", false, null],
        ["Dress", false, null],
      ]);
      expect(slots(t.target)).toEqual([
        ["Hair", "LongHair"],
        ["Outfit", "(none)"],
      ]);
    });
  }

  it("keeps the change and the note while the recreation has not finished", async () => {
    // 作り直しを諦めた (ビューポートが「作り直す」を出した) 間も、選んだものを出したまま待つ。
    const t = await draw();
    t.drop();
    checkbox(t.target, "Dress").click();
    flushSync();
    const again = fakePuppet(SLOTS);
    again.state.lost = true;
    t.dressUp.loaded(again.puppet);
    await settle();
    expect(rows(t.target)).toEqual([
      ["LongHair", false, null],
      ["ShortHair", true, null],
      ["Dress", false, OFF],
    ]);
  });

  it("unticks a module the new WebPuppet cannot put on, and tells why on its row", async () => {
    const t = await draw();
    t.lose();
    await choose(slotTrigger(t.target, "Hair"), "LongHair");
    await settle();
    expect(rows(t.target)[0]).toEqual(["LongHair", true, ON]);

    await t.recreate(["Hair"]);
    expect(rows(t.target)).toEqual([
      ["LongHair", false, null],
      ["ShortHair", false, null],
      ["Dress", true, null],
    ]);
    expect(slots(t.target)).toEqual([
      ["Hair", "(none)"],
      ["Outfit", "Dress"],
    ]);
    const alerts = [...t.target.querySelectorAll(".modules > li")][0]?.textContent ?? "";
    expect(alerts).toContain("LongHair was taken off");
  });
});
