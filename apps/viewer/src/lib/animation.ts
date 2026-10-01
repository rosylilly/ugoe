// モーションと表情のパネル (T0720) のための、一覧の読み取りと再生の状態の計算。
// 一覧の形は WebPuppet の `motions_json()`・`expressions_json()`、再生の規則は docs/spec/motion.md
// (優先度・フェード・表情の set と add・フレーム内の適用順)。待機モーションとトラッキング中のモーションの優先 (T0721) の
// 決まりもここに置く。

/** モーションの優先度 (docs/spec/motion.md#モーションマネージャ)。WebPuppet の `start_motion` に渡す文字列。 */
export type MotionPriority = "idle" | "normal" | "force";

export const MOTION_PRIORITIES: readonly { value: MotionPriority; label: string; title: string }[] =
  [
    {
      value: "idle",
      label: "待機",
      title:
        "待機 (idle): 通常・強制のモーションの再生中は始まらない。ループする待機モーションをほかのモーションで置き換えられるように",
    },
    {
      value: "normal",
      label: "通常",
      title:
        "通常 (normal): 待機・通常のモーションを置き換える。強制のモーションの再生中は始まらない",
    },
    {
      value: "force",
      label: "強制",
      title: "強制 (force): いつでも始まり、再生中のモーションを置き換える",
    },
  ];

/** バンドルのモーション 1 つ。 */
export interface MotionInfo {
  name: string;
  /** 長さ (秒)。 */
  duration: number;
  looping: boolean;
  fadeIn: number;
  fadeOut: number;
  /** カーブが動かすパラメータの ID (重なりなし、カーブの順)。 */
  parameters: string[];
}

/** 表情の項目の合成の仕方 (docs/spec/motion.md#表情)。 */
export type ExpressionOp = "add" | "multiply" | "overwrite";

/** バンドルの表情 1 つ。 */
export interface ExpressionInfo {
  name: string;
  fadeIn: number;
  fadeOut: number;
  parameters: { id: string; op: string }[];
}

function num(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function strings(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((v): v is string => typeof v === "string") : [];
}

function records(json: string): Record<string, unknown>[] {
  const value: unknown = JSON.parse(json);
  if (!Array.isArray(value)) return [];
  return value.filter(
    (v): v is Record<string, unknown> =>
      typeof v === "object" && v !== null && typeof (v as { name?: unknown }).name === "string",
  );
}

/** `motions_json()` を読む。名前の無い項目は飛ばし、欠けた値は 0・偽にする。 */
export function parseMotions(json: string): MotionInfo[] {
  return records(json).map((m) => ({
    name: m.name as string,
    duration: num(m.duration),
    looping: m.looping === true,
    fadeIn: num(m.fade_in),
    fadeOut: num(m.fade_out),
    parameters: strings(m.parameters),
  }));
}

/** `expressions_json()` を読む。 */
export function parseExpressions(json: string): ExpressionInfo[] {
  return records(json).map((e) => ({
    name: e.name as string,
    fadeIn: num(e.fade_in),
    fadeOut: num(e.fade_out),
    parameters: (Array.isArray(e.parameters) ? e.parameters : []).flatMap((p: unknown) => {
      const { parameter, op } = (p ?? {}) as { parameter?: unknown; op?: unknown };
      return typeof parameter === "string"
        ? [{ id: parameter, op: typeof op === "string" ? op : "" }]
        : [];
    }),
  }));
}

/** 秒の表示 (`1.2 s`。小数は 2 桁まで)。 */
export function formatSeconds(s: number): string {
  return `${Number(s.toFixed(2))} s`;
}

/** モーションの長さ・ループ・フェードの短い説明 (`1.2 s · フェード 0.2 / 0.3 s`)。 */
export function describeMotion(m: MotionInfo): string {
  const parts = [m.looping ? `${formatSeconds(m.duration)} ループ` : formatSeconds(m.duration)];
  if (m.fadeIn > 0 || m.fadeOut > 0)
    parts.push(`フェード ${Number(m.fadeIn.toFixed(2))} / ${formatSeconds(m.fadeOut)}`);
  return parts.join(" · ");
}

/** トラッキングが書き込むパラメータと重なるもの (`ids` の順)。 */
export function trackedOverlap(ids: readonly string[], tracked: ReadonlySet<string>): string[] {
  return ids.filter((id) => tracked.has(id));
}

/**
 * トラッキング中のモーションの見え方 (docs/spec/motion.md#フレーム内の適用順: トラッキングはモーションの後に
 * 書き込み、トラッキングのパラメータではモーションの値を置き換える)。
 * - `none`: 重ならない (そのまま動く)
 * - `some`: 一部のパラメータはトラッキングが上書きする
 * - `all`: すべてのパラメータをトラッキングが上書きする (見た目は動かない。パーツの不透明度などのカーブは除く)
 */
export function motionTrackingEffect(
  m: MotionInfo,
  tracked: ReadonlySet<string>,
): { kind: "none" | "some" | "all"; overlap: string[] } {
  const overlap = trackedOverlap(m.parameters, tracked);
  const kind =
    overlap.length === 0 ? "none" : overlap.length === m.parameters.length ? "all" : "some";
  return { kind, overlap };
}

/** 開始しなかったとき (優先度が低い) の説明。 */
export function describeRejectedMotion(name: string, priority: MotionPriority): string {
  const label = MOTION_PRIORITIES.find((p) => p.value === priority)?.label ?? priority;
  return `「${name}」は、再生中のモーションより優先度 (${label}) が低いので始まりませんでした`;
}

/**
 * 待機モーションの既定 (T0721): 名前が `idle` のもの (大文字小文字を区別しない)、無ければ `idle` で始まるもの
 * (`Idle_01` など。ループするものを先に)。どれも無ければ `null` (自動では流さない)。
 */
export function defaultIdleMotion(motions: readonly MotionInfo[]): string | null {
  const exact = motions.find((m) => m.name.toLowerCase() === "idle");
  if (exact) return exact.name;
  const prefixed = motions.filter((m) => m.name.toLowerCase().startsWith("idle"));
  return (prefixed.find((m) => m.looping) ?? prefixed[0])?.name ?? null;
}

/**
 * 「トラッキング中もモーションを優先する」を WebPuppet に渡すか (T0721)。再生中のモーションが待機 (idle) の優先度なら
 * 渡さない (ループする待機モーションが、トラッキングの頭の向きなどをずっと上書きしないように)。
 */
export function motionOverTrackingActive(
  enabled: boolean,
  tracking: boolean,
  current: string | null,
  priority: MotionPriority,
): boolean {
  return enabled && tracking && current !== null && priority !== "idle";
}

/** 設定 (オン・オフ) の localStorage のキー (T0721)。 */
export const AUTO_IDLE_STORAGE_KEY = "ugoe-viewer.auto-idle";
export const MOTION_OVER_TRACKING_STORAGE_KEY = "ugoe-viewer.motion-over-tracking";

/** 覚えたオン・オフ。覚えていない・読めなければ `fallback`。 */
export function loadToggle(key: string, fallback: boolean, storage?: Storage): boolean {
  try {
    const value = (storage ?? globalThis.localStorage)?.getItem(key);
    return value === "on" ? true : value === "off" ? false : fallback;
  } catch {
    return fallback;
  }
}

/** オン・オフを覚える。覚えられなくても無視する (今回の設定は効く)。 */
export function saveToggle(key: string, value: boolean, storage?: Storage): void {
  try {
    (storage ?? globalThis.localStorage)?.setItem(key, value ? "on" : "off");
  } catch {
    // 覚えられないだけで、今回の設定は効く。
  }
}
