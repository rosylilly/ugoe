//! `WebPuppet` の物理・モーション・表情・エフェクト・標準の当たり判定 (T0309)。
//!
//! どれも次の `update(dt)` から効く。フレーム内の適用順は `Puppet::update` と同じ
//! (`docs/spec/motion.md#フレーム内の適用順`)。処理の本体は [`crate::animation`]。

use ugoe::Puppet;
use ugoe::core::Vec2;
use wasm_bindgen::prelude::*;

use super::{WebPuppet, js_error};
use crate::animation;

/// モーション (JSON) の各カーブを再生位置 `t` (秒) で評価した値 (フェードなし)。
/// `[{"target": {"kind", "id"}, "value"}, …]` の JSON。エディタのタイムラインのシーク用。
#[wasm_bindgen]
pub fn sample_motion_json(json: &str, t: f32) -> Result<String, JsError> {
    animation::sample_motion_json(json, t).map_err(js_error)
}

#[wasm_bindgen]
impl WebPuppet {
    // --- モーション ---

    /// 読み込んだバンドルのモーションの名前 (チャンクの順)。
    pub fn motion_names(&self) -> Vec<String> {
        self.library.motion_names()
    }

    /// 読み込んだバンドルのモーションの一覧の JSON (長さ・ループ・フェード・動かすパラメータ。T0720。
    /// 形は [`crate::animation::Library::motions_json`])。
    pub fn motions_json(&self) -> String {
        self.library.motions_json()
    }

    /// バンドルのモーション `name` を開始する。`priority` は `"idle"`・`"normal"` (既定)・`"force"`。
    /// 再生中のモーションより優先度が低くて開始しなければ `false`。未知の名前はエラー。
    pub fn start_motion(&mut self, name: &str, priority: Option<String>) -> Result<bool, JsError> {
        let priority = animation::parse_priority(priority.as_deref()).map_err(js_error)?;
        let motion = self.library.motion(name).map_err(js_error)?.clone();
        Ok(self.puppet_mut()?.start_motion(motion, priority))
    }

    /// JSON (`*.motion.json` の形) のモーションを開始する。規則は [`WebPuppet::start_motion`] と同じ。
    pub fn start_motion_json(
        &mut self,
        json: &str,
        priority: Option<String>,
    ) -> Result<bool, JsError> {
        animation::start_motion_json(self.puppet_mut()?, json, priority.as_deref())
            .map_err(js_error)
    }

    /// 再生中のモーションをすべてフェードアウトさせる。最後に適用した値は基準値として残る。
    pub fn stop_motions(&mut self) {
        if let Some(puppet) = &mut self.puppet {
            puppet.motions_mut().stop_all();
        }
    }

    pub fn is_motion_playing(&self) -> bool {
        self.puppet
            .as_ref()
            .is_some_and(|p| p.motions().is_playing())
    }

    /// 直前の `update` で発火したモーションのイベントの値。
    pub fn motion_events(&self) -> Vec<String> {
        self.puppet
            .as_ref()
            .map(animation::motion_events)
            .unwrap_or_default()
    }

    // --- 表情 ---

    /// 読み込んだバンドルの表情の名前 (チャンクの順)。
    pub fn expression_names(&self) -> Vec<String> {
        self.library.expression_names()
    }

    /// 表情をバンドルの `name` だけにする (使用中の表情はフェードアウトする)。未知の名前はエラー。
    pub fn set_expression(&mut self, name: &str) -> Result<(), JsError> {
        let expression = self.library.expression(name).map_err(js_error)?.clone();
        self.puppet_mut()?.set_expression(expression);
        self.active_expressions.set(name);
        Ok(())
    }

    /// バンドルの表情 `name` を、使用中の表情の上に重ねる (T0720)。未知の名前はエラー。
    /// 同じ表情を重ねるとその分だけ重なる (止めるときは `stop_expression` で同じ名前のものがすべて止まる)。
    pub fn add_expression(&mut self, name: &str) -> Result<(), JsError> {
        let expression = self.library.expression(name).map_err(js_error)?.clone();
        self.puppet_mut()?.expressions_mut().add(expression);
        self.active_expressions.add(name);
        Ok(())
    }

    /// 名前が `name` の表情をフェードアウトさせる (T0720)。使っていなければ何もしない。
    pub fn stop_expression(&mut self, name: &str) {
        if let Some(puppet) = &mut self.puppet {
            puppet.expressions_mut().stop(name);
            self.active_expressions.stop(name);
        }
    }

    /// 読み込んだバンドルの表情の一覧の JSON (T0720。形は [`crate::animation::Library::expressions_json`])。
    pub fn expressions_json(&self) -> String {
        self.library.expressions_json()
    }

    /// 使っている表情の名前 (開始した順、重なりなし。止めてフェードアウト中のものは含めない。T0721)。
    /// `set_expression_json` の表情も、その JSON の `name` で含める。
    pub fn active_expressions(&mut self) -> Vec<String> {
        match &mut self.puppet {
            Some(puppet) => self.active_expressions.names(puppet),
            None => Vec::new(),
        }
    }

    /// 表情を JSON (`*.expression.json` の形) の表情だけにする。
    pub fn set_expression_json(&mut self, json: &str) -> Result<(), JsError> {
        let name = animation::set_expression_json(self.puppet_mut()?, json).map_err(js_error)?;
        self.active_expressions.set(&name);
        Ok(())
    }

    /// 使用中の表情をすべてフェードアウトさせる。
    pub fn stop_expressions(&mut self) {
        if let Some(puppet) = &mut self.puppet {
            puppet.expressions_mut().stop_all();
            self.active_expressions.clear();
        }
    }

    // --- ポーズ ---

    /// ポーズで、パーツ `part` を `part` に持つ要素をそのグループの表示中の要素にする (`links` のパーツは対象外)。
    /// 不透明度は次の `update` からフェードする。その間にモーションが同じグループの要素を要求すれば要求が勝つ。
    /// どの要素の `part` でもなければ何も変えず `false`。
    pub fn show_pose(&mut self, part: &str) -> Result<bool, JsError> {
        Ok(animation::show_pose(self.puppet_mut()?, part))
    }

    /// パーツ `part` の評価後の不透明度 (上書きを当てた自身の値に祖先の値を掛けたもの。そのパーツのメッシュの
    /// 不透明度に掛かる)。必要なら評価してから読む。モデルかパーツが無ければ `undefined`。
    pub fn part_evaluated_opacity(&mut self, part: &str) -> Option<f32> {
        self.evaluate();
        animation::part_evaluated_opacity(self.puppet.as_ref()?, part)
    }

    /// 今のポーズ (モデルの `pose` と同じ JSON。着せ替えで付けたモジュールのグループと追従も入る)。
    /// モデルが無ければ `undefined`。
    pub fn pose_json(&self) -> Result<Option<String>, JsError> {
        match &self.puppet {
            Some(puppet) => animation::pose_json(puppet).map(Some).map_err(js_error),
            None => Ok(None),
        }
    }

    // --- エフェクト ---

    /// 目パチ・呼吸・リップシンクの設定 (モデルの `effects` と同じ JSON) を差し替える。
    /// 再生状態は初めからになる。リップシンクの音量は保つ。
    pub fn set_effects_json(&mut self, json: &str) -> Result<(), JsError> {
        animation::set_effects_json(self.puppet_mut()?, json).map_err(js_error)
    }

    /// リップシンクの音量 (0〜1)。`undefined` なら口のパラメータに触らない。音声の解析はホストが行う。
    pub fn set_lip_sync_level(&mut self, level: Option<f32>) {
        if let Some(puppet) = &mut self.puppet {
            puppet.set_lip_sync_level(level);
        }
    }

    /// 段 `stage` (`"physics"`・`"eye_blink"`・`"breath"`・`"lip_sync"`・`"pose"`) を動かすかを設定する。
    /// 止めた段はパラメータに触らない。物理を止めると揺れの加算量も 0 に戻す。
    pub fn set_enabled(&mut self, stage: &str, enabled: bool) -> Result<(), JsError> {
        animation::set_stage_enabled(self.puppet_mut()?, stage, enabled).map_err(js_error)?;
        self.dirty = true;
        Ok(())
    }

    /// 段 `stage` を動かしているか。モデルが無ければ `false`。
    pub fn is_enabled(&self, stage: &str) -> Result<bool, JsError> {
        match &self.puppet {
            Some(puppet) => animation::stage_enabled(puppet, stage).map_err(js_error),
            None => Ok(false),
        }
    }

    // --- 物理 ---

    /// 物理の設定 (`*.physics.json` の形) を差し替える。`undefined` で外す。
    /// モデルと合わせて `validate_physics` で検査し、エラーなら何も変えない。粒子は静止姿勢から始める。
    pub fn set_physics_json(&mut self, json: Option<String>) -> Result<(), JsError> {
        animation::set_physics_json(self.puppet_mut()?, json.as_deref()).map_err(js_error)?;
        self.dirty = true;
        Ok(())
    }

    /// 本体の物理の設定 (`*.physics.json` の形) を差し替える。`undefined` で外す。着せ替えのモジュールを付けていれば
    /// 合成し直し、この後の付け外しでもこの設定から合成する (`set_physics_json` で渡した設定は次の付け外しで失われる)。
    /// 本体のモデルと合成したモデルに対して検査し、失敗すれば何も変えない。粒子は静止姿勢から始める。
    pub fn set_base_physics_json(&mut self, json: Option<String>) -> Result<(), JsError> {
        animation::set_base_physics_json(self.puppet_mut()?, json.as_deref()).map_err(js_error)?;
        self.dirty = true;
        Ok(())
    }

    /// 本体の物理の設定の JSON (モジュールを付ける前の設定。付けたことがなければ今の設定)。物理が無ければ `undefined`。
    pub fn base_physics_json(&self) -> Result<Option<String>, JsError> {
        match &self.puppet {
            Some(puppet) => animation::base_physics_json(puppet).map_err(js_error),
            None => Ok(None),
        }
    }

    /// 今の物理の設定の JSON。物理が無ければ `undefined`。
    pub fn physics_json(&self) -> Result<Option<String>, JsError> {
        match &self.puppet {
            Some(puppet) => animation::physics_json(puppet).map_err(js_error),
            None => Ok(None),
        }
    }

    pub fn has_physics(&self) -> bool {
        self.puppet.as_ref().is_some_and(|p| p.physics().is_some())
    }

    /// 動的物理 (揺れ用パラメータのいらない物理) を使う設定か。
    pub fn is_dynamic_physics(&self) -> bool {
        self.puppet
            .as_ref()
            .and_then(Puppet::physics)
            .is_some_and(|p| p.is_dynamic())
    }

    /// 物理の粒子を初期姿勢に戻す (次の `update` で静止姿勢に置き直す)。
    pub fn reset_physics(&mut self) {
        if let Some(puppet) = &mut self.puppet {
            puppet.reset_physics();
        }
    }

    // --- 標準の当たり判定 (`Model.avatar.hit_areas`、座標はキャンバス座標) ---

    /// 標準の名前 (`"Head"`・`"Body"`) に対応する当たり判定の ID。対応が無ければ `undefined`。
    pub fn standard_hit_area(&self, name: &str) -> Option<String> {
        self.puppet
            .as_ref()?
            .standard_hit_area(name)
            .map(str::to_owned)
    }

    /// 標準の名前 `name` の当たり判定がキャンバス座標 `(x, y)` を含むか。対応が無ければ `false`。
    pub fn hit_test_standard(&mut self, name: &str, x: f32, y: f32) -> bool {
        self.evaluate();
        self.puppet
            .as_ref()
            .is_some_and(|p| p.hit_test_standard(name, Vec2::new(x, y)))
    }

    /// キャンバス座標 `(x, y)` を含む当たり判定の標準の名前 (手前から奥へ)。
    pub fn standard_hit_areas_at(&mut self, x: f32, y: f32) -> Vec<String> {
        self.evaluate();
        self.puppet
            .as_ref()
            .map(|p| {
                let names = p.standard_hit_areas_at(Vec2::new(x, y));
                names.into_iter().map(str::to_owned).collect()
            })
            .unwrap_or_default()
    }
}

impl WebPuppet {
    fn puppet_mut(&mut self) -> Result<&mut Puppet, JsError> {
        self.puppet
            .as_mut()
            .ok_or_else(|| JsError::new("no model is loaded"))
    }
}
