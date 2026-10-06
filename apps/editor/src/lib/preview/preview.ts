// プレビューモード (docs/editor.md#プレビューモード) の、DOM と wasm に依存しない部分 (preview.test.ts)。
// スライダーのグループ分けはビューア (packages/player/src/parameters.ts) と同じ規則。
import type { PoseGroupInfo } from "@ugoe/player";
import { t } from "../i18n";
import type { Model, Parameter, ParameterGroup, Pose, PoseGroup } from "../ipc";
import { showEntry, shownEntry } from "../panels/poses/poses";
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

/** プレビューの段の並び。 */
export const STAGES: readonly PuppetStage[] = [
  "physics",
  "eye_blink",
  "breath",
  "lip_sync",
  "pose",
];

/** 段の表示名。 */
export function stageLabel(stage: PuppetStage): string {
  switch (stage) {
    case "physics":
      return t("preview.stages.physics");
    case "eye_blink":
      return t("preview.stages.eyeBlink");
    case "breath":
      return t("preview.stages.breath");
    case "lip_sync":
      return t("preview.stages.lipSync");
    case "pose":
      return t("preview.stages.pose");
  }
}

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

/**
 * プレビューの WebPuppet での、編集中のモデルのパーツの ID (ポーズの要素を表示するとき。T0660)。モジュールの
 * プロジェクトで今の編集を本体に付けて比べている間は、今の編集を別の名前 (`editing`。dressUp.ts の `editingName`) で
 * 付けるので、モジュールの接頭辞 `<name>/` をその名前に付け替える。本体のパーツはそのまま。
 */
export function previewPart(
  part: string,
  module: { name: string; editing: string } | null,
): string {
  if (!module) return part;
  const prefix = `${module.name}/`;
  return part.startsWith(prefix) ? `${module.editing}/${part.slice(prefix.length)}` : part;
}

/** 着せ替えで付けたモジュールのポーズのグループ (T0661)。パーツは WebPuppet の ID (`<モジュール>/<パーツ>`)。 */
export interface ModulePoseGroup {
  /** グループのパーツの接頭辞 (付けたモジュールの名前)。 */
  module: string;
  group: PoseGroup;
}

/**
 * WebPuppet の合成したポーズ (`pose_json`) のうち、編集中のモデル (`own`) に無いグループ: 着せ替えで付けた
 * モジュールのグループ。編集中のモデルのグループは、パーツを `previewPart` で WebPuppet の ID にして見分ける
 * (要素を 1 つでも共有すれば同じグループ)。`composed` が読めなければ空。
 */
export function modulePoseGroups(
  composed: string | undefined | null,
  own: readonly PoseGroup[],
  module: { name: string; editing: string } | null,
): ModulePoseGroup[] {
  let pose: Pose;
  try {
    pose = composed ? (JSON.parse(composed) as Pose) : { fade_duration: 0, groups: [] };
  } catch {
    return [];
  }
  const ownParts = new Set(own.flatMap((g) => g.entries.map((e) => previewPart(e.part, module))));
  return (pose.groups ?? []).flatMap((group) => {
    if (group.entries.length === 0) return [];
    if (group.entries.some((e) => ownParts.has(e.part))) return [];
    const first = group.entries[0]?.part ?? "";
    const slash = first.indexOf("/");
    return [{ module: slash < 0 ? "" : first.slice(0, slash), group }];
  });
}

/**
 * WebPuppet の表示中のポーズの要素 (`pose_visible_json`: グループごとのパーツか `null`。T0478) のパーツの集まり。
 * 読めないもの (`undefined`・形の違うもの) は `null`。
 */
export function visiblePoseParts(json: string | undefined | null): Set<string> | null {
  if (!json) return null;
  let value: unknown;
  try {
    value = JSON.parse(json);
  } catch {
    return null;
  }
  if (!Array.isArray(value)) return null;
  return new Set(value.filter((p): p is string => typeof p === "string"));
}

/**
 * 表示する要素の一覧 `shown` を、WebPuppet の表示中の要素 `visible` に合わせる (モーションがパーツの不透明度のカーブで
 * 要素を切り替えたとき、欄を追従させる。T0736)。`toPuppet` は要素のパーツを WebPuppet の ID にするもの (`previewPart`)。
 * 表示中の要素が無いグループは前のまま。何も変わらなければ `shown` そのものを返す。
 */
export function followShown(
  shown: readonly string[],
  groups: readonly PoseGroup[],
  visible: ReadonlySet<string>,
  toPuppet: (part: string) => string = (part) => part,
): readonly string[] {
  let next = shown;
  for (const group of groups) {
    const entry = group.entries.find((e) => visible.has(toPuppet(e.part)));
    if (entry === undefined || group.entries[shownEntry(group, next)]?.part === entry.part)
      continue;
    next = showEntry(next, group, entry.part);
  }
  return next;
}

/**
 * プレビューのポーズの欄 (@ugoe/player の PosePicker) のグループ: 編集中のモデルのグループ (`module: null`) の後に、
 * 着せ替えで付けたモジュールのグループ。`shown` はグループごとに表示している要素 (`groups` と同じ順)。
 */
export function previewPoseGroups(
  own: readonly PoseGroup[],
  modules: readonly ModulePoseGroup[],
  ownShown: readonly string[],
  moduleShown: readonly string[],
): { groups: PoseGroupInfo[]; shown: string[] } {
  const parts = (group: PoseGroup) => group.entries.map((e) => e.part);
  const current = (group: PoseGroup, shown: readonly string[]) =>
    group.entries[shownEntry(group, shown)]?.part ?? "";
  return {
    groups: [
      ...own.map((group) => ({ parts: parts(group), module: null })),
      ...modules.map(({ module, group }) => ({ parts: parts(group), module })),
    ],
    shown: [
      ...own.map((group) => current(group, ownShown)),
      ...modules.map(({ group }) => current(group, moduleShown)),
    ],
  };
}

/** ポーズの欄のグループの見出し (`Group 1`・`LongHair group 1`。`n` は本体とモジュールごとに数えた番号)。 */
export function previewPoseGroupLabel(group: PoseGroupInfo, n: number): string {
  if (group.module === null) return t("preview.pose.group", { n });
  return group.module
    ? t("preview.pose.moduleGroup", { module: group.module, n })
    : t("preview.pose.moduleGroupUnnamed", { n });
}
