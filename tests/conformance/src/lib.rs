//! 配布物の適合テスト (ADR 0017 の決定 3、`tests/conformance/README.md`)。
//!
//! 同じモデル・同じパラメータから、すべての配布物 (C ABI・wasm・エンジン向けのバインディング) が
//! 参照と同じ頂点と画像を出すことを検査する。このクレートは次の 3 つを持つ。
//!
//! - [`plan`] は、ケースの一覧 (`cases.json`) を、各バインディングが読む計画 (`plan.json`・`plan.txt`) に展開する。
//! - [`frame`] と [`reference`] は、評価の結果 (頂点・不透明度・描画順など) の形と、参照 (`ugoe::Puppet` と
//!   その `ModelInstance`) からそれを作る処理。
//! - [`compare`] は、参照とバインディングの結果を、公開仕様の許容誤差で比べる。

pub mod compare;
pub mod frame;
pub mod plan;
pub mod reference;

use std::path::PathBuf;

/// 適合テストのエラー。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{context}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{context}")]
    Json {
        context: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("{context}")]
    Format {
        context: String,
        #[source]
        source: ugoe_format::FormatError,
    },
    #[error("{context}")]
    Headless {
        context: String,
        #[source]
        source: ugoe_headless::Error,
    },
    /// ケースの一覧やモデルの誤り。
    #[error("{0}")]
    Invalid(String),
}

impl Error {
    fn io(context: impl Into<String>) -> impl FnOnce(std::io::Error) -> Self {
        let context = context.into();
        move |source| Self::Io { context, source }
    }

    fn json(context: impl Into<String>) -> impl FnOnce(serde_json::Error) -> Self {
        let context = context.into();
        move |source| Self::Json { context, source }
    }
}

/// ファイルを読む。
fn read(path: &std::path::Path) -> Result<Vec<u8>, Error> {
    std::fs::read(path).map_err(Error::io(format!("reading {}", path.display())))
}

/// JSON のファイルを読む。
fn read_json<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Result<T, Error> {
    serde_json::from_slice(&read(path)?).map_err(Error::json(format!("parsing {}", path.display())))
}

/// リポジトリのルート (このクレートは `tests/conformance` にある)。
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
