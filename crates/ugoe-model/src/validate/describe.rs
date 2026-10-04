//! 検証エラーの列を利用者に見せる文にする (`docs/spec/model.md#検証エラーの文`)。

use std::fmt::Write as _;

use super::ValidationError;

/// [`describe_errors`] が文に並べる検証エラーの数の上限。超えた分は件数だけを書く。
pub const DESCRIBED_ERRORS_MAX: usize = 10;

/// 検証エラーの列を、利用者に見せる 1 行の文にする。
///
/// 形は `{subject} failed validation with {件数} error(s): {1 件目}; {2 件目}; …` で、各エラーは `Display` の文。
/// [`DESCRIBED_ERRORS_MAX`] 件を超えた分は `; and {残り} more` にまとめる。列が空なら `: ` 以降を書かない。
/// SDK・C ABI・wasm・CLI・MCP が同じ形で見せるための共通の書き方。文は人が読むためのもので、
/// 安定した値として扱わない (機械はエラーの列そのものを読む)。
pub fn describe_errors(subject: &str, errors: &[ValidationError]) -> String {
    let mut text = format!("{subject} failed validation with {} error(s)", errors.len());
    for (i, error) in errors.iter().take(DESCRIBED_ERRORS_MAX).enumerate() {
        let separator = if i == 0 { ": " } else { "; " };
        // `String` への書き込みは失敗しない。
        let _ = write!(text, "{separator}{error}");
    }
    if let Some(rest) = errors.len().checked_sub(DESCRIBED_ERRORS_MAX)
        && rest > 0
    {
        let _ = write!(text, "; and {rest} more");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(i: usize) -> ValidationError {
        ValidationError::DuplicateId {
            kind: "part",
            id: format!("p{i}"),
        }
    }

    #[test]
    fn lists_every_error_up_to_the_limit() {
        assert_eq!(
            describe_errors("model", &[]),
            "model failed validation with 0 error(s)"
        );
        assert_eq!(
            describe_errors("model", &[error(0)]),
            "model failed validation with 1 error(s): duplicate part id `p0`"
        );
        let errors: Vec<_> = (0..DESCRIBED_ERRORS_MAX).map(error).collect();
        let text = describe_errors("physics settings", &errors);
        assert!(text.starts_with(
            "physics settings failed validation with 10 error(s): duplicate part id `p0`; duplicate part id `p1`; "
        ));
        assert!(text.ends_with("; duplicate part id `p9`"), "{text}");
    }

    #[test]
    fn summarizes_errors_beyond_the_limit() {
        let errors: Vec<_> = (0..DESCRIBED_ERRORS_MAX + 3).map(error).collect();
        let text = describe_errors("model", &errors);
        assert!(text.starts_with("model failed validation with 13 error(s): "));
        assert!(
            text.ends_with("; duplicate part id `p9`; and 3 more"),
            "{text}"
        );
        assert!(!text.contains("`p10`"), "{text}");
    }
}
