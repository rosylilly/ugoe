// @vitest-environment happy-dom
// 出力ウィンドウ (OutputPanel.svelte): 透過のシーンなのに透過が画面で効かないとき、「背景」の欄に注意と代わりの案を出す (T0846)。
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type { OutputState } from "./lib/host";
import { t } from "./lib/i18n";
import type { Background } from "./lib/profile";
import OutputPanel from "./OutputPanel.svelte";

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

const OUTPUT: OutputState = {
  visible: true,
  onTop: false,
  width: 1920,
  height: 1080,
  transparency: "unsupported",
};

function setup(output: OutputState | null, background: Background | null) {
  const props = $state({
    output,
    size: "1920x1080" as const,
    background,
    onvisible: () => {},
    ontop: () => {},
    onsize: () => {},
  });
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(OutputPanel, { target, props });
  flushSync();
  cleanups.push(() => {
    unmount(component);
    target.remove();
  });
  return { target, props };
}

/** 「背景」の欄の注意 (無ければ `null`)。 */
function opaqueNotice(target: HTMLElement): HTMLElement | null {
  return target.querySelector<HTMLElement>(".output-panel__facts .ugoe-notice");
}

describe("OutputPanel", () => {
  it("warns in the background row when a transparent scene is not composited", () => {
    const { target } = setup(OUTPUT, { kind: "transparent" });
    const notice = opaqueNotice(target);
    expect(notice).not.toBeNull();
    expect(notice?.dataset.tone).toBe("warning");
    expect(notice?.textContent).toContain(t("output.opaqueTitle"));
    expect(notice?.textContent).toContain(t("output.opaque"));
  });

  it("does not warn when transparency works or the scene does not rely on it", () => {
    const { target, props } = setup(
      { ...OUTPUT, transparency: "supported" },
      { kind: "transparent" },
    );
    expect(opaqueNotice(target)).toBeNull();
    props.output = { ...OUTPUT, transparency: "unknown" };
    flushSync();
    expect(opaqueNotice(target)).toBeNull();
    props.output = OUTPUT;
    props.background = { kind: "chroma", color: "#00ff00" };
    flushSync();
    expect(opaqueNotice(target)).toBeNull();
    // 透過のシーンに戻すと出る (通知で背景が変わったとき)。
    props.background = { kind: "transparent" };
    flushSync();
    expect(opaqueNotice(target)).not.toBeNull();
  });
});
