//! エラーのコードと引数 ([`Coded`]) の実装。コードの一覧は `docs/editor.md#エラーと警告のコード`。
//! **コードは公開の約束なので名前を変えない** (ADR 0029)。新しいバリアントには新しいコードを付ける。
//!
//! ほかのクレートのエラー (検証・合成・推定・自動メッシュなど) を含むものは、いまはその英語の文を
//! `detail` (`details`) の引数で渡す。

use ugoe_format::{FormatError, ReadLimit};
use ugoe_model::avatar::check::ProfileIssue;
use ugoe_model::validate::ValidationError;
use ugoe_runtime::CompileError;

use crate::atlas::AtlasError;
use crate::document::EditError;
use crate::export::ExportError;
use crate::grid::CellError;
use crate::import::PsdImportError;
use crate::lighting::LightingError;
use crate::message::{Coded, Message, MessageArgs};
use crate::message_args as args;
use crate::module_project::ModuleError;
use crate::overflow::OverflowError;
use crate::project::ProjectError;
use crate::proposal::ProposalError;
use crate::space::SpaceError;

/// 検証のエラーの一覧の引数 (`count` と、文を `; ` でつないだ `details`)。
fn validation_args(errors: &[ValidationError]) -> MessageArgs {
    let details = errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    args! { "count" => errors.len(), "details" => details }
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
            } => args! { "target" => target, "interpolation" => format!("{interpolation:?}") },
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
        }
    }

    fn cause(&self) -> Option<Message> {
        match self {
            Self::InvalidCell { error, .. } => Some(error.to_message()),
            _ => None,
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
            Self::Export(_) | Self::MissingModuleLink | Self::NotAModule => args! {},
        }
    }

    fn cause(&self) -> Option<Message> {
        match self {
            Self::Format { source, .. } | Self::Export(source) => Some(source.to_message()),
            Self::Base { source, .. } => Some(source.to_message()),
            Self::Module(error) => error.cause(),
            _ => None,
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
}

impl Coded for AtlasError {
    fn code(&self) -> &'static str {
        match self {
            Self::MissingTexture { .. } => "atlas.missing_texture",
            Self::NotPng { .. } => "atlas.not_png",
            Self::TextureOutOfRange { .. } => "atlas.texture_out_of_range",
            Self::InvalidMaxSize => "atlas.invalid_max_size",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::MissingTexture { name } | Self::NotPng { name } => args! { "name" => name },
            Self::TextureOutOfRange { mesh, texture } => {
                args! { "mesh" => mesh, "texture" => *texture }
            }
            Self::InvalidMaxSize => args! {},
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
            Self::InvalidMeshOptions(_) => "psd.invalid_mesh_options",
            Self::Mesh { .. } => "psd.automesh_failed",
        }
    }

    fn args(&self) -> MessageArgs {
        match self {
            Self::Psd(error) => args! { "detail" => error.to_string() },
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
        match self {
            Self::BadMagic | Self::TooLarge => args! {},
            Self::UnsupportedVersion { found, supported } => {
                args! { "found" => *found, "supported" => *supported }
            }
            Self::Invalid(errors) | Self::InvalidModule(errors) | Self::InvalidPhysics(errors) => {
                validation_args(errors)
            }
            Self::UnsupportedFeatures(features) => args! { "features" => features.join(", ") },
            Self::UnknownValues(values) => args! { "values" => values.join(", ") },
            Self::ReadLimitExceeded { tag, max, .. } => {
                args! { "chunk" => tag.to_string(), "max" => *max }
            }
            other => args! { "detail" => other.to_string() },
        }
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
}
