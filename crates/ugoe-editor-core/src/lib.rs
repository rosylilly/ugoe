//! ヘッドレスなエディタ中核。設計: `docs/editor.md`。
//!
//! **すべての編集操作は [`Command`] を通す。** GUI (Tauri)・CLI・AI エージェント (MCP) は
//! 同じ `Command` を発行するだけで、それぞれ独自の編集ロジックを持たない。
//! これにより Undo/Redo、操作ログ、AI による提案の差分プレビューが一元化される。

pub mod atlas;
pub mod automesh;
pub mod avatar;
pub mod codegen;
pub mod command;
mod deformer_edit;
pub mod depth;
pub mod document;
pub mod draw_clone;
pub mod export;
pub mod fingerprint;
mod grid;
pub mod ids;
pub mod import;
mod inference;
pub mod keyform;
pub mod lighting;
mod mesh_edit;
pub mod module_project;
pub mod motion_edit;
pub mod overflow;
pub mod physics_check;
pub mod physics_edit;
pub mod physics_template;
pub mod project;
pub mod proposal;
pub mod slot_edit;
pub mod space;
pub mod texture;

pub use atlas::{AtlasOptions, AtlasReport, build_atlas};
pub use automesh::AutoMeshOptions;
pub use command::{ChildRef, Command, InferMethod, KeyformCell, KeyformTarget, VertexMove};
pub use depth::DepthPreset;
pub use document::{ApplyReport, Document, EditError};
pub use export::{ExportReport, ExportSettings, export_bundle_with_settings};
pub use fingerprint::ProjectFingerprint;
pub use grid::CellError;
pub use import::{PsdImport, PsdImportError, PsdImportOptions, PsdImportWarning, import_psd};
pub use inference::{MirrorReport, RederiveWarning};
pub use keyform::Keyform;
pub use module_project::{ModuleBase, ModuleContext, ModuleError};
pub use motion_edit::{Animations, SegmentShape};
pub use physics_template::{PhysicsTemplate, PhysicsTemplateOptions};
pub use project::{
    ModuleExport, ModuleLink, Project, ProjectError, ProjectKind, ProjectMeta, export_base_bundle,
    export_bundle, export_module,
};
pub use proposal::{ProposalCommand, ProposalError, ProposalOrigin, ProposalView, Proposals};
pub use texture::Textures;

#[cfg(test)]
mod atlas_tests;
#[cfg(test)]
mod avatar_tests;
#[cfg(test)]
mod blend_shape_tests;
#[cfg(test)]
mod deformer_edit_tests;
#[cfg(test)]
mod depth_tests;
#[cfg(test)]
mod draw_clone_tests;
#[cfg(test)]
mod export_tests;
#[cfg(test)]
mod fingerprint_tests;
#[cfg(test)]
mod import_tests;
#[cfg(test)]
mod inference_tests;
#[cfg(test)]
mod interpolation_tests;
#[cfg(test)]
mod keyform_tests;
#[cfg(test)]
mod lighting_tests;
#[cfg(test)]
mod mesh_edit_tests;
#[cfg(test)]
mod module_project_tests;
#[cfg(test)]
mod motion_edit_tests;
#[cfg(test)]
mod overflow_tests;
#[cfg(test)]
mod physics_check_tests;
#[cfg(test)]
mod physics_edit_tests;
#[cfg(test)]
mod physics_template_tests;
#[cfg(test)]
mod project_tests;
#[cfg(test)]
mod proposal_tests;
#[cfg(test)]
mod slot_tests;
#[cfg(test)]
mod tests;
