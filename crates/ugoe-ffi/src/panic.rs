//! パニックのメッセージをホストに渡す口 (`ugoe_set_panic_handler`。T0494)。
//!
//! `std` の無い形の静的ライブラリはパニックを巻き戻せず、ホストの `abort` で止まる (nostd.rs)。止まる前に、ホストが登録した関数へ
//! メッセージと位置を渡す (crates/ugoe-ffi/README.md#パニックのメッセージをホストに渡す-t0494。ADR 0031)。
//! `std` がある形のパニックは `catch_unwind` で `UGOE_ERROR_PANIC` になり、メッセージは `UgoeError` に書くので、登録した関数は呼ばない。

use core::ffi::{c_char, c_void};
use core::fmt::{self, Display, Write as _};
use core::panic::Location;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

/// パニックのメッセージを受け取るホストの関数。`message` は UTF-8 の `len` バイト (NUL 終端もしている)。
/// `user` は `ugoe_set_panic_handler` に渡したもの。ポインタは呼ばれている間だけ有効。
/// 戻った後でライブラリはホストの `abort` を呼ぶ。関数の中でアプリを止めてもよいが、`longjmp` や例外で抜けてはならない。
pub type UgoePanicHandler =
    Option<unsafe extern "C" fn(message: *const c_char, len: usize, user: *mut c_void)>;

/// パニックのとき `UgoePanicHandler` に渡すメッセージの最大のバイト数 (NUL を除く)。超えた分は UTF-8 の文字の境界で切り詰める。
pub const UGOE_PANIC_MESSAGE_MAX: usize = 255;

/// 登録された関数 (`UgoePanicHandler` を `*mut ()` にしたもの。無ければ NULL)。
/// 64 ビットのアトミック操作の無いターゲットでも使えるよう、ポインタの幅のアトミック (読み書きだけ) に置く。
static HANDLER: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());
/// 登録された関数に渡す `user`。
static USER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

/// パニックしたときに呼ぶ関数を登録する。`handler` が NULL なら登録を外す。登録はプロセスに 1 つで、後から登録したものが勝つ。
///
/// `std` の無い形 (crates/ugoe-ffi/README.md#std-の無い環境) では、パニックすると `handler` にメッセージ
/// (`panicked at <file>:<line>:<column>: <message>`。最大 `UGOE_PANIC_MESSAGE_MAX` バイト) を渡し、戻った後でホストの `abort` を呼ぶ。
/// 書式化は固定の大きさのスタックのバッファで行い、確保しない (確保の失敗のパニックでも呼べる)。
/// `std` がある形では呼ばない (パニックは `UGOE_ERROR_PANIC` で返り、メッセージは `UgoeError` に書く)。
///
/// ほかのスレッドが ugoe の関数を呼んでいない間 (ふつうは初期化のとき) に呼ぶ。パニックと同時に登録を変えると、
/// 新しい関数に古い `user` が渡ることがある。`handler` と `user` は登録を外すまで有効であること。
#[unsafe(no_mangle)]
pub extern "C" fn ugoe_set_panic_handler(handler: UgoePanicHandler, user: *mut c_void) {
    // 関数を外してから `user` を変え、最後に関数を置く (入れ替えの途中のパニックは、なるべく呼ばずに止まる)。
    HANDLER.store(ptr::null_mut(), Ordering::Release);
    USER.store(user, Ordering::Release);
    if let Some(f) = handler {
        HANDLER.store(f as *mut (), Ordering::Release);
    }
}

/// 登録された関数があれば、パニックの位置とメッセージを書式化して渡す。nostd.rs の `#[panic_handler]` から呼ぶ。
#[cfg_attr(
    all(feature = "std", not(test)),
    expect(
        dead_code,
        reason = "std がある形のパニックは catch_unwind で UgoeError に書く"
    )
)]
pub(crate) fn report(location: Option<&Location<'_>>, message: &dyn Display) {
    let raw = HANDLER.load(Ordering::Acquire);
    if raw.is_null() {
        return;
    }
    // SAFETY: `HANDLER` に置くのは `ugoe_set_panic_handler` が受け取った `UgoePanicHandler` の関数ポインタだけ。
    let handler = unsafe {
        core::mem::transmute::<*mut (), unsafe extern "C" fn(*const c_char, usize, *mut c_void)>(
            raw,
        )
    };
    let user = USER.load(Ordering::Acquire);
    let mut buffer = MessageBuffer::new();
    // 書式化の失敗 (切り詰め) は無視する。パニックの処理の中でもう一度パニックしない。
    let _ = match location {
        Some(at) => write!(
            buffer,
            "panicked at {}:{}:{}: {message}",
            at.file(),
            at.line(),
            at.column()
        ),
        None => write!(buffer, "panicked: {message}"),
    };
    let (text, len) = buffer.finish();
    // SAFETY: ホストが登録した関数を、登録のときの約束どおりに呼ぶ。`text` は `len` バイトと NUL で、呼んでいる間は有効。
    unsafe { handler(text.cast(), len, user) };
}

/// メッセージを書く固定の大きさのバッファ (スタックに置き、確保しない)。入りきらない分は文字の境界で捨てる。
struct MessageBuffer {
    bytes: [u8; UGOE_PANIC_MESSAGE_MAX + 1],
    len: usize,
}

impl MessageBuffer {
    fn new() -> Self {
        Self {
            bytes: [0; UGOE_PANIC_MESSAGE_MAX + 1],
            len: 0,
        }
    }

    /// NUL 終端したバイト列の先頭と、NUL を除く長さ。
    fn finish(&mut self) -> (*const u8, usize) {
        if let Some(end) = self.bytes.get_mut(self.len) {
            *end = 0;
        }
        (self.bytes.as_ptr(), self.len)
    }

    #[cfg(test)]
    fn as_str(&self) -> &str {
        core::str::from_utf8(self.bytes.get(..self.len).unwrap_or_default()).unwrap_or_default()
    }
}

impl fmt::Write for MessageBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let room = UGOE_PANIC_MESSAGE_MAX.saturating_sub(self.len);
        let take = if s.len() <= room {
            s.len()
        } else {
            s.floor_char_boundary(room)
        };
        let end = self.len.saturating_add(take);
        if let (Some(dst), Some(src)) =
            (self.bytes.get_mut(self.len..end), s.as_bytes().get(..take))
        {
            dst.copy_from_slice(src);
            self.len = end;
        }
        // 入りきらなければ書式化を止める (続きは捨てる)
        if take < s.len() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::guard;
    use std::sync::Mutex;

    #[test]
    fn short_message_is_written_whole_and_nul_terminated() {
        let mut buffer = MessageBuffer::new();
        let (file, line) = ("a.rs", 3);
        write!(buffer, "panicked at {file}:{line}: boom").unwrap();
        assert_eq!(buffer.as_str(), "panicked at a.rs:3: boom");
        let (text, len) = buffer.finish();
        // SAFETY: `finish` は `len` バイトの後に NUL を置く。
        assert_eq!(unsafe { *text.add(len) }, 0);
    }

    #[test]
    fn long_message_is_truncated_at_a_char_boundary() {
        let mut buffer = MessageBuffer::new();
        // 3 バイトの文字を並べ、上限 (255 = 3 × 85) をまたぐように 1 バイトずらす
        let text = format!("x{}", "あ".repeat(200));
        assert!(write!(buffer, "{text}").is_err());
        assert_eq!(buffer.len, 1 + 3 * 84);
        assert!(buffer.as_str().starts_with("xあ"));
        assert!(buffer.as_str().ends_with('あ'));
        // 残りの 2 バイトには ASCII なら入り、いっぱいになった後は何も書かない
        assert!(buffer.write_str("yy").is_ok());
        assert_eq!(buffer.len, UGOE_PANIC_MESSAGE_MAX);
        assert!(buffer.write_str("y").is_err());
        assert_eq!(buffer.len, UGOE_PANIC_MESSAGE_MAX);
        let exact = "z".repeat(UGOE_PANIC_MESSAGE_MAX);
        let mut full = MessageBuffer::new();
        assert!(full.write_str(&exact).is_ok());
        assert_eq!(full.finish().1, UGOE_PANIC_MESSAGE_MAX);
    }

    static RECEIVED: Mutex<Vec<(String, usize)>> = Mutex::new(Vec::new());

    unsafe extern "C" fn record(message: *const c_char, len: usize, user: *mut c_void) {
        // SAFETY: `report` は `len` バイトの UTF-8 を渡す。
        let bytes = unsafe { core::slice::from_raw_parts(message.cast::<u8>(), len) };
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        RECEIVED.lock().unwrap().push((text, user as usize));
    }

    /// 登録はプロセスに 1 つなので、登録を使うテストはこの 1 つにまとめる (並行するテストと混ざらないように)。
    #[test]
    fn registered_handler_receives_location_and_message() {
        // 登録が無ければ何もしない
        report(Some(Location::caller()), &"ignored");
        assert!(RECEIVED.lock().unwrap().is_empty());

        ugoe_set_panic_handler(Some(record), 7 as *mut c_void);
        let at = Location::caller();
        report(Some(at), &format_args!("value {}", 42));
        report(None, &"no location");
        {
            let received = RECEIVED.lock().unwrap();
            assert_eq!(
                received[0],
                (
                    format!(
                        "panicked at {}:{}:{}: value 42",
                        at.file(),
                        at.line(),
                        at.column()
                    ),
                    7
                )
            );
            assert_eq!(received[1], ("panicked: no location".to_owned(), 7));
        }

        // std がある形 (テストも) のパニックは UgoeError で返り、登録した関数は呼ばない
        let code = guard(core::ptr::null_mut(), || panic!("caught"));
        assert_eq!(code, crate::UGOE_ERROR_PANIC);
        assert_eq!(RECEIVED.lock().unwrap().len(), 2);

        // 外すと呼ばない
        ugoe_set_panic_handler(None, core::ptr::null_mut());
        report(None, &"after unregister");
        assert_eq!(RECEIVED.lock().unwrap().len(), 2);
    }
}
