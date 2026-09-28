//! 機能の宣言と段階的な劣化。
//! 仕様: `docs/spec/file-formats.md#機能の宣言と段階的な劣化`、判断の経緯: ADR 0013・ADR 0018。
//!
//! - [`FEATURES`] はこのランタイムが知っている機能の一覧。仕様書の表と一致させる (テストで検査する)。
//! - バンドルは `FEAT` チャンクで、基準の機能以外に使っている機能を宣言する ([`FeatureDecl`])。
//! - 読み込みでは、知らない必須の機能があれば拒否し、知らない任意の機能は警告にする。

use serde::{Deserialize, Serialize};

use crate::Bundle;

/// このランタイムが知っている機能 1 つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Feature {
    /// 安定した機能 ID (例: `mask.intersection`)。
    pub id: &'static str,
    /// 内容 (仕様書の表の「内容」から、他の文書へのリンクを除いたもの)。
    pub description: &'static str,
    /// 知らないランタイムでの扱い (仕様書の表の「フォールバック」)。
    pub fallback: &'static str,
    /// 1.0 の基準の機能か。基準の機能は宣言しない (ADR 0018)。
    pub baseline: bool,
}

/// 知っている機能の一覧。`docs/spec/file-formats.md` の「機能とフォールバックの一覧」と同じ順。
///
/// 1.0 以降に機能を追加するときは、仕様書の「1.0 以降の機能」の表と、ここに `baseline: false` で足し、
/// [`detect`] にその機能を使っているかの判定を足す。
pub const FEATURES: &[Feature] = &[
    Feature {
        id: "authoring.keyform_origins",
        description: "キーフォームの由来と推定の設定",
        fallback: "読み飛ばす (ランタイムに影響しない)",
        baseline: true,
    },
    Feature {
        id: "mask.intersection",
        description: "マスクの共通部分",
        fallback: "和集合として扱う",
        baseline: true,
    },
    Feature {
        id: "physics.dynamic",
        description: "動的物理の出力先と衝突体",
        fallback: "パラメータ以外への出力と衝突体を無視する",
        baseline: true,
    },
    Feature {
        id: "lighting.normal_map",
        description: "法線マップとライティング",
        fallback: "ライティングなしで描く",
        baseline: true,
    },
    Feature {
        id: "modules",
        description: "着せ替えのスロット",
        fallback: "スロットを無視し、本体だけを表示する",
        baseline: true,
    },
    Feature {
        id: "avatar.profile",
        description: "アバタープロファイル",
        fallback: "読み飛ばす (アプリの連携機能が使えないだけ)",
        baseline: true,
    },
];

/// ID から知っている機能を探す。
pub fn find(id: &str) -> Option<&'static Feature> {
    FEATURES.iter().find(|f| f.id == id)
}

/// `FEAT` チャンクの 1 項目: バンドルが使う機能の宣言。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureDecl {
    pub id: String,
    /// `true` なら、知らないランタイムは読み込みを拒否する。`false` ならフォールバックを適用して読む。
    #[serde(default)]
    pub required: bool,
}

impl FeatureDecl {
    /// 任意の機能の宣言。
    pub fn optional(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            required: false,
        }
    }

    /// 必須の機能の宣言。
    pub fn required(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            required: true,
        }
    }
}

/// `FEAT` チャンクのペイロード (MessagePack、フィールド名付き)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FeatureTable {
    #[serde(default)]
    pub(crate) features: Vec<FeatureDecl>,
}

/// 書き出すバンドルが使っている機能の宣言を作る (書き出し時の自動検出)。
///
/// 基準の機能 ([`Feature::baseline`]) は宣言しない (ADR 0018)。1.0 の時点ではすべての機能が
/// 基準なので、宣言は常に空になる。必須 (`required`) の宣言を返したときは、書き出すツールが
/// 利用者に警告する (ADR 0013)。
pub fn detect(bundle: &Bundle) -> Vec<FeatureDecl> {
    // 1.0 以降の機能を足したら、ここで `bundle` がその機能を使っているかを調べて宣言を足す。
    let _ = bundle;
    Vec::new()
}

/// 宣言を、知っている機能の判定 `known` で調べる。
///
/// 知らない必須の機能があれば、その ID の一覧を `Err` で返す。
/// そうでなければ、知らない任意の機能の ID の一覧を `Ok` で返す (どちらも重複なし・宣言順)。
/// 同じ ID が必須と任意の両方で宣言されていれば、必須として扱う。
pub(crate) fn check(
    declared: &[FeatureDecl],
    known: impl Fn(&str) -> bool,
) -> Result<Vec<String>, Vec<String>> {
    let mut missing: Vec<String> = Vec::new();
    let mut optional: Vec<String> = Vec::new();
    for decl in declared.iter().filter(|d| !known(&d.id)) {
        let list = if declared.iter().any(|d| d.id == decl.id && d.required) {
            &mut missing
        } else {
            &mut optional
        };
        if !list.contains(&decl.id) {
            list.push(decl.id.clone());
        }
    }
    if missing.is_empty() {
        Ok(optional)
    } else {
        Err(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 仕様書の `heading` の直後にある表の行を、セルの列として返す (見出し行と区切り行を除く)。
    fn spec_table(spec: &str, heading: &str) -> Vec<Vec<String>> {
        let start = spec.find(heading).expect("heading");
        spec[start..]
            .lines()
            .skip_while(|line| !line.starts_with('|'))
            .take_while(|line| line.starts_with('|'))
            .skip(2)
            .map(|line| {
                line.trim_matches('|')
                    .split('|')
                    .map(|cell| cell.trim().to_owned())
                    .collect()
            })
            .collect()
    }

    /// 表のセルから `` ` `` と、他の文書へのリンク ` ([...](...))` を除く。
    fn plain(cell: &str) -> String {
        let mut text = cell.replace('`', "");
        while let Some(start) = text.find(" ([") {
            let end = text[start..].find("))").expect("link") + start + 2;
            text.replace_range(start..end, "");
        }
        text
    }

    fn registry_rows(baseline: bool) -> Vec<[String; 3]> {
        FEATURES
            .iter()
            .filter(|f| f.baseline == baseline)
            .map(|f| [f.id.into(), f.description.into(), f.fallback.into()])
            .collect()
    }

    fn spec_rows(spec: &str, heading: &str) -> Vec<[String; 3]> {
        spec_table(spec, heading)
            .into_iter()
            .filter(|row| row[0] != "(まだない)")
            .map(|row| [plain(&row[0]), plain(&row[1]), plain(&row[2])])
            .collect()
    }

    #[test]
    fn registry_matches_spec_tables() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/spec/file-formats.md"
        );
        let spec = std::fs::read_to_string(path).expect("docs/spec/file-formats.md");
        assert_eq!(
            registry_rows(true),
            spec_rows(&spec, "### 機能とフォールバックの一覧")
        );
        assert_eq!(
            registry_rows(false),
            spec_rows(&spec, "#### 1.0 以降の機能")
        );
    }

    #[test]
    fn ids_are_unique_and_found() {
        for feature in FEATURES {
            assert_eq!(find(feature.id), Some(feature));
        }
        assert_eq!(find("test.unknown"), None);
    }

    #[test]
    fn baseline_bundles_declare_nothing() {
        let bundle = Bundle::new(ugoe_model::fixtures::single_quad());
        assert!(detect(&bundle).is_empty());
    }

    #[test]
    fn check_splits_unknown_features() {
        let known = |id: &str| id == "test.known";
        let declared = [
            FeatureDecl::optional("test.known"),
            FeatureDecl::required("test.known"),
            FeatureDecl::optional("test.optional"),
            FeatureDecl::optional("test.optional"),
        ];
        assert_eq!(check(&declared, known), Ok(vec!["test.optional".into()]));

        let declared = [
            FeatureDecl::optional("test.optional"),
            FeatureDecl::required("test.required"),
            FeatureDecl::optional("test.both"),
            FeatureDecl::required("test.both"),
        ];
        assert_eq!(
            check(&declared, known),
            Err(vec!["test.required".into(), "test.both".into()])
        );
    }
}
