//! モデルの構造的整合性チェック。ランタイム・エディタ・AI 出力の受け入れ時に使う。

use crate::Model;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ValidationError {
    #[error("{kind} `{id}`: keyform count {actual} != expected {expected}")]
    KeyformCount {
        kind: &'static str,
        id: String,
        expected: usize,
        actual: usize,
    },
    #[error("mesh `{id}`: indices length {len} is not a multiple of 3")]
    IndicesNotTriangles { id: String, len: usize },
    #[error("mesh `{id}`: index {index} out of range (vertices = {vertices})")]
    IndexOutOfRange {
        id: String,
        index: u16,
        vertices: usize,
    },
    #[error("mesh `{id}`: keyform has {actual} positions, expected {expected}")]
    VertexCount {
        id: String,
        expected: usize,
        actual: usize,
    },
}

/// 検出したエラーを全て返す (最初の 1 件で止まらない)。
///
/// TODO(T0102): 参照整合性 (未定義 ID・循環親子・未定義パラメータ) の検査を追加する。
pub fn validate(model: &Model) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for mesh in &model.meshes {
        let id = mesh.id.to_string();
        let expected = mesh.keyforms.expected_len();
        if mesh.keyforms.keyforms.len() != expected {
            errors.push(ValidationError::KeyformCount {
                kind: "mesh",
                id: id.clone(),
                expected,
                actual: mesh.keyforms.keyforms.len(),
            });
        }
        if mesh.indices.len() % 3 != 0 {
            errors.push(ValidationError::IndicesNotTriangles {
                id: id.clone(),
                len: mesh.indices.len(),
            });
        }
        let vertices = mesh.uvs.len();
        if let Some(&index) = mesh.indices.iter().find(|&&i| usize::from(i) >= vertices) {
            errors.push(ValidationError::IndexOutOfRange {
                id: id.clone(),
                index,
                vertices,
            });
        }
        for kf in &mesh.keyforms.keyforms {
            if kf.positions.len() != vertices {
                errors.push(ValidationError::VertexCount {
                    id: id.clone(),
                    expected: vertices,
                    actual: kf.positions.len(),
                });
            }
        }
    }
    errors
}
