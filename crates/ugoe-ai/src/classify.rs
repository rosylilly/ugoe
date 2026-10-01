//! レイヤーの意味分類。設計: `docs/ai-integration.md#レイヤーの意味分類`。
//!
//! - ラベルの語彙は [`Label`] (40 個) で、左右は [`Side`] で別に持つ。
//! - 分類の結果はレイヤーごとの [`LayerLabel`] (`layer_path`・`label`・`side`・`confidence`)。
//!
//! 分類の経路は 2 つある。
//! - [`classify_by_name`]: レイヤー名の規則だけで分類する (API を使わない、決定的)。
//! - [`classify_layers`]: [`Provider`] に画像 (全体の合成画像とレイヤーのサムネイル) と
//!   レイヤー名を送り、[`output_schema`] に沿った JSON で分類させる。モデルが答えなかった
//!   レイヤーは名前の規則で補う。利用者の注文と前の分類を添えるなら [`classify_layers_guided`]
//!   ([`ClassifyGuidance`]、`docs/ai-integration.md#利用者の注文`)。

use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::json;
use ugoe_psd::{Layer, LayerNode, LayerTree};

use crate::instructions::{Instructions, system_rules};
use crate::{AiError, Provider, ProviderRequest};

mod image;
mod rules;

/// レイヤーの意味ラベル。左右は含めず [`Side`] で表す。
///
/// JSON では snake_case の文字列 (`"eye_white"` など)。標準パラメータやリグのテンプレートは
/// この語彙で部位を指す。語彙を変えるときは `docs/ai-integration.md` も直す。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Label {
    // 顔
    /// 顔の輪郭と肌 (頭の土台)。
    Face,
    Ear,
    Nose,
    /// 頬の赤み・影。
    Cheek,
    Neck,
    // 目
    /// 目の部位を分けていない目全体。
    Eye,
    /// 白目。
    EyeWhite,
    /// 黒目 (虹彩と瞳孔)。
    Iris,
    /// 目のハイライト。
    EyeHighlight,
    /// 上まつ毛 (上まぶたの線)。
    EyelashUpper,
    /// 下まつ毛 (下まぶたの線)。
    EyelashLower,
    /// 閉じた目 (目を閉じた差分の絵)。
    EyeClosed,
    Eyebrow,
    // 口
    /// 口の部位を分けていない口全体。
    Mouth,
    /// 開いた口 (差分の絵)。
    MouthOpen,
    /// 閉じた口 (差分の絵)。
    MouthClosed,
    /// 口の中 (口腔)。
    MouthInner,
    Teeth,
    Tongue,
    // 髪
    /// 前髪。
    HairFront,
    /// 横髪 (もみあげ・サイドの房)。
    HairSide,
    /// 後ろ髪。
    HairBack,
    // 体
    /// 胴体 (肌)。
    Body,
    Arm,
    Hand,
    Leg,
    Foot,
    /// しっぽ。
    Tail,
    // 服
    /// 上半身の服。
    Topwear,
    /// 下半身の服 (スカート・ズボン)。
    Bottomwear,
    /// 脚に着けるもの (靴下・タイツ)。
    Legwear,
    /// 靴。
    Footwear,
    /// 手に着けるもの (手袋・袖口)。
    Handwear,
    /// 首に着けるもの (襟・リボン・チョーカー)。
    Neckwear,
    // 装飾
    /// 頭に着けるもの (帽子・髪飾り・カチューシャ)。
    Headwear,
    /// 耳に着けるもの (イヤリング・ヘッドホン)。
    Earwear,
    /// 眼鏡など。
    Eyewear,
    /// その他の装飾品・持ち物。
    Accessory,
    // その他
    /// 背景。
    Background,
    /// どれにも当てはまらない (効果・影・分からないもの)。
    Other,
}

impl Label {
    /// すべてのラベル (宣言順)。
    pub const ALL: [Label; 40] = [
        Label::Face,
        Label::Ear,
        Label::Nose,
        Label::Cheek,
        Label::Neck,
        Label::Eye,
        Label::EyeWhite,
        Label::Iris,
        Label::EyeHighlight,
        Label::EyelashUpper,
        Label::EyelashLower,
        Label::EyeClosed,
        Label::Eyebrow,
        Label::Mouth,
        Label::MouthOpen,
        Label::MouthClosed,
        Label::MouthInner,
        Label::Teeth,
        Label::Tongue,
        Label::HairFront,
        Label::HairSide,
        Label::HairBack,
        Label::Body,
        Label::Arm,
        Label::Hand,
        Label::Leg,
        Label::Foot,
        Label::Tail,
        Label::Topwear,
        Label::Bottomwear,
        Label::Legwear,
        Label::Footwear,
        Label::Handwear,
        Label::Neckwear,
        Label::Headwear,
        Label::Earwear,
        Label::Eyewear,
        Label::Accessory,
        Label::Background,
        Label::Other,
    ];

    /// JSON での名前 (`"eye_white"` など)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Label::Face => "face",
            Label::Ear => "ear",
            Label::Nose => "nose",
            Label::Cheek => "cheek",
            Label::Neck => "neck",
            Label::Eye => "eye",
            Label::EyeWhite => "eye_white",
            Label::Iris => "iris",
            Label::EyeHighlight => "eye_highlight",
            Label::EyelashUpper => "eyelash_upper",
            Label::EyelashLower => "eyelash_lower",
            Label::EyeClosed => "eye_closed",
            Label::Eyebrow => "eyebrow",
            Label::Mouth => "mouth",
            Label::MouthOpen => "mouth_open",
            Label::MouthClosed => "mouth_closed",
            Label::MouthInner => "mouth_inner",
            Label::Teeth => "teeth",
            Label::Tongue => "tongue",
            Label::HairFront => "hair_front",
            Label::HairSide => "hair_side",
            Label::HairBack => "hair_back",
            Label::Body => "body",
            Label::Arm => "arm",
            Label::Hand => "hand",
            Label::Leg => "leg",
            Label::Foot => "foot",
            Label::Tail => "tail",
            Label::Topwear => "topwear",
            Label::Bottomwear => "bottomwear",
            Label::Legwear => "legwear",
            Label::Footwear => "footwear",
            Label::Handwear => "handwear",
            Label::Neckwear => "neckwear",
            Label::Headwear => "headwear",
            Label::Earwear => "earwear",
            Label::Eyewear => "eyewear",
            Label::Accessory => "accessory",
            Label::Background => "background",
            Label::Other => "other",
        }
    }

    /// 大まかな分類。
    pub const fn category(self) -> LabelCategory {
        use Label::*;
        match self {
            Face | Ear | Nose | Cheek | Neck => LabelCategory::Face,
            Eye | EyeWhite | Iris | EyeHighlight | EyelashUpper | EyelashLower | EyeClosed
            | Eyebrow => LabelCategory::Eye,
            Mouth | MouthOpen | MouthClosed | MouthInner | Teeth | Tongue => LabelCategory::Mouth,
            HairFront | HairSide | HairBack => LabelCategory::Hair,
            Body | Arm | Hand | Leg | Foot | Tail => LabelCategory::Body,
            Topwear | Bottomwear | Legwear | Footwear | Handwear | Neckwear => {
                LabelCategory::Clothing
            }
            Headwear | Earwear | Eyewear | Accessory => LabelCategory::Accessory,
            Background | Other => LabelCategory::Other,
        }
    }

    /// 左右の対で現れることの多い部位か (目・眉・耳・腕など)。
    ///
    /// これが `false` のラベルでも、左右に分かれたレイヤーなら [`Side`] は左右になりうる
    /// (左右の横髪など)。
    pub const fn is_paired(self) -> bool {
        use Label::*;
        matches!(
            self,
            Ear | Cheek
                | Eye
                | EyeWhite
                | Iris
                | EyeHighlight
                | EyelashUpper
                | EyelashLower
                | EyeClosed
                | Eyebrow
                | HairSide
                | Arm
                | Hand
                | Leg
                | Foot
                | Legwear
                | Footwear
                | Handwear
                | Earwear
        )
    }

    /// ラベルの説明 (英語)。分類を頼むモデル・エージェントに語彙として示す。
    pub const fn description(self) -> &'static str {
        label_description(self)
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 知らないラベルの名前。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown label: {0}")]
pub struct UnknownLabel(pub String);

impl FromStr for Label {
    type Err = UnknownLabel;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Label::ALL
            .into_iter()
            .find(|l| l.as_str() == s)
            .ok_or_else(|| UnknownLabel(s.to_owned()))
    }
}

/// ラベルの大まかな分類 (メッシュのプリセットやテンプレートの選択に使う)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelCategory {
    /// 顔の土台 (顔・耳・鼻・頬・首)。
    Face,
    /// 目と眉。
    Eye,
    Mouth,
    Hair,
    Body,
    Clothing,
    Accessory,
    /// 背景とその他。
    Other,
}

/// 左右。**キャラクター自身から見た** 左右 (`docs/spec/avatar-profile.md#向きと左右の約束`)。
///
/// 画面に正対したキャラクターの左目 (`Left`) は、画像の右側にある。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Left,
    Right,
    /// 中央 (左右のないもの・両側にまたがるもの)。
    Center,
}

impl Side {
    pub const ALL: [Side; 3] = [Side::Left, Side::Right, Side::Center];

    pub const fn as_str(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
            Side::Center => "center",
        }
    }
}

/// 1 つのレイヤーの分類の結果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerLabel {
    /// レイヤーの名前の列 (外側のグループから順に、最後がレイヤー自身)。
    /// `ugoe_psd::ImportWarning::path` と同じ形。
    pub layer_path: Vec<String>,
    pub label: Label,
    pub side: Side,
    /// 確からしさ (0..=1)。低いものはエディタで人間に確認を求める。
    pub confidence: f32,
    /// AI が同じ部品の別の絵 (一度に 1 つだけ見せる差分。袖 A / B など) とみなしたレイヤーの組の番号
    /// (組の中で最も小さいレイヤーの番号)。名前の規則では付けない。衣装の差分をポーズにするときの手がかり
    /// (`crate::module_rig`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<u32>,
}

impl LayerLabel {
    /// 確認が要るほど確からしさが低いか ([`ClassifyOptions::review_below`] 未満)。
    pub fn needs_review(&self, options: &ClassifyOptions) -> bool {
        self.confidence < options.review_below
    }
}

/// 分類の設定。送る画像の大きさと枚数の上限を決める。
#[derive(Debug, Clone, PartialEq)]
pub struct ClassifyOptions {
    /// 全体の合成画像の長い辺の上限 (画素)。
    pub overview_max_side: u32,
    /// レイヤーのサムネイルの長い辺の上限 (画素)。
    pub thumbnail_max_side: u32,
    /// これより小さいサムネイルは整数倍に拡大する (画素)。
    pub thumbnail_min_side: u32,
    /// 1 回のリクエストで分類するレイヤーの数の上限。これを超えると複数のリクエストに分け、
    /// それぞれに全体の合成画像を付ける (1 回に送る画像はこれ + 1 枚)。
    pub max_layers_per_request: usize,
    /// 確からしさがこれ未満の結果は、エディタで人間に確認を求める。
    pub review_below: f32,
}

impl Default for ClassifyOptions {
    fn default() -> Self {
        Self {
            // Claude は長い辺が 1568 画素を超える画像を縮めて読む。1024 なら約 1400 トークン
            overview_max_side: 1024,
            // 256x256 で約 90 トークン。32 枚で 3000 トークン弱
            thumbnail_max_side: 256,
            thumbnail_min_side: 64,
            // Messages API の上限は 1 リクエスト 100 枚。余裕をもって 32 + 1 枚にする
            max_layers_per_request: 32,
            review_below: 0.6,
        }
    }
}

/// 分類に添える、利用者の注文と前の分類 (作り直し)。既定 (どちらもない) なら [`classify_layers`] と同じ。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClassifyGuidance {
    /// 利用者の注文。ラベルと左右に関わるものだけに従わせ、ほかの注文 (リグの動きなど) はこの段階では扱わない
    /// (出力の形に理由の項目がないので、反映できない注文はリグの段階で示す)。
    pub instructions: Option<Instructions>,
    /// 前の分類 ([`classify_by_name`] と同じ並び)。レイヤーごとに前のラベルと左右をプロンプトに書き、
    /// 注文が求めるところだけ変えさせる。
    pub previous: Option<Vec<LayerLabel>>,
}

/// 分類の対象 (木を平らにした 1 枚のレイヤー)。
struct Target<'a> {
    path: Vec<String>,
    layer: &'a Layer,
}

/// 木のラスタレイヤーを、描画順 (下から上、グループは深さ優先) に並べる。
fn targets(tree: &LayerTree) -> Vec<Target<'_>> {
    fn walk<'a>(nodes: &'a [LayerNode], prefix: &mut Vec<String>, out: &mut Vec<Target<'a>>) {
        for node in nodes {
            match node {
                LayerNode::Group { name, children, .. } => {
                    prefix.push(name.clone());
                    walk(children, prefix, out);
                    prefix.pop();
                }
                LayerNode::Layer(layer) => {
                    let mut path = prefix.clone();
                    path.push(layer.name.clone());
                    out.push(Target { path, layer });
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(&tree.root, &mut Vec::new(), &mut out);
    out
}

/// グループの名前から受け継いだラベルの確からしさに掛ける係数。
const INHERITED_FACTOR: f32 = 0.7;

/// レイヤー名の規則だけで分類する (API を使わない)。
///
/// 結果は描画順 (下から上、グループは深さ優先) にすべてのラスタレイヤーを並べる。
/// - ラベル: レイヤー名の規則が当たればそれを使う。当たらなければ内側のグループから順に
///   名前を見て受け継ぐ (確からしさを下げる)。どれにも当たらなければ `other` で確からしさ 0。
/// - 左右: レイヤー名、グループ名の順に `l` / `r` / 「左」「右」などを探す。書かれていない
///   左右の対の部位 ([`Label::is_paired`]) は、顔 (なければキャンバス) の中心との位置で決める。
pub fn classify_by_name(tree: &LayerTree) -> Vec<LayerLabel> {
    let targets = targets(tree);
    let mut labels: Vec<LayerLabel> = targets.iter().map(label_target_by_name).collect();
    let center = reference_center_x(tree, &targets, &labels);
    for (label, target) in labels.iter_mut().zip(&targets) {
        if label.side == Side::Center
            && label.label.is_paired()
            && !target
                .path
                .iter()
                .any(|n| rules::side_from_name(n).is_some())
        {
            label.side = side_from_position(target.layer, center);
        }
    }
    labels
}

/// メッシュの名前の規則だけで意味ラベルを付ける (API を使わない、決定的)。規則が当たらないメッシュは含めない。
///
/// PSD の分類 ([`classify_by_name`] など) がないときに、法線マップのまとまり (`crate::normal`) を決めるのに使う。
/// PSD から取り込んだメッシュの名前はレイヤーの名前なので、グループの名前から受け継がない点のほかは
/// [`classify_by_name`] と同じラベルになる。
pub fn label_meshes_by_name(
    model: &ugoe_model::Model,
) -> std::collections::BTreeMap<ugoe_model::MeshId, Label> {
    model
        .meshes
        .iter()
        .filter_map(|m| Some((m.id.clone(), rules::label_from_name(&m.name)?.label)))
        .collect()
}

/// 名前だけで 1 枚を分類する (位置による左右は後で決める)。
fn label_target_by_name(target: &Target<'_>) -> LayerLabel {
    let (label, confidence) = target
        .path
        .iter()
        .rev()
        .enumerate()
        .find_map(|(depth, name)| {
            let m = rules::label_from_name(name)?;
            let factor = if depth == 0 { 1.0 } else { INHERITED_FACTOR };
            Some((m.label, m.confidence * factor))
        })
        .unwrap_or((Label::Other, 0.0));
    let side = target
        .path
        .iter()
        .rev()
        .find_map(|n| rules::side_from_name(n))
        .unwrap_or(Side::Center);
    LayerLabel {
        layer_path: target.path.clone(),
        label,
        side,
        confidence,
        variant: None,
    }
}

/// 左右を決める基準の x 座標。顔のレイヤーがあればその中心、なければキャンバスの中心。
fn reference_center_x(tree: &LayerTree, targets: &[Target<'_>], labels: &[LayerLabel]) -> f32 {
    targets
        .iter()
        .zip(labels)
        .find(|(t, l)| l.label == Label::Face && t.layer.width > 0)
        .map(|(t, _)| t.layer.offset.x + t.layer.width as f32 / 2.0)
        .unwrap_or(tree.width as f32 / 2.0)
}

/// 位置から左右を決める。画像の右にあるものがキャラクターの左 (正対している前提)。
/// 基準の x 座標をまたぐものは中央。
fn side_from_position(layer: &Layer, center_x: f32) -> Side {
    let x0 = layer.offset.x;
    let x1 = x0 + layer.width as f32;
    if layer.width == 0 || (x0 < center_x && center_x < x1) {
        Side::Center
    } else if x0 >= center_x {
        Side::Left
    } else {
        Side::Right
    }
}

/// モデルの出力の JSON Schema。
///
/// `{ "layers": [{ "index", "label", "side", "confidence" }], "variant_sets": [[index, ...]] }`。`index` はプロンプトに
/// 書いたレイヤーの番号。`variant_sets` は同じ部品の別の絵 (差分) のレイヤーの組 ([`LayerLabel::variant`])。
/// 構造化出力の制約により、数値の範囲 (`confidence` の 0..=1) はスキーマに書かず、受け取った後に丸める
/// (strict の構造化出力ではすべての項目が必須なので、`variant_sets` も必須にし、なければ空の配列を答えさせる)。
pub fn output_schema() -> serde_json::Value {
    let labels: Vec<&str> = Label::ALL.iter().map(|l| l.as_str()).collect();
    let sides: Vec<&str> = Side::ALL.iter().map(|s| s.as_str()).collect();
    json!({
        "type": "object",
        "properties": {
            "layers": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "index": { "type": "integer", "description": "The layer number from the prompt." },
                        "label": { "type": "string", "enum": labels },
                        "side": { "type": "string", "enum": sides },
                        "confidence": { "type": "number", "description": "How sure you are, from 0 to 1." }
                    },
                    "required": ["index", "label", "side", "confidence"],
                    "additionalProperties": false
                }
            },
            "variant_sets": {
                "type": "array",
                "description": "Groups of layer numbers that are alternative drawings of the same part, of which only one is shown at a time. Empty if there are none.",
                "items": { "type": "array", "items": { "type": "integer" } }
            }
        },
        "required": ["layers", "variant_sets"],
        "additionalProperties": false
    })
}

/// モデルに送るラベルの説明 (英語)。
const fn label_description(label: Label) -> &'static str {
    match label {
        Label::Face => "face base: facial outline and skin",
        Label::Ear => "ear",
        Label::Nose => "nose",
        Label::Cheek => "cheek blush or shading",
        Label::Neck => "neck skin",
        Label::Eye => "a whole eye that is not split into parts",
        Label::EyeWhite => "white of the eye (sclera)",
        Label::Iris => "iris and pupil",
        Label::EyeHighlight => "highlight drawn on the eye",
        Label::EyelashUpper => "upper eyelashes / upper eyelid line",
        Label::EyelashLower => "lower eyelashes / lower eyelid line",
        Label::EyeClosed => "alternative drawing of the closed eye",
        Label::Eyebrow => "eyebrow",
        Label::Mouth => "a whole mouth that is not split into parts",
        Label::MouthOpen => "alternative drawing of the open mouth",
        Label::MouthClosed => "alternative drawing of the closed mouth",
        Label::MouthInner => "inside of the mouth",
        Label::Teeth => "teeth or fangs",
        Label::Tongue => "tongue",
        Label::HairFront => "front hair / bangs, drawn in front of the face",
        Label::HairSide => "side locks hanging beside the face",
        Label::HairBack => "back hair, drawn behind the head and body",
        Label::Body => "torso skin",
        Label::Arm => "arm skin",
        Label::Hand => "bare hand",
        Label::Leg => "bare leg",
        Label::Foot => "bare foot",
        Label::Tail => "tail",
        Label::Topwear => "clothing on the upper body",
        Label::Bottomwear => "skirt, pants or shorts",
        Label::Legwear => "socks, stockings or tights",
        Label::Footwear => "shoes or boots",
        Label::Handwear => "gloves, sleeves or cuffs",
        Label::Neckwear => "collar, tie, ribbon or choker",
        Label::Headwear => "hat, hair ornament or headband",
        Label::Earwear => "earrings or headphones",
        Label::Eyewear => "glasses",
        Label::Accessory => "any other accessory or held item",
        Label::Background => "background",
        Label::Other => "effects, shadows, or anything that fits no other label",
    }
}

/// 分類の指示 (英語): 左右の約束・名前と画素の扱い・ラベルの語彙と説明。
///
/// [`classify_layers`] がモデルに送るシステムプロンプトと同じ文。MCP で外部のエージェントに分類を
/// 任せるときにも、同じ指示を示す (`crates/ugoe-mcp`)。
pub fn classification_instructions() -> String {
    system_prompt()
}

/// 注文か前の分類があるときにシステムプロンプトに足す規則。
fn guidance_rules(guidance: &ClassifyGuidance) -> String {
    let mut s = String::new();
    if guidance.previous.is_some() {
        s.push_str(
            "\n\nThis is a revision. Each listed layer shows your previous label and side. Keep \
             them unless the user's instructions or a clear mistake call for a change.",
        );
    }
    if guidance.instructions.is_some() {
        s.push_str(&system_rules(
            "follow the parts about labels and sides of layers. Other parts (such as how the rig \
             moves) are handled in a later step, so ignore them here.",
        ));
    }
    s
}

/// システムプロンプト。
fn system_prompt() -> String {
    let mut s = String::from(
        "You label the layers of a layered character illustration so that it can be rigged \
         for 2D animation. For each listed layer, choose exactly one label from the vocabulary \
         below, the side, and your confidence from 0 to 1.\n\n\
         Side is the character's OWN left or right, not the viewer's: for a character facing \
         the viewer, the character's left eye appears on the RIGHT side of the image. \
         Layer name suffixes such as \"-l\" / \"-r\" or \"L\" / \"R\" usually already follow \
         this convention. Use \"center\" for parts that are not one of a left/right pair or \
         that span both sides.\n\n\
         Use the layer name as a strong hint, but trust the pixels when they disagree. \
         Alternative drawings (open/closed eyes and mouths) may be hidden in the composite.\n\n\
         Labels:\n",
    );
    for label in Label::ALL {
        s.push_str(&format!(
            "- {}: {}\n",
            label.as_str(),
            label_description(label)
        ));
    }
    s
}

/// 1 回のリクエストのプロンプト。
fn prompt(
    tree: &LayerTree,
    batch: &[(usize, Target<'_>)],
    overview_scale: f32,
    guidance: &ClassifyGuidance,
) -> String {
    let (ow, oh) = (
        (tree.width as f32 * overview_scale).round() as u32,
        (tree.height as f32 * overview_scale).round() as u32,
    );
    let mut s = format!(
        "Image 1 is the whole illustration ({ow}x{oh} pixels, visible layers only). \
         Images 2 to {} are the layers listed below, in the same order, each cropped to its \
         own bounds and drawn over light gray. Bounding boxes are in image 1 pixels \
         as x, y, width, height.\n\nLayers (bottom to top):\n",
        batch.len() + 1,
    );
    for (i, t) in batch {
        let l = t.layer;
        let bbox = [
            l.offset.x * overview_scale,
            l.offset.y * overview_scale,
            l.width as f32 * overview_scale,
            l.height as f32 * overview_scale,
        ]
        .map(|v| v.round() as i64);
        let previous = guidance
            .previous
            .as_ref()
            .and_then(|p| p.get(*i))
            .map(|p| {
                let variant = p
                    .variant
                    .map(|v| format!(", variant set of layer {v}"))
                    .unwrap_or_default();
                format!(
                    "; previous: {} ({}){variant}",
                    p.label.as_str(),
                    p.side.as_str()
                )
            })
            .unwrap_or_default();
        s.push_str(&format!(
            "{i}. name: {}; bbox: {}, {}, {}, {}{}{previous}\n",
            serde_json::Value::String(t.path.join(" / ")),
            bbox[0],
            bbox[1],
            bbox[2],
            bbox[3],
            if l.visible { "" } else { "; hidden" },
        ));
    }
    if let Some(instructions) = &guidance.instructions {
        s.push_str("\nThe user's instructions:\n");
        s.push_str(&instructions.block());
    }
    s.push_str(
        "\nReturn one entry for every listed layer, using its number as `index`. In `variant_sets`, \
         list the groups of listed layers that are alternative drawings of the same clothing or \
         accessory part, of which only one is meant to be shown at a time (for example sleeves A \
         and B, or a hat and a cap worn in the same place); hidden layers are often such \
         alternatives. Do not group parts drawn on top of each other (shading, line art, \
         highlights) or open and closed eyes and mouths. Answer [] if there are none.",
    );
    s
}

/// 全体の合成画像 (見えるレイヤーだけ) を、長い辺が `max_side` 以下になるように縮めた PNG。
///
/// 自動リギング ([`crate::rig::propose_rig_with_ai`]) で AI に送る原画に使う。
pub fn overview_png(tree: &LayerTree, max_side: u32) -> Result<Vec<u8>, AiError> {
    image::overview(tree, max_side).0.to_png()
}

/// 分類のために送るリクエスト。レイヤーが多ければ複数に分ける。
///
/// 画素を持たないレイヤーは送らない (名前の規則で分類する)。エディタは送る前に、これを
/// [`Provider::disclosure`] に渡して利用者に確認を取る。
pub fn classification_requests(
    tree: &LayerTree,
    options: &ClassifyOptions,
) -> Result<Vec<ProviderRequest>, AiError> {
    classification_requests_guided(tree, options, &ClassifyGuidance::default())
}

/// [`classification_requests`] の、注文と前の分類を添える版 ([`classify_layers_guided`] が送るもの)。
pub fn classification_requests_guided(
    tree: &LayerTree,
    options: &ClassifyOptions,
    guidance: &ClassifyGuidance,
) -> Result<Vec<ProviderRequest>, AiError> {
    Ok(plan(tree, options, guidance)?
        .into_iter()
        .map(|b| b.request)
        .collect())
}

/// 分類の対象の 1 枚のレイヤー ([`layers`] が返す)。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayerInfo {
    /// 番号 ([`classify_by_name`] の結果の並び・[`output_schema`] の `index` と同じ)。
    pub index: usize,
    /// レイヤーの名前の列 (外側のグループから順に、最後がレイヤー自身)。
    pub layer_path: Vec<String>,
    /// キャンバスの画素での外接矩形 (`[x, y, width, height]`)。
    pub bbox: [f32; 4],
    pub visible: bool,
    /// 画素を持つ (サムネイルを作れる)。
    pub has_pixels: bool,
}

/// 分類の対象のレイヤー (描画順。[`classify_by_name`] の結果と同じ並び)。
///
/// MCP で外部のエージェントに分類を任せるとき、番号とレイヤーの対応を示すのに使う。
pub fn layers(tree: &LayerTree) -> Vec<LayerInfo> {
    targets(tree)
        .into_iter()
        .enumerate()
        .map(|(index, t)| LayerInfo {
            index,
            bbox: [
                t.layer.offset.x,
                t.layer.offset.y,
                t.layer.width as f32,
                t.layer.height as f32,
            ],
            visible: t.layer.visible,
            has_pixels: t.layer.width > 0 && t.layer.height > 0,
            layer_path: t.path,
        })
        .collect()
}

/// `index` 番のレイヤーのサムネイルの PNG ([`classify_layers`] が送るものと同じ作り方。
/// 大きさは `options` の `thumbnail_max_side` / `thumbnail_min_side`)。画素がなければ `None`。
pub fn layer_thumbnail_png(
    tree: &LayerTree,
    index: usize,
    options: &ClassifyOptions,
) -> Result<Option<Vec<u8>>, AiError> {
    let targets = targets(tree);
    let target = targets.get(index).ok_or_else(|| {
        AiError::InvalidRequest(format!(
            "layer {index} does not exist (there are {} layers)",
            targets.len()
        ))
    })?;
    image::thumbnail(
        target.layer,
        options.thumbnail_max_side,
        options.thumbnail_min_side,
    )
    .map(|img| img.to_png())
    .transpose()
}

/// 1 回のリクエストと、そこで分類するレイヤーの番号 (全体の中の)。
struct Batch {
    request: ProviderRequest,
    indices: Vec<usize>,
}

fn plan(
    tree: &LayerTree,
    options: &ClassifyOptions,
    guidance: &ClassifyGuidance,
) -> Result<Vec<Batch>, AiError> {
    if options.max_layers_per_request == 0 {
        return Err(AiError::InvalidRequest(
            "max_layers_per_request must be at least 1".into(),
        ));
    }
    let mut thumbs = Vec::new();
    for (i, t) in targets(tree).into_iter().enumerate() {
        if let Some(img) = image::thumbnail(
            t.layer,
            options.thumbnail_max_side,
            options.thumbnail_min_side,
        ) {
            thumbs.push((i, t, img));
        }
    }
    if thumbs.is_empty() {
        return Ok(Vec::new());
    }
    let (overview, scale) = image::overview(tree, options.overview_max_side);
    let overview = overview.to_png()?;
    let system = system_prompt() + &guidance_rules(guidance);
    let mut batches = Vec::new();
    let mut rest = thumbs;
    while !rest.is_empty() {
        let tail = rest.split_off(rest.len().min(options.max_layers_per_request));
        let chunk = std::mem::replace(&mut rest, tail);
        let mut images = vec![overview.clone()];
        let mut listed = Vec::with_capacity(chunk.len());
        for (i, t, img) in chunk {
            images.push(img.to_png()?);
            listed.push((i, t));
        }
        batches.push(Batch {
            request: ProviderRequest {
                system: system.clone(),
                prompt: prompt(tree, &listed, scale, guidance),
                images,
            },
            indices: listed.iter().map(|(i, _)| *i).collect(),
        });
    }
    Ok(batches)
}

#[derive(Deserialize)]
struct Output {
    layers: Vec<OutputLayer>,
    /// 差分の組。答えになければ (前からの答え・外部のエージェントの分類) 差分の手がかりを変えない。
    #[serde(default)]
    variant_sets: Option<Vec<Vec<i64>>>,
}

#[derive(Deserialize)]
struct OutputLayer {
    index: i64,
    label: Label,
    side: Side,
    confidence: f64,
}

/// [`Provider`] で分類する。
///
/// 結果の並びは [`classify_by_name`] と同じ。モデルが答えなかったレイヤー (画素のない
/// レイヤーを含む) は名前の規則の結果を使う。モデルがリクエストにない番号を返したり、
/// 同じレイヤーに 2 度答えたりしたら [`AiError::InvalidResponse`] を返す。
pub fn classify_layers(
    provider: &dyn Provider,
    tree: &LayerTree,
    options: &ClassifyOptions,
) -> Result<Vec<LayerLabel>, AiError> {
    classify_layers_guided(provider, tree, options, &ClassifyGuidance::default())
}

/// [`classify_layers`] の、利用者の注文と前の分類を添える版 (`docs/ai-integration.md#利用者の注文`)。
///
/// 出力の形は [`output_schema`] のまま。前の分類があれば、モデルが答えなかったレイヤーは前の分類を使う
/// (なければ名前の規則)。`previous` の長さがレイヤーの数と違えば [`AiError::InvalidRequest`]。
pub fn classify_layers_guided(
    provider: &dyn Provider,
    tree: &LayerTree,
    options: &ClassifyOptions,
    guidance: &ClassifyGuidance,
) -> Result<Vec<LayerLabel>, AiError> {
    let mut labels = classify_by_name(tree);
    if let Some(previous) = &guidance.previous {
        if previous.len() != labels.len() {
            return Err(AiError::InvalidRequest(format!(
                "the previous classification has {} layers, but the PSD has {}",
                previous.len(),
                labels.len()
            )));
        }
        labels.clone_from_slice(previous);
    }
    let schema = output_schema();
    for batch in plan(tree, options, guidance)? {
        let value = provider.complete_json(&batch.request, &schema)?;
        merge_answer(
            &mut labels,
            value,
            |i| batch.indices.contains(&i),
            |index| format!("layer {index} was not in the request"),
        )?;
    }
    Ok(labels)
}

/// [`output_schema`] の形の答え (外部のエージェントが決めた分類) を `labels` に書き込む。
///
/// `labels` は [`classify_by_name`] の結果など、すべてのレイヤーを同じ並びで持つもの。答えにない
/// レイヤーはそのまま。番号が範囲の外・同じレイヤーに 2 度答えた・形が違うものは
/// [`AiError::InvalidResponse`] で、そのときは `labels` を変えない。`confidence` は 0..=1 に丸める。
/// 書き込んだレイヤーの数を返す。
pub fn apply_answer(
    labels: &mut [LayerLabel],
    answer: serde_json::Value,
) -> Result<usize, AiError> {
    let count = labels.len();
    let mut next = labels.to_vec();
    let written = merge_answer(
        &mut next,
        answer,
        |i| i < count,
        |index| format!("layer {index} does not exist (there are {count} layers, numbered from 0)"),
    )?;
    labels.clone_from_slice(&next);
    Ok(written)
}

/// 答えを `labels` に書き込む。`allowed` でない番号は `not_allowed` の文のエラー。
fn merge_answer(
    labels: &mut [LayerLabel],
    answer: serde_json::Value,
    allowed: impl Fn(usize) -> bool,
    not_allowed: impl Fn(i64) -> String,
) -> Result<usize, AiError> {
    let output: Output = serde_json::from_value(answer).map_err(|e| {
        AiError::InvalidResponse(format!("classification does not match the schema: {e}"))
    })?;
    let mut seen = HashSet::new();
    for entry in output.layers {
        let index = usize::try_from(entry.index)
            .ok()
            .filter(|&i| i < labels.len() && allowed(i))
            .ok_or_else(|| AiError::InvalidResponse(not_allowed(entry.index)))?;
        if !seen.insert(index) {
            return Err(AiError::InvalidResponse(format!(
                "layer {index} was labeled twice"
            )));
        }
        let confidence = if entry.confidence.is_finite() {
            entry.confidence.clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        let slot = &mut labels[index];
        slot.label = entry.label;
        slot.side = entry.side;
        slot.confidence = confidence;
    }
    if let Some(sets) = output.variant_sets {
        let mut grouped: Vec<Vec<usize>> = Vec::new();
        let mut placed = HashSet::new();
        for set in sets {
            let mut indices = Vec::with_capacity(set.len());
            for index in set {
                let i = usize::try_from(index)
                    .ok()
                    .filter(|&i| i < labels.len() && allowed(i))
                    .ok_or_else(|| AiError::InvalidResponse(not_allowed(index)))?;
                if !placed.insert(i) {
                    return Err(AiError::InvalidResponse(format!(
                        "layer {i} is in more than one variant set"
                    )));
                }
                indices.push(i);
            }
            // 1 枚だけの組は差分にならないので捨てる。
            if indices.len() >= 2 {
                grouped.push(indices);
            }
        }
        // 答えた範囲 (このリクエストのレイヤー) の手がかりを、答えで置き換える。
        for (i, label) in labels.iter_mut().enumerate() {
            if allowed(i) {
                label.variant = None;
            }
        }
        for set in grouped {
            let id = set.iter().copied().min().unwrap_or_default() as u32;
            for i in set {
                labels[i].variant = Some(id);
            }
        }
    }
    Ok(seen.len())
}
