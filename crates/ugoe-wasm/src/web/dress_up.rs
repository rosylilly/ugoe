//! `WebPuppet` の着せ替えの API (`docs/spec/modules.md#puppet-での着せ替え`)。
//!
//! `.ugoemod` を読み込んで名前で持ち、付け外しは `ugoe::Puppet` に任せる。テクスチャと法線マップの
//! 登録し直しと、使わなくなった画像を捨てることは [`crate::dress_up::Wardrobe`] が行う。
//! 失敗は `name` が `"DressUpError"` の `Error` で、`code` (と `module`・`slot` などの補足) を持つ
//! ([`crate::dress_up::error_info`])。

use ugoe::render::{RenderError, Renderer};
use wasm_bindgen::prelude::*;

use super::WebPuppet;
use crate::dress_up::{self, TextureSink, WardrobeError, attached_names};
use crate::load::Rgba8;

/// レンダラへの登録 ([`TextureSink`])。
pub(super) struct RendererSink<'a> {
    pub renderer: &'a mut Renderer,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
}

impl TextureSink for RendererSink<'_> {
    type Error = RenderError;

    fn set_texture(&mut self, index: u32, image: &Rgba8) -> Result<(), RenderError> {
        self.renderer.set_texture(
            self.device,
            self.queue,
            index,
            image.width,
            image.height,
            &image.pixels,
        )
    }

    fn set_normal_map(&mut self, index: u32, image: Option<&Rgba8>) -> Result<(), RenderError> {
        match image {
            Some(t) => self.renderer.set_normal_map(
                self.device,
                self.queue,
                index,
                t.width,
                t.height,
                &t.pixels,
            ),
            None => {
                self.renderer.remove_normal_map(self.device, index);
                Ok(())
            }
        }
    }
}

/// `WardrobeError` を JS の `Error` にする (`name` = `"DressUpError"`、`code` と補足の値を持つ)。
fn dress_up_error(error: WardrobeError) -> JsValue {
    let info = dress_up::error_info(&error);
    super::coded_error("DressUpError", &error.to_string(), info)
}

/// `WebPuppet` のフィールドを分けて借り、付け外しを 1 つ行う。
macro_rules! with_wardrobe {
    ($self:ident, |$wardrobe:ident, $puppet:ident, $sink:ident| $body:expr) => {{
        if $self.is_lost() {
            return Err(JsError::new("the GPU device was lost").into());
        }
        let $wardrobe = &mut $self.wardrobe;
        let $puppet = &mut $self.puppet;
        let mut $sink = RendererSink {
            renderer: &mut $self.renderer,
            device: &$self.device,
            queue: &$self.queue,
        };
        let $sink = &mut $sink;
        $body.map_err(dress_up_error)
    }};
}

#[wasm_bindgen]
impl WebPuppet {
    /// `.ugoemod` のバイト列を読み込み (テクスチャをデコードする)、モジュールの名前を返す。付けはしない。
    ///
    /// 同じ名前のモジュールは置き換える。そのモジュールを付けていれば新しい版に付け替える
    /// (付け替えられなければ何も変えずにエラー)。モデルが無くても読み込める。
    /// `limits_json` は `load_bundle` と同じ読み込みの上限 (T0152)。超えたら `DressUpError` で、`code` は
    /// `"read_limit_exceeded"` か `"image_limit_exceeded"`。
    pub fn load_module(
        &mut self,
        bytes: &[u8],
        limits_json: Option<String>,
    ) -> Result<String, JsValue> {
        let limits = super::load_limits(limits_json)?;
        let loaded =
            dress_up::from_module_file_with(bytes, limits).map_err(|e| dress_up_error(e.into()))?;
        self.check_texture_sizes(&loaded.textures, &loaded.normal_maps)
            .map_err(|e| dress_up_error(WardrobeError::Texture(e.to_string())))?;
        let name = loaded.module.name.clone();
        with_wardrobe!(self, |wardrobe, puppet, sink| wardrobe.load(
            puppet.as_mut(),
            loaded,
            sink
        ))?;
        Ok(name)
    }

    /// 読み込んだモジュール `name` を付ける。同じスロットか同じ名前のモジュールが付いていれば差し替える。
    /// 失敗したとき (スロットが無い・要件を満たさないなど) は何も変えない。
    pub fn attach_module(&mut self, name: &str) -> Result<(), JsValue> {
        with_wardrobe!(self, |wardrobe, puppet, sink| match puppet {
            Some(puppet) => wardrobe.attach(puppet, name, sink),
            None => Err(WardrobeError::NoModel),
        })
    }

    /// モジュール `name` を外す。付いていなければエラー (`code` = `"not_attached"`)。
    pub fn detach_module(&mut self, name: &str) -> Result<(), JsValue> {
        with_wardrobe!(self, |wardrobe, puppet, sink| match puppet {
            Some(puppet) => wardrobe.detach(puppet, name, sink),
            None => Err(WardrobeError::NoModel),
        })
    }

    /// 付けるモジュールの組を、読み込んだモジュールの名前 `names` (合成の順) にする。空なら本体だけに戻す。
    pub fn set_modules(&mut self, names: Vec<String>) -> Result<(), JsValue> {
        with_wardrobe!(self, |wardrobe, puppet, sink| match puppet {
            Some(puppet) => wardrobe.set_modules(puppet, &names, sink),
            None => Err(WardrobeError::NoModel),
        })
    }

    /// 読み込んだモジュール `name` を捨てる。付けていれば外す。
    pub fn unload_module(&mut self, name: &str) -> Result<(), JsValue> {
        with_wardrobe!(self, |wardrobe, puppet, sink| wardrobe.unload(
            puppet.as_mut(),
            name,
            sink
        ))
    }

    /// 付けているモジュールの名前 (合成の順)。
    pub fn attached_modules(&self) -> Vec<String> {
        attached_names(self.puppet.as_ref())
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// 読み込んだモジュールの一覧の JSON (読み込んだ順):
    /// `[{"name", "slot", "requires": {"avatar_level"?, "slots"?, "parameters"?}, "attached", "textures", "warnings"}]`。
    pub fn modules_json(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.wardrobe.infos(self.puppet.as_ref()))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// 直前の `load_json` / `load_bundle` で付け直せずに外したモジュールの JSON (付けていた順):
    /// `[{"module", "slot", "error": {"code", "message", "module"?, "slot"?, "parameter"?, "mesh"?, "attached"?}, "warning"}]`。
    /// `error` は付けるときの `DressUpError` と同じ `code` と補足の値 (と英語の文)。外したものが無ければ `[]`。
    /// 読み込みの戻り値の文の警告 (`module `X` was detached: ...`) と同じものを、文に頼らずに読むためのもの。
    /// `warning` はその文の警告と同じ文字列で、戻り値の一覧から除くときに比べる。
    pub fn detached_modules_json(&self) -> Result<String, JsError> {
        let detached: Vec<_> = self
            .wardrobe
            .detached()
            .iter()
            .map(dress_up::Detached::to_json)
            .collect();
        serde_json::to_string(&detached).map_err(|e| JsError::new(&e.to_string()))
    }

    /// 本体のモデルのスロットの JSON (モジュールを付けていても本体のもの)。各要素はモデルの JSON 形式と同じ
    /// (`id`・`anchor`・`draw_order`・`replaces`・`parameters`・`mask_sources`・`part`・`colliders`・`pose_parts`)。
    /// モデルが無ければ `[]`。
    pub fn slots_json(&self) -> Result<String, JsError> {
        match &self.puppet {
            Some(puppet) => serde_json::to_string(dress_up::slots(puppet))
                .map_err(|e| JsError::new(&e.to_string())),
            None => Ok("[]".to_owned()),
        }
    }

    /// 本体のアバタープロファイルがアプリに公開するスロット (`avatar.slots`) の ID の JSON 配列
    /// (モジュールを付けていても本体のもの)。本体にアバタープロファイルが無いか、モデルが無ければ `null`。
    /// 公開していないスロットは作者用で、利用者に見せないことが推奨される (`docs/spec/avatar-profile.md#アプリ向けの情報`)。
    pub fn avatar_slots_json(&self) -> Result<String, JsError> {
        let slots = self.puppet.as_ref().and_then(dress_up::published_slots);
        serde_json::to_string(&slots).map_err(|e| JsError::new(&e.to_string()))
    }
}
