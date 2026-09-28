//! 新しい要素に振る ID の生成。ID の形式は `docs/spec/model.md#ID` (`[A-Za-z0-9_]+`)。
//!
//! UI や取り込み (PSD・AI) が要素を追加するときに、`Add*` コマンドを作る前に使う。

use ugoe_model::{DeformerId, MeshId, Model, ParamId, PartId};

/// 元の名前が ID に使える文字を 1 つも含まないときの ID。
const FALLBACK: &str = "Item";

/// `base` を ID の形式に直す。`[A-Za-z0-9_]` 以外の文字は `_` にする。
/// 英数字を 1 つも含まなければ `Item` を返す。
pub fn sanitize_id(base: &str) -> String {
    if !base.chars().any(|c| c.is_ascii_alphanumeric()) {
        return FALLBACK.to_owned();
    }
    base.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// `base` を ID の形式に直し、`existing` と重ならない ID を返す。
///
/// 重なる場合は `_2`、`_3`、… を順に付けて最初に空いているものを使う。
pub fn unique_id<'a>(base: &str, existing: impl IntoIterator<Item = &'a str>) -> String {
    let base = sanitize_id(base);
    let existing: std::collections::HashSet<&str> = existing.into_iter().collect();
    if !existing.contains(base.as_str()) {
        return base;
    }
    (2u64..)
        .map(|n| format!("{base}_{n}"))
        .find(|id| !existing.contains(id.as_str()))
        .unwrap_or(base)
}

/// モデル内で未使用のパラメータ ID。
pub fn new_parameter_id(model: &Model, base: &str) -> ParamId {
    ParamId::new(unique_id(
        base,
        model.parameters.iter().map(|p| p.id.as_str()),
    ))
}

/// モデル内で未使用のパーツ ID。
pub fn new_part_id(model: &Model, base: &str) -> PartId {
    PartId::new(unique_id(base, model.parts.iter().map(|p| p.id.as_str())))
}

/// モデル内で未使用のデフォーマ ID。
pub fn new_deformer_id(model: &Model, base: &str) -> DeformerId {
    DeformerId::new(unique_id(
        base,
        model.deformers.iter().map(|d| d.id.as_str()),
    ))
}

/// モデル内で未使用のメッシュ ID。
pub fn new_mesh_id(model: &Model, base: &str) -> MeshId {
    MeshId::new(unique_id(base, model.meshes.iter().map(|m| m.id.as_str())))
}

#[cfg(test)]
mod tests {
    use ugoe_model::fixtures;

    use super::*;

    #[test]
    fn sanitize_replaces_invalid_characters() {
        assert_eq!(sanitize_id("Eye L-open"), "Eye_L_open");
        assert_eq!(sanitize_id("前髪 1"), "___1");
        assert_eq!(sanitize_id("前髪"), "Item");
        assert_eq!(sanitize_id(""), "Item");
    }

    #[test]
    fn unique_id_appends_counter() {
        assert_eq!(unique_id("A", ["B"]), "A");
        assert_eq!(unique_id("A", ["A", "A_2"]), "A_3");
    }

    #[test]
    fn model_helpers_avoid_existing_ids() {
        let model = fixtures::single_quad();
        assert_eq!(new_parameter_id(&model, "X").as_str(), "X_2");
        assert_eq!(new_part_id(&model, "Root").as_str(), "Root_2");
        assert_eq!(new_deformer_id(&model, "Root").as_str(), "Root");
        assert_eq!(new_mesh_id(&model, "Quad").as_str(), "Quad_2");
    }
}
