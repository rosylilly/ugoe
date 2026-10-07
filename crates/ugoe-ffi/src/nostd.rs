//! `std` の無い形 (feature `std` を外したビルド) のアロケータとパニックの処理 (ADR 0031、
//! crates/ugoe-ffi/README.md#std-の無い環境)。
//!
//! パニックのメッセージは、ホストが登録した関数があればそれに渡す (panic.rs。T0494)。
//! どちらもホストの C ライブラリの関数 (`malloc`・`calloc`・`realloc`・`aligned_alloc`・`free`・`abort`) を呼ぶだけで、
//! シンボルはホストのリンクで解決する。ゲーム機などの SDK ごとの知識はこのクレートに持たない。

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

/// ホストの C ライブラリの関数。
mod c {
    use core::ffi::c_void;

    unsafe extern "C" {
        pub(super) fn malloc(size: usize) -> *mut c_void;
        pub(super) fn calloc(count: usize, size: usize) -> *mut c_void;
        pub(super) fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
        /// C11。`size` は `alignment` の倍数であること (C17 より前の規格の制約。呼ぶ側で切り上げる)。
        pub(super) fn aligned_alloc(alignment: usize, size: usize) -> *mut c_void;
        pub(super) fn free(ptr: *mut c_void);
        pub(super) safe fn abort() -> !;
    }
}

/// `malloc` が返すポインタの整列 (C の `max_align_t`)。std の `System` と同じく、64 ビットのターゲットで 16、ほかで 8 とみなす。
/// これより大きな整列は `aligned_alloc` で確保する。
const MIN_ALIGN: usize = if cfg!(target_pointer_width = "64") {
    16
} else {
    8
};

/// `malloc` で足りるか。小さな確保では大きさまでしか整列しない `malloc` があるので、整列が大きさ以下であることも求める
/// (std の `System` と同じ判断)。
fn fits_malloc(align: usize, size: usize) -> bool {
    align <= MIN_ALIGN && align <= size
}

/// ホストの C ライブラリの `malloc`・`free` などを呼ぶアロケータ。
struct HostAllocator;

// SAFETY: 確保は C ライブラリに任せ、返すポインタは `layout` の整列を満たす (`fits_malloc` か `aligned_alloc`)。
// `realloc` は C の `realloc` が保つ中身か、写した中身を返す。
unsafe impl GlobalAlloc for HostAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if fits_malloc(layout.align(), layout.size()) {
            // SAFETY: C の関数を呼ぶだけ。
            return unsafe { c::malloc(layout.size()) }.cast();
        }
        // `aligned_alloc` はポインタの大きさ未満の整列を受け付けない実装があるので切り上げ、大きさは整列の倍数にする。
        let align = layout.align().max(size_of::<*const c_void>());
        let Some(size) = layout.size().checked_next_multiple_of(align) else {
            return core::ptr::null_mut();
        };
        // SAFETY: `align` は 2 の冪 (`Layout` の約束) で、`size` はその倍数。
        unsafe { c::aligned_alloc(align, size) }.cast()
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if fits_malloc(layout.align(), layout.size()) {
            // SAFETY: C の関数を呼ぶだけ。
            return unsafe { c::calloc(1, layout.size()) }.cast();
        }
        // SAFETY: `layout` は呼び出し側の約束どおり。
        let ptr = unsafe { self.alloc(layout) };
        if !ptr.is_null() {
            // SAFETY: `ptr` は `layout.size()` バイトを書き込める。
            unsafe { core::ptr::write_bytes(ptr, 0, layout.size()) };
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // SAFETY: `ptr` はこのアロケータ (`malloc`・`calloc`・`realloc`・`aligned_alloc`) が返したもの (`GlobalAlloc` の約束)。
        unsafe { c::free(ptr.cast()) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if fits_malloc(layout.align(), new_size) {
            // SAFETY: `ptr` はこのアロケータが返したもので、C の `realloc` に渡してよい (`aligned_alloc` の結果も含む)。
            // 新しい領域は `malloc` と同じ整列で、`layout.align()` 以下。
            return unsafe { c::realloc(ptr.cast(), new_size) }.cast();
        }
        // 大きな整列は C の `realloc` が保たないので、新しく確保して写す (`GlobalAlloc` の既定の実装と同じ)。
        // SAFETY: `new_size` は 0 でなく、整列に合わせて切り上げても `isize::MAX` を超えない (`GlobalAlloc::realloc` の約束)。
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        // SAFETY: 上の `new_layout` は正しい。
        let new = unsafe { self.alloc(new_layout) };
        if !new.is_null() {
            // SAFETY: 古い領域と新しい領域はどちらも少なくとも `min(layout.size(), new_size)` バイトあり、重ならない。
            unsafe {
                core::ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size));
                self.dealloc(ptr, layout);
            }
        }
        new
    }
}

#[global_allocator]
static ALLOCATOR: HostAllocator = HostAllocator;

/// パニックしたら、ホストが登録した関数 (`ugoe_set_panic_handler`) にメッセージと位置を渡してから、ホストの `abort` を呼ぶ。
/// std の無い環境では巻き戻せない (`catch_unwind` が無い) ので、C ABI の外へ戻らずにアプリを止める。
/// 確保の失敗 (`handle_alloc_error`) もここに来る (メッセージの書式化は確保しない。panic.rs)。
#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    // ホストの関数や書式化の中でもう一度パニックしたら、もう呼ばずに止める。
    // 別々のスレッドが同時にパニックしたときも、後のほうは呼ばずに止める (どちらにしてもアプリは止まる)。
    static PANICKING: AtomicBool = AtomicBool::new(false);
    if !PANICKING.swap(true, Ordering::AcqRel) {
        crate::panic::report(info.location(), &info.message());
    }
    c::abort()
}

/// わざとパニックする (feature `panic-test`。`just ffi-check` がパニックのメッセージがホストの関数に届くことを確かめる)。
/// ヘッダには出さない (このモジュールは cbindgen が読まない)。
#[cfg(feature = "panic-test")]
#[unsafe(no_mangle)]
pub extern "C" fn ugoe_test_panic(value: u32) {
    #[expect(clippy::panic, reason = "パニックの処理を確かめるため")]
    {
        panic!("test panic requested by the host ({value})");
    }
}
