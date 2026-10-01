// モーションと表情のパネル (T0720) のための、一覧の読み取りと再生の状態の計算。
// 一覧の形は WebPuppet の `motions_json()`・`expressions_json()`、再生の規則は docs/spec/motion.md
// (優先度・フェード・表情の set と add・フレーム内の適用順)。

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

/**
 * 使っている表情の名前 (開始した順) の操作。WebPuppet は使っている表情を返さないので、操作から求める
 * (`ExpressionManager` の set・add・stop・stop_all と同じ規則。フェードアウト中のものは含めない)。
 */
export const activeExpressions = {
  /** `set`: ほかをすべて止めて `name` だけにする。 */
  set: (_active: readonly string[], name: string): string[] => [name],
  /** `add`: 重ねる。同じ名前を重ねても 1 つと数える (止めると同じ名前のものがすべて止まる)。 */
  add: (active: readonly string[], name: string): string[] =>
    active.includes(name) ? [...active] : [...active, name],
  /** `stop(name)`: 名前の一致するものを止める。 */
  stop: (active: readonly string[], name: string): string[] => active.filter((n) => n !== name),
} as const;

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
