// @vitest-environment happy-dom
// プレビューの光源の行: 色の値とスライダーの名前は見せずに、読み上げの名前には使う (docs/ui.md#そのほか。T1657)。
import { describe, expect, it, vi } from "vitest";
import { t } from "../../i18n";
import { render } from "../../viewport/tools/test/harness.svelte";
import { LightingView } from "./lightingView.svelte";
import PreviewLightsSection from "./PreviewLightsSection.svelte";

vi.mock("../../ipc", () => ({ ipc: {} }));

/** `aria-labelledby` が指す要素の文字をつないだもの。 */
function labelledByText(el: Element): string {
  return (el.getAttribute("aria-labelledby") ?? "")
    .split(/\s+/)
    .map((id) => document.getElementById(id)?.textContent?.trim() ?? "")
    .join(" ");
}

describe("PreviewLightsSection", () => {
  it("names the colors and the intensities without showing the names", () => {
    const view = new LightingView();
    const { target } = render(PreviewLightsSection, { view, canvas: { width: 512, height: 512 } });

    const sliders = [...target.querySelectorAll('[role="slider"]')];
    expect(sliders.map(labelledByText)).toEqual([
      t("lighting.lights.ambient.intensity.aria"),
      t("lighting.lights.intensity.aria", { index: 1 }),
    ]);
    const fields = [...target.querySelectorAll(".ugoe-slider [role='spinbutton']")];
    expect(fields.map(labelledByText)).toEqual(sliders.map(labelledByText));
    for (const label of target.querySelectorAll(".ugoe-slider__label")) {
      expect(label.classList.contains("ugoe-visually-hidden")).toBe(true);
    }

    const colors = [...target.querySelectorAll(".ugoe-color-picker__trigger")];
    expect(colors.map((c) => c.getAttribute("aria-label"))).toEqual([
      `${t("lighting.lights.ambient.color.aria")} #ffffff`,
      `${t("lighting.lights.color.aria", { index: 1 })} #ffffff`,
    ]);
    for (const value of target.querySelectorAll(".ugoe-color-picker__value")) {
      expect(value.classList.contains("ugoe-visually-hidden")).toBe(true);
    }
  });
});
