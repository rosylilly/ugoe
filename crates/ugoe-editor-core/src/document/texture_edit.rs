//! テクスチャと法線マップのコマンド (画像の表は `crate::texture`)。

use std::sync::Arc;

use ugoe_model::{Model, TextureRef};

use super::EditError;
use super::lookup::{ensure_no_children, find_mut};
use crate::texture::{Textures, is_valid_texture_name, png_size};

/// PNG の寸法を読む。PNG でなければ [`EditError::NotPng`]。
fn png_dimensions(name: &str, png: &[u8]) -> Result<(u32, u32), EditError> {
    png_size(png).ok_or_else(|| EditError::NotPng {
        name: name.to_owned(),
    })
}

/// テクスチャを `Model::textures` の末尾に追加する。寸法は PNG から読む。
///
/// 名前は大文字と小文字を区別せずに重複を拒否する (大文字小文字を区別しないファイルシステムで
/// `textures/<name>.png` が衝突しないように)。法線マップの画像も同じ場所に置くので、その名前とも比べる。
pub(super) fn add_texture(
    model: &mut Model,
    textures: &mut Textures,
    name: String,
    png: Vec<u8>,
) -> Result<(), EditError> {
    if !is_valid_texture_name(&name) {
        return Err(EditError::InvalidTextureName { name });
    }
    if model.textures.iter().any(|t| {
        t.name.eq_ignore_ascii_case(&name)
            || t.normal_map
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case(&name))
    }) {
        return Err(EditError::DuplicateId {
            kind: "texture",
            id: name,
        });
    }
    let (width, height) = png_dimensions(&name, &png)?;
    model.textures.push(TextureRef {
        name: name.clone(),
        width,
        height,
        normal_map: None,
    });
    textures.insert(name, Arc::from(png));
    Ok(())
}

/// テクスチャの画像を差し替える。`TextureRef` の寸法も PNG に合わせて更新する。
pub(super) fn set_texture_image(
    model: &mut Model,
    textures: &mut Textures,
    name: String,
    png: Vec<u8>,
) -> Result<(), EditError> {
    let (width, height) = png_dimensions(&name, &png)?;
    let texture = find_mut(&mut model.textures, "texture", &name, |t| &t.name)?;
    (texture.width, texture.height) = (width, height);
    textures.insert(name, Arc::from(png));
    Ok(())
}

/// 名前 (`TextureRef::name`) のテクスチャの番号 (`Mesh::texture` の値)。なければ [`EditError::NotFound`]。
pub(super) fn texture_index(model: &Model, name: &str) -> Result<u32, EditError> {
    model
        .textures
        .iter()
        .position(|t| t.name == name)
        .and_then(|i| u32::try_from(i).ok())
        .ok_or_else(|| EditError::NotFound {
            kind: "texture",
            id: name.to_owned(),
        })
}

/// テクスチャを削除する。使っているメッシュがあれば拒否する。
/// 後ろのテクスチャを指すメッシュの `texture` (インデックス) は 1 つ詰める。
pub(super) fn remove_texture(
    model: &mut Model,
    textures: &mut Textures,
    name: &str,
) -> Result<(), EditError> {
    let index = model
        .textures
        .iter()
        .position(|t| t.name == name)
        .ok_or_else(|| EditError::NotFound {
            kind: "texture",
            id: name.to_owned(),
        })?;
    let children = model
        .meshes
        .iter()
        .filter(|m| usize::try_from(m.texture).is_ok_and(|t| t == index))
        .map(|m| format!("mesh `{}`", m.id))
        .collect();
    ensure_no_children("texture", &name, children)?;
    let removed = model.textures.remove(index);
    for mesh in &mut model.meshes {
        if usize::try_from(mesh.texture).is_ok_and(|t| t > index) {
            mesh.texture -= 1;
        }
    }
    textures.remove(name);
    if let Some(normal_map) = removed.normal_map {
        drop_unused_normal_map(model, textures, &normal_map);
    }
    Ok(())
}

/// [`crate::Command::SetNormalMap`]。
pub(super) fn set_normal_map(
    model: &mut Model,
    textures: &mut Textures,
    texture: &str,
    normal_map: Option<String>,
    png: Option<Vec<u8>>,
) -> Result<(), EditError> {
    let index = model
        .textures
        .iter()
        .position(|t| t.name == texture)
        .ok_or_else(|| EditError::NotFound {
            kind: "texture",
            id: texture.to_owned(),
        })?;
    let invalid = |reason: String| EditError::InvalidNormalMap {
        texture: texture.to_owned(),
        reason,
    };
    match (&normal_map, png) {
        (None, Some(_)) => {
            return Err(invalid("an image was given without a name".into()));
        }
        (None, None) => {}
        (Some(name), png) => {
            if !is_valid_texture_name(name) {
                return Err(EditError::InvalidTextureName { name: name.clone() });
            }
            // テクスチャの画像と同じ場所 (`textures/<name>.png`) に置くので、テクスチャの名前と重ならず、
            // ほかの法線マップの名前とは大文字小文字まで一致すること。
            if model
                .textures
                .iter()
                .any(|t| t.name.eq_ignore_ascii_case(name))
            {
                return Err(EditError::DuplicateId {
                    kind: "texture",
                    id: name.clone(),
                });
            }
            let shared = model
                .textures
                .iter()
                .filter_map(|t| t.normal_map.as_deref())
                .find(|n| n.eq_ignore_ascii_case(name));
            if let Some(existing) = shared
                && existing != name
            {
                return Err(invalid(format!(
                    "`{name}` differs only in case from the normal map `{existing}`"
                )));
            }
            match png {
                Some(png) => {
                    png_dimensions(name, &png)?;
                    textures.insert(name.clone(), Arc::from(png));
                }
                None if shared.is_some() && textures.contains_key(name.as_str()) => {}
                None => {
                    return Err(EditError::NotFound {
                        kind: "normal map",
                        id: name.clone(),
                    });
                }
            }
        }
    }
    let previous = std::mem::replace(&mut model.textures[index].normal_map, normal_map);
    if let Some(previous) = previous {
        drop_unused_normal_map(model, textures, &previous);
    }
    Ok(())
}

/// どのテクスチャからも指されなくなった法線マップの画像を表から除く。
fn drop_unused_normal_map(model: &Model, textures: &mut Textures, name: &str) {
    let used = model
        .textures
        .iter()
        .any(|t| t.name == name || t.normal_map.as_deref() == Some(name));
    if !used {
        textures.remove(name);
    }
}
