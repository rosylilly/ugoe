// モデルのデータの型 (spec/model.md)。フィールド名はファイルのものと同じ。
// 既定値は復号 (decode.ts) で埋めてある。拡張できる列挙のうち、知らない値を残すものは "unknown"。

export type Vec2 = [number, number];

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

export interface Model {
  schema_version: number;
  meta: { name: string; canvas: { width: number; height: number; origin: Vec2 } };
  parameters: Parameter[];
  parameter_groups: ParameterGroup[];
  parts: Part[];
  deformers: Deformer[];
  meshes: Mesh[];
  glues: Glue[];
  textures: TextureRef[];
  hit_areas: HitArea[];
  pose: Pose;
  effects: Effects;
  /** アバタープロファイルの `level` だけ (無ければ null)。 */
  avatar_level: number | null;
  slots: Slot[];
  lighting: boolean;
}

export interface Parameter {
  id: string;
  name: string;
  min: number;
  max: number;
  default: number;
  kind: "normal" | "blend_shape" | "unknown";
  mirror: string | null;
  group: string | null;
  internal: boolean;
}

export interface ParameterGroup {
  id: string;
  name: string;
}

export interface Part {
  id: string;
  name: string;
  parent: string | null;
  keyforms: KeyformGrid<PartKeyform>;
}

export interface PartKeyform {
  opacity: number;
}

export interface KeyformBinding {
  parameter: string;
  keys: number[];
}

export type KeyformInference =
  | { method: "additive" }
  | {
      method: "depth_rotation";
      angle_scale: number;
      pivot: Vec2;
      pivot_depth: number;
      yaw: string | null;
      pitch: string | null;
    }
  | { method: "unknown" };

export interface MirrorSource {
  id: string;
  axis: number | null;
  tolerance: number | null;
}

export interface KeyformGrid<T> {
  bindings: KeyformBinding[];
  keyforms: T[];
  /** 知らない値はフォールバックの linear にしてある。 */
  interpolation: "linear" | "smooth";
  blend_shapes: KeyformGrid<T>[];
  origins: string[];
  inference: KeyformInference | null;
  mirror_source: MirrorSource | null;
}

export interface WarpKeyform {
  points: Vec2[];
  opacity: number;
}

export interface RotationKeyform {
  origin: Vec2;
  angle: number;
  scale: number;
  reflect_x: boolean;
  reflect_y: boolean;
  opacity: number;
}

export type DeformerKind =
  | {
      type: "warp";
      rows: number;
      cols: number;
      keyforms: KeyformGrid<WarpKeyform>;
      depth: number[] | null;
    }
  | { type: "rotation"; keyforms: KeyformGrid<RotationKeyform> }
  | { type: "unknown" };

export interface Deformer {
  id: string;
  name: string;
  part: string;
  parent: string | null;
  kind: DeformerKind;
  mirror: string | null;
  mirror_axis: number | null;
}

export interface MeshKeyform {
  positions: Vec2[];
  opacity: number;
  draw_order: number;
  multiply_color: Rgb;
  screen_color: Rgb;
  mask_strength: number;
}

export interface MaskRef {
  mesh: string;
  invert: boolean;
}

export interface Mesh {
  id: string;
  name: string;
  part: string;
  parent: string | null;
  texture: number;
  uvs: Vec2[];
  indices: number[];
  blend: "normal" | "additive" | "multiply";
  culling: boolean;
  masks: MaskRef[];
  mask_mode: "union" | "intersection";
  mask_only: boolean;
  keyforms: KeyformGrid<MeshKeyform>;
  mirror: string | null;
  mirror_axis: number | null;
  depth: number[] | null;
  lit: boolean;
}

export interface GluePair {
  a: number;
  b: number;
  weight: number;
}

export interface GlueKeyform {
  intensity: number;
}

export interface Glue {
  id: string;
  mesh_a: string;
  mesh_b: string;
  pairs: GluePair[];
  keyforms: KeyformGrid<GlueKeyform>;
}

export interface TextureRef {
  name: string;
  width: number;
  height: number;
  normal_map: string | null;
}

export interface HitArea {
  id: string;
  mesh: string;
}

export interface PoseEntry {
  part: string;
  links: string[];
}

export interface PoseGroup {
  entries: PoseEntry[];
}

export interface Pose {
  fade_duration: number;
  groups: PoseGroup[];
}

export interface EyeBlink {
  parameters: string[];
  interval: number;
  closing: number;
  closed: number;
  opening: number;
  seed: number;
}

export interface BreathParameter {
  parameter: string;
  offset: number;
  peak: number;
  cycle: number;
  weight: number;
}

export interface Effects {
  eye_blink: EyeBlink;
  breath: { parameters: BreathParameter[] };
  lip_sync: { parameters: string[] };
}

export interface DrawOrderRange {
  min: number;
  max: number;
}

export interface Slot {
  id: string;
  anchor: string | null;
  draw_order: DrawOrderRange;
  replaces: string[];
  parameters: string[];
  mask_sources: string[];
  part: string | null;
  colliders: string[];
  pose_parts: string[];
}

// ---- 物理 ----

export interface PhysicsSettings {
  gravity: Vec2;
  wind: Vec2;
  fps: number;
  rigs: PhysicsRig[];
  colliders: PhysicsCollider[];
}

export type PhysicsSource = "x" | "y" | "angle" | "unknown";

export interface PhysicsInput {
  parameter: string;
  source: PhysicsSource;
  weight: number;
  reflect: boolean;
}

export interface VertexOwner {
  kind: "mesh" | "warp";
  id: string;
}

export type PhysicsTarget =
  | { type: "parameter"; id: string }
  | { type: "deformer_angle" | "deformer_offset"; deformer: string }
  | { type: "vertices"; owner: VertexOwner; weights: number[] }
  | { type: "unknown" };

export interface PhysicsOutput {
  target: PhysicsTarget;
  particle: number;
  source: PhysicsSource;
  scale: number;
  weight: number;
  reflect: boolean;
  blend: "overwrite" | "add";
}

export interface PhysicsParticle {
  position: Vec2;
  mobility: number;
  delay: number;
  acceleration: number;
  radius: number;
  collision_radius: number;
}

export interface NormalizationRange {
  min: number;
  default: number;
  max: number;
}

export interface PhysicsRig {
  id: string;
  name: string;
  inputs: PhysicsInput[];
  outputs: PhysicsOutput[];
  particles: PhysicsParticle[];
  normalization: { position: NormalizationRange; angle: NormalizationRange };
  colliders: string[];
  frame: string | null;
}

export type ColliderShape =
  | { type: "circle"; center: Vec2; radius: number }
  | { type: "capsule"; a: Vec2; b: Vec2; radius: number }
  | { type: "unknown" };

export interface PhysicsCollider {
  id: string;
  name: string;
  attach: string | null;
  shape: ColliderShape;
}

// ---- モーションと表情 ----

export interface Point {
  t: number;
  v: number;
}

export type Segment =
  | { type: "start"; p: Point }
  | { type: "linear" | "stepped" | "inverse_stepped"; to: Point }
  | { type: "bezier"; c1: Point; c2: Point; to: Point };

export interface Curve {
  target: { kind: "parameter" | "part_opacity" | "effect_group"; id: string };
  segments: Segment[];
  fade_in: number | null;
  fade_out: number | null;
}

export interface MotionEvent {
  t: number;
  value: string;
}

export interface Motion {
  name: string;
  duration: number;
  looping: boolean;
  fade_in: number;
  fade_out: number;
  /** 知らない対象やセグメントを持つ曲線は除いてある。 */
  curves: Curve[];
  events: MotionEvent[];
}

export interface ExpressionParameter {
  parameter: string;
  value: number;
  op: "add" | "multiply" | "overwrite";
}

export interface Expression {
  name: string;
  fade_in: number;
  fade_out: number;
  /** 知らない演算の項目は除いてある。 */
  parameters: ExpressionParameter[];
}

// ---- モジュール ----

export interface Module {
  schema_version: number;
  name: string;
  slot: string;
  requires: { avatar_level: number | null; slots: string[]; parameters: string[] };
  parts: Part[];
  deformers: Deformer[];
  meshes: Mesh[];
  glues: Glue[];
  textures: TextureRef[];
  physics: { rigs: PhysicsRig[]; colliders: PhysicsCollider[] };
  pose: { groups: PoseGroup[]; links: PoseEntry[] };
}
