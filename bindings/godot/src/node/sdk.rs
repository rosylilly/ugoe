//! `UgoePuppet2D` の、SDK (`ugoe::Puppet`) と C ABI にある機能のうち、ノードの基本の操作から分けたもの (T0449)。
//!
//! 読み込みの警告 (着せ替えのモジュールの分は T0456)・受け手 (`follow`)・指揮役の値 (T0454)・標準の当たり判定・表情を止める・パーツの不透明度とポーズ・効果と物理の設定の差し替え・
//! スロットとトラッキングの対象の一覧。有限でない値の扱いは C ABI (`crates/ugoe-ffi`) と同じ。

use godot::prelude::*;
use ugoe::model::physics::PhysicsSettings;
use ugoe::model::validate::{ValidationError, describe_errors};
use ugoe::motion::TrackingMap;
use ugoe::{
    DressUpError, FieldValue, LipSyncInput, LoadWarning, PartOpacityInput, validation_fields,
};

use super::{UgoePuppet2D, error_dictionary, field_variant, to_canvas};

/// JSON を読めないときの種類 (読み込みの `invalid_file` と同じ値)。
const INVALID_FILE: &str = DressUpError::INVALID_FILE;

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

    // --- 指揮役 (T0454。docs/spec/motion.md#受け手として進める)

    /// 指揮役の口: 直前の `advance` (か `follow`) で、物理の段に入る直前のパラメータの値 (`get_parameter_ids()` の順。
    /// `ugoe::Puppet::values_before_physics`。C ABI の `ugoe_puppet_values_before_physics`)。受け手の `follow` の `values` に
    /// そのまま渡す (`get_parameter_values()` は物理の後の値なので、物理を持つモデルでは指揮役と同じ形にならない)。
    /// 物理の段を止めていても同じ位置の値。モデルが無ければ空。
    #[func]
    pub fn get_values_before_physics(&self) -> PackedFloat32Array {
        self.loaded
            .as_ref()
            .map(|l| PackedFloat32Array::from(l.puppet.values_before_physics()))
            .unwrap_or_default()
    }

    /// 指揮役の口: 直前の `advance` (か `follow`) でリップシンクの段が当てた音量 (`x`) と強さ (`y`)
    /// (`ugoe::Puppet::lip_sync_applied`。C ABI の `ugoe_puppet_lip_sync_applied`)。受け手の `follow` の `lip_sync_level` と
    /// `lip_sync_strength` にそのまま渡す。当てていなければ (段を止めていたか音量が無いか、モデルが無ければ) `Vector2(-1, 1)`
    /// (`follow` は負の音量を「当てない」とみなす)。
    #[func]
    pub fn get_lip_sync_applied(&self) -> Vector2 {
        self.loaded
            .as_ref()
            .and_then(|l| l.puppet.lip_sync_applied())
            .map_or(Vector2::new(-1.0, 1.0), |l| {
                Vector2::new(l.level, l.strength)
            })
    }

    /// 指揮役の口: 直前の `advance` (か `follow`) で、モーションのパーツの不透明度のカーブが値を求めたパーツ (カーブのあるパーツだけ。
    /// `ugoe::Puppet::part_opacities_applied`。C ABI の `ugoe_puppet_part_opacities_applied`)。要素は `follow` の
    /// `part_opacities` と同じ `{ "part": パーツの番号 (get_part_ids() の順), "value", "weight" }` で、そのまま渡す。
    /// 型の無い配列で返す (`follow` の `part_opacities` は型の無い `Array` なので、`Array[Dictionary]` は渡せない)。
    /// モデルが無ければ空。
    #[func]
    pub fn get_part_opacities_applied(&self) -> VarArray {
        let Some(loaded) = &self.loaded else {
            return VarArray::new();
        };
        loaded
            .puppet
            .part_opacities_applied()
            .iter()
            .map(|p| {
                let mut d = VarDictionary::new();
                d.set("part", i64::from(p.part));
                d.set("value", p.value);
                d.set("weight", p.weight);
                d.to_variant()
            })
            .collect()
    }

    // --- モーションのイベント

    /// 直前の `advance` で発火したモーションのイベント (T0462)。要素は `{ "value", "time", "motion", "number" }` で、
    /// `time` はモーションの中の秒、`motion` と `number` はイベントを発火したモーションの名前と始めた番号 (このノードで開始した
    /// モーションに 1 から振る。開始しなかった要求では進まず、モデルを読み直すと 1 からになる)。クロスフェードでフェードアウト中の
    /// モーションのイベントはそのモーションのもの。並びは開始した順のモーションごとに発火した順で、wasm の `motion_events_json()`・
    /// C ABI の `ugoe_puppet_motion_event`・`ugoe_puppet_motion_event_source` と同じ。`follow` はイベントを変えない。モデルが無ければ空。
    #[func]
    pub fn get_motion_events(&self) -> Array<VarDictionary> {
        let Some(loaded) = &self.loaded else {
            return Array::new();
        };
        let puppet = &loaded.puppet;
        puppet
            .motion_events()
            .iter()
            .zip(puppet.motion_event_sources())
            .map(|(e, source)| {
                let mut d = VarDictionary::new();
                d.set("value", e.value.as_str());
                d.set("time", e.t);
                d.set("motion", source.name.as_str());
                d.set("number", super::to_i64(source.number));
                d
            })
            .collect()
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

    /// ポーズのグループごとの表示中の要素のパーツの ID (`docs/spec/motion.md#ポーズ`)。グループごとに 1 つで、並びは wasm の
    /// `pose_visible_json` (`pose_json` の `groups`。付けた着せ替えのモジュールのグループも入る) と同じ。要素の無いグループは空の文字列。
    /// `show_pose` の選択はすぐに、モーションのパーツの不透明度のカーブの要求は次の更新の後に変わる (不透明度のフェードの途中でも
    /// 選ばれた要素を返す)。モデルが無ければ空。
    #[func]
    pub fn get_visible_pose_parts(&self) -> PackedStringArray {
        self.loaded
            .as_ref()
            .map(|l| {
                visible_pose_parts(&l.puppet)
                    .into_iter()
                    .map(|part| GString::from(part.unwrap_or_default()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// ポーズの定義: グループごとの要素のパーツの ID (`docs/spec/motion.md#ポーズ`)。グループの並びは `get_visible_pose_parts` と
    /// 同じ (付けた着せ替えのモジュールのグループも入る)、要素の並びはモデルの `entries` の並びで、先頭が初めに表示する要素。
    /// `links` のパーツは含めない (`show_pose` に渡せるパーツだけ)。要素の無いグループは空の配列。モデルが無ければ空。
    #[func]
    pub fn get_pose_entry_parts(&self) -> Array<PackedStringArray> {
        self.loaded
            .as_ref()
            .map(|l| {
                pose_entry_parts(&l.puppet)
                    .into_iter()
                    .map(|parts| parts.into_iter().map(GString::from).collect())
                    .collect()
            })
            .unwrap_or_default()
    }

    // --- 目パチ・呼吸・リップシンクと物理の設定の差し替え

    /// 目パチ・呼吸・リップシンクの設定を JSON (モデルの `effects` と同じ形。省いたフィールドは既定値) で差し替える
    /// (`docs/spec/motion.md#エフェクト設定の差し替え`)。再生の状態は初めからになり、リップシンクの音量は保つ。
    /// モデルを読み直すとモデルの設定に戻る。
    ///
    /// 戻り値は結果の辞書 (T0470): 成功なら `{ "ok": true }`。失敗ならエラーを出して何も変えず、
    /// `{ "ok": false, "code", "error", ... }` (`get_load_error()` と同じ形)。種類は、モデルが無ければ `no_model`、JSON を読めなければ
    /// `invalid_file`、モデルの読み込みと同じ検査 (`ugoe::Puppet::try_set_effects`。有限でない値・範囲の外の値・モデルにないパラメータ) を
    /// 通らなければ `invalid_model` で、補足の値 `errors` (検査のエラーの数)・`error_codes` (その種類を `", "` でつないだもの) を持つ
    /// (C ABI の `ugoe_puppet_set_effects_json`・wasm の `set_effects_json` と同じ。文は C ABI の `UGOE_ERROR_INVALID_MODEL` の文と同じ形)。
    #[func]
    pub fn set_effects_json(&mut self, json: GString) -> VarDictionary {
        let Some(loaded) = &mut self.loaded else {
            return no_model();
        };
        let effects = match serde_json::from_str(&json.to_string()) {
            Ok(effects) => effects,
            Err(e) => return settings_error(INVALID_FILE, &format!("effects: {e}"), []),
        };
        match loaded.puppet.try_set_effects(effects) {
            Ok(()) => {
                self.dirty = true;
                settings_ok()
            }
            Err(errors) => settings_error(
                "invalid_model",
                &describe_errors("effects", &errors),
                validation_fields(&errors),
            ),
        }
    }

    /// 今の物理の設定を JSON (`.ugoe` の物理と同じ形) で差し替える。空の文字列なら物理を外す。
    /// 着せ替えのモジュールを付けていれば、次の付け外しで本体の設定から合成し直すので失われる
    /// (付けている間も保つなら `set_base_physics_json`)。モデルを読み直すとモデルの設定に戻る。
    ///
    /// 戻り値は `set_effects_json` と同じ結果の辞書。失敗の種類は、JSON を読めなければ `invalid_file`、モデルが無ければ `no_model`、
    /// モデルに対する検査を通らなければ `invalid_physics` (補足の値 `errors`・`error_codes`。着せ替えの `invalid_physics` と、C ABI の
    /// `ugoe_puppet_set_physics_json`・wasm の `set_physics_json` と同じ。T0469)。
    #[func]
    pub fn set_physics_json(&mut self, json: GString) -> VarDictionary {
        let settings = match read_physics(&json) {
            Ok(settings) => settings,
            Err(error) => return error,
        };
        let Some(loaded) = &mut self.loaded else {
            return no_model();
        };
        if let Err(errors) = loaded.puppet.set_physics(settings) {
            return invalid_physics(&errors);
        }
        self.dirty = true;
        settings_ok()
    }

    /// 本体の物理の設定を JSON で差し替える。空の文字列なら外す (`ugoe::Puppet::set_base_physics`、
    /// `docs/spec/modules.md#puppet-での着せ替え`)。モジュールを付けていれば、付けているモジュールと合成し直し、
    /// この後の付け外しでもこの設定から合成する。付けたことがなければ `set_physics_json` と同じ。
    /// モデルを読み直すとモデルの設定に戻る。
    ///
    /// 戻り値は `set_physics_json` と同じ結果の辞書。付けているモジュールと合成できなければ、種類と補足の値は着せ替えの失敗
    /// (`get_load_error()` のモジュールの付け外しと同じ。C ABI の `UGOE_ERROR_DRESS_UP` の種類) になる。
    #[func]
    pub fn set_base_physics_json(&mut self, json: GString) -> VarDictionary {
        let settings = match read_physics(&json) {
            Ok(settings) => settings,
            Err(error) => return error,
        };
        let Some(loaded) = &mut self.loaded else {
            return no_model();
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
                settings_ok()
            }
            Err(DressUpError::Physics(errors)) => invalid_physics(&errors),
            Err(e) => settings_error(e.code(), &format!("base physics: {e}"), e.fields()),
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

/// 物理の設定の JSON を読む。空なら外す (`Ok(None)`)。読めなければエラーを出して `invalid_file` の結果の辞書。
fn read_physics(json: &GString) -> Result<Option<PhysicsSettings>, VarDictionary> {
    let json = json.to_string();
    if json.trim().is_empty() {
        return Ok(None);
    }
    ugoe::format::physics_from_json(&json)
        .map(Some)
        .map_err(|e| settings_error(INVALID_FILE, &format!("physics: {e}"), []))
}

/// 設定の差し替え (`set_*_json`) が成功したときの結果の辞書: `{ "ok": true }`。
fn settings_ok() -> VarDictionary {
    let mut d = VarDictionary::new();
    d.set("ok", true);
    d
}

/// 設定の差し替えが失敗したときの結果の辞書: エラーを出し、`{ "ok": false, "code", "error", ...fields }`
/// (`get_load_error()` と同じ形に `ok` を足したもの)。
fn settings_error<'a>(
    code: &str,
    message: &str,
    fields: impl IntoIterator<Item = (&'static str, FieldValue<'a>)>,
) -> VarDictionary {
    godot_error!("ugoe: {message}");
    let fields = fields
        .into_iter()
        .map(|(key, value)| (key, field_variant(value)));
    let mut d = error_dictionary(code, message, fields);
    d.set("ok", false);
    d
}

/// モデルが無いときの結果の辞書 (wasm の着せ替えの `no_model` と同じ種類)。
fn no_model() -> VarDictionary {
    settings_error("no_model", "no model is loaded", [])
}

/// 物理の設定の検査の失敗の結果の辞書: 種類は `invalid_physics`、補足の値は `errors`・`error_codes`
/// (文は C ABI の `UGOE_ERROR_INVALID_MODEL` の文と同じ形。`docs/spec/model.md#検証エラーの文`)。
fn invalid_physics(errors: &[ValidationError]) -> VarDictionary {
    settings_error(
        "invalid_physics",
        &describe_errors("physics settings", errors),
        validation_fields(errors),
    )
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

/// ポーズのグループごとの表示中の要素のパーツ (要素の無いグループは `None`)。`ugoe-wasm` の `pose_visible_parts` と同じ並び
/// (`PosePlayer::visible_parts` は要素の無いグループを飛ばすので、グループと突き合わせる)。
fn visible_pose_parts(puppet: &ugoe::Puppet) -> Vec<Option<&str>> {
    let player = puppet.pose();
    let mut visible = player.visible_parts();
    player
        .pose()
        .groups
        .iter()
        .map(|group| {
            if group.entries.is_empty() {
                None
            } else {
                visible.next().map(|part| part.as_str())
            }
        })
        .collect()
}

/// ポーズのグループごとの要素のパーツ (`links` は含めない)。C ABI の `ugoe_puppet_pose_entry_part` と同じ並び。
fn pose_entry_parts(puppet: &ugoe::Puppet) -> Vec<Vec<&str>> {
    puppet
        .pose()
        .pose()
        .groups
        .iter()
        .map(|group| group.entries.iter().map(|e| e.part.as_str()).collect())
        .collect()
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
                    FieldValue::Codes(_) => d.set(key, &GString::from(&value.to_string())),
                }
            }
            d
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ugoe::Puppet;
    use ugoe::model::fixtures;
    use ugoe::model::pose::{PoseEntry, PoseGroup};
    use ugoe::motion::Priority;

    use super::{pose_entry_parts, visible_pose_parts};

    const DT: f32 = 1.0 / 60.0;

    /// 表示中の要素は `show_pose` の選択 (すぐ) とモーションの要求 (次の更新の後) の両方に従う。
    #[test]
    fn visible_pose_parts_follow_show_pose_and_motions() {
        let mut puppet = Puppet::new(fixtures::pose_and_effects()).expect("puppet");
        assert_eq!(visible_pose_parts(&puppet), [Some("ArmA")]);

        assert!(puppet.pose_mut().show(&"ArmB".into()));
        assert_eq!(visible_pose_parts(&puppet), [Some("ArmB")]);
        puppet.update(DT);
        assert_eq!(visible_pose_parts(&puppet), [Some("ArmB")]);

        let motion = ugoe::format::motion_from_json(
            r#"{
                "name": "arm_a",
                "duration": 1.0,
                "looping": true,
                "fade_in": 0.0,
                "fade_out": 0.0,
                "curves": [{
                    "target": { "kind": "part_opacity", "id": "ArmA" },
                    "segments": [
                        { "type": "start", "p": { "t": 0.0, "v": 1.0 } },
                        { "type": "linear", "to": { "t": 1.0, "v": 1.0 } }
                    ]
                }]
            }"#,
        )
        .expect("motion");
        assert!(puppet.start_motion(motion, Priority::Normal));
        assert_eq!(visible_pose_parts(&puppet), [Some("ArmB")]);
        puppet.update(DT);
        assert_eq!(visible_pose_parts(&puppet), [Some("ArmA")]);
    }

    /// 要素の無いグループは `None` で、ほかのグループの並びはずれない。付けたモジュールのグループも入る。
    #[test]
    fn visible_pose_parts_keep_empty_groups_and_include_modules() {
        let mut model = fixtures::pose_and_effects();
        model.pose.groups.insert(
            0,
            PoseGroup {
                entries: Vec::new(),
            },
        );
        let puppet = Puppet::new(model).expect("puppet");
        assert_eq!(visible_pose_parts(&puppet), [None, Some("ArmA")]);

        let mut puppet = Puppet::new(fixtures::dress_up_base()).expect("puppet");
        assert!(visible_pose_parts(&puppet).is_empty());
        let mut module = fixtures::long_hair_module();
        module.pose.groups = vec![PoseGroup {
            entries: vec![PoseEntry {
                part: "Hair".into(),
                links: Vec::new(),
            }],
        }];
        puppet.attach_module(Arc::new(module)).expect("attach");
        assert_eq!(visible_pose_parts(&puppet), [Some("LongHair/Hair")]);
    }

    /// 定義はグループごとの要素の `part` をモデルの並びで返し、`links` と要素の無いグループの扱いは表示中の要素と並びがそろう。
    #[test]
    fn pose_entry_parts_list_the_definition() {
        // ArmA の要素はリンクに SleeveA を持つ
        let mut model = fixtures::pose_and_effects();
        model.pose.groups.insert(
            0,
            PoseGroup {
                entries: Vec::new(),
            },
        );
        let mut puppet = Puppet::new(model).expect("puppet");
        let empty: [&str; 0] = [];
        assert_eq!(
            pose_entry_parts(&puppet),
            [&empty[..], &["ArmA", "ArmB"][..]]
        );
        assert_eq!(
            pose_entry_parts(&puppet).len(),
            visible_pose_parts(&puppet).len()
        );
        // 定義の要素は `show_pose` に渡せる
        for part in ["ArmB", "ArmA"] {
            assert!(puppet.pose_mut().show(&part.into()));
            assert_eq!(visible_pose_parts(&puppet), [None, Some(part)]);
        }

        let mut puppet = Puppet::new(fixtures::dress_up_base()).expect("puppet");
        assert!(pose_entry_parts(&puppet).is_empty());
        let mut module = fixtures::long_hair_module();
        module.pose.groups = vec![PoseGroup {
            entries: vec![PoseEntry {
                part: "Hair".into(),
                links: Vec::new(),
            }],
        }];
        puppet.attach_module(Arc::new(module)).expect("attach");
        assert_eq!(pose_entry_parts(&puppet), [["LongHair/Hair"]]);
    }
}
