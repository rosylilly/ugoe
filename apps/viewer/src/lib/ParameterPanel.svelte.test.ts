// @vitest-environment happy-dom
// パラメータのパネル (ParameterPanel.svelte): 絞り込みで行を減らし、合うものが無ければそう書くこと、
// 長い名前の全体と ID を行の title に出すこと (T0725)。
import { setLocale } from "@ugoe/i18n";
import { groupParameters, type Player } from "@ugoe/player";
import { flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import ParameterPanel from "./ParameterPanel.svelte";

const cleanups: (() => void)[] = [];

beforeEach(() => setLocale("ja"));

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

const param = (id: string, name: string, group: string) => ({
  id,
  name,
  min: -1,
  max: 1,
  default: 0,
  group,
});

/** パネルが読む Player の状態だけを持つ偽物。 */
function setup() {
  const player = $state({
    status: "ready" as Player["status"],
    sections: groupParameters(
      {
        parameters: [
          param("AngleX", "", "Face"),
          param("EyeLOpen", "Left eye open (a very long name for the slider label)", "Eye"),
        ],
        groups: [
          { id: "Face", name: "Head" },
          { id: "Eye", name: "Eyes" },
        ],
      },
      "その他",
    ),
    values: {} as Record<string, number>,
    driven: new Set<string>(),
    setParameter() {},
    resetParameter() {},
    resetAllParameters() {},
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(ParameterPanel, {
    target,
    props: { player: player as unknown as Player },
  });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  return { target };
}

function filterTo(target: HTMLElement, text: string) {
  const input = target.querySelector<HTMLInputElement>('input[type="search"]');
  if (!input) throw new Error("no filter");
  input.value = text;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  flushSync();
}

const rows = (target: HTMLElement) =>
  [...target.querySelectorAll<HTMLElement>("[data-parameter]")].map((r) => r.dataset.parameter);

describe("ParameterPanel", () => {
  it("filters the rows by group name and says when nothing matches", () => {
    const { target } = setup();
    expect(rows(target)).toEqual(["AngleX", "EyeLOpen"]);
    filterTo(target, "eyes");
    expect(rows(target)).toEqual(["EyeLOpen"]);
    expect(target.querySelector('[role="status"]')).toBeNull();
    filterTo(target, "mouth");
    expect(rows(target)).toEqual([]);
    expect(target.querySelector('[role="status"]')?.textContent).toBe(
      "「mouth」に合うパラメータはありません",
    );
  });

  it("puts the whole name and the ID in the row title", () => {
    const { target } = setup();
    const titles = [...target.querySelectorAll<HTMLElement>("[data-parameter]")].map(
      (r) => r.title,
    );
    expect(titles).toEqual([
      "AngleX",
      "Left eye open (a very long name for the slider label) (EyeLOpen)",
    ]);
  });
});
