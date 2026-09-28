//! PSD 取り込み。仕様: `docs/ai-integration.md#psd-取り込み`。
//!
//! 出力は PSD 形式から独立した [`LayerTree`] で、エディタ・AI パイプラインはこれだけを見る。
//! PSD パーサの実装 (外部クレート採用か自前か) はこのクレート内に閉じ込めること。

use ugoe_core::Vec2;

#[derive(Debug, thiserror::Error)]
pub enum PsdError {
    #[error("invalid psd: {0}")]
    Invalid(String),
    #[error("unsupported psd feature: {0}")]
    Unsupported(String),
}

/// 取り込んだドキュメント。
#[derive(Debug, Clone)]
pub struct LayerTree {
    pub width: u32,
    pub height: u32,
    pub root: Vec<LayerNode>,
}

#[derive(Debug, Clone)]
pub enum LayerNode {
    Group {
        name: String,
        visible: bool,
        children: Vec<LayerNode>,
    },
    Layer(Layer),
}

/// ラスタライズ済みレイヤー。画像はレイヤーの外接矩形で切り出した非乗算 RGBA8。
#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    /// キャンバス上の左上位置。
    pub offset: Vec2,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// PSD のブレンドモード名 (正規化前)。
    pub blend_mode: String,
}

/// PSD バイト列を読み込む。
pub fn import(_bytes: &[u8]) -> Result<LayerTree, PsdError> {
    // TODO(T0501)
    Err(PsdError::Unsupported("not implemented yet".into()))
}
