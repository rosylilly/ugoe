//! MCP (Model Context Protocol) サーバ。外部のエージェント (Claude Code など) が `.ugoeproj` を読み、
//! `ugoe_editor_core::Command` の列を提案・適用できるようにする。
//!
//! 設計: `docs/ai-integration.md#mcp-サーバ`。使い方: `crates/ugoe-mcp/README.md`。
//!
//! - トランスポートは stdio (1 行 1 メッセージの JSON-RPC 2.0)。行の読み書きは `main.rs`、
//!   メッセージの処理は [`Server`]。
//! - 編集はすべて `Document::apply_with_report` を通す (エディタと同じ検証と Undo)。

mod diff;
pub mod render;
pub mod schema;
pub mod server;
mod summary;
pub mod tools;

pub use server::Server;
pub use tools::Options;
