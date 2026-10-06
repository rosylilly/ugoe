// @vitest-environment happy-dom
// ポーズのパネル (PosePanel.svelte): グループごとに要素を選ぶ欄を出し、選ぶと Player.showPose を呼ぶこと (T0735)。
// モーションが要素を切り替えると (Player が poseShown を表示中の要素に合わせる)、欄も追従すること (T0736)。
// 欄そのもの (見出し・表示名・5 個以上なら Select) のテストは @ugoe/player の PosePicker.svelte.test.ts (T0948)。
import { setLocale } from "@ugoe/i18n";
import type { Player, PoseGroupInfo } from "@ugoe/player";
import { flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import PosePanel from "./PosePanel.svelte";

const cleanups: (() => void)[] = [];

beforeEach(() => setLocale("ja"));

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

/** パネルが読む Player の状態と `showPose` だけを持つ偽物 (`showPose` は選んだ要素を `poseShown` に書く)。 */
function setup(groups: PoseGroupInfo[]) {
  const shown: string[] = [];
  const player = $state({
    poseGroups: groups,
    poseShown: groups.map((g) => g.parts[0] ?? ""),
    showPose(part: string): boolean {
      shown.push(part);
      const i = player.poseGroups.findIndex((g) => g.parts.includes(part));
      if (i < 0) return false;
      player.poseShown = player.poseShown.map((p, j) => (j === i ? part : p));
      return true;
    },
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(PosePanel, { target, props: { player: player as unknown as Player } });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  return { target, player, shown };
}

/** グループの欄の見出しと、選んでいる要素の表示名。 */
function rows(target: HTMLElement): [string, string][] {
  return [...target.querySelectorAll(".ugoe-pose-picker__group")].map((g) => [
    g.querySelector(".ugoe-segmented__label")?.textContent?.trim() ?? "",
    g.querySelector('[data-state="checked"]')?.textContent?.trim() ?? "",
  ]);
}

describe("PosePanel", () => {
  it("shows a choice for each group of the model and of attached modules", () => {
    const { target, player } = setup([
      { parts: ["earwear", "no_earwear"], module: null },
      { parts: ["Hat/On", "Hat/Off"], module: "Hat" },
    ]);
    expect(target.textContent).toContain("2 グループ");
    expect(rows(target)).toEqual([
      ["グループ 1", "earwear"],
      ["Hat のグループ 1", "On"],
    ]);

    // 着せ替えでモジュールを外すと、Player が読み直したグループに合わせる。
    player.poseGroups = [{ parts: ["earwear", "no_earwear"], module: null }];
    player.poseShown = ["no_earwear"];
    flushSync();
    expect(rows(target)).toEqual([["グループ 1", "no_earwear"]]);
  });

  it("calls showPose with the chosen part", () => {
    const { target, shown } = setup([{ parts: ["HandsDown", "Wave"], module: null }]);
    const wave = [...target.querySelectorAll<HTMLButtonElement>(".ugoe-segmented__item")].find(
      (b) => b.textContent?.trim() === "Wave",
    );
    wave?.click();
    flushSync();
    expect(shown).toEqual(["Wave"]);
    expect(rows(target)).toEqual([["グループ 1", "Wave"]]);
  });

  it("follows the entry a motion switches to", () => {
    const { target, player } = setup([
      { parts: ["HandsDown", "Wave"], module: null },
      { parts: ["A", "B", "C", "D", "E"], module: null },
    ]);
    // Player が表示中の要素 (pose_visible_json) に合わせて poseShown を書き換える。
    player.poseShown = ["Wave", "C"];
    flushSync();
    expect(rows(target)[0]).toEqual(["グループ 1", "Wave"]);
    expect(target.querySelector(".ugoe-select__value")?.textContent?.trim()).toBe("C");

    // モーションが終わった後に利用者が選び直せる。
    const down = [...target.querySelectorAll<HTMLButtonElement>(".ugoe-segmented__item")].find(
      (b) => b.textContent?.trim() === "HandsDown",
    );
    down?.click();
    flushSync();
    expect(rows(target)[0]).toEqual(["グループ 1", "HandsDown"]);
  });
});
