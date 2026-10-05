//! ファイルの読み書きの補助。アプリ (エディタ・ビューア・スタジオ)・MCP サーバ (`ugoe-mcp`)・エディタの中核
//! (`ugoe-editor-core`) が同じ実装を使い、片方だけが直る (Windows の置き換え・権限・失敗時の後始末) のを避ける (T0025)。
//!
//! - [`write_atomic`]・[`write_atomic_private`]・[`StagedFile`]: 同じディレクトリの一時ファイルに書いてから名前を変えて
//!   置き換える。途中で失敗しても前のファイルは残る (`atomic` モジュール)。
//! - [`read_regular_file`]: 通常のファイルだけを、大きさの上限までで読む (`read` モジュール)。
//!
//! ツール層の葉のクレートで、ほかの ugoe のクレートにも外部のクレートにも依存しない (std だけ)。ランタイム層は
//! ファイル I/O を持たないので使わない。置き場所を決めた理由は `README.md`。

mod atomic;
mod read;

pub use atomic::{Access, StagedFile, write_atomic, write_atomic_private};
pub use read::{ReadError, read_regular_file};
