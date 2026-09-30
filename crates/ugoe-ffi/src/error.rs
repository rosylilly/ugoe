//! エラーコードとスレッドローカルのエラーメッセージ、パニックを境界で止める仕組み
//! (`docs/portability.md#c-abi-ugoe-ffi-の規約`)。

use std::cell::RefCell;
use std::ffi::{CString, c_char};
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
/// ホストが渡したバッファが小さすぎた。何も書き込んでいない。
pub const UGOE_ERROR_BUFFER_TOO_SMALL: UgoeResult = 4;
/// ファイルを読めなかった (壊れたバイト列、対応していない版、知らない必須の機能など)。
pub const UGOE_ERROR_FORMAT: UgoeResult = 5;
/// モデルの検証やコンパイルに失敗した。
pub const UGOE_ERROR_INVALID_MODEL: UgoeResult = 6;
/// 指定した ID が見つからなかった。
pub const UGOE_ERROR_NOT_FOUND: UgoeResult = 7;
/// ugoe の内部でパニックが起きた。対象のオブジェクトの状態は不定なので、破棄して作り直す。
pub const UGOE_ERROR_PANIC: UgoeResult = 8;

/// 境界の内側で使う失敗。コードとメッセージを持つ。
#[derive(Debug)]
pub(crate) struct Error {
    code: UgoeResult,
    message: String,
}

impl Error {
    pub(crate) fn new(code: UgoeResult, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
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

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(message: String) {
    // メッセージに NUL が含まれていたら、そこまでで切る。
    let bytes: Vec<u8> = message
        .into_bytes()
        .into_iter()
        .take_while(|&b| b != 0)
        .collect();
    let message = CString::new(bytes).unwrap_or_default();
    LAST_ERROR.with(|last| *last.borrow_mut() = Some(message));
}

/// `f` をパニックを捕まえて実行し、結果をエラーコードにする。失敗したらメッセージを記録する。
pub(crate) fn guard(f: impl FnOnce() -> Result<(), Error>) -> UgoeResult {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => UGOE_OK,
        Ok(Err(error)) => {
            set_last_error(error.message);
            error.code
        }
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic payload".to_owned());
            set_last_error(format!("panic inside ugoe: {message}"));
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
            .as_ref()
            .map_or(std::ptr::null(), |message| message.as_ptr())
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;

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
    }
}
