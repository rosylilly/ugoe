//! ヘッドレスなエディタ中核。設計: `docs/editor.md`。
//!
//! **すべての編集操作は [`Command`] を通す。** GUI (Tauri)・CLI・AI エージェント (MCP) は
//! 同じ `Command` を発行するだけで、それぞれ独自の編集ロジックを持たない。
//! これにより Undo/Redo、操作ログ、AI による提案の差分プレビューが一元化される。

pub mod automesh;
pub mod codegen;
pub mod command;
mod deformer_edit;
pub mod document;
mod grid;
pub mod ids;
pub mod import;
mod inference;
pub mod keyform;
mod mesh_edit;
pub mod project;
pub mod space;
pub mod texture;

pub use automesh::AutoMeshOptions;
pub use command::{ChildRef, Command, InferMethod, KeyformCell, KeyformTarget, VertexMove};
pub use document::{ApplyReport, Document, EditError};
pub use grid::CellError;
pub use import::{PsdImport, PsdImportError, PsdImportOptions, PsdImportWarning, import_psd};
pub use inference::RederiveWarning;
pub use keyform::Keyform;
pub use project::{Project, ProjectError, ProjectMeta, export_bundle};
pub use texture::Textures;

#[cfg(test)]
mod deformer_edit_tests;
#[cfg(test)]
mod import_tests;
#[cfg(test)]
mod inference_tests;
#[cfg(test)]
mod keyform_tests;
#[cfg(test)]
mod mesh_edit_tests;
#[cfg(test)]
mod project_tests;
#[cfg(test)]
mod tests;
