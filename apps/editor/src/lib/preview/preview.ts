// プレビューモード (docs/editor.md#プレビューモード) の、DOM と wasm に依存しない部分 (preview.test.ts)。
// スライダーのグループ分けはビューア (apps/viewer/src/lib/parameters.ts) と同じ規則。
import type { Model, Parameter, ParameterGroup } from "../ipc";
import type { PuppetStage } from "../viewport/controller.svelte";

/** スライダーの一覧の 1 区切り。`id` が `null` のものはグループに属さないパラメータ。 */
export interface ParameterSection {
  id: string | null;
  name: string;
  parameters: Parameter[];
}

/**
 * グループの定義順に区切り、各区切りの中はモデルの定義順に並べる。空のグループは出さない。
 * グループの無いパラメータ (と、定義に無いグループを指すもの) は最後の区切りにまとめる。
 */
export function groupParameters(
  parameters: readonly Parameter[],
  groups: readonly ParameterGroup[],
  ungroupedName: string,
): ParameterSection[] {
  const sections = new Map<string, ParameterSection>(
    groups.map((g) => [g.id, { id: g.id, name: g.name || g.id, parameters: [] }]),
  );
  const rest: ParameterSection = { id: null, name: ungroupedName, parameters: [] };
  for (const p of parameters) {
    const section = p.group == null ? undefined : sections.get(p.group);
    (section ?? rest).parameters.push(p);
  }
  return [...sections.values(), rest].filter((s) => s.parameters.length > 0);
}

/** 名前か ID に `query` を含むパラメータだけを残す (大文字と小文字を区別しない)。空の区切りは消す。 */
export function filterSections(sections: ParameterSection[], query: string): ParameterSection[] {
  const q = query.trim().toLowerCase();
  if (q === "") return sections;
  return sections
    .map((s) => ({
      ...s,
      parameters: s.parameters.filter(
        (p) => p.id.toLowerCase().includes(q) || p.name.toLowerCase().includes(q),
      ),
    }))
    .filter((s) => s.parameters.length > 0);
}

/** スライダーの刻み。値域を 1000 等分した程度の、きりのよい大きさ。 */
export function sliderStep(p: Pick<Parameter, "min" | "max">): number {
  const range = p.max - p.min;
  if (!(range > 0)) return 0.001;
  return 10 ** Math.floor(Math.log10(range / 1000));
}

/** プレビューの段の並びと表示名。 */
export const STAGES: readonly { id: PuppetStage; label: string }[] = [
  { id: "physics", label: "Physics" },
  { id: "eye_blink", label: "Eye blink" },
  { id: "breath", label: "Breath" },
  { id: "lip_sync", label: "Lip sync" },
  { id: "pose", label: "Pose" },
];

/**
 * 最初の段の有効・無効。リップシンクは音量を与えると口のパラメータを上書きするので、
 * 口のスライダーを動かせるよう最初は止めておく。
 */
export function defaultStages(): Record<PuppetStage, boolean> {
  return { physics: true, eye_blink: true, breath: true, lip_sync: false, pose: true };
}

/**
 * 段ごとに、モデル (と物理の設定) にその段の設定があるか。無い段は有効にしても何もしない。
 * 物理はエディタの物理の設定 (`hasPhysics`) で決める。
 */
export function stageAvailability(
  model: Pick<Model, "effects" | "pose"> | null,
  hasPhysics: boolean,
): Record<PuppetStage, boolean> {
  return {
    physics: hasPhysics,
    eye_blink: (model?.effects.eye_blink.parameters.length ?? 0) > 0,
    breath: (model?.effects.breath.parameters.length ?? 0) > 0,
    lip_sync: (model?.effects.lip_sync.parameters.length ?? 0) > 0,
    pose: (model?.pose.groups.length ?? 0) > 0,
  };
}

/** 覚えておくモーションのイベントの数。 */
export const EVENT_HISTORY = 5;

/** 新しく発火したイベントを先頭に足し、`EVENT_HISTORY` 個までにする。 */
export function pushEvents(history: readonly string[], fired: readonly string[]): string[] {
  if (fired.length === 0) return history as string[];
  return [...[...fired].reverse(), ...history].slice(0, EVENT_HISTORY);
}
