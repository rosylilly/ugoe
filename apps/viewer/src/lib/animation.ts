// モーションと表情のパネル (T0720) のための、一覧の読み取りと再生の状態の計算。
// 一覧の形は WebPuppet の `motions_json()`・`expressions_json()`、再生の規則は docs/spec/motion.md
// (優先度・フェード・表情の set と add・フレーム内の適用順)。待機モーションとトラッキング中のモーションの優先 (T0721) の
// 決まりもここに置く。モデルごとに覚える待機モーションの選択 (T0722) の入れ物は modelMemory.ts の `ModelMemory`。
import { ModelMemory } from "./modelMemory";

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

/** モデルごとに覚えた待機モーションの選択 (T0722)。`motion` が `null` なら「(なし)」を選んだ (自動では流さない)。 */
export interface IdleChoice {
  motion: string | null;
}

/** 待機モーションの選択の localStorage のキー。 */
export const IDLE_MOTION_STORAGE_KEY = "ugoe-viewer.idle-motion.models";
/** 待機モーションの選択を覚えるモデルの数の上限 (1 体で数十バイト)。 */
export const MAX_IDLE_MODELS = 50;

/** 覚えた選択を読む。形が違えば `null` (読み飛ばす)。 */
export function parseIdleChoice(value: unknown): IdleChoice | null {
  if (typeof value !== "object" || value === null) return null;
  const motion = (value as { motion?: unknown }).motion;
  return typeof motion === "string" || motion === null ? { motion } : null;
}

/** モデルごとの待機モーションの選択の入れ物。 */
export function idleMemory(storage?: Storage): ModelMemory<IdleChoice> {
  return new ModelMemory(IDLE_MOTION_STORAGE_KEY, parseIdleChoice, {
    limit: MAX_IDLE_MODELS,
    storage,
  });
}

/**
 * モデルを開いたときの待機モーション (T0722): 覚えた選択があり、そのモーションがモデルにあれば (「(なし)」も) それ。
 * 無ければ既定 (`defaultIdleMotion`)。覚えたモーションがモデルから消えていたら既定に戻す (覚えたものは消さない)。
 */
export function initialIdleMotion(
  motions: readonly MotionInfo[],
  remembered: IdleChoice | null,
): string | null {
  if (remembered !== null) {
    const name = remembered.motion;
    if (name === null || motions.some((m) => m.name === name)) return name;
  }
  return defaultIdleMotion(motions);
}

/** 覚えたものの一覧に出す、待機モーションの選択の説明。 */
export function describeIdleChoice(choice: IdleChoice): string {
  return choice.motion ?? "(なし)";
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

/** トラッキングが書き込むパラメータのまとまり (モーションの優先をまとめて切り替える。T0722)。 */
export type TrackingGroup = "head" | "body" | "eyes" | "brows" | "mouth" | "other";

export const TRACKING_GROUPS: readonly { value: TrackingGroup; label: string }[] = [
  { value: "head", label: "頭の向き" },
  { value: "body", label: "体・肩" },
  { value: "eyes", label: "目" },
  { value: "brows", label: "眉" },
  { value: "mouth", label: "口" },
  { value: "other", label: "その他" },
];

/**
 * パラメータ ID のまとまり。標準のパラメータ (docs/spec/avatar-profile.md#フェイストラッキングとの対応) の名前の
 * 先頭で分け、それ以外 (`avatar.tracking` の上書きの書き込み先など) は `other`。
 */
export function trackingGroup(id: string): TrackingGroup {
  if (/^Angle[XYZ]$/.test(id)) return "head";
  if (/^(Body|Shoulder)/.test(id)) return "body";
  if (/^Eye/.test(id)) return "eyes";
  if (/^Brow/.test(id)) return "brows";
  if (/^(Mouth|Jaw|Cheek|Tongue|Nose)/.test(id)) return "mouth";
  return "other";
}

/** `ids` をまとまりごとに分ける (`TRACKING_GROUPS` の順。空のまとまりは除く。中は `ids` の順)。 */
export function groupTrackingParameters(
  ids: readonly string[],
): { group: TrackingGroup; label: string; ids: string[] }[] {
  return TRACKING_GROUPS.map(({ value, label }) => ({
    group: value,
    label,
    ids: ids.filter((id) => trackingGroup(id) === value),
  })).filter((g) => g.ids.length > 0);
}

/**
 * 「トラッキング中もモーションを優先」でも、トラッキングのままにするパラメータの ID の localStorage のキー (T0722)。
 * 標準のパラメータの ID はモデルによらないので、モデルごとではなく 1 つだけ覚える。
 */
export const TRACKING_KEPT_STORAGE_KEY = "ugoe-viewer.motion-over-tracking.kept";

/** 覚えた「トラッキングのままにする」パラメータ。覚えていない・読めなければ空 (すべてモーションを優先)。 */
export function loadTrackingKept(storage?: Storage): string[] {
  try {
    const text = (storage ?? globalThis.localStorage)?.getItem(TRACKING_KEPT_STORAGE_KEY);
    if (text == null) return [];
    const value: unknown = JSON.parse(text);
    return [...new Set(strings(value))];
  } catch {
    return [];
  }
}

/** 「トラッキングのままにする」パラメータを覚える。覚えられなくても無視する。 */
export function saveTrackingKept(ids: readonly string[], storage?: Storage): void {
  try {
    (storage ?? globalThis.localStorage)?.setItem(TRACKING_KEPT_STORAGE_KEY, JSON.stringify(ids));
  } catch {
    // 覚えられないだけで、今回の設定は効く。
  }
}

/**
 * 再生中のモーションの値をトラッキングが置き換えるパラメータ (印に使う)。モーションの優先が効いていれば
 * トラッキングのままにするものだけ、効いていなければトラッキングが書き込むものすべて。
 */
export function trackingWins(
  tracked: ReadonlySet<string>,
  over: boolean,
  kept: readonly string[],
): ReadonlySet<string> {
  return over ? new Set(kept.filter((id) => tracked.has(id))) : tracked;
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
