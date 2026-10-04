//! PSD 取り込み。仕様: `docs/ai-integration.md#psd-取り込み`。
//!
//! 出力は PSD 形式から独立した [`LayerTree`] で、エディタ・AI パイプラインはこれだけを見る。
//! PSD パーサは自前の小さな実装 (`parse` モジュール) で、このクレートの外には出さない。
//! 採用の理由は `README.md` を参照。

use std::fmt;

use ugoe_core::Vec2;

mod parse;

use parse::{Mask, RawLayer, Section};

/// 取り込みのエラー。
#[derive(Debug, thiserror::Error)]
pub enum PsdError {
    /// 壊れた入力・途中で切れた入力。
    #[error("invalid psd: {0}")]
    Invalid(String),
    /// 形式としては正しいが対応していない (PSB・8/16 ビット以外・RGB 以外・深すぎるグループ)。
    #[error("unsupported psd feature: {0}")]
    Unsupported(String),
}

/// 取り込んだドキュメント。
///
/// `root` と各グループの `children` は **下から上** (描画順) に並ぶ。
/// 先頭が最も奥で、末尾が最も手前 (Photoshop のレイヤーパネルでは一番上)。
#[derive(Debug, Clone)]
pub struct LayerTree {
    pub width: u32,
    pub height: u32,
    pub root: Vec<LayerNode>,
    /// 取り込めなかった・無視した機能。ファイル内の順 (下から上) に並ぶ。
    pub warnings: Vec<ImportWarning>,
}

#[derive(Debug, Clone)]
pub enum LayerNode {
    Group {
        name: String,
        visible: bool,
        children: Vec<LayerNode>,
        /// PSD のレイヤー ID (`lyid`)。ドキュメント内で一意。
        id: Option<u32>,
        /// グループの不透明度 (0..=1)。
        opacity: f32,
        /// PSD のブレンドモード名 (正規化前)。通過は `"pass"`。
        blend_mode: String,
        /// 直下のレイヤー (兄弟) でクリッピングされている。
        clipping: bool,
    },
    Layer(Layer),
}

/// ラスタライズ済みレイヤー。画像はレイヤーの外接矩形で切り出した非乗算 RGBA8。
#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    /// レイヤーの不透明度 (0..=1)。
    pub opacity: f32,
    /// キャンバス上の左上位置。キャンバスの外 (負の値など) にはみ出すこともある。
    pub offset: Vec2,
    /// 画像の大きさ。空のレイヤーでは 0 で、`rgba` も空になる。
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// PSD のブレンドモード名 (正規化前)。例: `"norm"` `"mul"` `"scrn"`。
    pub blend_mode: String,
    /// PSD のレイヤー ID (`lyid`)。ドキュメント内で一意。
    pub id: Option<u32>,
    /// 塗りの不透明度 (0..=1)。レイヤー効果を除いた画素にだけ掛かる。効果は取り込まないので、
    /// 実質的には `opacity` と掛け合わせて使う。
    pub fill_opacity: f32,
    /// 直下のレイヤー (兄弟) でクリッピングされている。連続するクリッピングレイヤーは、
    /// その下で最初に現れるクリッピングでないレイヤーを土台にする。
    pub clipping: bool,
}

/// 取り込みの警告。取り込みは続けるが、見た目が PSD と変わる可能性がある。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportWarning {
    /// レイヤーの名前の列 (外側のグループから順に、最後がレイヤー自身)。
    pub path: Vec<String>,
    pub kind: WarningKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarningKind {
    /// 調整レイヤーまたは塗りつぶしレイヤー。画素を持たないので取り込まない。`key` は PSD のキー。
    AdjustmentLayer { key: String },
    /// 合成済みの画素を持たないスマートオブジェクト。取り込まない。
    SmartObjectWithoutPixels,
    /// レイヤー効果 (ドロップシャドウなど)。効果を除いた画素だけを取り込む。
    LayerEffectsIgnored,
    /// レイヤーマスクを掛けられなかった (グループのマスク、マスクの画素がないなど)。
    /// マスクを掛けていない画素を取り込む。
    LayerMaskIgnored,
    /// 対応していない圧縮方式のチャンネル。取り込まない。
    UnsupportedCompression { compression: u16 },
    /// 色チャンネルの画素がない。取り込まない。
    MissingPixels,
    /// レイヤー情報がなく、合成画像を 1 枚のレイヤーとして取り込んだ。
    FlattenedImage,
}

impl fmt::Display for ImportWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.path.join(" / "))?;
        match &self.kind {
            WarningKind::AdjustmentLayer { key } => {
                write!(f, "adjustment or fill layer ({key}) skipped")
            }
            WarningKind::SmartObjectWithoutPixels => {
                write!(f, "smart object without rendered pixels skipped")
            }
            WarningKind::LayerEffectsIgnored => write!(f, "layer effects ignored"),
            WarningKind::LayerMaskIgnored => write!(f, "layer mask ignored"),
            WarningKind::UnsupportedCompression { compression } => {
                write!(
                    f,
                    "unsupported channel compression {compression}, layer skipped"
                )
            }
            WarningKind::MissingPixels => write!(f, "no color channel data, layer skipped"),
            WarningKind::FlattenedImage => {
                write!(f, "no layer data, imported the composite image")
            }
        }
    }
}

/// 対応する色空間とビット深度。これ以外は [`PsdError::Unsupported`] になる。
/// - 8 ビット・16 ビットの RGB (アルファの有無は問わない)。16 ビットは 8 ビットに丸める
///
/// レイヤーマスク (チャンネル -2 / -3) はレイヤーのアルファに掛ける。
/// グループの入れ子が 256 段より深いものも [`PsdError::Unsupported`] になる。
/// 壊れた入力や途中で切れた入力は [`PsdError::Invalid`] を返し、パニックしない。
pub fn import(bytes: &[u8]) -> Result<LayerTree, PsdError> {
    let doc = parse::parse(bytes)?;
    if let Some(rgba) = doc.composite {
        let name = "Background".to_string();
        return Ok(LayerTree {
            width: doc.width,
            height: doc.height,
            root: vec![LayerNode::Layer(Layer {
                name: name.clone(),
                visible: true,
                opacity: 1.0,
                offset: Vec2::ZERO,
                width: doc.width,
                height: doc.height,
                rgba,
                blend_mode: "norm".into(),
                id: None,
                fill_opacity: 1.0,
                clipping: false,
            })],
            warnings: vec![ImportWarning {
                path: vec![name],
                kind: WarningKind::FlattenedImage,
            }],
        });
    }

    // 下から順に読み、グループの終わりの印で枠を積み、グループ本体で枠を閉じる。
    let mut stack = vec![Frame::default()];
    for raw in doc.layers {
        match raw.section {
            Some(Section::Divider) => {
                // 木を作る・複製する・捨てる処理は入れ子の深さだけ再帰するので、深さを抑える。
                if stack.len() > MAX_GROUP_DEPTH {
                    return Err(PsdError::Unsupported(format!(
                        "groups nested deeper than {MAX_GROUP_DEPTH} levels"
                    )));
                }
                stack.push(Frame::default());
            }
            Some(Section::Folder) => {
                // 対応する終わりの印がないフォルダは、空のグループとして扱う。
                let frame = if stack.len() > 1 {
                    stack.pop().unwrap_or_default()
                } else {
                    Frame::default()
                };
                let parent = stack.last_mut().ok_or_else(unbalanced)?;
                let name = raw.name.clone();
                parent
                    .warnings
                    .extend(frame.warnings.into_iter().map(|mut w| {
                        w.path.insert(0, name.clone());
                        w
                    }));
                // グループのマスクは合成したグループに掛かるもので、子のアルファに掛けても同じに
                // ならないため、警告だけにする。
                if raw.mask.is_some() || raw.real_mask.is_some() {
                    parent.warnings.push(ImportWarning {
                        path: vec![name.clone()],
                        kind: WarningKind::LayerMaskIgnored,
                    });
                }
                parent.nodes.push(group(raw, frame.nodes));
            }
            None => {
                let frame = stack.last_mut().ok_or_else(unbalanced)?;
                let name = raw.name.clone();
                let (layer, kinds) = layer(raw);
                frame
                    .warnings
                    .extend(kinds.into_iter().map(|kind| ImportWarning {
                        path: vec![name.clone()],
                        kind,
                    }));
                if let Some(layer) = layer {
                    frame.nodes.push(LayerNode::Layer(layer));
                }
            }
        }
    }
    if stack.len() != 1 {
        return Err(unbalanced());
    }
    let root = stack.pop().unwrap_or_default();
    Ok(LayerTree {
        width: doc.width,
        height: doc.height,
        root: root.nodes,
        warnings: root.warnings,
    })
}

/// グループの入れ子の深さの上限。これより深い PSD は [`PsdError::Unsupported`] にする。
const MAX_GROUP_DEPTH: usize = 256;

#[derive(Default)]
struct Frame {
    nodes: Vec<LayerNode>,
    warnings: Vec<ImportWarning>,
}

fn unbalanced() -> PsdError {
    PsdError::Invalid("group start without a matching group end".into())
}

fn blend_name(key: [u8; 4]) -> String {
    String::from_utf8_lossy(&key).trim_end().to_string()
}

fn unit(v: u8) -> f32 {
    f32::from(v) / 255.0
}

fn group(raw: RawLayer, children: Vec<LayerNode>) -> LayerNode {
    LayerNode::Group {
        visible: !raw.hidden,
        id: raw.id,
        opacity: unit(raw.opacity),
        blend_mode: blend_name(raw.section_blend_key.unwrap_or(raw.blend_key)),
        clipping: raw.clipping,
        name: raw.name,
        children,
    }
}

/// 調整レイヤー・塗りつぶしレイヤーのキー。
const ADJUSTMENT_KEYS: &[&[u8; 4]] = &[
    b"SoCo", b"GdFl", b"PtFl", b"brit", b"levl", b"curv", b"expA", b"vibA", b"hue ", b"hue2",
    b"blnc", b"blwh", b"phfl", b"mixr", b"clrL", b"nvrt", b"post", b"thrs", b"grdm", b"selc",
];
/// スマートオブジェクトのキー。
const SMART_OBJECT_KEYS: &[&[u8; 4]] = &[b"SoLd", b"PlLd", b"SoLE"];
/// レイヤー効果のキー。
const EFFECT_KEYS: &[&[u8; 4]] = &[b"lrFX", b"lfx2", b"lfxs", b"lmfx"];

fn layer(raw: RawLayer) -> (Option<Layer>, Vec<WarningKind>) {
    let has = |keys: &[&[u8; 4]]| raw.keys.iter().find(|k| keys.contains(k)).copied();
    if let Some(key) = has(ADJUSTMENT_KEYS) {
        let key = blend_name(key);
        return (None, vec![WarningKind::AdjustmentLayer { key }]);
    }

    // 色やアルファのチャンネルが 1 つでも読めなければ、見た目が変わるので取り込まない。
    if let Some(compression) = raw.unsupported_compression {
        return (
            None,
            vec![WarningKind::UnsupportedCompression { compression }],
        );
    }
    let pixel_count = raw.width as usize * raw.height as usize;
    let mut mask_ignored = false;
    let rgba = if pixel_count == 0 {
        Some(Vec::new())
    } else {
        rgba(&raw, pixel_count).map(|mut rgba| {
            mask_ignored = !apply_masks(&raw, &mut rgba);
            rgba
        })
    };
    let Some(rgba) = rgba.filter(|p| !(p.is_empty() && has(SMART_OBJECT_KEYS).is_some())) else {
        let kind = if has(SMART_OBJECT_KEYS).is_some() {
            WarningKind::SmartObjectWithoutPixels
        } else {
            WarningKind::MissingPixels
        };
        return (None, vec![kind]);
    };

    let mut warnings = Vec::new();
    if has(EFFECT_KEYS).is_some() {
        warnings.push(WarningKind::LayerEffectsIgnored);
    }
    if mask_ignored {
        warnings.push(WarningKind::LayerMaskIgnored);
    }
    let layer = Layer {
        visible: !raw.hidden,
        opacity: unit(raw.opacity),
        offset: Vec2::new(raw.rect.left as f32, raw.rect.top as f32),
        width: raw.width,
        height: raw.height,
        rgba,
        blend_mode: blend_name(raw.blend_key),
        id: raw.id,
        fill_opacity: raw.fill_opacity.map_or(1.0, unit),
        clipping: raw.clipping,
        name: raw.name,
    };
    (Some(layer), warnings)
}

/// チャンネルを非乗算 RGBA8 に並べ直す。色チャンネルが揃っていなければ `None`。
fn rgba(raw: &RawLayer, pixel_count: usize) -> Option<Vec<u8>> {
    let plane = |id: i16| {
        raw.channels
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| c.pixels.as_deref())
            .filter(|p| p.len() == pixel_count)
    };
    let (r, g, b) = (plane(0)?, plane(1)?, plane(2)?);
    let a = plane(-1);
    let mut out = Vec::with_capacity(pixel_count * 4);
    for i in 0..pixel_count {
        out.extend_from_slice(&[r[i], g[i], b[i], a.map_or(255, |a| a[i])]);
    }
    Some(out)
}

/// レイヤーマスクをアルファに掛ける。掛けられない (マスクの画素がない) ときは false。
///
/// ベクトルマスクと併用したレイヤーでは、両方を合成した最終的なマスク (-3) だけを掛ける。
fn apply_masks(raw: &RawLayer, rgba: &mut [u8]) -> bool {
    let (id, mask) = match (&raw.real_mask, &raw.mask) {
        (Some(real), _) => (-3, real),
        (None, Some(user)) => (-2, user),
        (None, None) => return true,
    };
    let pixels = raw
        .channels
        .iter()
        .find(|c| c.id == id)
        .and_then(|c| c.pixels.as_deref())
        .filter(|p| p.len() == mask.width as usize * mask.height as usize);
    match pixels {
        Some(pixels) => {
            apply_mask(raw, mask, pixels, rgba);
            true
        }
        None => false,
    }
}

/// マスクの値 (0 = 隠す、255 = 見せる) をアルファに掛ける。矩形の外は既定の色、
/// 濃度が 255 未満なら隠す量をその割合に弱める。
fn apply_mask(raw: &RawLayer, mask: &Mask, pixels: &[u8], rgba: &mut [u8]) {
    let density = u32::from(mask.density);
    let (mw, mh) = (i64::from(mask.width), i64::from(mask.height));
    for (i, px) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = i64::from(raw.rect.left) + (i % raw.width as usize) as i64;
        let y = i64::from(raw.rect.top) + (i / raw.width as usize) as i64;
        let (mx, my) = (x - i64::from(mask.rect.left), y - i64::from(mask.rect.top));
        let m = if (0..mw).contains(&mx) && (0..mh).contains(&my) {
            pixels[(my * mw + mx) as usize]
        } else {
            mask.default_color
        };
        let m = 255 - ((255 - u32::from(m)) * density + 127) / 255;
        px[3] = ((u32::from(px[3]) * m + 127) / 255) as u8;
    }
}
