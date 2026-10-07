//! 着せ替え: `.ugoemod` の読み込み (`UgoeModule`) と、`UgoePuppet` へのモジュールの付け外し
//! (`docs/spec/modules.md#puppet-での着せ替え`)。
//!
//! 失敗の種類 (`ugoe_error_kind`) と補足の値は SDK の `ugoe::DressUpError::code`・`fields` で、
//! wasm の `DressUpError` の `code` と同じ (T0412)。

use std::ffi::{CString, c_char};
use std::sync::Arc;

use ugoe::DressUpError;
use ugoe::format::TextureData;
use ugoe::format::module_file;
use ugoe::model::module::Module;

use crate::error::{Error, UGOE_ERROR_DRESS_UP, UgoeError, UgoeResult, c_string, guard};
use crate::limits::{UgoeReadLimits, format_error, read_limits};
use crate::puppet::{UgoePuppet, c_str, puppet_mut, puppet_ref, write};
use crate::warning::{
    UgoeWarning, Warning, missing_textures, read_warnings, write_warning, write_warning_field,
};

/// 読み込んだ `.ugoemod` の中身。`UgoeModule` と、それを付けた `UgoePuppet` が共有する。
#[derive(Debug)]
pub(crate) struct ModuleData {
    pub(crate) module: Arc<Module>,
    /// `Module::textures` の順の画像 (エンコードされたまま)。
    pub(crate) textures: Vec<TextureData>,
    /// 法線マップ (添字 = `Module::textures` の番号)。
    pub(crate) normal_maps: Vec<Option<TextureData>>,
}

/// 読み込んだ `.ugoemod` 1 つ (着せ替えのモジュール)。不透明な型。
///
/// `ugoe_module_create` で作り、`ugoe_module_destroy` で破棄する。`UgoePuppet` に付けたモジュールは、
/// `UgoeModule` を破棄しても付いたまま残る (中身は共有される)。1 つの `UgoeModule` は複数の `UgoePuppet` に付けてよい。
#[derive(Debug)]
pub struct UgoeModule {
    data: Arc<ModuleData>,
    name: CString,
    slot: CString,
    warnings: Vec<Warning>,
}

/// 着せ替えの失敗を、種類と補足の値の付いたエラーにする。番号の補足の値は 10 進の文字列にする。
pub(crate) fn dress_up_error(error: DressUpError) -> Error {
    let fields = error
        .fields()
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect();
    Error::new(UGOE_ERROR_DRESS_UP, error.to_string()).with_kind(error.code(), fields)
}

impl UgoePuppet {
    /// モジュールの組を `modules` にする (合成の順)。失敗したときは何も変えない。
    fn set_module_data(&mut self, modules: Vec<Arc<ModuleData>>) -> Result<(), Error> {
        let arcs = modules.iter().map(|m| Arc::clone(&m.module)).collect();
        let change = self.puppet.set_modules(arcs).map_err(dress_up_error)?;
        self.modules = modules;
        self.changed_textures = change.textures;
        self.refresh_model();
        Ok(())
    }
}

/// `.ugoemod` のバイト列から `UgoeModule` を作る。バイト列は呼び出しの間だけ読む。
/// 本体との整合 (スロット・パラメータ) は付けるときに検査する。
///
/// 読めなければ `UGOE_ERROR_FORMAT` を返し、`ugoe_error_kind` は `"invalid_file"` になる。
/// 既定の読み込みの上限を超えたら `UGOE_ERROR_LIMIT_EXCEEDED` (`ugoe_module_create_with_limits` を参照)。
///
/// # Safety
///
/// `bytes` は `len` バイトの読める領域を指すこと。`out` は `UgoeModule*` を書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_module_create(
    bytes: *const u8,
    len: usize,
    out: *mut *mut UgoeModule,
    error: *mut UgoeError,
) -> UgoeResult {
    // SAFETY: 呼び出し側の約束。
    unsafe { ugoe_module_create_with_limits(bytes, len, std::ptr::null(), out, error) }
}

/// 読み込みの上限 `limits` を指定して、`.ugoemod` のバイト列から `UgoeModule` を作る (T0152)。
///
/// `limits` が NULL なら既定値 (`ugoe_module_create` と同じ)。上限を超えたら `UGOE_ERROR_LIMIT_EXCEEDED` を返し、
/// `ugoe_error_kind` は `"read_limit_exceeded"` (補足の値は `"limit"`・`"chunk"`・`"max"`)。ほかは `ugoe_module_create` と同じ。
///
/// # Safety
///
/// `bytes` は `len` バイトの読める領域を指すこと。`limits` は NULL か `UgoeReadLimits` を読める領域を指すこと。
/// `out` は `UgoeModule*` を書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_module_create_with_limits(
    bytes: *const u8,
    len: usize,
    limits: *const UgoeReadLimits,
    out: *mut *mut UgoeModule,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        if out.is_null() {
            return Err(Error::null("out"));
        }
        // SAFETY: 呼び出し側の約束。
        let (bytes, limits) = unsafe {
            (
                crate::puppet::bytes(bytes, len, "bytes")?,
                read_limits(limits),
            )
        };
        let (file, report) = module_file::read_module_with(bytes, limits)
            .map_err(|e| format_error(&e, Some(DressUpError::INVALID_FILE)))?;
        let count = file.module.textures.len();
        let normal_maps = (0..count).map(|i| file.normal_map(i).cloned()).collect();
        let mut textures: Vec<Option<TextureData>> =
            file.textures.iter().cloned().map(Some).collect();
        textures.resize(count, None);
        let mut warnings = read_warnings(&report);
        warnings.extend(missing_textures(&textures));
        let module = UgoeModule {
            name: c_string(&file.module.name),
            slot: c_string(file.module.slot.as_str()),
            warnings,
            data: Arc::new(ModuleData {
                module: Arc::new(file.module),
                textures: file.textures,
                normal_maps,
            }),
        };
        let module = Box::into_raw(Box::new(module));
        // SAFETY: `out` は NULL でなく、呼び出し側の約束で書き込める。
        unsafe { out.write(module) };
        Ok(())
    })
}

/// `UgoeModule` を破棄する。NULL なら何もしない。`UgoePuppet` に付けたモジュールは付いたまま残る。
///
/// # Safety
///
/// `module` は NULL か、`ugoe_module_create` で作ってまだ破棄していない `UgoeModule` であること。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_module_destroy(module: *mut UgoeModule) {
    if module.is_null() {
        return;
    }
    guard(std::ptr::null_mut(), || {
        // SAFETY: `ugoe_module_create` が Box::into_raw で作ったポインタ。
        drop(unsafe { Box::from_raw(module) });
        Ok(())
    });
}

/// `UgoeModule` の共有参照を得る。
///
/// # Safety
///
/// `ptr` は NULL か、破棄されていない `UgoeModule` を指すこと。
unsafe fn module_ref<'a>(ptr: *const UgoeModule) -> Result<&'a UgoeModule, Error> {
    // SAFETY: 呼び出し側の約束。
    unsafe { ptr.as_ref() }.ok_or_else(|| Error::null("module"))
}

/// モジュールの名前 (UTF-8、NUL 終端)。ポインタは `UgoeModule` を破棄するまで有効。
///
/// # Safety
///
/// `module` は有効な `UgoeModule`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_module_name(
    module: *const UgoeModule,
    out: *mut *const c_char,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let m = unsafe { module_ref(module)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", m.name.as_ptr()) }
    })
}

/// モジュールを付けるスロットの ID (UTF-8、NUL 終端)。ポインタは `UgoeModule` を破棄するまで有効。
///
/// # Safety
///
/// `module` は有効な `UgoeModule`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_module_slot(
    module: *const UgoeModule,
    out: *mut *const c_char,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let m = unsafe { module_ref(module)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", m.slot.as_ptr()) }
    })
}

/// モジュールを読み込んだときの警告の数。
///
/// # Safety
///
/// `module` は有効な `UgoeModule`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_module_warning_count(
    module: *const UgoeModule,
    out: *mut u32,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let m = unsafe { module_ref(module)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", m.warnings.len() as u32) }
    })
}

/// モジュールを読み込んだときの `index` 番目の警告。文字列は `UgoeModule` を破棄するまで有効。
///
/// # Safety
///
/// `module` は有効な `UgoeModule`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_module_warning(
    module: *const UgoeModule,
    index: u32,
    out: *mut UgoeWarning,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let m = unsafe { module_ref(module)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write_warning(&m.warnings, index, out) }
    })
}

/// モジュールを読み込んだときの `index` 番目の警告の補足の値 `key` (`ugoe_puppet_load_warning_field` と同じ)。
///
/// # Safety
///
/// `module` は有効な `UgoeModule`、`key` は NUL 終端の文字列、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_module_warning_field(
    module: *const UgoeModule,
    index: u32,
    key: *const c_char,
    out: *mut *const c_char,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let m = unsafe { module_ref(module)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write_warning_field(&m.warnings, index, key, out) }
    })
}

/// モジュールを付ける。同じスロットか同じ名前のモジュールが付いていれば、それと差し替える
/// (合成の順の同じ位置に入る)。そうでなければ最後に加える。
///
/// 成功すると、モデルが差し替わる。この `UgoePuppet` から借りたポインタ (ID・頂点・UV・インデックス・描画順・
/// 外したモジュールのテクスチャ) は無効になり、描画可能要素の数や並びも変わる。設定し直すテクスチャの番号は
/// `ugoe_puppet_changed_textures` で得る。失敗したら `UGOE_ERROR_DRESS_UP` を返し、何も変えない。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`module` は有効な `UgoeModule` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_attach_module(
    puppet: *mut UgoePuppet,
    module: *const UgoeModule,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let m = unsafe { module_ref(module)? };
        // 組み替えの規則は `Puppet::attach_module` と同じ。
        let new = &m.data.module;
        let same = |d: &Arc<ModuleData>| d.module.slot == new.slot || d.module.name == new.name;
        let mut modules = p.modules.clone();
        match modules.iter().position(same) {
            Some(i) => {
                modules[i] = Arc::clone(&m.data);
                let mut k = 0;
                modules.retain(|d| {
                    k += 1;
                    k - 1 == i || !same(d)
                });
            }
            None => modules.push(Arc::clone(&m.data)),
        }
        p.set_module_data(modules)
    })
}

/// 名前が `name` (UTF-8、NUL 終端) のモジュールを外す。付いていなければ `UGOE_ERROR_DRESS_UP`
/// (種類は `"not_attached"`)。成功したときのポインタの扱いは `ugoe_puppet_attach_module` と同じ。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`name` は NUL 終端の文字列を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_detach_module(
    puppet: *mut UgoePuppet,
    name: *const c_char,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let name = unsafe { c_str(name, "name")? };
        let mut modules = p.modules.clone();
        modules.retain(|m| m.module.name != name);
        if modules.len() == p.modules.len() {
            return Err(dress_up_error(DressUpError::NotAttached(name.to_owned())));
        }
        p.set_module_data(modules)
    })
}

/// 付けるモジュールの組を `modules` (`count` 個、合成の順) にする。`count` が 0 なら本体だけに戻す
/// (`modules` は NULL でもよい)。成功したときのポインタの扱いは `ugoe_puppet_attach_module` と同じ。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`modules` は `count` 個の有効な `UgoeModule*` の並びを指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_modules(
    puppet: *mut UgoePuppet,
    modules: *const *const UgoeModule,
    count: usize,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        let list: &[*const UgoeModule] = if count == 0 {
            &[]
        } else if modules.is_null() {
            return Err(Error::null("modules"));
        } else {
            // SAFETY: 呼び出し側の約束で、`modules` は `count` 個のポインタを読める。
            unsafe { std::slice::from_raw_parts(modules, count) }
        };
        let data = list
            .iter()
            // SAFETY: 呼び出し側の約束で、各要素は有効な `UgoeModule`。
            .map(|&m| unsafe { module_ref(m) }.map(|m| Arc::clone(&m.data)))
            .collect::<Result<_, _>>()?;
        p.set_module_data(data)
    })
}

/// 付けているモジュールの数。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_attached_module_count(
    puppet: *const UgoePuppet,
    out: *mut u32,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.module_names.len() as u32) }
    })
}

/// 付けている `index` 番目 (合成の順) のモジュールの名前 (UTF-8、NUL 終端)。
/// ポインタは、次の着せ替えの操作か `UgoePuppet` の破棄まで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_attached_module_name(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut *const c_char,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let name = p.module_names.get(index as usize).ok_or_else(|| {
            Error::out_of_range("attached module", index as usize, p.module_names.len())
        })?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", name.as_ptr()) }
    })
}

/// 本体のモデルのスロットの数 (モジュールを付けていても本体のもの)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_slot_count(
    puppet: *const UgoePuppet,
    out: *mut u32,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.slot_ids.len() as u32) }
    })
}

/// 本体の `index` 番目のスロットの ID (UTF-8、NUL 終端。モジュールの `ugoe_module_slot` と比べる)。
/// ポインタは `UgoePuppet` を破棄するまで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_slot_id(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut *const c_char,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let id = p
            .slot_ids
            .get(index as usize)
            .ok_or_else(|| Error::out_of_range("slot", index as usize, p.slot_ids.len()))?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", id.as_ptr()) }
    })
}

/// 直前の着せ替えの操作で、画像 (と法線マップ) を設定し直すテクスチャの番号 (昇順) を借りる。
/// 出どころが変わったか新しく増えた番号で、`ugoe_puppet_texture` / `ugoe_puppet_normal_map` で引き直す。
/// `ugoe_puppet_texture_count` 以上の番号の画像は、もう使われない (捨ててよい)。
/// 着せ替えをしていなければ空。ポインタは、次の着せ替えの操作か `UgoePuppet` の破棄まで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` と `out_len` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_changed_textures(
    puppet: *const UgoePuppet,
    out: *mut *const u32,
    out_len: *mut usize,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        if out.is_null() {
            return Err(Error::null("out"));
        }
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out_len, "out_len", p.changed_textures.len())? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.changed_textures.as_ptr()) }
    })
}

#[cfg(test)]
mod tests {
    use ugoe::model::validate::ValidationError;
    use ugoe::runtime::ComposeError;

    use super::*;
    use crate::error::TestError;

    /// 補足の値は wasm (`crates/ugoe-wasm/src/dress_up/tests.rs` の `numbered_and_referenced_fields_reach_js`) と同じキーで出る。
    /// 番号は 10 進の文字列 (wasm では数)。無い値 (本体がプロファイルを持たないときの `actual`) は NULL。
    #[test]
    fn numbered_and_referenced_fields_reach_the_c_abi() {
        let s = |v: &str| v.to_owned();
        let invalid = ValidationError::InvalidModuleName { name: s("") };
        type Fields = Vec<(&'static str, Option<&'static str>)>;
        let cases: Vec<(DressUpError, &str, Fields)> = vec![
            (
                ComposeError::UnknownMask {
                    module: s("m"),
                    mesh: s("x"),
                    mask: s("k"),
                }
                .into(),
                "unknown_mask",
                vec![
                    ("module", Some("m")),
                    ("mesh", Some("x")),
                    ("mask", Some("k")),
                ],
            ),
            (
                ComposeError::UnknownCollider {
                    module: s("m"),
                    rig: s("r"),
                    collider: s("c"),
                }
                .into(),
                "unknown_collider",
                vec![
                    ("module", Some("m")),
                    ("rig", Some("r")),
                    ("collider", Some("c")),
                ],
            ),
            (
                ComposeError::PoseNotExposed {
                    module: s("m"),
                    slot: s("s"),
                    part: s("p"),
                }
                .into(),
                "pose_not_exposed",
                vec![
                    ("module", Some("m")),
                    ("slot", Some("s")),
                    ("part", Some("p")),
                ],
            ),
            (
                ComposeError::TextureOutOfRange {
                    module: s("m"),
                    mesh: s("x"),
                    texture: 3,
                }
                .into(),
                "texture_out_of_range",
                vec![
                    ("module", Some("m")),
                    ("mesh", Some("x")),
                    ("texture", Some("3")),
                ],
            ),
            (
                ComposeError::AvatarLevel {
                    module: s("m"),
                    required: 2,
                    actual: Some(1),
                }
                .into(),
                "avatar_level",
                vec![
                    ("module", Some("m")),
                    ("required", Some("2")),
                    ("actual", Some("1")),
                ],
            ),
            (
                ComposeError::AvatarLevel {
                    module: s("m"),
                    required: 2,
                    actual: None,
                }
                .into(),
                "avatar_level",
                vec![
                    ("module", Some("m")),
                    ("required", Some("2")),
                    ("actual", None),
                ],
            ),
            // 検査のエラーの種類の一覧は、出た順に重なりを除いて `, ` でつないだ文字列 (wasm と同じ。T0168)。
            (
                DressUpError::Physics(vec![
                    invalid.clone(),
                    ValidationError::DuplicateId {
                        kind: "slot",
                        id: s("s"),
                    },
                    invalid.clone(),
                ]),
                "invalid_physics",
                vec![
                    ("errors", Some("3")),
                    ("error_codes", Some("invalid_module_name, duplicate_id")),
                ],
            ),
            (
                ComposeError::Invalid(vec![invalid]).into(),
                "invalid_composition",
                vec![
                    ("errors", Some("1")),
                    ("error_codes", Some("invalid_module_name")),
                ],
            ),
        ];
        let out = TestError::new();
        for (error, code, fields) in cases {
            assert_eq!(
                guard(out.0, || Err(dress_up_error(error))),
                UGOE_ERROR_DRESS_UP
            );
            assert_eq!(out.kind().as_deref(), Some(code));
            // 番号で列挙すると、値を持つ補足の値だけが同じ順に並ぶ (T0464)。
            let listed: Vec<_> = (0..out.field_count())
                .map(|i| (out.key_at(i), out.value_at(i)))
                .collect();
            let expected: Vec<_> = fields
                .iter()
                .filter_map(|(k, v)| v.map(|v| (Some(k.to_string()), Some(v.to_owned()))))
                .collect();
            assert_eq!(listed, expected, "{code}");
            for (key, value) in fields {
                let key = CString::new(key).unwrap();
                assert_eq!(out.field(&key).as_deref(), value, "{code} {key:?}");
            }
        }
    }
}
