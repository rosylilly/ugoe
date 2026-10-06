//! エージェントに見せるパスから、Windows の verbatim の形 (`\\?\`) を外す (T0967)。
//!
//! Windows の `canonicalize` は `\\?\C:\…`・`\\?\UNC\server\share\…` の形を返す。サーバは root の判定と
//! 開いているプロジェクトの鍵にこの形を使うが、ツールの結果・エラーの文にそのまま出すと読みにくく、
//! エージェントがそのパスに `/` 区切りの続きをつないで次の呼び出しに使うと失敗する (verbatim の形は `/` を
//! 区切りとして受け付けない)。見せるときだけ通常の形 (`C:\…`・`\\server\share\…`) に直す。比べるのは
//! 正規化した形のまま。
//!
//! 直すのはドライブ (`\\?\C:\`) と UNC (`\\?\UNC\`) だけ。ほかの verbatim の形 (`\\?\Volume{…}\` など) は
//! 通常の形で書けないのでそのまま残す。Windows でなければ正規化したパスにこの形は現れないので、どの OS でも
//! 同じに直す (文字列だけを見る)。

use std::borrow::Cow;

use serde_json::Value;

const VERBATIM: &str = r"\\?\";

/// `path` の先頭の verbatim の部分を、通常の形の先頭に置き換えたもの (置き換える先頭と残り)。
fn split_verbatim(path: &str) -> Option<(&'static str, &str)> {
    let rest = path.strip_prefix(VERBATIM)?;
    if let Some(unc) = rest.strip_prefix(r"UNC\") {
        return Some((r"\\", unc));
    }
    // `\\?\C:` (`\` が続かない) はドライブの根ではなくボリュームを指すので直さない。
    match rest.as_bytes() {
        [drive, b':', b'\\', ..] if drive.is_ascii_alphabetic() => Some(("", rest)),
        _ => None,
    }
}

/// パス 1 つを通常の形にする (`\\?\C:\x` は `C:\x`、`\\?\UNC\server\share\x` は `\\server\share\x`)。
/// verbatim の形でなければそのまま返す。
pub fn plain_path(path: &str) -> Cow<'_, str> {
    match split_verbatim(path) {
        Some((head, rest)) => Cow::Owned(format!("{head}{rest}")),
        None => Cow::Borrowed(path),
    }
}

/// 文の中のすべての verbatim のパスを通常の形にする (エラーの文はパスを文の途中に書く)。
pub fn plain_paths_in(text: &str) -> Cow<'_, str> {
    if !text.contains(VERBATIM) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(VERBATIM) {
        out.push_str(&rest[..at]);
        match split_verbatim(&rest[at..]) {
            Some((head, after)) => {
                out.push_str(head);
                rest = after;
            }
            None => {
                out.push_str(VERBATIM);
                rest = &rest[at + VERBATIM.len()..];
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

/// JSON の中のすべての文字列 (ツールの結果・エラーの `Message`) に [`plain_paths_in`] をかける。
pub(crate) fn plain_paths_in_value(value: &mut Value) {
    match value {
        Value::String(text) => {
            if let Cow::Owned(plain) = plain_paths_in(text) {
                *text = plain;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(plain_paths_in_value),
        Value::Object(map) => map.values_mut().for_each(plain_paths_in_value),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn verbatim_paths_become_plain() {
        for (verbatim, plain) in [
            (r"\\?\C:\Users\a\x.ugoeproj", r"C:\Users\a\x.ugoeproj"),
            (r"\\?\d:\", r"d:\"),
            (r"\\?\UNC\server\share\x", r"\\server\share\x"),
        ] {
            assert_eq!(plain_path(verbatim), plain);
        }
        // 通常の形と、通常の形で書けない verbatim の形はそのまま。
        for path in [
            r"C:\Users\a",
            "/tmp/x",
            r"\\server\share\x",
            r"\\?\C:",
            r"\\?\Volume{0b1c}\x",
            r"\\?\GLOBALROOT\Device\x",
            "",
        ] {
            assert!(
                matches!(plain_path(path), Cow::Borrowed(p) if p == path),
                "{path}"
            );
        }
    }

    #[test]
    fn verbatim_paths_in_text_become_plain() {
        assert_eq!(
            plain_paths_in(
                r"base model \\?\C:\t\outside\a.ugoe: outside the root directory `\\?\C:\t\root` (x)"
            ),
            r"base model C:\t\outside\a.ugoe: outside the root directory `C:\t\root` (x)"
        );
        assert_eq!(
            plain_paths_in(r"`\\?\UNC\s\share\a` and `\\?\Volume{1}\b` and \\?\"),
            r"`\\s\share\a` and `\\?\Volume{1}\b` and \\?\"
        );
        let text = "no paths here";
        assert!(matches!(plain_paths_in(text), Cow::Borrowed(t) if t == text));
    }

    #[test]
    fn verbatim_paths_in_json_become_plain() {
        let mut value = json!({
            "path": r"\\?\C:\a\b.ugoeproj",
            "message": r"`\\?\C:\a` is not open",
            "items": [{ "args": { "path": r"\\?\UNC\s\share\c" } }],
            "count": 3,
        });
        plain_paths_in_value(&mut value);
        assert_eq!(
            value,
            json!({
                "path": r"C:\a\b.ugoeproj",
                "message": r"`C:\a` is not open",
                "items": [{ "args": { "path": r"\\s\share\c" } }],
                "count": 3,
            })
        );
    }
}
