//! `UgoePuppet`: 読み込み・破棄・パラメータ・更新・モーションと表情。

use std::ffi::{CStr, CString, c_char};
use std::sync::Arc;

use ugoe::format::{Bundle, ReadReport, TextureData};
use ugoe::model::expression::Expression;
use ugoe::model::motion::Motion;
use ugoe::motion::Priority;
use ugoe::runtime::{CompiledModel, ParamIndex};
use ugoe::{Puppet, TextureSource};

use crate::animation::MotionEvents;
use crate::dress_up::ModuleData;
use crate::error::{
    Error, UGOE_ERROR_INVALID_ARGUMENT, UGOE_ERROR_NOT_FOUND, UgoeError, UgoeResult, c_string,
    guard,
};
use crate::limits::{UgoeReadLimits, load_error, read_limits};
use crate::warning::{
    UgoeWarning, Warning, missing_textures, read_warnings, write_warning, write_warning_field,
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
    /// 当たり判定の ID (NUL 終端。`Model::hit_areas` の並び)。
    pub(crate) hit_area_ids: Vec<CString>,
    /// パーツの ID (NUL 終端。`Model::parts` の並び)。
    pub(crate) part_ids: Vec<CString>,
    motions: Vec<Motion>,
    motion_names: Vec<CString>,
    expressions: Vec<Expression>,
    expression_names: Vec<CString>,
    /// 本体のテクスチャの番号ごとの画像。JSON から作ったときや、バンドルに無い番号は `None`。
    pub(crate) base_textures: Vec<Option<TextureData>>,
    /// 本体のテクスチャの番号ごとの法線マップ。無い番号は `None`。
    pub(crate) base_normal_maps: Vec<Option<TextureData>>,
    /// 取り付けているモジュールの中身 (画像を含む。合成の順)。テクスチャの借用はこれを指す。
    pub(crate) modules: Vec<Arc<ModuleData>>,
    /// 取り付けているモジュールの名前 (NUL 終端。合成の順)。
    pub(crate) module_names: Vec<CString>,
    /// 合成したモデルのテクスチャの出どころ (`Puppet::texture_sources`)。
    pub(crate) texture_sources: Vec<TextureSource>,
    /// 直前の着せ替えで設定し直すテクスチャの番号 (`DressUpChange::textures`)。
    pub(crate) changed_textures: Vec<u32>,
    /// 本体のスロットの ID (NUL 終端)。
    pub(crate) slot_ids: Vec<CString>,
    /// 読み込みの警告。
    pub(crate) warnings: Vec<Warning>,
    /// 直前の `update` で発火したモーションのイベント。
    pub(crate) motion_events: MotionEvents,
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

impl UgoePuppet {
    fn new(
        puppet: Puppet,
        motions: Vec<Motion>,
        expressions: Vec<Expression>,
        base_textures: Vec<Option<TextureData>>,
        base_normal_maps: Vec<Option<TextureData>>,
        warnings: Vec<Warning>,
    ) -> Self {
        let motion_names = motions.iter().map(|m| c_string(&m.name)).collect();
        let expression_names = expressions.iter().map(|e| c_string(&e.name)).collect();
        let slot_ids = puppet
            .base_model()
            .slots
            .iter()
            .map(|s| c_string(s.id.as_str()))
            .collect();
        let mut this = Self {
            puppet,
            parameter_ids: Vec::new(),
            drawable_ids: Vec::new(),
            hit_area_ids: Vec::new(),
            part_ids: Vec::new(),
            motions,
            motion_names,
            expressions,
            expression_names,
            base_textures,
            base_normal_maps,
            modules: Vec::new(),
            module_names: Vec::new(),
            texture_sources: Vec::new(),
            changed_textures: Vec::new(),
            slot_ids,
            warnings,
            motion_events: MotionEvents::default(),
        };
        this.refresh_model();
        this
    }

    /// モデルから作る ID の一覧などを作り直す (作ったときと、着せ替えでモデルを差し替えたとき)。
    pub(crate) fn refresh_model(&mut self) {
        let model = self.puppet.instance().compiled().model();
        self.parameter_ids = model
            .parameters
            .iter()
            .map(|p| c_string(p.id.as_str()))
            .collect();
        self.drawable_ids = model
            .meshes
            .iter()
            .map(|m| c_string(m.id.as_str()))
            .collect();
        self.hit_area_ids = model
            .hit_areas
            .iter()
            .map(|h| c_string(h.id.as_str()))
            .collect();
        self.part_ids = model
            .parts
            .iter()
            .map(|p| c_string(p.id.as_str()))
            .collect();
        self.module_names = self
            .modules
            .iter()
            .map(|m| c_string(&m.module.name))
            .collect();
        self.texture_sources = self.puppet.texture_sources();
    }

    fn from_bundle(bundle: Bundle, report: ReadReport) -> Result<Self, Error> {
        let count = texture_count(&bundle.model);
        let base_normal_maps = (0..count).map(|i| bundle.normal_map(i).cloned()).collect();
        let Bundle {
            model,
            physics,
            motions,
            expressions,
            textures,
            ..
        } = bundle;
        let compiled = CompiledModel::compile(model).map_err(|e| load_error(e.into()))?;
        let mut puppet = Puppet::from_compiled(compiled);
        if let Some(physics) = physics {
            // `read_bundle` は物理の設定を検査済みで返す。
            puppet = puppet.with_physics(physics);
        }
        let mut textures: Vec<Option<TextureData>> = textures.into_iter().map(Some).collect();
        textures.resize(count, None);
        let mut warnings = read_warnings(&report);
        warnings.extend(missing_textures(&textures));
        Ok(Self::new(
            puppet,
            motions,
            expressions,
            textures,
            base_normal_maps,
            warnings,
        ))
    }

    fn from_json(json: &str) -> Result<Self, Error> {
        let model = ugoe::format::model_from_json(json).map_err(|e| load_error(e.into()))?;
        let count = texture_count(&model);
        let puppet = Puppet::new(model).map_err(|e| load_error(e.into()))?;
        Ok(Self::new(
            puppet,
            Vec::new(),
            Vec::new(),
            vec![None; count],
            vec![None; count],
            Vec::new(),
        ))
    }

    /// 合成したモデルのテクスチャ `index` の画像と法線マップ。範囲外なら `None`。
    pub(crate) fn texture_data(
        &self,
        index: usize,
    ) -> Option<(Option<&TextureData>, Option<&TextureData>)> {
        if index >= self.texture_count() {
            return None;
        }
        match self.texture_sources.get(index) {
            Some(TextureSource::Module { module, index }) => {
                let data = self.modules.iter().find(|m| m.module.name == *module);
                let i = *index as usize;
                Some((
                    data.and_then(|m| m.textures.get(i)),
                    data.and_then(|m| m.normal_maps.get(i)?.as_ref()),
                ))
            }
            _ => Some((
                self.base_textures.get(index).and_then(Option::as_ref),
                self.base_normal_maps.get(index).and_then(Option::as_ref),
            )),
        }
    }

    /// 合成したモデルのテクスチャの数。
    pub(crate) fn texture_count(&self) -> usize {
        self.base_textures.len().max(self.texture_sources.len())
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

/// 本体の描画に使うテクスチャの枚数。`model.textures` と、メッシュが参照する番号の大きい方。
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
    error: *mut UgoeError,
    make: impl FnOnce() -> Result<UgoePuppet, Error>,
) -> UgoeResult {
    guard(error, || {
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
/// 既定の読み込みの上限で読み、超えたら `UGOE_ERROR_LIMIT_EXCEEDED` (`ugoe_puppet_create_from_bundle_with_limits` を参照)。
/// 読めなければ `UGOE_ERROR_FORMAT`、モデルを評価できなければ `UGOE_ERROR_INVALID_MODEL` で、種類 (`ugoe_error_kind`) と
/// 補足の値は wasm の `LoadError` と同じ (crates/ugoe-ffi/README.md の「モデルの読み込みの失敗」。T0457)。
///
/// # Safety
///
/// `bytes` は `len` バイトの読める領域を指すこと。`out` は `UgoePuppet*` を書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_create_from_bundle(
    bytes: *const u8,
    len: usize,
    out: *mut *mut UgoePuppet,
    error: *mut UgoeError,
) -> UgoeResult {
    // SAFETY: 呼び出し側の約束。
    unsafe { ugoe_puppet_create_from_bundle_with_limits(bytes, len, std::ptr::null(), out, error) }
}

/// 読み込みの上限 `limits` を指定して、`.ugoe` バンドルのバイト列から `UgoePuppet` を作る (T0152)。
///
/// `limits` が NULL なら既定値 (`ugoe_puppet_create_from_bundle` と同じ)。展開したバイトが上限を超えそうになったら
/// 展開の途中でやめて `UGOE_ERROR_LIMIT_EXCEEDED` を返す (種類は `"read_limit_exceeded"`、補足の値は `"limit"`・`"chunk"`・`"max"`)。
/// ほかは `ugoe_puppet_create_from_bundle` と同じ。
///
/// # Safety
///
/// `bytes` は `len` バイトの読める領域を指すこと。`limits` は NULL か `UgoeReadLimits` を読める領域を指すこと。
/// `out` は `UgoePuppet*` を書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_create_from_bundle_with_limits(
    bytes: *const u8,
    len: usize,
    limits: *const UgoeReadLimits,
    out: *mut *mut UgoePuppet,
    error: *mut UgoeError,
) -> UgoeResult {
    // SAFETY: 呼び出し側の約束。
    unsafe {
        create(out, error, || {
            let limits = read_limits(limits);
            let (bundle, report) =
                ugoe::format::read_bundle_with(self::bytes(bytes, len, "bytes")?, limits)
                    .map_err(|e| load_error(e.into()))?;
            UgoePuppet::from_bundle(bundle, report)
        })
    }
}

/// モデルの JSON (UTF-8、`len` バイト。NUL 終端は要らない) から `UgoePuppet` を作る。
///
/// JSON はテクスチャを持たないので、テクスチャはすべて「画像なし」になる。失敗の種類は `ugoe_puppet_create_from_bundle` と同じ
/// (JSON は読むときに検査しないので、検査を通らないモデルは `UGOE_ERROR_INVALID_MODEL` で種類が `"invalid_model"`)。
/// UTF-8 でなければ `UGOE_ERROR_INVALID_ARGUMENT` (種類なし)。
///
/// # Safety
///
/// `json` は `len` バイトの読める領域を指すこと。`out` は `UgoePuppet*` を書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_create_from_json(
    json: *const u8,
    len: usize,
    out: *mut *mut UgoePuppet,
    error: *mut UgoeError,
) -> UgoeResult {
    // SAFETY: 呼び出し側の約束。
    unsafe {
        create(out, error, || {
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
    guard(std::ptr::null_mut(), || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
/// 呼ぶと、この `UgoePuppet` から借りた頂点位置と描画順とモーションのイベントのポインタは無効になる。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_update(
    puppet: *mut UgoePuppet,
    dt: f32,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        if !dt.is_finite() || dt < 0.0 {
            return Err(Error::new(
                UGOE_ERROR_INVALID_ARGUMENT,
                format!("`dt` must be a finite non-negative number (got {dt})"),
            ));
        }
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        p.puppet.update(dt);
        p.motion_events.refresh(&p.puppet);
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.parameter_ids.len() as u32) }
    })
}

/// `index` 番目のパラメータの ID (UTF-8、NUL 終端)。ポインタは、着せ替えの操作か `UgoePuppet` の破棄まで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_parameter_id(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut *const c_char,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
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

/// 使用中の表情をすべてフェードアウトさせる (表情の解除)。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_stop_expressions(
    puppet: *mut UgoePuppet,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        p.puppet.expressions_mut().stop_all();
        Ok(())
    })
}

/// ID (UTF-8、NUL 終端) で複数のパラメータに値を設定する (範囲に丸める)。`ids` と `values` は `count` 個ずつ。
/// 知らない ID は飛ばし、設定した数を `*out_applied` に書く (`out_applied` は NULL でもよい)。
/// 値に NaN があるか ID が UTF-8 でなければ、何も設定せずにエラーを返す。
/// 設定した値の扱いは `ugoe_puppet_set_parameter` と同じ。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`ids` は `count` 個の NUL 終端の文字列へのポインタの並び、
/// `values` は `count` 個の float を読める領域、`out_applied` は NULL か書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_parameters_by_id(
    puppet: *mut UgoePuppet,
    ids: *const *const c_char,
    values: *const f32,
    count: usize,
    out_applied: *mut usize,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        let (ids, values): (&[*const c_char], &[f32]) = if count == 0 {
            (&[], &[])
        } else if ids.is_null() {
            return Err(Error::null("ids"));
        } else if values.is_null() {
            return Err(Error::null("values"));
        } else {
            // SAFETY: 呼び出し側の約束で、どちらも `count` 個を読める。
            unsafe {
                (
                    std::slice::from_raw_parts(ids, count),
                    std::slice::from_raw_parts(values, count),
                )
            }
        };
        let parameters = p.puppet.instance().parameters();
        let mut targets = Vec::with_capacity(count);
        for (i, (&id, &value)) in ids.iter().zip(values).enumerate() {
            // `c_str` と同じ検査。何番目かはエラーのときだけ文字列にする (成功する呼び出しで確保しない)。
            if id.is_null() {
                return Err(Error::null(&format!("ids[{i}]")));
            }
            // SAFETY: 呼び出し側の約束で、NULL でない `id` は NUL 終端の文字列。
            let id = unsafe { CStr::from_ptr(id) }.to_str().map_err(|_| {
                Error::new(
                    UGOE_ERROR_INVALID_ARGUMENT,
                    format!("`ids[{i}]` is not UTF-8"),
                )
            })?;
            if value.is_nan() {
                return Err(Error::new(
                    UGOE_ERROR_INVALID_ARGUMENT,
                    format!("`values[{i}]` is NaN"),
                ));
            }
            if let Some(index) = parameters.index_of(id) {
                targets.push((index, value));
            }
        }
        let parameters = p.puppet.instance_mut().parameters_mut();
        for &(index, value) in &targets {
            parameters.set(index, value);
        }
        if !out_applied.is_null() {
            // SAFETY: 呼び出し側の約束。
            unsafe { out_applied.write(targets.len()) };
        }
        Ok(())
    })
}

/// `ugoe_puppet_update` の段: 物理。
pub const UGOE_STAGE_PHYSICS: u32 = 0;
/// 目パチ。
pub const UGOE_STAGE_EYE_BLINK: u32 = 1;
/// 呼吸。
pub const UGOE_STAGE_BREATH: u32 = 2;
/// リップシンク。
pub const UGOE_STAGE_LIP_SYNC: u32 = 3;
/// ポーズ。
pub const UGOE_STAGE_POSE: u32 = 4;

/// `ugoe_puppet_update` の段 `stage` (`UGOE_STAGE_*`) を動かすか (既定はすべて動かす)。
/// 止めた段はパラメータ・パーツの不透明度に触らない (`docs/spec/motion.md#段の有効無効`)。
/// 例: トラッキング中はモデルの目パチを止める。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet` を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_set_stage_enabled(
    puppet: *mut UgoePuppet,
    stage: u32,
    enabled: bool,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_mut(puppet)? };
        let mut toggles = p.puppet.toggles();
        let flag = match stage {
            UGOE_STAGE_PHYSICS => &mut toggles.physics,
            UGOE_STAGE_EYE_BLINK => &mut toggles.eye_blink,
            UGOE_STAGE_BREATH => &mut toggles.breath,
            UGOE_STAGE_LIP_SYNC => &mut toggles.lip_sync,
            UGOE_STAGE_POSE => &mut toggles.pose,
            other => {
                return Err(Error::new(
                    UGOE_ERROR_INVALID_ARGUMENT,
                    format!("unknown stage {other}"),
                ));
            }
        };
        *flag = enabled;
        p.puppet.set_toggles(toggles);
        Ok(())
    })
}

/// 読み込みの警告の数 (読み込みは成功しているが、表示が劣化しうること)。JSON から作ったときは 0。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_load_warning_count(
    puppet: *const UgoePuppet,
    out: *mut u32,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write(out, "out", p.warnings.len() as u32) }
    })
}

/// `index` 番目の読み込みの警告 (`code` は wasm の `load_warnings_json` と同じ)。
/// 文字列は `UgoePuppet` を破棄するまで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_load_warning(
    puppet: *const UgoePuppet,
    index: u32,
    out: *mut UgoeWarning,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write_warning(&p.warnings, index, out) }
    })
}

/// `index` 番目の読み込みの警告の補足の値 `key` (UTF-8、NUL 終端。`"texture"`・`"feature"`・`"path"`)。
/// 数も 10 進の文字列で返す。その警告が `key` を持たなければ `UGOE_ERROR_NOT_FOUND`。
/// 文字列は `UgoePuppet` を破棄するまで有効。
///
/// # Safety
///
/// `puppet` は有効な `UgoePuppet`、`key` は NUL 終端の文字列、`out` は書き込める領域を指すこと。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ugoe_puppet_load_warning_field(
    puppet: *const UgoePuppet,
    index: u32,
    key: *const c_char,
    out: *mut *const c_char,
    error: *mut UgoeError,
) -> UgoeResult {
    guard(error, || {
        // SAFETY: 呼び出し側の約束。
        let p = unsafe { puppet_ref(puppet)? };
        // SAFETY: 呼び出し側の約束。
        unsafe { write_warning_field(&p.warnings, index, key, out) }
    })
}
