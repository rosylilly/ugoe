//! レイヤー名による分類 (API を使わない、決定的な規則)。
//!
//! 名前を語に分け (区切り文字・camelCase・数字)、連続する 1〜3 語をつないだ文字列を
//! キーワードと比べる。日本語の名前は部分文字列で比べる。規則は具体的なものから順に並び、
//! 最初に当たったものを採る (`eyewhite` を `eye` より先に、`handwear` を `hand` より先に)。

use super::{Label, Side};

/// 名前の規則が当たったときの確からしさ。
pub(super) const NAME_CONFIDENCE: f32 = 0.9;
/// あいまいなキーワード (`hair` だけ、など) が当たったときの確からしさ。
const VAGUE_CONFIDENCE: f32 = 0.5;

/// 1 つの規則。`words` は小文字の英語 (空白なしでつないだ形)、`kanji` は日本語の部分文字列。
struct Rule {
    label: Label,
    words: &'static [&'static str],
    kanji: &'static [&'static str],
    confidence: f32,
}

const fn rule(
    label: Label,
    words: &'static [&'static str],
    kanji: &'static [&'static str],
) -> Rule {
    Rule {
        label,
        words,
        kanji,
        confidence: NAME_CONFIDENCE,
    }
}

const fn vague(
    label: Label,
    words: &'static [&'static str],
    kanji: &'static [&'static str],
) -> Rule {
    Rule {
        label,
        words,
        kanji,
        confidence: VAGUE_CONFIDENCE,
    }
}

/// 規則の表。上にあるものほど優先する。
const RULES: &[Rule] = &[
    // 装飾・服 (部位の語を含むので部位より先に見る)
    rule(
        Label::Earwear,
        &[
            "earwear",
            "earring",
            "earrings",
            "headphone",
            "headphones",
            "earphone",
        ],
        &["イヤリング", "ピアス", "ヘッドホン", "ヘッドフォン", "耳飾"],
    ),
    rule(
        Label::Eyewear,
        &[
            "eyewear",
            "glasses",
            "eyeglasses",
            "spectacles",
            "goggles",
            "sunglasses",
        ],
        &["眼鏡", "メガネ", "めがね", "ゴーグル"],
    ),
    rule(
        Label::Headwear,
        &[
            "headwear",
            "hat",
            "cap",
            "helmet",
            "hood",
            "hairpin",
            "hairclip",
            "hairband",
            "headband",
            "ribbon",
            "headdress",
            "crown",
            "tiara",
            "hairaccessory",
            "hairornament",
        ],
        &[
            "帽子",
            "髪飾",
            "ヘアピン",
            "カチューシャ",
            "リボン",
            "ヘッドドレス",
            "冠",
        ],
    ),
    rule(
        Label::Neckwear,
        &[
            "neckwear", "collar", "necktie", "tie", "choker", "scarf", "necklace", "bowtie",
        ],
        &[
            "襟",
            "ネクタイ",
            "チョーカー",
            "マフラー",
            "ネックレス",
            "スカーフ",
        ],
    ),
    rule(
        Label::Handwear,
        &[
            "handwear", "glove", "gloves", "cuff", "cuffs", "sleeve", "sleeves", "bracelet",
        ],
        &["手袋", "袖", "ブレスレット"],
    ),
    rule(
        Label::Footwear,
        &[
            "footwear", "shoe", "shoes", "boot", "boots", "sneakers", "sandals",
        ],
        &["靴", "ブーツ"],
    ),
    rule(
        Label::Legwear,
        &[
            "legwear",
            "sock",
            "socks",
            "stocking",
            "stockings",
            "tights",
            "leggings",
            "kneesocks",
        ],
        &["靴下", "ソックス", "タイツ", "ストッキング", "ニーソ"],
    ),
    rule(
        Label::Bottomwear,
        &[
            "bottomwear",
            "skirt",
            "pants",
            "trousers",
            "shorts",
            "jeans",
        ],
        &["スカート", "ズボン", "パンツ", "ショーツ", "袴"],
    ),
    rule(
        Label::Topwear,
        &[
            "topwear", "shirt", "tshirt", "blouse", "jacket", "coat", "sweater", "hoodie", "vest",
            "dress", "uniform", "clothes", "clothing", "outfit", "top",
        ],
        &[
            "服",
            "シャツ",
            "ブラウス",
            "上着",
            "ジャケット",
            "コート",
            "制服",
            "衣装",
        ],
    ),
    // 髪 (目・口より先に見る。髪のハイライトを目のハイライトにしない)
    rule(
        Label::HairFront,
        &["fronthair", "hairfront", "bangs", "fringe", "forelock"],
        &["前髪"],
    ),
    rule(
        Label::HairSide,
        &[
            "sidehair",
            "hairside",
            "sidelock",
            "sidelocks",
            "sideburn",
            "sideburns",
            "sidetail",
        ],
        &["横髪", "もみあげ", "サイド髪", "触覚", "触角"],
    ),
    rule(
        Label::HairBack,
        &[
            "backhair",
            "hairback",
            "rearhair",
            "ponytail",
            "twintail",
            "twintails",
            "braid",
        ],
        &[
            "後ろ髪",
            "後髪",
            "うしろ髪",
            "ポニーテール",
            "ツインテール",
            "おさげ",
        ],
    ),
    vague(Label::HairFront, &["hair"], &["髪"]),
    // 目 (具体的なものから)
    rule(
        Label::EyeClosed,
        &[
            "eyeclose",
            "eyeclosed",
            "eyesclosed",
            "closedeye",
            "closedeyes",
            "closeeye",
            "eyeshut",
        ],
        &["閉じ目", "目閉じ", "閉眼"],
    ),
    rule(
        Label::EyeWhite,
        &["eyewhite", "eyewhites", "whiteeye", "sclera"],
        &["白目"],
    ),
    rule(
        Label::EyeHighlight,
        &[
            "eyehighlight",
            "highlight",
            "highlights",
            "eyelight",
            "catchlight",
            "hl",
        ],
        &["ハイライト"],
    ),
    rule(
        Label::Iris,
        &[
            "iris", "irides", "irises", "pupil", "pupils", "eyeball", "eyeballs",
        ],
        &["黒目", "瞳", "虹彩"],
    ),
    rule(
        Label::EyelashLower,
        &[
            "lowerlash",
            "lowerlashes",
            "eyelashlower",
            "lowereyelash",
            "lowereyelid",
            "bottomlash",
            "undereye",
        ],
        &["下まつ毛", "下睫毛", "下まつげ", "下まぶた"],
    ),
    rule(
        Label::EyelashUpper,
        &[
            "eyelash",
            "eyelashes",
            "lash",
            "lashes",
            "eyelashupper",
            "uppereyelash",
            "uppereyelid",
            "eyelid",
            "eyeline",
        ],
        &["まつ毛", "睫毛", "まつげ", "まぶた", "瞼", "アイライン"],
    ),
    rule(
        Label::Eyebrow,
        &["eyebrow", "eyebrows", "brow", "brows"],
        &["眉", "まゆ"],
    ),
    rule(Label::Eye, &["eye", "eyes"], &["目", "眼"]),
    // 口
    rule(
        Label::MouthOpen,
        &["mouthopen", "openmouth", "mouthopened"],
        &["開き口", "口開", "口_開"],
    ),
    rule(
        Label::MouthClosed,
        &["mouthclose", "mouthclosed", "closedmouth", "closemouth"],
        &["閉じ口", "口閉", "口_閉"],
    ),
    rule(
        Label::MouthInner,
        &[
            "mouthinner",
            "innermouth",
            "mouthinside",
            "insidemouth",
            "oral",
        ],
        &["口内", "口の中", "口腔"],
    ),
    rule(
        Label::Teeth,
        &["teeth", "tooth", "fang", "fangs"],
        &["歯", "牙"],
    ),
    rule(Label::Tongue, &["tongue"], &["舌"]),
    rule(Label::Mouth, &["mouth", "lip", "lips"], &["口", "唇"]),
    // 顔
    rule(
        Label::Cheek,
        &["cheek", "cheeks", "blush", "blushes"],
        &["頬", "ほほ", "ほっぺ", "チーク"],
    ),
    rule(Label::Nose, &["nose"], &["鼻"]),
    rule(Label::Ear, &["ear", "ears"], &["耳"]),
    rule(Label::Neck, &["neck"], &["首"]),
    rule(
        Label::Face,
        &["face", "head", "skinface", "facebase"],
        &["顔", "輪郭", "頭"],
    ),
    // 体
    rule(Label::Tail, &["tail"], &["尻尾", "しっぽ", "尾"]),
    rule(
        Label::Hand,
        &["hand", "hands", "finger", "fingers", "palm"],
        &["手", "指"],
    ),
    rule(
        Label::Arm,
        &["arm", "arms", "forearm", "upperarm", "shoulder"],
        &["腕", "肩"],
    ),
    rule(Label::Foot, &["foot", "feet"], &["足首", "足先"]),
    rule(
        Label::Leg,
        &["leg", "legs", "thigh", "thighs", "knee", "shin", "calf"],
        &["脚", "足", "太もも", "膝"],
    ),
    rule(
        Label::Body,
        &["body", "torso", "chest", "waist", "skin", "bust"],
        &["体", "胴", "胸", "腰", "身体"],
    ),
    // その他
    rule(
        Label::Accessory,
        &[
            "accessory",
            "accessories",
            "jewelry",
            "weapon",
            "item",
            "prop",
            "bag",
        ],
        &["アクセサリ", "小物", "武器", "持ち物"],
    ),
    rule(
        Label::Background,
        &["background", "bg", "backdrop"],
        &["背景"],
    ),
];

/// 名前から分かったこと。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct NameMatch {
    pub label: Label,
    pub confidence: f32,
}

/// 名前の語 (小文字)。区切り文字・camelCase の境目・数字で分ける。
fn tokens(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in name.chars() {
        if c.is_ascii_alphabetic() {
            if c.is_ascii_uppercase() && prev_lower && !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            cur.push(c.to_ascii_lowercase());
            prev_lower = c.is_ascii_lowercase();
        } else {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// 連続する 1〜3 語をつないだ文字列。
fn phrases(tokens: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for len in 1..=3 {
        for w in tokens.windows(len) {
            out.push(w.concat());
        }
    }
    out
}

/// レイヤー (またはグループ) の名前からラベルを決める。当たらなければ `None`。
pub(super) fn label_from_name(name: &str) -> Option<NameMatch> {
    let phrases = phrases(&tokens(name));
    RULES
        .iter()
        .find(|r| {
            r.words.iter().any(|w| phrases.iter().any(|p| p == w))
                || r.kanji.iter().any(|k| name.contains(k))
        })
        .map(|r| NameMatch {
            label: r.label,
            confidence: r.confidence,
        })
}

/// 名前から左右を決める (キャラクター自身の左右)。書かれていなければ `None`。
///
/// `l` / `left` / `r` / `right` の語と、日本語の「左」「右」を見る。See-Through の
/// `eyewhite-r` のように、名前の左右はキャラクター自身の左右とみなす。
pub(super) fn side_from_name(name: &str) -> Option<Side> {
    let tokens = tokens(name);
    let has = |words: &[&str]| tokens.iter().any(|t| words.contains(&t.as_str()));
    let left = has(&["l", "left", "lt"]) || name.contains('左');
    let right = has(&["r", "right", "rt"]) || name.contains('右');
    match (left, right) {
        (true, false) => Some(Side::Left),
        (false, true) => Some(Side::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(name: &str) -> Option<Label> {
        label_from_name(name).map(|m| m.label)
    }

    #[test]
    fn splits_separators_camel_case_and_digits() {
        assert_eq!(tokens("front hair"), ["front", "hair"]);
        assert_eq!(tokens("eyeWhite_L2"), ["eye", "white", "l"]);
        assert_eq!(tokens("EyeL"), ["eye", "l"]);
        assert_eq!(tokens("HAIR-back"), ["hair", "back"]);
    }

    #[test]
    fn specific_rules_win_over_general_ones() {
        assert_eq!(label("eyewhite-r"), Some(Label::EyeWhite));
        assert_eq!(label("eye white L"), Some(Label::EyeWhite));
        assert_eq!(label("eyeclose-l"), Some(Label::EyeClosed));
        assert_eq!(label("irides-l"), Some(Label::Iris));
        assert_eq!(label("eyelash-r"), Some(Label::EyelashUpper));
        assert_eq!(label("lower lash"), Some(Label::EyelashLower));
        assert_eq!(label("eyebrow-l"), Some(Label::Eyebrow));
        assert_eq!(label("eye-r"), Some(Label::Eye));
        assert_eq!(label("handwear-l"), Some(Label::Handwear));
        assert_eq!(label("earwear-r"), Some(Label::Earwear));
        assert_eq!(label("ears-l"), Some(Label::Ear));
        assert_eq!(label("mouth-open"), Some(Label::MouthOpen));
        assert_eq!(label("mouth-close"), Some(Label::MouthClosed));
        assert_eq!(label("back hair"), Some(Label::HairBack));
        assert_eq!(label("front hair"), Some(Label::HairFront));
    }

    #[test]
    fn hair_highlights_stay_hair() {
        // 髪のハイライトは目のハイライトではない (目の規則より髪の規則を先に見る)。
        assert_eq!(label("前髪ハイライト"), Some(Label::HairFront));
        assert_eq!(label("後ろ髪ハイライト"), Some(Label::HairBack));
        assert_eq!(label("back hair hl"), Some(Label::HairBack));
        assert_eq!(label("hair highlight"), Some(Label::HairFront));
        assert_eq!(label("髪ハイライト"), Some(Label::HairFront));
        // 部位の語のないハイライトは目のハイライトのまま。
        assert_eq!(label("highlight-l"), Some(Label::EyeHighlight));
        assert_eq!(label("目ハイライト"), Some(Label::EyeHighlight));
        // 触角 (顔の横に垂れる房) は横髪。
        assert_eq!(label("触角"), Some(Label::HairSide));
    }

    #[test]
    fn does_not_match_inside_other_words() {
        // `topwear` や `wear` の中の `ear` には当たらない
        assert_eq!(label("topwear"), Some(Label::Topwear));
        assert_eq!(label("wear"), None);
        assert_eq!(label("Layer 12"), None);
    }

    #[test]
    fn japanese_names() {
        assert_eq!(label("白目_左"), Some(Label::EyeWhite));
        assert_eq!(label("前髪"), Some(Label::HairFront));
        assert_eq!(label("後ろ髪"), Some(Label::HairBack));
        assert_eq!(label("まつ毛右"), Some(Label::EyelashUpper));
        assert_eq!(label("手袋"), Some(Label::Handwear));
        assert_eq!(side_from_name("白目_左"), Some(Side::Left));
    }

    #[test]
    fn vague_names_get_low_confidence() {
        let m = label_from_name("hair").unwrap();
        assert!(m.confidence < NAME_CONFIDENCE);
    }

    #[test]
    fn sides() {
        assert_eq!(side_from_name("irides-l"), Some(Side::Left));
        assert_eq!(side_from_name("EyeR"), Some(Side::Right));
        assert_eq!(side_from_name("right arm"), Some(Side::Right));
        assert_eq!(side_from_name("face"), None);
        assert_eq!(side_from_name("L-R"), None);
    }
}
