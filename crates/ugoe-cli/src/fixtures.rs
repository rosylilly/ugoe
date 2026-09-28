//! 組み込みフィクスチャ (`ugoe_model::fixtures`) の名前表。

use ugoe_model::{Model, fixtures};

type Fixture = (&'static str, fn() -> Model);

/// フィクスチャ名 (関数名と同じ snake_case) と生成関数。
pub const FIXTURES: &[Fixture] = &[
    ("single_quad", fixtures::single_quad),
    ("pose_and_effects", fixtures::pose_and_effects),
    ("warp_grid", fixtures::warp_grid),
    ("rotation_chain", fixtures::rotation_chain),
    ("rotation_under_warp", fixtures::rotation_under_warp),
    ("part_opacity_hierarchy", fixtures::part_opacity_hierarchy),
    ("glued_quads", fixtures::glued_quads),
    ("nested_masks", fixtures::nested_masks),
    ("mixed_masks", fixtures::mixed_masks),
    ("intersection_masks", fixtures::intersection_masks),
    ("avatar_profile", fixtures::avatar_profile),
];

/// 名前からフィクスチャを作る。`single-quad` のような kebab-case も受け付ける。
pub fn by_name(name: &str) -> anyhow::Result<Model> {
    let normalized = name.replace('-', "_");
    FIXTURES
        .iter()
        .find(|(n, _)| *n == normalized)
        .map(|(_, f)| f())
        .ok_or_else(|| anyhow::anyhow!("unknown fixture `{name}` (known: {})", names().join(", ")))
}

pub fn names() -> Vec<&'static str> {
    FIXTURES.iter().map(|(n, _)| *n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_fixtures_are_listed_and_valid() {
        for (name, f) in FIXTURES {
            let model = f();
            assert!(
                ugoe_model::validate::validate(&model).is_empty(),
                "fixture {name} is invalid"
            );
        }
    }

    #[test]
    fn accepts_snake_and_kebab_case() {
        assert_eq!(by_name("single_quad").unwrap().meta.name, "single_quad");
        assert_eq!(by_name("single-quad").unwrap().meta.name, "single_quad");
        assert_eq!(by_name("nested-masks").unwrap().meshes.len(), 3);
    }

    #[test]
    fn unknown_name_lists_known_names() {
        let err = by_name("nope").unwrap_err().to_string();
        assert!(err.contains("single_quad"), "{err}");
        assert!(err.contains("intersection_masks"), "{err}");
    }
}
