//! 着せ替えのスロットとモジュール (`docs/spec/modules.md`)。
//!
//! 本体のモデルはスロット ([`Slot`]、`Model::slots`) を公開し、モジュール ([`Module`]) は
//! スロットの名前とパラメータの ID だけに依存する部分的なモデルとして作る。
//! 本体とモジュールの合成は `ugoe_runtime::compose` が行う。

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use ugoe_core::Id;

use crate::deformer::DeformerKind;
use crate::physics::{ColliderId, PhysicsCollider, PhysicsRig, PhysicsTarget};
use crate::pose::{PoseEntry, PoseGroup};
use crate::{
    Canvas, Deformer, DeformerId, Effects, Glue, Mesh, MeshId, Model, ModelMeta, ParamId,
    Parameter, Part, PartId, Pose, TextureRef,
};

pub type SlotId = Id<Slot>;

/// 着せ替えのスロット: モジュールの取り付け位置 (`docs/spec/modules.md#スロット`)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Slot {
    /// スロットの名前 (例: `Hair`)。モジュールはこの名前だけで取り付け先を指定する。
    pub id: SlotId,
    /// 取り付け先のデフォーマ。`None` ならキャンバス。
    /// モジュールの最上位の要素の座標は、このデフォーマの局所座標系で表す。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<DeformerId>,
    /// このスロットに入るメッシュの描画順の範囲。モジュールの描画順 (0〜1000) をこの範囲に写す。
    pub draw_order: DrawOrderRange,
    /// モジュールを取り付けたときに非表示にする本体のパーツ。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replaces: Vec<PartId>,
    /// このスロットのモジュールが使ってよい本体のパラメータ。標準パラメータは常に使える。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParamId>,
    /// このスロットのモジュールがマスクとして参照してよい本体のメッシュ。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mask_sources: Vec<MeshId>,
    /// モジュールの最上位のパーツの親にする本体のパーツ。`None` ならモジュールの最上位のパーツは最上位のまま。
    /// 親のパーツの不透明度 (ポーズやモーションで隠したときなど) にモジュールが追従する。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<PartId>,
    /// このスロットのモジュールの物理のリグが衝突してよい本体の衝突体 (`PhysicsCollider::id`)。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colliders: Vec<ColliderId>,
    /// このスロットのモジュールがパーツを追従 (`PoseEntry::links`) させてよい、本体のポーズの要素のパーツ。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pose_parts: Vec<PartId>,
}

/// 描画順の範囲 `[min, max]` (`min <= max`)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DrawOrderRange {
    pub min: f32,
    pub max: f32,
}

/// 着せ替えのモジュール: スロットに取り付ける部分的なモデル (`docs/spec/modules.md#モジュール`)。
///
/// 中の ID はモジュールの中でだけ一意で、合成時に `<name>/<ID>` に付け替える。
/// パラメータは定義せず、標準パラメータとスロットが公開するパラメータを ID で参照する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Module {
    pub schema_version: u32,
    /// モジュールの名前。ID の付け替えに使うので、合成するモジュールの間で一意にする。
    /// 空でなく、`/` を含まず、`#` で始まらないこと。
    pub name: String,
    /// 取り付け先のスロットの名前。
    pub slot: SlotId,
    /// 対応する本体の条件。
    #[serde(default)]
    pub requires: ModuleRequirements,
    #[serde(default)]
    pub parts: Vec<Part>,
    #[serde(default)]
    pub deformers: Vec<Deformer>,
    #[serde(default)]
    pub meshes: Vec<Mesh>,
    #[serde(default)]
    pub glues: Vec<Glue>,
    /// テクスチャの参照。メッシュの `texture` はこの列のインデックス。
    #[serde(default)]
    pub textures: Vec<TextureRef>,
    /// 物理のリグと衝突体。重力・風・周波数は本体の設定を使う。
    #[serde(default, skip_serializing_if = "ModulePhysics::is_empty")]
    pub physics: ModulePhysics,
    /// ポーズ (排他的なパーツの切り替え)。
    #[serde(default, skip_serializing_if = "ModulePose::is_empty")]
    pub pose: ModulePose,
}

/// モジュールが対応する本体の条件。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ModuleRequirements {
    /// 本体に求めるアバタープロファイルのレベル (`Model::avatar` の `level` 以上)。`None` なら求めない。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_level: Option<u8>,
    /// 本体に必要なスロット (取り付け先のスロットは書かなくてよい)。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub slots: Vec<SlotId>,
    /// 本体に必要なパラメータ。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParamId>,
}

/// モジュールの物理。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ModulePhysics {
    pub rigs: Vec<PhysicsRig>,
    /// 衝突体。モジュールのリグは、モジュールの衝突体だけを参照できる。
    pub colliders: Vec<PhysicsCollider>,
}

impl ModulePhysics {
    pub fn is_empty(&self) -> bool {
        self.rigs.is_empty() && self.colliders.is_empty()
    }
}

/// モジュールのポーズ (`docs/spec/modules.md#ポーズ`)。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ModulePose {
    /// モジュールのパーツだけでできたグループ。合成で本体のグループの後ろに加える。
    /// クロスフェードの時間は本体のもの。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<PoseGroup>,
    /// 本体のポーズの要素に追従させるモジュールのパーツ。`part` は本体のパーツ
    /// (スロットの `pose_parts` にあるもの)、`links` はモジュールのパーツ。
    /// 合成で本体の要素の `links` に加える (例: 腕の差分 A / B に合わせて袖の差分を切り替える)。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<PoseEntry>,
}

impl ModulePose {
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty() && self.links.is_empty()
    }
}

impl Module {
    /// 名前がモジュールの名前として使えるか (空でなく、`/` を含まず、`#` で始まらない)。
    pub fn is_valid_name(name: &str) -> bool {
        !name.is_empty() && !name.contains('/') && !name.starts_with('#')
    }

    /// モジュールが参照するパラメータの ID (昇順、重複なし)。
    /// キーフォーム格子の軸 (ブレンドシェイプ格子を含む)・深度からの回転の設定・物理の入力と出力。
    pub fn referenced_parameters(&self) -> BTreeSet<ParamId> {
        let mut found = BTreeSet::new();
        let mut grid = |bindings: Vec<&ParamId>| found.extend(bindings.into_iter().cloned());
        for part in &self.parts {
            grid(grid_parameters(&part.keyforms));
        }
        for deformer in &self.deformers {
            match &deformer.kind {
                DeformerKind::Warp(warp) => grid(grid_parameters(&warp.keyforms)),
                DeformerKind::Rotation(rotation) => grid(grid_parameters(&rotation.keyforms)),
                DeformerKind::Unknown => {}
            }
        }
        for mesh in &self.meshes {
            grid(grid_parameters(&mesh.keyforms));
        }
        for glue in &self.glues {
            grid(grid_parameters(&glue.keyforms));
        }
        for rig in &self.physics.rigs {
            found.extend(rig.inputs.iter().map(|input| input.parameter.clone()));
            for output in &rig.outputs {
                if let PhysicsTarget::Parameter { id } = &output.target {
                    found.insert(id.clone());
                }
            }
        }
        found
    }

    /// 検証用に、モジュールの要素だけを持つモデルを作る。`parameters` はモジュールが参照する
    /// パラメータの代わり。マスクのうちモジュールの外のメッシュ (スロットの `mask_sources`) を
    /// 参照するものは取り除く (合成時に検査する)。ポーズはモジュールのグループだけを入れる。
    pub(crate) fn stand_in_model(&self, parameters: Vec<Parameter>) -> Model {
        let own: BTreeSet<&MeshId> = self.meshes.iter().map(|m| &m.id).collect();
        let meshes = self
            .meshes
            .iter()
            .map(|mesh| {
                let mut mesh = mesh.clone();
                mesh.masks.retain(|mask| own.contains(&mask.mesh));
                mesh
            })
            .collect();
        Model {
            schema_version: self.schema_version,
            meta: ModelMeta {
                name: self.name.clone(),
                canvas: Canvas {
                    width: 0.0,
                    height: 0.0,
                    origin: ugoe_core::Vec2::ZERO,
                },
            },
            parameters,
            parameter_groups: Vec::new(),
            parts: self.parts.clone(),
            deformers: self.deformers.clone(),
            meshes,
            glues: self.glues.clone(),
            textures: self.textures.clone(),
            hit_areas: Vec::new(),
            // 本体の要素への追従 (`links`) は合成時に検査する。
            pose: Pose {
                groups: self.pose.groups.clone(),
                ..Pose::default()
            },
            effects: Effects::default(),
            avatar: None,
            slots: Vec::new(),
            lighting: false,
        }
    }
}

/// 格子 (とブレンドシェイプ格子) の軸と、深度からの回転の設定が参照するパラメータ。
fn grid_parameters<T>(grid: &crate::KeyformGrid<T>) -> Vec<&ParamId> {
    let mut found = Vec::new();
    for g in std::iter::once(grid).chain(&grid.blend_shapes) {
        found.extend(g.bindings.iter().map(|b| &b.parameter));
        if let Some(crate::KeyformInference::DepthRotation(rotation)) = &g.inference {
            found.extend(rotation.yaw.iter().chain(&rotation.pitch));
        }
    }
    found
}
