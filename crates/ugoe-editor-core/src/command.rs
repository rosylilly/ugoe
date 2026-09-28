use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ugoe_model::deformer::{RotationKeyform, WarpKeyform};
use ugoe_model::mesh::MeshKeyform;
use ugoe_model::part::PartKeyform;
use ugoe_model::{
    BlendMode, Deformer, DeformerId, KeyformGrid, MaskMode, MaskRef, Mesh, MeshId, ParamId,
    Parameter, Part, PartId,
};

/// キーフォーム格子の 1 セルの指定: 軸のパラメータ ID → その軸のキーの値。
///
/// 格子のすべての軸を 1 回ずつ指定し、値は `KeyformBinding::keys` のどれかと完全に一致させる。
/// 軸が 0 本の格子 (定数) は空の指定で唯一のセルを指す。
pub type KeyformCell = BTreeMap<ParamId, f32>;

/// 編集操作。JSON で表現でき、AI エージェントのツール呼び出しにそのまま対応する。
///
/// 追加時のルール (`docs/editor.md#コマンド`):
/// - 1 コマンド = ユーザーから見た 1 操作 (Undo の単位)。
/// - フィールドは ID で対象を指定する (インデックスは使わない)。
/// - 適用前検証を [`crate::Document::apply`] 内で行い、失敗時はモデルを一切変更しない。
/// - 新しいバリアントは末尾に追加する。
///
/// 削除時の連鎖 (カスケード) と拒否の規則は `docs/editor.md#コマンド` にある。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Command {
    AddParameter {
        parameter: Parameter,
    },
    /// パラメータを削除する。このパラメータを軸に持つキーフォーム格子は、その軸を
    /// 既定値のキー (なければ最も近いキー) の断面に畳む。ミラーとエフェクトの参照も外す。
    RemoveParameter {
        id: ParamId,
    },
    SetParameterRange {
        id: ParamId,
        min: f32,
        default: f32,
        max: f32,
    },
    /// 複数コマンドを 1 つの Undo 単位にまとめる (AI の一括提案など)。
    Batch {
        label: String,
        commands: Vec<Command>,
    },
    /// パラメータの表示名を変える (ID は変えない)。
    RenameParameter {
        id: ParamId,
        name: String,
    },
    /// パラメータグループを設定する (`None` で外す)。グループは `parameter_groups` に存在すること。
    SetParameterGroup {
        id: ParamId,
        group: Option<String>,
    },
    /// 左右で対になるパラメータを設定する (`None` で外す)。相手側は変更しない。
    /// 自分自身を指定すると、値の符号を反転して対応させる (`docs/spec/keyform-inference.md#ミラー`)。
    SetParameterMirror {
        id: ParamId,
        mirror: Option<ParamId>,
    },
    AddPart {
        part: Part,
    },
    /// パーツを削除する。子パーツ・所属するデフォーマ・メッシュが残っていれば拒否する。
    /// ポーズからは取り除く (空になったグループも消す)。
    RemovePart {
        id: PartId,
    },
    RenamePart {
        id: PartId,
        name: String,
    },
    SetPartParent {
        id: PartId,
        parent: Option<PartId>,
    },
    SetPartKeyforms {
        id: PartId,
        keyforms: KeyformGrid<PartKeyform>,
    },
    AddDeformer {
        deformer: Deformer,
    },
    /// デフォーマを削除する。子 (デフォーマ・メッシュ) が残っていれば拒否する。
    RemoveDeformer {
        id: DeformerId,
    },
    RenameDeformer {
        id: DeformerId,
        name: String,
    },
    /// 親デフォーマを付け替える。子の座標はそのまま (新しい親の局所座標として解釈される)。
    SetDeformerParent {
        id: DeformerId,
        parent: Option<DeformerId>,
    },
    SetDeformerPart {
        id: DeformerId,
        part: PartId,
    },
    /// ワープデフォーマのキーフォーム格子を置き換える (`rows` / `cols` は変えない)。
    SetWarpKeyforms {
        id: DeformerId,
        keyforms: KeyformGrid<WarpKeyform>,
    },
    SetRotationKeyforms {
        id: DeformerId,
        keyforms: KeyformGrid<RotationKeyform>,
    },
    /// メッシュを `meshes` の末尾 (同じ描画順の中で最も手前) に追加する。
    AddMesh {
        mesh: Mesh,
    },
    /// メッシュを削除する。他のメッシュのマスク参照から外し、
    /// このメッシュを使うグルーと当たり判定を削除する。
    RemoveMesh {
        id: MeshId,
    },
    RenameMesh {
        id: MeshId,
        name: String,
    },
    /// メッシュの属性を変える。省略した (`null` の) フィールドは変更しない。
    SetMeshProperties {
        id: MeshId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        part: Option<PartId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        texture: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blend: Option<BlendMode>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        culling: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        masks: Option<Vec<MaskRef>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mask_mode: Option<MaskMode>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mask_only: Option<bool>,
    },
    /// 親デフォーマを付け替える。頂点座標はそのまま (新しい親の局所座標として解釈される)。
    SetMeshParent {
        id: MeshId,
        parent: Option<DeformerId>,
    },
    SetMeshKeyforms {
        id: MeshId,
        keyforms: KeyformGrid<MeshKeyform>,
    },
    /// メッシュのキーフォーム格子の 1 セルだけを置き換える。
    SetMeshKeyform {
        id: MeshId,
        cell: KeyformCell,
        keyform: MeshKeyform,
    },
    /// `meshes` の中でメッシュを `before` の直前へ移す (`None` なら末尾)。
    /// 描画順 (`draw_order`) が同じメッシュどうしの前後関係が変わる。
    ReorderMesh {
        id: MeshId,
        before: Option<MeshId>,
    },
}
