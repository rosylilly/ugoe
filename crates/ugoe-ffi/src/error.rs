//! エラーコードと、呼び出し側が渡す失敗の中身 (`UgoeError`)、パニックを境界で止める仕組み
//! (`docs/portability.md#c-abi-ugoe-ffi-の規約`。ADR 0031)。

use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{CStr, c_char};

/// 関数の結果。`UGOE_OK` (0) が成功、それ以外は失敗の種類。詳細は、呼び出しに渡した `UgoeError` から得る。
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
/// (`ugoe_puppet_create_from_*`) では、種類 (`ugoe_error_kind`) が `"invalid_file"`・`"unsupported_version"`・
/// `"unsupported_features"`・`"invalid_model"` のどれか (T0457)。
pub const UGOE_ERROR_FORMAT: UgoeResult = 5;
/// モデルの検証やコンパイルに失敗した。モデルの読み込みでは、種類が `"invalid_model"` か `"compile"` (T0457)。
/// 物理の設定の差し替え (`ugoe_puppet_set_physics_json`・`ugoe_puppet_set_base_physics_json`) の検査の失敗では `"invalid_physics"`、
/// エフェクトの設定の差し替え (`ugoe_puppet_set_effects_json`) では `"invalid_model"` (T0461・T0469)。
pub const UGOE_ERROR_INVALID_MODEL: UgoeResult = 6;
/// 指定した ID が見つからなかった。
pub const UGOE_ERROR_NOT_FOUND: UgoeResult = 7;
/// ugoe の内部でパニックが起きた。対象のオブジェクトの状態は不定なので、破棄して作り直す。
/// `std` の無い形 (crates/ugoe-ffi/README.md#std-の無い環境) では返らない (パニックはホストの `abort` を呼ぶ)。
pub const UGOE_ERROR_PANIC: UgoeResult = 8;
/// 着せ替え (モジュールの付け外し) に失敗した。何も変えていない。種類は `ugoe_error_kind`、
/// 補足の値は `ugoe_error_field` で得る (T0409)。
pub const UGOE_ERROR_DRESS_UP: UgoeResult = 9;
/// 読み込みの上限 (`UgoeReadLimits`) を超えたので、読み込みをやめた。ファイルが壊れているとは限らない
/// (上限を上げれば読めることがある)。種類 (`ugoe_error_kind`) は `"read_limit_exceeded"`、補足の値は
/// `ugoe_error_field` の `"limit"`・`"chunk"`・`"max"` (T0152)。
pub const UGOE_ERROR_LIMIT_EXCEEDED: UgoeResult = 10;

/// 境界の内側で使う失敗。コードとメッセージを持つ。
#[derive(Debug)]
pub(crate) struct Error {
    code: UgoeResult,
    message: String,
    /// 機械が読む種類 (`ugoe_error_kind`)。wasm の `DressUpError` の `code` と同じ値。
    kind: Option<&'static str>,
    /// 補足の値 (`ugoe_error_field`。`module`・`slot` など)。
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

/// `items` の `index` 番目。範囲外なら `UGOE_ERROR_OUT_OF_RANGE` の失敗にする。確かめた添字でもパニックの代わりに
/// 失敗にするために使う (`docs/portability.md#パニックしない`)。
pub(crate) fn at<'a, T>(items: &'a [T], what: &str, index: usize) -> Result<&'a T, Error> {
    items
        .get(index)
        .ok_or_else(|| Error::out_of_range(what, index, items.len()))
}

/// 失敗の中身 (メッセージ・種類・補足の値) を受け取る入れ物。不透明な型で、中身には触れない (ADR 0031)。
///
/// `ugoe_error_create` で作り、`ugoe_error_destroy` で破棄する。失敗しうる関数は最後の引数 `error` にこれを取り、
/// 失敗したら中身を書き換える (NULL を渡せば書かない。結果コードだけで足りるとき)。成功した呼び出しは中身を変えない
/// (前の失敗の中身が残る。どの呼び出しが失敗したかは結果コードで判断する)。
///
/// スレッドに結び付かないので、どのスレッドで作り、書かせ、読んでもよい。ただし 1 つの `UgoeError` を同時に複数のスレッドから
/// 使ってはならない (呼び出しごとか、スレッドごとに 1 つ持つ)。
#[derive(Debug, Default)]
pub struct UgoeError {
    /// メッセージ。まだ失敗を書いていなければ `None`。
    message: Option<CString>,
    kind: Option<CString>,
    fields: Vec<(CString, CString)>,
}

impl UgoeError {
    fn set(&mut self, error: &Error) {
        self.message = Some(c_string(&error.message));
        self.kind = error.kind.map(c_string);
        self.fields = error
            .fields
            .iter()
            .map(|(k, v)| (c_string(k), c_string(v)))
            .collect();
    }
}

/// 文字列を C の文字列にする。NUL が含まれていたら、そこまでで切る。
pub(crate) fn c_string(s: &str) -> CString {
    let head = s.split('\0').next().unwrap_or_default();
    CString::new(head).unwrap_or_default()
}

/// 失敗を `out` に書く。`out` が NULL なら何もしない。
///
/// # Safety
///
/// `out` は NULL か、破棄されていない `UgoeError` を指し、ほかから同時に使われていないこと。
unsafe fn report(out: *mut UgoeError, error: &Error) {
    // SAFETY: 呼び出し側の約束。
    if let Some(out) = unsafe { out.as_mut() } {
        out.set(error);
    }
}

/// `f` を実行し、結果をエラーコードにする。失敗したら中身を `error` に書く (NULL なら書かない)。
/// `std` がある形ではパニックも捕まえて [`UGOE_ERROR_PANIC`] にする (無い形ではパニックはホストの `abort` を呼ぶ。nostd.rs)。
///
/// 公開する関数の `error` 引数をそのまま渡す。呼び出し側の約束として、`error` は NULL か、破棄されていない `UgoeError` を指し、
/// ほかから同時に使われていないこと (ヘッダの `UgoeError` の説明)。
pub(crate) fn guard(error: *mut UgoeError, f: impl FnOnce() -> Result<(), Error>) -> UgoeResult {
    let failure = match catch_panic(f) {
        Ok(()) => return UGOE_OK,
        Err(failure) => failure,
    };
    // SAFETY: 公開する関数の呼び出し側の約束 (上の説明)。
    unsafe { report(error, &failure) };
    failure.code
}

/// `f` を実行し、パニックしたら [`UGOE_ERROR_PANIC`] の失敗にする。
#[cfg(any(feature = "std", test))]
fn catch_panic(f: impl FnOnce() -> Result<(), Error>) -> Result<(), Error> {
    use alloc::borrow::ToOwned;
    use std::panic::{self, AssertUnwindSafe};

    panic::catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic payload".to_owned());
        Err(Error::new(
            UGOE_ERROR_PANIC,
            format!("panic inside ugoe: {message}"),
        ))
    })
}

/// `std` の無い形では巻き戻せないので、そのまま実行する (パニックはホストの `abort` を呼ぶ。nostd.rs)。
#[cfg(not(any(feature = "std", test)))]
fn catch_panic(f: impl FnOnce() -> Result<(), Error>) -> Result<(), Error> {
    f()
}

/// 空の `UgoeError` を作る。`ugoe_error_destroy` で破棄する。
#[unsafe(no_mangle)]
pub extern "C" fn ugoe_error_create() -> *mut UgoeError {
    Box::into_raw(Box::default())
}

/// `UgoeError` を破棄する。NULL なら何もしない。破棄した後は、これから読んだ文字列のポインタも無効になる。
///
/// # Safety
///
/// `error` は NULL か、`ugoe_error_create` で作ってまだ破棄していない `UgoeError` であること。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_error_destroy(error: *mut UgoeError) {
    if error.is_null() {
        return;
    }
    // SAFETY: `ugoe_error_create` が Box::into_raw で作ったポインタ。
    drop(unsafe { Box::from_raw(error) });
}

/// `UgoeError` を読む。NULL なら `None`。
///
/// # Safety
///
/// `error` は NULL か、破棄されていない `UgoeError` を指すこと。
unsafe fn read<'a>(error: *const UgoeError) -> Option<&'a UgoeError> {
    // SAFETY: 呼び出し側の約束。
    unsafe { error.as_ref() }
}

fn c_ptr(s: Option<&CString>) -> *const c_char {
    s.map_or(core::ptr::null(), |s| s.as_ptr())
}

/// `error` に書いた失敗のメッセージ (UTF-8、NUL 終端)。まだ失敗を書いていないか、`error` が NULL なら NULL。
///
/// 読み出しの関数 (`ugoe_error_*`) が返すポインタは、その `UgoeError` に次の失敗を書くか、破棄するまで有効。
///
/// # Safety
///
/// `error` は NULL か、破棄されていない `UgoeError` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_error_message(error: *const UgoeError) -> *const c_char {
    // SAFETY: 呼び出し側の約束。
    c_ptr(unsafe { read(error) }.and_then(|e| e.message.as_ref()))
}

/// `error` に書いた失敗の、機械が読む種類 (UTF-8、NUL 終端)。種類を持たない失敗か、まだ失敗を書いていなければ NULL。
///
/// 着せ替えの失敗 (`UGOE_ERROR_DRESS_UP` と、`.ugoemod` を読めなかったときの `UGOE_ERROR_FORMAT`) で、
/// wasm の `DressUpError` の `code` と同じ値 (`"unknown_slot"`・`"slot_occupied"`・`"invalid_file"` など) になる。
/// モデルの読み込み (`ugoe_puppet_create_from_*`) の失敗 (`UGOE_ERROR_FORMAT`・`UGOE_ERROR_INVALID_MODEL`) では、
/// wasm の `LoadError` の `code` と同じ値 (`"invalid_file"`・`"unsupported_version"`・`"unsupported_features"`・
/// `"invalid_model"`・`"compile"`。T0457)。読み込みの上限を超えたとき (`UGOE_ERROR_LIMIT_EXCEEDED`) は `"read_limit_exceeded"`。
/// 一覧は crates/ugoe-ffi/README.md。ポインタの寿命は `ugoe_error_message` と同じ。
///
/// # Safety
///
/// `error` は NULL か、破棄されていない `UgoeError` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_error_kind(error: *const UgoeError) -> *const c_char {
    // SAFETY: 呼び出し側の約束。
    c_ptr(unsafe { read(error) }.and_then(|e| e.kind.as_ref()))
}

/// `error` に書いた失敗の補足の値 `key` (`"module"`・`"slot"`・`"parameter"`・`"mesh"`・
/// `"attached"`・`"mask"`・`"rig"`・`"collider"`・`"part"`・`"texture"`・`"required"`・`"actual"`・`"errors"`、
/// 読み込みの上限では `"limit"`・`"chunk"`・`"max"`、モデルの読み込みでは `"found"`・`"supported"`・`"features"`・`"errors"`)。
/// 番号や数は 10 進の文字列。無ければ NULL。`error` か `key` が NULL のときも NULL。ポインタの寿命は `ugoe_error_message` と同じ。
///
/// # Safety
///
/// `error` は NULL か、破棄されていない `UgoeError` を指すこと。`key` は NULL か、NUL 終端の文字列を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_error_field(
    error: *const UgoeError,
    key: *const c_char,
) -> *const c_char {
    if key.is_null() {
        return core::ptr::null();
    }
    // SAFETY: 呼び出し側の約束で、`key` は NUL 終端の文字列。
    let key = unsafe { CStr::from_ptr(key) };
    // SAFETY: 呼び出し側の約束。
    let value = unsafe { read(error) }
        .and_then(|e| e.fields.iter().find(|(k, _)| k.as_c_str() == key))
        .map(|(_, v)| v);
    c_ptr(value)
}

/// `error` に書いた失敗の補足の値の数。補足の値を持たない失敗、まだ失敗を書いていないとき、`error` が NULL のときは 0 (T0464)。
///
/// 補足の値は `ugoe_error_field_key_at`・`ugoe_error_field_value_at` で 0 から順に読める。
/// キーを知らなくても (新しい版でキーが増えても) すべての補足の値を得られる。並びは SDK の `fields` の順。
///
/// # Safety
///
/// `error` は NULL か、破棄されていない `UgoeError` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_error_field_count(error: *const UgoeError) -> usize {
    // SAFETY: 呼び出し側の約束。
    unsafe { read(error) }.map_or(0, |e| e.fields.len())
}

/// `error` に書いた失敗の、`index` 番目の補足の値のキー (UTF-8、NUL 終端。`"module"` など。T0464)。
/// `index` が `ugoe_error_field_count(error)` 以上なら NULL。ポインタの寿命は `ugoe_error_message` と同じ。
///
/// # Safety
///
/// `error` は NULL か、破棄されていない `UgoeError` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_error_field_key_at(
    error: *const UgoeError,
    index: usize,
) -> *const c_char {
    // SAFETY: 呼び出し側の約束。
    c_ptr(unsafe { read(error) }.and_then(|e| e.fields.get(index).map(|(k, _)| k)))
}

/// `error` に書いた失敗の、`index` 番目の補足の値 (UTF-8、NUL 終端。`ugoe_error_field` と同じ書き方。T0464)。
/// `index` が `ugoe_error_field_count(error)` 以上なら NULL。ポインタの寿命は `ugoe_error_message` と同じ。
///
/// # Safety
///
/// `error` は NULL か、破棄されていない `UgoeError` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_error_field_value_at(
    error: *const UgoeError,
    index: usize,
) -> *const c_char {
    // SAFETY: 呼び出し側の約束。
    c_ptr(unsafe { read(error) }.and_then(|e| e.fields.get(index).map(|(_, v)| v)))
}

/// テストで使う `UgoeError`。テストのあいだ `UgoeError` を持ち、最後に破棄する。
#[cfg(test)]
pub(crate) struct TestError(pub(crate) *mut UgoeError);

#[cfg(test)]
impl TestError {
    pub(crate) fn new() -> Self {
        Self(ugoe_error_create())
    }

    fn text(p: *const c_char) -> Option<String> {
        // SAFETY: NULL でないポインタは、次の失敗を書くまで有効な NUL 終端の文字列。
        (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
    }

    pub(crate) fn message(&self) -> Option<String> {
        // SAFETY: `self.0` は破棄していない `UgoeError`。
        Self::text(unsafe { ugoe_error_message(self.0) })
    }

    pub(crate) fn kind(&self) -> Option<String> {
        // SAFETY: 同上。
        Self::text(unsafe { ugoe_error_kind(self.0) })
    }

    pub(crate) fn field(&self, key: &CStr) -> Option<String> {
        // SAFETY: 同上。`key` は NUL 終端。
        Self::text(unsafe { ugoe_error_field(self.0, key.as_ptr()) })
    }

    pub(crate) fn field_count(&self) -> usize {
        // SAFETY: 同上。
        unsafe { ugoe_error_field_count(self.0) }
    }

    pub(crate) fn key_at(&self, i: usize) -> Option<String> {
        // SAFETY: 同上。
        Self::text(unsafe { ugoe_error_field_key_at(self.0, i) })
    }

    pub(crate) fn value_at(&self, i: usize) -> Option<String> {
        // SAFETY: 同上。
        Self::text(unsafe { ugoe_error_field_value_at(self.0, i) })
    }
}

#[cfg(test)]
impl Drop for TestError {
    fn drop(&mut self) {
        // SAFETY: `ugoe_error_create` で作ったもの。
        unsafe { ugoe_error_destroy(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_becomes_error_code() {
        let e = TestError::new();
        let code = guard(e.0, || panic!("boom"));
        assert_eq!(code, UGOE_ERROR_PANIC);
        assert!(e.message().is_some_and(|m| m.contains("boom")));

        let code = guard(e.0, || panic!("formatted {}", 42));
        assert_eq!(code, UGOE_ERROR_PANIC);
        assert!(e.message().is_some_and(|m| m.contains("formatted 42")));
    }

    #[test]
    fn error_sets_message_and_success_keeps_it() {
        let e = TestError::new();
        assert_eq!(e.message(), None);
        let code = guard(e.0, || {
            Err(Error::new(UGOE_ERROR_NOT_FOUND, "no such thing\0ignored"))
        });
        assert_eq!(code, UGOE_ERROR_NOT_FOUND);
        assert_eq!(e.message().as_deref(), Some("no such thing"));
        assert_eq!(guard(e.0, || Ok(())), UGOE_OK);
        assert_eq!(e.message().as_deref(), Some("no such thing"));
        assert_eq!(e.kind(), None);
    }

    #[test]
    fn null_error_is_not_written() {
        let code = guard(std::ptr::null_mut(), || Err(Error::null("x")));
        assert_eq!(code, UGOE_ERROR_NULL_POINTER);
        // SAFETY: NULL は読み出しの関数に渡してよい。
        unsafe {
            assert!(ugoe_error_message(std::ptr::null()).is_null());
            assert!(ugoe_error_kind(std::ptr::null()).is_null());
            assert!(ugoe_error_field(std::ptr::null(), c"slot".as_ptr()).is_null());
            assert_eq!(ugoe_error_field_count(std::ptr::null()), 0);
            assert!(ugoe_error_field_key_at(std::ptr::null(), 0).is_null());
            assert!(ugoe_error_field_value_at(std::ptr::null(), 0).is_null());
            ugoe_error_destroy(std::ptr::null_mut());
        }
    }

    #[test]
    fn kind_and_fields_follow_the_last_failure() {
        let e = TestError::new();
        let code = guard(e.0, || {
            Err(Error::new(UGOE_ERROR_DRESS_UP, "taken")
                .with_kind("slot_occupied", vec![("slot", "Hair".into())]))
        });
        assert_eq!(code, UGOE_ERROR_DRESS_UP);
        assert_eq!(e.kind().as_deref(), Some("slot_occupied"));
        assert_eq!(e.field(c"slot").as_deref(), Some("Hair"));
        assert_eq!(e.field(c"module"), None);
        // SAFETY: `e.0` は破棄していない `UgoeError`。`key` が NULL でもよい。
        assert!(unsafe { ugoe_error_field(e.0, std::ptr::null()) }.is_null());

        // 番号でも読める (T0464)。
        assert_eq!(e.field_count(), 1);
        assert_eq!(e.key_at(0).as_deref(), Some("slot"));
        assert_eq!(e.value_at(0).as_deref(), Some("Hair"));
        assert_eq!(e.key_at(1), None);
        assert_eq!(e.value_at(usize::MAX), None);

        // 種類を持たない失敗で消える。
        guard(e.0, || Err(Error::null("x")));
        assert_eq!(e.kind(), None);
        assert_eq!(e.field(c"slot"), None);
        assert_eq!(e.field_count(), 0);
        assert_eq!(e.key_at(0), None);
        assert_eq!(e.value_at(0), None);
    }

    #[test]
    fn errors_are_independent() {
        let a = TestError::new();
        let b = TestError::new();
        guard(a.0, || Err(Error::null("a")));
        guard(b.0, || Err(Error::null("b")));
        assert_eq!(a.message().as_deref(), Some("`a` is null"));
        assert_eq!(b.message().as_deref(), Some("`b` is null"));
    }
}
