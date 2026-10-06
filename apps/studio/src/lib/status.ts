// 操作ウィンドウの「いまの状態」の印 (docs/studio.md#操作ウィンドウ、T0817)。トラッキング・口パク・ホットキー・出力ウィンドウ (T0818)・
// 負荷・操作 API・ブラウザソースの様子を、印の形 (StatusBadge.svelte) と短い文と Tooltip の説明にする。色だけで伝えないよう、形は tone ごとに変える。
import { formatNumber } from "@ugoe/i18n";
import type { ApiView } from "./api";
import { apiUrl } from "./api";
import type { OutputState, StatusMessage } from "./host";
import { t } from "./i18n";
import { backgroundLabel } from "./labels";
import { loadWarnings } from "./load";
import type { Background } from "./profile";

/** 印の種類。`ok` は動いている、`off` は切っている、`warning` は注意が要る、`danger` は動かせない。 */
export type StatusTone = "ok" | "off" | "warning" | "danger";

export type StatusId = "tracking" | "lipSync" | "hotkeys" | "output" | "load" | "api" | "source";

/** 右の列の設定のパネル (印を押すと開いて送る先。T0845)。 */
export type PanelId = "inputs" | "output" | "source" | "hotkeys" | "api" | "load";

/** 印ごとの、その設定を変えるパネル。 */
export const STATUS_PANELS: Record<StatusId, PanelId> = {
  tracking: "inputs",
  lipSync: "inputs",
  hotkeys: "hotkeys",
  output: "output",
  load: "load",
  api: "api",
  source: "source",
};

export interface StatusBadgeView {
  id: StatusId;
  /** 押すと開くパネル。 */
  panel: PanelId;
  tone: StatusTone;
  /** 印の横に出す短い文。 */
  label: string;
  /** Tooltip の説明 (どこで変えるか・数値の意味)。 */
  tooltip: string;
}

/** トラッキングの様子 (@ugoe/face-tracking の FaceTracker から写す)。 */
export interface TrackingState {
  status: "idle" | "loading" | "running" | "error";
  faceFound: boolean;
  fps: number;
  /** ホットキーなどで一時停止している (指揮役の様子)。 */
  paused: boolean;
}

export interface StatusInputs {
  tracking: TrackingState;
  lipSync: { running: boolean; paused: boolean };
  hotkeys: { count: number; problems: number; wayland: boolean };
  /** 出力ウィンドウ (まだ分からなければ `null`。出力の印を出さない)。 */
  output: OutputBadgeState | null;
  /** 最新の負荷 (まだ届いていなければ `null`。負荷の印を出さない)。 */
  load: StatusMessage | null;
  api: ApiView;
}

/** 出力ウィンドウの印に要るもの。 */
export interface OutputBadgeState {
  window: OutputState;
  /** 選んだ大きさより OS が小さくした。 */
  scaled: boolean;
  /** 配信しているシーンの背景 (配信していなければ `null`)。 */
  background: Background | null;
}

function badge(id: StatusId, tone: StatusTone, label: string, tooltip: string): StatusBadgeView {
  return { id, panel: STATUS_PANELS[id], tone, label, tooltip };
}

export function trackingBadge(state: TrackingState): StatusBadgeView {
  if (state.paused)
    return badge(
      "tracking",
      "warning",
      t("status.tracking.paused"),
      t("status.tracking.pausedTip"),
    );
  switch (state.status) {
    case "idle":
      return badge("tracking", "off", t("status.tracking.off"), t("status.tracking.offTip"));
    case "loading":
      return badge(
        "tracking",
        "off",
        t("status.tracking.loading"),
        t("status.tracking.loadingTip"),
      );
    case "error":
      return badge("tracking", "danger", t("status.tracking.error"), t("status.tracking.errorTip"));
    case "running":
      return state.faceFound
        ? badge(
            "tracking",
            "ok",
            t("status.tracking.face", { fps: state.fps }),
            t("status.tracking.faceTip"),
          )
        : badge("tracking", "warning", t("status.tracking.noFace"), t("status.tracking.noFaceTip"));
  }
}

export function lipSyncBadge(state: StatusInputs["lipSync"]): StatusBadgeView {
  if (state.paused)
    return badge("lipSync", "warning", t("status.lipSync.paused"), t("status.lipSync.pausedTip"));
  return state.running
    ? badge("lipSync", "ok", t("status.lipSync.on"), t("status.lipSync.onTip"))
    : badge("lipSync", "off", t("status.lipSync.off"), t("status.lipSync.offTip"));
}

export function hotkeysBadge(state: StatusInputs["hotkeys"]): StatusBadgeView {
  if (state.count === 0)
    return badge("hotkeys", "off", t("status.hotkeys.none"), t("status.hotkeys.noneTip"));
  if (state.problems > 0)
    return badge(
      "hotkeys",
      "warning",
      t("status.hotkeys.problems", { count: state.problems }),
      t("status.hotkeys.problemsTip", { count: state.problems }),
    );
  if (state.wayland)
    return badge("hotkeys", "warning", t("status.hotkeys.wayland"), t("status.hotkeys.waylandTip"));
  return badge(
    "hotkeys",
    "ok",
    t("status.hotkeys.on", { count: state.count }),
    t("status.hotkeys.onTip", { count: state.count }),
  );
}

/**
 * 透過のシーンを出しているのに、出力ウィンドウの透過が画面で効かない (docs/studio.md#出力ウィンドウの透過、T0846)。
 * 透過の所が黒などで埋まって配信に乗る。単色・クロマキー・画像の背景は透過に頼らないので注意しない。
 */
export function transparencyLost(
  output: OutputState | null,
  background: Background | null,
): boolean {
  return output?.transparency === "unsupported" && background?.kind === "transparent";
}

export function outputBadge(state: OutputBadgeState): StatusBadgeView {
  const { window } = state;
  if (!window.visible)
    return badge("output", "off", t("status.output.hidden"), t("status.output.hiddenTip"));
  const label = t("status.output.on", { width: window.width, height: window.height });
  if (state.scaled) return badge("output", "warning", label, t("status.output.scaledTip"));
  if (transparencyLost(window, state.background))
    return badge("output", "warning", label, t("status.output.opaqueTip"));
  const background =
    state.background === null ? t("output.backgroundNoScene") : backgroundLabel(state.background);
  return badge("output", "ok", label, t("status.output.onTip", { background }));
}

export function loadBadge(status: StatusMessage): StatusBadgeView {
  const label = t("load.summary", {
    tick: t("load.ms", { value: formatNumber(status.tickMs, { maximumFractionDigits: 2 }) }),
    rate: t("load.rateValue", { value: status.tickRate }),
  });
  const warnings = loadWarnings(status).length;
  return warnings === 0
    ? badge("load", "ok", label, t("status.load.okTip", { count: status.outputs }))
    : badge("load", "warning", label, t("status.load.warningTip", { count: warnings }));
}

export function apiBadge(api: ApiView): StatusBadgeView {
  if (!api.enabled) return badge("api", "off", t("status.api.off"), t("status.api.offTip"));
  if (!api.listening)
    return badge("api", "danger", t("status.api.error"), t("status.api.errorTip"));
  if (api.pending.length > 0)
    return badge(
      "api",
      "warning",
      t("status.api.pending", { count: api.pending.length }),
      t("status.api.pendingTip", { count: api.pending.length }),
    );
  return badge(
    "api",
    "ok",
    t("status.api.on"),
    t("status.api.onTip", { url: apiUrl(api.port), count: api.connections }),
  );
}

export function sourceBadge(api: ApiView): StatusBadgeView {
  const source = api.source;
  if (!source.enabled)
    return badge("source", "off", t("status.source.off"), t("status.source.offTip"));
  if (!source.listening)
    return badge("source", "danger", t("status.source.error"), t("status.source.errorTip"));
  return badge(
    "source",
    "ok",
    t("status.source.on"),
    t("status.source.onTip", { count: source.connections }),
  );
}

/**
 * パネルを開いて送り、見出し (開閉のボタン) にフォーカスを移す (T0845)。パネルは `data-panel` の付いた要素で囲んだ Section。
 * 閉じていれば `open` で開き、開いた中身を描き終えてから送る。見つからなければ `false`。
 */
export async function revealPanel(
  root: ParentNode,
  panel: PanelId,
  open: (panel: PanelId) => Promise<void>,
): Promise<boolean> {
  await open(panel);
  const container = root.querySelector<HTMLElement>(`[data-panel="${panel}"]`);
  const heading = container?.querySelector<HTMLElement>(".ugoe-section__trigger");
  if (container == null || heading == null) return false;
  scrollToTop(container);
  // 送るのは上で済ませる (focus の既定の送りで位置が変わらないように)
  heading.focus({ preventScroll: true });
  return true;
}

/**
 * 要素の頭が、送れるいちばん近い祖先 (右の列) の頭に来るよう送る。scrollIntoView は送らない祖先 (body など) まで
 * 動かして画面がずれるので使わない。動きを減らす設定なら一度に送る。
 */
function scrollToTop(el: HTMLElement) {
  let box = el.parentElement;
  while (box !== null) {
    const overflow = getComputedStyle(box).overflowY;
    if (box.scrollHeight > box.clientHeight && (overflow === "auto" || overflow === "scroll"))
      break;
    box = box.parentElement;
  }
  if (box === null) return;
  const top = box.scrollTop + el.getBoundingClientRect().top - box.getBoundingClientRect().top;
  const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
  box.scrollTo({ top, behavior: reduce ? "auto" : "smooth" });
}

/** 「いまの状態」に並べる印 (配信中に気にする順)。 */
export function statusBadges(inputs: StatusInputs): StatusBadgeView[] {
  const out = [
    trackingBadge(inputs.tracking),
    lipSyncBadge(inputs.lipSync),
    hotkeysBadge(inputs.hotkeys),
  ];
  if (inputs.output !== null) out.push(outputBadge(inputs.output));
  if (inputs.load !== null) out.push(loadBadge(inputs.load));
  out.push(apiBadge(inputs.api), sourceBadge(inputs.api));
  return out;
}
