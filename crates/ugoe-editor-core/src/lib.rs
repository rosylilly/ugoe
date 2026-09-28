//! ヘッドレスなエディタ中核。設計: `docs/editor.md`。
//!
//! **すべての編集操作は [`Command`] を通す。** GUI (Tauri)・CLI・AI エージェント (MCP) は
//! 同じ `Command` を発行するだけで、それぞれ独自の編集ロジックを持たない。
//! これにより Undo/Redo、操作ログ、AI による提案の差分プレビューが一元化される。

pub mod command;
pub mod document;
mod grid;
pub mod ids;
pub mod project;
pub mod texture;

pub use command::{Command, KeyformCell};
pub use document::{Document, EditError};
pub use grid::CellError;
pub use project::{Project, ProjectError, ProjectMeta, export_bundle};
pub use texture::Textures;

#[cfg(test)]
mod project_tests;
#[cfg(test)]
mod tests;
