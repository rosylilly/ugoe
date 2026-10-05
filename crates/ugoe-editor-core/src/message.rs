//! 画面に出すエラー・警告・問題の、コードと引数の形 (ADR 0029 の「Rust 側のメッセージ」、
//! `docs/editor.md#エラーと警告のコード`)。
//!
//! - `code` は安定した識別子 (小文字の `snake_case` を `.` でつなぐ)。**公開の約束として扱い、名前を変えない。**
//!   意味が変わるなら新しいコードを作る。画面はカタログの `errors.<code>` を `args` で引く。
//! - `args` は文に差し込む値 (文字列か数値)。パスや名前はそのまま渡し、単位や書式は画面で付ける。
//! - `message` は英語の文 (エラーの `Display` と同じ)。ログ・MCP・CLI・開発者向けの表示で使う。
//!   カタログに無いコードは、画面もこれを出す。
//! - 翻訳のカタログはこのクレートに置かない。

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 文に差し込む値。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
pub enum MessageArg {
    Text(String),
    Number(f64),
}

impl From<String> for MessageArg {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for MessageArg {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<&String> for MessageArg {
    fn from(value: &String) -> Self {
        Self::Text(value.clone())
    }
}

macro_rules! number_arg {
    ($($t:ty),*) => {
        $(impl From<$t> for MessageArg {
            fn from(value: $t) -> Self {
                Self::Number(value.into())
            }
        })*
    };
}

number_arg!(u8, u16, u32, i32);

impl From<f64> for MessageArg {
    fn from(value: f64) -> Self {
        // JSON は NaN・無限大を数で表せないので、文字列 (`NaN`・`inf`) で渡す。
        if value.is_finite() {
            Self::Number(value)
        } else {
            Self::Text(value.to_string())
        }
    }
}

impl From<f32> for MessageArg {
    fn from(value: f32) -> Self {
        f64::from(value).into()
    }
}

impl From<usize> for MessageArg {
    fn from(value: usize) -> Self {
        // 画面に出す数 (個数・番号) は f64 で正確に表せる大きさに収まる。
        Self::Number(value as f64)
    }
}

impl From<u64> for MessageArg {
    fn from(value: u64) -> Self {
        Self::Number(value as f64)
    }
}

/// 引数の名前 → 値。
pub type MessageArgs = BTreeMap<String, MessageArg>;

/// [`MessageArgs`] を作る。`message_args! { "path" => path.display().to_string(), "index" => index }`。
#[macro_export]
macro_rules! message_args {
    () => { $crate::message::MessageArgs::new() };
    ($($name:literal => $value:expr),+ $(,)?) => {
        [$(($name.to_owned(), $crate::message::MessageArg::from($value))),+]
            .into_iter()
            .collect::<$crate::message::MessageArgs>()
    };
}

/// 画面に出すエラー・警告・問題 (ADR 0029)。Tauri のコマンドのエラーもこの形で返す。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Message {
    /// 安定した識別子 (`project.not_a_project` など)。一覧は `docs/editor.md#エラーと警告のコード`。
    pub code: String,
    /// 文に差し込む値。
    pub args: MessageArgs,
    /// 英語の文 (ログ・MCP・CLI・開発者向け。カタログに無いコードのときは画面もこれを出す)。
    pub message: String,
    /// 原因になったエラー (`proposal.rejected` の中の `edit.*` など)。`message` は原因の文も含む。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub cause: Option<Box<Message>>,
    /// 一覧の中身 (検証のエラーの列 `validation.*` など)。並べるのは引数 `details` と同じく先頭の
    /// `DESCRIBED_ERRORS_MAX` 件までで、全件の数は引数 `count`。画面は各項目を訳して `details` の代わりに差し込む。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<Message>,
}

impl Message {
    /// `code` と `args` と英語の文から作る。
    pub fn new(code: &str, args: MessageArgs, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            args,
            message: message.into(),
            cause: None,
            items: Vec::new(),
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// コードと引数を持つエラー・警告。英語の文は `Display`。
pub trait Coded: fmt::Display {
    /// 安定した識別子 ([`Message::code`])。ほかのクレートのエラーは、元のコードに領域の名前を付けて組み立てる
    /// (`validation.duplicate_id` など) ので `Cow`。
    fn code(&self) -> Cow<'static, str>;

    /// 文に差し込む値 ([`Message::args`])。
    fn args(&self) -> MessageArgs {
        MessageArgs::new()
    }

    /// 原因になったエラー ([`Message::cause`])。
    fn cause(&self) -> Option<Message> {
        None
    }

    /// 一覧の中身 ([`Message::items`])。
    fn items(&self) -> Vec<Message> {
        Vec::new()
    }

    /// 画面に渡す形にする。
    fn to_message(&self) -> Message {
        Message {
            code: self.code().into_owned(),
            args: self.args(),
            message: self.to_string(),
            cause: self.cause().map(Box::new),
            items: self.items(),
        }
    }
}

impl<E: Coded> From<E> for Message {
    fn from(error: E) -> Self {
        error.to_message()
    }
}
