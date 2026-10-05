//! 見本のモデルに入れるモーションと表情 (T0719)。
//!
//! 中身は `animations/` の JSON (`docs/spec/file-formats.md` の `<name>.motion.json`・`<name>.expression.json`)
//! で、手で書いたもの。モーションは標準パラメータの頭と体の向き・目玉だけを動かすので、見本のアバターと
//! 着せ替えの本体で共有する。表情は使えるパラメータが違うので、モデルごとに分ける。着せ替えの本体には、
//! ポーズを切り替えるモーション (`wave`。パーツの不透明度のカーブ) も入れる (T0474)。

use anyhow::Context;
use ugoe_model::Model;
use ugoe_model::expression::Expression;
use ugoe_model::motion::{CurveTarget, Motion};

/// 見本で共有するモーション (名前の順)。
const MOTIONS: &[(&str, &str)] = &[
    (
        "idle.motion.json",
        include_str!("animations/motions/idle.motion.json"),
    ),
    (
        "nod.motion.json",
        include_str!("animations/motions/nod.motion.json"),
    ),
    (
        "shake.motion.json",
        include_str!("animations/motions/shake.motion.json"),
    ),
    (
        "tilt.motion.json",
        include_str!("animations/motions/tilt.motion.json"),
    ),
];

/// 着せ替えの本体 (`dress-up.ugoe`) だけのモーション (名前の順。共有するモーションの後ろに並べる)。
/// `wave` はポーズの要素 `Wave` を表示し、終わりに `HandsDown` へ戻す。
const DRESS_UP_MOTIONS: &[(&str, &str)] = &[(
    "wave.motion.json",
    include_str!("animations/dress-up/wave.motion.json"),
)];

/// 見本のアバター (`avatar.ugoe`・`avatar-lit.ugoe`) の表情 (名前の順)。
const AVATAR_EXPRESSIONS: &[(&str, &str)] = &[
    (
        "angry.expression.json",
        include_str!("animations/avatar/angry.expression.json"),
    ),
    (
        "sad.expression.json",
        include_str!("animations/avatar/sad.expression.json"),
    ),
    (
        "smile.expression.json",
        include_str!("animations/avatar/smile.expression.json"),
    ),
    (
        "surprised.expression.json",
        include_str!("animations/avatar/surprised.expression.json"),
    ),
];

/// 着せ替えの本体 (`dress-up.ugoe`) の表情 (名前の順)。レベル 2 のパラメータだけを使う。
const DRESS_UP_EXPRESSIONS: &[(&str, &str)] = &[
    (
        "sad.expression.json",
        include_str!("animations/dress-up/sad.expression.json"),
    ),
    (
        "smile.expression.json",
        include_str!("animations/dress-up/smile.expression.json"),
    ),
    (
        "surprised.expression.json",
        include_str!("animations/dress-up/surprised.expression.json"),
    ),
];

/// 見本で共有するモーション。
pub fn motions() -> anyhow::Result<Vec<Motion>> {
    parse_motions(MOTIONS)
}

/// 着せ替えの本体のモーション (共有するものと `wave`)。
pub fn dress_up_motions() -> anyhow::Result<Vec<Motion>> {
    let mut motions = motions()?;
    motions.extend(parse_motions(DRESS_UP_MOTIONS)?);
    Ok(motions)
}

fn parse_motions(files: &[(&str, &str)]) -> anyhow::Result<Vec<Motion>> {
    files
        .iter()
        .map(|(file, json)| ugoe_format::motion_from_json(json).with_context(|| file.to_string()))
        .collect()
}

/// 見本のアバターの表情。
pub fn avatar_expressions() -> anyhow::Result<Vec<Expression>> {
    expressions(AVATAR_EXPRESSIONS)
}

/// 着せ替えの本体の表情。
pub fn dress_up_expressions() -> anyhow::Result<Vec<Expression>> {
    expressions(DRESS_UP_EXPRESSIONS)
}

fn expressions(files: &[(&str, &str)]) -> anyhow::Result<Vec<Expression>> {
    files
        .iter()
        .map(|(file, json)| {
            ugoe_format::expression_from_json(json).with_context(|| file.to_string())
        })
        .collect()
}

/// モーションと表情が `model` に無いパラメータやパーツを動かさないこと (無いものはランタイムが黙って無視するので、
/// 見本では誤りにする)。
pub fn check(model: &Model, motions: &[Motion], expressions: &[Expression]) -> anyhow::Result<()> {
    let has = |id: &str| model.parameters.iter().any(|p| p.id == id);
    let has_part = |id: &str| model.parts.iter().any(|p| p.id == id);
    for motion in motions {
        for curve in &motion.curves {
            match &curve.target {
                CurveTarget::Parameter(id) if has(id.as_str()) => {}
                CurveTarget::PartOpacity(id) if has_part(id.as_str()) => {}
                target => anyhow::bail!("motion `{}` targets {target:?}", motion.name),
            }
        }
    }
    for expression in expressions {
        for item in &expression.parameters {
            if !has(item.parameter.as_str()) {
                anyhow::bail!(
                    "expression `{}` uses the unknown parameter `{}`",
                    expression.name,
                    item.parameter.as_str()
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ファイル名が中身の名前 (`<name>.motion.json`・`<name>.expression.json`) と合うこと
    /// (プロジェクトに保存すると名前がファイル名になる)。
    #[test]
    fn file_names_match_the_names() {
        let files = MOTIONS.iter().chain(DRESS_UP_MOTIONS);
        let all = dress_up_motions().unwrap();
        assert_eq!(all.len(), MOTIONS.len() + DRESS_UP_MOTIONS.len());
        for ((file, _), motion) in files.zip(all) {
            assert_eq!(*file, format!("{}.motion.json", motion.name));
        }
        for (files, expressions) in [
            (AVATAR_EXPRESSIONS, avatar_expressions().unwrap()),
            (DRESS_UP_EXPRESSIONS, dress_up_expressions().unwrap()),
        ] {
            for ((file, _), expression) in files.iter().zip(expressions) {
                assert_eq!(*file, format!("{}.expression.json", expression.name));
            }
        }
    }
}
