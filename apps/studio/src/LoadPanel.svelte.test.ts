// @vitest-environment happy-dom
// 負荷のパネル (LoadPanel.svelte) の GPU の優先度 (T0861): Windows でだけ選べ、当てられなかった理由をコードから訳して出すこと。
// 受け手ごとの行 (T0862): 描く機会の回数を出し、値が読み上げで区切られること。
// トラッキングの待ち (T0863): 届いていなければ「—」、届いていれば平均・最大・遅れた割合を区切りでつなぐこと。
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import LoadPanel from "./LoadPanel.svelte";
import type { GpuPriority, GpuView, RendererLoad, StatusMessage } from "./lib/host";
import { t } from "./lib/i18n";

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

const WINDOWS: GpuView = {
  preference: "auto",
  launched: "auto",
  graphicsSettings: true,
  priority: "normal",
  prioritySupported: true,
  priorityErrors: [],
};

function setup(gpu: GpuView, status: StatusMessage | null = null) {
  const chosen: GpuPriority[] = [];
  const props = $state({
    tickRate: 60 as const,
    renderScale: 1 as const,
    status,
    gpu,
    onTickRate: () => {},
    onRenderScale: () => {},
    onGpuPreference: () => {},
    onGpuPriority: (priority: GpuPriority) => chosen.push(priority),
    onGraphicsSettings: () => {},
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(LoadPanel, { target, props });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  return { target, props, chosen };
}

/** 文字が `text` の選択肢。 */
function option(target: HTMLElement, text: string): HTMLElement | undefined {
  return [...target.querySelectorAll<HTMLElement>("[role=radio]")].find(
    (el) => el.textContent?.trim() === text,
  );
}

describe("GPU の優先度", () => {
  it("Windows の外では出さない", () => {
    const { target } = setup({ ...WINDOWS, graphicsSettings: false, prioritySupported: false });
    expect(target.textContent).not.toContain(t("load.priority"));
    expect(option(target, t("load.priorityLow"))).toBeUndefined();
  });

  it("Windows では選べ、選んだ優先度を渡す", () => {
    const { target, chosen } = setup(WINDOWS);
    expect(target.textContent).toContain(t("load.priority"));
    const low = option(target, t("load.priorityLow"));
    expect(low).toBeDefined();
    low?.click();
    flushSync();
    expect(chosen).toEqual(["low"]);
  });

  it("当てられなかった理由をコードから訳して出す", () => {
    const { target, props } = setup({
      ...WINDOWS,
      priority: "low",
      priorityErrors: [
        {
          code: "gpu_priority.access_denied",
          args: { process: "msedgewebview2 (GPU, 42)" },
          message: "access to msedgewebview2 (GPU, 42) was denied when changing its GPU priority",
        },
      ],
    });
    expect(target.textContent).toContain(
      t("errors.gpu_priority.access_denied", { process: "msedgewebview2 (GPU, 42)" }),
    );
    // 当て直せたら (通知で様子が変わったら) 消える
    props.gpu = { ...props.gpu, priorityErrors: [] };
    flushSync();
    expect(target.textContent).not.toContain("msedgewebview2");
  });
});

function renderer(overrides: Partial<RendererLoad>): RendererLoad {
  return {
    label: "output",
    stale: false,
    targetFps: null,
    fps: 42,
    refreshFps: 144,
    renderMs: 1.5,
    renderMaxMs: 3,
    dropped: 0,
    backend: "webgpu",
    adapter: "Test GPU",
    ...overrides,
  };
}

function status(renderers: RendererLoad[]): StatusMessage {
  return {
    kind: "status",
    seq: 1,
    values: [],
    tickMs: 0.5,
    outputs: renderers.length,
    tickRate: 60,
    measuredRate: 60,
    maxIntervalMs: 17,
    trackingWait: null,
    renderers,
  };
}

/** 受け手の行の値 (`<dd>`) の文字。見た目では隠した区切りも含む (読み上げはこの文字を読む)。 */
function rowText(target: HTMLElement, name: string): string {
  const dt = [...target.querySelectorAll("dt")].find((el) => el.textContent === name);
  return dt?.nextElementSibling?.textContent ?? "";
}

describe("受け手ごとの行", () => {
  const sep = t("load.separator");
  const drawn = (fps: number) => t("load.drawn", { fps: t("load.perSecond", { value: fps }) });
  const refreshed = (fps: number, refresh: number) =>
    t("load.drawnRefresh", {
      fps: t("load.perSecond", { value: fps }),
      refresh: t("load.perSecond", { value: refresh }),
    });

  it("描いた回数と並べて描く機会の回数を出し、値と API・GPU の名前を区切りでつなぐ", () => {
    const { target } = setup(WINDOWS, status([renderer({})]));
    expect(rowText(target, t("load.outputWindow"))).toBe(
      [
        refreshed(42, 144),
        t("load.renderTime", {
          render: t("load.ms", { value: 1.5 }),
          max: t("load.ms", { value: 3 }),
        }),
        t("load.dropped", { count: 0 }),
        "WebGPU",
        "Test GPU",
      ].join(sep),
    );
    // 2 行目の前の区切りは見た目では隠し、読み上げにだけ置く
    const hidden = target.querySelector(".load__gpu .ugoe-visually-hidden");
    expect(hidden?.textContent).toBe(sep);
  });

  it("描く機会の回数の無い受け手 (仮想カメラ) と古い報告には描く機会の回数を出さない", () => {
    const { target } = setup(
      WINDOWS,
      status([
        renderer({
          label: "virtualCamera",
          targetFps: 30,
          fps: 30,
          refreshFps: null,
          backend: "vulkan",
        }),
        renderer({ label: "browserSource", stale: true }),
      ]),
    );
    const camera = rowText(target, t("load.virtualCamera"));
    expect(camera.startsWith(`${drawn(30)}${sep}`)).toBe(true);
    const source = rowText(target, t("load.browserSource"));
    expect(source).toBe([t("load.stale"), "WebGPU", "Test GPU"].join(sep));
    expect(target.textContent).not.toContain(t("load.perSecond", { value: 144 }));
  });

  it("API の名前が無ければ 2 行目を出さない", () => {
    const { target } = setup(WINDOWS, status([renderer({ backend: null })]));
    expect(target.querySelector(".load__gpu")).toBeNull();
    expect(rowText(target, t("load.outputWindow")).endsWith(t("load.dropped", { count: 0 }))).toBe(
      true,
    );
  });
});

describe("トラッキングの待ち", () => {
  const sep = t("load.separator");

  it("値が届いていなければ「—」を出し、読み上げには届いていないことを読ませる", () => {
    const { target } = setup(WINDOWS, status([]));
    expect(rowText(target, t("load.trackingWait"))).toBe(`—${t("load.trackingWaitNone")}`);
    const hidden = [...target.querySelectorAll("dd .ugoe-visually-hidden")].map(
      (el) => el.textContent,
    );
    expect(hidden).toContain(t("load.trackingWaitNone"));
  });

  it("平均・最大・刻みに遅れた割合を区切りでつなぎ、遅れても注意は出さない", () => {
    const { target } = setup(WINDOWS, {
      ...status([]),
      trackingWait: { meanMs: 1.234, maxMs: 15.5, late: 0.125 },
    });
    expect(rowText(target, t("load.trackingWait"))).toBe(
      [
        t("load.trackingWaitMean", { value: t("load.ms", { value: 1.23 }) }),
        t("load.trackingWaitMax", { value: t("load.ms", { value: 15.5 }) }),
        t("load.trackingWaitLate", { value: "12.5%" }),
      ].join(sep),
    );
    expect(target.querySelector(".ugoe-notice")).toBeNull();
  });
});
