//! 読み込みの警告の種類 (`code`) と補足の値。wasm の `load_warnings_json` と C ABI の
//! `ugoe_puppet_load_warning_*` はここの値を使う (`crates/ugoe-wasm/README.md#読み込みの警告-t0125`)。
//!
//! どの警告を出すかは組み込み先が決める (画像のデコードをしない C ABI は KTX2 の警告を出さないなど)。

use std::fmt;

use ugoe_format::ReadWarning;

/// 読み込めたが注意が要ること。`Display` は英語の文で、安定した値として扱わない
/// (wasm の読み込みの戻り値に入る文。KTX2 の 2 つは T0222 で、対応しない形式だけに出すことに合わせて変えた)。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoadWarning {
    /// テクスチャが、デコードしない形式 (ETC2・ASTC・Basis Universal など) の KTX2 なので、白のプレースホルダで代用した。
    #[error(
        "texture {texture} is KTX2 in a format that is not supported yet; using a white placeholder"
    )]
    TextureKtx2 {
        /// テクスチャの番号。
        texture: usize,
    },
    /// テクスチャの画像が無いので、白のプレースホルダで代用した。
    #[error("no data for texture {texture}; using a white placeholder")]
    TextureMissing {
        /// テクスチャの番号。
        texture: usize,
    },
    /// 法線マップが、デコードしない形式 (ETC2・ASTC・Basis Universal など) の KTX2 なので、法線マップなしで描く。
    #[error(
        "the normal map of texture {texture} is KTX2 in a format that is not supported yet; drawing without it"
    )]
    NormalMapKtx2 {
        /// 法線マップを持つテクスチャの番号。
        texture: usize,
    },
    /// 読み直す前の物理の設定が新しいモデルと合わないので外した。`reason` は検査の結果 (英語)。
    #[error("physics settings were removed: {reason}")]
    PhysicsRemoved {
        /// 検査の結果 (英語)。
        reason: String,
        /// 検査のエラーの種類 ([`ValidationError::code`](ugoe_model::validate::ValidationError::code)。エラーの順で、重なりを除かない)。
        /// 外した理由が検査のエラーでなければ空。
        codes: Vec<&'static str>,
    },
    /// 知らない任意の機能を宣言したファイル。仕様の定めるフォールバックで読んだ
    /// (`ugoe_format::ReadWarning::UnknownOptionalFeature`)。
    #[error("unknown optional feature {feature}")]
    UnknownOptionalFeature {
        /// 機能の名前。
        feature: String,
    },
    /// 知らない列挙の値をフォールバックで読んだ。`path` はチャンクとその中の場所
    /// (`ugoe_format::ReadWarning::UnknownValue`)。
    #[error("unknown value at {path}")]
    UnknownValue {
        /// チャンクとその中の場所。
        path: String,
    },
}

/// 警告 ([`LoadWarning::fields`]) と着せ替えの失敗 ([`crate::DressUpError::fields`]) の補足の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue<'a> {
    /// 番号 (テクスチャの番号など)。
    Index(usize),
    /// 文 (理由・機能の名前・場所など)。
    Text(&'a str),
    /// 種類 (`code`) の一覧 (検査のエラーの種類など。出た順で、重なりを除いたもの)。
    /// 組み込み先 (wasm・C ABI) には、`Display` と同じ `, ` でつないだ文として出す (T0168)。
    Codes(Vec<&'static str>),
}

impl FieldValue<'_> {
    /// 検査のエラーの種類の一覧 ([`ValidationError::code`](ugoe_model::validate::ValidationError::code) を出た順に、重なりを除いて並べる)。
    pub fn validation_codes(codes: impl IntoIterator<Item = &'static str>) -> Self {
        let mut list: Vec<&'static str> = Vec::new();
        for code in codes {
            if !list.contains(&code) {
                list.push(code);
            }
        }
        Self::Codes(list)
    }
}

impl fmt::Display for FieldValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Index(i) => write!(f, "{i}"),
            Self::Text(s) => f.write_str(s),
            Self::Codes(codes) => f.write_str(&codes.join(", ")),
        }
    }
}

impl LoadWarning {
    /// 機械が読む種類。値は組み込み先が照らし合わせるので変えない。
    pub fn code(&self) -> &'static str {
        match self {
            Self::TextureKtx2 { .. } => "texture_ktx2",
            Self::TextureMissing { .. } => "texture_missing",
            Self::NormalMapKtx2 { .. } => "normal_map_ktx2",
            Self::PhysicsRemoved { .. } => "physics_removed",
            Self::UnknownOptionalFeature { .. } => "unknown_optional_feature",
            Self::UnknownValue { .. } => "unknown_value",
        }
    }

    /// 補足の値 (キーと値)。キーは `texture`・`reason`・`feature`・`path`。`physics_removed` は、外した理由が
    /// 検査のエラーなら `errors` (数) と `error_codes` (種類の一覧) も持つ (着せ替えの `invalid_physics` と同じ。T0168)。
    pub fn fields(&self) -> Vec<(&'static str, FieldValue<'_>)> {
        match self {
            Self::TextureKtx2 { texture }
            | Self::TextureMissing { texture }
            | Self::NormalMapKtx2 { texture } => vec![("texture", FieldValue::Index(*texture))],
            Self::PhysicsRemoved { reason, codes } => {
                let mut fields = vec![("reason", FieldValue::Text(reason))];
                if !codes.is_empty() {
                    fields.push(("errors", FieldValue::Index(codes.len())));
                    fields.push((
                        "error_codes",
                        FieldValue::validation_codes(codes.iter().copied()),
                    ));
                }
                fields
            }
            Self::UnknownOptionalFeature { feature } => {
                vec![("feature", FieldValue::Text(feature))]
            }
            Self::UnknownValue { path } => vec![("path", FieldValue::Text(path))],
        }
    }
}

impl From<&ReadWarning> for LoadWarning {
    fn from(warning: &ReadWarning) -> Self {
        match warning {
            ReadWarning::UnknownOptionalFeature(feature) => Self::UnknownOptionalFeature {
                feature: feature.clone(),
            },
            ReadWarning::UnknownValue(path) => Self::UnknownValue { path: path.clone() },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_warnings_keep_the_format_crate_text() {
        let feature = ReadWarning::UnknownOptionalFeature("test.optional".into());
        let warning = LoadWarning::from(&feature);
        assert_eq!(warning.code(), "unknown_optional_feature");
        assert_eq!(
            warning.fields(),
            [("feature", FieldValue::Text("test.optional"))]
        );
        assert_eq!(warning.to_string(), feature.to_string());

        let value = ReadWarning::UnknownValue("MODL/meshes/0/blend".into());
        let warning = LoadWarning::from(&value);
        assert_eq!(warning.code(), "unknown_value");
        assert_eq!(
            warning.fields(),
            [("path", FieldValue::Text("MODL/meshes/0/blend"))]
        );
        assert_eq!(warning.to_string(), value.to_string());
    }

    #[test]
    fn texture_fields_are_indices() {
        let warning = LoadWarning::TextureMissing { texture: 3 };
        assert_eq!(warning.fields(), [("texture", FieldValue::Index(3))]);
        assert_eq!(warning.fields()[0].1.to_string(), "3");
    }

    #[test]
    fn removed_physics_lists_the_validation_codes() {
        let warning = LoadWarning::PhysicsRemoved {
            reason: "r".into(),
            codes: vec!["unknown_id", "duplicate_id", "unknown_id"],
        };
        let fields = warning.fields();
        assert_eq!(
            fields,
            [
                ("reason", FieldValue::Text("r")),
                ("errors", FieldValue::Index(3)),
                (
                    "error_codes",
                    FieldValue::Codes(vec!["unknown_id", "duplicate_id"])
                ),
            ]
        );
        assert_eq!(fields[2].1.to_string(), "unknown_id, duplicate_id");

        let warning = LoadWarning::PhysicsRemoved {
            reason: "r".into(),
            codes: vec![],
        };
        assert_eq!(warning.fields(), [("reason", FieldValue::Text("r"))]);
    }
}
