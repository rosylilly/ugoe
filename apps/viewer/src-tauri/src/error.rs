//! コマンドのエラー。ADR 0029 の形 (`code`・`args`・`message`) で画面に渡し、画面はカタログの `errors.<code>` を
//! `args` で引いて訳す (`src/lib/errors.ts`。カタログに無いコードは `message` を出す)。コードの一覧は README の
//! 「コマンドのエラー」。コードは公開の約束として扱い、名前を変えない。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

/// コマンドのエラー (`code`・`args`・`message`)。`message` は英語の文 (ログと、訳の無い画面の向け)。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct CommandError {
    pub(crate) code: &'static str,
    pub(crate) args: BTreeMap<&'static str, Value>,
    pub(crate) message: String,
}

impl CommandError {
    fn new(code: &'static str, message: String) -> Self {
        Self {
            code,
            args: BTreeMap::new(),
            message,
        }
    }

    fn arg(mut self, name: &'static str, value: impl Into<Value>) -> Self {
        self.args.insert(name, value.into());
        self
    }

    fn path(self, path: &Path) -> Self {
        self.arg("path", path.display().to_string())
    }

    /// ビューアが読めるファイル (`.ugoe`・`.json`・`.ugoemod`) ではない。
    pub(crate) fn not_model(path: &Path) -> Self {
        Self::new(
            "file.not_model",
            format!(
                "not a model or module file (.ugoe, .json or .ugoemod): {}",
                path.display()
            ),
        )
        .path(path)
    }

    /// ファイルが読み込みの上限 (`max` バイト) より大きい。`size` は分かった大きさ (バイト)。
    pub(crate) fn too_large(path: &Path, size: u64, max: u64) -> Self {
        Self::new(
            "file.too_large",
            format!(
                "{}: file is too large ({size} bytes; the limit is {max} bytes)",
                path.display()
            ),
        )
        .path(path)
        .arg("size", size)
        .arg("max", max)
    }

    /// 読めなかった。ファイルが無ければ `file.not_found`、ほかは `file.read_failed` (`reason` は OS の英語の文)。
    pub(crate) fn read_failed(path: &Path, error: &std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::NotFound {
            return Self::new(
                "file.not_found",
                format!("{}: the file does not exist", path.display()),
            )
            .path(path);
        }
        Self::new("file.read_failed", format!("{}: {error}", path.display()))
            .path(path)
            .arg("reason", error.to_string())
    }

    /// 書けなかった (`reason` は OS の英語の文)。
    pub(crate) fn write_failed(path: &Path, error: &std::io::Error) -> Self {
        Self::new("file.write_failed", format!("{}: {error}", path.display()))
            .path(path)
            .arg("reason", error.to_string())
    }

    /// JSON を書くパスの拡張子が `.json` ではない。
    pub(crate) fn not_json(path: &Path) -> Self {
        Self::new(
            "file.not_json",
            format!("not a JSON file: {}", path.display()),
        )
        .path(path)
    }

    /// PNG を書くパスの拡張子が `.png` ではない。
    pub(crate) fn not_png(path: &Path) -> Self {
        Self::new(
            "file.not_png",
            format!("not a PNG file: {}", path.display()),
        )
        .path(path)
    }

    /// 読もうとしたファイルを、利用者が選んでいない (ダイアログ・ドロップ・OS から渡されたもののどれでもない。T0727)。
    pub(crate) fn not_chosen(path: &Path) -> Self {
        Self::new(
            "file.not_chosen",
            format!("{}: the file was not chosen by the user", path.display()),
        )
        .path(path)
    }

    /// 書こうとしたパスを、保存のダイアログで選んでいない (選んだパスにも 1 回しか書けない。T0727)。
    pub(crate) fn not_chosen_for_save(path: &Path) -> Self {
        Self::new(
            "file.not_chosen_for_save",
            format!(
                "{}: the path was not chosen in the save dialog",
                path.display()
            ),
        )
        .path(path)
    }

    /// 保存のダイアログで、書けない拡張子を求められた (書けるのは `json` と `png`)。
    pub(crate) fn unsupported_save_type(extension: &str) -> Self {
        Self::new(
            "file.unsupported_save_type",
            format!("cannot save .{extension} files"),
        )
        .arg("extension", extension)
    }

    /// 要求の本文が PNG のバイト列ではない。
    pub(crate) fn not_png_data() -> Self {
        Self::new("request.not_png_data", "the data is not a PNG image".into())
    }

    /// 保存先のパスを渡すヘッダー (`header`) が無い。
    pub(crate) fn missing_path(header: &str) -> Self {
        Self::new(
            "request.missing_path",
            format!("missing the {header} header"),
        )
        .arg("header", header)
    }

    /// 保存先のパスを渡すヘッダーが読めない (`reason` は英語の文)。
    pub(crate) fn invalid_path(reason: String) -> Self {
        Self::new("request.invalid_path", format!("invalid path: {reason}")).arg("reason", reason)
    }

    /// ダイアログが選ばれたものをパスにできなかった (`reason` は英語の文)。
    pub(crate) fn dialog_failed(reason: String) -> Self {
        Self::new("dialog.failed", format!("the dialog failed: {reason}")).arg("reason", reason)
    }

    /// 別のスレッドの処理が終わらなかった (パニックした・取り消された)。スタジオ (T0819) と同じコード。
    pub(crate) fn internal(reason: String) -> Self {
        Self::new("internal", format!("the file task failed: {reason}")).arg("reason", reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_as_code_args_and_message() {
        let error = CommandError::too_large(Path::new("/m/a.ugoe"), 9, 8);
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            json!({
                "code": "file.too_large",
                "args": { "path": "/m/a.ugoe", "size": 9, "max": 8 },
                "message": "/m/a.ugoe: file is too large (9 bytes; the limit is 8 bytes)",
            })
        );
        // 引数の無いエラーも `args` を (空で) 持つ。
        assert_eq!(
            serde_json::to_value(CommandError::not_png_data()).unwrap()["args"],
            json!({})
        );
    }

    #[test]
    fn tells_missing_files_from_other_read_errors() {
        let path = Path::new("/m/a.ugoe");
        let missing = std::io::Error::from(std::io::ErrorKind::NotFound);
        let error = CommandError::read_failed(path, &missing);
        assert_eq!(error.code, "file.not_found");
        assert_eq!(error.args.keys().collect::<Vec<_>>(), [&"path"]);
        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let error = CommandError::read_failed(path, &denied);
        assert_eq!(error.code, "file.read_failed");
        assert_eq!(error.args["reason"], json!(denied.to_string()));
    }
}
