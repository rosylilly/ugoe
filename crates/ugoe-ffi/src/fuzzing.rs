//! fuzz のための安全な入口 (feature `fuzzing`。ヘッダには出さない。T0497。docs/testing.md#fuzz)。
//!
//! 入力のバイト列を、C のホストと同じ呼び方の C ABI の呼び出しの列 (読み込み → 問い合わせ → 設定と `update` → 解放) に
//! 組み立てて回す。fuzz のターゲット (`fuzz/fuzz_targets/ffi.rs`) はこれを呼ぶので、`unsafe` はこのクレートの中に留まる。
//!
//! 入力は `[ファイル] [SCRIPT_MARKER] [台本]` の形 (印が無ければ全体がファイルで、台本は空)。ファイルは `.ugoe` のバンドル
//! (読めなければモデルの JSON) とモジュール (`.ugoemod`。読めたら着せ替えの本体に取り付ける) として読む。台本は 1 バイトずつ
//! 操作を選び、続くバイトから番号・名前・数 (NaN と無限大を含む) を組み立てる。
//!
//! 見つけたいのは、C ABI の入口の確かめの漏れによるパニック (`UGOE_ERROR_PANIC`)・返したポインタと長さの食い違い
//! (fuzz のビルドは AddressSanitizer で、借りた領域をすべて読む)・ホストが頼る値の約束 (番号が範囲の中など) の破れ。

use alloc::borrow::ToOwned;
use alloc::ffi::CString;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::{CStr, c_char};
use core::fmt;
use core::hint::black_box;
use core::mem::MaybeUninit;
use core::ptr;

use crate::*;

/// 入力のうち、ファイルと台本を分ける印 (最後に現れたものの後ろが台本)。
pub const SCRIPT_MARKER: &[u8] = b"\0ugoe-ffi-script\0";

/// 1 つの入力で行う台本の操作の数の上限 (1 つの入力に時間をかけすぎない)。
const MAX_OPS: usize = 256;

/// 台本が JSON や配列に取る長さの上限。
const MAX_LEN: usize = 64;

/// テクスチャをデコードするときの一辺の上限 (画像そのものの fuzz は `image` のターゲットが受け持つ)。
const MAX_IMAGE_SIDE: u32 = 1024;

/// 台本の数の表 (`Script::value`)。
const SPECIAL_VALUES: [f32; 16] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    1.0 / 60.0,
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::MAX,
    f32::MIN,
    f32::MIN_POSITIVE,
    f32::EPSILON,
    1.0e30,
    -1.0e30,
    1.0e-30,
];

/// fuzz が見つけた誤り: C ABI の中のパニック (`UGOE_ERROR_PANIC`) か、返した値がホストへの約束を破ったこと。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// 誤りを見つけた C ABI の関数。
    pub call: &'static str,
    /// 中身 (パニックなら `UgoeError` のメッセージ)。
    pub message: String,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.call, self.message)
    }
}

/// 入力をファイルと台本に分ける (`SCRIPT_MARKER`)。
pub fn split_input(input: &[u8]) -> (&[u8], &[u8]) {
    let marker = SCRIPT_MARKER.len();
    let found = (0..=input.len().saturating_sub(marker))
        .rev()
        .find(|&i| input.get(i..i + marker) == Some(SCRIPT_MARKER));
    match found {
        Some(i) => (
            input.get(..i).unwrap_or_default(),
            input.get(i + marker..).unwrap_or_default(),
        ),
        None => (input, &[]),
    }
}

/// 入力 (`[ファイル] [SCRIPT_MARKER] [台本]`) を C ABI に渡す。
///
/// ファイルをバンドル (読めなければモデルの JSON) として読んで台本で動かし、モジュールとしても読んで、読めたら
/// `dress_up_base` (着せ替えの本体の `.ugoe`) に取り付けて同じ台本で動かす。読めないのはかまわない。
pub fn run(input: &[u8], dress_up_base: &[u8]) -> Result<(), Failure> {
    let (file, script) = split_input(input);
    let mut runner = Runner::new(script);
    if let Some(puppet) = runner.create_puppet(file) {
        runner.drive(puppet);
        // SAFETY: `create_puppet` が作り、まだ破棄していない。
        unsafe { ugoe_puppet_destroy(puppet) };
    }
    if let Some(module) = runner.create_module(file) {
        if let Some(base) = runner.create_puppet(dress_up_base) {
            runner.module = module;
            // SAFETY: どちらも作ってまだ破棄していない。
            let code = unsafe { ugoe_puppet_attach_module(base, module, runner.error) };
            runner.check("ugoe_puppet_attach_module", code);
            runner.drive(base);
            runner.module = ptr::null_mut();
            // SAFETY: 同上。
            unsafe { ugoe_puppet_destroy(base) };
        }
        // SAFETY: `create_module` が作り、まだ破棄していない。
        unsafe { ugoe_module_destroy(module) };
    }
    runner.finish()
}

/// 台本 (1 バイトずつ読む。尽きたら 0 を返す)。
#[derive(Debug, Clone, Copy)]
struct Script<'a> {
    bytes: &'a [u8],
}

impl<'a> Script<'a> {
    fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    fn byte(&mut self) -> u8 {
        match self.bytes.split_first() {
            Some((&b, rest)) => {
                self.bytes = rest;
                b
            }
            None => 0,
        }
    }

    fn take(&mut self, len: usize) -> &'a [u8] {
        let (head, rest) = self.bytes.split_at(len.min(self.bytes.len()));
        self.bytes = rest;
        head
    }

    fn u32(&mut self) -> u32 {
        let mut raw = [0; 4];
        for (dst, src) in raw.iter_mut().zip(self.take(4)) {
            *dst = *src;
        }
        u32::from_le_bytes(raw)
    }

    fn flag(&mut self) -> bool {
        self.byte() & 1 == 1
    }

    /// 長さ (0〜`MAX_LEN`)。
    fn len(&mut self) -> usize {
        usize::from(self.byte()) % (MAX_LEN + 1)
    }

    /// `count` 個のものの番号。ほとんどは範囲の中で、ちょうど端・少し外・`u32::MAX`・任意の値も作る。
    fn index(&mut self, count: u32) -> u32 {
        match self.byte() {
            b @ 0..=0xBF if count > 0 => u32::from(b) % count,
            b @ 0..=0xEF => count.saturating_add(u32::from(b & 0x0F)),
            0xF0 => u32::MAX,
            0xF1 => i32::MAX as u32,
            _ => self.u32(),
        }
    }

    /// 数。表 (`SPECIAL_VALUES`) の値・-8〜8 の値・任意のビット列。
    fn value(&mut self) -> f32 {
        let b = self.byte();
        match SPECIAL_VALUES.get(usize::from(b)) {
            Some(&v) => v,
            None if b == 0xFF => f32::from_bits(self.u32()),
            None => (f32::from(b) - 128.0) / 16.0,
        }
    }

    /// 名前。知っている名前 (`names`)・任意のバイト列 (UTF-8 でないものも)・空・NULL。
    fn name(&mut self, names: &[CString]) -> Option<CString> {
        match self.byte() % 5 {
            0 if !names.is_empty() => {
                let i = usize::from(self.byte()) % names.len();
                names.get(i).cloned()
            }
            0 | 1 => {
                let len = self.len();
                let raw = self.take(len);
                let head = raw.split(|&b| b == 0).next().unwrap_or_default();
                CString::new(head).ok()
            }
            2 => CString::new(b"\xff\xfe".as_slice()).ok(),
            3 => Some(CString::default()),
            _ => None,
        }
    }
}

/// 名前の C の文字列 (NULL なら NULL)。
fn name_ptr(name: &Option<CString>) -> *const c_char {
    name.as_ref().map_or(ptr::null(), |n| n.as_ptr())
}

/// 台本で C ABI を呼ぶ。失敗の中身は 1 つの `UgoeError` に受け、最初の誤りを残す。
struct Runner<'a> {
    script: &'a [u8],
    error: *mut UgoeError,
    /// 着せ替えの操作に使うモジュール (無ければ NULL)。
    module: *mut UgoeModule,
    /// 問い合わせで知った名前 (パラメータ・パーツ・当たり判定など)。台本の名前に使う。
    names: Vec<CString>,
    failure: Option<Failure>,
    /// 借りた領域から読んだ値を捨てる先 (読み出しを最適化で消さない)。
    sink: u64,
}

impl Drop for Runner<'_> {
    fn drop(&mut self) {
        // SAFETY: `ugoe_error_create` で作り、まだ破棄していない。
        unsafe { ugoe_error_destroy(self.error) };
    }
}

impl<'a> Runner<'a> {
    fn new(script: &'a [u8]) -> Self {
        Self {
            script,
            error: ugoe_error_create(),
            module: ptr::null_mut(),
            names: Vec::new(),
            failure: None,
            sink: 0,
        }
    }

    fn finish(mut self) -> Result<(), Failure> {
        black_box(self.sink);
        self.failure.take().map_or(Ok(()), Err)
    }

    fn fail(&mut self, call: &'static str, message: String) {
        if self.failure.is_none() {
            self.failure = Some(Failure { call, message });
        }
    }

    /// 結果コードを確かめる。成功なら true。失敗なら `UgoeError` の中身を読み (借りた文字列もすべて読む)、パニックなら誤りにする。
    fn check(&mut self, call: &'static str, code: UgoeResult) -> bool {
        if code == UGOE_OK {
            return true;
        }
        // SAFETY: `self.error` は破棄していない `UgoeError`。返る文字列は NULL か NUL 終端で、次の失敗まで有効。
        let message = unsafe {
            self.read_str(ugoe_error_kind(self.error));
            for i in 0..ugoe_error_field_count(self.error) {
                self.read_str(ugoe_error_field_key_at(self.error, i));
                self.read_str(ugoe_error_field_value_at(self.error, i));
            }
            // キーで引く形も、無いキー・空のキー・NULL を含めて呼ぶ
            for key in [c"limit", c"module", c"errors", c"", c"no-such-key"] {
                self.read_str(ugoe_error_field(self.error, key.as_ptr()));
            }
            self.read_str(ugoe_error_field(self.error, ptr::null()));
            let message = ugoe_error_message(self.error);
            self.read_str(message);
            if message.is_null() {
                String::new()
            } else {
                CStr::from_ptr(message).to_string_lossy().into_owned()
            }
        };
        if code == UGOE_ERROR_PANIC {
            self.fail(call, message);
        } else if !(UGOE_ERROR_NULL_POINTER..=UGOE_ERROR_LIMIT_EXCEEDED).contains(&code) {
            self.fail(call, format!("unknown result code {code}: {message}"));
        }
        false
    }

    /// 借りた C の文字列を読む。
    ///
    /// # Safety
    ///
    /// `s` は NULL か NUL 終端の文字列を指すこと (C ABI がホストに約束していること)。
    unsafe fn read_str(&mut self, s: *const c_char) {
        if !s.is_null() {
            // SAFETY: 呼び出し側の約束。
            let bytes = unsafe { CStr::from_ptr(s) }.to_bytes();
            self.sink = self.sink.wrapping_add(bytes.len() as u64);
        }
    }

    /// 借りた C の文字列を読み、名前として覚える。
    ///
    /// # Safety
    ///
    /// `read_str` と同じ。
    unsafe fn remember(&mut self, s: *const c_char) {
        // SAFETY: 呼び出し側の約束。
        unsafe { self.read_str(s) };
        if !s.is_null() && self.names.len() < 256 {
            // SAFETY: 同上。
            self.names.push(unsafe { CStr::from_ptr(s) }.to_owned());
        }
    }

    /// 借りた配列を読む。
    ///
    /// # Safety
    ///
    /// `len` が 0 でなければ、`data` は `len` 個の `T` を読める領域を指すこと (C ABI がホストに約束していること)。
    unsafe fn read_slice<T: Copy>(
        &mut self,
        call: &'static str,
        data: *const T,
        len: usize,
    ) -> &[T] {
        if len == 0 {
            return &[];
        }
        if data.is_null() {
            self.fail(call, format!("returned NULL for {len} items"));
            return &[];
        }
        // SAFETY: 呼び出し側の約束。
        let items = unsafe { core::slice::from_raw_parts(data, len) };
        // 全体を読む (AddressSanitizer が範囲の外を見つける)
        let mut sum = 0u64;
        for item in items {
            // SAFETY: `T` は `Copy` の値で、バイトとして読むだけ。
            let raw = unsafe {
                core::slice::from_raw_parts(ptr::from_ref(item).cast::<u8>(), size_of::<T>())
            };
            sum = raw
                .iter()
                .fold(sum, |acc, &b| acc.wrapping_add(u64::from(b)));
        }
        self.sink = self.sink.wrapping_add(black_box(sum));
        items
    }

    /// 台本の最初のバイトで、`UgoeError` を渡すか (渡さなければ NULL) を決める。
    fn error_arg(&self, script: &mut Script<'_>) -> *mut UgoeError {
        if script.byte() & 0x80 == 0 {
            self.error
        } else {
            ptr::null_mut()
        }
    }

    /// バンドルとして (読めなければモデルの JSON として) `UgoePuppet` を作る。
    fn create_puppet(&mut self, file: &[u8]) -> Option<*mut UgoePuppet> {
        let mut puppet = ptr::null_mut();
        // SAFETY: `file` は `file.len()` バイトの読める領域、`puppet` は書き込める。
        let code = unsafe {
            ugoe_puppet_create_from_bundle(file.as_ptr(), file.len(), &raw mut puppet, self.error)
        };
        if self.check("ugoe_puppet_create_from_bundle", code) {
            return Some(puppet);
        }
        // 小さな上限でも読み、上限を超えたら止まることを確かめる
        let limits = UgoeReadLimits {
            max_chunk_decompressed: 64,
            max_total_decompressed: 128,
        };
        // SAFETY: 同上。`limits` は読める。
        let code = unsafe {
            ugoe_puppet_create_from_bundle_with_limits(
                file.as_ptr(),
                file.len(),
                &raw const limits,
                &raw mut puppet,
                self.error,
            )
        };
        if self.check("ugoe_puppet_create_from_bundle_with_limits", code) {
            return Some(puppet);
        }
        // SAFETY: 同上。
        let code = unsafe {
            ugoe_puppet_create_from_json(file.as_ptr(), file.len(), &raw mut puppet, self.error)
        };
        self.check("ugoe_puppet_create_from_json", code)
            .then_some(puppet)
    }

    /// モジュールとして読み、読めたら名前・スロット・警告を問い合わせる。
    fn create_module(&mut self, file: &[u8]) -> Option<*mut UgoeModule> {
        let mut module = ptr::null_mut();
        // 小さな上限でも読み、上限を超えたら止まることを確かめる (読めたら破棄して、ふつうの上限で読み直す)
        let limits = UgoeReadLimits {
            max_chunk_decompressed: 64,
            max_total_decompressed: 128,
        };
        // SAFETY: `file` は `file.len()` バイトの読める領域、`limits` は読める、`module` は書き込める。
        let code = unsafe {
            ugoe_module_create_with_limits(
                file.as_ptr(),
                file.len(),
                &raw const limits,
                &raw mut module,
                self.error,
            )
        };
        if self.check("ugoe_module_create_with_limits", code) {
            // SAFETY: 作ったばかりで、ほかに渡していない。
            unsafe { ugoe_module_destroy(module) };
            module = ptr::null_mut();
        }
        // SAFETY: `file` は `file.len()` バイトの読める領域、`module` は書き込める。
        let code =
            unsafe { ugoe_module_create(file.as_ptr(), file.len(), &raw mut module, self.error) };
        if !self.check("ugoe_module_create", code) {
            return None;
        }
        let mut s = ptr::null();
        // SAFETY: `module` は作ったばかり。出力は書き込める。借りた文字列はモジュールを破棄するまで有効。
        unsafe {
            if self.check(
                "ugoe_module_name",
                ugoe_module_name(module, &raw mut s, self.error),
            ) {
                self.remember(s);
            }
            if self.check(
                "ugoe_module_slot",
                ugoe_module_slot(module, &raw mut s, self.error),
            ) {
                self.read_str(s);
            }
            let mut count = 0;
            if self.check(
                "ugoe_module_warning_count",
                ugoe_module_warning_count(module, &raw mut count, self.error),
            ) {
                for i in 0..=count {
                    let mut w = MaybeUninit::<UgoeWarning>::uninit();
                    if self.check(
                        "ugoe_module_warning",
                        ugoe_module_warning(module, i, w.as_mut_ptr(), self.error),
                    ) {
                        let w = w.assume_init();
                        self.read_str(w.code);
                        self.read_str(w.message);
                    }
                    for key in [c"field", c"feature", c"texture", c""] {
                        if self.check(
                            "ugoe_module_warning_field",
                            ugoe_module_warning_field(
                                module,
                                i,
                                key.as_ptr(),
                                &raw mut s,
                                self.error,
                            ),
                        ) {
                            self.read_str(s);
                        }
                    }
                }
            }
        }
        Some(module)
    }

    /// 問い合わせ → 台本 → 1 フレーム進めて問い合わせ。
    fn drive(&mut self, puppet: *mut UgoePuppet) {
        self.inspect(puppet);
        let mut script = Script { bytes: self.script };
        let mut ops = 0;
        while !script.is_empty() && ops < MAX_OPS {
            self.op(puppet, &mut script);
            ops += 1;
        }
        // SAFETY: `puppet` は有効。
        let code = unsafe { ugoe_puppet_update(puppet, 1.0 / 60.0, self.error) };
        self.check("ugoe_puppet_update", code);
        self.inspect(puppet);
    }

    /// 数を返す問い合わせ (`ugoe_puppet_*_count`) を呼ぶ。失敗したら 0。
    fn count(
        &mut self,
        call: &'static str,
        f: unsafe extern "C" fn(*const UgoePuppet, *mut u32, *mut UgoeError) -> UgoeResult,
        puppet: *const UgoePuppet,
    ) -> u32 {
        let mut n = 0;
        // SAFETY: `f` は C ABI の数の問い合わせで、`puppet` は有効、`n` は書き込める。
        let code = unsafe { f(puppet, &raw mut n, self.error) };
        if self.check(call, code) { n } else { 0 }
    }

    /// 番号の文字列を返す問い合わせ (`ugoe_puppet_*_id`・`*_name`) を、範囲の中と 1 つ外で呼び、名前を覚える。
    fn names_of(
        &mut self,
        call: &'static str,
        f: unsafe extern "C" fn(
            *const UgoePuppet,
            u32,
            *mut *const c_char,
            *mut UgoeError,
        ) -> UgoeResult,
        puppet: *const UgoePuppet,
        count: u32,
    ) {
        for i in 0..=count {
            let mut s = ptr::null();
            // SAFETY: `f` は C ABI の名前の問い合わせで、`puppet` は有効、`s` は書き込める。
            let code = unsafe { f(puppet, i, &raw mut s, self.error) };
            if self.check(call, code) {
                // SAFETY: 成功したら NUL 終端の文字列を指す。
                unsafe { self.remember(s) };
            }
        }
    }

    /// 読めたものをすべて問い合わせ、借りた領域をすべて読む (範囲の中と 1 つ外の番号で)。
    fn inspect(&mut self, puppet: *mut UgoePuppet) {
        let p = puppet.cast_const();
        let e = self.error;
        let mut canvas = UgoeCanvas::default();
        // SAFETY: `p` は有効、出力は書き込める。
        let code = unsafe { ugoe_puppet_canvas(p, &raw mut canvas, e) };
        self.check("ugoe_puppet_canvas", code);

        let params = self.count(
            "ugoe_puppet_parameter_count",
            ugoe_puppet_parameter_count,
            p,
        );
        self.names_of(
            "ugoe_puppet_parameter_id",
            ugoe_puppet_parameter_id,
            p,
            params,
        );
        for i in 0..=params {
            let mut info = UgoeParameterInfo::default();
            let mut value = 0.0;
            // SAFETY: `p` は有効、出力は書き込める。
            unsafe {
                let code = ugoe_puppet_parameter_info(p, i, &raw mut info, e);
                self.check("ugoe_puppet_parameter_info", code);
                let code = ugoe_puppet_get_parameter(p, i, &raw mut value, e);
                self.check("ugoe_puppet_get_parameter", code);
            }
        }

        let parts = self.count("ugoe_puppet_part_count", ugoe_puppet_part_count, p);
        self.names_of("ugoe_puppet_part_id", ugoe_puppet_part_id, p, parts);
        for i in 0..=parts {
            let mut opacity = UgoePartOpacity::default();
            let mut value = 0.0;
            // SAFETY: `p` は有効、出力は書き込める。
            unsafe {
                let code = ugoe_puppet_part_opacity(p, i, &raw mut opacity, e);
                self.check("ugoe_puppet_part_opacity", code);
                let code = ugoe_puppet_part_evaluated_opacity(p, i, &raw mut value, e);
                self.check("ugoe_puppet_part_evaluated_opacity", code);
            }
        }
        let groups = self.count(
            "ugoe_puppet_pose_group_count",
            ugoe_puppet_pose_group_count,
            p,
        );
        for g in 0..=groups {
            let (mut visible, mut entries) = (0, 0);
            // SAFETY: `p` は有効、出力は書き込める。
            unsafe {
                let code = ugoe_puppet_pose_visible_part(p, g, &raw mut visible, e);
                if self.check("ugoe_puppet_pose_visible_part", code) && visible >= parts {
                    self.fail(
                        "ugoe_puppet_pose_visible_part",
                        format!("part {visible} is out of range (count {parts})"),
                    );
                }
                let code = ugoe_puppet_pose_entry_count(p, g, &raw mut entries, e);
                self.check("ugoe_puppet_pose_entry_count", code);
                for entry in 0..=entries {
                    let mut part = 0;
                    let code = ugoe_puppet_pose_entry_part(p, g, entry, &raw mut part, e);
                    if self.check("ugoe_puppet_pose_entry_part", code) && part >= parts {
                        self.fail(
                            "ugoe_puppet_pose_entry_part",
                            format!("part {part} is out of range (count {parts})"),
                        );
                    }
                }
            }
        }

        let motions = self.count("ugoe_puppet_motion_count", ugoe_puppet_motion_count, p);
        self.names_of(
            "ugoe_puppet_motion_name",
            ugoe_puppet_motion_name,
            p,
            motions,
        );
        let expressions = self.count(
            "ugoe_puppet_expression_count",
            ugoe_puppet_expression_count,
            p,
        );
        self.names_of(
            "ugoe_puppet_expression_name",
            ugoe_puppet_expression_name,
            p,
            expressions,
        );
        let hits = self.count("ugoe_puppet_hit_area_count", ugoe_puppet_hit_area_count, p);
        self.names_of("ugoe_puppet_hit_area_id", ugoe_puppet_hit_area_id, p, hits);
        let modules = self.count(
            "ugoe_puppet_attached_module_count",
            ugoe_puppet_attached_module_count,
            p,
        );
        self.names_of(
            "ugoe_puppet_attached_module_name",
            ugoe_puppet_attached_module_name,
            p,
            modules,
        );
        let slots = self.count("ugoe_puppet_slot_count", ugoe_puppet_slot_count, p);
        self.names_of("ugoe_puppet_slot_id", ugoe_puppet_slot_id, p, slots);

        let warnings = self.count(
            "ugoe_puppet_load_warning_count",
            ugoe_puppet_load_warning_count,
            p,
        );
        for i in 0..=warnings {
            let mut w = MaybeUninit::<UgoeWarning>::uninit();
            let mut s = ptr::null();
            // SAFETY: `p` は有効、出力は書き込める。成功したら `w` は書かれ、文字列は NUL 終端。
            unsafe {
                let code = ugoe_puppet_load_warning(p, i, w.as_mut_ptr(), e);
                if self.check("ugoe_puppet_load_warning", code) {
                    let w = w.assume_init();
                    self.read_str(w.code);
                    self.read_str(w.message);
                }
                for key in [c"texture", c"feature", c"field", c""] {
                    let code = ugoe_puppet_load_warning_field(p, i, key.as_ptr(), &raw mut s, e);
                    if self.check("ugoe_puppet_load_warning_field", code) {
                        self.read_str(s);
                    }
                }
            }
        }

        self.inspect_drawables(p);
        self.inspect_textures(p);
        self.inspect_misc(p);
        self.frame(puppet);
    }

    fn inspect_drawables(&mut self, p: *const UgoePuppet) {
        let e = self.error;
        let drawables = self.count("ugoe_puppet_drawable_count", ugoe_puppet_drawable_count, p);
        let textures = self.count("ugoe_puppet_texture_count", ugoe_puppet_texture_count, p);
        for i in 0..=drawables {
            let mut info = MaybeUninit::<UgoeDrawableInfo>::uninit();
            // SAFETY: `p` は有効、出力は書き込める。
            let code = unsafe { ugoe_puppet_drawable_info(p, i, info.as_mut_ptr(), e) };
            let info = if self.check("ugoe_puppet_drawable_info", code) {
                // SAFETY: 成功したら書かれている。
                let info = unsafe { info.assume_init() };
                // SAFETY: ID は NUL 終端。
                unsafe { self.read_str(info.id) };
                if info.texture >= textures {
                    self.fail(
                        "ugoe_puppet_drawable_info",
                        format!(
                            "texture {} is out of range (count {textures})",
                            info.texture
                        ),
                    );
                }
                Some(info)
            } else {
                None
            };
            let mask_count = info.map_or(0, |info| info.mask_count);
            for m in 0..=mask_count {
                let mut mask = UgoeMask::default();
                // SAFETY: `p` は有効、出力は書き込める。
                let code = unsafe { ugoe_puppet_drawable_mask(p, i, m, &raw mut mask, e) };
                if self.check("ugoe_puppet_drawable_mask", code) && mask.drawable >= drawables {
                    self.fail(
                        "ugoe_puppet_drawable_mask",
                        format!(
                            "drawable {} is out of range (count {drawables})",
                            mask.drawable
                        ),
                    );
                }
            }
            let (mut uvs, mut indices, mut len) = (ptr::null(), ptr::null(), 0);
            // SAFETY: `p` は有効、出力は書き込める。成功したら `len` 個の領域を借りる。
            unsafe {
                let code = ugoe_puppet_drawable_uvs(p, i, &raw mut uvs, &raw mut len, e);
                if self.check("ugoe_puppet_drawable_uvs", code) {
                    self.read_slice("ugoe_puppet_drawable_uvs", uvs, len);
                    if let Some(info) = info
                        && len != info.vertex_count as usize * 2
                    {
                        self.fail(
                            "ugoe_puppet_drawable_uvs",
                            format!("{len} floats for {} vertices", info.vertex_count),
                        );
                    }
                }
                let code = ugoe_puppet_drawable_indices(p, i, &raw mut indices, &raw mut len, e);
                if self.check("ugoe_puppet_drawable_indices", code) {
                    let vertices = info.map_or(u32::MAX, |info| info.vertex_count);
                    let items = self.read_slice("ugoe_puppet_drawable_indices", indices, len);
                    if let Some(&bad) = items.iter().find(|&&v| u32::from(v) >= vertices) {
                        self.fail(
                            "ugoe_puppet_drawable_indices",
                            format!("vertex {bad} is out of range (count {vertices})"),
                        );
                    }
                }
                let mut lit = false;
                let code = ugoe_puppet_drawable_lit(p, i, &raw mut lit, e);
                self.check("ugoe_puppet_drawable_lit", code);
            }
        }
    }

    fn inspect_textures(&mut self, p: *const UgoePuppet) {
        let e = self.error;
        let textures = self.count("ugoe_puppet_texture_count", ugoe_puppet_texture_count, p);
        for i in 0..=textures {
            for (call, f) in [
                (
                    "ugoe_puppet_texture",
                    ugoe_puppet_texture
                        as unsafe extern "C" fn(
                            *const UgoePuppet,
                            u32,
                            *mut UgoeTexture,
                            *mut UgoeError,
                        ) -> UgoeResult,
                ),
                ("ugoe_puppet_normal_map", ugoe_puppet_normal_map),
            ] {
                let mut t = MaybeUninit::<UgoeTexture>::uninit();
                // SAFETY: `p` は有効、出力は書き込める。
                let code = unsafe { f(p, i, t.as_mut_ptr(), e) };
                if self.check(call, code) {
                    // SAFETY: 成功したら書かれ、`data` は `len` バイトを借りる。
                    let t = unsafe { t.assume_init() };
                    // SAFETY: 同上。
                    unsafe { self.read_slice(call, t.data, t.len) };
                    self.decode(t);
                }
            }
        }
    }

    /// ホストと同じく、借りたテクスチャを `ugoe_image_decode` でデコードする。
    fn decode(&mut self, texture: UgoeTexture) {
        let limits = UgoeImageLimits {
            max_side: MAX_IMAGE_SIDE,
        };
        let mut image = ptr::null_mut();
        // SAFETY: `data` は `len` バイトの読める領域 (`UGOE_TEXTURE_NONE` なら NULL と 0)。出力は書き込める。
        let code = unsafe {
            ugoe_image_decode(
                texture.format,
                texture.data,
                texture.len,
                &raw const limits,
                true,
                &raw mut image,
                self.error,
            )
        };
        if !self.check("ugoe_image_decode", code) {
            return;
        }
        let (mut width, mut height, mut pixels, mut len) = (0, 0, ptr::null(), 0);
        // SAFETY: `image` は作ったばかり。出力は書き込める。画素は破棄するまで借りる。
        unsafe {
            let code = ugoe_image_size(image, &raw mut width, &raw mut height, self.error);
            self.check("ugoe_image_size", code);
            let code = ugoe_image_pixels(image, &raw mut pixels, &raw mut len, self.error);
            if self.check("ugoe_image_pixels", code) {
                self.read_slice("ugoe_image_pixels", pixels, len);
                let expected = u64::from(width) * u64::from(height) * 4;
                if len as u64 != expected {
                    self.fail(
                        "ugoe_image_pixels",
                        format!("{len} bytes for {width}x{height}"),
                    );
                }
            }
            ugoe_image_destroy(image);
        }
    }

    fn inspect_misc(&mut self, p: *const UgoePuppet) {
        let e = self.error;
        let mut buffer = [0u32; 64];
        let (mut count, mut changed, mut len) = (0, ptr::null(), 0);
        let mut flag = false;
        // SAFETY: `p` は有効、出力は書き込める (`buffer` は 64 個)。借りた領域は `len` 個。
        unsafe {
            let code = ugoe_puppet_tracking_targets(
                p,
                buffer.as_mut_ptr(),
                buffer.len(),
                &raw mut count,
                e,
            );
            self.check("ugoe_puppet_tracking_targets", code);
            let code = ugoe_puppet_tracking_targets(p, ptr::null_mut(), 0, &raw mut count, e);
            self.check("ugoe_puppet_tracking_targets", code);
            let code = ugoe_puppet_changed_textures(p, &raw mut changed, &raw mut len, e);
            if self.check("ugoe_puppet_changed_textures", code) {
                self.read_slice("ugoe_puppet_changed_textures", changed, len);
            }
            let code = ugoe_puppet_uses_lighting(p, &raw mut flag, e);
            self.check("ugoe_puppet_uses_lighting", code);
            let code = ugoe_puppet_motion_over_tracking(p, &raw mut flag, e);
            self.check("ugoe_puppet_motion_over_tracking", code);
            let mut level = 0.0;
            let mut strength = 0.0;
            let code = ugoe_puppet_lip_sync_applied(p, &raw mut level, &raw mut strength, e);
            self.check("ugoe_puppet_lip_sync_applied", code);
            let mut values = [0.0f32; 16];
            let code =
                ugoe_puppet_values_before_physics(p, values.as_mut_ptr(), 16, &raw mut len, e);
            self.check("ugoe_puppet_values_before_physics", code);
            let mut opacities = [UgoePartOpacityInput::default(); 16];
            let code =
                ugoe_puppet_part_opacities_applied(p, opacities.as_mut_ptr(), 16, &raw mut len, e);
            self.check("ugoe_puppet_part_opacities_applied", code);
            let events = self.count(
                "ugoe_puppet_motion_event_count",
                ugoe_puppet_motion_event_count,
                p,
            );
            for i in 0..=events {
                let mut event = MaybeUninit::<UgoeMotionEvent>::uninit();
                let code = ugoe_puppet_motion_event(p, i, event.as_mut_ptr(), e);
                if self.check("ugoe_puppet_motion_event", code) {
                    self.read_str(event.assume_init().value);
                }
                let mut source = MaybeUninit::<UgoeMotionEventSource>::uninit();
                let code = ugoe_puppet_motion_event_source(p, i, source.as_mut_ptr(), e);
                if self.check("ugoe_puppet_motion_event_source", code) {
                    self.read_str(source.assume_init().motion);
                }
            }
        }
        self.physics_round_trip(p);
    }

    /// 物理の設定を JSON で読み出し (小さすぎるバッファでも)、そのまま戻せるかを見る。
    fn physics_round_trip(&mut self, p: *const UgoePuppet) {
        type Read = unsafe extern "C" fn(
            *const UgoePuppet,
            *mut u8,
            usize,
            *mut usize,
            *mut UgoeError,
        ) -> UgoeResult;
        for (call, f) in [
            ("ugoe_puppet_physics_json", ugoe_puppet_physics_json as Read),
            (
                "ugoe_puppet_base_physics_json",
                ugoe_puppet_base_physics_json,
            ),
        ] {
            let mut len = 0;
            // SAFETY: 容量 0 なら書き込まない。`len` は書き込める。
            let code = unsafe { f(p, ptr::null_mut(), 0, &raw mut len, self.error) };
            if code != UGOE_ERROR_BUFFER_TOO_SMALL && !self.check(call, code) {
                continue;
            }
            let mut json = vec![0u8; len.min(1 << 24)];
            // SAFETY: `json` は `json.len()` バイト書ける。
            let code = unsafe { f(p, json.as_mut_ptr(), json.len(), &raw mut len, self.error) };
            if self.check(call, code) && len > json.len() {
                self.fail(call, format!("wrote {len} bytes into {}", json.len()));
            }
        }
    }

    /// 毎フレームの状態を借りて読み、ホストのバッファにも写す。
    fn frame(&mut self, puppet: *mut UgoePuppet) {
        let mut changed = false;
        // SAFETY: `puppet` は有効、出力は書き込める。
        let code = unsafe { ugoe_puppet_changed(puppet, &raw mut changed, self.error) };
        self.check("ugoe_puppet_changed", code);
        let mut frame = MaybeUninit::<UgoeFrame>::uninit();
        // SAFETY: `puppet` は有効、出力は書き込める。
        let code = unsafe { ugoe_puppet_frame(puppet, frame.as_mut_ptr(), self.error) };
        let mut sizes = (0, 0);
        if self.check("ugoe_puppet_frame", code) {
            // SAFETY: 成功したら書かれ、ポインタは数の分だけ借りる。
            let frame = unsafe { frame.assume_init() };
            let n = frame.drawable_count as usize;
            sizes = (n, frame.positions_len);
            // SAFETY: 同上。
            unsafe {
                self.read_slice("ugoe_puppet_frame", frame.states, n);
                let order = self
                    .read_slice("ugoe_puppet_frame", frame.render_order, n)
                    .to_vec();
                let ranges = self
                    .read_slice("ugoe_puppet_frame", frame.position_ranges, n)
                    .to_vec();
                self.read_slice("ugoe_puppet_frame", frame.positions, frame.positions_len);
                self.check_frame(&order, &ranges, frame.positions_len);
            }
        }
        // ちょうどの大きさ・小さすぎる大きさ・空 (NULL) のバッファに写す
        for (drawables, positions) in [
            sizes,
            (sizes.0.saturating_sub(1), sizes.1),
            (sizes.0, sizes.1.saturating_sub(1)),
            (0, 0),
        ] {
            let mut states = vec![UgoeDrawableState::default(); drawables];
            let mut order = vec![0u32; drawables];
            let mut ranges = vec![UgoePositionRange::default(); drawables];
            let mut floats = vec![0f32; positions];
            let nonnull = drawables > 0 || positions > 0;
            let buffers = UgoeFrameBuffers {
                states: if nonnull {
                    states.as_mut_ptr()
                } else {
                    ptr::null_mut()
                },
                render_order: if nonnull {
                    order.as_mut_ptr()
                } else {
                    ptr::null_mut()
                },
                position_ranges: if nonnull {
                    ranges.as_mut_ptr()
                } else {
                    ptr::null_mut()
                },
                drawable_capacity: drawables,
                positions: if nonnull {
                    floats.as_mut_ptr()
                } else {
                    ptr::null_mut()
                },
                positions_capacity: positions,
            };
            let mut copy = UgoeFrameCopy::default();
            // SAFETY: `buffers` のポインタは容量の分だけ書ける。
            let code = unsafe {
                ugoe_puppet_copy_frame(puppet, &raw const buffers, false, &raw mut copy, self.error)
            };
            if self.check("ugoe_puppet_copy_frame", code) {
                if copy.drawable_count as usize > drawables || copy.positions_len > positions {
                    self.fail(
                        "ugoe_puppet_copy_frame",
                        format!(
                            "succeeded with {} drawables and {} floats into {drawables} and {positions}",
                            copy.drawable_count, copy.positions_len
                        ),
                    );
                } else {
                    self.check_frame(&order, &ranges, copy.positions_len);
                }
            }
        }
    }

    /// 描画順が描画可能要素の番号で、頂点の位置の範囲が配列の中にあること。
    fn check_frame(&mut self, order: &[u32], ranges: &[UgoePositionRange], positions: usize) {
        let n = order.len();
        if let Some(bad) = order.iter().find(|&&i| i as usize >= n) {
            self.fail(
                "ugoe_puppet_frame",
                format!("render order has {bad} (count {n})"),
            );
        }
        if let Some(bad) = ranges.iter().find(|r| {
            r.first
                .checked_add(r.count)
                .is_none_or(|end| end > positions)
        }) {
            self.fail(
                "ugoe_puppet_frame",
                format!("range {bad:?} is outside {positions} floats"),
            );
        }
    }

    /// 台本の操作を 1 つ行う。
    fn op(&mut self, puppet: *mut UgoePuppet, s: &mut Script<'_>) {
        let p = puppet.cast_const();
        let op = s.byte() % 24;
        let e = self.error_arg(s);
        let params = self.count(
            "ugoe_puppet_parameter_count",
            ugoe_puppet_parameter_count,
            p,
        );
        // SAFETY: `puppet` は有効で、ほかの引数は読み書きできる領域か NULL。借りた領域は数の分だけ読む。
        unsafe {
            match op {
                0 => {
                    let (index, value) = (s.index(params), s.value());
                    let code = ugoe_puppet_set_parameter(puppet, index, value, e);
                    self.check("ugoe_puppet_set_parameter", code);
                }
                1 => {
                    let n = s.len() % 5;
                    let ids: Vec<Option<CString>> = (0..n).map(|_| s.name(&self.names)).collect();
                    let ptrs: Vec<*const c_char> = ids.iter().map(name_ptr).collect();
                    let values: Vec<f32> = (0..n).map(|_| s.value()).collect();
                    let mut applied = 0;
                    let code = ugoe_puppet_set_parameters_by_id(
                        puppet,
                        ptrs.as_ptr(),
                        values.as_ptr(),
                        n,
                        if s.flag() {
                            &raw mut applied
                        } else {
                            ptr::null_mut()
                        },
                        e,
                    );
                    if self.check("ugoe_puppet_set_parameters_by_id", code) && applied > n {
                        self.fail(
                            "ugoe_puppet_set_parameters_by_id",
                            format!("applied {applied} of {n}"),
                        );
                    }
                }
                2 => {
                    let name = s.name(&self.names);
                    let mut out = 0;
                    let code = ugoe_puppet_parameter_index(p, name_ptr(&name), &raw mut out, e);
                    if self.check("ugoe_puppet_parameter_index", code) && out >= params {
                        self.fail(
                            "ugoe_puppet_parameter_index",
                            format!("index {out} is out of range (count {params})"),
                        );
                    }
                    let mut out = 0;
                    let code = ugoe_puppet_part_index(p, name_ptr(&name), &raw mut out, e);
                    self.check("ugoe_puppet_part_index", code);
                }
                3 => {
                    let code = ugoe_puppet_update(puppet, s.value(), e);
                    self.check("ugoe_puppet_update", code);
                }
                4 => {
                    let motions =
                        self.count("ugoe_puppet_motion_count", ugoe_puppet_motion_count, p);
                    let (index, priority) = (s.index(motions), s.index(3));
                    let mut started = false;
                    let started = if s.flag() {
                        &raw mut started
                    } else {
                        ptr::null_mut()
                    };
                    let code = ugoe_puppet_start_motion(puppet, index, priority, started, e);
                    self.check("ugoe_puppet_start_motion", code);
                }
                5 => {
                    let expressions = self.count(
                        "ugoe_puppet_expression_count",
                        ugoe_puppet_expression_count,
                        p,
                    );
                    let code = if s.flag() {
                        ugoe_puppet_set_expression(puppet, s.index(expressions), e)
                    } else {
                        ugoe_puppet_stop_expressions(puppet, e)
                    };
                    self.check("ugoe_puppet_set_expression", code);
                }
                6 => {
                    let code = ugoe_puppet_set_lip_sync_level(puppet, s.value(), e);
                    self.check("ugoe_puppet_set_lip_sync_level", code);
                }
                7 => {
                    let (stage, enabled) = (s.index(5), s.flag());
                    let code = ugoe_puppet_set_stage_enabled(puppet, stage, enabled, e);
                    self.check("ugoe_puppet_set_stage_enabled", code);
                }
                8 => {
                    let values: Vec<f32> = (0..s.len()).map(|_| s.value()).collect();
                    let code =
                        ugoe_puppet_set_base_values(puppet, values.as_ptr(), values.len(), e);
                    self.check("ugoe_puppet_set_base_values", code);
                }
                9 => {
                    let values: Vec<f32> = (0..s.len()).map(|_| s.value()).collect();
                    let (level, strength) = (s.value(), s.value());
                    let parts = self.count("ugoe_puppet_part_count", ugoe_puppet_part_count, p);
                    let opacities: Vec<UgoePartOpacityInput> = (0..s.len() % 5)
                        .map(|_| UgoePartOpacityInput {
                            part: s.index(parts),
                            value: s.value(),
                            weight: s.value(),
                        })
                        .collect();
                    let code = ugoe_puppet_follow(
                        puppet,
                        values.as_ptr(),
                        values.len(),
                        level,
                        strength,
                        opacities.as_ptr(),
                        opacities.len(),
                        s.value(),
                        e,
                    );
                    self.check("ugoe_puppet_follow", code);
                }
                10 => {
                    let mut input = UgoeTrackingInput {
                        head_yaw: s.value(),
                        head_pitch: s.value(),
                        head_roll: s.value(),
                        body_yaw: s.value(),
                        body_pitch: s.value(),
                        body_roll: s.value(),
                        shoulder_left: s.value(),
                        shoulder_right: s.value(),
                        blend_shapes: [s.value(); UGOE_BLEND_SHAPE_COUNT],
                    };
                    if let Some(v) = input
                        .blend_shapes
                        .get_mut(s.index(UGOE_BLEND_SHAPE_COUNT as u32) as usize)
                    {
                        *v = s.value();
                    }
                    let input = if s.flag() {
                        &raw const input
                    } else {
                        ptr::null()
                    };
                    let code = ugoe_puppet_set_tracking(puppet, input, e);
                    self.check("ugoe_puppet_set_tracking", code);
                }
                11 => {
                    let indices: Vec<u32> = (0..s.len() % 8)
                        .map(|_| s.index(UGOE_BLEND_SHAPE_COUNT as u32 + 8))
                        .collect();
                    let code =
                        ugoe_puppet_set_tracking_kept(puppet, indices.as_ptr(), indices.len(), e);
                    self.check("ugoe_puppet_set_tracking_kept", code);
                    let code = ugoe_puppet_set_motion_over_tracking(puppet, s.flag(), e);
                    self.check("ugoe_puppet_set_motion_over_tracking", code);
                }
                12 => {
                    let name = s.name(&self.names);
                    let mut shown = false;
                    let code = ugoe_puppet_show_pose(puppet, name_ptr(&name), &raw mut shown, e);
                    self.check("ugoe_puppet_show_pose", code);
                }
                13 => {
                    let (x, y) = (s.value(), s.value());
                    let mut out = [0u32; 8];
                    let mut count = 0;
                    let capacity = s.len() % (out.len() + 1);
                    let code = ugoe_puppet_hit_areas_at(
                        p,
                        x,
                        y,
                        out.as_mut_ptr(),
                        capacity,
                        &raw mut count,
                        e,
                    );
                    self.check("ugoe_puppet_hit_areas_at", code);
                    let name = s.name(&self.names);
                    let mut hit = false;
                    let code = ugoe_puppet_hit_test(p, name_ptr(&name), x, y, &raw mut hit, e);
                    self.check("ugoe_puppet_hit_test", code);
                    let code =
                        ugoe_puppet_hit_test_standard(p, name_ptr(&name), x, y, &raw mut hit, e);
                    self.check("ugoe_puppet_hit_test_standard", code);
                    let mut index = 0;
                    let code = ugoe_puppet_standard_hit_area(p, name_ptr(&name), &raw mut index, e);
                    self.check("ugoe_puppet_standard_hit_area", code);
                }
                14 => {
                    type SetJson = unsafe extern "C" fn(
                        *mut UgoePuppet,
                        *const u8,
                        usize,
                        *mut UgoeError,
                    ) -> UgoeResult;
                    let (call, f): (&'static str, SetJson) = match s.byte() % 3 {
                        0 => ("ugoe_puppet_set_physics_json", ugoe_puppet_set_physics_json),
                        1 => (
                            "ugoe_puppet_set_base_physics_json",
                            ugoe_puppet_set_base_physics_json,
                        ),
                        _ => ("ugoe_puppet_set_effects_json", ugoe_puppet_set_effects_json),
                    };
                    let len = s.len() + usize::from(s.byte()) * 4;
                    let json = s.take(len);
                    let code = f(puppet, json.as_ptr(), json.len(), e);
                    self.check(call, code);
                }
                15 => {
                    let code = ugoe_puppet_reset_physics(puppet, e);
                    self.check("ugoe_puppet_reset_physics", code);
                }
                16 => self.frame(puppet),
                17 => {
                    let ambient = [s.value(), s.value(), s.value()];
                    let lights: Vec<UgoeLight> = (0..s.len() % (UGOE_MAX_LIGHTS + 3))
                        .map(|_| UgoeLight {
                            kind: s.index(2),
                            vector: [s.value(), s.value(), s.value()],
                            color: [s.value(), s.value(), s.value()],
                            range: s.value(),
                        })
                        .collect();
                    let mut out = UgoeLightingUniform::default();
                    let ambient = if s.flag() {
                        ambient.as_ptr()
                    } else {
                        ptr::null()
                    };
                    let code = ugoe_lighting_uniform(
                        ambient,
                        lights.as_ptr(),
                        lights.len(),
                        &raw mut out,
                        e,
                    );
                    self.check("ugoe_lighting_uniform", code);
                }
                18 => {
                    let drawables =
                        self.count("ugoe_puppet_drawable_count", ugoe_puppet_drawable_count, p);
                    let (index, mask) = (s.index(drawables), s.index(4));
                    let mut out = UgoeMask::default();
                    let code = ugoe_puppet_drawable_mask(p, index, mask, &raw mut out, e);
                    self.check("ugoe_puppet_drawable_mask", code);
                }
                19 => {
                    let groups = self.count(
                        "ugoe_puppet_pose_group_count",
                        ugoe_puppet_pose_group_count,
                        p,
                    );
                    let (group, entry) = (s.index(groups), s.index(4));
                    let mut out = 0;
                    let code = ugoe_puppet_pose_entry_part(p, group, entry, &raw mut out, e);
                    self.check("ugoe_puppet_pose_entry_part", code);
                }
                20 => {
                    let mut name = ptr::null();
                    let code = ugoe_blend_shape_name(
                        s.index(UGOE_BLEND_SHAPE_COUNT as u32),
                        &raw mut name,
                        e,
                    );
                    if self.check("ugoe_blend_shape_name", code) {
                        self.read_str(name);
                    }
                    let name = s.name(&self.names);
                    let mut index = 0;
                    let code = ugoe_blend_shape_index(name_ptr(&name), &raw mut index, e);
                    self.check("ugoe_blend_shape_index", code);
                }
                21 => self.dress_up(puppet, s, e),
                22 => self.inspect(puppet),
                _ => {
                    // NULL の `UgoePuppet` は、どの関数も `UGOE_ERROR_NULL_POINTER` で断る
                    let mut n = 0;
                    let code = ugoe_puppet_parameter_count(ptr::null(), &raw mut n, e);
                    self.check("ugoe_puppet_parameter_count", code);
                    let code = ugoe_puppet_update(ptr::null_mut(), 0.0, e);
                    self.check("ugoe_puppet_update", code);
                    let code = ugoe_puppet_frame(ptr::null_mut(), ptr::null_mut(), e);
                    self.check("ugoe_puppet_frame", code);
                }
            }
        }
    }

    /// 着せ替えの操作 (モジュールの付け外しと入れ替え)。
    fn dress_up(&mut self, puppet: *mut UgoePuppet, s: &mut Script<'_>, e: *mut UgoeError) {
        let module = self.module.cast_const();
        // SAFETY: `puppet` は有効、`module` は NULL か有効。名前は NUL 終端か NULL。
        unsafe {
            match s.byte() % 4 {
                0 => {
                    let code = ugoe_puppet_attach_module(puppet, module, e);
                    self.check("ugoe_puppet_attach_module", code);
                }
                1 => {
                    let name = s.name(&self.names);
                    let code = ugoe_puppet_detach_module(puppet, name_ptr(&name), e);
                    self.check("ugoe_puppet_detach_module", code);
                }
                2 => {
                    // 同じモジュールを重ねて渡すこともある
                    let modules: Vec<*const UgoeModule> =
                        (0..s.len() % 3).map(|_| module).collect();
                    let code = ugoe_puppet_set_modules(puppet, modules.as_ptr(), modules.len(), e);
                    self.check("ugoe_puppet_set_modules", code);
                }
                _ => {
                    let mut name = ptr::null();
                    let code = ugoe_module_name(module, &raw mut name, e);
                    if self.check("ugoe_module_name", code) {
                        self.read_str(name);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_is_split_at_the_last_marker() {
        let mut input = b"file".to_vec();
        input.extend_from_slice(SCRIPT_MARKER);
        input.extend_from_slice(b"a");
        input.extend_from_slice(SCRIPT_MARKER);
        input.extend_from_slice(b"script");
        let (file, script) = split_input(&input);
        assert_eq!(script, b"script");
        assert!(file.starts_with(b"file") && file.ends_with(b"a"));
        assert_eq!(split_input(b"only file"), (&b"only file"[..], &b""[..]));
        assert_eq!(split_input(SCRIPT_MARKER), (&b""[..], &b""[..]));
    }

    #[test]
    fn script_reads_zero_after_the_end() {
        let mut s = Script { bytes: &[0xF0, 6] };
        assert_eq!(s.index(3), u32::MAX);
        assert!(s.value().is_nan());
        assert_eq!(s.byte(), 0);
        assert_eq!(s.u32(), 0);
        assert!(s.is_empty());
    }
}
