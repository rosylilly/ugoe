//! エラーのコードと引数 ([`Coded`]) の実装。コードの一覧は `docs/editor.md#エラーと警告のコード`。
//! **コードは公開の約束なので名前を変えない** (ADR 0029)。新しいバリアントには新しいコードを付ける。
//!
//! ほかのクレートのエラーを含むものは、その英語の文を `detail` (`details`) の引数で渡す。検証・合成・推定・法線マップの
//! エラーは、それに加えて元のエラーを原因 (`cause`。検証の一覧は `items`) に載せ、画面はそれを訳して `detail`
//! (`details`) の代わりに差し込む (T0163)。自動メッシュなどはまだ英語の文だけ。

use ugoe_format::{FormatError, ReadLimit};
use ugoe_model::KeyformInterpolation;
use ugoe_model::avatar::check::ProfileIssue;
use ugoe_model::validate::{DESCRIBED_ERRORS_MAX, ValidationError};
use ugoe_rigging::{InferError, KeyformTarget, NormalMapError, WorldError};
use ugoe_runtime::{CompileError, ComposeError};

use crate::atlas::AtlasError;
use crate::document::EditError;
use crate::export::ExportError;
use crate::grid::CellError;
use crate::import::{PsdImportError, PsdLimit};
use crate::lighting::LightingError;
use crate::message::{Coded, Message, MessageArgs};
use crate::message_args as args;
use crate::module_project::ModuleError;
use crate::overflow::OverflowError;
use crate::project::ProjectError;
use crate::proposal::ProposalError;
use crate::space::SpaceError;

/// 検証のエラーの一覧の引数 (`count` と、文を `; ` でつないだ `details`)。
///
/// `details` に並べるのは `describe_errors` と同じく先頭の [`DESCRIBED_ERRORS_MAX`] 件までで、残りは
/// `; and {残り} more` にまとめる (`docs/spec/model.md#検証エラーの文`)。`count` は全件の数。
fn validation_args(errors: &[ValidationError]) -> MessageArgs {
    let mut details = errors
        .iter()
        .take(DESCRIBED_ERRORS_MAX)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    if let Some(rest) = errors.len().checked_sub(DESCRIBED_ERRORS_MAX)
        && rest > 0
    {
        details.push_str(&format!("; and {rest} more"));
    }
    args! { "count" => errors.len(), "details" => details }
}

/// 検証のエラーの一覧の中身 ([`Message::items`])。`details` と同じく先頭の [`DESCRIBED_ERRORS_MAX`] 件まで。
fn validation_items(errors: &[ValidationError]) -> Vec<Message> {
    errors
        .iter()
        .take(DESCRIBED_ERRORS_MAX)
        .map(Coded::to_message)
        .collect()
}

/// 数の並びの引数 (`, ` でつなぐ)。
fn join<T: ToString>(values: &[T]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// 推定の対象の表記 (``mesh `EyeL` ``。[`crate::command::KeyformTarget`] の表記と同じ)。
fn target_text(target: &KeyformTarget) -> String {
    let (kind, id) = match target {
        KeyformTarget::Part(id) => ("part", id.as_str()),
        KeyformTarget::Mesh(id) => ("mesh", id.as_str()),
        KeyformTarget::Deformer(id) => ("deformer", id.as_str()),
        KeyformTarget::Glue(id) => ("glue", id.as_str()),
    };
    format!("{kind} `{id}`")
}

/// 補間の種類の JSON の名前 (`linear`・`smooth`)。未知の値は `unknown`。
fn interpolation_name(interpolation: KeyformInterpolation) -> &'static str {
    match interpolation {
        KeyformInterpolation::Linear => "linear",
        KeyformInterpolation::Smooth => "smooth",
        KeyformInterpolation::Unknown => "unknown",
    }
}

impl Coded for EditError {
    fn code(&self) -> &'static str {
        match self {
            Self::DuplicateId { .. } => "edit.duplicate_id",
            Self::NotFound { .. } => "edit.not_found",
            Self::InvalidRange { .. } => "edit.invalid_range",
            Self::HasChildren { .. } => "edit.has_children",
            Self::SelfReference { .. } => "edit.self_reference",
            Self::DeformerKindMismatch { .. } => "edit.deformer_kind_mismatch",
            Self::InvalidCell { .. } => "edit.invalid_cell",
            Self::InvalidTextureName { .. } => "edit.invalid_texture_name",
            Self::NotPng { .. } => "edit.texture_not_png",
            Self::InvalidCanvas { .. } => "edit.invalid_canvas",
            Self::CannotRegenerateMesh { .. } => "edit.cannot_regenerate_mesh",
            Self::Mesher { .. } => "edit.automesh_failed",
            Self::VertexOutOfRange { .. } => "edit.vertex_out_of_range",
            Self::TooFewVertices { .. } => "edit.too_few_vertices",
            Self::MeshGeometry { .. } => "edit.mesh_geometry",
            Self::InvalidDrawOrder { .. } => "edit.invalid_draw_order",
            Self::KeyformKindMismatch { .. } => "edit.keyform_kind_mismatch",
            Self::Invalid(_) => "edit.invalid_model",
            Self::Inference { .. } => "edit.inference_failed",
            Self::UnsupportedInference { .. } => "edit.unsupported_inference",
            Self::UnsupportedInterpolation { .. } => "edit.unsupported_interpolation",
            Self::DeformerEdit { .. } => "edit.deformer_edit_failed",
            Self::DepthLength { .. } => "edit.depth_length",
            Self::NonFiniteDepth { .. } => "edit.non_finite_depth",
            Self::DepthPointOutOfRange { .. } => "edit.depth_point_out_of_range",
            Self::MirrorPairNotSupported { .. } => "edit.mirror_pair_not_supported",
            Self::InvalidName { .. } => "edit.invalid_name",
            Self::InvalidMotion { .. } => "edit.invalid_motion",
            Self::InvalidExpression { .. } => "edit.invalid_expression",
            Self::InvalidPhysics { .. } => "edit.invalid_physics",
            Self::DrawCloneShape { .. } => "edit.draw_clone_shape",
            Self::NotDrawClone { .. } => "edit.not_draw_clone",
            Self::InvalidAvatarProfile { .. } => "edit.invalid_avatar_profile",
            Self::InvalidNormalMap { .. } => "edit.invalid_normal_map",
            Self::InvalidSlot { .. } => "edit.invalid_slot",
            Self::InvalidGlue { .. } => "edit.invalid_glue",
            Self::ReadOnlyBase { .. } => "edit.read_only_base",
            Self::ModuleShape { .. } => "edit.module_shape",
            Self::NotModule => "edit.not_module",
            Self::Module(error) => error.code(),
            Self::TextureTooLarge { .. } => "edit.texture_too_large",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::DuplicateId { kind, id } | Self::NotFound { kind, id } => {
                args! { "kind" => *kind, "id" => id }
            }
            Self::InvalidRange { min, default, max } => {
                args! { "min" => *min, "default" => *default, "max" => *max }
            }
            Self::HasChildren { kind, id, children } => {
                args! { "kind" => *kind, "id" => id, "children" => children.join(", ") }
            }
            Self::SelfReference { kind, id, field } => {
                args! { "kind" => *kind, "id" => id, "field" => *field }
            }
            Self::DeformerKindMismatch { id, expected } => {
                args! { "id" => id, "expected" => *expected }
            }
            Self::InvalidCell { kind, id, .. } => args! { "kind" => *kind, "id" => id },
            Self::InvalidTextureName { name } | Self::NotPng { name } => args! { "name" => name },
            Self::InvalidCanvas { width, height } => {
                args! { "width" => *width, "height" => *height }
            }
            Self::CannotRegenerateMesh { id, reason } | Self::MeshGeometry { id, reason } => {
                args! { "id" => id, "reason" => reason }
            }
            Self::Mesher { id, message } => args! { "id" => id, "detail" => message },
            Self::VertexOutOfRange {
                id,
                index,
                vertices,
            } => args! { "id" => id, "index" => *index, "vertices" => *vertices },
            Self::TooFewVertices { id, remaining } => {
                args! { "id" => id, "remaining" => *remaining }
            }
            Self::InvalidDrawOrder { id, draw_order } => {
                args! { "id" => id, "draw_order" => *draw_order }
            }
            Self::KeyformKindMismatch {
                kind,
                id,
                expected,
                actual,
            } => args! {
                "kind" => *kind,
                "id" => id,
                "expected" => *expected,
                "actual" => *actual,
            },
            Self::Invalid(errors) => validation_args(errors),
            Self::Inference { target, error } => {
                args! { "target" => target, "detail" => error.to_string() }
            }
            Self::UnsupportedInference { target } | Self::MirrorPairNotSupported { target } => {
                args! { "target" => target }
            }
            Self::UnsupportedInterpolation {
                target,
                interpolation,
            } => {
                args! { "target" => target, "interpolation" => interpolation_name(*interpolation) }
            }
            Self::DeformerEdit { reason }
            | Self::InvalidPhysics { reason }
            | Self::InvalidAvatarProfile { reason }
            | Self::ModuleShape { reason } => args! { "reason" => reason },
            Self::DepthLength {
                target,
                expected,
                actual,
            } => args! { "target" => target, "expected" => *expected, "actual" => *actual },
            Self::NonFiniteDepth { target, value } => {
                args! { "target" => target, "value" => *value }
            }
            Self::DepthPointOutOfRange {
                target,
                index,
                points,
            } => args! { "target" => target, "index" => *index, "points" => *points },
            Self::InvalidName { kind, name } => args! { "kind" => *kind, "name" => name },
            Self::InvalidMotion { motion, reason } => {
                args! { "motion" => motion, "reason" => reason }
            }
            Self::InvalidExpression { expression, reason } => {
                args! { "expression" => expression, "reason" => reason }
            }
            Self::DrawCloneShape { id, original } => args! { "id" => id, "original" => original },
            Self::NotDrawClone { id } => args! { "id" => id },
            Self::InvalidNormalMap { texture, reason } => {
                args! { "texture" => texture, "reason" => reason }
            }
            Self::InvalidSlot { id, reason } | Self::InvalidGlue { id, reason } => {
                args! { "id" => id, "reason" => reason }
            }
            Self::ReadOnlyBase { element } => args! { "element" => element },
            Self::NotModule => args! {},
            Self::Module(error) => error.args(),
            Self::TextureTooLarge {
                texture,
                width,
                height,
                max,
            } => {
                args! { "texture" => texture, "width" => *width, "height" => *height, "max" => *max }
            }
        }
    }

    fn cause(&self) -> Option<Message> {
        match self {
            Self::InvalidCell { error, .. } => Some(error.to_message()),
            Self::Inference { error, .. } => Some(error.to_message()),
            Self::Module(error) => error.cause(),
            _ => None,
        }
    }

    fn items(&self) -> Vec<Message> {
        match self {
            Self::Invalid(errors) => validation_items(errors),
            Self::Module(error) => error.items(),
            _ => Vec::new(),
        }
    }
}

impl Coded for CellError {
    fn code(&self) -> &'static str {
        match self {
            Self::MissingAxis(_) => "cell.missing_axis",
            Self::UnknownAxis(_) => "cell.unknown_axis",
            Self::UnknownKey { .. } => "cell.unknown_key",
            Self::OutOfRange { .. } => "cell.out_of_range",
            Self::DuplicateKey { .. } => "cell.duplicate_key",
            Self::NonFiniteKey { .. } => "cell.non_finite_key",
            Self::KeyformCount { .. } => "cell.keyform_count",
            Self::ShapeMismatch { .. } => "cell.shape_mismatch",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::MissingAxis(parameter) | Self::UnknownAxis(parameter) => {
                args! { "parameter" => parameter }
            }
            Self::UnknownKey { parameter, value }
            | Self::DuplicateKey { parameter, value }
            | Self::NonFiniteKey { parameter, value } => {
                args! { "parameter" => parameter, "value" => *value }
            }
            Self::OutOfRange { index, len } => args! { "index" => *index, "len" => *len },
            Self::KeyformCount { expected, actual } | Self::ShapeMismatch { expected, actual } => {
                args! { "expected" => *expected, "actual" => *actual }
            }
        }
    }
}

impl Coded for ProposalError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "proposal.not_found",
            Self::Stale => "proposal.stale",
            Self::InvalidIndex { .. } => "proposal.invalid_index",
            Self::NothingSelected => "proposal.nothing_selected",
            Self::Rejected(_) => "proposal.rejected",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::NotFound(id) => args! { "id" => *id },
            Self::InvalidIndex { index } => args! { "index" => *index },
            Self::Stale | Self::NothingSelected | Self::Rejected(_) => args! {},
        }
    }

    fn cause(&self) -> Option<Message> {
        match self {
            Self::Rejected(error) => Some(error.to_message()),
            _ => None,
        }
    }
}

impl Coded for ProjectError {
    fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "project.io",
            Self::Json { .. } => "project.json",
            Self::Format { .. } => "project.format",
            Self::NotAProject { .. } => "project.not_a_project",
            Self::NotEmpty { .. } => "project.not_empty",
            Self::UnsupportedVersion { .. } => "project.unsupported_version",
            Self::InvalidName { .. } => "project.invalid_name",
            Self::DuplicateName { .. } => "project.duplicate_name",
            Self::MissingTexture { .. } => "project.missing_texture",
            Self::NotPng { .. } => "project.not_png",
            Self::Export(_) => "project.export_failed",
            Self::UnreadableModuleFile(_) => "project.module_file_unreadable",
            Self::MissingModuleLink => "project.missing_module_link",
            Self::BaseNotFound { .. } => "project.base_not_found",
            Self::Base { .. } => "project.base_unreadable",
            Self::BaseRefused { .. } => "project.base_refused",
            Self::BaseIsModule { .. } => "project.base_is_module",
            Self::UnsupportedBaseTexture { .. } => "project.unsupported_base_texture",
            Self::UnsupportedTexture { .. } => "project.unsupported_texture",
            Self::Module(error) => error.code(),
            Self::NotAModule => "project.not_a_module",
            Self::InvalidModule(_) => "project.invalid_module",
        }
    }

    fn args(&self) -> MessageArgs {
        let path = |path: &std::path::Path| path.display().to_string();
        match self {
            Self::Io { path: p, source } => {
                args! { "path" => path(p), "detail" => source.to_string() }
            }
            Self::Json { path: p, source } => {
                args! { "path" => path(p), "detail" => source.to_string() }
            }
            Self::Format { path: p, .. }
            | Self::NotAProject { path: p }
            | Self::NotEmpty { path: p }
            | Self::NotPng { path: p }
            | Self::BaseNotFound { path: p }
            | Self::Base { path: p, .. }
            | Self::BaseIsModule { path: p } => args! { "path" => path(p) },
            Self::UnsupportedVersion { found, supported } => {
                args! { "found" => *found, "supported" => *supported }
            }
            Self::InvalidName { kind, name } | Self::DuplicateName { kind, name } => {
                args! { "kind" => *kind, "name" => name }
            }
            Self::MissingTexture { name }
            | Self::UnsupportedBaseTexture { name }
            | Self::UnsupportedTexture { name } => args! { "name" => name },
            Self::BaseRefused { path: p, reason } => {
                args! { "path" => path(p), "reason" => reason }
            }
            Self::Module(error) => error.args(),
            Self::InvalidModule(errors) => validation_args(errors),
            Self::Export(_)
            | Self::UnreadableModuleFile(_)
            | Self::MissingModuleLink
            | Self::NotAModule => args! {},
        }
    }

    fn cause(&self) -> Option<Message> {
        match self {
            Self::Format { source, .. }
            | Self::Export(source)
            | Self::UnreadableModuleFile(source) => Some(source.to_message()),
            Self::Base { source, .. } => Some(source.to_message()),
            Self::Module(error) => error.cause(),
            _ => None,
        }
    }

    fn items(&self) -> Vec<Message> {
        match self {
            Self::InvalidModule(errors) => validation_items(errors),
            Self::Module(error) => error.items(),
            _ => Vec::new(),
        }
    }
}

impl Coded for ModuleError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidName(_) => "module.invalid_name",
            Self::UnknownSlot(_) => "module.unknown_slot",
            Self::Compose(_) => "module.compose_failed",
            Self::ColliderNeedsBasePhysics { .. } => "module.collider_needs_base_physics",
            Self::ReadOnlyBase { .. } => "module.read_only_base",
            Self::Shape { .. } => "module.shape",
            Self::NotModule => "module.not_module",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::InvalidName(name) => args! { "name" => name },
            Self::UnknownSlot(slot) => args! { "slot" => slot },
            Self::Compose(error) => args! { "detail" => error.to_string() },
            Self::ColliderNeedsBasePhysics {
                rig,
                collider,
                slot,
            } => args! { "rig" => rig, "collider" => collider, "slot" => slot },
            Self::ReadOnlyBase { element } => args! { "element" => element },
            Self::Shape { reason } => args! { "reason" => reason },
            Self::NotModule => args! {},
        }
    }

    fn cause(&self) -> Option<Message> {
        match self {
            Self::Compose(error) => Some(error.to_message()),
            _ => None,
        }
    }
}

impl Coded for ExportError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidMaxSize(_) => "export.invalid_max_size",
            Self::InvalidPadding(_) => "export.invalid_padding",
            Self::Settings(_) => "export.invalid_settings",
            Self::Atlas(error) => error.code(),
            Self::Project(error) => error.code(),
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::InvalidMaxSize(value) => {
                args! { "value" => *value, "max" => crate::export::MAX_ATLAS_SIZE }
            }
            Self::InvalidPadding(value) => {
                args! { "value" => *value, "max" => crate::export::MAX_ATLAS_PADDING }
            }
            Self::Settings(error) => args! { "detail" => error.to_string() },
            Self::Atlas(error) => error.args(),
            Self::Project(error) => error.args(),
        }
    }

    fn cause(&self) -> Option<Message> {
        match self {
            Self::Atlas(error) => error.cause(),
            Self::Project(error) => error.cause(),
            _ => None,
        }
    }

    fn items(&self) -> Vec<Message> {
        match self {
            Self::Project(error) => error.items(),
            _ => Vec::new(),
        }
    }
}

impl Coded for AtlasError {
    fn code(&self) -> &'static str {
        match self {
            Self::MissingTexture { .. } => "atlas.missing_texture",
            Self::NotPng { .. } => "atlas.not_png",
            Self::TextureOutOfRange { .. } => "atlas.texture_out_of_range",
            Self::InvalidMaxSize => "atlas.invalid_max_size",
            Self::TextureTooLarge { .. } => "atlas.texture_too_large",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::MissingTexture { name } | Self::NotPng { name } => args! { "name" => name },
            Self::TextureOutOfRange { mesh, texture } => {
                args! { "mesh" => mesh, "texture" => *texture }
            }
            Self::InvalidMaxSize => args! {},
            Self::TextureTooLarge {
                name,
                width,
                height,
                max,
            } => args! { "name" => name, "width" => *width, "height" => *height, "max" => *max },
        }
    }
}

impl Coded for OverflowError {
    fn code(&self) -> &'static str {
        match self {
            Self::UnknownParameter { .. } => "overflow.unknown_parameter",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::UnknownParameter { target, parameter } => {
                args! { "target" => target.to_string(), "parameter" => parameter }
            }
        }
    }
}

impl Coded for LightingError {
    fn code(&self) -> &'static str {
        match self {
            Self::TextureNotFound(_) => "lighting.texture_not_found",
            Self::UnreadableTexture(_) => "lighting.unreadable_texture",
            Self::UnreadableNormalMap => "lighting.unreadable_normal_map",
            Self::Generate(_) => "lighting.generate_failed",
            Self::InvalidPreview(_) => "lighting.invalid_preview",
            Self::Settings(_) => "lighting.invalid_settings",
            Self::TextureTooLarge { .. } => "lighting.texture_too_large",
            Self::NormalMapTooLarge { .. } => "lighting.normal_map_too_large",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::TextureNotFound(texture) | Self::UnreadableTexture(texture) => {
                args! { "texture" => texture }
            }
            Self::UnreadableNormalMap => args! {},
            Self::Generate(error) => args! { "detail" => error.to_string() },
            Self::InvalidPreview(reason) => args! { "reason" => reason },
            Self::Settings(error) => args! { "detail" => error.to_string() },
            Self::TextureTooLarge {
                texture,
                width,
                height,
                max,
            } => {
                args! { "texture" => texture, "width" => *width, "height" => *height, "max" => *max }
            }
            Self::NormalMapTooLarge { width, height, max } => {
                args! { "width" => *width, "height" => *height, "max" => *max }
            }
        }
    }

    fn cause(&self) -> Option<Message> {
        match self {
            Self::Generate(error) => Some(error.to_message()),
            _ => None,
        }
    }
}

impl Coded for SpaceError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "space.not_found",
            Self::NoSpace { .. } => "space.no_space",
            Self::Evaluate(_) => "space.evaluate_failed",
            Self::LengthMismatch { .. } => "space.length_mismatch",
            Self::Singular { .. } => "space.singular",
            Self::NotConverged { .. } => "space.not_converged",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::NotFound { kind, id } | Self::NoSpace { kind, id } => {
                args! { "kind" => *kind, "id" => id }
            }
            Self::Evaluate(detail) => args! { "detail" => detail },
            Self::LengthMismatch { points, guesses } => {
                args! { "points" => *points, "guesses" => *guesses }
            }
            Self::Singular { index } => args! { "index" => *index },
            Self::NotConverged { index, residual } => {
                args! { "index" => *index, "residual" => *residual }
            }
        }
    }
}

impl Coded for PsdImportError {
    fn code(&self) -> &'static str {
        match self {
            Self::Psd(_) => "psd.read_failed",
            // 読み込みの上限 (ファイルが壊れているとは限らない)。
            Self::LimitExceeded {
                limit: PsdLimit::Channel,
                ..
            } => "psd.channel_limit_exceeded",
            Self::LimitExceeded {
                limit: PsdLimit::Total,
                ..
            } => "psd.total_limit_exceeded",
            Self::InvalidMeshOptions(_) => "psd.invalid_mesh_options",
            Self::Mesh { .. } => "psd.automesh_failed",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::Psd(error) => args! { "detail" => error.to_string() },
            Self::LimitExceeded { max, .. } => args! { "max" => *max },
            Self::InvalidMeshOptions(reason) => args! { "reason" => *reason },
            Self::Mesh { layer, source } => {
                args! { "layer" => layer, "detail" => source.to_string() }
            }
        }
    }
}

impl Coded for FormatError {
    fn code(&self) -> &'static str {
        match self {
            Self::BadMagic => "format.not_ugoe_file",
            Self::UnsupportedVersion { .. } => "format.unsupported_version",
            Self::TooLarge => "format.too_large",
            Self::Invalid(_) => "format.invalid_model",
            Self::InvalidModule(_) => "format.invalid_module",
            Self::InvalidPhysics(_) => "format.invalid_physics",
            Self::InvalidMotion { .. } => "format.invalid_motion",
            Self::InvalidExpression { .. } => "format.invalid_expression",
            Self::UnsupportedFeatures(_) => "format.unsupported_features",
            Self::UnknownValues(_) => "format.unknown_values",
            // 読み込みの上限 (ファイルが壊れているとは限らない)。
            Self::ReadLimitExceeded {
                limit: ReadLimit::TotalDecompressed,
                ..
            } => "format.total_limit_exceeded",
            Self::ReadLimitExceeded { .. } => "format.chunk_limit_exceeded",
            // 壊れたファイル (チャンク・圧縮・テクスチャ・MessagePack・JSON)。細かい理由は `detail`。
            _ => "format.corrupt",
        }
    }

    fn args(&self) -> MessageArgs {
        // 検証エラーの列を持つ種類は、種類ごとに展開せずに `validation_errors` で読む (種類が増えても取りこぼさない)。
        if let Some(errors) = self.validation_errors() {
            let mut out = validation_args(errors);
            if let Self::InvalidMotion { name, .. } | Self::InvalidExpression { name, .. } = self {
                out.extend(args! { "name" => name });
            }
            return out;
        }
        match self {
            Self::BadMagic | Self::TooLarge => args! {},
            Self::UnsupportedVersion { found, supported } => {
                args! { "found" => *found, "supported" => *supported }
            }
            Self::UnsupportedFeatures(features) => args! { "features" => features.join(", ") },
            Self::UnknownValues(values) => args! { "values" => values.join(", ") },
            Self::ReadLimitExceeded { tag, max, .. } => {
                args! { "chunk" => tag.to_string(), "max" => *max }
            }
            other => args! { "detail" => other.to_string() },
        }
    }

    fn items(&self) -> Vec<Message> {
        self.validation_errors()
            .map(validation_items)
            .unwrap_or_default()
    }
}

/// アバタープロファイルの検査の問題 (`avatar.*`。[`crate::avatar::AvatarCheck`])。
impl Coded for ProfileIssue {
    fn code(&self) -> &'static str {
        match self {
            Self::MissingProfile => "avatar.missing_profile",
            Self::NewerProfileVersion { .. } => "avatar.newer_profile_version",
            Self::InvalidLevel { .. } => "avatar.invalid_level",
            Self::MissingMetadata { .. } => "avatar.missing_metadata",
            Self::MissingParameter { .. } => "avatar.missing_parameter",
            Self::TooFewExtendedParameters { .. } => "avatar.too_few_extended_parameters",
            Self::ParameterRange { .. } => "avatar.parameter_range",
            Self::UnknownReference { .. } => "avatar.unknown_reference",
            Self::UnknownTrackingInput { .. } => "avatar.unknown_tracking_input",
            Self::MissingTrackingTarget { .. } => "avatar.missing_tracking_target",
            Self::DuplicateHitAreaName { .. } => "avatar.duplicate_hit_area_name",
            Self::UnknownValue { .. } => "avatar.unknown_value",
            Self::InvalidValue { .. } => "avatar.invalid_value",
            Self::InternalTrackingTarget { .. } => "avatar.internal_tracking_target",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::MissingProfile => args! {},
            Self::NewerProfileVersion { version, supported } => {
                args! { "version" => *version, "supported" => *supported }
            }
            Self::InvalidLevel { level, max } => args! { "level" => *level, "max" => *max },
            Self::MissingMetadata { field } => args! { "field" => *field },
            Self::MissingParameter { level, id } => args! { "level" => *level, "id" => *id },
            Self::TooFewExtendedParameters { required, found } => {
                args! { "required" => *required, "found" => *found }
            }
            Self::ParameterRange {
                id,
                min,
                default,
                max,
                expected,
            } => args! {
                "id" => id,
                "min" => *min,
                "default" => *default,
                "max" => *max,
                "expected_min" => expected.0,
                "expected_default" => expected.1,
                "expected_max" => expected.2,
            },
            Self::UnknownReference {
                field,
                target_kind,
                target,
            } => args! { "field" => field, "target_kind" => *target_kind, "target" => target },
            Self::UnknownTrackingInput { field, kind, input } => {
                args! { "field" => field, "kind" => *kind, "input" => input }
            }
            Self::MissingTrackingTarget { field } | Self::UnknownValue { field } => {
                args! { "field" => field }
            }
            Self::DuplicateHitAreaName { name } => args! { "name" => name },
            Self::InvalidValue {
                field,
                value,
                expected,
            } => args! { "field" => field, "value" => *value, "expected" => *expected },
            Self::InternalTrackingTarget { field, id } => args! { "field" => field, "id" => id },
        }
    }
}

/// 評価のためにモデルを前処理できない (物理の検査など)。
impl Coded for CompileError {
    fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "compile.invalid_model",
            Self::UnknownParameter(_) => "compile.unknown_parameter",
            Self::Unsupported(_) => "compile.unsupported",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::Invalid(errors) => validation_args(errors),
            Self::UnknownParameter(parameter) => args! { "parameter" => parameter },
            Self::Unsupported(feature) => args! { "feature" => *feature },
        }
    }

    fn items(&self) -> Vec<Message> {
        match self {
            Self::Invalid(errors) => validation_items(errors),
            _ => Vec::new(),
        }
    }
}

/// モデルの検証のエラー (`validation.*`)。コードは `validation.` と [`ValidationError::code`] をつないだもの、
/// 引数はバリアントのフィールド (並びは `, ` でつなぐ)。
impl Coded for ValidationError {
    fn code(&self) -> &'static str {
        match self {
            Self::DuplicateId { .. } => "validation.duplicate_id",
            Self::KeyformCount { .. } => "validation.keyform_count",
            Self::KeysNotAscending { .. } => "validation.keys_not_ascending",
            Self::EmptyKeys { .. } => "validation.empty_keys",
            Self::DuplicateBinding { .. } => "validation.duplicate_binding",
            Self::IndicesNotTriangles { .. } => "validation.indices_not_triangles",
            Self::IndexOutOfRange { .. } => "validation.index_out_of_range",
            Self::VertexCount { .. } => "validation.vertex_count",
            Self::WarpPointCount { .. } => "validation.warp_point_count",
            Self::EmptyWarpGrid { .. } => "validation.empty_warp_grid",
            Self::UnknownReference { .. } => "validation.unknown_reference",
            Self::TextureOutOfRange { .. } => "validation.texture_out_of_range",
            Self::GlueVertexOutOfRange { .. } => "validation.glue_vertex_out_of_range",
            Self::ParentCycle { .. } => "validation.parent_cycle",
            Self::MaskCycle { .. } => "validation.mask_cycle",
            Self::ParameterRange { .. } => "validation.parameter_range",
            Self::DuplicatePosePart { .. } => "validation.duplicate_pose_part",
            Self::InvalidModuleName { .. } => "validation.invalid_module_name",
            Self::OriginCount { .. } => "validation.origin_count",
            Self::UnboundInferenceParameter { .. } => "validation.unbound_inference_parameter",
            Self::AsymmetricMirror { .. } => "validation.asymmetric_mirror",
            Self::DepthCount { .. } => "validation.depth_count",
            Self::DeformerKindMismatch { .. } => "validation.deformer_kind_mismatch",
            Self::WeightCount { .. } => "validation.weight_count",
            Self::ParameterKindMismatch { .. } => "validation.parameter_kind_mismatch",
            Self::InvalidBlendShape { .. } => "validation.invalid_blend_shape",
            Self::InvalidValue { .. } => "validation.invalid_value",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::DuplicateId { kind, id } => args! { "kind" => *kind, "id" => id },
            Self::KeyformCount {
                kind,
                id,
                expected,
                actual,
            }
            | Self::OriginCount {
                kind,
                id,
                expected,
                actual,
            }
            | Self::DepthCount {
                kind,
                id,
                expected,
                actual,
            } => args! {
                "kind" => *kind,
                "id" => id,
                "expected" => *expected,
                "actual" => *actual,
            },
            Self::KeysNotAscending {
                kind,
                id,
                parameter,
                keys,
            } => args! {
                "kind" => *kind,
                "id" => id,
                "parameter" => parameter,
                "keys" => join(keys),
            },
            Self::EmptyKeys {
                kind,
                id,
                parameter,
            }
            | Self::DuplicateBinding {
                kind,
                id,
                parameter,
            }
            | Self::UnboundInferenceParameter {
                kind,
                id,
                parameter,
            } => args! { "kind" => *kind, "id" => id, "parameter" => parameter },
            Self::IndicesNotTriangles { id, len } => args! { "id" => id, "len" => *len },
            Self::IndexOutOfRange {
                id,
                index,
                vertices,
            } => args! { "id" => id, "index" => *index, "vertices" => *vertices },
            Self::VertexCount {
                id,
                expected,
                actual,
            }
            | Self::WarpPointCount {
                id,
                expected,
                actual,
            } => args! { "id" => id, "expected" => *expected, "actual" => *actual },
            Self::EmptyWarpGrid { id, rows, cols } => {
                args! { "id" => id, "rows" => *rows, "cols" => *cols }
            }
            Self::UnknownReference {
                kind,
                id,
                field,
                target_kind,
                target,
            } => args! {
                "kind" => *kind,
                "id" => id,
                "field" => *field,
                "target_kind" => *target_kind,
                "target" => target,
            },
            Self::TextureOutOfRange {
                id,
                texture,
                textures,
            } => args! { "id" => id, "texture" => *texture, "textures" => *textures },
            Self::GlueVertexOutOfRange {
                id,
                mesh,
                index,
                vertices,
            } => args! {
                "id" => id,
                "mesh" => mesh,
                "index" => *index,
                "vertices" => *vertices,
            },
            Self::ParentCycle { kind, cycle } => {
                args! { "kind" => *kind, "cycle" => cycle.join(", ") }
            }
            Self::MaskCycle { cycle } => args! { "cycle" => cycle.join(", ") },
            Self::ParameterRange {
                id,
                min,
                default,
                max,
            } => args! { "id" => id, "min" => *min, "default" => *default, "max" => *max },
            Self::DuplicatePosePart { part } => args! { "part" => part },
            Self::InvalidModuleName { name } => args! { "name" => name },
            Self::AsymmetricMirror {
                kind,
                id,
                mirror,
                back,
            } => {
                let mut out = args! { "kind" => *kind, "id" => id, "mirror" => mirror };
                // 指し返す先が無ければ `back` は渡さない。
                if let Some(back) = back {
                    out.extend(args! { "back" => back });
                }
                out
            }
            Self::DeformerKindMismatch {
                kind,
                id,
                field,
                deformer,
                expected,
            } => args! {
                "kind" => *kind,
                "id" => id,
                "field" => *field,
                "deformer" => deformer,
                "expected" => *expected,
            },
            Self::WeightCount {
                rig,
                output,
                expected,
                actual,
            } => args! {
                "rig" => rig,
                "output" => *output,
                "expected" => *expected,
                "actual" => *actual,
            },
            Self::ParameterKindMismatch {
                kind,
                id,
                parameter,
                expected,
            } => args! {
                "kind" => *kind,
                "id" => id,
                "parameter" => parameter,
                "expected" => *expected,
            },
            Self::InvalidBlendShape {
                kind,
                id,
                index,
                reason,
            } => args! { "kind" => *kind, "id" => id, "index" => *index, "reason" => *reason },
            Self::InvalidValue {
                field,
                value,
                expected,
            } => args! { "field" => field, "value" => *value, "expected" => *expected },
        }
    }
}

/// キーフォームの推定のエラー (`infer.*`。[`InferError::code`])。
impl Coded for InferError {
    fn code(&self) -> &'static str {
        match self {
            Self::UnknownParameter { .. } => "infer.unknown_parameter",
            Self::MissingNeutralKey { .. } => "infer.missing_neutral_key",
            Self::DuplicateAxis { .. } => "infer.duplicate_axis",
            Self::NonFiniteKey { .. } => "infer.non_finite_key",
            Self::KeysNotAscending { .. } => "infer.keys_not_ascending",
            Self::TooManyAxes { .. } => "infer.too_many_axes",
            Self::GridTooLarge => "infer.grid_too_large",
            Self::GridSizeMismatch { .. } => "infer.grid_size_mismatch",
            Self::OriginsLengthMismatch { .. } => "infer.origins_length_mismatch",
            Self::BaseNotAuthored { .. } => "infer.base_not_authored",
            Self::ShapeMismatch { .. } => "infer.shape_mismatch",
            Self::UnknownTarget(_) => "infer.unknown_target",
            Self::UnsupportedTarget(_) => "infer.unsupported_target",
            Self::UnknownParent { .. } => "infer.unknown_parent",
            Self::UnsupportedParent { .. } => "infer.unsupported_parent",
            Self::ParentCycle { .. } => "infer.parent_cycle",
            Self::NotMirrorPair { .. } => "infer.not_mirror_pair",
            Self::IncompatibleMirror { .. } => "infer.incompatible_mirror",
            Self::MirrorAxisMismatch { .. } => "infer.mirror_axis_mismatch",
            Self::MissingMirrorKey { .. } => "infer.missing_mirror_key",
            Self::UnmatchedVertices { .. } => "infer.unmatched_vertices",
            Self::ParentTransform(_) => "infer.parent_transform",
            Self::NotDepthRotation(_) => "infer.not_depth_rotation",
            Self::MissingDepth(_) => "infer.missing_depth",
            Self::DepthLengthMismatch { .. } => "infer.depth_length_mismatch",
            Self::MissingRotationAxis { .. } => "infer.missing_rotation_axis",
            Self::NonAffineParent(_) => "infer.non_affine_parent",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::UnknownParameter { parameter }
            | Self::DuplicateAxis { parameter }
            | Self::MirrorAxisMismatch { parameter }
            | Self::MissingRotationAxis { parameter } => {
                args! { "parameter" => parameter.as_str() }
            }
            Self::MissingNeutralKey { parameter, default } => {
                args! { "parameter" => parameter.as_str(), "default" => *default }
            }
            Self::NonFiniteKey { parameter, key } | Self::MissingMirrorKey { parameter, key } => {
                args! { "parameter" => parameter.as_str(), "key" => *key }
            }
            Self::KeysNotAscending { parameter, keys } => {
                args! { "parameter" => parameter.as_str(), "keys" => join(keys) }
            }
            Self::TooManyAxes { count } => args! { "axes" => *count, "max" => 64u32 },
            Self::GridTooLarge | Self::ParentTransform(_) => args! {},
            Self::GridSizeMismatch { expected, found }
            | Self::OriginsLengthMismatch { expected, found }
            | Self::DepthLengthMismatch { expected, found } => {
                args! { "expected" => *expected, "found" => *found }
            }
            Self::BaseNotAuthored { cell } => args! { "cell" => *cell },
            Self::ShapeMismatch {
                cell,
                expected,
                found,
            } => args! { "cell" => *cell, "expected" => *expected, "found" => *found },
            Self::UnknownTarget(target)
            | Self::UnsupportedTarget(target)
            | Self::NotDepthRotation(target)
            | Self::MissingDepth(target)
            | Self::NonAffineParent(target) => args! { "target" => target_text(target) },
            Self::UnknownParent { deformer }
            | Self::UnsupportedParent { deformer }
            | Self::ParentCycle { deformer } => args! { "deformer" => deformer.as_str() },
            Self::NotMirrorPair { from, to } | Self::IncompatibleMirror { from, to } => {
                args! { "from" => target_text(from), "to" => target_text(to) }
            }
            Self::UnmatchedVertices {
                vertices,
                tolerance,
            } => args! { "vertices" => join(vertices), "tolerance" => *tolerance },
        }
    }

    fn cause(&self) -> Option<Message> {
        match self {
            Self::ParentTransform(error) => Some(error.to_message()),
            _ => None,
        }
    }
}

/// 親の座標系とキャンバス座標の写像のエラー (`world.*`。[`WorldError::code`])。
impl Coded for WorldError {
    fn code(&self) -> &'static str {
        match self {
            Self::Evaluate(_) => "world.evaluate",
            Self::LengthMismatch { .. } => "world.length_mismatch",
            Self::Singular { .. } => "world.singular",
            Self::NotConverged { .. } => "world.not_converged",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::Evaluate(detail) => args! { "detail" => detail },
            Self::LengthMismatch { points, guesses } => {
                args! { "points" => *points, "guesses" => *guesses }
            }
            Self::Singular { index } => args! { "index" => *index },
            Self::NotConverged { index, residual } => {
                args! { "index" => *index, "residual" => *residual }
            }
        }
    }
}

/// 法線マップの生成のエラー (`normal_map.*`。[`NormalMapError::code`])。
impl Coded for NormalMapError {
    fn code(&self) -> &'static str {
        match self {
            Self::EmptyImage { .. } => "normal_map.empty_image",
            Self::SizeMismatch { .. } => "normal_map.size_mismatch",
            Self::InvalidOption { .. } => "normal_map.invalid_option",
            Self::GridTooLarge { .. } => "normal_map.grid_too_large",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::EmptyImage { width, height } => {
                args! { "width" => *width, "height" => *height }
            }
            Self::SizeMismatch {
                width,
                height,
                expected,
                found,
            } => args! {
                "width" => *width,
                "height" => *height,
                "expected" => *expected,
                "found" => *found,
            },
            Self::InvalidOption { field, value } => args! { "field" => *field, "value" => *value },
            Self::GridTooLarge { width, height, max } => {
                args! { "width" => *width, "height" => *height, "max" => *max }
            }
        }
    }
}

/// モジュールの合成のエラー (`compose.*`)。コードの後ろの部分と引数の名前は SDK の `DressUpError::code`・`fields` と同じ。
impl Coded for ComposeError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidModuleName(_) => "compose.invalid_module_name",
            Self::DuplicateModule(_) => "compose.duplicate_module",
            Self::UnknownSlot { .. } => "compose.unknown_slot",
            Self::SlotOccupied { .. } => "compose.slot_occupied",
            Self::AvatarLevel { .. } => "compose.avatar_level",
            Self::MissingSlot { .. } => "compose.missing_slot",
            Self::MissingParameter { .. } => "compose.missing_parameter",
            Self::ParameterNotExposed { .. } => "compose.parameter_not_exposed",
            Self::UnknownMask { .. } => "compose.unknown_mask",
            Self::TextureOutOfRange { .. } => "compose.texture_out_of_range",
            Self::UnknownCollider { .. } => "compose.unknown_collider",
            Self::PoseNotExposed { .. } => "compose.pose_not_exposed",
            Self::Invalid(_) => "compose.invalid_composition",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::InvalidModuleName(module) | Self::DuplicateModule(module) => {
                args! { "module" => module }
            }
            Self::UnknownSlot { module, slot } | Self::MissingSlot { module, slot } => {
                args! { "module" => module, "slot" => slot }
            }
            Self::SlotOccupied {
                slot,
                first,
                second,
            } => args! { "slot" => slot, "module" => second, "attached" => first },
            Self::AvatarLevel {
                module,
                required,
                actual,
            } => {
                let mut out = args! { "module" => module, "required" => *required };
                // 本体がアバタープロファイルを持たなければ `actual` は渡さない。
                if let Some(actual) = actual {
                    out.extend(args! { "actual" => *actual });
                }
                out
            }
            Self::MissingParameter { module, parameter } => {
                args! { "module" => module, "parameter" => parameter }
            }
            Self::ParameterNotExposed {
                module,
                slot,
                parameter,
            } => args! { "module" => module, "slot" => slot, "parameter" => parameter },
            Self::UnknownMask { module, mesh, mask } => {
                args! { "module" => module, "mesh" => mesh, "mask" => mask }
            }
            Self::TextureOutOfRange {
                module,
                mesh,
                texture,
            } => args! { "module" => module, "mesh" => mesh, "texture" => *texture },
            Self::UnknownCollider {
                module,
                rig,
                collider,
            } => args! { "module" => module, "rig" => rig, "collider" => collider },
            Self::PoseNotExposed { module, slot, part } => {
                args! { "module" => module, "slot" => slot, "part" => part }
            }
            Self::Invalid(errors) => validation_args(errors),
        }
    }

    fn items(&self) -> Vec<Message> {
        match self {
            Self::Invalid(errors) => validation_items(errors),
            _ => Vec::new(),
        }
    }
}
