//! 見本のモデルに入れるモーションと表情 (T0719)。
//!
//! 中身は `animations/` の JSON (`docs/spec/file-formats.md` の `<name>.motion.json`・`<name>.expression.json`)
//! で、手で書いたもの。モーションは標準パラメータの頭と体の向き・目玉だけを動かすので、見本のアバターと
//! 着せ替えの本体で共有する。表情は使えるパラメータが違うので、モデルごとに分ける。

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

/// 見本のモーション。
pub fn motions() -> anyhow::Result<Vec<Motion>> {
    MOTIONS
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

/// モーションと表情が `model` に無いパラメータを動かさないこと (無いものはランタイムが黙って無視するので、
/// 見本では誤りにする)。
pub fn check(model: &Model, motions: &[Motion], expressions: &[Expression]) -> anyhow::Result<()> {
    let has = |id: &str| model.parameters.iter().any(|p| p.id == id);
    for motion in motions {
        for curve in &motion.curves {
            match &curve.target {
                CurveTarget::Parameter(id) if has(id.as_str()) => {}
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
        for ((file, _), motion) in MOTIONS.iter().zip(motions().unwrap()) {
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
