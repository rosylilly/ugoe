use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ugoe_core::Vec2;
use ugoe_model::avatar::{AvatarHitArea, AvatarMeta, AvatarProfile, LookAt, TrackingOverride};
use ugoe_model::deformer::{RotationKeyform, WarpKeyform};
use ugoe_model::effects::Effects;
use ugoe_model::expression::{Expression, ExpressionParameter};
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::motion::{Curve, CurveTarget, Motion, MotionEvent, Point};
use ugoe_model::part::PartKeyform;
use ugoe_model::physics::{
    PhysicsCollider, PhysicsInput, PhysicsNormalization, PhysicsOutput, PhysicsParticle, PhysicsRig,
};
use ugoe_model::{
    BlendMode, Deformer, DeformerId, HitArea, KeyformGrid, KeyformInference, KeyformInterpolation,
    MaskMode, MaskRef, Mesh, MeshId, ParamId, Parameter, Part, PartId,
};

use crate::automesh::AutoMeshOptions;
use crate::depth::DepthPreset;
use crate::keyform::Keyform;
use crate::motion_edit::SegmentShape;
use crate::physics_template::{PhysicsTemplate, PhysicsTemplateOptions};

/// キーフォーム格子の 1 セルの指定: 軸のパラメータ ID → その軸のキーの値。
///
/// 格子のすべての軸を 1 回ずつ指定し、値は `KeyformBinding::keys` のどれかと完全に一致させる。
/// 軸が 0 本の格子 (定数) は空の指定で唯一のセルを指す。
pub type KeyformCell = BTreeMap<ParamId, f32>;

/// 編集操作。JSON で表現でき、AI エージェントのツール呼び出しにそのまま対応する。
///
/// 追加時のルール (`docs/editor.md#コマンド`):
/// - 1 コマンド = ユーザーから見た 1 操作 (Undo の単位)。
/// - フィールドは ID で対象を指定する (インデックスは使わない)。
/// - 適用前検証を [`crate::Document::apply`] 内で行い、失敗時はモデルを一切変更しない。
/// - 新しいバリアントは末尾に追加する。
/// - TypeScript の型と JSON Schema は Rust から生成する。変えたら `just gen` を実行する
///   (`docs/editor.md#型の生成`)。
///
/// 削除時の連鎖 (カスケード) と拒否の規則は `docs/editor.md#コマンド` にある。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
#[schemars(
    description = "エディタの編集操作。`op` で種類を選ぶ。1 コマンドが Undo の 1 単位で、対象は ID で指定する。"
)]
pub enum Command {
    AddParameter {
        parameter: Parameter,
    },
    /// パラメータを削除する。このパラメータを軸に持つキーフォーム格子は、その軸を
    /// 既定値のキー (なければ最も近いキー) の断面に畳む。ミラーとエフェクトの参照も外す。
    RemoveParameter {
        id: ParamId,
    },
    SetParameterRange {
        id: ParamId,
        min: f32,
        default: f32,
        max: f32,
    },
    /// 複数コマンドを 1 つの Undo 単位にまとめる (AI の一括提案など)。
    Batch {
        label: String,
        commands: Vec<Command>,
    },
    /// パラメータの表示名を変える (ID は変えない)。
    RenameParameter {
        id: ParamId,
        name: String,
    },
    /// パラメータグループを設定する (`None` で外す)。グループは `parameter_groups` に存在すること。
    SetParameterGroup {
        id: ParamId,
        group: Option<String>,
    },
    /// 左右で対になるパラメータを設定する (`None` で外す)。対は双方向に張り、`id` と `mirror` の
    /// どちらかと対になっていたパラメータの対は外す (`SetMirrorPair` と同じ)。
    /// 自分自身を指定すると、値の符号を反転して対応させる (`docs/spec/keyform-inference.md#ミラー`)。
    SetParameterMirror {
        id: ParamId,
        mirror: Option<ParamId>,
    },
    AddPart {
        part: Part,
    },
    /// パーツを削除する。子パーツ・所属するデフォーマ・メッシュが残っていれば拒否する。
    /// ポーズからは取り除く (空になったグループも消す)。
    RemovePart {
        id: PartId,
    },
    RenamePart {
        id: PartId,
        name: String,
    },
    SetPartParent {
        id: PartId,
        parent: Option<PartId>,
    },
    SetPartKeyforms {
        id: PartId,
        keyforms: KeyformGrid<PartKeyform>,
    },
    AddDeformer {
        deformer: Deformer,
    },
    /// デフォーマを削除する。子 (デフォーマ・メッシュ) が残っていれば拒否する。
    RemoveDeformer {
        id: DeformerId,
    },
    RenameDeformer {
        id: DeformerId,
        name: String,
    },
    /// 親デフォーマを付け替える。子の座標はそのまま (新しい親の局所座標として解釈される)。
    SetDeformerParent {
        id: DeformerId,
        parent: Option<DeformerId>,
    },
    SetDeformerPart {
        id: DeformerId,
        part: PartId,
    },
    /// ワープデフォーマのキーフォーム格子を置き換える (`rows` / `cols` は変えない)。
    SetWarpKeyforms {
        id: DeformerId,
        keyforms: KeyformGrid<WarpKeyform>,
    },
    SetRotationKeyforms {
        id: DeformerId,
        keyforms: KeyformGrid<RotationKeyform>,
    },
    /// メッシュを `meshes` の末尾 (同じ描画順の中で最も手前) に追加する。
    AddMesh {
        mesh: Mesh,
    },
    /// メッシュを削除する。他のメッシュのマスク参照から外し、
    /// このメッシュを使うグルーと当たり判定を削除する。
    RemoveMesh {
        id: MeshId,
    },
    RenameMesh {
        id: MeshId,
        name: String,
    },
    /// メッシュの属性を変える。省略した (`null` の) フィールドは変更しない。
    SetMeshProperties {
        id: MeshId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        part: Option<PartId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        texture: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blend: Option<BlendMode>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        culling: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        masks: Option<Vec<MaskRef>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mask_mode: Option<MaskMode>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mask_only: Option<bool>,
        /// ライティングを受けるか (`Mesh::lit`、`docs/editor.md#ライティングの画面`)。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lit: Option<bool>,
    },
    /// 親デフォーマを付け替える。頂点座標はそのまま (新しい親の局所座標として解釈される)。
    SetMeshParent {
        id: MeshId,
        parent: Option<DeformerId>,
    },
    SetMeshKeyforms {
        id: MeshId,
        keyforms: KeyformGrid<MeshKeyform>,
    },
    /// メッシュのキーフォーム格子の 1 セルだけを置き換える。
    SetMeshKeyform {
        id: MeshId,
        cell: KeyformCell,
        keyform: MeshKeyform,
    },
    /// `meshes` の中でメッシュを `before` の直前へ移す (`None` なら末尾)。
    /// 描画順 (`draw_order`) が同じメッシュどうしの前後関係が変わる。
    ReorderMesh {
        id: MeshId,
        before: Option<MeshId>,
    },
    /// テクスチャ (PNG) を `Model::textures` の末尾に追加する。寸法は PNG の IHDR から読む。
    /// 名前は `[A-Za-z0-9_.-]` (`.` 始まりは不可) で、大文字小文字を区別せずに一意。
    /// `png` は JSON では数値の配列になるので、主にバックエンド内 (PSD 取り込みなど) から使う。
    AddTexture {
        name: String,
        png: Vec<u8>,
    },
    /// テクスチャの画像を差し替える。`TextureRef` の寸法も PNG に合わせる。
    SetTextureImage {
        name: String,
        png: Vec<u8>,
    },
    /// テクスチャを削除する。使っているメッシュがあれば拒否する。
    /// 後ろのテクスチャを指すメッシュの `texture` は 1 つ詰める。ほかのテクスチャから指されない
    /// 法線マップの画像も表から除く。
    RemoveTexture {
        name: String,
    },
    /// キャンバスの大きさと原点を変える。要素の座標は変えない。
    /// `width` と `height` は 0 より大きい有限の値。
    SetCanvas {
        width: f32,
        height: f32,
        #[ts(as = "ugoe_core::codegen::Vec2Repr")]
        #[schemars(with = "ugoe_core::codegen::Vec2Repr")]
        origin: Vec2,
    },
    /// メッシュをテクスチャの画像から自動メッシュ (`ugoe-mesher`) で作り直す。
    /// すべてのキーフォームの頂点位置が同じで、頂点位置が UV のアフィン写像になっている
    /// メッシュだけが対象 (それ以外は拒否する)。キーフォームの頂点位置以外の値は保つ。
    /// 規則は `docs/editor.md#メッシュの再生成`。
    RegenerateMesh {
        id: MeshId,
        #[serde(default)]
        options: AutoMeshOptions,
    },
    /// `target` の格子の `derived` のセルを推定で作り直す (`docs/spec/keyform-inference.md#推定の実行`)。
    /// `authored` のセルは変えない。書き換えるのは `method` で作ったセル (方法を記録していないセルは
    /// 格子の `inference` の方法のもの) だけで、`overwrite_derived` なら別の方法で作ったセルも書き換える。
    InferKeyforms {
        target: KeyformTarget,
        #[serde(default)]
        method: InferMethod,
        #[serde(default)]
        overwrite_derived: bool,
    },
    /// `source` のキーフォームを対称軸で反転して、`target` の格子のセルを作る
    /// (`docs/spec/keyform-inference.md#ミラー`)。先の `authored` 以外のセルをすべて書き換え、
    /// 作ったセルは `mirror` になる。`source` と `axis` / `tolerance` は先の格子の `mirror_source` に記録し、
    /// 以後 `source` の格子が変わると自動でミラーし直す。
    MirrorKeyforms {
        source: KeyformTarget,
        target: KeyformTarget,
        /// 対称軸 `x = a` の `a`。省略すると元の要素の `mirror_axis` (または親で決まる既定値)。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis: Option<f32>,
        /// 頂点の対応付けの許容距離 (元の要素の親座標系)。省略するとキャンバスの 1px。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tolerance: Option<f32>,
        /// `true` なら先の `authored` のセルも上書きする。
        #[serde(default)]
        overwrite_authored: bool,
    },
    /// 格子のセルの由来を `authored` にする (値は変えない)。
    MarkKeyformAuthored {
        target: KeyformTarget,
        cells: Vec<KeyformCell>,
    },
    /// 格子のセルの由来を `derived` (方法の記録なし) にし、格子の推定の設定で作り直す。
    MarkKeyformDerived {
        target: KeyformTarget,
        cells: Vec<KeyformCell>,
    },
    /// メッシュの頂点をキャンバス座標で動かす (トポロジーと基準の形の編集)。
    /// ずれは既定値のパラメータでの親デフォーマの変換で局所座標に直し、すべてのキーフォームに
    /// 同じ量を加える。UV は、動かした先でのテクスチャの対応 (既定値の姿の三角形) から求め直す。
    /// 式は `docs/editor.md#メッシュの編集`。
    MoveMeshVertices {
        id: MeshId,
        moves: Vec<VertexMove>,
    },
    /// キャンバス座標 `position` に頂点を 1 つ足し (頂点の末尾に追加)、三角形を作り直す。
    /// UV・各キーフォームの位置・深度は、その点を含む三角形 (外側なら最も近い境界辺の三角形) の
    /// 重心座標で補間する。
    AddMeshVertex {
        id: MeshId,
        #[ts(as = "ugoe_core::codegen::Vec2Repr")]
        #[schemars(with = "ugoe_core::codegen::Vec2Repr")]
        position: Vec2,
    },
    /// 頂点を削除し (後ろの頂点の番号は詰める)、三角形を作り直す。3 頂点未満になるなら拒否する。
    /// グルーのペアのうち削除した頂点を使うものは消し、残りは番号を付け替える。
    RemoveMeshVertices {
        id: MeshId,
        indices: Vec<u16>,
    },
    /// 今の輪郭を保ったまま、UV の制約付きドロネー三角形分割で三角形を作り直す。
    RetriangulateMesh {
        id: MeshId,
    },
    /// 要素の格子の、パラメータ `parameter` の軸にキー `key` を加える。
    /// 軸がなければキー 1 つの軸として加える。新しいセルは既存の格子を補間して埋めるので、
    /// 見た目は変わらない。規則は `docs/editor.md#キーフォームの編集`。
    AddKeyformKey {
        target: KeyformTarget,
        parameter: ParamId,
        key: f32,
    },
    /// 要素の格子の、パラメータ `parameter` の軸からキー `key` とその断面のセルを取り除く。
    /// 軸の最後のキーなら軸ごと取り除く (その断面が残る)。
    RemoveKeyformKey {
        target: KeyformTarget,
        parameter: ParamId,
        key: f32,
    },
    /// 要素の格子の 1 セルを置き換え、`authored` にする。キーフォームの種類は格子と一致させ、
    /// 頂点 (制御点) の数は変えない。
    SetKeyformCell {
        target: KeyformTarget,
        cell: KeyformCell,
        keyform: Keyform,
    },
    /// メッシュの描画順 (`draw_order`) を、格子のすべてのセルで `draw_order` にする。
    /// セルの由来と他の値は変えない。`draw_order` は有限の値。
    SetMeshDrawOrder {
        id: MeshId,
        draw_order: f32,
    },
    /// `children` を包むワープデフォーマ (`rows` × `cols` セル) を作り、子を見た目を保って付け替える。
    /// 制御点は、子の基準の姿の点の外接矩形 (と余白) を親の座標系で等分した格子になる。
    /// `parts` のパーツ (と子孫のパーツ) のいちばん外側のメッシュ・デフォーマも子にする。
    /// `name` が空なら子の名前から作り (`顔のワープ`)、`id` を省略すると名前 (英数字を含まなければ
    /// 子の ID: `WarpFace`) から作る。`parent` を省略すると子の親の最も近い共通の祖先。
    /// 式と制限は `docs/editor.md#デフォーマの編集`。
    CreateWarpDeformer {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<DeformerId>,
        #[serde(default)]
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<ChildRef>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        parts: Vec<PartId>,
        rows: u16,
        cols: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<DeformerId>,
    },
    /// `children` を包む回転デフォーマ (角度 0・拡大率 1) を作り、子を見た目を保って付け替える。
    /// `origin` はキャンバス座標 (基準の姿) で、省略すると子の外接矩形の中心。
    /// `parts`・`name`・`id`・`parent` は `CreateWarpDeformer` と同じ。
    CreateRotationDeformer {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<DeformerId>,
        #[serde(default)]
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<ChildRef>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        parts: Vec<PartId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(as = "Option<ugoe_core::codegen::Vec2Repr>")]
        #[schemars(with = "Option<ugoe_core::codegen::Vec2Repr>")]
        origin: Option<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<DeformerId>,
    },
    /// メッシュまたはデフォーマの親を付け替え、すべてのキーフォームの局所座標を、既定値のパラメータで
    /// 見た目が変わらないように新しい親の座標系へ写し直す (`None` でキャンバス直下)。
    /// 他のパラメータ値では、親が変形すると見た目が変わりうる (`docs/editor.md#デフォーマの編集`)。
    ReparentPreservingLook {
        element: ChildRef,
        new_parent: Option<DeformerId>,
    },
    /// 格子の推定の設定 (`KeyformGrid::inference`) を置き換える (`None` なら加算合成)。
    /// 方法を記録していない `derived` のセルは新しい設定で作り直す
    /// (`docs/spec/keyform-inference.md#セルの由来`)。未知の方法は拒否する。
    SetKeyformInference {
        target: KeyformTarget,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        inference: Option<KeyformInference>,
    },
    /// メッシュ・デフォーマの左右の対 (`mirror`) と対称軸 (`mirror_axis`) を設定する。
    /// 対は双方向に張り、`target` と `mirror` のどちらかと対になっていた要素の対は外す。
    /// `mirror` が `None` (または自分自身) なら対にしない。`axis` は `target` の `mirror_axis`
    /// (`None` なら親で決まる既定値)。規則は `docs/spec/keyform-inference.md#ミラー`。
    SetMirrorPair {
        target: KeyformTarget,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mirror: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        axis: Option<f32>,
    },
    /// メッシュ (頂点ごと) またはワープデフォーマ (制御点ごと) の深度を置き換える (`None` で外す)。
    /// 長さは点の数と同じで、値は有限 (キャンバスのピクセル単位、手前が正)。深度からの回転のセルは
    /// 新しい深度で作り直す (`docs/spec/keyform-inference.md#深度の入力`)。
    SetDepth {
        target: ChildRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        depth: Option<Vec<f32>>,
    },
    /// プリセット (一様・ドーム・選んだ点の値) で深度を作り、`SetDepth` と同じく置き換える。
    /// 式は `docs/editor.md#深度の編集`。
    ApplyDepthPreset {
        target: ChildRef,
        preset: DepthPreset,
    },
    /// 回転デフォーマの原点を、格子のすべてのセルで `delta` (親の局所座標) だけ動かす。
    /// 子 (メッシュ・デフォーマ) の局所座標は、既定値のパラメータで見た目が変わらないように写し直す
    /// (絵はそのままで回転の中心だけが動く)。式は `docs/editor.md#デフォーマの編集`。
    MoveRotationOrigin {
        id: DeformerId,
        #[ts(as = "ugoe_core::codegen::Vec2Repr")]
        #[schemars(with = "ugoe_core::codegen::Vec2Repr")]
        delta: Vec2,
    },
    /// デフォーマを外す: 子を見た目を保ってデフォーマの親へ付け替え (`ReparentPreservingLook` と同じ)、
    /// デフォーマを削除する。
    UnwrapDeformer {
        id: DeformerId,
    },
    // --- モーションと表情 (`docs/editor.md#モーションと表情の編集`) ---
    /// モーションを末尾に追加する。名前はファイル名に使える文字 (`[A-Za-z0-9_.-]`、`.` 始まりは不可) で、
    /// 大文字小文字を区別せずに一意。カーブの対象はモデルにあること。
    AddMotion {
        motion: Motion,
    },
    RemoveMotion {
        name: String,
    },
    /// モーションの名前 (ファイル名にもなる) を変える。
    RenameMotion {
        name: String,
        new_name: String,
    },
    /// 長さ (秒、正)・ループ・フェード (秒、0 以上) を変える。省略したフィールドは変えない。
    SetMotionProperties {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        looping: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fade_in: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fade_out: Option<f32>,
    },
    /// カーブを追加する。同じ対象のカーブがあれば拒否する。
    AddMotionCurve {
        motion: String,
        curve: Curve,
    },
    RemoveMotionCurve {
        motion: String,
        target: CurveTarget,
    },
    /// カーブ個別のフェード (`None` ならモーション全体の値) を設定する。
    SetMotionCurveFade {
        motion: String,
        target: CurveTarget,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fade_in: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fade_out: Option<f32>,
    },
    /// 時刻 `point.t` にキーを打つ。その時刻のキーがあれば値を変え、なければ加える (カーブがなければ作る)。
    /// 区間の中に加えると区間を同じ種類の 2 つに分ける (ベジェは形を保って分ける)。
    SetMotionKey {
        motion: String,
        target: CurveTarget,
        point: Point,
    },
    /// 時刻 `t` のキーを `to` へ動かす。隣のキーを越えられない。隣のベジェの取っ手も一緒に動く。
    MoveMotionKey {
        motion: String,
        target: CurveTarget,
        t: f32,
        to: Point,
    },
    /// 時刻 `t` のキーを消し、前後の区間をつなぐ。最後のキーならカーブを消す。
    RemoveMotionKey {
        motion: String,
        target: CurveTarget,
        t: f32,
    },
    /// 時刻 `t` のキーで終わる区間の種類 (直線・ベジェと取っ手・ステップ) を変える。
    /// 取っ手の時刻は区間の中 (`docs/spec/motion.md#カーブ`)。
    SetMotionSegment {
        motion: String,
        target: CurveTarget,
        t: f32,
        segment: SegmentShape,
    },
    /// イベントの一覧を置き換える (時刻の順に並べ直す)。時刻は 0 以上。
    SetMotionEvents {
        motion: String,
        events: Vec<MotionEvent>,
    },
    /// 表情を末尾に追加する。名前の規則はモーションと同じ。パラメータはモデルにあること。
    AddExpression {
        expression: Expression,
    },
    RemoveExpression {
        name: String,
    },
    RenameExpression {
        name: String,
        new_name: String,
    },
    /// フェード (秒、0 以上) を変える。省略したフィールドは変えない。
    SetExpressionProperties {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fade_in: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fade_out: Option<f32>,
    },
    /// 表情のパラメータの一覧 (パラメータ・値・`add` / `multiply` / `overwrite`) を置き換える。
    /// 同じパラメータは 1 回だけ。
    SetExpressionParameters {
        name: String,
        parameters: Vec<ExpressionParameter>,
    },
    // ---- 物理 (T0629)。規則は `docs/editor.md#物理の編集` ----
    /// 物理全体の重力・風・ステップの周波数を設定する。物理の設定がなければ作る (リグなし)。
    /// `fps` は 0 より大きい有限の値、`gravity`・`wind` は有限の値。
    SetPhysicsWorld {
        #[ts(as = "ugoe_core::codegen::Vec2Repr")]
        #[schemars(with = "ugoe_core::codegen::Vec2Repr")]
        gravity: Vec2,
        #[ts(as = "ugoe_core::codegen::Vec2Repr")]
        #[schemars(with = "ugoe_core::codegen::Vec2Repr")]
        wind: Vec2,
        fps: f32,
    },
    /// 物理のリグを末尾に追加する。物理の設定がなければ既定値 (重力 `(0, 1)`・風なし・60 Hz) で作る。
    AddPhysicsRig {
        rig: PhysicsRig,
    },
    /// 回転デフォーマの鎖 (`deformers`、根元から順) に沿った動的物理のリグを作り、末尾に追加する。
    /// 粒子は基準の姿での各デフォーマの原点と、最後のデフォーマの原点から重力の向きに子孫の最も深い点まで
    /// 進んだ先端に置き、粒子 k の角度を k 番目のデフォーマの角度に足す (`deformer_angle`)。`name` が空なら最初のデフォーマの名前、
    /// `id` を省略すると名前 (英数字を含まなければ最初のデフォーマの ID) から作る。
    CreatePhysicsRig {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        name: String,
        deformers: Vec<DeformerId>,
    },
    /// 物理のリグを削除する。
    RemovePhysicsRig {
        id: String,
    },
    /// 物理のリグの表示名を変える (ID は変えない)。
    RenamePhysicsRig {
        id: String,
        name: String,
    },
    /// リグの入力を置き換える。入力・出力・粒子は ID を持たないので、並びごと置き換える。
    SetPhysicsInputs {
        rig: String,
        inputs: Vec<PhysicsInput>,
    },
    /// リグの出力を置き換える。
    SetPhysicsOutputs {
        rig: String,
        outputs: Vec<PhysicsOutput>,
    },
    /// リグの粒子の鎖 (位置と係数) を置き換える。粒子は 1 つ以上。
    SetPhysicsParticles {
        rig: String,
        particles: Vec<PhysicsParticle>,
    },
    /// リグの入力の正規化の範囲を置き換える。
    SetPhysicsNormalization {
        rig: String,
        normalization: PhysicsNormalization,
    },
    /// リグの粒子と衝突する衝突体 (`PhysicsCollider::id`) を置き換える。
    SetPhysicsRigColliders {
        rig: String,
        colliders: Vec<String>,
    },
    /// リグの重力と風の基準のデフォーマを設定する (`None` でキャンバス)。
    SetPhysicsFrame {
        rig: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        frame: Option<DeformerId>,
    },
    /// 衝突体を末尾に追加する。物理の設定がなければ `AddPhysicsRig` と同じ既定値で作る。
    AddPhysicsCollider {
        collider: PhysicsCollider,
    },
    /// 衝突体を削除し、リグの `colliders` からも外す。
    RemovePhysicsCollider {
        id: String,
    },
    /// 衝突体 (`collider.id` のもの) の名前・取り付け先・形状を置き換える。
    SetPhysicsCollider {
        collider: PhysicsCollider,
    },
    /// 格子のキーの間の補間の種類 (`KeyformGrid::interpolation`) を設定する
    /// (`docs/spec/evaluation.md#曲線補間`)。セルの値は変えない。
    /// `blend_shape` を指定すると、そのパラメータを軸に持つブレンドシェイプ格子に設定する。
    /// 未知の値と、形を持たないパーツ・グルーの格子への `smooth` は拒否する。
    SetKeyformInterpolation {
        target: KeyformTarget,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blend_shape: Option<ParamId>,
        interpolation: KeyformInterpolation,
    },
    // ---- リグの順序と中継パラメータ (T0312)。`docs/spec/physics.md#リグの順序と中継` ----
    /// 物理のリグを `before` の直前へ移す (`None` なら末尾)。リグは並びの順に処理するので、
    /// 前のリグの出力を入力に読むリグ (中継) の結果が変わる。
    ReorderPhysicsRig {
        id: String,
        before: Option<String>,
    },
    /// パラメータを中継用 (内部) にする・戻す (`Parameter::internal`、`docs/spec/model.md#パラメータ`)。
    SetParameterInternal {
        id: ParamId,
        internal: bool,
    },
    // ---- 描画用の複製 (T0633)。規則は `docs/editor.md#描画用の複製` ----
    /// `source` の描画用の複製を作る: UV・三角形・テクスチャ・親・マスクなどが同じで、キーフォームが
    /// 基準の形 1 つのメッシュを元の直後に加え、全頂点を元に一致させるグルー
    /// (`DrawClone_<複製の ID>`、`weight = 0`・`intensity = 1`) を `glues` の末尾に張る。
    /// `source` が複製なら、その元の複製を作る。`id` を省略すると `<元の ID>_Clone`、
    /// `name` が空なら `<元の名前>の複製`。元の形・テクスチャ・親が変わると複製は自動で追従する。
    CreateDrawClone {
        source: MeshId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<MeshId>,
        #[serde(default)]
        name: String,
    },
    /// 描画用の複製を元に合わせ直す (`id` が複製ならその複製、元ならそのすべての複製)。
    /// UV・三角形・テクスチャ・親と、グルーのペア・強さ・順序を作り直す。
    /// 描画順・不透明度・マスクなど複製が持つ値は変えない。
    SyncDrawClone {
        id: MeshId,
    },
    // ---- 物理のテンプレート (T0634)。規則は `docs/editor.md#物理のテンプレート` ----
    /// 定番の物理のリグの組 (体の追従・弾み・首の追従) を作る。[`crate::physics_template::commands`] が作る
    /// コマンドの列 (パラメータの追加・リグの追加や置き換え・並べ替え) を、1 つの Undo 単位で適用する。
    ApplyPhysicsTemplate {
        template: PhysicsTemplate,
        #[serde(default)]
        options: PhysicsTemplateOptions,
    },
    // ---- アバタープロファイル (T0631)。規則は `docs/editor.md#アバタープロファイルの編集` ----
    /// アバタープロファイル (`Model::avatar`) を丸ごと置き換える。`null` で外す。
    SetAvatarProfile {
        profile: Option<AvatarProfile>,
    },
    /// アバタープロファイルの一部を変える。省略したフィールドは変えない。
    /// プロファイルがなければ既定値 (許諾は最も制限の強い値) から作る。
    UpdateAvatarProfile {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        level: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        meta: Option<AvatarMeta>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tracking: Option<Vec<TrackingOverride>>,
        /// 視線の追従。`null` で外す (既定の `EyeBallX` / `EyeBallY` に戻す)。
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "crate::avatar::nullable"
        )]
        #[ts(as = "Option<LookAt>")]
        #[schemars(with = "Option<LookAt>")]
        look_at: Option<Option<LookAt>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hit_areas: Option<Vec<AvatarHitArea>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slots: Option<Vec<String>>,
    },
    /// 当たり判定 (`Model::hit_areas`) を丸ごと置き換える。なくなった当たり判定を指す、
    /// アバタープロファイルの標準の当たり判定 (`avatar.hit_areas`) は外す。
    SetHitAreas {
        hit_areas: Vec<HitArea>,
    },
    /// テクスチャ `texture` の法線マップ (`TextureRef::normal_map`) を設定・差し替え・外す
    /// (`docs/editor.md#法線マップ`)。法線マップの画像はテクスチャと同じ表に `normal_map` の名前で置く。
    /// - `normal_map` と `png`: 画像を `normal_map` の名前で置く (すでにあれば差し替える。その画像を
    ///   指すほかのテクスチャにも効く)。名前はテクスチャの名前と同じ規則で、テクスチャの名前と
    ///   (大文字小文字を区別せずに) 重ならないこと。
    /// - `normal_map` だけ: ほかのテクスチャがすでに指している法線マップの画像を共有する。
    /// - どちらもなし: 法線マップを外す。
    ///
    /// どのテクスチャからも指されなくなった法線マップの画像は表から除く。
    SetNormalMap {
        texture: String,
        normal_map: Option<String>,
        /// 法線マップの画像 (PNG)。JSON では数値の配列になるので、主にバックエンド内 (生成の提案など) から使う。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        png: Option<Vec<u8>>,
    },
    /// モデル全体のライティングの有無 (`Model::lighting`) を変える。
    SetLighting {
        enabled: bool,
    },
    // ---- エフェクト (T0510)。規則は `docs/editor.md#エフェクトの設定` ----
    /// 目パチ・呼吸・リップシンクの設定 (`Model::effects`) を丸ごと置き換える。対象のパラメータがモデルに
    /// あること・時間の値の範囲は、適用後の検証 (`validate`) が調べる。
    SetEffects {
        effects: Effects,
    },
}

/// デフォーマの子になれる要素。JSON では `{"kind": "mesh", "id": "EyeL"}` と書く
/// ([`KeyformTarget`] と同じ形)。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS, JsonSchema)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum ChildRef {
    Mesh(MeshId),
    Deformer(DeformerId),
}

/// キーフォームの格子を持つ要素。JSON では `{"kind": "mesh", "id": "EyeL"}` と書く。
///
/// どの要素も格子を 1 つだけ持つ (`ugoe_rigging::KeyformTarget` と同じ)。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS, JsonSchema)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum KeyformTarget {
    Part(PartId),
    Mesh(MeshId),
    /// ワープデフォーマまたは回転デフォーマ。
    Deformer(DeformerId),
    /// グルー (`Glue::id`)。
    Glue(String),
}

impl From<&KeyformTarget> for ugoe_rigging::KeyformTarget {
    fn from(target: &KeyformTarget) -> Self {
        match target {
            KeyformTarget::Part(id) => Self::Part(id.clone()),
            KeyformTarget::Mesh(id) => Self::Mesh(id.clone()),
            KeyformTarget::Deformer(id) => Self::Deformer(id.clone()),
            KeyformTarget::Glue(id) => Self::Glue(id.clone()),
        }
    }
}

impl std::fmt::Display for KeyformTarget {
    /// エラーメッセージ用の表記 (例: ``mesh `EyeL` ``)。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, id) = match self {
            KeyformTarget::Part(id) => ("part", id.as_str()),
            KeyformTarget::Mesh(id) => ("mesh", id.as_str()),
            KeyformTarget::Deformer(id) => ("deformer", id.as_str()),
            KeyformTarget::Glue(id) => ("glue", id.as_str()),
        };
        write!(f, "{kind} `{id}`")
    }
}

/// [`Command::InferKeyforms`] の推定の方法。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InferMethod {
    /// 格子の `inference` の設定に従う (省略時は加算合成)。
    #[default]
    FromGrid,
    /// 加算合成 (`docs/spec/keyform-inference.md#加算合成`)。
    Additive,
    /// 深度からの回転 (`docs/spec/keyform-inference.md#深度からの回転`)。
    /// 格子の `inference` が `depth_rotation` でなければならない。
    DepthRotation,
}

/// [`Command::MoveMeshVertices`] の 1 頂点分の移動。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
pub struct VertexMove {
    /// 頂点の番号 (`Mesh::uvs` の添字)。
    pub index: u16,
    /// キャンバス座標でのずれ。
    #[ts(as = "ugoe_core::codegen::Vec2Repr")]
    #[schemars(with = "ugoe_core::codegen::Vec2Repr")]
    pub delta: Vec2,
}
