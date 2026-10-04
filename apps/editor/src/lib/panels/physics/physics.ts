// 物理のパネル (docs/editor.md#物理の編集) の、DOM に依存しない補助: 新しいリグ・衝突体の値、
// 粒子の鎖の編集、出力先の組み立てと表記、コマンドの組み立て。physics.test.ts で検査する。

import { t } from "../../i18n";
import type {
  ColliderShape,
  Command,
  Model,
  NormalizationRange,
  PhysicsBlend,
  PhysicsCollider,
  PhysicsOutput,
  PhysicsParticle,
  PhysicsRig,
  PhysicsSettings,
  PhysicsTarget,
  Vec2,
  VertexOwner,
} from "../../ipc";
import { labelOf } from "../../labels";

/** 物理の設定がないときに作られる設定 (ugoe_editor_core::physics_edit::default_settings と同じ)。 */
export const DEFAULT_SETTINGS: PhysicsSettings = {
  gravity: [0, 1],
  wind: [0, 0],
  fps: 60,
  rigs: [],
  colliders: [],
};

/** 新しい粒子の係数 (CreatePhysicsRig と同じ)。 */
export const DEFAULT_PARTICLE = { mobility: 0.9, delay: 0.9, acceleration: 1.5 } as const;

/** 新しいリグの区間の長さ (キャンバスのピクセル)。 */
export const DEFAULT_SEGMENT = 30;

const DEFAULT_RANGE: NormalizationRange = { min: -10, default: 0, max: 10 };

/** ID の形式 (`[A-Za-z0-9_]`) に直し、`existing` と重ならない ID にする (ugoe_editor_core::ids::unique_id と同じ規則)。 */
export function uniqueId(base: string, existing: readonly string[]): string {
  const sanitized = /[A-Za-z0-9]/.test(base) ? base.replace(/[^A-Za-z0-9_]/g, "_") : "Item";
  const used = new Set(existing);
  if (!used.has(sanitized)) return sanitized;
  for (let n = 2; ; n++) {
    const id = `${sanitized}_${n}`;
    if (!used.has(id)) return id;
  }
}

/** 動的物理の機能 (パラメータ以外の出力先・衝突体・重力の基準) を使うリグか (ugoe_physics と同じ判定)。 */
export function isDynamicRig(rig: PhysicsRig): boolean {
  return (
    (rig.frame ?? null) !== null ||
    rig.colliders.length > 0 ||
    rig.outputs.some((o) => o.target.type !== "parameter")
  );
}

/** 設定全体が動的物理を使うか (1 つでも動的物理のリグがあれば)。粒子はキャンバス座標で表示できる。 */
export function isDynamicSettings(settings: PhysicsSettings | null): boolean {
  return settings?.rigs.some(isDynamicRig) ?? false;
}

/** 空のリグ (根元と区間 1 つ、入力・出力なし)。出力を足すと動く。 */
export function newRig(settings: PhysicsSettings | null, name: string): PhysicsRig {
  const existing = settings?.rigs.map((r) => r.id) ?? [];
  const trimmed = name.trim() || "Rig";
  return {
    id: uniqueId(trimmed, existing),
    name: trimmed,
    inputs: [],
    outputs: [],
    particles: [rootParticle(), segmentParticle([0, DEFAULT_SEGMENT], DEFAULT_SEGMENT)],
    normalization: { position: { ...DEFAULT_RANGE }, angle: { ...DEFAULT_RANGE } },
    colliders: [],
  };
}

function rootParticle(): PhysicsParticle {
  return {
    position: [0, 0],
    mobility: 1,
    delay: 1,
    acceleration: DEFAULT_PARTICLE.acceleration,
    radius: 0,
    collision_radius: 0,
  };
}

function segmentParticle(position: Vec2, radius: number): PhysicsParticle {
  return { ...DEFAULT_PARTICLE, position, radius, collision_radius: 0 };
}

/**
 * 鎖の先に粒子を 1 つ足す。最後の区間と同じ向き・長さ (区間がなければ下向きに DEFAULT_SEGMENT)。
 * 係数は最後の粒子と同じ。
 */
export function appendParticle(particles: readonly PhysicsParticle[]): PhysicsParticle[] {
  const last = particles.at(-1);
  if (!last) return [rootParticle()];
  const prev = particles.at(-2);
  let dir: Vec2 = [0, 1];
  let length = last.radius > 0 ? last.radius : DEFAULT_SEGMENT;
  if (prev) {
    const d: Vec2 = [last.position[0] - prev.position[0], last.position[1] - prev.position[1]];
    const len = Math.hypot(d[0], d[1]);
    if (len > 1e-6) {
      dir = [d[0] / len, d[1] / len];
      length = len;
    }
  }
  const position: Vec2 = [last.position[0] + dir[0] * length, last.position[1] + dir[1] * length];
  const next =
    particles.length === 1
      ? segmentParticle(position, length)
      : { ...last, position, radius: length };
  return [...particles, next];
}

/** 粒子 `index` を消す (根元は消さない)。次の粒子の区間の長さは、つながる粒子どうしの距離にする。 */
export function removeParticle(
  particles: readonly PhysicsParticle[],
  index: number,
): PhysicsParticle[] {
  if (index <= 0 || index >= particles.length) return [...particles];
  const next = particles.filter((_, i) => i !== index);
  const after = next[index];
  const before = next[index - 1];
  if (after && before) {
    next[index] = {
      ...after,
      radius: Math.hypot(
        after.position[0] - before.position[0],
        after.position[1] - before.position[1],
      ),
    };
  }
  return next;
}

/** 粒子の数値のフィールド。 */
export type ParticleField =
  | "mobility"
  | "delay"
  | "acceleration"
  | "radius"
  | "collision_radius"
  | "x"
  | "y";

/** 入力欄の範囲 (`NumberField` の `min`・`max`)。端を持たない側は省く。 */
export interface FieldBounds {
  min?: number;
  max?: number;
}

/**
 * 粒子のフィールドの範囲。物理の検証 (docs/spec/model.md#物理の検証) と同じ: `mobility` は 0〜1、
 * `delay`・`acceleration`・`radius`・`collision_radius` は 0 以上。位置は有限値であればよい。
 */
export const PARTICLE_BOUNDS: Readonly<Record<ParticleField, FieldBounds>> = {
  mobility: { min: 0, max: 1 },
  delay: { min: 0 },
  acceleration: { min: 0 },
  radius: { min: 0 },
  collision_radius: { min: 0 },
  x: {},
  y: {},
};

/**
 * 正規化の範囲のフィールド `field` の入力欄の範囲。検証は `min <= default <= max` を求める
 * (docs/spec/model.md#物理の検証) ので、ほかの 2 つの今の値を端にする。
 */
export function normalizationBounds(
  range: NormalizationRange,
  field: keyof NormalizationRange,
): FieldBounds {
  switch (field) {
    case "min":
      return { max: range.default };
    case "default":
      return { min: range.min, max: range.max };
    case "max":
      return { min: range.default };
  }
}

/** 粒子 `index` のフィールドを変えた鎖 (位置は x・y で指定)。 */
export function setParticleField(
  particles: readonly PhysicsParticle[],
  index: number,
  field: ParticleField,
  value: number,
): PhysicsParticle[] {
  return particles.map((p, i) => {
    if (i !== index) return p;
    if (field === "x") return { ...p, position: [value, p.position[1]] };
    if (field === "y") return { ...p, position: [p.position[0], value] };
    return { ...p, [field]: value };
  });
}

/** 出力先の種類 (選択欄の値)。 */
export type TargetKind = "parameter" | "deformer_angle" | "deformer_offset" | "mesh" | "warp";

// 選択肢の名前は読むたびに t() を通す (言語の切り替えに追従する)。
export const TARGET_KINDS: { readonly kind: TargetKind; readonly label: string }[] = [
  {
    kind: "parameter",
    get label() {
      return t("physics.targetKinds.parameter");
    },
  },
  {
    kind: "deformer_angle",
    get label() {
      return t("physics.targetKinds.deformerAngle");
    },
  },
  {
    kind: "deformer_offset",
    get label() {
      return t("physics.targetKinds.deformerOffset");
    },
  },
  {
    kind: "mesh",
    get label() {
      return t("physics.targetKinds.mesh");
    },
  },
  {
    kind: "warp",
    get label() {
      return t("physics.targetKinds.warp");
    },
  },
];

/**
 * パラメータへの合成の方法 (docs/spec/physics.md#合成の方法)。`add` は物理の揺れ (静止時からのずれ) を
 * トラッキングなどの値に足す。
 */
export const BLENDS: {
  readonly blend: PhysicsBlend;
  readonly label: string;
  readonly title: string;
}[] = [
  {
    blend: "overwrite",
    get label() {
      return t("physics.blends.overwrite.label");
    },
    get title() {
      return t("physics.blends.overwrite.description");
    },
  },
  {
    blend: "add",
    get label() {
      return t("physics.blends.add.label");
    },
    get title() {
      return t("physics.blends.add.description");
    },
  },
];

/** 出力の合成の方法。省略は `overwrite`。 */
export function outputBlend(output: PhysicsOutput): PhysicsBlend {
  return output.blend ?? "overwrite";
}

/** 合成の方法を選べる出力か (パラメータへの出力だけ。ほかの出力先は常に加算量として足す)。 */
export function hasBlend(output: PhysicsOutput): boolean {
  return output.target.type === "parameter";
}

export function targetKind(target: PhysicsTarget): TargetKind {
  switch (target.type) {
    case "vertices":
      return "mesh" in target.owner ? "mesh" : "warp";
    default:
      return target.type;
  }
}

/** 出力先の要素の ID。 */
export function targetRef(target: PhysicsTarget): string {
  switch (target.type) {
    case "parameter":
      return target.id;
    case "deformer_angle":
    case "deformer_offset":
      return target.deformer;
    case "vertices":
      return ownerId(target.owner);
  }
}

function ownerId(owner: VertexOwner): string {
  return "mesh" in owner ? owner.mesh : owner.warp;
}

/** 頂点 (制御点) の数。見つからなければ 0。 */
export function vertexCount(model: Model, owner: VertexOwner): number {
  if ("mesh" in owner) return model.meshes.find((m) => m.id === owner.mesh)?.uvs.length ?? 0;
  const d = model.deformers.find((x) => x.id === owner.warp);
  return d?.kind.type === "warp" ? (d.kind.rows + 1) * (d.kind.cols + 1) : 0;
}

/** 出力先の種類 `kind` の要素の候補 (回転デフォーマ・ワープ・メッシュ・パラメータ)。 */
export function targetCandidates(model: Model, kind: TargetKind): { id: string; name?: string }[] {
  switch (kind) {
    case "parameter":
      return model.parameters;
    case "deformer_angle":
    case "deformer_offset":
      return model.deformers.filter((d) => d.kind.type === "rotation");
    case "warp":
      return model.deformers.filter((d) => d.kind.type === "warp");
    case "mesh":
      return model.meshes;
  }
}

/**
 * 種類 `kind`・要素 `id` の出力先。頂点への出力の重みは、同じ要素の前の重みを引き継げれば引き継ぎ、
 * そうでなければすべて 1。
 */
export function makeTarget(
  model: Model,
  kind: TargetKind,
  id: string,
  previous?: PhysicsTarget,
): PhysicsTarget {
  switch (kind) {
    case "parameter":
      return { type: "parameter", id };
    case "deformer_angle":
    case "deformer_offset":
      return { type: kind, deformer: id };
    case "mesh":
    case "warp": {
      const owner: VertexOwner = kind === "mesh" ? { mesh: id } : { warp: id };
      const count = vertexCount(model, owner);
      const keep =
        previous?.type === "vertices" &&
        ownerId(previous.owner) === id &&
        targetKind(previous) === kind &&
        previous.weights.length === count;
      return {
        type: "vertices",
        owner,
        weights: keep ? [...previous.weights] : new Array<number>(count).fill(1),
      };
    }
  }
}

/** 出力の表記 (`Deformer angle: 前髪 (Hair1)`)。 */
export function describeTarget(model: Model | null, target: PhysicsTarget): string {
  const kind = TARGET_KINDS.find((k) => k.kind === targetKind(target))?.label ?? target.type;
  const id = targetRef(target);
  const items =
    target.type === "parameter"
      ? model?.parameters
      : target.type === "vertices" && "mesh" in target.owner
        ? model?.meshes
        : model?.deformers;
  return `${kind}: ${labelOf(items, id)}`;
}

/** 新しい出力: 最初の候補の要素へ、粒子 `particle` の角度を 100% で。候補がなければ null。 */
export function newOutput(model: Model, kind: TargetKind, particle: number): PhysicsOutput | null {
  const first = targetCandidates(model, kind)[0];
  if (!first) return null;
  return {
    target: makeTarget(model, kind, first.id),
    particle: Math.max(1, particle),
    source: kind === "mesh" || kind === "warp" || kind === "deformer_offset" ? "x" : "angle",
    scale: 1,
    weight: 100,
    reflect: false,
  };
}

/** 衝突体の既定の半径 (キャンバスのピクセル)。 */
export const DEFAULT_COLLIDER_RADIUS = 20;

/**
 * 新しい衝突体。`center` は取り付け先の局所座標 (取り付けなしならキャンバス座標)。
 * カプセルは `center` を中心に横向き (取り付け先の局所座標で幅 `2 × radius`)。
 */
export function newCollider(
  settings: PhysicsSettings | null,
  kind: "circle" | "capsule",
  attach: string | null,
  center: Vec2,
  radius = DEFAULT_COLLIDER_RADIUS,
): PhysicsCollider {
  const existing = settings?.colliders.map((c) => c.id) ?? [];
  const base = kind === "circle" ? "Circle" : "Capsule";
  const id = uniqueId(base, existing);
  const shape: ColliderShape =
    kind === "circle"
      ? { type: "circle", center, radius }
      : {
          type: "capsule",
          a: [center[0] - radius, center[1]],
          b: [center[0] + radius, center[1]],
          radius,
        };
  return { id, name: id, ...(attach ? { attach } : {}), shape };
}

/** 衝突体の形状の半径を変える。 */
export function withRadius(shape: ColliderShape, radius: number): ColliderShape {
  return { ...shape, radius };
}

/** 数として使える値か (入力欄の値が空・NaN なら null)。 */
export function finiteValue(value: number | null | undefined): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

// ---- コマンド ----

/**
 * 物理のパネルがコマンドを送る関数 (`EditorDocument.send`)。`build` は前の編集を読み直した後、送る直前の
 * 物理の設定で呼ぶので、リスト・ベクトル全体を置き換えるコマンドもそこから組み立てる (前の編集を戻さない)。
 * null を返したら送らない。適用できたら true。
 */
export type SendPhysics = (
  build: (settings: PhysicsSettings | null) => Command | null,
) => Promise<boolean>;

/** ID のリグ (無ければ null。送る直前の設定から探す)。 */
export function rigOf(settings: PhysicsSettings | null, id: string): PhysicsRig | null {
  return settings?.rigs.find((r) => r.id === id) ?? null;
}

/** ID の衝突体 (無ければ null。送る直前の設定から探す)。 */
export function colliderOf(settings: PhysicsSettings | null, id: string): PhysicsCollider | null {
  return settings?.colliders.find((c) => c.id === id) ?? null;
}

/** 世界の設定の欄 (重力・風の成分と fps)。 */
export type WorldField = "gx" | "gy" | "wx" | "wy" | "fps";

/** 世界の設定の欄を 1 つ変えるコマンド (ほかの成分は `settings` の値のまま)。 */
export function worldFieldCommand(
  settings: PhysicsSettings | null,
  field: WorldField,
  value: number,
): Command {
  const s = settings ?? DEFAULT_SETTINGS;
  const [gx, gy] = s.gravity;
  const [wx, wy] = s.wind;
  const patch =
    field === "fps"
      ? { fps: value }
      : field === "gx" || field === "gy"
        ? { gravity: (field === "gx" ? [value, gy] : [gx, value]) as [number, number] }
        : { wind: (field === "wx" ? [value, wy] : [wx, value]) as [number, number] };
  return worldCommand(settings, patch);
}

export function worldCommand(
  settings: PhysicsSettings | null,
  patch: Partial<PhysicsSettings>,
): Command {
  const s = settings ?? DEFAULT_SETTINGS;
  return {
    op: "set_physics_world",
    gravity: patch.gravity ?? s.gravity,
    wind: patch.wind ?? s.wind,
    fps: patch.fps ?? s.fps,
  };
}

/** 回転デフォーマの鎖から作るリグ (根元から順)。 */
export function chainCommand(deformers: readonly string[], name = ""): Command {
  return { op: "create_physics_rig", name, deformers: [...deformers] };
}

/** 選択されたデフォーマのうち回転デフォーマを、親から子の順 (祖先が先) に並べる。 */
export function chainOrder(model: Model, ids: readonly string[]): string[] {
  const rotations = ids.filter((id) =>
    model.deformers.some((d) => d.id === id && d.kind.type === "rotation"),
  );
  const depth = (id: string): number => {
    let n = 0;
    let current = model.deformers.find((d) => d.id === id);
    const seen = new Set<string>();
    while (current?.parent && !seen.has(current.id)) {
      seen.add(current.id);
      n += 1;
      const parent = current.parent;
      current = model.deformers.find((d) => d.id === parent);
    }
    return n;
  };
  return rotations
    .map((id, i) => ({ id, depth: depth(id), i }))
    .sort((a, b) => a.depth - b.depth || a.i - b.i)
    .map((x) => x.id);
}

/**
 * リグを 1 つ上 (`delta = -1`) か下 (`delta = 1`) へ動かすコマンド (reorder_physics_rig)。端なら null。
 * リグは上から順に処理され、前のリグの出力を後ろのリグが同じステップで読む (docs/spec/physics.md#リグの順序と中継)。
 */
export function moveRigCommand(
  settings: PhysicsSettings | null,
  id: string,
  delta: -1 | 1,
): Command | null {
  const ids = settings?.rigs.map((r) => r.id) ?? [];
  const i = ids.indexOf(id);
  const to = i + delta;
  if (i < 0 || to < 0 || to >= ids.length) return null;
  // 上へは 1 つ上のリグの前、下へは 2 つ下のリグの前 (なければ末尾)。
  const before = (delta < 0 ? ids[to] : ids[to + 1]) ?? null;
  return { op: "reorder_physics_rig", id, before };
}

/**
 * リグ `index` の入力のうち、前のリグがパラメータに出力するパラメータ (中継) を読むものの ID
 * (ugoe_physics の判定と同じ)。
 */
export function relayInputs(settings: PhysicsSettings, index: number): string[] {
  const written = new Set(
    settings.rigs
      .slice(0, Math.max(index, 0))
      .flatMap((r) => r.outputs)
      .flatMap((o) => (o.target.type === "parameter" ? [o.target.id] : [])),
  );
  const rig = settings.rigs[index];
  return rig ? [...new Set(rig.inputs.map((i) => i.parameter).filter((p) => written.has(p)))] : [];
}
