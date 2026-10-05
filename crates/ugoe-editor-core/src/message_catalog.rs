//! 画面に出すエラー・警告のコードと引数の名前の一覧 (`docs/editor.md#エラーと警告のコード`)。
//!
//! 翻訳のカタログ (エディタの `locales/<言語>.json` の `errors.<code>`) にすべてのコードの文があり、文の引数が
//! コードの引数の名前に収まることを、エディタ側で検査するために使う (`just gen` が一覧を書き出す。T0681)。
//! カタログはこのクレートから読まない。
//!
//! 一覧は、各コードの見本の値 ([`message_samples`]) から [`message_codes`] で作る。見本は本物のエラーの
//! `to_message()` なので、引数の名前は [`Coded::args`] と一致する。ソースに書いたコードがすべて見本にあることは
//! `message_tests.rs` が確かめる (コードを足したら、ここにも見本を足す)。

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ugoe_format::{ChunkTag, FormatError, ReadLimit};
use ugoe_mesher::MeshError;
use ugoe_model::KeyformInterpolation;
use ugoe_model::avatar::check::ProfileIssue;
use ugoe_model::validate::ValidationError;
use ugoe_psd::{ImportLimit, PsdError, WarningKind};
use ugoe_rigging::{InferError, NormalMapError, WorldError};
use ugoe_runtime::{CompileError, ComposeError};

use crate::atlas::AtlasError;
use crate::command::KeyformTarget;
use crate::document::EditError;
use crate::export::ExportError;
use crate::grid::CellError;
use crate::import::{PsdImportError, PsdLimit, psd_warning_code};
use crate::inference::RederiveWarning;
use crate::lighting::LightingError;
use crate::message::{Coded, Message, MessageArgs};
use crate::message_args as args;
use crate::module_project::ModuleError;
use crate::overflow::OverflowError;
use crate::project::ProjectError;
use crate::proposal::ProposalError;
use crate::space::SpaceError;

/// コード → 引数の名前。同じコードの見本が複数あれば、引数の名前を合わせたもの。
pub type MessageCodes = BTreeMap<String, BTreeSet<String>>;

/// 見本の一覧から、コードと引数の名前の一覧を作る。原因 (`cause`) と一覧の中身 (`items`) のコードも含める。
pub fn message_codes(messages: impl IntoIterator<Item = Message>) -> MessageCodes {
    let mut codes = MessageCodes::new();
    let mut pending: Vec<Message> = messages.into_iter().collect();
    while let Some(message) = pending.pop() {
        codes
            .entry(message.code)
            .or_default()
            .extend(message.args.into_keys());
        if let Some(cause) = message.cause {
            pending.push(*cause);
        }
        pending.extend(message.items);
    }
    codes
}

/// editor-core が出すすべてのコードの見本 (エラー・警告・問題)。引数の値に意味はない。
pub fn message_samples() -> Vec<Message> {
    let mut out = Vec::new();
    out.extend(edit_errors().iter().map(Coded::to_message));
    out.extend(cell_errors().iter().map(Coded::to_message));
    out.extend(proposal_errors().iter().map(Coded::to_message));
    out.extend(project_errors().iter().map(Coded::to_message));
    out.extend(module_errors().iter().map(Coded::to_message));
    out.extend(export_errors().iter().map(Coded::to_message));
    out.extend(atlas_errors().iter().map(Coded::to_message));
    out.extend(overflow_errors().iter().map(Coded::to_message));
    out.extend(lighting_errors().iter().map(Coded::to_message));
    out.extend(space_errors().iter().map(Coded::to_message));
    out.extend(psd_import_errors().iter().map(Coded::to_message));
    out.extend(format_errors().iter().map(Coded::to_message));
    out.extend(profile_issues().iter().map(Coded::to_message));
    out.extend(compile_errors().iter().map(Coded::to_message));
    out.extend(validation_errors().iter().map(Coded::to_message));
    out.extend(infer_errors().iter().map(Coded::to_message));
    out.extend(world_errors().iter().map(Coded::to_message));
    out.extend(normal_map_errors().iter().map(Coded::to_message));
    out.extend(compose_errors().iter().map(Coded::to_message));
    out.extend(psd_errors().iter().map(Coded::to_message));
    out.extend(mesh_errors().iter().map(Coded::to_message));
    out.extend(warnings());
    let mirror = RederiveWarning::mirror_failed(
        KeyformTarget::Mesh("EyeR".into()),
        KeyformTarget::Mesh("EyeL".into()),
        &InferError::UnmatchedVertices {
            vertices: vec![0],
            tolerance: 1.0,
        },
    );
    let mut message = Message::new(&mirror.code, mirror.args, mirror.message);
    message.cause = mirror.cause.map(Box::new);
    out.push(message);
    out
}

fn id() -> String {
    "Face".to_owned()
}

fn reason() -> String {
    "reason".to_owned()
}

fn path() -> PathBuf {
    PathBuf::from("/work/a.ugoeproj")
}

fn json_error() -> serde_json::Error {
    serde_json::Error::io(std::io::Error::other("bad json"))
}

fn edit_errors() -> Vec<EditError> {
    vec![
        EditError::DuplicateId {
            kind: "mesh",
            id: id(),
        },
        EditError::NotFound {
            kind: "mesh",
            id: id(),
        },
        EditError::InvalidRange {
            min: 1.0,
            default: 0.0,
            max: -1.0,
        },
        EditError::HasChildren {
            kind: "part",
            id: id(),
            children: vec!["Eye".to_owned()],
        },
        EditError::SelfReference {
            kind: "mesh",
            id: id(),
            field: "masks",
        },
        EditError::DeformerKindMismatch {
            id: id(),
            expected: "warp",
        },
        EditError::InvalidCell {
            kind: "mesh",
            id: id(),
            error: CellError::UnknownAxis("AngleX".to_owned()),
        },
        EditError::InvalidTextureName { name: id() },
        EditError::NotPng { name: id() },
        EditError::InvalidCanvas {
            width: 0.0,
            height: 0.0,
        },
        EditError::CannotRegenerateMesh {
            id: id(),
            reason: reason(),
        },
        EditError::Mesher {
            id: id(),
            error: MeshError::Empty,
        },
        EditError::VertexOutOfRange {
            id: id(),
            index: 9,
            vertices: 4,
        },
        EditError::TooFewVertices {
            id: id(),
            remaining: 2,
        },
        EditError::MeshGeometry {
            id: id(),
            reason: reason(),
        },
        EditError::InvalidDrawOrder {
            id: id(),
            draw_order: f32::NAN,
        },
        EditError::KeyformKindMismatch {
            kind: "deformer",
            id: id(),
            expected: "warp",
            actual: "rotation",
        },
        EditError::Invalid(Vec::new()),
        EditError::Inference {
            target: id(),
            error: InferError::TooManyAxes { count: 65 },
        },
        EditError::UnsupportedInference { target: id() },
        EditError::UnsupportedInterpolation {
            target: id(),
            interpolation: KeyformInterpolation::Smooth,
        },
        EditError::DeformerEdit { reason: reason() },
        EditError::DepthLength {
            target: id(),
            expected: 4,
            actual: 3,
        },
        EditError::NonFiniteDepth {
            target: id(),
            value: f32::INFINITY,
        },
        EditError::DepthPointOutOfRange {
            target: id(),
            index: 9,
            points: 4,
        },
        EditError::MirrorPairNotSupported { target: id() },
        EditError::InvalidName {
            kind: "motion",
            name: id(),
        },
        EditError::InvalidMotion {
            motion: id(),
            reason: reason(),
        },
        EditError::InvalidExpression {
            expression: id(),
            reason: reason(),
        },
        EditError::InvalidPhysics { reason: reason() },
        EditError::DrawCloneShape {
            id: id(),
            original: id(),
        },
        EditError::NotDrawClone { id: id() },
        EditError::InvalidAvatarProfile { reason: reason() },
        EditError::InvalidNormalMap {
            texture: id(),
            reason: reason(),
        },
        EditError::InvalidSlot {
            id: id(),
            reason: reason(),
        },
        EditError::InvalidGlue {
            id: id(),
            reason: reason(),
        },
        EditError::ReadOnlyBase { element: id() },
        EditError::ModuleShape { reason: reason() },
        EditError::NotModule,
        EditError::Module(ModuleError::UnknownSlot("hair".to_owned())),
        EditError::TextureTooLarge {
            texture: id(),
            width: 20000,
            height: 10,
            max: 16384,
        },
        EditError::ConflictingTexture { id: id() },
        EditError::InvalidLayerCorrection { reason: reason() },
    ]
}

fn cell_errors() -> Vec<CellError> {
    let parameter = || "AngleX".to_owned();
    vec![
        CellError::MissingAxis(parameter()),
        CellError::UnknownAxis(parameter()),
        CellError::UnknownKey {
            parameter: parameter(),
            value: 0.5,
        },
        CellError::OutOfRange { index: 3, len: 2 },
        CellError::DuplicateKey {
            parameter: parameter(),
            value: 0.5,
        },
        CellError::NonFiniteKey {
            parameter: parameter(),
            value: f32::NAN,
        },
        CellError::KeyformCount {
            expected: 3,
            actual: 2,
        },
        CellError::ShapeMismatch {
            expected: 4,
            actual: 3,
        },
    ]
}

fn proposal_errors() -> Vec<ProposalError> {
    vec![
        ProposalError::NotFound(1),
        ProposalError::Stale,
        ProposalError::InvalidIndex { index: 2 },
        ProposalError::NothingSelected,
        ProposalError::Rejected(EditError::NotModule),
    ]
}

fn project_errors() -> Vec<ProjectError> {
    vec![
        ProjectError::Io {
            path: path(),
            source: std::io::Error::other("disk full"),
        },
        ProjectError::NotRegularFile { path: path() },
        ProjectError::FileTooLarge {
            path: path(),
            len: 300 * 1024 * 1024,
            max: 256 * 1024 * 1024,
        },
        ProjectError::Json {
            path: path(),
            source: json_error(),
        },
        ProjectError::Format {
            path: path(),
            source: FormatError::BadMagic,
        },
        ProjectError::NotAProject { path: path() },
        ProjectError::NotEmpty { path: path() },
        ProjectError::UnsupportedVersion {
            found: 9,
            supported: 1,
        },
        ProjectError::InvalidName {
            kind: "motion",
            name: id(),
        },
        ProjectError::DuplicateName {
            kind: "motion",
            name: id(),
        },
        ProjectError::MissingTexture { name: id() },
        ProjectError::NotPng { path: path() },
        ProjectError::Export(FormatError::TooLarge),
        ProjectError::UnreadableModuleFile(FormatError::BadMagic),
        ProjectError::MissingModuleLink,
        ProjectError::BaseNotFound { path: path() },
        ProjectError::Base {
            path: path(),
            source: Box::new(ProjectError::NotAProject { path: path() }),
        },
        ProjectError::BaseRefused {
            path: path(),
            reason: reason(),
        },
        ProjectError::BaseIsModule { path: path() },
        ProjectError::UnsupportedBaseTexture { name: id() },
        ProjectError::UnsupportedTexture { name: id() },
        ProjectError::NotAModule,
        ProjectError::InvalidModule(Vec::new()),
    ]
}

fn module_errors() -> Vec<ModuleError> {
    vec![
        ModuleError::InvalidName(id()),
        ModuleError::UnknownSlot("hair".to_owned()),
        ModuleError::Compose(ComposeError::InvalidModuleName(id())),
        ModuleError::ColliderNeedsBasePhysics {
            rig: "Hair".to_owned(),
            collider: "Head".to_owned(),
            slot: "hair".to_owned(),
        },
        ModuleError::ReadOnlyBase { element: id() },
        ModuleError::Shape { reason: reason() },
        ModuleError::NotModule,
    ]
}

fn export_errors() -> Vec<ExportError> {
    vec![
        ExportError::InvalidMaxSize(0),
        ExportError::InvalidPadding(999),
        ExportError::Settings(json_error()),
    ]
}

fn atlas_errors() -> Vec<AtlasError> {
    vec![
        AtlasError::MissingTexture { name: id() },
        AtlasError::NotPng { name: id() },
        AtlasError::TextureOutOfRange {
            mesh: id(),
            texture: 3,
        },
        AtlasError::InvalidMaxSize,
        AtlasError::TextureTooLarge {
            name: id(),
            width: 20000,
            height: 10,
            max: 16384,
        },
    ]
}

fn overflow_errors() -> Vec<OverflowError> {
    vec![OverflowError::UnknownParameter {
        target: KeyformTarget::Mesh("Face".into()),
        parameter: "AngleX".to_owned(),
    }]
}

fn lighting_errors() -> Vec<LightingError> {
    vec![
        LightingError::TextureNotFound(id()),
        LightingError::UnreadableTexture(id()),
        LightingError::UnreadableNormalMap,
        LightingError::Generate(NormalMapError::EmptyImage {
            width: 0,
            height: 0,
        }),
        LightingError::InvalidPreview(reason()),
        LightingError::Settings(json_error()),
        LightingError::TextureTooLarge {
            texture: id(),
            width: 20000,
            height: 10,
            max: 16384,
        },
        LightingError::NormalMapTooLarge {
            width: 20000,
            height: 10,
            max: 16384,
        },
    ]
}

fn space_errors() -> Vec<SpaceError> {
    vec![
        SpaceError::NotFound {
            kind: "mesh",
            id: id(),
        },
        SpaceError::NoSpace {
            kind: "mesh",
            id: id(),
        },
        SpaceError::Evaluate(reason()),
        SpaceError::LengthMismatch {
            points: 3,
            guesses: 2,
        },
        SpaceError::Singular { index: 1 },
        SpaceError::NotConverged {
            index: 1,
            residual: 0.5,
        },
    ]
}

fn psd_import_errors() -> Vec<PsdImportError> {
    vec![
        PsdImportError::Psd(PsdError::Invalid(reason())),
        PsdImportError::LimitExceeded {
            limit: PsdLimit::Channel,
            size: 2048,
            max: 1024,
        },
        PsdImportError::LimitExceeded {
            limit: PsdLimit::Total,
            size: 5000,
            max: 4096,
        },
        PsdImportError::InvalidMeshOptions("max_edge"),
        PsdImportError::Mesh {
            layer: id(),
            source: MeshError::TooComplex,
        },
    ]
}

fn format_errors() -> Vec<FormatError> {
    let tag = ChunkTag::MOTN;
    vec![
        FormatError::BadMagic,
        FormatError::UnsupportedVersion {
            found: 9,
            supported: 1,
        },
        FormatError::TooLarge,
        FormatError::Invalid(Vec::new()),
        FormatError::InvalidModule(Vec::new()),
        FormatError::InvalidPhysics(Vec::new()),
        FormatError::InvalidMotion {
            name: "idle".to_owned(),
            errors: Vec::new(),
        },
        FormatError::InvalidExpression {
            name: "smile".to_owned(),
            errors: Vec::new(),
        },
        FormatError::UnsupportedFeatures(vec!["feature".to_owned()]),
        FormatError::UnknownValues(vec!["value".to_owned()]),
        FormatError::ReadLimitExceeded {
            tag,
            limit: ReadLimit::TotalDecompressed,
            max: 1024,
        },
        FormatError::ReadLimitExceeded {
            tag,
            limit: ReadLimit::ChunkDecompressed,
            max: 256,
        },
        FormatError::Truncated,
    ]
}

fn profile_issues() -> Vec<ProfileIssue> {
    vec![
        ProfileIssue::MissingProfile,
        ProfileIssue::NewerProfileVersion {
            version: 9,
            supported: 1,
        },
        ProfileIssue::InvalidLevel { level: 9, max: 3 },
        ProfileIssue::MissingMetadata { field: "name" },
        ProfileIssue::MissingParameter {
            level: 1,
            id: "AngleX",
        },
        ProfileIssue::TooFewExtendedParameters {
            required: 4,
            found: 1,
        },
        ProfileIssue::ParameterRange {
            id: "AngleX".to_owned(),
            min: -1.0,
            default: 0.0,
            max: 1.0,
            expected: (-30.0, 0.0, 30.0),
        },
        ProfileIssue::UnknownReference {
            field: "field".to_owned(),
            target_kind: "parameter",
            target: id(),
        },
        ProfileIssue::UnknownTrackingInput {
            field: "field".to_owned(),
            kind: "face",
            input: id(),
        },
        ProfileIssue::MissingTrackingTarget {
            field: "field".to_owned(),
        },
        ProfileIssue::DuplicateHitAreaName { name: id() },
        ProfileIssue::UnknownValue {
            field: "field".to_owned(),
        },
        ProfileIssue::InvalidValue {
            field: "field".to_owned(),
            value: -1.0,
            expected: "a positive number",
        },
        ProfileIssue::InternalTrackingTarget {
            field: "field".to_owned(),
            id: id(),
        },
    ]
}

fn compile_errors() -> Vec<CompileError> {
    vec![
        CompileError::Invalid(Vec::new()),
        CompileError::UnknownParameter("AngleX".to_owned()),
        CompileError::Unsupported("feature"),
    ]
}

/// 警告。`ugoe-psd` の警告は [`psd_warning_code`] で作る。コードと引数をその場で書いて出す警告 (PSD の取り込みの
/// 途中の `Builder::warn`・`.ugoemod` の書き出し) は、出す箇所と同じ引数で作る (`message_tests.rs` がソースと
/// 照らし合わせる)。
fn warnings() -> Vec<Message> {
    let kinds = [
        WarningKind::AdjustmentLayer {
            key: "levl".to_owned(),
        },
        WarningKind::SmartObjectWithoutPixels,
        WarningKind::LayerEffectsIgnored,
        WarningKind::LayerMaskIgnored,
        WarningKind::UnsupportedCompression { compression: 3 },
        WarningKind::MissingPixels,
        WarningKind::FlattenedImage,
    ];
    let mut out: Vec<Message> = kinds
        .iter()
        .map(|kind| {
            let (code, args) = psd_warning_code(kind);
            Message::new(code, args, code)
        })
        .collect();
    let inline: [(&str, MessageArgs); 11] = [
        (
            "psd.canvas_mismatch",
            args! {
                "psd_width" => 1u32,
                "psd_height" => 1u32,
                "model_width" => 2.0f32,
                "model_height" => 2.0f32,
            },
        ),
        (
            "psd.group_blend_mode_ignored",
            args! { "blend_mode" => "mul" },
        ),
        ("psd.group_clipping_unsupported", args! {}),
        ("psd.empty_layer", args! {}),
        ("psd.transparent_layer", args! {}),
        ("psd.layer_too_complex", args! {}),
        (
            "psd.layer_too_large",
            args! { "width" => 20000u64, "height" => 20000u64, "max" => 1usize << 28 },
        ),
        (
            "psd.layer_texture_too_large",
            args! { "layer" => "Background", "width" => 20000u32, "height" => 12000u32, "max" => 16384u32 },
        ),
        (
            "psd.blend_mode_unsupported",
            args! { "blend_mode" => "diss" },
        ),
        ("psd.clipping_base_missing", args! {}),
        ("module.draw_order_outside_slot", args! { "mesh" => "Hat" }),
    ];
    out.extend(
        inline
            .into_iter()
            .map(|(code, args)| Message::new(code, args, code)),
    );
    out
}

/// モデルの検証のエラー (`validation.*`)。省けるもの (`back`) は、ある見本と無い見本の両方。
pub(crate) fn validation_errors() -> Vec<ValidationError> {
    vec![
        ValidationError::DuplicateId {
            kind: "part",
            id: id(),
        },
        ValidationError::KeyformCount {
            kind: "mesh",
            id: id(),
            expected: 2,
            actual: 1,
        },
        ValidationError::KeysNotAscending {
            kind: "mesh",
            id: id(),
            parameter: "AngleX".into(),
            keys: vec![1.0, 0.0],
        },
        ValidationError::EmptyKeys {
            kind: "mesh",
            id: id(),
            parameter: "AngleX".into(),
        },
        ValidationError::DuplicateBinding {
            kind: "mesh",
            id: id(),
            parameter: "AngleX".into(),
        },
        ValidationError::IndicesNotTriangles { id: id(), len: 4 },
        ValidationError::IndexOutOfRange {
            id: id(),
            index: 9,
            vertices: 3,
        },
        ValidationError::VertexCount {
            id: id(),
            expected: 3,
            actual: 2,
        },
        ValidationError::WarpPointCount {
            id: id(),
            expected: 4,
            actual: 3,
        },
        ValidationError::EmptyWarpGrid {
            id: id(),
            rows: 0,
            cols: 1,
        },
        ValidationError::UnknownReference {
            kind: "mesh",
            id: id(),
            field: "part",
            target_kind: "part",
            target: "Body".into(),
        },
        ValidationError::TextureOutOfRange {
            id: id(),
            texture: 1,
            textures: 1,
        },
        ValidationError::GlueVertexOutOfRange {
            id: "Neck".into(),
            mesh: id(),
            index: 9,
            vertices: 3,
        },
        ValidationError::ParentCycle {
            kind: "part",
            cycle: vec!["A".into(), "B".into()],
        },
        ValidationError::MaskCycle {
            cycle: vec!["A".into(), "B".into()],
        },
        ValidationError::ParameterRange {
            id: "AngleX".into(),
            min: 1.0,
            default: 0.0,
            max: 2.0,
        },
        ValidationError::DuplicatePosePart {
            part: "ArmL".into(),
        },
        ValidationError::InvalidModuleName { name: "a/b".into() },
        ValidationError::OriginCount {
            kind: "mesh",
            id: id(),
            expected: 2,
            actual: 1,
        },
        ValidationError::UnboundInferenceParameter {
            kind: "mesh",
            id: id(),
            parameter: "AngleX".into(),
        },
        ValidationError::AsymmetricMirror {
            kind: "mesh",
            id: "EyeL".into(),
            mirror: "EyeR".into(),
            back: None,
        },
        ValidationError::AsymmetricMirror {
            kind: "mesh",
            id: "EyeL".into(),
            mirror: "EyeR".into(),
            back: Some("BrowR".into()),
        },
        ValidationError::DepthCount {
            kind: "mesh",
            id: id(),
            expected: 3,
            actual: 2,
        },
        ValidationError::DeformerKindMismatch {
            kind: "mesh",
            id: id(),
            field: "deformer",
            deformer: "Head".into(),
            expected: "warp",
        },
        ValidationError::WeightCount {
            rig: "Hair".into(),
            output: 0,
            expected: 3,
            actual: 2,
        },
        ValidationError::ParameterKindMismatch {
            kind: "mesh",
            id: id(),
            parameter: "Smile".into(),
            expected: "blend_shape",
        },
        ValidationError::InvalidBlendShape {
            kind: "mesh",
            id: id(),
            index: 0,
            reason: "has no bindings",
        },
        ValidationError::InvalidValue {
            field: "pose.fade_duration".into(),
            value: -1.0,
            expected: "finite and >= 0",
        },
    ]
}

/// キーフォームの推定のエラー (`infer.*`)。
pub(crate) fn infer_errors() -> Vec<InferError> {
    let parameter = || ugoe_model::ParamId::new("AngleX");
    let deformer = || ugoe_model::DeformerId::new("Head");
    let target = || ugoe_rigging::KeyformTarget::Mesh("EyeL".into());
    let other = || ugoe_rigging::KeyformTarget::Mesh("EyeR".into());
    vec![
        InferError::UnknownParameter {
            parameter: parameter(),
        },
        InferError::MissingNeutralKey {
            parameter: parameter(),
            default: 0.0,
        },
        InferError::DuplicateAxis {
            parameter: parameter(),
        },
        InferError::NonFiniteKey {
            parameter: parameter(),
            key: f32::NAN,
        },
        InferError::KeysNotAscending {
            parameter: parameter(),
            keys: vec![1.0, 0.0],
        },
        InferError::TooManyAxes { count: 65 },
        InferError::GridTooLarge,
        InferError::GridSizeMismatch {
            expected: 2,
            found: 1,
        },
        InferError::OriginsLengthMismatch {
            expected: 2,
            found: 1,
        },
        InferError::BaseNotAuthored { cell: 0 },
        InferError::ShapeMismatch {
            cell: 1,
            expected: 4,
            found: 3,
        },
        InferError::UnknownTarget(target()),
        InferError::UnsupportedTarget(target()),
        InferError::UnknownParent {
            deformer: deformer(),
        },
        InferError::UnsupportedParent {
            deformer: deformer(),
        },
        InferError::ParentCycle {
            deformer: deformer(),
        },
        InferError::NotMirrorPair {
            from: target(),
            to: other(),
        },
        InferError::IncompatibleMirror {
            from: target(),
            to: other(),
        },
        InferError::MirrorAxisMismatch {
            parameter: parameter(),
        },
        InferError::MissingMirrorKey {
            parameter: parameter(),
            key: -30.0,
        },
        InferError::UnmatchedVertices {
            vertices: vec![0, 2],
            tolerance: 1.0,
        },
        InferError::ParentTransform(WorldError::Singular { index: 0 }),
        InferError::NotDepthRotation(target()),
        InferError::MissingDepth(target()),
        InferError::DepthLengthMismatch {
            expected: 4,
            found: 3,
        },
        InferError::MissingRotationAxis {
            parameter: parameter(),
        },
        InferError::NonAffineParent(target()),
    ]
}

/// PSD の読み込みのエラー (`psd_read.*`)。
pub(crate) fn psd_errors() -> Vec<PsdError> {
    vec![
        PsdError::Invalid(reason()),
        PsdError::Unsupported(reason()),
        PsdError::LimitExceeded {
            limit: ImportLimit::ChannelDecompressed,
            size: 2048,
            max: 1024,
        },
    ]
}

/// 自動メッシュの生成のエラー (`automesh.*`)。
pub(crate) fn mesh_errors() -> Vec<MeshError> {
    vec![
        MeshError::Empty,
        MeshError::BadImage {
            width: 2,
            height: 2,
            len: 12,
        },
        MeshError::InvalidOptions("margin"),
        MeshError::TooComplex,
        MeshError::TooLarge {
            width: 20_000,
            height: 20_000,
        },
    ]
}

/// 親の座標系とキャンバス座標の写像のエラー (`world.*`)。
pub(crate) fn world_errors() -> Vec<WorldError> {
    vec![
        WorldError::Evaluate("detail".into()),
        WorldError::LengthMismatch {
            points: 2,
            guesses: 1,
        },
        WorldError::Singular { index: 0 },
        WorldError::NotConverged {
            index: 0,
            residual: 0.5,
        },
    ]
}

/// 法線マップの生成のエラー (`normal_map.*`)。
pub(crate) fn normal_map_errors() -> Vec<NormalMapError> {
    vec![
        NormalMapError::EmptyImage {
            width: 0,
            height: 4,
        },
        NormalMapError::SizeMismatch {
            width: 2,
            height: 2,
            expected: 4,
            found: 3,
        },
        NormalMapError::InvalidOption {
            field: "strength",
            value: -1.0,
        },
        NormalMapError::GridTooLarge {
            width: 1e6,
            height: 1e6,
            max: 1 << 24,
        },
    ]
}

/// モジュールの合成のエラー (`compose.*`)。省けるもの (`actual`) は、ある見本と無い見本の両方。
pub(crate) fn compose_errors() -> Vec<ComposeError> {
    let module = || "hat".to_owned();
    vec![
        ComposeError::InvalidModuleName("a/b".into()),
        ComposeError::DuplicateModule(module()),
        ComposeError::UnknownSlot {
            module: module(),
            slot: "head".into(),
        },
        ComposeError::SlotOccupied {
            slot: "head".into(),
            first: "cap".into(),
            second: module(),
        },
        ComposeError::AvatarLevel {
            module: module(),
            required: 2,
            actual: None,
        },
        ComposeError::AvatarLevel {
            module: module(),
            required: 2,
            actual: Some(1),
        },
        ComposeError::MissingSlot {
            module: module(),
            slot: "head".into(),
        },
        ComposeError::MissingParameter {
            module: module(),
            parameter: "AngleX".into(),
        },
        ComposeError::ParameterNotExposed {
            module: module(),
            slot: "head".into(),
            parameter: "Blush".into(),
        },
        ComposeError::UnknownMask {
            module: module(),
            mesh: "Brim".into(),
            mask: "Face".into(),
        },
        ComposeError::TextureOutOfRange {
            module: module(),
            mesh: "Brim".into(),
            texture: 3,
        },
        ComposeError::UnknownCollider {
            module: module(),
            rig: "Ribbon".into(),
            collider: "Head".into(),
        },
        ComposeError::PoseNotExposed {
            module: module(),
            slot: "head".into(),
            part: "ArmL".into(),
        },
        ComposeError::Invalid(vec![ValidationError::DuplicateId {
            kind: "mesh",
            id: id(),
        }]),
    ]
}
