// このファイルは生成物です。手で編集せず `just gen` で作り直してください (docs/editor.md#型の生成)。

/**
 * 外部の AI に送る IPC コマンドの戻り値。
 */
export type AiReply<T> = { "status": "done", value: T, } | { "status": "needs_consent", disclosure: DisclosureView, };

/**
 * 設定の画面に出す AI の状態。キーの中身は含めない。
 */
export type AiSettings = { 
/**
 * 選んだプロバイダ・モデル・ベースの URL (エディタ全体の設定)。
 */
provider: ProviderSettings, 
/**
 * 利用者に示す送り先のサービスの名前。
 */
service: string, 
/**
 * 送り先がこの計算機 (ループバック) か。外部に出ないので同意を求めず、「外部に送らない」でも使える。
 */
local: boolean, 
/**
 * 選んだプロバイダの API キーの、資格情報の保存先での状態。
 */
key: KeyStatus, 
/**
 * 選んだプロバイダがキーなしでは送れないか (手元のサーバなどはキーが要らない)。
 */
key_required: boolean, 
/**
 * 保存先にキーがないときに読む環境変数 (例: `ANTHROPIC_API_KEY`)。なければ `null`。
 */
key_env: string | null, 
/**
 * その環境変数にキーがあるか (保存先にキーがなければこちらを使う)。
 */
env_key: boolean, 
/**
 * 開いているプロジェクトのデータを外部の AI サービスに送ってよいか。
 */
send_externally: boolean, 
/**
 * このプロジェクトで「今後は確かめない」を選んだか (エディタを終えるかプロジェクトを替えると消える)。
 */
consent_remembered: boolean, };

/**
 * ドキュメントのモーションと表情 (`get_animations` の戻り値)。
 */
export type AnimationData = { motions: Array<Motion>, expressions: Array<Expression>, };

export type AppInfo = { version: string, };

/**
 * [`Document::apply_with_report`] の結果: コマンドは適用したが、利用者に知らせたいこと。
 */
export type ApplyReport = { 
/**
 * 自動の作り直しで失敗し、前の値のまま残したセルがある格子と、親を変えたことでミラーし直せなく
 * なった左右の対。
 */
warnings: Array<RederiveWarning>, 
/**
 * 明示的に実行したミラー (`MirrorKeyforms`) ごとの、書いたセルと残したセルの数。
 */
mirrored: Array<MirrorReport>, };

/**
 * エージェントが求めた、利用者の確認が要る操作の種類。
 */
export type ApprovalAction = "apply" | "undo" | "redo" | "save";

/**
 * 確認がどう終わったか (画面は、利用者が答えずに閉じたわけを知らせる)。
 */
export type ApprovalOutcome = "approved" | "declined" | "timed_out" | "cancelled";

/**
 * 確認のダイアログに出す、エージェントの求め。
 */
export type ApprovalRequest = { 
/**
 * 答えるときの番号 (`answer_mcp_approval`)。
 */
id: number, action: ApprovalAction, 
/**
 * 最後に握手したクライアントの名前 (複数のクライアントが同時につながると区別できない)。
 */
client: string | null, 
/**
 * 適用する提案の番号と題名 (`apply`)。
 */
proposal: number | null, title: string | null, 
/**
 * 外すコマンドの番号 (`apply`)。
 */
excluded: Array<number>, 
/**
 * 保存と一緒に書き出す `.ugoe` (`save`)。
 */
bundle: string | null, 
/**
 * 答えを待つ秒数。
 */
timeout_secs: number, };

/**
 * アトラス化の結果の要約。
 */
export type AtlasReport = { 
/**
 * アトラスにまとめた元のテクスチャの数 (重複として除いたものを含む)。
 */
packed: number, 
/**
 * 作ったアトラスの幅と高さ (出力の `Model::textures` の先頭から順に並ぶ)。
 */
atlases: Array<[number, number]>, 
/**
 * まとめずに残したテクスチャ (元の名前と理由)。アトラスの後ろに元の順で並ぶ。
 */
kept: Array<[string, KeptReason]>, 
/**
 * どのメッシュも使わないので取り除いたテクスチャの名前。
 */
removed: Array<string>, 
/**
 * 画素が同じ画像が先にあったので詰めなかったテクスチャ (名前と、代わりに使う画像の名前)。
 * 描画順で最初に使われる順に並ぶ。
 */
deduplicated: Array<[string, string]>, };

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
 * 準拠の検査 (`ugoe_model::avatar::check::check_profile`) の結果を、画面に出す形にしたもの。
 */
export type AvatarCheck = { 
/**
 * モデルが主張するレベル。プロファイルがなければ `None`。
 */
claimed: number | null, 
/**
 * メタデータと必要なパラメータから見て満たしているレベル (どのレベルも満たさなければ 0)。
 */
satisfied: number, 
/**
 * エラー (1 つでもあれば準拠していない)。コードは `avatar.*` (`docs/editor.md#エラーと警告のコード`)。
 */
errors: Array<Message>, 
/**
 * 警告 (準拠は妨げない)。コードは `avatar.*`。
 */
warnings: Array<Message>, };

/**
 * 標準の当たり判定の名前と `HitArea` の対応。
 */
export type AvatarHitArea = { 
/**
 * 標準の名前 ([`HIT_AREA_HEAD`]、[`HIT_AREA_BODY`])。
 */
name: string, 
/**
 * `Model::hit_areas` の ID。
 */
hit_area: string, };

/**
 * アバターのメタデータと利用許諾 (`docs/spec/avatar-profile.md#メタデータ`)。
 */
export type AvatarMeta = { 
/**
 * アバターの名前 (必須)。
 */
name: string, 
/**
 * アバターのバージョン。
 */
version: string | null, 
/**
 * 作者の一覧 (必須。1 人以上)。
 */
authors: Array<string>, 
/**
 * 著作権の表示。
 */
copyright: string | null, 
/**
 * 連絡先。
 */
contact: string | null, 
/**
 * 原作・参考元。
 */
references: Array<string>, 
/**
 * サムネイル画像 (`Model::textures` のインデックス)。
 */
thumbnail: number | null, 
/**
 * 利用規約の URL。
 */
license_url: string | null, 
/**
 * アバターとして使ってよい人。
 */
avatar_permission: AvatarPermission, 
/**
 * 商用利用の範囲。
 */
commercial_usage: CommercialUsage, 
/**
 * 過度に暴力的な表現での利用を許すか。
 */
allow_violent_usage: boolean, 
/**
 * 過度に性的な表現での利用を許すか。
 */
allow_sexual_usage: boolean, 
/**
 * 政治・宗教の目的での利用を許すか。
 */
allow_political_or_religious_usage: boolean, 
/**
 * 反社会的・差別的な目的での利用を許すか。
 */
allow_antisocial_or_hate_usage: boolean, 
/**
 * クレジット表記が要るか。
 */
credit_notation: CreditNotation, 
/**
 * 改変していないモデル (ファイル) の再配布を許すか。
 */
allow_redistribution: boolean, 
/**
 * 改変の許諾。
 */
modification: Modification, 
/**
 * 素材などの第三者のライセンス表記。
 */
third_party_licenses: string | null, };

/**
 * アバターとして使ってよい人。
 */
export type AvatarPermission = "only_author" | "only_licensed" | "everyone";

/**
 * アバタープロファイル (`Model::avatar`)。`docs/spec/avatar-profile.md`。
 *
 * 省略したフィールドは既定値になる。許諾の既定値は最も制限の強い値。
 */
export type AvatarProfile = { 
/**
 * 準拠するプロファイルのバージョン。既定は [`PROFILE_VERSION`]。
 */
profile_version: number, 
/**
 * 準拠を主張するレベル (1〜[`MAX_LEVEL`])。既定は 1。
 */
level: number, 
/**
 * メタデータと利用許諾。
 */
meta: AvatarMeta, 
/**
 * トラッキングの対応の上書きと追加。
 */
tracking: Array<TrackingOverride>, 
/**
 * 視線の追従。省略すると [`LookAt::default`] (`EyeBallX` / `EyeBallY`) を、モデルにあれば使う。
 */
look_at?: LookAt | null, 
/**
 * 標準の当たり判定の名前と `Model::hit_areas` の対応。
 */
hit_areas: Array<AvatarHitArea>, 
/**
 * アプリに公開する着せ替えのスロットの ID (`docs/spec/modules.md`)。
 */
slots: Array<string>, };

/**
 * 本体の読み直し (`reload_base`) の結果。読み直せなければ `problem` があり、ドキュメントはそのまま。
 */
export type BaseReload = { status: ProjectStatus, problem: ModuleProblem | null, };

/**
 * 本体として選んだファイルの要約 (`read_module_base` の戻り値)。
 */
export type BaseSummary = { 
/**
 * 本体のモデルの名前。
 */
name: string, slots: Array<SlotSummary>, };

/**
 * 合成モード。式は `docs/spec/rendering.md`。
 */
export type BlendMode = "normal" | "additive" | "multiply";

/**
 * 呼吸。各パラメータに `weight * (offset + peak * sin(2π t / cycle))` を加算する。
 */
export type Breath = { 
/**
 * 呼吸で動かすパラメータと、それぞれの波。
 */
parameters: Array<BreathParameter>, };

/**
 * 呼吸で動かす 1 つのパラメータの波。
 */
export type BreathParameter = { 
/**
 * 対象のパラメータ。
 */
parameter: string, 
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
 * 握手したクライアント。
 */
export type BridgeClient = { name: string, version: string | null, 
/**
 * 最後に握手した時刻 (UNIX 時刻のミリ秒)。
 */
connected_at: number, };

/**
 * 画面へ送るイベント。
 */
export type BridgeEvent = { "kind": "status" } | { "kind": "approval", request: ApprovalRequest, } | { "kind": "approval_closed", id: number, outcome: ApprovalOutcome, } | { "kind": "document" } | { "kind": "proposals" };

/**
 * アルファの形からの法線マップの生成の設定 (`ugoe_rigging::BulgeOptions` のうち画面で選ぶもの)。
 */
export type BulgeSettings = { 
/**
 * ふくらみの強さ (高さと幅の比)。1 で断面が円、0 で平ら。
 */
strength: number, 
/**
 * 縁の丸みの幅 (テクスチャの画素)。縁だけが丸まり中は平らになる。`None` なら形全体を 1 つのふくらみにする。
 */
edge: number | null, 
/**
 * 高さをならすぼかしの標準偏差 (画素)。
 */
smoothing: number, };

/**
 * キャンバス。モデル座標系は「キャンバス左上原点・ピクセル単位・Y 下向き」。
 * 表示時の正規化 (高さ 2.0 の NDC 相当など) は描画側の責務。
 */
export type Canvas = { 
/**
 * キャンバスの幅 (ピクセル)。
 */
width: number, 
/**
 * キャンバスの高さ (ピクセル)。
 */
height: number, 
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
 * PSD のレイヤーの分類の結果。
 */
export type Classification = { 
/**
 * 分類した PSD のパス。
 */
psd: string, 
/**
 * AI で分類したか (偽なら名前の規則だけ)。
 */
by_ai: boolean, 
/**
 * AI で分類したときに添えた注文。
 */
instructions: string | null, layers: Array<LayerLabelView>, };

/**
 * Claude の設定。
 */
export type ClaudeSettings = { 
/**
 * モデルの ID。
 */
model: string, };

/**
 * モジュールの物理のリグが参照する、スロットの公開する本体の衝突体 1 つ (本体に物理が無いと当たれない。
 * [`ModuleContext::drop_unavailable_colliders`] が外したもの。T0661)。ID はモジュールの中のもの。
 * エラー型と違い、呼び出し側へデータとして返すので ID は型付きで持つ (TS ではどちらも `string`。T1600)。
 */
export type ColliderReference = { rig: string, collider: string, };

/**
 * 衝突体の形状。半径はキャンバスのピクセル単位。
 */
export type ColliderShape = { "type": "circle", 
/**
 * 円の中心 (`attach` の局所座標系、無ければキャンバス座標)。
 */
center: Vec2, 
/**
 * 円の半径 (ピクセル)。
 */
radius: number, } | { "type": "capsule", 
/**
 * 線分の一端 (`attach` の局所座標系、無ければキャンバス座標)。
 */
a: Vec2, 
/**
 * 線分のもう一端 (`a` と同じ座標系)。
 */
b: Vec2, 
/**
 * 線分からの距離で表す半径 (ピクセル)。
 */
radius: number, };

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
export type Command = { "op": "add_parameter", parameter: Parameter, } | { "op": "remove_parameter", id: string, } | { "op": "set_parameter_range", id: string, min: number, default: number, max: number, } | { "op": "batch", label: string, commands: Array<Command>, } | { "op": "rename_parameter", id: string, name: string, } | { "op": "set_parameter_group", id: string, group: string | null, } | { "op": "set_parameter_mirror", id: string, mirror: string | null, } | { "op": "add_part", part: Part, } | { "op": "remove_part", id: string, } | { "op": "rename_part", id: string, name: string, } | { "op": "set_part_parent", id: string, parent: string | null, } | { "op": "set_part_keyforms", id: string, keyforms: KeyformGrid<PartKeyform>, } | { "op": "add_deformer", deformer: Deformer, } | { "op": "remove_deformer", id: string, } | { "op": "rename_deformer", id: string, name: string, } | { "op": "set_deformer_parent", id: string, parent: string | null, } | { "op": "set_deformer_part", id: string, part: string, } | { "op": "set_warp_keyforms", id: string, keyforms: KeyformGrid<WarpKeyform>, } | { "op": "set_rotation_keyforms", id: string, keyforms: KeyformGrid<RotationKeyform>, } | { "op": "add_mesh", mesh: Mesh, } | { "op": "remove_mesh", id: string, } | { "op": "rename_mesh", id: string, name: string, } | { "op": "set_mesh_properties", id: string, part?: string | null, texture?: number | null, blend?: BlendMode | null, culling?: boolean | null, masks?: Array<MaskRef> | null, mask_mode?: MaskMode | null, mask_only?: boolean | null, 
/**
 * ライティングを受けるか (`Mesh::lit`、`docs/editor.md#ライティングの画面`)。
 */
lit?: boolean | null, } | { "op": "set_mesh_parent", id: string, parent: string | null, } | { "op": "set_mesh_keyforms", id: string, keyforms: KeyformGrid<MeshKeyform>, } | { "op": "set_mesh_keyform", id: string, cell: { [key in string]: number }, keyform: MeshKeyform, } | { "op": "reorder_mesh", id: string, before: string | null, } | { "op": "add_texture", name: string, png: Array<number>, } | { "op": "set_texture_image", name: string, png: Array<number>, } | { "op": "remove_texture", name: string, } | { "op": "set_canvas", width: number, height: number, origin: Vec2, } | { "op": "regenerate_mesh", id: string, options: AutoMeshOptions, } | { "op": "infer_keyforms", target: KeyformTarget, method: InferMethod, overwrite_derived: boolean, } | { "op": "mirror_keyforms", source: KeyformTarget, target: KeyformTarget, 
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
overwrite_authored: boolean, } | { "op": "mark_keyform_authored", target: KeyformTarget, cells: Array<{ [key in string]: number }>, } | { "op": "mark_keyform_derived", target: KeyformTarget, cells: Array<{ [key in string]: number }>, } | { "op": "move_mesh_vertices", id: string, moves: Array<VertexMove>, } | { "op": "add_mesh_vertex", id: string, position: Vec2, } | { "op": "remove_mesh_vertices", id: string, indices: Array<number>, } | { "op": "retriangulate_mesh", id: string, } | { "op": "add_keyform_key", target: KeyformTarget, parameter: string, key: number, } | { "op": "remove_keyform_key", target: KeyformTarget, parameter: string, key: number, } | { "op": "set_keyform_cell", target: KeyformTarget, cell: { [key in string]: number }, keyform: Keyform, } | { "op": "set_mesh_draw_order", id: string, draw_order: number, } | { "op": "create_warp_deformer", id?: string | null, name: string, children?: Array<ChildRef>, parts?: Array<string>, rows: number, cols: number, parent?: string | null, } | { "op": "create_rotation_deformer", id?: string | null, name: string, children?: Array<ChildRef>, parts?: Array<string>, origin?: Vec2 | null, parent?: string | null, } | { "op": "reparent_preserving_look", element: ChildRef, new_parent: string | null, } | { "op": "set_keyform_inference", target: KeyformTarget, inference?: KeyformInference | null, } | { "op": "set_mirror_pair", target: KeyformTarget, mirror?: string | null, axis?: number | null, } | { "op": "set_depth", target: ChildRef, depth?: Array<number> | null, } | { "op": "apply_depth_preset", target: ChildRef, preset: DepthPreset, } | { "op": "move_rotation_origin", id: string, delta: Vec2, } | { "op": "unwrap_deformer", id: string, } | { "op": "add_motion", motion: Motion, } | { "op": "remove_motion", name: string, } | { "op": "rename_motion", name: string, new_name: string, } | { "op": "set_motion_properties", name: string, duration?: number | null, looping?: boolean | null, fade_in?: number | null, fade_out?: number | null, } | { "op": "add_motion_curve", motion: string, curve: Curve, } | { "op": "remove_motion_curve", motion: string, target: CurveTarget, } | { "op": "set_motion_curve_fade", motion: string, target: CurveTarget, fade_in?: number | null, fade_out?: number | null, } | { "op": "set_motion_key", motion: string, target: CurveTarget, point: Point, } | { "op": "move_motion_key", motion: string, target: CurveTarget, t: number, to: Point, } | { "op": "remove_motion_key", motion: string, target: CurveTarget, t: number, } | { "op": "set_motion_segment", motion: string, target: CurveTarget, t: number, segment: SegmentShape, } | { "op": "set_motion_events", motion: string, events: Array<MotionEvent>, } | { "op": "add_expression", expression: Expression, } | { "op": "remove_expression", name: string, } | { "op": "rename_expression", name: string, new_name: string, } | { "op": "set_expression_properties", name: string, fade_in?: number | null, fade_out?: number | null, } | { "op": "set_expression_parameters", name: string, parameters: Array<ExpressionParameter>, } | { "op": "set_physics_world", gravity: Vec2, wind: Vec2, fps: number, } | { "op": "add_physics_rig", rig: PhysicsRig, } | { "op": "create_physics_rig", id?: string | null, name: string, deformers: Array<string>, } | { "op": "remove_physics_rig", id: string, } | { "op": "rename_physics_rig", id: string, name: string, } | { "op": "set_physics_inputs", rig: string, inputs: Array<PhysicsInput>, } | { "op": "set_physics_outputs", rig: string, outputs: Array<PhysicsOutput>, } | { "op": "set_physics_particles", rig: string, particles: Array<PhysicsParticle>, } | { "op": "set_physics_normalization", rig: string, normalization: PhysicsNormalization, } | { "op": "set_physics_rig_colliders", rig: string, colliders: Array<string>, } | { "op": "set_physics_frame", rig: string, frame?: string | null, } | { "op": "add_physics_collider", collider: PhysicsCollider, } | { "op": "remove_physics_collider", id: string, } | { "op": "set_physics_collider", collider: PhysicsCollider, } | { "op": "set_keyform_interpolation", target: KeyformTarget, blend_shape?: string | null, interpolation: KeyformInterpolation, } | { "op": "reorder_physics_rig", id: string, before: string | null, } | { "op": "set_parameter_internal", id: string, internal: boolean, } | { "op": "create_draw_clone", source: string, id?: string | null, name: string, } | { "op": "sync_draw_clone", id: string, } | { "op": "apply_physics_template", template: PhysicsTemplate, options: PhysicsTemplateOptions, } | { "op": "set_avatar_profile", profile: AvatarProfile | null, } | { "op": "update_avatar_profile", level?: number | null, meta?: AvatarMeta | null, tracking?: Array<TrackingOverride> | null, 
/**
 * 視線の追従。`null` で外す (既定の `EyeBallX` / `EyeBallY` に戻す)。
 */
look_at?: LookAt | null, hit_areas?: Array<AvatarHitArea> | null, slots?: Array<string> | null, } | { "op": "set_hit_areas", hit_areas: Array<HitArea>, } | { "op": "set_normal_map", texture: string, normal_map: string | null, 
/**
 * 法線マップの画像 (PNG)。JSON では数値の配列になるので、主にバックエンド内 (生成の提案など) から使う。
 */
png?: Array<number> | null, } | { "op": "set_lighting", enabled: boolean, } | { "op": "set_effects", effects: Effects, } | { "op": "add_slot", slot: Slot, } | { "op": "set_slot", id: string, slot: Slot, } | { "op": "remove_slot", id: string, } | { "op": "set_module_settings", name: string, slot: string, requires: ModuleRequirements, keep_look: boolean, } | { "op": "set_pose", pose: Pose, } | { "op": "create_glue", id?: string | null, mesh_a: string, mesh_b: string, distance: number, } | { "op": "set_glue", glue: Glue, distance?: number | null, } | { "op": "remove_glue", id: string, };

/**
 * 提案のコマンドを、画面でまとめて出す区切り (法線マップの提案のまとまりごと)。
 */
export type CommandSection = { 
/**
 * 区切りの名前 (まとまり `face` など。ラベルのないものは `unlabeled`、ライティングを有効にするのは `lighting`)。
 */
label: string, 
/**
 * 区切りに入るコマンドの番号 (提案の中の 0 始まり)。コマンドの対象 (テクスチャの名前) は
 * `ProposalCommand::subject`。
 */
indices: Array<number>, };

/**
 * 商用利用の範囲。
 */
export type CommercialUsage = "personal_non_profit" | "personal_profit" | "corporation";

/**
 * 確認のダイアログでの利用者の答え。送る IPC コマンドに付けて呼び直す。
 */
export type ConsentAnswer = { 
/**
 * 同意した内容 (ダイアログに示したもの)。今送ろうとしている内容と違えば、もう一度確かめる。
 */
disclosure: DisclosureView, 
/**
 * 「今後は確かめない」: このプロジェクトを閉じるかエディタを終えるまで、同じ送り先
 * (サービス・URL・モデル) には確かめずに送る。ファイルには残さない。
 */
remember: boolean, };

/**
 * クレジット表記。
 */
export type CreditNotation = "required" | "unnecessary";

/**
 * 1 つの対象の値を時間で動かすカーブ (`docs/spec/motion.md#カーブ`)。
 */
export type Curve = { 
/**
 * カーブが動かす対象。
 */
target: CurveTarget, 
/**
 * カーブの形。最初は必ず [`Segment::Start`] で、以降の各セグメントの始点は直前のセグメントの終点。
 */
segments: Array<Segment>, 
/**
 * カーブ個別のフェード (未指定ならモーション全体の値)。
 */
fade_in: number | null, 
/**
 * カーブ個別のフェードアウトの秒数 (未指定ならモーション全体の値)。終了時刻はモーションのものを使う。
 */
fade_out: number | null, };

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
export type Deformer = { 
/**
 * モデルの中で一意な ID。
 */
id: string, 
/**
 * 表示名 (エディタ用)。
 */
name: string, 
/**
 * 所属するパーツ。パーツの不透明度が子に掛かる。
 */
part: string, 
/**
 * `None` ならキャンバス座標直下。
 */
parent: string | null, 
/**
 * 変形の種類と、種類ごとのキーフォーム。
 */
kind: DeformerKind, 
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

/**
 * デフォーマの種類。
 */
export type DeformerKind = { "type": "warp" } & WarpDeformer | { "type": "rotation" } & RotationDeformer;

/**
 * [`crate::Command::ApplyDepthPreset`] の深度の作り方。JSON では `{"kind": "dome", "peak": 20}` と書く。
 */
export type DepthPreset = { "kind": "uniform", value: number, } | { "kind": "dome", peak: number, } | { "kind": "points", indices: Array<number>, value: number, };

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
 * 違いの行の要素 1 つ。
 */
export type DifferenceItem = { kind: DifferenceKind, 
/**
 * モジュールの中の ID (表示用)。
 */
id: string, 
/**
 * 何が違うか (英語。例: `vertices 12 → 14; draw order 500 → 520`。足したものは `added`、除いたものは `removed`)。
 */
text: string, 
/**
 * 選ぶときの、今の編集の ID (パーツ・デフォーマ・メッシュ・グルーは作業用のモデル、物理のリグと衝突体は
 * 作業用の物理の ID)。見つからなければ (除いた要素など) `None`。
 */
element: string | null, 
/**
 * プレビューのビューポートで強調するメッシュの、モジュールの中の ID (ファイルと今の編集のどちらかにあるもの)。
 * パーツはその下のメッシュ、デフォーマはその下のメッシュ、グルーは両端、物理のリグは出力先のメッシュとデフォーマの
 * 下のメッシュ。衝突体は空 (形をそのまま描く)。ポーズのグループは要素とリンクのパーツの下、追従はリンクの
 * パーツの下のメッシュ。
 */
meshes: Array<string>, 
/**
 * ポーズのグループと追従を選ぶときの、今の編集のパーツの ID (作業用のモデル。グループは要素のパーツ、
 * 追従は本体の要素のパーツとリンクのパーツ)。ほかの種類と除いた要素は空 (T0659)。
 */
parts: Array<string>, };

/**
 * 違いの行の要素の種類 (パーツ・デフォーマ・メッシュはエディタの選択の種類と同じ名前)。
 */
export type DifferenceKind = "part" | "deformer" | "mesh" | "glue" | "physics_rig" | "collider" | "pose_group" | "pose_follow";

/**
 * 外部に送る内容 (確認のダイアログに示す)。`ugoe_ai::Disclosure` の外部に送る場合の写し。
 */
export type DisclosureView = { 
/**
 * サービスの名前 (例: `Anthropic Claude API`)。
 */
service: string, 
/**
 * 送り先の URL。
 */
endpoint: string, 
/**
 * 使うモデル。
 */
model: string, image_count: number, 
/**
 * 画像の合計バイト数 (エンコード前)。
 */
image_bytes: number, 
/**
 * テキスト (システムプロンプトとプロンプト) の合計バイト数。
 */
text_bytes: number, 
/**
 * 答えが検証を通らなかったときに、同じ画像と前の答え・理由を添えて送り直すことがある回数の上限
 * (上の数は 1 回分。`ugoe_ai::rig::propose_rig_with_ai` の問い直し)。
 */
retries: number, 
/**
 * テキストに含まれる、利用者の注文の文字数 (`ugoe_ai::instructions::Instructions`)。注文がなければ 0。
 * 分かれたリクエストには、どれにも同じ注文が入る。
 */
instruction_chars: number, };

/**
 * 既定のパラメータで 1 フレームを描くときの、描画の見積もり。
 *
 * 参照レンダラの本描画のまとめ方 (`docs/spec/rendering.md#描画の最適化`) を、テクスチャ・合成モード・
 * カリングの有無だけで真似る。マスクアトラスの違い (GPU の上での配置で決まる) は数えないので、
 * マスクを使うモデルでは `ugoe render --stats` の `state_changes` より少なく出ることがある。
 */
export type DrawEstimate = { 
/**
 * テクスチャの枚数 (`Model::textures`)。
 */
textures: number, 
/**
 * 描く要素の数 (表示されていて三角形を持つメッシュ)。
 */
drawables: number, 
/**
 * まとめた後の描画の回数。
 */
draw_calls: number, 
/**
 * 前の描画から描画の状態を切り替える回数 (最初の描画は数えない)。
 */
state_changes: number, };

/**
 * 描画順の範囲 `[min, max]` (`min <= max`)。
 */
export type DrawOrderRange = { 
/**
 * 範囲の下限。
 */
min: number, 
/**
 * 範囲の上限。
 */
max: number, };

/**
 * エフェクト設定一式。
 */
export type Effects = { 
/**
 * 目パチ。
 */
eye_blink: EyeBlink, 
/**
 * 呼吸。
 */
breath: Breath, 
/**
 * リップシンク。
 */
lip_sync: LipSync, };

/**
 * 1 つの要素の、はみ出した点。
 */
export type ElementOverflow = { target: KeyformTarget, 
/**
 * 親のワープデフォーマ。
 */
parent: string, kind: OverflowPointKind, 
/**
 * はみ出した点 (番号の順)。
 */
points: Array<OverflowPoint>, };

/**
 * 書き出しの結果の要約 (エディタが書き出しの後に示す)。
 */
export type ExportReport = { 
/**
 * 書き出したバンドルの大きさ (バイト)。
 */
bytes: number, 
/**
 * アトラス化する前のモデルの見積もり。
 */
before: DrawEstimate, 
/**
 * 書き出したモデルの見積もり (アトラス化しなければ `before` と同じ)。
 */
after: DrawEstimate, 
/**
 * アトラス化したときの結果。しなければ `null`。
 */
atlas: AtlasReport | null, };

/**
 * 書き出しの設定。`project.json` の `editor.export` に残す (すべて既定値なら書かない)。
 * 足りないフィールドは既定値で読む。
 */
export type ExportSettings = { 
/**
 * テクスチャをアトラスにまとめるか。既定は `false`。
 */
atlas: boolean, 
/**
 * アトラス 1 枚の幅と高さの上限 (px、1〜[`MAX_ATLAS_SIZE`])。
 */
atlas_max_size: number, 
/**
 * 各画像の周りに複製する端の画素の幅 (px、0〜[`MAX_ATLAS_PADDING`])。
 */
atlas_padding: number, };

/**
 * 表情: モーションの上に重ねるパラメータ差分。
 */
export type Expression = { 
/**
 * 表情の名前。ホストはこの名前で表情を止める。
 */
name: string, 
/**
 * フェードインの時間 (秒)。0 以下ならフェードなし。
 */
fade_in: number, 
/**
 * 止めてからフェードアウトする時間 (秒)。0 以下ならフェードなし。
 */
fade_out: number, 
/**
 * 重ねるパラメータの値。
 */
parameters: Array<ExpressionParameter>, };

/**
 * 表情の値の合成の方法。`v` は現在の値、`w` はフェードの重み (`docs/spec/motion.md#表情`)。
 */
export type ExpressionOp = "add" | "multiply" | "overwrite";

/**
 * 表情が 1 つのパラメータに重ねる値。
 */
export type ExpressionParameter = { 
/**
 * 対象のパラメータ。
 */
parameter: string, 
/**
 * `op` で現在の値と合成する値。
 */
value: number, 
/**
 * 現在の値との合成の方法。
 */
op: ExpressionOp, };

/**
 * 開いているプロジェクトのファイルが、エディタの外で変わったこと (`docs/editor.md#外での書き換え`)。
 */
export type ExternalChange = { 
/**
 * 足された・消された・中身が変わったファイル (プロジェクトからの相対パス、`/` 区切り、名前の順)。
 */
files: Array<string>, 
/**
 * `project.json` がない (プロジェクトが消された・移された)。開き直せない。
 */
missing: boolean, 
/**
 * 外の状態の目印。同じ変更を二度知らせないために使う (エディタを起動している間だけ意味がある)。
 */
token: string, };

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
export type Glue = { 
/**
 * モデルの中で一意な ID。
 */
id: string, 
/**
 * 接着する一方のメッシュ (`GluePair::a` の頂点を持つ)。
 */
mesh_a: string, 
/**
 * 接着するもう一方のメッシュ (`GluePair::b` の頂点を持つ)。
 */
mesh_b: string, 
/**
 * 引き寄せる頂点のペア。この順に適用する。
 */
pairs: Array<GluePair>, 
/**
 * 引き寄せの強さのキーフォーム。
 */
keyforms: KeyformGrid<GlueKeyform>, };

/**
 * グルーの 1 つのキーフォーム。
 */
export type GlueKeyform = { 
/**
 * 引き寄せの強さ (0 なら引き寄せない、1 なら `weight` の位置に一致させる)。補間後の値はクランプしない。
 */
intensity: number, };

/**
 * グルーで引き寄せる頂点のペア。
 */
export type GluePair = { 
/**
 * `Glue::mesh_a` の頂点のインデックス。
 */
a: number, 
/**
 * `Glue::mesh_b` の頂点のインデックス。
 */
b: number, 
/**
 * 0.0 なら a 側に、1.0 なら b 側に寄せる。
 */
weight: number, };

/**
 * 編集の履歴の状態 (`history_status` の戻り値)。ボタンを押せるかと設定の画面に使う。
 */
export type HistoryStatus = { 
/**
 * Undo で戻れるか。
 */
can_undo: boolean, 
/**
 * Redo で進めるか。
 */
can_redo: boolean, 
/**
 * 続けて Undo できる数。
 */
undo_count: number, 
/**
 * 続けて Redo できる数。
 */
redo_count: number, 
/**
 * 覚える履歴 (Undo と Redo を合わせた数) の上限。
 */
undo_limit: number, };

/**
 * 当たり判定領域。メッシュの変形後形状で判定する。
 */
export type HitArea = { 
/**
 * モデルの中で一意な ID。ホストはこの ID で当たり判定を参照する。
 */
id: string, 
/**
 * 判定に使うメッシュ。
 */
mesh: string, };

/**
 * [`Command::InferKeyforms`] の推定の方法。
 */
export type InferMethod = "from_grid" | "additive" | "depth_rotation";

/**
 * まとめずに残したテクスチャの理由。
 */
export type KeptReason = "too_large" | "uv_out_of_range" | "has_normal_map";

/**
 * 資格情報の保存先の状態 (フロントエンドに見せるのはこれだけ。キーの中身は見せない)。
 */
export type KeyStatus = "stored" | "not_stored" | "unavailable";

/**
 * 1 つのセルのキーフォーム。種類は対象の格子と一致させる。
 * JSON では `{"kind": "mesh", "positions": [..], "opacity": 1, ..}` のようにフィールドを並べる。
 */
export type Keyform = { "kind": "part" } & PartKeyform | { "kind": "warp" } & WarpKeyform | { "kind": "rotation" } & RotationKeyform | { "kind": "mesh" } & MeshKeyform | { "kind": "glue" } & GlueKeyform;

/**
 * 1 本のパラメータ軸と、その軸上のキー位置 (昇順・重複なし)。
 */
export type KeyformBinding = { 
/**
 * 軸にするパラメータ。
 */
parameter: string, 
/**
 * 軸上のキー位置 (パラメータの値)。
 */
keys: Array<number>, };

/**
 * N 本のパラメータ軸の直積格子上に並んだキーフォーム。
 *
 * `keyforms.len() == bindings.iter().map(|b| b.keys.len()).product()` (軸 0 本なら 1)。
 * 並びは **軸 0 が最も速く変化する** 順 (列優先、column-major)。
 * 格子座標 `(i0, i1, ..)` の線形インデックスは `i0 + n0 * (i1 + n1 * (i2 + ..))`。
 *
 * `origins`・`inference`・`mirror_source` はオーサリング用のデータで、ランタイムは読み飛ばす
 * (`docs/spec/keyform-inference.md`)。
 *
 * `blend_shapes` はブレンドシェイプ格子の列で、この格子の補間結果に差分を加算する
 * (`docs/spec/evaluation.md#6-ブレンドシェイプ`)。
 */
export type KeyformGrid<T> = { 
/**
 * パラメータ軸。空ならキーフォームは 1 つで、パラメータに依存しない。
 */
bindings: Array<KeyformBinding>, 
/**
 * 格子点ごとのキーフォーム。並びは上記の列優先の順。
 */
keyforms: Array<T>, 
/**
 * キーの間の補間の種類 (`docs/spec/evaluation.md#曲線補間`)。既定は線形。
 */
interpolation?: KeyformInterpolation, 
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
mirror_source?: MirrorSource | null, 
/**
 * ブレンドシェイプ格子 (`docs/spec/model.md#ブレンドシェイプ`)。軸はすべて `kind = blend_shape` の
 * パラメータで、補間した値と既定値での値の差を、宣言順にこの格子の補間結果へ加える。
 * ブレンドシェイプ格子自身は `blend_shapes` を持たない (入れ子にしない)。
 */
blend_shapes?: Array<KeyformGrid<T>>, };

/**
 * 格子の `derived` のセルを作る推定の方法と、その設定。
 * JSON では `{"method": "additive"}` や `{"method": "depth_rotation", "pivot": .., ..}` と書く。
 */
export type KeyformInference = { "method": "additive" } | { "method": "depth_rotation" } & DepthRotation;

/**
 * キーフォーム格子のキーの間の補間の種類 (`docs/spec/evaluation.md#曲線補間`)。
 */
export type KeyformInterpolation = "linear" | "smooth";

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
 * 1 枚のレイヤーの分類 (`ugoe_ai::classify::LayerLabel`)。
 */
export type LayerLabelView = { 
/**
 * レイヤーの名前の列 (外側のグループから順に、最後がレイヤー自身)。
 */
path: Array<string>, 
/**
 * 意味ラベル (`face`・`eye_white` など。`docs/ai-integration.md#レイヤーの意味分類`)。
 */
label: string, 
/**
 * `left`・`right`・`center` (キャラクター自身の左右)。
 */
side: string, confidence: number, 
/**
 * 確からしさが低いか名前の規則と食い違い、人間が確かめたほうがよい。
 */
needs_review: boolean, 
/**
 * AI が同じ部品の別の絵 (差分) とみなしたレイヤーの組の番号 (`LayerLabel::variant`)。なければ `null`。
 */
variant: number | null, 
/**
 * AI の分類が名前の規則の確からしい答えと食い違うときの、名前の規則の答え
 * (`LayerLabel::name_disagreement`)。なければ `null`。
 */
name_disagreement: NameDisagreementView | null, };

/**
 * リップシンク。外部から与えた音量 (0〜1) を対象パラメータに書き込む。
 */
export type LipSync = { 
/**
 * エフェクトグループ `LipSync` の対象。
 */
parameters: Array<string>, };

/**
 * 視線の追従に使うパラメータと可動範囲。
 */
export type LookAt = { 
/**
 * 左右 (正でキャラクターの左)。既定は `EyeBallX`。
 */
x: string, 
/**
 * 上下 (正で上)。既定は `EyeBallY`。
 */
y: string, 
/**
 * 左右の可動範囲 (度)。見る先がこの角度だけ横にあるとき、`x` が端の値になる。0 より大きい。
 */
range_x: number, 
/**
 * 上下の可動範囲 (度)。0 より大きい。
 */
range_y: number, };

/**
 * 通常マスクの結合方法。式は `docs/spec/rendering.md#マスク`。
 */
export type MaskMode = "union" | "intersection";

/**
 * マスクとして使うメッシュへの参照。
 */
export type MaskRef = { 
/**
 * マスクにするメッシュ。
 */
mesh: string, 
/**
 * `true` なら反転マスク (メッシュの内側を隠す)。
 */
invert: boolean, };

/**
 * 「Connect agents」の画面に出す状態。トークンは含めない (`get_mcp_bridge_token`)。
 */
export type McpBridgeStatus = { enabled: boolean, 
/**
 * 待ち受けている。
 */
running: boolean, 
/**
 * MCP の URL (`http://127.0.0.1:<port>/mcp`)。
 */
url: string | null, 
/**
 * 決めたポート。`null` なら起動のたびに空いているポート。
 */
port: number | null, 
/**
 * 発見用のファイル (`ugoe-mcp --editor` が読む)。
 */
discovery: string | null, 
/**
 * 始められなかった理由。
 */
error: Message | null, clients: Array<BridgeClient>, 
/**
 * 答えを待っている確認 (画面を開き直したときに出し直す)。
 */
pending: Array<ApprovalRequest>, 
/**
 * エージェントがツールに渡すプロジェクトのパス。未保存なら `null` (パスで指せない)。
 */
project: string | null, sidecar: Sidecar, };

/**
 * 描画単位となるテクスチャ付き三角形メッシュ。
 */
export type Mesh = { 
/**
 * モデルの中で一意な ID。
 */
id: string, 
/**
 * 表示名 (エディタ用)。
 */
name: string, 
/**
 * 所属するパーツ。パーツの不透明度が掛かる。
 */
part: string, 
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
indices: Array<number>, 
/**
 * 合成モード。
 */
blend: BlendMode, 
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
mask_only: boolean, 
/**
 * 頂点位置・不透明度・描画順・色のキーフォーム。
 */
keyforms: KeyformGrid<MeshKeyform>, 
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
depth?: Array<number> | null, 
/**
 * ライティングを受けるか (既定 `true`、`true` は書き出しでも省略する)。
 * `Model::lighting` が `true` のときだけ効く (`docs/spec/rendering.md#ライティング`)。
 */
lit?: boolean, };

/**
 * メッシュの 1 つのキーフォーム。
 */
export type MeshKeyform = { 
/**
 * 親座標系での頂点位置。長さは `uvs` と同じ。
 */
positions: Array<Vec2>, 
/**
 * 不透明度 (0〜1)。
 */
opacity: number, 
/**
 * 描画順 (大きいほど手前)。同値はモデル内の宣言順で解決。
 */
draw_order: number, 
/**
 * 乗算色 (既定は白 = 変化なし)。式は `docs/spec/rendering.md`。
 */
multiply_color: Rgb, 
/**
 * スクリーン色 (既定は黒 = 変化なし)。式は `docs/spec/rendering.md`。
 */
screen_color: Rgb, 
/**
 * マスクの効き具合 (0〜1、既定 1)。範囲外の値はランタイムがクランプする。
 */
mask_strength: number, };

/**
 * 画面に出すエラー・警告・問題 (ADR 0029)。Tauri のコマンドのエラーもこの形で返す。
 */
export type Message = { 
/**
 * 安定した識別子 (`project.not_a_project` など)。一覧は `docs/editor.md#エラーと警告のコード`。
 */
code: string, 
/**
 * 文に差し込む値。
 */
args: { [key in string]: MessageArg }, 
/**
 * 英語の文 (ログ・MCP・CLI・開発者向け。カタログに無いコードのときは画面もこれを出す)。
 */
message: string, 
/**
 * 原因になったエラー (`proposal.rejected` の中の `edit.*` など)。`message` は原因の文も含む。
 */
cause?: Message, 
/**
 * 一覧の中身 (検証のエラーの列 `validation.*` など)。並べるのは引数 `details` と同じく先頭の
 * `DESCRIBED_ERRORS_MAX` 件までで、全件の数は引数 `count`。画面は各項目を訳して `details` の代わりに差し込む。
 */
items?: Array<Message>, };

/**
 * 文に差し込む値。
 */
export type MessageArg = string | number;

/**
 * 明示的に実行したミラー ([`crate::Command::MirrorKeyforms`]) の結果 (`ApplyReport::mirrored`)。
 *
 * 1 つもセルを書かなかったときに、エディタが理由を出すのに使う (`docs/editor.md#推定とミラー`)。
 */
export type MirrorReport = { 
/**
 * ミラー元の要素。
 */
source: KeyformTarget, 
/**
 * セルを書いた (書こうとした) 格子の要素。
 */
target: KeyformTarget, 
/**
 * 書いたセルの数。
 */
written: number, 
/**
 * 書いたセルのうち、未編集の `authored` のセル
 * (`docs/spec/keyform-inference.md#未編集のセル`) だった数。
 */
unedited: number, 
/**
 * `overwrite_authored` なら書けたが、`authored` なので書かずに残したセルの数。
 */
kept_authored: number, };

/**
 * ミラーで作ったセル (`KeyformOrigin::Mirror`) のミラー元と、ミラーの設定
 * (`docs/spec/keyform-inference.md#ミラー`)。エディタがミラーをやり直すときに使う。
 */
export type MirrorSource = { 
/**
 * ミラー元の要素の ID。格子を持つ要素と同じ種類 (メッシュならメッシュ、デフォーマならデフォーマ)。
 * 自分自身の ID なら、左右対称な 1 つの要素の中で写し合ったセル。
 *
 * 格子を持つ要素はメッシュとデフォーマの 2 種類で、どちらを指すかは格子の持ち主で決まるため、
 * `Id<T>` ではなく `String` にしている (`docs/spec/model.md#ID`)。
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
export type Model = { 
/**
 * このデータのスキーマバージョン ([`SCHEMA_VERSION`])。
 */
schema_version: number, 
/**
 * 名前とキャンバス。
 */
meta: ModelMeta, 
/**
 * パラメータ。キーフォーム・モーション・物理などが ID で参照する。
 */
parameters: Array<Parameter>, 
/**
 * エディタ表示用のパラメータグループ。
 */
parameter_groups: Array<ParameterGroup>, 
/**
 * パーツ。階層は各パーツの `parent` で表す。
 */
parts: Array<Part>, 
/**
 * デフォーマ。評価は親が先の順で、結果は配列の順序に依存しない (`docs/spec/evaluation.md`)。
 */
deformers: Array<Deformer>, 
/**
 * メッシュ。
 */
meshes: Array<Mesh>, 
/**
 * メッシュの頂点同士の接着。配列の順に適用する (`docs/spec/evaluation.md#5-グルー`)。
 */
glues: Array<Glue>, 
/**
 * テクスチャの参照。メッシュの `texture` はこの列のインデックス。
 */
textures: Array<TextureRef>, 
/**
 * 当たり判定。ホストがクリック・タップされた部位を調べるのに使う。
 */
hit_areas: Array<HitArea>, 
/**
 * 排他的なパーツ切り替え。省略時はグループなし (`docs/spec/model.md#ポーズ`)。
 */
pose: Pose, 
/**
 * 目パチ・呼吸・リップシンクの設定。省略時は対象なし (`docs/spec/motion.md#エフェクト`)。
 */
effects: Effects, 
/**
 * アバタープロファイル。省略時はなし (`docs/spec/avatar-profile.md`)。
 */
avatar?: AvatarProfile | null, 
/**
 * 着せ替えのスロット。省略時はなし (`docs/spec/modules.md#スロット`)。
 */
slots?: Array<Slot>, 
/**
 * ホストが渡す光源でライティングするか。省略時は `false` で、`false` は書き出しでも省略する。
 * メッシュごとの有無は `Mesh::lit` (`docs/spec/rendering.md#ライティング`)。
 */
lighting?: boolean, };

/**
 * モデルのメタデータ。
 */
export type ModelMeta = { 
/**
 * モデルの名前。
 */
name: string, 
/**
 * キャンバス (モデル座標系の範囲)。
 */
canvas: Canvas, };

/**
 * 改変の許諾。
 */
export type Modification = "prohibited" | "allow" | "allow_redistribution";

/**
 * 違いの 1 行 (T0654)。足した・変えた要素ごとの中身の違いを添える (T0656)。
 */
export type ModuleDifference = { 
/**
 * 英語の 1 行 (例: `Meshes: 2 → 1 (removed: Ears; changed: Band)`)。
 */
text: string, 
/**
 * 行の要素のうち、足したもの・変えたもの・除いたもの (要素の行だけ)。
 */
items: Array<DifferenceItem>, };

/**
 * `.ugoemod` の書き出しの結果。
 */
export type ModuleExportReport = { 
/**
 * 書き出したファイルの大きさ (バイト)。
 */
bytes: number, 
/**
 * 書き出しはしたが知らせたいこと (コードは `module.*`)。
 */
warnings: Array<Message>, 
/**
 * アトラス化したときの結果 (書き出しの設定 `editor.export` の `atlas`)。しなければ `null`。
 */
atlas: AtlasReport | null, };

/**
 * モジュールのプロジェクトが本体の問題で開けない・読み直せないこと (`module_open_problem`・`reload_base`)。
 */
export type ModuleProblem = { kind: ModuleProblemKind, 
/**
 * 本体のファイルの絶対パス。
 */
base: string, 
/**
 * モジュールのスロット。
 */
slot: string, 
/**
 * 本体を読めたなら、そのスロットの ID (別のスロットを選ぶ)。
 */
base_slots: Array<string>, 
/**
 * `base_needs_physics` のとき、リグから外せば開ける、本体に無い衝突体への参照 (`open_module_with_base` の
 * `drop_colliders`。外すことを利用者に確かめてから開く。T0661)。外しても開けない・ほかの種類なら空。
 */
collider_references: Array<ColliderReference>, 
/**
 * 安定した識別子 (`project.not_a_project` など)。一覧は `docs/editor.md#エラーと警告のコード`。
 */
code: string, 
/**
 * 文に差し込む値。
 */
args: { [key in string]: MessageArg }, 
/**
 * 英語の文 (ログ・MCP・CLI・開発者向け。カタログに無いコードのときは画面もこれを出す)。
 */
message: string, 
/**
 * 原因になったエラー (`proposal.rejected` の中の `edit.*` など)。`message` は原因の文も含む。
 */
cause?: Message, 
/**
 * 一覧の中身 (検証のエラーの列 `validation.*` など)。並べるのは引数 `details` と同じく先頭の
 * `DESCRIBED_ERRORS_MAX` 件までで、全件の数は引数 `count`。画面は各項目を訳して `details` の代わりに差し込む。
 */
items?: Array<Message>, };

/**
 * モジュールのプロジェクトが本体の問題で開けない・読み直せないときの、問題の種類 (直し方を選ぶ)。
 */
export type ModuleProblemKind = "base_not_found" | "base_unreadable" | "unknown_slot" | "does_not_fit" | "base_needs_physics";

/**
 * モジュールが対応する本体の条件。
 */
export type ModuleRequirements = { 
/**
 * 本体に求めるアバタープロファイルのレベル (`Model::avatar` の `level` 以上)。`None` なら求めない。
 */
avatar_level: number | null, 
/**
 * 本体に必要なスロット (取り付け先のスロットは書かなくてよい)。
 */
slots: Array<string>, 
/**
 * 本体に必要なパラメータ。
 */
parameters: Array<string>, };

/**
 * モジュールのプロジェクトのリグの組み方 (AI タブで選ぶ。`ugoe_ai::module_rig::ModuleRigOptions`)。
 * AI には送らない。
 */
export type ModuleRigChoice = { 
/**
 * 本体のワープの変形を衣装のメッシュのキーフォームに直接写す (偽なら衣装を追従のワープで包む)。
 */
direct: boolean, 
/**
 * 揺れのリグを、スロットが公開する本体の衝突体に当てる。
 */
colliders: boolean, 
/**
 * 名前の末尾が A / B … で違うだけの衣装の差分をポーズにする。
 */
poses: boolean, };

/**
 * 開いているモジュールのプロジェクトの情報 (`module_status` の戻り値)。
 */
export type ModuleStatus = { 
/**
 * モジュールの名前 (合成で ID の接頭辞になる)。
 */
name: string, 
/**
 * 本体のファイルの絶対パス。
 */
base: string, slot: SlotSummary, 
/**
 * 作業用のモデルのうち本体の要素 (読み取り専用) の ID。
 */
base_parts: Array<string>, base_deformers: Array<string>, base_meshes: Array<string>, 
/**
 * 本体のテクスチャの名前 (作業用のモデルの `textures` の先頭から、この数だけが本体のもの)。
 */
base_textures: Array<string>, 
/**
 * 本体の物理のリグと衝突体の ID (読み取り専用)。
 */
base_rigs: Array<string>, base_colliders: Array<string>, 
/**
 * スロットが公開する本体の衝突体のうち、本体に無いもの (本体に物理が無いとき。モジュールのリグは当たれない。
 * 物理の画面で、本体に物理を足すよう案内する。`ModuleContext::unavailable_colliders`)。
 */
unavailable_colliders: Array<string>, 
/**
 * モジュールの条件 (`Module::requires`)。変えるのは `Command::SetModuleSettings`。
 */
requires: ModuleRequirements, 
/**
 * 本体のスロット (モジュールの設定の画面で、付けるスロットと条件のスロットを選ぶ)。
 */
base_slots: Array<SlotSummary>, 
/**
 * 本体のパラメータの ID (条件のパラメータを選ぶ)。
 */
base_parameters: Array<string>, 
/**
 * 本体が名乗るアバタープロファイルのレベル (なければ `null`)。
 */
base_avatar_level: number | null, 
/**
 * 本体の版 (`ModuleContext::base_revision`)。本体を読み込み直すと変わる。プレビューが、本体だけの `.ugoe`
 * (`get_preview_base_bundle`) をこれが変わったときだけ取り直す。
 */
base_revision: number, 
/**
 * 作業用のモデルのポーズのグループのうち、先頭からこの数が本体のもの (読み取り専用。モジュールのグループは
 * その後ろに並ぶ)。本体の要素の `links` には、スロットの `pose_parts` の要素にだけモジュールのパーツを加えられる。
 */
base_pose_groups: number, };

/**
 * モーション (`*.motion.json`)。パラメータ・パーツの不透明度などを時間で動かすカーブの集まり (`docs/spec/motion.md#モーション`)。
 */
export type Motion = { 
/**
 * モーションの名前 (表示・選択に使う)。
 */
name: string, 
/**
 * 秒。
 */
duration: number, 
/**
 * `duration` で折り返して繰り返すか。ループの境目ではフェードをかけない。
 */
looping: boolean, 
/**
 * フェードインの秒数。0 以下ならフェードインしない。
 */
fade_in: number, 
/**
 * フェードアウトの秒数。ループしないモーションは終わりのこの秒数前から、ループするものは止めてから
 * この秒数でフェードアウトする。0 以下ならフェードしない。
 */
fade_out: number, 
/**
 * 時間で値を動かすカーブ。
 */
curves: Array<Curve>, 
/**
 * 再生時刻が通過したフレームで発火するイベント。
 */
events: Array<MotionEvent>, };

/**
 * モーションの中の時刻に置くイベント。再生時刻がこの時刻を通過したフレームで発火する。
 */
export type MotionEvent = { 
/**
 * 発火する時刻 (秒)。
 */
t: number, 
/**
 * イベントの値 (アプリが解釈する任意の文字列)。
 */
value: string, };

/**
 * 名前の規則と AI の分類の食い違い (`ugoe_ai::classify::NameDisagreement`)。食い違う項目にだけ名前の規則の答えが入る。
 */
export type NameDisagreementView = { 
/**
 * 名前の規則のラベル。ラベルが同じなら `null`。
 */
label: string | null, 
/**
 * 名前の規則の左右 (`left`・`right`・`center`)。左右が同じなら `null`。
 */
side: string | null, };

/**
 * AI タブで選んだ、法線マップを提案する 1 つのまとまりと設定 (`propose_normal_maps` の引数)。
 */
export type NormalMapGroupChoice = { 
/**
 * まとまり (`NormalMapGroupView::group`。`null` はラベルのないテクスチャ)。
 */
group: string | null, 
/**
 * まとまり全体の設定 (単独のまとまりでは 1 枚ごとの設定)。
 */
settings: BulgeSettings, };

/**
 * テクスチャの法線マップを、どのまとまりで作るか (`get_normal_map_group` の戻り値)。
 */
export type NormalMapGroupView = { 
/**
 * まとまり (`face`・`body`・`hair_front` など。`ugoe_ai::normal::SurfaceGroup`)。ラベルがなければ `null` (単独)。
 */
group: string | null, 
/**
 * 法線マップを作るテクスチャ (このテクスチャを含む。`get_normal_map_groups` の単独のまとまり
 * (`alone` と `null`) では、単独でふくらませるすべてのテクスチャ)。
 */
textures: Array<string>, 
/**
 * 形にだけ加えるテクスチャ (体のまとまりの顔など)。
 */
shape_only: Array<string>, 
/**
 * `textures` のうち、形に加えず向きだけを受けるテクスチャ (顔のまとまりの眼鏡)。
 */
oriented: Array<string>, 
/**
 * `textures` のうち、すでに法線マップを持つテクスチャ。
 */
with_normal_map: Array<string>, 
/**
 * まとまりの既定の設定 (長さ `edge`・`smoothing` はまとまりの格子の画素)。
 */
defaults: BulgeSettings, 
/**
 * まとまりの格子の 1 画素がキャンバスの何ピクセルか。画面は縁の幅とならしをキャンバスのピクセルで見せ、
 * 送るときにこれで割って格子の画素に直す (T0522)。単独でふくらませるもの (`alone` と `null`) は、
 * テクスチャの画素が単位なので `null`。
 */
canvas_per_pixel: number | null, 
/**
 * ラベルを PSD の分類 (AI の提案のパネル) から取ったか。偽ならメッシュの名前の規則だけ。
 */
from_classification: boolean, };

/**
 * 正規化の写し先の範囲。パラメータの `min` / `default` / `max` をそれぞれここの値に写し、
 * その間は default の両側で別々に線形補間する。
 */
export type NormalizationRange = { 
/**
 * パラメータの `min` を写した値。
 */
min: number, 
/**
 * パラメータの `default` を写した値。
 */
default: number, 
/**
 * パラメータの `max` を写した値。
 */
max: number, };

/**
 * OpenAI 互換の API の設定。
 */
export type OpenAiSettings = { 
/**
 * `/chat/completions` の手前までの URL (例: `https://api.openai.com/v1`・`http://localhost:11434/v1`)。
 */
base_url: string, 
/**
 * モデルの ID。
 */
model: string, output: OutputMode, };

/**
 * OpenAI 互換の API で、答えを JSON にさせる方法 (`ugoe_ai::openai::StructuredOutput`)。
 */
export type OutputMode = "json_schema" | "json_object" | "prompt";

/**
 * [`verify_overflows`] で見つけた、ある要素のあるセルでのはみ出し。
 */
export type OverflowFinding = { target: KeyformTarget, parent: string, kind: OverflowPointKind, 
/**
 * セル: 要素の格子 (とブレンドシェイプ格子) の軸のパラメータの値。ほかのパラメータは値に依らない。
 */
cell: { [key in string]: number }, 
/**
 * はみ出した点の数。
 */
count: number, 
/**
 * 最もはみ出した点の番号と量。
 */
worst_index: number, max_amount: number, };

/**
 * はみ出した 1 点。
 */
export type OverflowPoint = { index: number, 
/**
 * 親のワープの UV。
 */
uv: Vec2, 
/**
 * [0,1]² からの UV の距離 (格子の幅・高さを 1 とした量)。
 */
amount: number, };

/**
 * はみ出した点の種類。
 */
export type OverflowPointKind = "vertex" | "control_point" | "origin";

/**
 * [`verify_overflows`] の結果。
 */
export type OverflowReport = { 
/**
 * 見つけたはみ出し。要素のモデルでの順 (デフォーマ、メッシュ)、同じ要素ではセルの順。
 */
findings: Array<OverflowFinding>, 
/**
 * 調べた要素 (親がワープの要素) の数。
 */
elements: number, 
/**
 * 調べたセルの数 (要素ごとの組み合わせの和)。
 */
cells: number, 
/**
 * 組み合わせが [`MAX_COMBINATIONS`] を超え、各軸の両端のキーだけを調べた要素。
 */
truncated: Array<KeyformTarget>, };

/**
 * モデルを動かす入力スカラー値 (例: 顔の角度 X, 目の開閉)。
 */
export type Parameter = { 
/**
 * モデルの中で一意な ID。モーション・キーフォームなどはこの ID で参照する。
 */
id: string, 
/**
 * 表示名 (エディタ用)。
 */
name: string, 
/**
 * 値の下限。値はこの範囲に切り詰める。
 */
min: number, 
/**
 * 値の上限 (`min` 以上)。
 */
max: number, 
/**
 * 既定値 (`min..=max` の中)。
 */
default: number, 
/**
 * ブレンドシェイプ用パラメータか (加算変形。`docs/spec/model.md`)。
 */
kind: ParameterKind, 
/**
 * 左右ペア (例: EyeLOpen ↔ EyeROpen)。エディタのミラー機能・AI 自動設定で使う。
 */
mirror: string | null, 
/**
 * 所属する `ParameterGroup` の ID。
 */
group: string | null, 
/**
 * 中継用 (内部) のパラメータか。物理のリグの間で値を受け渡すためのもので
 * (`docs/spec/physics.md#リグの順序と中継`)、トラッキングは書き込まず、
 * ビューアなどのアプリの一覧では既定で隠す (`docs/spec/model.md#パラメータ`)。省略時は `false`。
 */
internal?: boolean, };

/**
 * エディタ表示用のパラメータグループ。ランタイムの挙動には影響しない。
 */
export type ParameterGroup = { 
/**
 * モデルの中で一意な ID。
 */
id: string, 
/**
 * 表示名。
 */
name: string, };

/**
 * パラメータの種類 (`docs/spec/model.md#ブレンドシェイプ`)。
 */
export type ParameterKind = "normal" | "blend_shape";

/**
 * パーツ: メッシュ・デフォーマをまとめる論理グループ。不透明度は親子で乗算される。
 */
export type Part = { 
/**
 * モデルの中で一意な ID。
 */
id: string, 
/**
 * 表示名 (エディタ用)。
 */
name: string, 
/**
 * 親のパーツ。`None` なら最上位。
 */
parent: string | null, 
/**
 * 不透明度のキーフォーム。
 */
keyforms: KeyformGrid<PartKeyform>, };

/**
 * パーツの 1 つのキーフォーム。
 */
export type PartKeyform = { 
/**
 * 不透明度 (0〜1)。子のパーツ・デフォーマ・メッシュに乗算される。
 */
opacity: number, };

/**
 * 物理の出力をパラメータへ合成する方法 (`docs/spec/physics.md#合成の方法`)。
 */
export type PhysicsBlend = "overwrite" | "add";

/**
 * 衝突体 (`docs/spec/physics.md#衝突体`)。
 */
export type PhysicsCollider = { 
/**
 * 衝突体の ID。リグは [`PhysicsRig::colliders`] でこの ID を選ぶ。
 */
id: string, 
/**
 * 表示名 (エディタ用。評価には使わない)。
 */
name: string, 
/**
 * 取り付け先のデフォーマ。形状の位置はこのデフォーマの局所座標系で表し、
 * そのワールド変換で動く。`None` ならキャンバスに固定する。
 */
attach?: string | null, 
/**
 * 形状。位置は `attach` のデフォーマの局所座標系 (無ければキャンバス座標) で表す。
 */
shape: ColliderShape, };

/**
 * 物理の入力。パラメータの値を正規化し、重みを掛けて根元の姿勢に足す (`docs/spec/physics.md#入力`)。
 */
export type PhysicsInput = { 
/**
 * 入力元のパラメータ。存在しなければその入力を無視する。
 */
parameter: string, 
/**
 * パラメータの値を根元のどの量 (平行移動の x / y、回転) に足すか。
 */
source: PhysicsSource, 
/**
 * 0..=100 (%)。
 */
weight: number, 
/**
 * 正規化した値の符号を反転するか。
 */
reflect: boolean, };

/**
 * 入力のパラメータの値を物理空間の量に写す範囲 (`docs/spec/physics.md#入力`)。
 */
export type PhysicsNormalization = { 
/**
 * 平行移動 (`source` が `x` / `y` の入力) の範囲。
 */
position: NormalizationRange, 
/**
 * 角度 (`source` が `angle` の入力) の範囲 (度)。
 */
angle: NormalizationRange, };

/**
 * 物理の出力。
 *
 * 旧形式 (出力先が `parameter` フィールドのパラメータだけ) も読める。読み込みは手で書く
 * ([`PhysicsOutput::deserialize`])。書き出しは常に `target` を使う。
 */
export type PhysicsOutput = { 
/**
 * 出力先 (`docs/spec/physics.md#出力先`)。
 */
target: PhysicsTarget, 
/**
 * 出力元となる粒子インデックス (その粒子と 1 つ前の粒子の成す角を使う)。
 */
particle: number, 
/**
 * 粒子のどの量 (相対位置の x / y、区間の角度) を出力するか。
 */
source: PhysicsSource, 
/**
 * 出力する値に掛ける倍率。
 */
scale: number, 
/**
 * 0..=100 (%)。パラメータへは現在の値とのブレンドの割合、動的物理では加算量に掛ける割合。
 * 0〜1 にクランプして使う。
 */
weight: number, 
/**
 * 出力する値の符号を反転するか。
 */
reflect: boolean, 
/**
 * パラメータへの合成の方法 (`docs/spec/physics.md#合成の方法`)。省略時は `overwrite`。
 * パラメータ以外の出力先では使わない (常に加算量として足す)。
 */
blend?: PhysicsBlend, };

/**
 * 振り子チェーンの粒子 1 つ。係数の意味は `docs/spec/physics.md#係数の意味`。
 */
export type PhysicsParticle = { 
/**
 * 初期位置 (根元からの相対)。
 */
position: Vec2, 
/**
 * 基準フレーム (1/30 秒) あたりに残す速度の割合 (0..=1)。1 で減衰なし、小さいほど揺れがすぐ止まる。
 */
mobility: number, 
/**
 * 時間の進み方の倍率。1 で等速、大きいほど入力に機敏に反応して速く揺れる。
 */
delay: number, 
/**
 * 重力の倍率。大きいほど元の向きに戻ろうとする力が強く、揺れが速く収まる。風には掛からない。
 */
acceleration: number, 
/**
 * 1 つ前の粒子との距離 (区間の長さ)。衝突には使わない。
 */
radius: number, 
/**
 * 衝突用の半径。衝突体からこの半径を含めて押し出す。省略時は 0。
 */
collision_radius: number, };

/**
 * 物理リグ。入力 → 振り子チェーン → 出力 の 1 組 (`docs/spec/physics.md#概要`)。
 */
export type PhysicsRig = { 
/**
 * リグの ID。
 */
id: string, 
/**
 * 表示名 (エディタ用。評価には使わない)。
 */
name: string, 
/**
 * チェーンの根元の平行移動と回転を作る入力。
 */
inputs: Array<PhysicsInput>, 
/**
 * 粒子の位置・角度を書き込む出力。
 */
outputs: Array<PhysicsOutput>, 
/**
 * 振り子チェーンの粒子。先頭 (0 番) が根元で、根元から順に並ぶ。
 */
particles: Array<PhysicsParticle>, 
/**
 * 入力のパラメータの値を物理空間の位置・角度に写す範囲。
 */
normalization: PhysicsNormalization, 
/**
 * このリグの粒子と衝突する衝突体 (`PhysicsCollider::id`)。省略時はなし。
 */
colliders: Array<string>, 
/**
 * 重力と風を解釈する座標系の基準のデフォーマ。`None` ならキャンバスの座標系
 * (`docs/spec/physics.md#重力と風の基準`)。
 */
frame?: string | null, };

/**
 * モデル 1 体分の物理の設定 (`*.physics.json`)。全リグに共通の重力・風・ステップの周波数と、リグ・衝突体の一覧。
 */
export type PhysicsSettings = { 
/**
 * 重力のベクトル。物理空間は y 軸下向きで、既定は `(0, 1)` (真下)。
 * リグの `acceleration` を掛けて使う。
 */
gravity: Vec2, 
/**
 * 風のベクトル。重力と違い `acceleration` は掛からず、入力の角度でも回さない。
 */
wind: Vec2, 
/**
 * 固定ステップ実行の周波数 (Hz)。
 */
fps: number, 
/**
 * 物理リグ。この並びの順に処理し、前のリグがパラメータに書いた出力を後ろのリグの入力が読める
 * (`docs/spec/physics.md#リグの順序と中継`)。
 */
rigs: Array<PhysicsRig>, 
/**
 * 衝突体。どのリグと衝突するかは `PhysicsRig::colliders` で選ぶ
 * (`docs/spec/physics.md#衝突体`)。省略時はなし。
 */
colliders: Array<PhysicsCollider>, };

/**
 * 物理の入力・出力で使う量。
 */
export type PhysicsSource = "x" | "y" | "angle";

/**
 * 物理の出力先。JSON では `{"type": "deformer_angle", "deformer": "HairFront1"}` と書く。
 * パラメータ以外の出力先は動的物理 (`docs/spec/physics.md#動的物理-揺れ用パラメータのいらない物理`)。
 */
export type PhysicsTarget = { "type": "parameter", 
/**
 * 書き込む先のパラメータ。
 */
id: string, } | { "type": "deformer_angle", 
/**
 * 角度を足す回転デフォーマ。
 */
deformer: string, } | { "type": "deformer_offset", 
/**
 * 原点をずらす回転デフォーマ。
 */
deformer: string, } | { "type": "vertices", 
/**
 * 動かす頂点の持ち主 (メッシュまたはワープデフォーマ)。
 */
owner: VertexOwner, 
/**
 * 頂点 (制御点) ごとの重み (0..=1)。並びと長さは、メッシュなら `Mesh::uvs`、
 * ワープなら `WarpKeyform::points` と同じ。
 */
weights: Array<number>, };

/**
 * 物理のテンプレートの種類。
 */
export type PhysicsTemplate = "body_follow" | "bounce" | "neck_follow";

/**
 * [`Command::ApplyPhysicsTemplate`] の選択。
 */
export type PhysicsTemplateOptions = { 
/**
 * モデルにない標準パラメータを、標準の値域で作るか。`false` なら、それを使うリグを作らない。
 * 中継パラメータ (`BodyBounce`) はこの選択によらず作る。
 */
create_missing: boolean, 
/**
 * 体の回転に書き込む合成の方法。`add` (既定) はトラッキング・モーションの値に遅れと行き過ぎを足し、
 * `overwrite` は物理の値で置き換える (体の回転を書かないホスト向け)。
 */
blend: PhysicsBlend, 
/**
 * 首の追従で傾ける回転デフォーマ。省略すると、ID か名前に `neck` (大文字小文字を問わない) か
 * `首` を含む最初の回転デフォーマ。
 */
neck?: string | null, };

/**
 * 点 (time, value)。
 */
export type Point = { 
/**
 * 時刻 (秒)。
 */
t: number, 
/**
 * その時刻での対象の値 (パラメータの値・不透明度など)。
 */
v: number, };

/**
 * ポーズ設定。グループごとに、表示中のパーツを 1 つに保つ。
 *
 * `groups` が空ならポーズは何もしない (`Default` はこの状態)。
 */
export type Pose = { 
/**
 * 表示パーツを切り替えるときのクロスフェード時間 (秒)。0 以上。
 */
fade_duration: number, 
/**
 * 排他的なパーツの組。
 */
groups: Array<PoseGroup>, };

/**
 * ポーズグループの 1 要素。
 */
export type PoseEntry = { 
/**
 * グループの要素となるパーツ。
 */
part: string, 
/**
 * `part` と同じ不透明度に追従するパーツ (例: 腕に付属する袖)。
 */
links: Array<string>, };

/**
 * 互いに排他的なパーツの組 (例: 腕の差分 A / B)。
 */
export type PoseGroup = { 
/**
 * 組の要素。最初は先頭の要素を表示する (`docs/spec/motion.md#ポーズ`)。
 */
entries: Array<PoseEntry>, };

/**
 * プレビューの光源 1 つ (`docs/spec/rendering.md#ライティング`)。
 */
export type PreviewLight = { "type": "directional", direction: [number, number, number], color: [number, number, number], intensity: number, } | { "type": "point", position: [number, number, number], color: [number, number, number], intensity: number, range: number | null, };

/**
 * プレビューの光源 (`project.json` の `editor.lighting`)。エディタのビューポートとプレビューで
 * `WebPuppet::set_lighting_json` に渡す。モデルには入らない。すべて既定値なら書かない。
 *
 * 座標はキャンバス座標 (ピクセル・Y 下向き) + 奥行き (手前が +z)。色は 0〜1 で、強さ `intensity` を掛けて渡す。
 * `project.json` に書く値が丸めの誤差で読みにくくならないよう、`f64` で持つ。
 */
export type PreviewLighting = { 
/**
 * ビューポートに光源を当てるか。`false` ならホストは光源を渡さない (環境光が白で光源なし)。
 */
enabled: boolean, 
/**
 * 環境光の色と強さ。
 */
ambient: [number, number, number], ambient_intensity: number, 
/**
 * 光源 (最大 [`MAX_PREVIEW_LIGHTS`] 個)。
 */
lights: Array<PreviewLight>, };

/**
 * プレビューで試すモジュール 1 つ (`editor.dress_up.modules` の要素)。
 */
export type PreviewModule = { 
/**
 * `.ugoemod` のパス (IPC では絶対パス)。
 */
path: string, 
/**
 * プレビューで付けるか。
 */
attached: boolean, };

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
 * 提案の 1 つのコマンドの要約 (画面の一覧に出す)。
 */
export type ProposalCommand = { 
/**
 * コマンドの種類 (`op`。例: `add_parameter`)。
 */
op: string, 
/**
 * 対象の ID や名前 (例: `AngleX`、``deformer `WarpHead` ``)。分からなければ `null`。
 */
subject: string | null, 
/**
 * このコマンドが頼る、前のコマンドの番号 (提案の中の 0 始まり。使う要素を作る・同じ要素を先に変える。
 * 規則は [`analyze`])。これを外すと、このコマンドは適用できないか、結果が変わる。
 */
requires: Array<number>, };

/**
 * 提案を複製に適用した姿 (`preview_proposal` の戻り値)。
 */
export type ProposalPreview = { 
/**
 * 適用した後のモデルの JSON (`get_model_json` と同じ形)。テクスチャ付きのバンドルは
 * `get_proposal_preview_bundle` で受け取る。
 */
model_json: string, report: ApplyReport, };

/**
 * 提案 1 つのコマンドの区切り。
 */
export type ProposalSections = { id: number, sections: Array<CommandSection>, };

/**
 * 提案のパネルの状態。
 */
export type ProposalState = { 
/**
 * 採用を待っている提案 (古い順)。
 */
proposals: Array<ProposalView>, 
/**
 * 提案の元にする PSD のパス。なければ `null` (分類するときに選ぶ)。
 */
psd: string | null, 
/**
 * 最後の分類の結果。
 */
classification: Classification | null, 
/**
 * 注文を添えて作り直せる提案の番号 (このパネルで作ったリグの提案。つないだエージェントの提案は含まない)。
 */
regenerable: Array<number>, 
/**
 * コマンドを区切って出す提案 (法線マップの提案)。ない提案は、同じ対象への続いたコマンドをまとめて出す。
 */
sections: Array<ProposalSections>, 
/**
 * 採用したモジュールのリグの提案が本体の変形を写した衣装のメッシュのうち、その後に形 (頂点・三角形・親) を
 * 編集したもの。写した変形が合わないので、提案し直すよう知らせる。
 */
stale_rigs: Array<StaleRig>, };

/**
 * 画面に出す提案。
 */
export type ProposalView = { id: number, title: string, 
/**
 * 提案の理由 (複数行)。
 */
rationale: string, commands: Array<ProposalCommand>, 
/**
 * 提案の後にドキュメントが変わった (プレビュー・採用できない。Undo で戻れば使える)。
 */
stale: boolean, 
/**
 * 提案を作ったもの (エディタにつないだエージェントの名前など。[`Proposals::set_source`])。
 * エディタの中で作った提案は `null`。
 */
source: string | null, 
/**
 * この提案が作り直した元の提案の番号 ([`ProposalOrigin::revises`])。
 */
revises: number | null, 
/**
 * 提案を作るときに AI に添えた利用者の注文 ([`ProposalOrigin::instructions`])。
 */
instructions: string | null, 
/**
 * 今の仕組みでは反映できなかった注文 (AI の答えの言葉のまま。[`ProposalOrigin::unapplied`])。
 */
unapplied: Array<string>, };

/**
 * 使うプロバイダ。
 */
export type ProviderKind = "claude" | "openai_compatible";

/**
 * AI のプロバイダの設定 (`ai-provider.json`)。API キーは含めない。
 */
export type ProviderSettings = { provider: ProviderKind, claude: ClaudeSettings, openai: OpenAiSettings, 
/**
 * 1 回の提案 (分類・リグの提案・接続の確認) にかける時間の合計の上限 (分)。再試行の待ちと問い直しを含む。
 * 超えたら止めて `ai.time_limit` のエラーにする。範囲は [`TIME_LIMIT_RANGE_MINUTES`]。
 */
time_limit_minutes: number, };

/**
 * 取り込みの警告。
 */
export type PsdImportWarning = { 
/**
 * レイヤーの名前の列 (外側のグループから順に、最後がレイヤー自身)。
 * ドキュメント全体についての警告では空。
 */
path: Array<string>, 
/**
 * 安定した識別子 (`psd.*`。`docs/editor.md#エラーと警告のコード`)。
 */
code: string, 
/**
 * 文に差し込む値。
 */
args: { [key in string]: MessageArg }, 
/**
 * 英語の文 (レイヤーのパスを含まない)。
 */
message: string, };

/**
 * 自動の作り直しで失敗したが、コマンドは拒否せずに続けたこと (`docs/editor.md#推定とミラー`)。
 *
 * ミラーのやり直しの失敗 (元の頂点が許容距離を超えて動いた、対を外した、など) を報告する。
 * そのとき `target` の格子の `mirror` のセルは前の値のまま残る。
 * 対の片側の親を変えた (付け替えた、デフォーマで包んだ) 後に、その対がミラーし直せなくなった
 * ことも報告する。このときセルは付け替えで写し直した値のまま。
 */
export type RederiveWarning = { 
/**
 * ミラーのセルを作り直せなかった格子の要素。
 */
target: KeyformTarget, 
/**
 * ミラー元 (`mirror_source`) の要素。
 */
source: KeyformTarget, 
/**
 * 安定した識別子 (いまは `inference.mirror_failed` だけ。`docs/editor.md#エラーと警告のコード`)。
 */
code: string, 
/**
 * 文に差し込む値 (`target`・`source`・`detail`)。
 */
args: { [key in string]: MessageArg }, 
/**
 * 失敗の理由 (`ugoe_rigging::InferError` の表記)。
 */
message: string, 
/**
 * 失敗の理由のコードと引数 (`infer.*`)。画面はこれを訳して `detail` の代わりに使う (T0163)。
 */
cause?: Message, };

/**
 * 衝突体を考えない静止姿勢で、リグの鎖が衝突体に当たっている (`docs/spec/physics.md#静止姿勢で衝突体に入っている場合`)。
 *
 * 最初の `update` で、髪は真下に垂れずに衝突体の手前に止まり、衝突体に沿って置かれる。
 * 作者が意図しない姿勢になりやすいので警告にする (エラーではない)。
 */
export type RestPoseCollision = { 
/**
 * リグの ID。
 */
rig: string, 
/**
 * 衝突体の ID。
 */
collider: string, 
/**
 * 当たっている区間の、子の粒子の添字 (昇順。区間は粒子 `i - 1` → `i`)。
 */
particles: Array<number>, };

/**
 * リニアではなく sRGB 値の RGB (0.0..=1.0)。乗算色・スクリーン色に使う。
 */
export type Rgb = { 
/**
 * 赤 (0.0..=1.0)。
 */
r: number, 
/**
 * 緑 (0.0..=1.0)。
 */
g: number, 
/**
 * 青 (0.0..=1.0)。
 */
b: number, };

/**
 * 回転デフォーマ: 原点まわりの回転・拡縮・反転。
 * 子の局所座標は原点を (0,0) とした回転前の座標 (単位はキャンバスピクセル)。
 */
export type RotationDeformer = { 
/**
 * 原点・角度・拡大率・反転・不透明度のキーフォーム。
 */
keyforms: KeyformGrid<RotationKeyform>, };

/**
 * 回転デフォーマの 1 つのキーフォーム。変換の式は `docs/spec/evaluation.md`。
 */
export type RotationKeyform = { 
/**
 * 親座標系での原点。
 */
origin: Vec2, 
/**
 * 度数法、時計回り正 (Y 下向き座標系)。
 */
angle: number, 
/**
 * 拡大率 (1 で等倍)。
 */
scale: number, 
/**
 * `true` なら局所座標の x を反転する。
 */
reflect_x: boolean, 
/**
 * `true` なら局所座標の y を反転する。
 */
reflect_y: boolean, 
/**
 * 子に掛ける不透明度 (0〜1)。
 */
opacity: number, };

/**
 * 連続するセグメント列。各セグメントの始点は直前セグメントの終点。
 */
export type Segment = { "type": "start", 
/**
 * 最初の点。
 */
p: Point, } | { "type": "linear", 
/**
 * 区間の終点。
 */
to: Point, } | { "type": "bezier", 
/**
 * 始点側の制御点。
 */
c1: Point, 
/**
 * 終点側の制御点。
 */
c2: Point, 
/**
 * 区間の終点。
 */
to: Point, } | { "type": "stepped", 
/**
 * 区間の終点。
 */
to: Point, } | { "type": "inverse_stepped", 
/**
 * 区間の終点。
 */
to: Point, };

/**
 * [`crate::Command::SetMotionSegment`] で設定する区間の種類。
 */
export type SegmentShape = { "type": "linear" } | { "type": "bezier", c1?: Point | null, c2?: Point | null, } | { "type": "stepped" } | { "type": "inverse_stepped" };

/**
 * 同梱の `ugoe-mcp` (登録のコマンドに埋める)。
 */
export type Sidecar = { 
/**
 * エディタの実行ファイルの隣の `ugoe-mcp` (`.exe`)。開発中は同じ `target/` のビルド結果。
 * 見つからなければ `null` (画面はビルドの方法を案内する)。
 */
path: string | null, 
/**
 * Windows で、`path` を WSL から指すパス (`/mnt/c/...`)。
 */
wsl_path: string | null, 
/**
 * `windows`・`macos`・`linux`。
 */
platform: string, 
/**
 * エディタ自身が WSL の中で動いている (Linux 版を WSLg で動かしている)。
 */
in_wsl: boolean, };

/**
 * 着せ替えのスロット: モジュールの取り付け位置 (`docs/spec/modules.md#スロット`)。
 */
export type Slot = { 
/**
 * スロットの名前 (例: `Hair`)。モジュールはこの名前だけで取り付け先を指定する。
 */
id: string, 
/**
 * 取り付け先のデフォーマ。`None` ならキャンバス。
 * モジュールの最上位の要素の座標は、このデフォーマの局所座標系で表す。
 */
anchor?: string | null, 
/**
 * このスロットに入るメッシュの描画順の範囲。モジュールの描画順 (0〜1000) をこの範囲に写す。
 */
draw_order: DrawOrderRange, 
/**
 * モジュールを取り付けたときに非表示にする本体のパーツ。
 */
replaces?: Array<string>, 
/**
 * このスロットのモジュールが使ってよい本体のパラメータ。標準パラメータは常に使える。
 */
parameters?: Array<string>, 
/**
 * このスロットのモジュールがマスクとして参照してよい本体のメッシュ。
 */
mask_sources?: Array<string>, 
/**
 * モジュールの最上位のパーツの親にする本体のパーツ。`None` ならモジュールの最上位のパーツは最上位のまま。
 * 親のパーツの不透明度 (ポーズやモーションで隠したときなど) にモジュールが追従する。
 */
part?: string | null, 
/**
 * このスロットのモジュールの物理のリグが衝突してよい本体の衝突体 (`PhysicsCollider::id`)。
 */
colliders?: Array<string>, 
/**
 * このスロットのモジュールがパーツを追従 (`PoseEntry::links`) させてよい、本体のポーズの要素のパーツ。
 */
pose_parts?: Array<string>, };

/**
 * 本体のスロット 1 つ (新規作成の画面で選ぶ)。
 */
export type SlotSummary = { id: string, 
/**
 * 取り付け先のデフォーマ。`null` ならキャンバス。
 */
anchor: string | null, 
/**
 * モジュールの最上位のパーツの親になる本体のパーツ。
 */
part: string | null, 
/**
 * 描画順の範囲。
 */
draw_order_min: number, draw_order_max: number, 
/**
 * モジュールを付けると隠れる本体のパーツ。
 */
replaces: Array<string>, 
/**
 * モジュールが使える本体のパラメータ (標準パラメータのほかに)。
 */
parameters: Array<string>, 
/**
 * モジュールのパーツを追従させてよい本体のポーズの要素のパーツ (`Slot::pose_parts`)。
 */
pose_parts: Array<string>, };

/**
 * 本体の変形を写した後に形を編集した衣装のメッシュ。
 */
export type StaleRig = { mesh: string, name: string, };

/**
 * テクスチャ参照。実データはコンテナ (`ugoe-format`) 側が保持する。
 */
export type TextureRef = { 
/**
 * テクスチャの画像の名前。コンテナの中の画像を指す。
 */
name: string, 
/**
 * 画像の幅 (ピクセル)。
 */
width: number, 
/**
 * 画像の高さ (ピクセル)。
 */
height: number, 
/**
 * 法線マップの画像の名前。`None` なら法線マップなし (法線は `(0, 0, 1)`)。
 * 画像は同じ UV 空間の接空間法線で、バンドルでは `NRML` チャンクに入る
 * (`docs/spec/rendering.md#ライティング`、`docs/spec/file-formats.md`)。
 */
normal_map?: string | null, };

/**
 * トラッキングの対応の上書き 1 つ (`docs/spec/avatar-profile.md#トラッキングの対応の上書き`)。
 *
 * 入力の値 `v` から `v * scale + offset` を `parameter` に書き込む (値域へのクランプはその後)。
 */
export type TrackingOverride = { 
/**
 * 入力の種類。`input` の名前の解釈を決める。
 */
source: TrackingSource, 
/**
 * 入力の名前。`source` によって、標準の対応表のパラメータ ID ([`TRACKED_PARAMETERS`])、
 * ARKit のブレンドシェイプ名 ([`ARKIT_BLEND_SHAPES`])、頭の回転 ([`HEAD_INPUTS`]) のいずれか。
 */
input: string, 
/**
 * 書き込むパラメータ。`source` が `standard` なら省略でき、省略すると `input` と同じ ID。
 */
parameter?: string | null, 
/**
 * 入力の値に掛ける倍率。既定は 1。
 */
scale: number, 
/**
 * 倍率を掛けた後に足す値。既定は 0。
 */
offset: number, };

/**
 * トラッキングの入力の種類。
 */
export type TrackingSource = "standard" | "blend_shape" | "head";

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
 * 頂点への出力の対象。JSON では `{"mesh": "Skirt"}` または `{"warp": "SkirtWarp"}` と書く。
 *
 * 外部タグの表現では `#[serde(other)]` が使えないので、読み込みは手で書く
 * ([`VertexOwner::deserialize`])。
 */
export type VertexOwner = { "mesh": string } | { "warp": string };

/**
 * ワープデフォーマ: (rows+1)×(cols+1) 個の制御点を持つ格子による自由変形。
 * 子の局所座標は格子全体を [0,1]² に正規化した UV 空間。
 */
export type WarpDeformer = { 
/**
 * 格子の行数 (縦の分割数)。制御点は縦に `rows + 1` 個。
 */
rows: number, 
/**
 * 格子の列数 (横の分割数)。制御点は横に `cols + 1` 個。
 */
cols: number, 
/**
 * 制御点と不透明度のキーフォーム。
 */
keyforms: KeyformGrid<WarpKeyform>, 
/**
 * 制御点ごとの深度 (キャンバスのピクセル単位、手前が正。オーサリング用)。
 * 並びは `WarpKeyform::points` と同じで、長さ `(rows+1)*(cols+1)`。
 */
depth?: Array<number> | null, };

/**
 * ワープデフォーマの 1 つのキーフォーム。
 */
export type WarpKeyform = { 
/**
 * 行優先 (x が速い) の制御点。長さ `(rows+1)*(cols+1)`。親座標系。
 */
points: Array<Vec2>, 
/**
 * 子に掛ける不透明度 (0〜1)。
 */
opacity: number, };
