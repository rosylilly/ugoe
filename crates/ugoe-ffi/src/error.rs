//! エラーコードとスレッドローカルのエラーメッセージ、パニックを境界で止める仕組み
//! (`docs/portability.md#c-abi-ugoe-ffi-の規約`)。

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};
use std::panic::{self, AssertUnwindSafe};

/// 関数の結果。`UGOE_OK` (0) が成功、それ以外は失敗の種類。詳細は `ugoe_last_error` で得る。
pub type UgoeResult = i32;

/// 成功。
pub const UGOE_OK: UgoeResult = 0;
/// 必須のポインタ引数が NULL だった。
pub const UGOE_ERROR_NULL_POINTER: UgoeResult = 1;
/// 引数の値が不正だった (UTF-8 でない文字列、範囲外の列挙値など)。
pub const UGOE_ERROR_INVALID_ARGUMENT: UgoeResult = 2;
/// 添字が範囲外だった。
pub const UGOE_ERROR_OUT_OF_RANGE: UgoeResult = 3;
/// ホストが渡したバッファが小さすぎた。バッファには何も書き込んでいない (可変長の結果では、要る数か長さだけを書く)。
pub const UGOE_ERROR_BUFFER_TOO_SMALL: UgoeResult = 4;
/// ファイルを読めなかった (壊れたバイト列、対応していない版、知らない必須の機能など)。モデルの読み込み
/// (`ugoe_puppet_create_from_*`) では、種類 (`ugoe_last_error_kind`) が `"invalid_file"`・`"unsupported_version"`・
/// `"unsupported_features"`・`"invalid_model"` のどれか (T0457)。
pub const UGOE_ERROR_FORMAT: UgoeResult = 5;
/// モデルの検証やコンパイルに失敗した。モデルの読み込みでは、種類が `"invalid_model"` か `"compile"` (T0457)。
pub const UGOE_ERROR_INVALID_MODEL: UgoeResult = 6;
/// 指定した ID が見つからなかった。
pub const UGOE_ERROR_NOT_FOUND: UgoeResult = 7;
/// ugoe の内部でパニックが起きた。対象のオブジェクトの状態は不定なので、破棄して作り直す。
pub const UGOE_ERROR_PANIC: UgoeResult = 8;
/// 着せ替え (モジュールの付け外し) に失敗した。何も変えていない。種類は `ugoe_last_error_kind`、
/// 補足の値は `ugoe_last_error_field` で得る (T0409)。
pub const UGOE_ERROR_DRESS_UP: UgoeResult = 9;
/// 読み込みの上限 (`UgoeReadLimits`) を超えたので、読み込みをやめた。ファイルが壊れているとは限らない
/// (上限を上げれば読めることがある)。種類 (`ugoe_last_error_kind`) は `"read_limit_exceeded"`、補足の値は
/// `ugoe_last_error_field` の `"limit"`・`"chunk"`・`"max"` (T0152)。
pub const UGOE_ERROR_LIMIT_EXCEEDED: UgoeResult = 10;

/// 境界の内側で使う失敗。コードとメッセージを持つ。
#[derive(Debug)]
pub(crate) struct Error {
    code: UgoeResult,
    message: String,
    /// 機械が読む種類 (`ugoe_last_error_kind`)。wasm の `DressUpError` の `code` と同じ値。
    kind: Option<&'static str>,
    /// 補足の値 (`ugoe_last_error_field`。`module`・`slot` など)。
    fields: Vec<(&'static str, String)>,
}

impl Error {
    pub(crate) fn new(code: UgoeResult, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            kind: None,
            fields: Vec::new(),
        }
    }

    /// 種類と補足の値を付ける。
    pub(crate) fn with_kind(
        mut self,
        kind: &'static str,
        fields: Vec<(&'static str, String)>,
    ) -> Self {
        self.kind = Some(kind);
        self.fields = fields;
        self
    }

    pub(crate) fn null(name: &str) -> Self {
        Self::new(UGOE_ERROR_NULL_POINTER, format!("`{name}` is null"))
    }

    pub(crate) fn out_of_range(what: &str, index: usize, len: usize) -> Self {
        Self::new(
            UGOE_ERROR_OUT_OF_RANGE,
            format!("{what} index {index} is out of range (count {len})"),
        )
    }
}

/// 最後の失敗の中身。
#[derive(Default)]
struct LastError {
    message: Option<CString>,
    kind: Option<CString>,
    fields: Vec<(CString, CString)>,
}

thread_local! {
    static LAST_ERROR: RefCell<LastError> = RefCell::new(LastError::default());
}

/// 文字列を C の文字列にする。NUL が含まれていたら、そこまでで切る。
pub(crate) fn c_string(s: &str) -> CString {
    let end = s.find('\0').unwrap_or(s.len());
    CString::new(&s[..end]).unwrap_or_default()
}

fn set_last_error(error: Error) {
    let last = LastError {
        message: Some(c_string(&error.message)),
        kind: error.kind.map(c_string),
        fields: error
            .fields
            .iter()
            .map(|(k, v)| (c_string(k), c_string(v)))
            .collect(),
    };
    LAST_ERROR.with(|cell| *cell.borrow_mut() = last);
}

/// `f` をパニックを捕まえて実行し、結果をエラーコードにする。失敗したらメッセージを記録する。
pub(crate) fn guard(f: impl FnOnce() -> Result<(), Error>) -> UgoeResult {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => UGOE_OK,
        Ok(Err(error)) => {
            let code = error.code;
            set_last_error(error);
            code
        }
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic payload".to_owned());
            set_last_error(Error::new(
                UGOE_ERROR_PANIC,
                format!("panic inside ugoe: {message}"),
            ));
            UGOE_ERROR_PANIC
        }
    }
}

/// このスレッドで最後に失敗した関数のエラーメッセージ (UTF-8、NUL 終端)。失敗がまだなければ NULL。
///
/// ポインタは、同じスレッドで次に関数が失敗するまで有効。成功した呼び出しではメッセージを消さない。
#[unsafe(no_mangle)]
pub extern "C" fn ugoe_last_error() -> *const c_char {
    LAST_ERROR.with(|last| {
        last.borrow()
            .message
            .as_ref()
            .map_or(std::ptr::null(), |message| message.as_ptr())
    })
}

/// このスレッドで最後に失敗した関数の、機械が読む種類 (UTF-8、NUL 終端)。種類を持たない失敗なら NULL。
///
/// 着せ替えの失敗 (`UGOE_ERROR_DRESS_UP` と、`.ugoemod` を読めなかったときの `UGOE_ERROR_FORMAT`) で、
/// wasm の `DressUpError` の `code` と同じ値 (`"unknown_slot"`・`"slot_occupied"`・`"invalid_file"` など) になる。
/// モデルの読み込み (`ugoe_puppet_create_from_*`) の失敗 (`UGOE_ERROR_FORMAT`・`UGOE_ERROR_INVALID_MODEL`) では、
/// wasm の `LoadError` の `code` と同じ値 (`"invalid_file"`・`"unsupported_version"`・`"unsupported_features"`・
/// `"invalid_model"`・`"compile"`。T0457)。読み込みの上限を超えたとき (`UGOE_ERROR_LIMIT_EXCEEDED`) は `"read_limit_exceeded"`。
/// 一覧は crates/ugoe-ffi/README.md。ポインタの寿命は `ugoe_last_error` と同じ。
#[unsafe(no_mangle)]
pub extern "C" fn ugoe_last_error_kind() -> *const c_char {
    LAST_ERROR.with(|last| {
        last.borrow()
            .kind
            .as_ref()
            .map_or(std::ptr::null(), |kind| kind.as_ptr())
    })
}

/// このスレッドで最後に失敗した関数の補足の値 `key` (`"module"`・`"slot"`・`"parameter"`・`"mesh"`・
/// `"attached"`・`"mask"`・`"rig"`・`"collider"`・`"part"`・`"texture"`・`"required"`・`"actual"`・`"errors"`、
/// 読み込みの上限では `"limit"`・`"chunk"`・`"max"`、モデルの読み込みでは `"found"`・`"supported"`・`"features"`・`"errors"`)。
/// 番号や数は 10 進の文字列。無ければ NULL。`key` が NULL か UTF-8 でなくても NULL。ポインタの寿命は `ugoe_last_error` と同じ。
///
/// # Safety
///
/// `key` は NULL か、NUL 終端の文字列を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_last_error_field(key: *const c_char) -> *const c_char {
    if key.is_null() {
        return std::ptr::null();
    }
    // SAFETY: 呼び出し側の約束で、`key` は NUL 終端の文字列。
    let key = unsafe { CStr::from_ptr(key) };
    LAST_ERROR.with(|last| {
        last.borrow()
            .fields
            .iter()
            .find(|(k, _)| k.as_c_str() == key)
            .map_or(std::ptr::null(), |(_, v)| v.as_ptr())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn last_error() -> String {
        let ptr = ugoe_last_error();
        assert!(!ptr.is_null());
        // SAFETY: ugoe_last_error は NUL 終端の文字列を返し、次の失敗まで有効。
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn panic_becomes_error_code() {
        let code = guard(|| panic!("boom"));
        assert_eq!(code, UGOE_ERROR_PANIC);
        assert!(last_error().contains("boom"));

        let code = guard(|| panic!("formatted {}", 42));
        assert_eq!(code, UGOE_ERROR_PANIC);
        assert!(last_error().contains("formatted 42"));
    }

    #[test]
    fn error_sets_message_and_success_keeps_it() {
        let code = guard(|| Err(Error::new(UGOE_ERROR_NOT_FOUND, "no such thing\0ignored")));
        assert_eq!(code, UGOE_ERROR_NOT_FOUND);
        assert_eq!(last_error(), "no such thing");
        assert_eq!(guard(|| Ok(())), UGOE_OK);
        assert_eq!(last_error(), "no such thing");
        assert!(ugoe_last_error_kind().is_null());
    }

    #[test]
    fn kind_and_fields_follow_the_last_failure() {
        let code = guard(|| {
            Err(Error::new(UGOE_ERROR_DRESS_UP, "taken")
                .with_kind("slot_occupied", vec![("slot", "Hair".into())]))
        });
        assert_eq!(code, UGOE_ERROR_DRESS_UP);
        // SAFETY: NULL でないポインタは、次の失敗まで有効な NUL 終端の文字列。`key` は C の文字列リテラルか NULL。
        unsafe {
            let text = |p: *const c_char| CStr::from_ptr(p).to_string_lossy().into_owned();
            assert_eq!(text(ugoe_last_error_kind()), "slot_occupied");
            assert_eq!(text(ugoe_last_error_field(c"slot".as_ptr())), "Hair");
            assert!(ugoe_last_error_field(c"module".as_ptr()).is_null());
            assert!(ugoe_last_error_field(std::ptr::null()).is_null());

            // 種類を持たない失敗で消える。
            guard(|| Err(Error::null("x")));
            assert!(ugoe_last_error_kind().is_null());
            assert!(ugoe_last_error_field(c"slot".as_ptr()).is_null());
        }
    }
}
