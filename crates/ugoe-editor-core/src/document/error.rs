//! 編集の失敗 [`EditError`]。

use ugoe_model::validate::ValidationError;

use crate::grid::CellError;
use crate::module_project::ModuleError;
use ugoe_rigging::InferError;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum EditError {
    /// `kind` は要素の種類 (`parameter`、`part`、`deformer`、`mesh`)。
    #[error("{kind} `{id}` already exists")]
    DuplicateId { kind: &'static str, id: String },
    #[error("{kind} `{id}` not found")]
    NotFound { kind: &'static str, id: String },
    #[error("invalid range: min {min} <= default {default} <= max {max} must hold")]
    InvalidRange { min: f32, default: f32, max: f32 },
    /// 子が残っている要素は削除できない。`children` は ``mesh `Quad` `` のような表記。
    #[error("{kind} `{id}` still has children: {}", children.join(", "))]
    HasChildren {
        kind: &'static str,
        id: String,
        children: Vec<String>,
    },
    /// `field` が自分自身を指している (並べ替えの基準など)。
    #[error("{kind} `{id}`: {field} must not refer to itself")]
    SelfReference {
        kind: &'static str,
        id: String,
        field: &'static str,
    },
    #[error("deformer `{id}` is not a {expected} deformer")]
    DeformerKindMismatch { id: String, expected: &'static str },
    #[error("{kind} `{id}`: invalid keyform cell: {error}")]
    InvalidCell {
        kind: &'static str,
        id: String,
        error: CellError,
    },
    /// テクスチャ名がファイル名に使えない (`crate::texture::is_valid_texture_name`)。
    #[error("texture name `{name}` is invalid (use [A-Za-z0-9_.-], not starting with `.`)")]
    InvalidTextureName { name: String },
    /// テクスチャの画像が PNG でない (または IHDR が読めない)。
    #[error("texture `{name}`: image is not a PNG")]
    NotPng { name: String },
    /// キャンバスの大きさが 0 以下か有限でない。
    #[error("invalid canvas size {width}x{height}")]
    InvalidCanvas { width: f32, height: f32 },
    /// メッシュを作り直せない (`Command::RegenerateMesh` の条件を満たさない)。
    #[error("mesh `{id}` cannot be regenerated: {reason}")]
    CannotRegenerateMesh { id: String, reason: String },
    /// 自動メッシュの生成に失敗した (画像が透明、設定が不正など)。
    #[error("mesh `{id}`: automatic meshing failed: {message}")]
    Mesher { id: String, message: String },
    #[error("mesh `{id}`: vertex {index} out of range (vertices = {vertices})")]
    VertexOutOfRange {
        id: String,
        index: u16,
        vertices: usize,
    },
    /// 頂点の削除で、メッシュの頂点が 3 つ未満になる。
    #[error("mesh `{id}`: only {remaining} vertices would remain (at least 3 are needed)")]
    TooFewVertices { id: String, remaining: usize },
    /// メッシュの形の編集ができない (退化した三角形・同じ UV の頂点・評価できないモデルなど)。
    #[error("mesh `{id}`: {reason}")]
    MeshGeometry { id: String, reason: String },
    /// 描画順が有限の値でない。
    #[error("mesh `{id}`: draw order {draw_order} is not finite")]
    InvalidDrawOrder { id: String, draw_order: f32 },
    /// `SetKeyformCell` のキーフォームの種類が、対象の格子の種類 (`expected`) と違う。
    #[error("{kind} `{id}`: expected a {expected} keyform, got a {actual} keyform")]
    KeyformKindMismatch {
        kind: &'static str,
        id: String,
        expected: &'static str,
        actual: &'static str,
    },
    /// 適用するとモデルが検証 (`ugoe_model::validate`) に通らなくなる。新たに生じるエラーの一覧。
    #[error("command would make the model invalid: {}", join(.0))]
    Invalid(Vec<ValidationError>),
    /// キーフォームの推定 (ミラー・`derived` のセルの作り直しを含む) に失敗した。
    /// `target` は書き込み先の格子の要素 (``mesh `EyeL` `` のような表記)。
    #[error("{target}: keyform inference failed: {error}")]
    Inference { target: String, error: InferError },
    /// 格子の推定の方法が未知 (新しい版のファイル) なので、その設定では推定できない。
    #[error("{target}: keyform inference method is not supported by this version")]
    UnsupportedInference { target: String },
    /// 格子に設定できない補間の種類 (未知の値、パーツ・グルーの格子への曲線補間)。
    #[error("{target}: keyform interpolation `{interpolation:?}` is not supported for this grid")]
    UnsupportedInterpolation {
        target: String,
        interpolation: ugoe_model::KeyformInterpolation,
    },
    /// デフォーマの作成・付け替え・セルの編集ができない (子がない、循環する、座標を写せないなど)。
    #[error("deformer edit failed: {reason}")]
    DeformerEdit { reason: String },
    /// 深度の長さが要素の点 (頂点・制御点) の数と違う。
    #[error("{target}: depth has {actual} values but the element has {expected} points")]
    DepthLength {
        target: String,
        expected: usize,
        actual: usize,
    },
    /// 深度に有限でない値がある。
    #[error("{target}: depth value {value} is not finite")]
    NonFiniteDepth { target: String, value: f32 },
    /// 深度を設定する点の番号が範囲外。
    #[error("{target}: point {index} out of range (points = {points})")]
    DepthPointOutOfRange {
        target: String,
        index: u32,
        points: usize,
    },
    /// 左右の対 (`mirror`) を持てない要素 (パーツ・グルー)。
    #[error("{target}: only meshes and deformers have mirror pairs")]
    MirrorPairNotSupported { target: String },
    /// モーション・表情の名前がファイル名に使えない (`[A-Za-z0-9_.-]`、`.` 始まりは不可)。
    #[error("{kind} name `{name}` is invalid (use [A-Za-z0-9_.-], not starting with `.`)")]
    InvalidName { kind: &'static str, name: String },
    /// 編集するとモーションに問題 (長さ・フェード・キーの時刻の順・取っ手の範囲など) が生じる。
    #[error("motion `{motion}`: {reason}")]
    InvalidMotion { motion: String, reason: String },
    /// 編集すると表情に問題 (フェード・値・同じパラメータの重複など) が生じる。
    #[error("expression `{expression}`: {reason}")]
    InvalidExpression { expression: String, reason: String },
    /// 物理の設定の値が不正 (有限でない、範囲外、粒子がないなど。`docs/editor.md#物理の編集`)。
    #[error("invalid physics settings: {reason}")]
    InvalidPhysics { reason: String },
    /// 描画用の複製の形 (頂点・三角形) は元に従うので、直接は編集できない
    /// (`docs/editor.md#描画用の複製`)。
    #[error("mesh `{id}` is a draw clone of `{original}`; edit the shape of the original")]
    DrawCloneShape { id: String, original: String },
    /// 描画用の複製でも、複製を持つメッシュでもない。
    #[error("mesh `{id}` is neither a draw clone nor has draw clones")]
    NotDrawClone { id: String },
    /// アバタープロファイルの値が不正 (範囲外のレベル・有限でない数・未知の値など。
    /// `docs/editor.md#アバタープロファイルの編集`)。
    #[error("invalid avatar profile: {reason}")]
    InvalidAvatarProfile { reason: String },
    /// 法線マップの指定が不正 (`Command::SetNormalMap`)。
    #[error("texture `{texture}`: invalid normal map: {reason}")]
    InvalidNormalMap { texture: String, reason: String },
    /// スロットの指定が不正 (ID の形式・ほかのスロットとの描画順の範囲の重なり。
    /// `docs/editor.md#スロットの編集`)。
    #[error("slot `{id}`: {reason}")]
    InvalidSlot { id: String, reason: String },
    /// グルーの作成・設定の値が不正 (同じメッシュ・組ができない距離・有限でない重みなど。
    /// `docs/editor.md#グルーの編集`)。`id` はグルーの ID か、作るときは両端のメッシュ (`A + B`)。
    #[error("glue `{id}`: {reason}")]
    InvalidGlue { id: String, reason: String },
    /// モジュールのプロジェクトで、読み取り専用の本体の要素を変えようとした
    /// (`element` は ``parameter `AngleX` `` のような表記。`docs/editor.md#モジュールのプロジェクト`)。
    #[error(
        "{element} belongs to the base model and can't be edited here. Edit it in the base project"
    )]
    ReadOnlyBase { element: String },
    /// モジュールのプロジェクトで、編集した結果がモジュールとして表せない
    /// (本体の要素への参照・接頭辞を外した ID の重なりなど)。
    #[error("cannot be kept as a module: {reason}")]
    ModuleShape { reason: String },
    /// モジュールのプロジェクトでないのに、モジュールの設定を変えようとした (`Command::SetModuleSettings`)。
    #[error("this is not a module project")]
    NotModule,
    /// モジュールのプロジェクトで、作業用のモデルを作り直せない (スロットがない・合成できない・本体に物理がないなど)。
    /// コードと引数は中の [`ModuleError`] のもの (`module.*`)。
    #[error(transparent)]
    Module(ModuleError),
    /// テクスチャの画像の一辺が、デコードの上限 (`ugoe_format::image::ImageLimits`) を超える。
    /// テクスチャと法線マップの追加・差し替え (`AddTexture`・`SetTextureImage`・`SetNormalMap`) は置く前に、
    /// メッシュの作り直しは画素を確保する前に返す (T0560)。
    #[error("texture `{texture}` is {width}x{height}, larger than the limit {max}x{max}")]
    TextureTooLarge {
        texture: String,
        width: u32,
        height: u32,
        max: u32,
    },
    /// `set_mesh_properties` で、テクスチャを名前 (`texture_name`) と非推奨の番号 (`texture`) の両方で指定した。
    #[error(
        "mesh `{id}`: give the texture by `texture_name` only, not together with the deprecated `texture` index"
    )]
    ConflictingTexture { id: String },
    /// 分類の直し (`Command::SetLayerCorrection`) が正しくない (空のパス・形の違うラベル・数の上限など。
    /// `docs/editor.md#分類の直し`)。`Batch` の中で使ったときも。
    #[error("layer correction: {reason}")]
    InvalidLayerCorrection { reason: String },
}

fn join(errors: &[ValidationError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}
