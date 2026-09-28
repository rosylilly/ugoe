//! ugoe の参照レンダラ (wgpu)。仕様: `docs/spec/rendering.md`。
//!
//! 設計原則:
//! - **デバイスやサーフェスを所有しない。** ホスト (エディタ・ゲームエンジン・ビューア) が
//!   作った `wgpu::Device` / `Queue` と描画先 `TextureView` を受け取る。
//! - 入力は [`ugoe_runtime::ModelInstance`] の評価結果のみ。モデルの意味論には踏み込まない。
//! - ネイティブ (Vulkan/Metal/DX12/GL) と Web (WebGPU/WebGL2) の双方で動くこと。
//!   WebGL2 で使えない機能 (storage buffer・compute 等) には必ずフォールバックを用意する。

use ugoe_runtime::ModelInstance;

/// 参照シェーダの WGSL ソース。
pub const DRAWABLE_WGSL: &str = include_str!("shaders/drawable.wgsl");

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("texture index {0} is not uploaded")]
    MissingTexture(u32),
}

/// キャンバス座標から描画先への変換。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// キャンバス座標 → クリップ座標 (列優先 3x3)。
    pub view_proj: [[f32; 3]; 3],
}

/// モデル描画器。モデル (テクスチャ集合) 1 つにつき 1 つ作る。
#[derive(Debug)]
pub struct Renderer {
    target_format: wgpu::TextureFormat,
}

impl Renderer {
    pub fn new(_device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        // TODO(T0201): パイプライン (Normal / Additive / Multiply × マスク有無) の生成。
        Self { target_format }
    }

    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
    }

    /// 乗算済みアルファの RGBA8 テクスチャを登録する。
    pub fn set_texture(
        &mut self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _index: u32,
        _width: u32,
        _height: u32,
        _rgba_premultiplied: &[u8],
    ) {
        // TODO(T0201)
    }

    /// 現在のインスタンス状態を描画するコマンドを `encoder` に積む。
    /// マスク描画パスと本描画パスを内部で作る。
    pub fn encode(
        &mut self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
        _target: &wgpu::TextureView,
        _view: &View,
        _instance: &ModelInstance,
    ) -> Result<(), RenderError> {
        // TODO(T0201) / TODO(T0202): 頂点アップロード・マスクアトラス・描画。
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reference_shader_is_valid_wgsl() {
        let module = naga::front::wgsl::parse_str(super::DRAWABLE_WGSL).expect("parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("validate");
    }
}
