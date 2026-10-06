// AI の提案の、コマンドの選び方 (一部だけの採用) と表示の補助 (docs/editor.md#ai-の提案)。
// 外したコマンドの番号の集合を `excluded` として持つ。あるコマンドを外すと、それが作る要素を使うコマンド
// (`ProposalCommand.requires` で後から指すもの) も外す。戻すと、それが使う要素を作るコマンドも戻す。
// バックエンドも同じ列を複製で確かめるので、ここの依存が足りなくても誤った採用にはならない (断られる)。

import { t } from "../i18n";
import type {
  Classification,
  CommandSection,
  NormalMapGroupChoice,
  NormalMapGroupView,
  ProposalCommand,
} from "../ipc";
import { type BulgeForm, bulgeForm, bulgeSettings } from "../panels/lighting/lighting";

/** `index` を外した集合。`index` を (直接・間接に) 使うコマンドも外す。 */
export function excludeCommand(
  commands: ProposalCommand[],
  excluded: ReadonlySet<number>,
  index: number,
): Set<number> {
  const next = new Set(excluded);
  const stack = [index];
  while (stack.length > 0) {
    const i = stack.pop() as number;
    if (next.has(i)) continue;
    next.add(i);
    commands.forEach((c, k) => {
      if (c.requires.includes(i) && !next.has(k)) stack.push(k);
    });
  }
  return next;
}

/** `index` を戻した集合。`index` が (直接・間接に) 使う要素を作るコマンドも戻す。 */
export function includeCommand(
  commands: ProposalCommand[],
  excluded: ReadonlySet<number>,
  index: number,
): Set<number> {
  const next = new Set(excluded);
  const stack = [index];
  while (stack.length > 0) {
    const i = stack.pop() as number;
    next.delete(i);
    for (const r of commands[i]?.requires ?? []) {
      if (next.has(r)) stack.push(r);
    }
  }
  return next;
}

/** チェックボックスの切り替え。 */
export function toggleCommand(
  commands: ProposalCommand[],
  excluded: ReadonlySet<number>,
  index: number,
): Set<number> {
  return excluded.has(index)
    ? includeCommand(commands, excluded, index)
    : excludeCommand(commands, excluded, index);
}

/** 同じ対象への続いたコマンドのまとまり (一覧はまとまりごとに出す)。 */
export interface CommandGroup {
  /** 対象 (`ProposalCommand.subject`)。なければ最初のコマンドの種類。 */
  label: string;
  indices: number[];
  /** 提案の区切り (`ProposalState.sections`) から作ったか。1 つだけでも名前を付けて出し、中のコマンドは対象 (`subject`) 付きで出す。 */
  section?: boolean;
  /**
   * リグの提案に添えた分類の直し (続いた `set_layer_correction`) のまとまりか。中のコマンドは対象 (レイヤーのパス) 付きで出す。
   * 直しはリグのコマンドが頼らない (`requires` に入らない) ので、まとめて外してもリグは残る。
   */
  labels?: boolean;
}

/** 分類の直しのコマンド (crates/ugoe-mcp/README.md のエージェントの分類を残す)。 */
const LAYER_CORRECTION = "set_layer_correction";

/**
 * 続いたコマンドを対象ごとにまとめる (順序は変えない)。2 つ以上続いた分類の直し (`set_layer_correction`) は、
 * 対象が違っても 1 つのまとまり (「分類を残す N 件」) にする。
 */
export function groupCommands(commands: ProposalCommand[]): CommandGroup[] {
  const groups: CommandGroup[] = [];
  let lastSubject: string | null | undefined;
  commands.forEach((c, i) => {
    const last = groups.at(-1);
    const correction = c.op === LAYER_CORRECTION;
    // 直しと直しはまとめ、直しとほかの種類は同じ対象でもまとめない
    const join =
      correction === (commands[i - 1]?.op === LAYER_CORRECTION) &&
      (correction || (c.subject !== null && c.subject === lastSubject));
    if (last && join) {
      last.indices.push(i);
      if (correction) last.labels = true;
    } else groups.push({ label: c.subject ?? c.op, indices: [i] });
    lastSubject = c.subject;
  });
  for (const g of groups) {
    if (g.labels) g.label = t("proposals.list.keepLabels", { count: g.indices.length });
  }
  return groups;
}

/** まとまりの選び方: すべて選んでいる・一部だけ・何も選んでいない。 */
export function groupState(
  group: CommandGroup,
  excluded: ReadonlySet<number>,
): "all" | "some" | "none" {
  const out = group.indices.filter((i) => excluded.has(i)).length;
  return out === 0 ? "all" : out === group.indices.length ? "none" : "some";
}

/** まとまりの切り替え: 1 つでも選んでいればすべて外し (依存するものも)、何も選んでいなければすべて戻す。 */
export function toggleGroup(
  commands: ProposalCommand[],
  excluded: ReadonlySet<number>,
  group: CommandGroup,
): Set<number> {
  const include = groupState(group, excluded) === "none";
  let next = new Set(excluded);
  for (const i of group.indices) {
    next = include ? includeCommand(commands, next, i) : excludeCommand(commands, next, i);
  }
  return next;
}

/** 集合を IPC に渡す並び (昇順)。 */
export function excludedList(excluded: ReadonlySet<number>): number[] {
  return [...excluded].sort((a, b) => a - b);
}

/**
 * コマンドの種類 (`ProposalCommand.op`、`ugoe_editor_core::Command` の `op`) ごとの、「採用で変わるもの」に出す利用者の言葉の
 * キー (数を取る形)。キーを組み立てずに並べる (docs/translation.md の「文言を足す・変える」)。コマンドを足したらここにも足す
 * (proposals.test.ts が `command.rs` の種類と照らし合わせる)。
 */
export const CHANGE_KINDS = {
  add_parameter: "proposals.changeKinds.addParameter",
  remove_parameter: "proposals.changeKinds.removeParameter",
  set_parameter_range: "proposals.changeKinds.setParameterRange",
  batch: "proposals.changeKinds.batch",
  rename_parameter: "proposals.changeKinds.renameParameter",
  set_parameter_group: "proposals.changeKinds.setParameterGroup",
  set_parameter_mirror: "proposals.changeKinds.setParameterMirror",
  add_part: "proposals.changeKinds.addPart",
  remove_part: "proposals.changeKinds.removePart",
  rename_part: "proposals.changeKinds.renamePart",
  set_part_parent: "proposals.changeKinds.setPartParent",
  set_part_keyforms: "proposals.changeKinds.setPartKeyforms",
  add_deformer: "proposals.changeKinds.addDeformer",
  remove_deformer: "proposals.changeKinds.removeDeformer",
  rename_deformer: "proposals.changeKinds.renameDeformer",
  set_deformer_parent: "proposals.changeKinds.setDeformerParent",
  set_deformer_part: "proposals.changeKinds.setDeformerPart",
  set_warp_keyforms: "proposals.changeKinds.setWarpKeyforms",
  set_rotation_keyforms: "proposals.changeKinds.setRotationKeyforms",
  add_mesh: "proposals.changeKinds.addMesh",
  remove_mesh: "proposals.changeKinds.removeMesh",
  rename_mesh: "proposals.changeKinds.renameMesh",
  set_mesh_properties: "proposals.changeKinds.setMeshProperties",
  set_mesh_parent: "proposals.changeKinds.setMeshParent",
  set_mesh_keyforms: "proposals.changeKinds.setMeshKeyforms",
  set_mesh_keyform: "proposals.changeKinds.setMeshKeyform",
  reorder_mesh: "proposals.changeKinds.reorderMesh",
  add_texture: "proposals.changeKinds.addTexture",
  set_texture_image: "proposals.changeKinds.setTextureImage",
  remove_texture: "proposals.changeKinds.removeTexture",
  set_canvas: "proposals.changeKinds.setCanvas",
  regenerate_mesh: "proposals.changeKinds.regenerateMesh",
  infer_keyforms: "proposals.changeKinds.inferKeyforms",
  mirror_keyforms: "proposals.changeKinds.mirrorKeyforms",
  mark_keyform_authored: "proposals.changeKinds.markKeyformAuthored",
  mark_keyform_derived: "proposals.changeKinds.markKeyformDerived",
  move_mesh_vertices: "proposals.changeKinds.moveMeshVertices",
  add_mesh_vertex: "proposals.changeKinds.addMeshVertex",
  remove_mesh_vertices: "proposals.changeKinds.removeMeshVertices",
  retriangulate_mesh: "proposals.changeKinds.retriangulateMesh",
  add_keyform_key: "proposals.changeKinds.addKeyformKey",
  remove_keyform_key: "proposals.changeKinds.removeKeyformKey",
  set_keyform_cell: "proposals.changeKinds.setKeyformCell",
  set_mesh_draw_order: "proposals.changeKinds.setMeshDrawOrder",
  create_warp_deformer: "proposals.changeKinds.createWarpDeformer",
  create_rotation_deformer: "proposals.changeKinds.createRotationDeformer",
  reparent_preserving_look: "proposals.changeKinds.reparentPreservingLook",
  set_keyform_inference: "proposals.changeKinds.setKeyformInference",
  set_mirror_pair: "proposals.changeKinds.setMirrorPair",
  set_depth: "proposals.changeKinds.setDepth",
  apply_depth_preset: "proposals.changeKinds.applyDepthPreset",
  move_rotation_origin: "proposals.changeKinds.moveRotationOrigin",
  unwrap_deformer: "proposals.changeKinds.unwrapDeformer",
  add_motion: "proposals.changeKinds.addMotion",
  remove_motion: "proposals.changeKinds.removeMotion",
  rename_motion: "proposals.changeKinds.renameMotion",
  set_motion_properties: "proposals.changeKinds.setMotionProperties",
  add_motion_curve: "proposals.changeKinds.addMotionCurve",
  remove_motion_curve: "proposals.changeKinds.removeMotionCurve",
  set_motion_curve_fade: "proposals.changeKinds.setMotionCurveFade",
  set_motion_key: "proposals.changeKinds.setMotionKey",
  move_motion_key: "proposals.changeKinds.moveMotionKey",
  remove_motion_key: "proposals.changeKinds.removeMotionKey",
  set_motion_segment: "proposals.changeKinds.setMotionSegment",
  set_motion_events: "proposals.changeKinds.setMotionEvents",
  add_expression: "proposals.changeKinds.addExpression",
  remove_expression: "proposals.changeKinds.removeExpression",
  rename_expression: "proposals.changeKinds.renameExpression",
  set_expression_properties: "proposals.changeKinds.setExpressionProperties",
  set_expression_parameters: "proposals.changeKinds.setExpressionParameters",
  set_physics_world: "proposals.changeKinds.setPhysicsWorld",
  add_physics_rig: "proposals.changeKinds.addPhysicsRig",
  create_physics_rig: "proposals.changeKinds.createPhysicsRig",
  remove_physics_rig: "proposals.changeKinds.removePhysicsRig",
  rename_physics_rig: "proposals.changeKinds.renamePhysicsRig",
  set_physics_inputs: "proposals.changeKinds.setPhysicsInputs",
  set_physics_outputs: "proposals.changeKinds.setPhysicsOutputs",
  set_physics_particles: "proposals.changeKinds.setPhysicsParticles",
  set_physics_normalization: "proposals.changeKinds.setPhysicsNormalization",
  set_physics_rig_colliders: "proposals.changeKinds.setPhysicsRigColliders",
  set_physics_frame: "proposals.changeKinds.setPhysicsFrame",
  add_physics_collider: "proposals.changeKinds.addPhysicsCollider",
  remove_physics_collider: "proposals.changeKinds.removePhysicsCollider",
  set_physics_collider: "proposals.changeKinds.setPhysicsCollider",
  set_keyform_interpolation: "proposals.changeKinds.setKeyformInterpolation",
  reorder_physics_rig: "proposals.changeKinds.reorderPhysicsRig",
  set_parameter_internal: "proposals.changeKinds.setParameterInternal",
  create_draw_clone: "proposals.changeKinds.createDrawClone",
  sync_draw_clone: "proposals.changeKinds.syncDrawClone",
  apply_physics_template: "proposals.changeKinds.applyPhysicsTemplate",
  set_avatar_profile: "proposals.changeKinds.setAvatarProfile",
  update_avatar_profile: "proposals.changeKinds.updateAvatarProfile",
  set_hit_areas: "proposals.changeKinds.setHitAreas",
  set_normal_map: "proposals.changeKinds.setNormalMap",
  set_lighting: "proposals.changeKinds.setLighting",
  set_effects: "proposals.changeKinds.setEffects",
  add_slot: "proposals.changeKinds.addSlot",
  set_slot: "proposals.changeKinds.setSlot",
  remove_slot: "proposals.changeKinds.removeSlot",
  set_module_settings: "proposals.changeKinds.setModuleSettings",
  set_pose: "proposals.changeKinds.setPose",
  create_glue: "proposals.changeKinds.createGlue",
  set_glue: "proposals.changeKinds.setGlue",
  remove_glue: "proposals.changeKinds.removeGlue",
  set_layer_correction: "proposals.changeKinds.setLayerCorrection",
  add_parameter_group: "proposals.changeKinds.addParameterGroup",
  remove_parameter_group: "proposals.changeKinds.removeParameterGroup",
  rename_parameter_group: "proposals.changeKinds.renameParameterGroup",
  reorder_parameter_group: "proposals.changeKinds.reorderParameterGroup",
} as const;

/** 「採用で変わるもの」の 1 つ: コマンドの種類と数。`label` は利用者の言葉で、カタログに無い種類なら `null` (種類名のまま出す)。 */
export interface Change {
  op: string;
  count: number;
  label: string | null;
}

/**
 * 採用すると変わるもの: 選んだ (外していない) コマンドの種類ごとの数 (最初に現れた順)。提案のカードの「採用で変わるもの」に出す。
 */
export function changeSummary(
  commands: ProposalCommand[],
  excluded: ReadonlySet<number>,
): Change[] {
  const counts = new Map<string, number>();
  commands.forEach((c, i) => {
    if (!excluded.has(i)) counts.set(c.op, (counts.get(c.op) ?? 0) + 1);
  });
  return [...counts].map(([op, count]) => ({ op, count, label: changeLabel(op, count) }));
}

/** 種類 `op` のコマンド `count` 個で変わるものを利用者の言葉で (`add 5 parameters`)。カタログに無い種類なら `null`。 */
export function changeLabel(op: string, count: number): string | null {
  if (!Object.hasOwn(CHANGE_KINDS, op)) return null;
  return t(CHANGE_KINDS[op as keyof typeof CHANGE_KINDS], { count });
}

/**
 * 一覧に出すコマンドの 1 行 (`add_parameter AngleX`)。分類の直しは残すラベルと左右も添える
 * (`set_layer_correction Face / Eye R → eye_white (right)`)。
 */
export function commandLabel(command: ProposalCommand): string {
  if (command.op === LAYER_CORRECTION) return `${command.op} ${correctionLabel(command)}`;
  return command.subject ? `${command.op} ${command.subject}` : command.op;
}

/**
 * 分類の直し (`set_layer_correction`) の中身: レイヤーのパスと、残すラベル・左右 (`Face / Eye R → eye_white (right)`)。
 * ラベルと左右はコマンドの値のまま (`ProposalCommand.correction`)。直しを消すコマンドは分類の答えに戻す旨を出す。
 */
export function correctionLabel(command: ProposalCommand): string {
  const kept = command.correction
    ? `${command.correction.label} (${command.correction.side})`
    : t("proposals.list.correctionCleared");
  return `${command.subject ?? ""} → ${kept}`;
}

/** 分類の要約 (レイヤー数・確かめたほうがよいレイヤー数・ラベルごとの数)。 */
export function classificationSummary(c: Classification): {
  layers: number;
  review: number;
  labels: [string, number][];
  /** AI が同じ部品の別の絵 (差分) とみなしたレイヤーの組 (レイヤーの名前。組の番号の順)。 */
  variants: string[][];
} {
  const counts = new Map<string, number>();
  const variants = new Map<number, string[]>();
  for (const l of c.layers) {
    counts.set(l.label, (counts.get(l.label) ?? 0) + 1);
    if (l.variant !== null) {
      const names = variants.get(l.variant) ?? [];
      names.push(l.path[l.path.length - 1] ?? "");
      variants.set(l.variant, names);
    }
  }
  return {
    layers: c.layers.length,
    review: c.layers.filter((l) => l.needs_review).length,
    labels: [...counts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])),
    variants: [...variants].sort((a, b) => a[0] - b[0]).map(([, names]) => names),
  };
}

/** 注文の文字数の上限 (`ugoe_ai::instructions::Instructions::MAX_CHARS`)。 */
export const MAX_INSTRUCTION_CHARS = 2000;

/** 注文の文字数 (Unicode のスカラー値の数。バックエンドの数え方と同じ)。前後の空白は数えない。 */
export function instructionChars(text: string): number {
  return [...text.trim()].length;
}

/**
 * AI に添える注文 (docs/editor.md#ai-への注文と作り直し)。AI を使わないとき・空のときは送らない (`null`)。
 * 長すぎるかどうかはバックエンドが確かめる。
 */
export function instructionsToSend(text: string, useAi: boolean): string | null {
  const trimmed = text.trim();
  return useAi && trimmed !== "" ? trimmed : null;
}

/** 区切り (法線マップの提案のまとまり。`ProposalState.sections`) を、一覧のまとまりにする。 */
export function sectionGroups(sections: CommandSection[]): CommandGroup[] {
  return sections.map((s) => ({
    label: surfaceGroupLabel(s.label),
    indices: [...s.indices],
    section: true,
  }));
}

// ---- 法線マップの提案 (docs/editor.md#法線マップの提案) ----

/** ラベルのないテクスチャのまとまりの名前 (`NormalMapGroupView.group` が null)。 */
export const UNLABELED = "unlabeled";

/** まとまりの画面の名前 (呼ぶたびに今の言語で引く)。 */
const SURFACE_GROUP_LABELS: Record<string, () => string> = {
  face: () => t("proposals.groups.face"),
  body: () => t("proposals.groups.body"),
  hair_front: () => t("proposals.groups.hairFront"),
  hair_side: () => t("proposals.groups.hairSide"),
  hair_back: () => t("proposals.groups.hairBack"),
  ear: () => t("proposals.groups.ear"),
  earwear: () => t("proposals.groups.earwear"),
  headwear: () => t("proposals.groups.headwear"),
  tail: () => t("proposals.groups.tail"),
  alone: () => t("proposals.groups.alone"),
  [UNLABELED]: () => t("proposals.groups.unlabeled"),
  lighting: () => t("proposals.groups.lighting"),
};

/** まとまり (`face` など) の画面の名前。知らない名前はそのまま。 */
export function surfaceGroupLabel(group: string): string {
  return SURFACE_GROUP_LABELS[group]?.() ?? group;
}

/** まとまりの鍵 (`group`、ラベルがなければ `unlabeled`)。 */
export function normalGroupKey(group: NormalMapGroupView): string {
  return group.group ?? UNLABELED;
}

/** まとまりごとの選び方: 含めるか・強さ・縁の丸み・ならし (長さは画面の単位。panels/lighting/lighting.ts の `BulgeForm`)。 */
export interface NormalMapPick extends BulgeForm {
  include: boolean;
}

/**
 * まとまりの一覧に合わせた選び方。前の選び方 (`previous`) にあるまとまりはそれを残し、新しいまとまりは含めてまとまりの既定の設定にする。
 */
export function normalMapPicks(
  groups: NormalMapGroupView[],
  previous: Readonly<Record<string, NormalMapPick>> = {},
): Record<string, NormalMapPick> {
  const picks: Record<string, NormalMapPick> = {};
  for (const g of groups) {
    const key = normalGroupKey(g);
    picks[key] = previous[key] ?? { include: true, ...bulgeForm(g) };
  }
  return picks;
}

/** IPC に渡す、含めるまとまりと設定 (長さはまとまりの格子の画素に直す)。 */
export function normalMapChoices(
  groups: NormalMapGroupView[],
  picks: Readonly<Record<string, NormalMapPick>>,
): NormalMapGroupChoice[] {
  return groups.flatMap((g) => {
    const pick = picks[normalGroupKey(g)];
    if (!pick?.include) return [];
    return [{ group: g.group, settings: bulgeSettings(pick, g) }];
  });
}

/** 提案で法線マップを作るテクスチャの数 (`replaceExisting` でなければ、法線マップを持つものを除く)。 */
export function normalMapTargetCount(
  groups: NormalMapGroupView[],
  picks: Readonly<Record<string, NormalMapPick>>,
  replaceExisting: boolean,
): number {
  return groups
    .filter((g) => picks[normalGroupKey(g)]?.include)
    .reduce((n, g) => n + g.textures.length - (replaceExisting ? 0 : g.with_normal_map.length), 0);
}
