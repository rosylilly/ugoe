//! MCP (Model Context Protocol) サーバ。外部のエージェント (Claude Code など) が `.ugoeproj` を読み、
//! `ugoe_editor_core::Command` の列を提案・適用できるようにする。
//!
//! 設計: `docs/ai-integration.md#mcp-サーバ`。使い方: `crates/ugoe-mcp/README.md`。
//!
//! - トランスポートは stdio (1 行 1 メッセージの JSON-RPC 2.0。行の読み書きは `main.rs`) と
//!   HTTP (MCP の Streamable HTTP。[`http`])。メッセージの処理はどちらも [`Server`]。
//! - ツールは文書を持ち主 ([`host::DocumentHost`]) から借りる。`ugoe-mcp` は `.ugoeproj` を自分で開き
//!   ([`FileHost`])、エディタは開いている文書を貸して [`EditorBridge`] で待ち受ける (ADR 0023)。
//!   `ugoe-mcp --editor` は stdio を実行中のエディタへ中継する ([`relay`]。発見は [`discovery`]、
//!   WSL と Windows のあいだは [`wsl`])。Claude Desktop には、中継を MCP Bundle に包んで入れる ([`mcpb`])。
//! - 編集はすべて `Document::apply_with_report` を通す (エディタと同じ検証と Undo)。
//! - PSD からの自動リギングでは、画像を見て分類し値を決める判断を、クライアント側のエージェントに
//!   任せる (`autorig`。ADR 0022)。サーバは API を呼ばない。

mod autorig;
pub mod bridge;
mod diff;
pub mod discovery;
pub mod host;
pub mod http;
pub mod mcpb;
pub mod relay;
pub mod render;
pub mod root;
pub mod schema;
pub mod server;
mod summary;
pub mod tools;
pub mod verbatim;
pub mod wsl;

pub use bridge::{BridgeOptions, EditorBridge};
pub use host::{DocumentHost, FileHost};
pub use root::Root;
pub use server::Server;
pub use tools::Options;
