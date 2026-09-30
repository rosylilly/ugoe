//! 着せ替え: `.ugoemod` の読み込みと、付け外しに合わせたテクスチャの登録。GPU には触れない
//! (`docs/spec/modules.md#puppet-での着せ替え`。JS への公開は `src/web/dress_up.rs`)。
//!
//! 読み込んだモジュールは [`Wardrobe`] が名前ごとに持ち (画像はデコード済み)、付け外しは
//! `ugoe::Puppet` の `attach_module` / `detach_module` / `set_modules` に任せる。その結果の
//! [`DressUpChange`] に従って、出どころが変わった番号のテクスチャと法線マップを登録し直し、
//! 使われなくなった番号の画像を 1x1 のプレースホルダに置き換えて捨てる。

use std::sync::Arc;

use serde::Serialize;
use ugoe::format::module_file;
use ugoe::model::module::{Module, ModuleRequirements, Slot};
use ugoe::{DressUpChange, DressUpError, Puppet, TextureSource};

use crate::load::{self, LoadError, LoadWarning, Rgba8};

/// 読み込んだ `.ugoemod` 1 つ分。
#[derive(Debug)]
pub struct LoadedModule {
    pub module: Arc<Module>,
    /// `Module::textures` の順の画像 (乗算済みアルファ)。
    pub textures: Vec<Rgba8>,
    /// 法線マップ (添字 = `Module::textures` の番号。アルファを掛けない)。`None` の番号は法線マップなし。
    pub normal_maps: Vec<Option<Rgba8>>,
    /// 読み込めたが注意が要ること (プレースホルダで代用したテクスチャなど)。
    pub warnings: Vec<LoadWarning>,
}

/// `.ugoemod` を読み、テクスチャと法線マップをデコードする。
/// 本体との整合 (スロット・パラメータ) は付けるときに合成が検査する。
pub fn from_module_file(bytes: &[u8]) -> Result<LoadedModule, LoadError> {
    let (file, report) = module_file::read_module_with_report(bytes)?;
    let count = file.module.textures.len();
    // 読み込みの報告の警告を先に並べる (C ABI の `ugoe_module_warning_*` と同じ順)。
    let mut warnings: Vec<LoadWarning> = report.warnings.iter().map(LoadWarning::from).collect();
    let normal_maps =
        load::decode_normal_maps((0..count).map(|i| file.normal_map(i)), &mut warnings)?;
    let textures = load::decode_textures(count, |i| file.textures.get(i), &mut warnings)?;
    Ok(LoadedModule {
        module: Arc::new(file.module),
        textures,
        normal_maps,
        warnings,
    })
}

/// テクスチャと法線マップの登録先 (wasm では `ugoe::render::Renderer`。テストでは記録するだけ)。
pub trait TextureSink {
    type Error: std::fmt::Display;
    /// テクスチャ `index` を `image` (乗算済みアルファ) にする。
    fn set_texture(&mut self, index: u32, image: &Rgba8) -> Result<(), Self::Error>;
    /// テクスチャ `index` の法線マップを `image` にする。`None` なら外す。
    fn set_normal_map(&mut self, index: u32, image: Option<&Rgba8>) -> Result<(), Self::Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum WardrobeError {
    #[error(transparent)]
    Load(#[from] LoadError),
    #[error("no model is loaded")]
    NoModel,
    #[error("module `{0}` is not loaded")]
    NotLoaded(String),
    #[error(transparent)]
    DressUp(#[from] DressUpError),
    /// レンダラへの登録に失敗した (付け外し自体は済んでいる)。
    #[error("registering the textures failed: {0}")]
    Texture(String),
}

/// 読み込んだモジュールの一覧 (`modules_json`) の要素。
#[derive(Debug, Serialize)]
pub struct ModuleInfo<'a> {
    pub name: &'a str,
    pub slot: &'a str,
    pub requires: &'a ModuleRequirements,
    /// 今の Puppet に付けているか。
    pub attached: bool,
    pub textures: usize,
    /// 読み込めたが注意が要ること (英語の文。T0123 から変えていない)。
    pub warnings: Vec<String>,
    /// `warnings` と同じ順の構造 (`load::warning_json`。T0125)。
    pub warning_details: Vec<serde_json::Value>,
}

/// モデルを読み直したときに付け直せず、外したモジュール (`detached_modules_json` の要素)。
#[derive(Debug)]
pub struct Detached {
    /// モジュールの名前。
    pub module: String,
    /// モジュールを付けるスロット。
    pub slot: String,
    /// 付けられなかった理由 (付けようとしたときの合成のエラー)。
    pub error: WardrobeError,
}

impl Detached {
    /// 文の警告 (`load_json` / `load_bundle` の戻り値。T0123 から変えていない)。
    pub fn warning(&self) -> String {
        format!("module `{}` was detached: {}", self.module, self.error)
    }

    /// JSON: `{"code": "module_detached", "module", "slot", "error": {"code", "message", "module"?, "slot"?, ...}, "warning"}`。
    /// `error` は JS の `DressUpError` と同じ `code` と補足の値に、英語の文 `message` を添えたもの。
    /// `warning` は読み込みの戻り値に入る文の警告と同じ文字列 (ホストが一覧から除けるように)。
    /// `code` は読み込みのほかの警告 ([`load::warning_json`]) と同じ一覧に並べるためのもの (T0125)。
    pub fn to_json(&self) -> serde_json::Value {
        let info = error_info(&self.error);
        let mut error = serde_json::Map::new();
        error.insert("code".into(), info.code.into());
        error.insert("message".into(), self.error.to_string().into());
        for (key, value) in info.fields {
            error.insert(key.into(), value.into());
        }
        serde_json::json!({
            "code": "module_detached",
            "module": self.module,
            "slot": self.slot,
            "error": error,
            "warning": self.warning(),
        })
    }
}

/// 読み込んだモジュールと、レンダラに登録したテクスチャの番号の範囲。
#[derive(Debug, Default)]
pub struct Wardrobe {
    /// 読み込んだモジュール (読み込んだ順)。名前は一意。
    library: Vec<LoadedModule>,
    /// 本体のテクスチャの番号の数 (`load::texture_count`)。
    base_slots: u32,
    /// テクスチャ (と法線マップ) を登録したかもしれない番号の数。これ以上の番号には何も登録していない。
    slots: u32,
    /// 直前の [`Wardrobe::load_base`] で付け直せずに外したモジュール (付けていた順)。
    detached: Vec<Detached>,
}

impl Wardrobe {
    /// 読み込んだモジュール (読み込んだ順)。
    pub fn library(&self) -> &[LoadedModule] {
        &self.library
    }

    fn find(&self, name: &str) -> Result<&LoadedModule, WardrobeError> {
        self.library
            .iter()
            .find(|m| m.module.name == name)
            .ok_or_else(|| WardrobeError::NotLoaded(name.to_owned()))
    }

    /// 直前の [`Wardrobe::load_base`] (モデルの読み直し) で付け直せずに外したモジュール (付けていた順)。
    pub fn detached(&self) -> &[Detached] {
        &self.detached
    }

    /// 読み込んだモジュールの一覧。`puppet` に付けているかを添える。
    pub fn infos<'a>(&'a self, puppet: Option<&Puppet>) -> Vec<ModuleInfo<'a>> {
        let attached = attached_names(puppet);
        self.library
            .iter()
            .map(|m| ModuleInfo {
                name: &m.module.name,
                slot: m.module.slot.as_str(),
                requires: &m.module.requires,
                attached: attached.iter().any(|n| *n == m.module.name),
                textures: m.textures.len(),
                warnings: m.warnings.iter().map(ToString::to_string).collect(),
                warning_details: m.warnings.iter().map(load::warning_json).collect(),
            })
            .collect()
    }

    /// 本体のモデルを読み込んだ直後に呼ぶ。本体のテクスチャと法線マップを登録し、`attached`
    /// (前のモデルに付けていたモジュールの名前、合成の順) を付け直す。付けられないモジュールは外して警告を返す
    /// (外したものは [`Wardrobe::detached`] でも読める)。
    /// 前のモデルやモジュールの画像で、もう使わない番号のものは捨てる。
    pub fn load_base<S: TextureSink>(
        &mut self,
        puppet: &mut Puppet,
        attached: &[String],
        textures: &[Rgba8],
        normal_maps: &[Option<Rgba8>],
        sink: &mut S,
    ) -> Result<Vec<String>, WardrobeError> {
        for (i, texture) in textures.iter().enumerate() {
            sink.set_texture(i as u32, texture).map_err(texture_error)?;
        }
        for i in 0..textures.len() {
            let normal_map = normal_maps.get(i).and_then(Option::as_ref);
            sink.set_normal_map(i as u32, normal_map)
                .map_err(texture_error)?;
        }
        self.base_slots = textures.len() as u32;

        let mut detached = Vec::new();
        let modules: Vec<Arc<Module>> = attached
            .iter()
            .filter_map(|name| self.find(name).ok())
            .map(|m| Arc::clone(&m.module))
            .collect();
        if !modules.is_empty() && puppet.set_modules(modules.clone()).is_err() {
            // まとめて付けられなければ 1 つずつ試し、付けられないものだけを外す。
            let mut accepted = Vec::new();
            for module in modules {
                let mut next = accepted.clone();
                next.push(Arc::clone(&module));
                match puppet.set_modules(next) {
                    Ok(_) => accepted.push(module),
                    Err(e) => detached.push(Detached {
                        module: module.name.clone(),
                        slot: module.slot.as_str().to_owned(),
                        error: e.into(),
                    }),
                }
            }
        }
        // 新しい Puppet なので、モジュールのテクスチャはすべて登録し直す。
        let sources = puppet.texture_sources();
        let change = DressUpChange {
            textures: sources
                .iter()
                .enumerate()
                .filter(|(_, s)| matches!(s, TextureSource::Module { .. }))
                .map(|(i, _)| i as u32)
                .collect(),
            texture_count: sources.len() as u32,
        };
        let warnings = detached.iter().map(Detached::warning).collect();
        self.detached = detached;
        self.apply(&sources, &change, sink)?;
        Ok(warnings)
    }

    /// `.ugoemod` を読み込んだモジュールに加える。同じ名前のモジュールは置き換え、`puppet` に付いていれば
    /// 新しい版に付け替える (付け替えに失敗したら何も変えない)。
    pub fn load<S: TextureSink>(
        &mut self,
        puppet: Option<&mut Puppet>,
        loaded: LoadedModule,
        sink: &mut S,
    ) -> Result<(), WardrobeError> {
        let name = loaded.module.name.clone();
        let change = match puppet {
            Some(puppet) if attached_names(Some(puppet)).contains(&name.as_str()) => {
                let change = puppet.attach_module(Arc::clone(&loaded.module))?;
                Some((puppet.texture_sources(), change))
            }
            _ => None,
        };
        match self.library.iter_mut().find(|m| m.module.name == name) {
            Some(slot) => *slot = loaded,
            None => self.library.push(loaded),
        }
        match change {
            Some((sources, change)) => self.apply(&sources, &change, sink),
            None => Ok(()),
        }
    }

    /// 読み込んだモジュール `name` を付ける。同じスロットか同じ名前のモジュールが付いていれば差し替える
    /// (`Puppet::attach_module`)。失敗したときは何も変えない。
    pub fn attach<S: TextureSink>(
        &mut self,
        puppet: &mut Puppet,
        name: &str,
        sink: &mut S,
    ) -> Result<(), WardrobeError> {
        let module = Arc::clone(&self.find(name)?.module);
        let change = puppet.attach_module(module)?;
        self.apply(&puppet.texture_sources(), &change, sink)
    }

    /// モジュール `name` を外す。付いていなければ `DressUpError::NotAttached`。
    pub fn detach<S: TextureSink>(
        &mut self,
        puppet: &mut Puppet,
        name: &str,
        sink: &mut S,
    ) -> Result<(), WardrobeError> {
        let change = puppet.detach_module(name)?;
        self.apply(&puppet.texture_sources(), &change, sink)
    }

    /// 付けるモジュールの組を、読み込んだモジュールの名前 `names` (合成の順) にする。空なら本体だけに戻す。
    pub fn set_modules<S: TextureSink>(
        &mut self,
        puppet: &mut Puppet,
        names: &[String],
        sink: &mut S,
    ) -> Result<(), WardrobeError> {
        let modules = names
            .iter()
            .map(|name| self.find(name).map(|m| Arc::clone(&m.module)))
            .collect::<Result<_, _>>()?;
        let change = puppet.set_modules(modules)?;
        self.apply(&puppet.texture_sources(), &change, sink)
    }

    /// 読み込んだモジュール `name` を捨てる。付いていれば先に外す。
    pub fn unload<S: TextureSink>(
        &mut self,
        puppet: Option<&mut Puppet>,
        name: &str,
        sink: &mut S,
    ) -> Result<(), WardrobeError> {
        self.find(name)?;
        if let Some(puppet) = puppet
            && attached_names(Some(puppet)).contains(&name)
        {
            self.detach(puppet, name, sink)?;
        }
        self.library.retain(|m| m.module.name != name);
        Ok(())
    }

    /// 付け外しの結果 `change` に従ってテクスチャを登録し直し、使われなくなった番号の画像を捨てる。
    fn apply<S: TextureSink>(
        &mut self,
        sources: &[TextureSource],
        change: &DressUpChange,
        sink: &mut S,
    ) -> Result<(), WardrobeError> {
        for &i in &change.textures {
            // 本体のテクスチャは番号が変わらないので、登録し直すのはモジュールのものだけ。
            let Some(TextureSource::Module { module, index }) = sources.get(i as usize) else {
                continue;
            };
            let loaded = self.find(module)?;
            let index = *index as usize;
            let placeholder = Rgba8::placeholder();
            let texture = loaded.textures.get(index).unwrap_or(&placeholder);
            let normal_map = loaded.normal_maps.get(index).and_then(Option::as_ref);
            sink.set_texture(i, texture).map_err(texture_error)?;
            sink.set_normal_map(i, normal_map).map_err(texture_error)?;
        }
        let count = change.texture_count.max(self.base_slots);
        if count < self.slots {
            let placeholder = Rgba8::placeholder();
            for i in count..self.slots {
                sink.set_texture(i, &placeholder).map_err(texture_error)?;
                sink.set_normal_map(i, None).map_err(texture_error)?;
            }
        }
        self.slots = count;
        Ok(())
    }
}

/// `puppet` に付けているモジュールの名前 (合成の順)。
pub fn attached_names(puppet: Option<&Puppet>) -> Vec<&str> {
    puppet.map_or_else(Vec::new, |p| {
        p.modules().iter().map(|m| m.name.as_str()).collect()
    })
}

/// 本体のモデルのスロット (モジュールを付けていても本体のもの)。
pub fn slots(puppet: &Puppet) -> &[Slot] {
    &puppet.base_model().slots
}

/// 本体のアバタープロファイルがアプリに公開するスロットの ID (`avatar.slots`)。プロファイルが無ければ `None`
/// (公開の区別が無い)。`docs/spec/avatar-profile.md#アプリ向けの情報`
pub fn published_slots(puppet: &Puppet) -> Option<&[String]> {
    puppet
        .base_model()
        .avatar
        .as_ref()
        .map(|a| a.slots.as_slice())
}

fn texture_error(e: impl std::fmt::Display) -> WardrobeError {
    WardrobeError::Texture(e.to_string())
}

/// JS に渡すエラーの中身: 機械が読む `code` と、補足の値 (`module`・`slot` など)。
/// 文は `WardrobeError` の `Display` (英語)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorInfo {
    pub code: &'static str,
    pub fields: Vec<(&'static str, String)>,
}

/// エラーの種類を JS で分けられるように、コードと補足の値にする (`docs/spec/modules.md#puppet-での着せ替え`)。
/// 付け外しの失敗 (`DressUpError`) と `invalid_file` は SDK の値 (`ugoe::DressUpError::code`・`fields`) を使い、
/// C ABI と揃える (T0412)。`no_model`・`not_loaded`・`texture` は wasm だけのもの。
pub fn error_info(error: &WardrobeError) -> ErrorInfo {
    let (code, fields): (_, Vec<(&'static str, &str)>) = match error {
        WardrobeError::Load(_) => (DressUpError::INVALID_FILE, vec![]),
        WardrobeError::NoModel => ("no_model", vec![]),
        WardrobeError::NotLoaded(name) => ("not_loaded", vec![("module", name)]),
        WardrobeError::Texture(_) => ("texture", vec![]),
        WardrobeError::DressUp(e) => (e.code(), e.fields()),
    };
    ErrorInfo {
        code,
        fields: fields.into_iter().map(|(k, v)| (k, v.to_owned())).collect(),
    }
}

#[cfg(test)]
mod tests;
