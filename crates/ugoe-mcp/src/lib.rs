//! MCP (Model Context Protocol) サーバ。外部のエージェント (Claude Code など) が `.ugoeproj` を読み、
//! `ugoe_editor_core::Command` の列を提案・適用できるようにする。
//!
//! 設計: `docs/ai-integration.md#mcp-サーバ`。使い方: `crates/ugoe-mcp/README.md`。
//!
//! - トランスポートは stdio (1 行 1 メッセージの JSON-RPC 2.0。行の読み書きは `main.rs`) と
//!   HTTP (MCP の Streamable HTTP。[`http`])。メッセージの処理はどちらも [`Server`]。
//! - 編集はすべて `Document::apply_with_report` を通す (エディタと同じ検証と Undo)。
//! - PSD からの自動リギングでは、画像を見て分類し値を決める判断を、クライアント側のエージェントに
//!   任せる (`autorig`。ADR 0022)。サーバは API を呼ばない。

mod autorig;
mod diff;
pub mod http;
pub mod render;
pub mod root;
pub mod schema;
pub mod server;
mod summary;
pub mod tools;

pub use root::Root;
pub use server::Server;
pub use tools::Options;
