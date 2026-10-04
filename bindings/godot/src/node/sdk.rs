//! `UgoePuppet2D` の、SDK (`ugoe::Puppet`) と C ABI にある機能のうち、ノードの基本の操作から分けたもの (T0449)。
//!
//! 読み込みの警告 (着せ替えのモジュールの分は T0456)・受け手 (`follow`)・標準の当たり判定・表情を止める・パーツの不透明度とポーズ・効果と物理の設定の差し替え・
//! スロットとトラッキングの対象の一覧。有限でない値の扱いは C ABI (`crates/ugoe-ffi`) と同じ。

use godot::prelude::*;
use ugoe::model::physics::PhysicsSettings;
use ugoe::model::validate::{ValidationError, describe_errors};
use ugoe::motion::TrackingMap;
use ugoe::{DressUpError, FieldValue, LipSyncInput, LoadWarning, PartOpacityInput};

use super::{UgoePuppet2D, to_canvas};

#[godot_api(secondary)]
impl UgoePuppet2D {
    // --- 読み込みの警告 (ADR 0013)

    /// 直前に読み込んだモデルの読み込みの警告 (読み込めたが表示が劣化しうること)。要素は
    /// `{ "code", "warning", ... }` で、`code` と補足のキー (`"texture"`・`"feature"`・`"path"`) は wasm の
    /// `load_warnings_json` と C ABI の `ugoe_puppet_load_warning_*` と同じ (`crates/ugoe-wasm/README.md#読み込みの警告-t0125`)。
    /// `warning` は英語の文 (安定した値として扱わない)。テクスチャの番号は整数。モデルが無ければ空。
    #[func]
    pub fn get_load_warnings(&self) -> Array<VarDictionary> {
        self.loaded
            .as_ref()
            .map(|l| warning_dictionaries(&l.warnings))
            .unwrap_or_default()
    }

    /// 取り付けている名前が `name` の着せ替えのモジュール (`.ugoemod`) の読み込みの警告 (T0456)。要素の形は
    /// `get_load_warnings()` と同じで、wasm の `modules_json()` の `warning_details` と C ABI の `ugoe_module_warning_*` に揃える。
    /// 付けているモジュールの探し方は `get_module_names()` と同じ (モデルを読み込んでいれば実際に付いているものだけ)。
    /// そのモジュールが付いていなければ空。
    #[func]
    pub fn get_module_load_warnings(&self, name: GString) -> Array<VarDictionary> {
        let name = name.to_string();
        let found = match &self.loaded {
            Some(loaded) => loaded.modules().iter().find(|d| d.name() == name),
            None => self
                .attached
                .iter()
                .map(|(_, d)| d)
                .find(|d| d.name() == name),
        };
        found
            .map(|d| warning_dictionaries(&d.warnings))
            .unwrap_or_default()
    }

    // --- 受け手 (docs/spec/motion.md#受け手として進める)

    /// すべてのパラメータの今の値 (`get_parameter_ids()` の順)。モデルが無ければ空。
    #[func]
    pub fn get_parameter_values(&self) -> PackedFloat32Array {
        self.loaded
            .as_ref()
            .map(|l| PackedFloat32Array::from(l.puppet.instance().parameters().as_slice()))
            .unwrap_or_default()
    }

    /// パラメータの値 (`get_parameter_ids()` の順) をまとめて **基準値として** 書く (値域にクランプする)。
    /// `set_parameter` の値は前のフレームの出力と同じなら基準値にならないが、これは同じでも基準値にする。
    /// モデルが無い・数が違う・NaN を含むときは、エラーを出して何も変えずに false。
    #[func]
    pub fn set_base_values(&mut self, values: PackedFloat32Array) -> bool {
        let Some(loaded) = &mut self.loaded else {
            godot_error!("ugoe: no model is loaded");
            return false;
        };
        if let Err(e) = loaded.puppet.set_base_values(values.as_slice()) {
            godot_error!("ugoe: set_base_values: {e}");
            return false;
        }
        self.dirty = true;
        true
    }

    /// 受け手として 1 フレーム進めて描く: `values` を基準値として書き、物理 → リップシンク → パーツの不透明度
    /// (モーションのカーブとポーズ) → 評価の順に `delta` 秒進める (`ugoe::Puppet::follow`。C ABI の `ugoe_puppet_follow`)。
    ///
    /// `values` は指揮役の物理の段に入る直前の値 (`get_parameter_ids()` の順)。`lip_sync_level` と `lip_sync_strength` は
    /// 指揮役のリップシンクの音量と強さで、`lip_sync_level` が負ならリップシンクを当てない。`part_opacities` は指揮役の
    /// モーションのパーツの不透明度のカーブで、`{ "part": パーツの番号 (get_part_ids() の順), "value", "weight" }` の配列
    /// (無ければ空の配列)。モーション・トラッキング・表情・目パチ・呼吸は通さず、`motion_event` も出さない。
    ///
    /// 受け手にするノードは `playing` を false にして、毎フレームこれを呼ぶ (`playing` が true だと `update` が上書きする)。
    /// ノードは読み込みの直後に 1 度評価して物理の粒子を置くので、指揮役と同じ形から始めるなら最初に `reset_physics` を呼ぶ。
    /// モデルが無い・数が違う・NaN を含む・`delta` が負か有限でない・リップシンクの音量 (当てるとき) と強さ・
    /// パーツの不透明度の値と重みが有限でない・`part_opacities` の形が違うときは、エラーを出して何も変えずに false。
    #[func]
    pub fn follow(
        &mut self,
        values: PackedFloat32Array,
        delta: f32,
        lip_sync_level: f32,
        lip_sync_strength: f32,
        part_opacities: VarArray,
    ) -> bool {
        if !(delta.is_finite() && delta >= 0.0) {
            godot_error!("ugoe: delta must be a finite non-negative number (got {delta})");
            return false;
        }
        let Some(part_opacities) = read_part_opacities(&part_opacities) else {
            return false;
        };
        let Some(loaded) = &mut self.loaded else {
            godot_error!("ugoe: no model is loaded");
            return false;
        };
        // NaN の音量は当てる側に入れ、`Puppet::follow` に拒ませる (C ABI と同じ)。
        let lip_sync = (lip_sync_level.is_nan() || lip_sync_level >= 0.0).then_some(LipSyncInput {
            level: lip_sync_level,
            strength: lip_sync_strength,
        });
        if let Err(e) = loaded
            .puppet
            .follow(values.as_slice(), lip_sync, &part_opacities, delta)
        {
            godot_error!("ugoe: follow: {e}");
            return false;
        }
        self.draw();
        true
    }

    // --- 標準の当たり判定 (docs/spec/avatar-profile.md#アプリ向けの情報)

    /// 標準の当たり判定の名前 (`"Head"`・`"Body"`。`Model.avatar.hit_areas`) に対応する当たり判定の ID。無ければ空。
    #[func]
    pub fn get_standard_hit_area(&self, name: GString) -> GString {
        self.loaded
            .as_ref()
            .and_then(|l| l.puppet.standard_hit_area(&name.to_string()))
            .map(GString::from)
            .unwrap_or_default()
    }

    /// 標準の名前 `name` の当たり判定が、ノードの座標 `local_position` を含むか。対応が無ければ false。
    #[func]
    pub fn hit_test_standard(&self, name: GString, local_position: Vector2) -> bool {
        let Some(loaded) = &self.loaded else {
            return false;
        };
        let p = to_canvas(loaded, local_position);
        loaded.puppet.hit_test_standard(&name.to_string(), p)
    }

    /// ノードの座標 `local_position` を含む当たり判定のうち、標準の名前を持つものの名前を手前から順に並べる。
    #[func]
    pub fn get_standard_hit_areas_at(&self, local_position: Vector2) -> PackedStringArray {
        let Some(loaded) = &self.loaded else {
            return PackedStringArray::new();
        };
        let p = to_canvas(loaded, local_position);
        loaded
            .puppet
            .standard_hit_areas_at(p)
            .into_iter()
            .map(GString::from)
            .collect()
    }

    // --- 表情・パーツ・ポーズ

    /// すべての表情を (フェードアウトをかけて) 止める。
    #[func]
    pub fn stop_expressions(&mut self) {
        if let Some(loaded) = &mut self.loaded {
            loaded.puppet.expressions_mut().stop_all();
            self.dirty = true;
        }
    }

    /// パーツの ID をモデルの順に並べる (`follow` の `part_opacities` の番号はこの順)。
    #[func]
    pub fn get_part_ids(&self) -> PackedStringArray {
        self.loaded
            .as_ref()
            .map(|l| {
                l.model()
                    .parts
                    .iter()
                    .map(|p| GString::from(p.id.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 直前の評価でパーツ `id` に書いた不透明度の上書き (モーションのパーツの不透明度のカーブとポーズの結果):
    /// `{ "value", "weight" }`。上書きしていなければ `weight` は 0。パーツが無ければ空。
    #[func]
    pub fn get_part_opacity(&self, id: GString) -> VarDictionary {
        let mut d = VarDictionary::new();
        if let Some((loaded, i)) = self.part(&id) {
            let (value, weight) = loaded.puppet.instance().part_opacity_override(i);
            d.set("value", value);
            d.set("weight", weight);
        }
        d
    }

    /// 直前の評価でのパーツ `id` の不透明度 (上書きを当てた自身の値に祖先の値を掛けたもの。そのパーツのメッシュの
    /// 不透明度に掛かる)。パーツが無ければ警告を出して 0。
    #[func]
    pub fn get_part_evaluated_opacity(&self, id: GString) -> f32 {
        match self.part(&id) {
            Some((loaded, i)) => loaded.puppet.instance().part_opacity(i),
            None => {
                godot_warn!("ugoe: part `{id}` not found");
                0.0
            }
        }
    }

    /// ポーズで、パーツ `part` を持つ要素をそのグループの表示中の要素にする (`links` のパーツは対象外。
    /// `docs/spec/motion.md#ポーズ`)。不透明度は次の更新からフェードで近づく。どの要素のパーツでもなければ false。
    #[func]
    pub fn show_pose(&mut self, part: GString) -> bool {
        let Some(loaded) = &mut self.loaded else {
            return false;
        };
        let shown = loaded
            .puppet
            .pose_mut()
            .show(&part.to_string().as_str().into());
        self.dirty |= shown;
        shown
    }

    // --- 目パチ・呼吸・リップシンクと物理の設定の差し替え

    /// 目パチ・呼吸・リップシンクの設定を JSON (モデルの `effects` と同じ形。省いたフィールドは既定値) で差し替える
    /// (`docs/spec/motion.md#エフェクト設定の差し替え`)。再生の状態は初めからになり、リップシンクの音量は保つ。
    /// モデルを読み直すとモデルの設定に戻る。読めなければエラーを出して何も変えずに false。
    #[func]
    pub fn set_effects_json(&mut self, json: GString) -> bool {
        let Some(loaded) = &mut self.loaded else {
            godot_error!("ugoe: no model is loaded");
            return false;
        };
        match serde_json::from_str(&json.to_string()) {
            Ok(effects) => {
                loaded.puppet.set_effects(effects);
                self.dirty = true;
                true
            }
            Err(e) => {
                godot_error!("ugoe: effects: {e}");
                false
            }
        }
    }

    /// 今の物理の設定を JSON (`.ugoe` の物理と同じ形) で差し替える。空の文字列なら物理を外す。
    /// 着せ替えのモジュールを付けていれば、次の付け外しで本体の設定から合成し直すので失われる
    /// (付けている間も保つなら `set_base_physics_json`)。モデルを読み直すとモデルの設定に戻る。
    /// 読めないか、モデルに対する検査を通らなければ、エラーを出して何も変えずに false。
    #[func]
    pub fn set_physics_json(&mut self, json: GString) -> bool {
        let Some(settings) = read_physics(&json) else {
            return false;
        };
        let Some(loaded) = &mut self.loaded else {
            godot_error!("ugoe: no model is loaded");
            return false;
        };
        if let Err(errors) = loaded.puppet.set_physics(settings) {
            godot_error!("ugoe: {}", invalid_physics(&errors));
            return false;
        }
        self.dirty = true;
        true
    }

    /// 本体の物理の設定を JSON で差し替える。空の文字列なら外す (`ugoe::Puppet::set_base_physics`、
    /// `docs/spec/modules.md#puppet-での着せ替え`)。モジュールを付けていれば、付けているモジュールと合成し直し、
    /// この後の付け外しでもこの設定から合成する。付けたことがなければ `set_physics_json` と同じ。
    /// モデルを読み直すとモデルの設定に戻る。失敗したらエラーを出して何も変えずに false。
    #[func]
    pub fn set_base_physics_json(&mut self, json: GString) -> bool {
        let Some(settings) = read_physics(&json) else {
            return false;
        };
        let Some(loaded) = &mut self.loaded else {
            godot_error!("ugoe: no model is loaded");
            return false;
        };
        match loaded.set_base_physics(settings) {
            Ok(swapped) => {
                if swapped {
                    // 合成したモデルが変わった (テクスチャは同じなので使い回す)。
                    let textures = self
                        .renderer
                        .as_mut()
                        .map(|r| r.take_textures())
                        .unwrap_or_default();
                    self.rebuild_renderer(textures);
                }
                self.dirty = true;
                true
            }
            Err(DressUpError::Physics(errors)) => {
                godot_error!("ugoe: {}", invalid_physics(&errors));
                false
            }
            Err(e) => {
                godot_error!("ugoe: base physics: {e}");
                false
            }
        }
    }

    /// 今の物理の設定の JSON。物理が無ければ空。
    #[func]
    pub fn get_physics_json(&self) -> GString {
        let settings = self
            .loaded
            .as_ref()
            .and_then(|l| l.puppet.physics())
            .map(|p| p.settings());
        physics_json(settings)
    }

    /// 本体の物理の設定の JSON (モジュールを付けていれば合成する前の設定)。物理が無ければ空。
    #[func]
    pub fn get_base_physics_json(&self) -> GString {
        physics_json(self.loaded.as_ref().and_then(|l| l.puppet.base_physics()))
    }

    // --- スロット・トラッキングの対象

    /// 本体のモデルのスロットの ID (モジュールを付けていても本体のもの。`UgoeModule` のスロットと比べる)。
    #[func]
    pub fn get_slot_ids(&self) -> PackedStringArray {
        self.loaded
            .as_ref()
            .map(|l| {
                l.puppet
                    .base_model()
                    .slots
                    .iter()
                    .map(|s| GString::from(s.id.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// トラッキングが書き込むパラメータの ID (`get_parameter_ids()` の順。モデルの `avatar.tracking` から決まる)。
    #[func]
    pub fn get_tracking_targets(&self) -> PackedStringArray {
        let Some(loaded) = &self.loaded else {
            return PackedStringArray::new();
        };
        let instance = loaded.puppet.instance();
        let parameters = &loaded.model().parameters;
        let map = TrackingMap::for_model(loaded.model(), instance.parameters());
        let mut targets: Vec<usize> = map.targets().map(|i| i.0 as usize).collect();
        targets.sort_unstable();
        targets.dedup();
        targets
            .into_iter()
            .filter_map(|i| parameters.get(i))
            .map(|p| GString::from(p.id.as_str()))
            .collect()
    }
}

impl UgoePuppet2D {
    /// 読み込んだモデルと、ID が `id` のパーツの番号。
    fn part(&self, id: &GString) -> Option<(&crate::model::LoadedModel, usize)> {
        let loaded = self.loaded.as_ref()?;
        let i = loaded
            .puppet
            .instance()
            .compiled()
            .part_index(&id.to_string())?;
        Some((loaded, i))
    }
}

/// `follow` の `part_opacities` (`{ "part", "value", "weight" }` の配列) を読む。形が違えばエラーを出して `None`。
/// 値と重みの有限性は `Puppet::follow` が確かめる。
fn read_part_opacities(list: &VarArray) -> Option<Vec<PartOpacityInput>> {
    let mut out = Vec::with_capacity(list.len());
    for (i, v) in list.iter_shared().enumerate() {
        let entry = v.try_to::<VarDictionary>().ok().and_then(|d| {
            let part = d.get("part")?.try_to::<i64>().ok()?;
            Some(PartOpacityInput {
                part: u32::try_from(part).ok()?,
                value: d.get("value")?.try_to::<f32>().ok()?,
                weight: d.get("weight")?.try_to::<f32>().ok()?,
            })
        });
        let Some(entry) = entry else {
            godot_error!(
                "ugoe: part_opacities[{i}] must be {{ \"part\": int >= 0, \"value\": float, \"weight\": float }} (got {v})"
            );
            return None;
        };
        out.push(entry);
    }
    Some(out)
}

/// 物理の設定の JSON を読む。空なら外す (`Some(None)`)。読めなければエラーを出して `None`。
fn read_physics(json: &GString) -> Option<Option<PhysicsSettings>> {
    let json = json.to_string();
    if json.trim().is_empty() {
        return Some(None);
    }
    match ugoe::format::physics_from_json(&json) {
        Ok(settings) => Some(Some(settings)),
        Err(e) => {
            godot_error!("ugoe: physics: {e}");
            None
        }
    }
}

/// 物理の設定の検査の結果を 1 行の文にする (C ABI の `UGOE_ERROR_INVALID_MODEL` の文と同じ形。
/// `docs/spec/model.md#検証エラーの文`)。
fn invalid_physics(errors: &[ValidationError]) -> String {
    describe_errors("physics settings", errors)
}

fn physics_json(settings: Option<&PhysicsSettings>) -> GString {
    let Some(settings) = settings else {
        return GString::new();
    };
    match ugoe::format::physics_to_json(settings) {
        Ok(json) => GString::from(&json),
        Err(e) => {
            godot_error!("ugoe: physics: {e}");
            GString::new()
        }
    }
}

/// 読み込みの警告を GDScript の辞書の配列にする: `{ "code", "warning", ... }`。補足のキーは `LoadWarning::fields`。
fn warning_dictionaries(warnings: &[LoadWarning]) -> Array<VarDictionary> {
    warnings
        .iter()
        .map(|w| {
            let mut d = VarDictionary::new();
            d.set("code", w.code());
            d.set("warning", &GString::from(&w.to_string()));
            for (key, value) in w.fields() {
                match value {
                    FieldValue::Index(i) => d.set(key, i as i64),
                    FieldValue::Text(t) => d.set(key, t),
                }
            }
            d
        })
        .collect()
}
