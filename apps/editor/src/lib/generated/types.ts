// このファイルは生成物です。手で編集せず `just gen` で作り直してください (docs/editor.md#型の生成)。

export type AppInfo = { version: string, };

/**
 * [`ugoe_mesher::MeshOptions`] の JSON 表現。省略したフィールドは既定値になる。
 */
export type AutoMeshOptions = { 
/**
 * この値以上のアルファを中身とする (0..=255)。
 */
alpha_threshold: number, 
/**
 * 輪郭を外側に広げる距離 (px)。
 */
margin: number, 
/**
 * 輪郭上の頂点間隔の目安 (px)。
 */
edge_spacing: number, 
/**
 * 内部頂点の間隔の目安 (px)。0 なら内部点を置かない。
 */
interior_spacing: number, 
/**
 * 画素数がこれ未満の島を捨てる。
 */
min_island_area: number, };

/**
 * 合成モード。式は `docs/spec/rendering.md`。
 */
export type BlendMode = "normal" | "additive" | "multiply";

/**
 * 呼吸。各パラメータに `weight * (offset + peak * sin(2π t / cycle))` を加算する。
 */
export type Breath = { parameters: Array<BreathParameter>, };

export type BreathParameter = { parameter: string, 
/**
 * 波の中心値。
 */
offset: number, 
/**
 * 波の振幅。
 */
peak: number, 
/**
 * 周期 (秒)。0 より大きい。
 */
cycle: number, 
/**
 * 加算するときに掛ける重み。
 */
weight: number, };

/**
 * キャンバス。モデル座標系は「キャンバス左上原点・ピクセル単位・Y 下向き」。
 * 表示時の正規化 (高さ 2.0 の NDC 相当など) は描画側の責務。
 */
export type Canvas = { width: number, height: number, 
/**
 * 表示時の原点 (通常は足元中央など)。
 */
origin: Vec2, };

/**
 * デフォーマの子になれる要素。JSON では `{"kind": "mesh", "id": "EyeL"}` と書く
 * ([`KeyformTarget`] と同じ形)。
 */
export type ChildRef = { "kind": "mesh", "id": string } | { "kind": "deformer", "id": string };

/**
 * 編集操作。JSON で表現でき、AI エージェントのツール呼び出しにそのまま対応する。
 *
 * 追加時のルール (`docs/editor.md#コマンド`):
 * - 1 コマンド = ユーザーから見た 1 操作 (Undo の単位)。
 * - フィールドは ID で対象を指定する (インデックスは使わない)。
 * - 適用前検証を [`crate::Document::apply`] 内で行い、失敗時はモデルを一切変更しない。
 * - 新しいバリアントは末尾に追加する。
 * - TypeScript の型と JSON Schema は Rust から生成する。変えたら `just gen` を実行する
 *   (`docs/editor.md#型の生成`)。
 *
 * 削除時の連鎖 (カスケード) と拒否の規則は `docs/editor.md#コマンド` にある。
 */
export type Command = { "op": "add_parameter", parameter: Parameter, } | { "op": "remove_parameter", id: string, } | { "op": "set_parameter_range", id: string, min: number, default: number, max: number, } | { "op": "batch", label: string, commands: Array<Command>, } | { "op": "rename_parameter", id: string, name: string, } | { "op": "set_parameter_group", id: string, group: string | null, } | { "op": "set_parameter_mirror", id: string, mirror: string | null, } | { "op": "add_part", part: Part, } | { "op": "remove_part", id: string, } | { "op": "rename_part", id: string, name: string, } | { "op": "set_part_parent", id: string, parent: string | null, } | { "op": "set_part_keyforms", id: string, keyforms: KeyformGrid<PartKeyform>, } | { "op": "add_deformer", deformer: Deformer, } | { "op": "remove_deformer", id: string, } | { "op": "rename_deformer", id: string, name: string, } | { "op": "set_deformer_parent", id: string, parent: string | null, } | { "op": "set_deformer_part", id: string, part: string, } | { "op": "set_warp_keyforms", id: string, keyforms: KeyformGrid<WarpKeyform>, } | { "op": "set_rotation_keyforms", id: string, keyforms: KeyformGrid<RotationKeyform>, } | { "op": "add_mesh", mesh: Mesh, } | { "op": "remove_mesh", id: string, } | { "op": "rename_mesh", id: string, name: string, } | { "op": "set_mesh_properties", id: string, part?: string | null, texture?: number | null, blend?: BlendMode | null, culling?: boolean | null, masks?: Array<MaskRef> | null, mask_mode?: MaskMode | null, mask_only?: boolean | null, } | { "op": "set_mesh_parent", id: string, parent: string | null, } | { "op": "set_mesh_keyforms", id: string, keyforms: KeyformGrid<MeshKeyform>, } | { "op": "set_mesh_keyform", id: string, cell: { [key in string]: number }, keyform: MeshKeyform, } | { "op": "reorder_mesh", id: string, before: string | null, } | { "op": "add_texture", name: string, png: Array<number>, } | { "op": "set_texture_image", name: string, png: Array<number>, } | { "op": "remove_texture", name: string, } | { "op": "set_canvas", width: number, height: number, origin: Vec2, } | { "op": "regenerate_mesh", id: string, options: AutoMeshOptions, } | { "op": "infer_keyforms", target: KeyformTarget, method: InferMethod, overwrite_derived: boolean, } | { "op": "mirror_keyforms", source: KeyformTarget, target: KeyformTarget, 
/**
 * 対称軸 `x = a` の `a`。省略すると元の要素の `mirror_axis` (または親で決まる既定値)。
 */
axis?: number | null, 
/**
 * 頂点の対応付けの許容距離 (元の要素の親座標系)。省略するとキャンバスの 1px。
 */
tolerance?: number | null, 
/**
 * `true` なら先の `authored` のセルも上書きする。
 */
overwrite_authored: boolean, } | { "op": "mark_keyform_authored", target: KeyformTarget, cells: Array<{ [key in string]: number }>, } | { "op": "mark_keyform_derived", target: KeyformTarget, cells: Array<{ [key in string]: number }>, } | { "op": "move_mesh_vertices", id: string, moves: Array<VertexMove>, } | { "op": "add_mesh_vertex", id: string, position: Vec2, } | { "op": "remove_mesh_vertices", id: string, indices: Array<number>, } | { "op": "retriangulate_mesh", id: string, } | { "op": "add_keyform_key", target: KeyformTarget, parameter: string, key: number, } | { "op": "remove_keyform_key", target: KeyformTarget, parameter: string, key: number, } | { "op": "set_keyform_cell", target: KeyformTarget, cell: { [key in string]: number }, keyform: Keyform, } | { "op": "set_mesh_draw_order", id: string, draw_order: number, } | { "op": "create_warp_deformer", id?: string | null, name: string, children: Array<ChildRef>, rows: number, cols: number, parent?: string | null, } | { "op": "create_rotation_deformer", id?: string | null, name: string, children: Array<ChildRef>, origin?: Vec2 | null, parent?: string | null, } | { "op": "reparent_preserving_look", element: ChildRef, new_parent: string | null, };

export type Curve = { target: CurveTarget, segments: Array<Segment>, 
/**
 * カーブ個別のフェード (未指定ならモーション全体の値)。
 */
fade_in: number | null, fade_out: number | null, };

/**
 * カーブの対象。JSON では `{"kind": "parameter", "id": "ParamAngleX"}` と書く。
 *
 * 隣接タグ付きの表現では `#[serde(other)]` が中身 (`id`) を読めないので、
 * 読み込みは手で書く ([`CurveTarget::deserialize`])。
 */
export type CurveTarget = { "kind": "parameter", "id": string } | { "kind": "part_opacity", "id": string } | { "kind": "effect_group", "id": string };

/**
 * デフォーマ: 子 (デフォーマ / メッシュ) の頂点を変形する。
 * 子の頂点座標は **親デフォーマの局所座標系** で表現される (`docs/spec/evaluation.md`)。
 */
export type Deformer = { id: string, name: string, part: string, 
/**
 * `None` ならキャンバス座標直下。
 */
parent: string | null, kind: DeformerKind, 
/**
 * ミラーで対にするデフォーマ (オーサリング用)。`None` と自分自身は「自分と対応する」。
 * 対は双方向に張る (`docs/spec/keyform-inference.md#ミラー`)。
 */
mirror?: string | null, 
/**
 * ミラーの対称軸 (縦の直線) の x 座標。原点・制御点と同じ親座標系で表す (オーサリング用)。
 * `None` なら親で決まる既定値 (`docs/spec/keyform-inference.md#対称軸`)。
 */
mirror_axis?: number | null, };

export type DeformerKind = { "type": "warp" } & WarpDeformer | { "type": "rotation" } & RotationDeformer;

/**
 * 深度からの回転の設定。対象の深度はメッシュ・ワープデフォーマの `depth` に持つ。
 */
export type DepthRotation = { 
/**
 * パラメータの値に掛けて角度 (度) にする係数。既定 1。
 */
angle_scale: number, 
/**
 * 回転の中心の位置 (対象の頂点・制御点と同じ親座標系)。
 */
pivot: Vec2, 
/**
 * 回転の中心の深度 (キャンバスのピクセル単位、手前が正)。既定 0。
 */
pivot_depth: number, 
/**
 * 縦軸まわりの回転 (yaw) に使う軸のパラメータ (通常 `AngleX`)。`None` なら回さない。
 */
yaw?: string | null, 
/**
 * 横軸まわりの回転 (pitch) に使う軸のパラメータ (通常 `AngleY`)。`None` なら回さない。
 */
pitch?: string | null, };

/**
 * エフェクト設定一式。
 */
export type Effects = { eye_blink: EyeBlink, breath: Breath, lip_sync: LipSync, };

/**
 * 表情: モーションの上に重ねるパラメータ差分。
 */
export type Expression = { name: string, fade_in: number, fade_out: number, parameters: Array<ExpressionParameter>, };

export type ExpressionOp = "add" | "multiply" | "overwrite";

export type ExpressionParameter = { parameter: string, value: number, op: ExpressionOp, };

/**
 * 目パチ。対象パラメータを 1 → 0 → 1 と動かす。時間はすべて秒。
 */
export type EyeBlink = { 
/**
 * エフェクトグループ `EyeBlink` の対象。
 */
parameters: Array<string>, 
/**
 * まばたきの平均間隔。0 より大きい。
 */
interval: number, 
/**
 * 閉じるのにかける時間。0 以上。
 */
closing: number, 
/**
 * 閉じたまま保つ時間。0 以上。
 */
closed: number, 
/**
 * 開くのにかける時間。0 以上。
 */
opening: number, 
/**
 * 間隔を決める決定的な擬似乱数のシード。
 */
seed: number, };

/**
 * 2 つのメッシュの頂点同士を引き寄せて継ぎ目を防ぐ「接着」。
 */
export type Glue = { id: string, mesh_a: string, mesh_b: string, pairs: Array<GluePair>, keyforms: KeyformGrid<GlueKeyform>, };

export type GlueKeyform = { intensity: number, };

export type GluePair = { a: number, b: number, 
/**
 * 0.0 なら a 側に、1.0 なら b 側に寄せる。
 */
weight: number, };

/**
 * 当たり判定領域。メッシュの変形後形状で判定する。
 */
export type HitArea = { id: string, mesh: string, };

/**
 * [`Command::InferKeyforms`] の推定の方法。
 */
export type InferMethod = "from_grid" | "additive" | "depth_rotation";

/**
 * 1 つのセルのキーフォーム。種類は対象の格子と一致させる。
 * JSON では `{"kind": "mesh", "positions": [..], "opacity": 1, ..}` のようにフィールドを並べる。
 */
export type Keyform = { "kind": "part" } & PartKeyform | { "kind": "warp" } & WarpKeyform | { "kind": "rotation" } & RotationKeyform | { "kind": "mesh" } & MeshKeyform | { "kind": "glue" } & GlueKeyform;

/**
 * 1 本のパラメータ軸と、その軸上のキー位置 (昇順・重複なし)。
 */
export type KeyformBinding = { parameter: string, keys: Array<number>, };

/**
 * N 本のパラメータ軸の直積格子上に並んだキーフォーム。
 *
 * `keyforms.len() == bindings.iter().map(|b| b.keys.len()).product()` (軸 0 本なら 1)。
 * 並びは **軸 0 が最も速く変化する** 行優先 (column-major 的) 順。
 * 格子座標 `(i0, i1, ..)` の線形インデックスは `i0 + n0 * (i1 + n1 * (i2 + ..))`。
 *
 * `origins`・`inference`・`mirror_source` はオーサリング用のデータで、ランタイムは読み飛ばす
 * (`docs/spec/keyform-inference.md`)。
 */
export type KeyformGrid<T> = { bindings: Array<KeyformBinding>, keyforms: Array<T>, 
/**
 * セルごとの由来。並びは `keyforms` と同じ。
 * 空ならすべて `authored`。空でなければ長さは `keyforms` と同じ。
 */
origins?: Array<KeyformOrigin>, 
/**
 * 方法を記録していない `derived` のセルを作り直すときの推定の設定。`None` なら加算合成 (`additive`)。
 */
inference?: KeyformInference | null, 
/**
 * `mirror` のセルのミラー元と、ミラーの設定。ミラーで作ったセルがあれば記録する。
 */
mirror_source?: MirrorSource | null, };

/**
 * 格子の `derived` のセルを作る推定の方法と、その設定。
 * JSON では `{"method": "additive"}` や `{"method": "depth_rotation", "pivot": .., ..}` と書く。
 */
export type KeyformInference = { "method": "additive" } | { "method": "depth_rotation" } & DepthRotation;

/**
 * キーフォームのセルの由来 (`docs/spec/keyform-inference.md#セルの由来`)。
 */
export type KeyformOrigin = "authored" | "derived" | "additive" | "depth_rotation" | "mirror";

/**
 * キーフォームの格子を持つ要素。JSON では `{"kind": "mesh", "id": "EyeL"}` と書く。
 *
 * どの要素も格子を 1 つだけ持つ (`ugoe_rigging::KeyformTarget` と同じ)。
 */
export type KeyformTarget = { "kind": "part", "id": string } | { "kind": "mesh", "id": string } | { "kind": "deformer", "id": string } | { "kind": "glue", "id": string };

/**
 * リップシンク。外部から与えた音量 (0〜1) を対象パラメータに書き込む。
 */
export type LipSync = { 
/**
 * エフェクトグループ `LipSync` の対象。
 */
parameters: Array<string>, };

/**
 * 通常マスクの結合方法。式は `docs/spec/rendering.md#マスク`。
 */
export type MaskMode = "union" | "intersection";

/**
 * マスクとして使うメッシュへの参照。
 */
export type MaskRef = { mesh: string, 
/**
 * `true` なら反転マスク (メッシュの内側を隠す)。
 */
invert: boolean, };

/**
 * 描画単位となるテクスチャ付き三角形メッシュ。
 */
export type Mesh = { id: string, name: string, part: string, 
/**
 * `None` ならキャンバス座標直下。
 */
parent: string | null, 
/**
 * `Model::textures` のインデックス。
 */
texture: number, 
/**
 * テクスチャ座標 (0..1, 左上原点)。頂点数と同じ長さ。
 */
uvs: Array<Vec2>, 
/**
 * 三角形インデックス (3 の倍数)。
 */
indices: Array<number>, blend: BlendMode, 
/**
 * 裏面カリングするか。
 */
culling: boolean, 
/**
 * クリッピングマスクの参照。意味論は `docs/spec/rendering.md#マスク`。
 */
masks: Array<MaskRef>, 
/**
 * 通常マスク (`invert = false` の参照) の結合方法。
 */
mask_mode: MaskMode, 
/**
 * `true` なら色としては描画せず、マスクとしてだけ働く。
 */
mask_only: boolean, keyforms: KeyformGrid<MeshKeyform>, 
/**
 * ミラーで対にするメッシュ (オーサリング用)。`None` と自分自身は「自分と対応する」。
 * 対は双方向に張る (`docs/spec/keyform-inference.md#ミラー`)。
 */
mirror?: string | null, 
/**
 * ミラーの対称軸 (縦の直線) の x 座標。頂点と同じ親座標系で表す (オーサリング用)。
 * `None` なら親で決まる既定値 (`docs/spec/keyform-inference.md#対称軸`)。
 */
mirror_axis?: number | null, 
/**
 * 頂点ごとの深度 (キャンバスのピクセル単位、手前が正。オーサリング用)。長さは `uvs` と同じ。
 */
depth?: Array<number> | null, };

export type MeshKeyform = { 
/**
 * 親座標系での頂点位置。長さは `uvs` と同じ。
 */
positions: Array<Vec2>, opacity: number, 
/**
 * 描画順 (大きいほど手前)。同値はモデル内の宣言順で解決。
 */
draw_order: number, multiply_color: Rgb, screen_color: Rgb, 
/**
 * マスクの効き具合 (0〜1、既定 1)。範囲外の値はランタイムがクランプする。
 */
mask_strength: number, };

/**
 * ミラーで作ったセル (`KeyformOrigin::Mirror`) のミラー元と、ミラーの設定
 * (`docs/spec/keyform-inference.md#ミラー`)。エディタがミラーをやり直すときに使う。
 */
export type MirrorSource = { 
/**
 * ミラー元の要素の ID。格子を持つ要素と同じ種類 (メッシュならメッシュ、デフォーマならデフォーマ)。
 * 自分自身の ID なら、左右対称な 1 つの要素の中で写し合ったセル。
 */
id: string, 
/**
 * 対称軸 `x = a` の `a` を明示した場合の値。`None` なら元の要素の `mirror_axis` (または既定値)。
 */
axis?: number | null, 
/**
 * 頂点の対応付けの許容距離を明示した場合の値。`None` ならキャンバスの 1px。
 */
tolerance?: number | null, };

/**
 * 1 体のリグ付きモデル全体。
 */
export type Model = { schema_version: number, meta: ModelMeta, parameters: Array<Parameter>, parameter_groups: Array<ParameterGroup>, parts: Array<Part>, deformers: Array<Deformer>, meshes: Array<Mesh>, glues: Array<Glue>, textures: Array<TextureRef>, hit_areas: Array<HitArea>, 
/**
 * 排他的なパーツ切り替え。省略時はグループなし (`docs/spec/model.md#ポーズ`)。
 */
pose: Pose, 
/**
 * 目パチ・呼吸・リップシンクの設定。省略時は対象なし (`docs/spec/motion.md#エフェクト`)。
 */
effects: Effects, };

export type ModelMeta = { name: string, canvas: Canvas, };

export type Motion = { name: string, 
/**
 * 秒。
 */
duration: number, looping: boolean, fade_in: number, fade_out: number, curves: Array<Curve>, events: Array<MotionEvent>, };

export type MotionEvent = { t: number, value: string, };

export type NormalizationRange = { min: number, default: number, max: number, };

/**
 * モデルを動かす入力スカラー値 (例: 顔の角度 X, 目の開閉)。
 */
export type Parameter = { id: string, name: string, min: number, max: number, default: number, 
/**
 * ブレンドシェイプ用パラメータか (加算変形。`docs/spec/model.md`)。
 */
kind: ParameterKind, 
/**
 * 左右ペア (例: EyeLOpen ↔ EyeROpen)。エディタのミラー機能・AI 自動設定で使う。
 */
mirror: string | null, group: string | null, };

/**
 * エディタ表示用のパラメータグループ。ランタイムの挙動には影響しない。
 */
export type ParameterGroup = { id: string, name: string, };

export type ParameterKind = "normal" | "blend_shape";

/**
 * パーツ: メッシュ・デフォーマをまとめる論理グループ。不透明度は親子で乗算される。
 */
export type Part = { id: string, name: string, parent: string | null, keyforms: KeyformGrid<PartKeyform>, };

export type PartKeyform = { opacity: number, };

export type PhysicsInput = { parameter: string, source: PhysicsSource, 
/**
 * 0..=100 (%)。
 */
weight: number, reflect: boolean, };

export type PhysicsNormalization = { position: NormalizationRange, angle: NormalizationRange, };

export type PhysicsOutput = { parameter: string, 
/**
 * 出力元となる粒子インデックス (その粒子と 1 つ前の粒子の成す角を使う)。
 */
particle: number, source: PhysicsSource, scale: number, weight: number, reflect: boolean, };

export type PhysicsParticle = { 
/**
 * 初期位置 (根元からの相対)。
 */
position: Vec2, mobility: number, delay: number, acceleration: number, radius: number, };

export type PhysicsRig = { id: string, name: string, inputs: Array<PhysicsInput>, outputs: Array<PhysicsOutput>, particles: Array<PhysicsParticle>, normalization: PhysicsNormalization, };

export type PhysicsSettings = { gravity: Vec2, wind: Vec2, 
/**
 * 固定ステップ実行の周波数 (Hz)。
 */
fps: number, rigs: Array<PhysicsRig>, };

export type PhysicsSource = "x" | "y" | "angle";

/**
 * 点 (time, value)。
 */
export type Point = { t: number, v: number, };

/**
 * ポーズ設定。グループごとに、表示中のパーツを 1 つに保つ。
 *
 * `groups` が空ならポーズは何もしない (`Default` はこの状態)。
 */
export type Pose = { 
/**
 * 表示パーツを切り替えるときのクロスフェード時間 (秒)。0 以上。
 */
fade_duration: number, groups: Array<PoseGroup>, };

/**
 * ポーズグループの 1 要素。
 */
export type PoseEntry = { part: string, 
/**
 * `part` と同じ不透明度に追従するパーツ (例: 腕に付属する袖)。
 */
links: Array<string>, };

/**
 * 互いに排他的なパーツの組 (例: 腕の差分 A / B)。
 */
export type PoseGroup = { entries: Array<PoseEntry>, };

/**
 * フロントエンドに見せるプロジェクトの状態。
 */
export type ProjectStatus = { name: string, 
/**
 * 保存先のディレクトリ。未保存なら `null`。
 */
path: string | null, 
/**
 * 最後に保存 (または開いた) ときから変更があるか。
 */
modified: boolean, };

/**
 * 取り込みの警告。
 */
export type PsdImportWarning = { 
/**
 * レイヤーの名前の列 (外側のグループから順に、最後がレイヤー自身)。
 * ドキュメント全体についての警告では空。
 */
path: Array<string>, message: string, };

/**
 * リニアではなく sRGB 値の RGB (0.0..=1.0)。乗算色・スクリーン色に使う。
 */
export type Rgb = { r: number, g: number, b: number, };

/**
 * 回転デフォーマ: 原点まわりの回転・拡縮・反転。
 * 子の局所座標は原点を (0,0) とした回転前の座標 (単位はキャンバスピクセル)。
 */
export type RotationDeformer = { keyforms: KeyformGrid<RotationKeyform>, };

export type RotationKeyform = { 
/**
 * 親座標系での原点。
 */
origin: Vec2, 
/**
 * 度数法、時計回り正 (Y 下向き座標系)。
 */
angle: number, scale: number, reflect_x: boolean, reflect_y: boolean, opacity: number, };

/**
 * 連続するセグメント列。各セグメントの始点は直前セグメントの終点。
 */
export type Segment = { "type": "start", p: Point, } | { "type": "linear", to: Point, } | { "type": "bezier", c1: Point, c2: Point, to: Point, } | { "type": "stepped", to: Point, } | { "type": "inverse_stepped", to: Point, };

/**
 * テクスチャ参照。実データはコンテナ (`ugoe-format`) 側が保持する。
 */
export type TextureRef = { name: string, width: number, height: number, };

/**
 * [`crate::Vec2`] の JSON 表現 `[x, y]` (glam の serde 形式)。
 *
 * glam の型には `TS` / `JsonSchema` を実装できないので、`Vec2` のフィールドに
 * `#[ts(as = "ugoe_core::codegen::Vec2Repr")]` や `#[schemars(with = "..")]` を付けて使う。
 * 生成される型の名前は `Vec2`。
 */
export type Vec2 = [number, number];

/**
 * [`Command::MoveMeshVertices`] の 1 頂点分の移動。
 */
export type VertexMove = { 
/**
 * 頂点の番号 (`Mesh::uvs` の添字)。
 */
index: number, 
/**
 * キャンバス座標でのずれ。
 */
delta: Vec2, };

/**
 * ワープデフォーマ: (rows+1)×(cols+1) 個の制御点を持つ格子による自由変形。
 * 子の局所座標は格子全体を [0,1]² に正規化した UV 空間。
 */
export type WarpDeformer = { rows: number, cols: number, keyforms: KeyformGrid<WarpKeyform>, 
/**
 * 制御点ごとの深度 (キャンバスのピクセル単位、手前が正。オーサリング用)。
 * 並びは `WarpKeyform::points` と同じで、長さ `(rows+1)*(cols+1)`。
 */
depth?: Array<number> | null, };

export type WarpKeyform = { 
/**
 * 行優先 (x が速い) の制御点。長さ `(rows+1)*(cols+1)`。親座標系。
 */
points: Array<Vec2>, opacity: number, };
