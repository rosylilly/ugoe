// @vitest-environment happy-dom
// 仮想カメラ (VirtualCameraPanel.svelte): 設定はプロファイルに持ち、通知を待たずに続けて変えても変更が消えないこと (T0827)。
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type { StudioHost } from "./lib/host";
import { t } from "./lib/i18n";
import {
  DEFAULT_CAMERA_PROFILE,
  type VirtualCameraProfile,
  type VirtualCameraStatus,
} from "./lib/virtualCamera";
import VirtualCameraPanel from "./VirtualCameraPanel.svelte";

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

function setup() {
  const sent: VirtualCameraProfile[] = [];
  const finish: (() => void)[] = [];
  let started = 0;
  const host = {
    setVirtualCameraSettings: (settings: VirtualCameraProfile) => {
      sent.push(settings);
      return new Promise<void>((resolve) => finish.push(resolve));
    },
    startVirtualCamera: (...args: unknown[]) => {
      expect(args).toEqual([]);
      started++;
      return Promise.resolve();
    },
    stopVirtualCamera: () => Promise.resolve(),
  } as Partial<StudioHost> as StudioHost;
  const props = $state({
    host,
    status: { state: "stopped", unavailable: null } as VirtualCameraStatus | null,
    settings: DEFAULT_CAMERA_PROFILE,
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(VirtualCameraPanel, { target, props });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  /** バックエンドが送った設定を受け取り、通知で返してから送り終える (送っていないものが無くなるまで)。 */
  const notify = async () => {
    for (let resolve = finish.shift(); resolve !== undefined; resolve = finish.shift()) {
      const last = sent.at(-1);
      if (last !== undefined) props.settings = last;
      resolve();
      await new Promise((r) => setTimeout(r, 0));
    }
    flushSync();
  };
  return { target, props, sent, notify, started: () => started };
}

function rate(root: Element, fps: number): HTMLElement {
  const item = [...root.querySelectorAll<HTMLElement>(".ugoe-segmented__item")].find(
    (i) => i.getAttribute("data-value") === String(fps) || i.textContent?.includes(String(fps)),
  );
  if (item === undefined) throw new Error(`no rate ${fps}`);
  return item;
}

/** 色を選ぶ (Popover を開き、OS の色の選択に値を入れる)。 */
function color(root: Element, hex: string) {
  root.querySelector<HTMLElement>(".ugoe-color-picker__trigger")?.click();
  flushSync();
  const input = document.querySelector<HTMLInputElement>('input[type="color"]');
  if (input === null) throw new Error("no color input");
  input.value = hex;
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("VirtualCameraPanel", () => {
  it("keeps every change made before the profile notification arrives", async () => {
    const { target, props, sent, notify } = setup();
    rate(target, 60).click();
    flushSync();
    color(target, "#00ff00");
    flushSync();
    // 通知の前でも、両方の変更が見える。
    expect(rate(target, 60).getAttribute("data-state")).toBe("checked");
    expect(target.querySelector(".ugoe-color-picker__value")?.textContent).toBe("#00ff00");
    await notify();
    expect(sent).toHaveLength(2);
    expect(sent.at(-1)).toEqual({ ...DEFAULT_CAMERA_PROFILE, fps: 60, background: "#00ff00" });
    expect(props.settings).toEqual(sent.at(-1));
    expect(rate(target, 60).getAttribute("data-state")).toBe("checked");
  });

  it("shows the settings of a newly opened profile", () => {
    const { target, props } = setup();
    props.settings = { ...DEFAULT_CAMERA_PROFILE, fps: 60 };
    flushSync();
    expect(rate(target, 60).getAttribute("data-state")).toBe("checked");
  });

  it("starts the camera with the settings in the profile", async () => {
    const { target, started } = setup();
    target.querySelector<HTMLElement>('[role="switch"]')?.click();
    flushSync();
    await new Promise((r) => setTimeout(r, 0));
    expect(started()).toBe(1);
  });

  // 描画中の GPU のエラーは、OS のエラーでなく GPU の理由として出す (T0836)。
  it("tells a GPU error apart from an OS error", () => {
    const { target, props } = setup();
    props.status = {
      state: "failed",
      settings: { width: 1280, height: 720, fps: 30, background: [240, 240, 240] },
      code: "gpuError",
      detail: "device lost",
    };
    flushSync();
    const text = target.textContent ?? "";
    expect(t("virtualCamera.errors.gpuError")).not.toBe("virtualCamera.errors.gpuError");
    expect(text).toContain(t("virtualCamera.errors.gpuError"));
    expect(text).not.toContain(t("virtualCamera.errors.os"));
  });
});
