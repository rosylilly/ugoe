//! `UgoePuppet`: 読み込み・破棄・パラメータ・更新・モーションと表情。

use std::ffi::{CStr, CString, c_char};

use ugoe::Puppet;
use ugoe::format::{Bundle, TextureData};
use ugoe::model::expression::Expression;
use ugoe::model::motion::Motion;
use ugoe::motion::Priority;
use ugoe::runtime::{CompiledModel, ParamIndex};

use crate::error::{
    Error, UGOE_ERROR_FORMAT, UGOE_ERROR_INVALID_ARGUMENT, UGOE_ERROR_INVALID_MODEL,
    UGOE_ERROR_NOT_FOUND, UgoeResult, guard,
};

/// 1 体分のモデルの動作単位 (Rust の `ugoe::Puppet`)。不透明な型で、中身には触れない。
///
/// `ugoe_puppet_create_*` で作り、`ugoe_puppet_destroy` で破棄する。
/// 1 つの `UgoePuppet` を複数のスレッドから同時に使ってはならない (別々の `UgoePuppet` なら並行してよい)。
#[derive(Debug)]
pub struct UgoePuppet {
    pub(crate) puppet: Puppet,
    /// パラメータの ID (NUL 終端。`Model::parameters` の並び)。
    parameter_ids: Vec<CString>,
    /// 描画可能要素 (メッシュ) の ID (NUL 終端。`Model::meshes` の並び)。
    pub(crate) drawable_ids: Vec<CString>,
    motions: Vec<Motion>,
    motion_names: Vec<CString>,
    expressions: Vec<Expression>,
    expression_names: Vec<CString>,
    /// テクスチャの番号ごとの画像。JSON から作ったときや、バンドルに無い番号は `None`。
    pub(crate) textures: Vec<Option<TextureData>>,
}

/// パラメータの範囲と既定値。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct UgoeParameterInfo {
    pub min: f32,
    pub max: f32,
    pub default_value: f32,
}

/// モデルのキャンバス (ピクセル単位・Y 下向き。`docs/spec/model.md`)。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct UgoeCanvas {
    pub width: f32,
    pub height: f32,
    /// 表示時の原点 (足元の中央など)。
    pub origin_x: f32,
    pub origin_y: f32,
}

/// 待機モーション。ほかのモーションが再生中なら開始しない (`docs/spec/motion.md#モーションマネージャ`)。
pub const UGOE_PRIORITY_IDLE: u32 = 0;
/// アプリの操作などで再生する普通のモーション。
pub const UGOE_PRIORITY_NORMAL: u32 = 1;
/// 常に開始する。
pub const UGOE_PRIORITY_FORCE: u32 = 2;

/// NUL を含む文字列は、最初の NUL までにする (ID に NUL が入ることは想定しない)。
fn c_string(s: &str) -> CString {
    let end = s.find('\0').unwrap_or(s.len());
    CString::new(&s[..end]).unwrap_or_default()
}

impl UgoePuppet {
    fn new(
        puppet: Puppet,
        motions: Vec<Motion>,
        expressions: Vec<Expression>,
        textures: Vec<Option<TextureData>>,
    ) -> Self {
        let model = puppet.instance().compiled().model();
        let parameter_ids = model
            .parameters
            .iter()
            .map(|p| c_string(p.id.as_str()))
            .collect();
        let drawable_ids = model
            .meshes
            .iter()
            .map(|m| c_string(m.id.as_str()))
            .collect();
        let motion_names = motions.iter().map(|m| c_string(&m.name)).collect();
        let expression_names = expressions.iter().map(|e| c_string(&e.name)).collect();
        Self {
            puppet,
            parameter_ids,
            drawable_ids,
            motions,
            motion_names,
            expressions,
            expression_names,
            textures,
        }
    }

    fn from_bundle(bundle: Bundle) -> Result<Self, Error> {
        let count = texture_count(&bundle.model);
        let Bundle {
            model,
            physics,
            motions,
            expressions,
            textures,
            ..
        } = bundle;
        let compiled = CompiledModel::compile(model)
            .map_err(|e| Error::new(UGOE_ERROR_INVALID_MODEL, e.to_string()))?;
        let mut puppet = Puppet::from_compiled(compiled);
        if let Some(physics) = physics {
            // `read_bundle` は物理の設定を検査済みで返す。
            puppet = puppet.with_physics(physics);
        }
        let mut textures: Vec<Option<TextureData>> = textures.into_iter().map(Some).collect();
        textures.resize(count, None);
        Ok(Self::new(puppet, motions, expressions, textures))
    }

    fn from_json(json: &str) -> Result<Self, Error> {
        let model = ugoe::format::model_from_json(json)
            .map_err(|e| Error::new(UGOE_ERROR_FORMAT, e.to_string()))?;
        let count = texture_count(&model);
        let puppet =
            Puppet::new(model).map_err(|e| Error::new(UGOE_ERROR_INVALID_MODEL, e.to_string()))?;
        Ok(Self::new(puppet, Vec::new(), Vec::new(), vec![None; count]))
    }

    fn parameter_index(&self, index: u32) -> Result<ParamIndex, Error> {
        let len = self.parameter_ids.len();
        if (index as usize) < len {
            Ok(ParamIndex(index))
        } else {
            Err(Error::out_of_range("parameter", index as usize, len))
        }
    }
}

/// 描画に使うテクスチャの枚数。`model.textures` と、メッシュが参照する番号の大きい方。
fn texture_count(model: &ugoe::model::Model) -> usize {
    model
        .meshes
        .iter()
        .map(|m| m.texture as usize + 1)
        .max()
        .unwrap_or(0)
        .max(model.textures.len())
}

/// `ptr` から `len` バイトを借りる。`len` が 0 なら `ptr` は NULL でもよい。
///
/// # Safety
///
/// `len` が 0 でなければ、`ptr` は `len` バイトの読める領域を指すこと。
pub(crate) unsafe fn bytes<'a>(ptr: *const u8, len: usize, name: &str) -> Result<&'a [u8], Error> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(Error::null(name));
    }
    // SAFETY: 呼び出し側の約束。
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

/// NUL 終端の UTF-8 文字列を借りる。
///
/// # Safety
///
/// `ptr` は NULL か、NUL 終端の文字列を指すこと。
pub(crate) unsafe fn c_str<'a>(ptr: *const c_char, name: &str) -> Result<&'a str, Error> {
    if ptr.is_null() {
        return Err(Error::null(name));
    }
    // SAFETY: 呼び出し側の約束。
    unsafe { CStr::from_ptr(ptr) }.to_str().map_err(|_| {
        Error::new(
            UGOE_ERROR_INVALID_ARGUMENT,
            format!("`{name}` is not UTF-8"),
        )
    })
}

/// `UgoePuppet` の共有参照を得る。
///
/// # Safety
///
/// `ptr` は NULL か、破棄されていない `UgoePuppet` を指すこと。
pub(crate) unsafe fn puppet_ref<'a>(ptr: *const UgoePuppet) -> Result<&'a UgoePuppet, Error> {
    // SAFETY: 呼び出し側の約束。
    unsafe { ptr.as_ref() }.ok_or_else(|| Error::null("puppet"))
}

/// `UgoePuppet` の排他参照を得る。
///
/// # Safety
///
/// `ptr` は NULL か、破棄されていない `UgoePuppet` を指し、ほかから同時に使われていないこと。
pub(crate) unsafe fn puppet_mut<'a>(ptr: *mut UgoePuppet) -> Result<&'a mut UgoePuppet, Error> {
    // SAFETY: 呼び出し側の約束。
    unsafe { ptr.as_mut() }.ok_or_else(|| Error::null("puppet"))
}

/// 出力の引数に書き込む。
///
/// # Safety
///
/// `out` は NULL か、`T` を書き込める領域を指すこと。
pub(crate) unsafe fn write<T>(out: *mut T, name: &str, value: T) -> Result<(), Error> {
    if out.is_null() {
        return Err(Error::null(name));
    }
    // SAFETY: 呼び出し側の約束。書き込み先は初期化されていなくてもよい。
    unsafe { out.write(value) };
    Ok(())
}

/// 作った `UgoePuppet` を `out` に渡す。
///
/// # Safety
///
/// `out` は NULL か、ポインタを書き込める領域を指すこと。
unsafe fn create(
    out: *mut *mut UgoePuppet,
    make: impl FnOnce() -> Result<UgoePuppet, Error>,
) -> UgoeResult {
    guard(|| {
        if out.is_null() {
            return Err(Error::null("out"));
        }
        let puppet = Box::into_raw(Box::new(make()?));
        // SAFETY: `out` は NULL でなく、呼び出し側の約束で書き込める。
        unsafe { out.write(puppet) };
        Ok(())
    })
}

/// `.ugoe` バンドルのバイト列から `UgoePuppet` を作る。
///
/// バンドルの物理・モーション・表情・テクスチャも読み込む。バイト列は呼び出しの間だけ読むので、
/// 戻った後に解放してよい。成功したら `*out` に新しい `UgoePuppet` を書き込む。
///
/// # Safety
///
/// `bytes` は `len` バイトの読める領域を指すこと。`out` は `UgoePuppet*` を書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_create_from_bundle(
    bytes: *const u8,
    len: usize,
    out: *mut *mut UgoePuppet,
) -> UgoeResult {
    // SAFETY: 呼び出し側の約束。
    unsafe {
        create(out, || {
            let bundle = ugoe::format::read_bundle(self::bytes(bytes, len, "bytes")?)
                .map_err(|e| Error::new(UGOE_ERROR_FORMAT, e.to_string()))?;
            UgoePuppet::from_bundle(bundle)
        })
    }
}

/// モデルの JSON (UTF-8、`len` バイト。NUL 終端は要らない) から `UgoePuppet` を作る。
///
/// JSON はテクスチャを持たないので、テクスチャはすべて「画像なし」になる。
///
/// # Safety
///
/// `json` は `len` バイトの読める領域を指すこと。`out` は `UgoePuppet*` を書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_create_from_json(
    json: *const u8,
    len: usize,
    out: *mut *mut UgoePuppet,
) -> UgoeResult {
    // SAFETY: 呼び出し側の約束。
    unsafe {
        create(out, || {
            let json = std::str::from_utf8(bytes(json, len, "json")?)
                .map_err(|_| Error::new(UGOE_ERROR_INVALID_ARGUMENT, "`json` is not UTF-8"))?;
            UgoePuppet::from_json(json)
        })
    }
}

/// `UgoePuppet` を破棄する。NULL なら何もしない。破棄した後は、このオブジェクトから借りたポインタも無効になる。
///
/// # Safety
///
/// `puppet` は NULL か、`ugoe_puppet_create_*` で作ってまだ破棄していない `UgoePuppet` であること。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_destroy(puppet: *mut UgoePuppet) {
    if puppet.is_null() {
        return;
    }
    // Drop の中のパニックも境界の外へ出さない。
    guard(|| {
        // SAFETY: `ugoe_puppet_create_*` が Box::into_raw で作ったポインタ。
        drop(unsafe { Box::from_raw(puppet) });
        Ok(())
    });
}

/// モデルのキャンバスの大きさと原点。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_canvas(
    puppet: *const UgoePuppet,
    out: *mut UgoeCanvas,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let canvas = p.puppet.instance().compiled().model().meta.canvas;
        let value = UgoeCanvas {
            width: canvas.width,
            height: canvas.height,
            origin_x: canvas.origin.x,
            origin_y: canvas.origin.y,
        };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", value) }
    })
}

/// 1 フレーム進める (モーション → 表情 → 目パチ・呼吸 → 物理 → リップシンク → ポーズ → 評価。
/// `docs/spec/motion.md#フレーム内の適用順`)。`dt` は前のフレームからの経過秒数。
///
/// 呼ぶと、この `UgoePuppet` から借りた頂点位置と描画順のポインタは無効になる。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_update(puppet: *mut UgoePuppet, dt: f32) -> UgoeResult {
    guard(|| {
        if !dt.is_finite() || dt < 0.0 {
            return Err(Error::new(
                UGOE_ERROR_INVALID_ARGUMENT,
                format!("`dt` must be a finite non-negative number (got {dt})"),
            ));
        }
        // SAFETY: 呼び出し側の約束。
        unsafe { puppet_mut(puppet)? }.puppet.update(dt);
        Ok(())
    })
}

/// パラメータの数。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_parameter_count(
    puppet: *const UgoePuppet,
    out: *mut u32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.parameter_ids.len() as u32) }
    })
}

/// `index` 番目のパラメータの ID (UTF-8、NUL 終端)。ポインタは `UgoePuppet` を破棄するまで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_parameter_id(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut *const c_char,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.parameter_index(index)?;
        let id = p.parameter_ids[i.0 as usize].as_ptr();
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", id) }
    })
}

/// ID (UTF-8、NUL 終端) からパラメータの番号を引く。無ければ `UGOE_ERROR_NOT_FOUND`。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`id` は NUL 終端の文字列、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_parameter_index(
    puppet: *const UgoePuppet,
    id: *const c_char,
    out: *mut u32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        let id = unsafe { c_str(id, "id")? };
        let index = p
            .puppet
            .instance()
            .parameters()
            .index_of(id)
            .ok_or_else(|| Error::new(UGOE_ERROR_NOT_FOUND, format!("no parameter `{id}`")))?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", index.0) }
    })
}

/// `index` 番目のパラメータの範囲と既定値。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_parameter_info(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut UgoeParameterInfo,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.parameter_index(index)?;
        let values = p.puppet.instance().parameters();
        let info = UgoeParameterInfo {
            min: values.min(i),
            max: values.max(i),
            default_value: values.default_value(i),
        };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", info) }
    })
}

/// `index` 番目のパラメータの現在の値。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_get_parameter(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut f32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let i = p.parameter_index(index)?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.puppet.instance().parameters().get(i)) }
    })
}

/// `index` 番目のパラメータに値を設定する (範囲に丸める)。次の `ugoe_puppet_update` から、
/// モーションなどを重ねる基準の値になる (`docs/spec/motion.md#フレーム内の適用順`)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_parameter(
    puppet: *mut UgoePuppet,
    index: u32,
    value: f32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        let i = p.parameter_index(index)?;
        if value.is_nan() {
            return Err(Error::new(UGOE_ERROR_INVALID_ARGUMENT, "`value` is NaN"));
        }
        p.puppet.instance_mut().parameters_mut().set(i, value);
        Ok(())
    })
}

/// リップシンクの音量 (0〜1) を設定する。`value` が負なら入力をやめ、
/// リップシンクは口のパラメータに触らなくなる。音声の解析はホストが行う。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_lip_sync_level(
    puppet: *mut UgoePuppet,
    value: f32,
) -> UgoeResult {
    guard(|| {
        if value.is_nan() {
            return Err(Error::new(UGOE_ERROR_INVALID_ARGUMENT, "`value` is NaN"));
        }
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        p.puppet
            .set_lip_sync_level((value >= 0.0).then(|| value.min(1.0)));
        Ok(())
    })
}

/// バンドルに入っていたモーションの数 (JSON から作ったときは 0)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_motion_count(
    puppet: *const UgoePuppet,
    out: *mut u32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.motions.len() as u32) }
    })
}

/// `index` 番目のモーションの名前 (UTF-8、NUL 終端)。ポインタは `UgoePuppet` を破棄するまで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_motion_name(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut *const c_char,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let name = p
            .motion_names
            .get(index as usize)
            .ok_or_else(|| Error::out_of_range("motion", index as usize, p.motions.len()))?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", name.as_ptr()) }
    })
}

/// `index` 番目のモーションを優先度 `priority` (`UGOE_PRIORITY_*`) で開始する。
/// 再生中のモーションより低い優先度で開始しなかったときは、`*started` に false を書く
/// (`started` は NULL でもよい)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`started` は NULL か書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_start_motion(
    puppet: *mut UgoePuppet,
    index: u32,
    priority: u32,
    started: *mut bool,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        let priority = match priority {
            UGOE_PRIORITY_IDLE => Priority::Idle,
            UGOE_PRIORITY_NORMAL => Priority::Normal,
            UGOE_PRIORITY_FORCE => Priority::Force,
            other => {
                return Err(Error::new(
                    UGOE_ERROR_INVALID_ARGUMENT,
                    format!("unknown priority {other}"),
                ));
            }
        };
        let motion = p
            .motions
            .get(index as usize)
            .ok_or_else(|| Error::out_of_range("motion", index as usize, p.motions.len()))?
            .clone();
        let result = p.puppet.start_motion(motion, priority);
        if !started.is_null() {
            // SAFETY: 呼び出し側の約束。
            unsafe { started.write(result) };
        }
        Ok(())
    })
}

/// バンドルに入っていた表情の数 (JSON から作ったときは 0)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_expression_count(
    puppet: *const UgoePuppet,
    out: *mut u32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.expressions.len() as u32) }
    })
}

/// `index` 番目の表情の名前 (UTF-8、NUL 終端)。ポインタは `UgoePuppet` を破棄するまで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_expression_name(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut *const c_char,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        let name = p.expression_names.get(index as usize).ok_or_else(|| {
            Error::out_of_range("expression", index as usize, p.expressions.len())
        })?;
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", name.as_ptr()) }
    })
}

/// 表情を `index` 番目の表情だけにする (使用中の表情はフェードアウトする)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_expression(
    puppet: *mut UgoePuppet,
    index: u32,
) -> UgoeResult {
    guard(|| {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        let expression = p
            .expressions
            .get(index as usize)
            .ok_or_else(|| Error::out_of_range("expression", index as usize, p.expressions.len()))?
            .clone();
        p.puppet.set_expression(expression);
        Ok(())
    })
}
