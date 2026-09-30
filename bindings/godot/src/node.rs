//! `UgoePuppet2D`: `.ugoe` のモデル 1 体を表示して動かす 2D ノード。

use std::collections::HashMap;

use godot::classes::{Engine, FileAccess, INode2D, Node2D};
use godot::prelude::*;
use godot::register::info::{PropertyHint, PropertyHintInfo, PropertyInfo, PropertyUsageFlags};
use ugoe::motion::Priority;
use ugoe::runtime::ParamIndex;

use crate::model::LoadedModel;
use crate::render::Renderer;

/// インスペクタに出すパラメータのプロパティ名の接頭辞 (`parameters/<ID>`)。
const PARAMETER_PREFIX: &str = "parameters/";

/// `.ugoe` のモデル 1 体を表示して動かす 2D ノード。
///
/// ノードの原点にモデルのキャンバスの原点 (`canvas.origin`) が来る。座標はキャンバスのピクセル単位 (Y 下向き) で、
/// Godot の 2D と同じ向き。毎フレーム `ugoe::Puppet::update` (モーション・表情・目パチ・呼吸・物理・リップシンク・ポーズ・評価)
/// を進めて描く。パラメータはインスペクタの `parameters/<ID>` か [`UgoePuppet2D::set_parameter`] で設定する。
#[derive(GodotClass)]
#[class(base = Node2D, tool)]
pub(crate) struct UgoePuppet2D {
    /// 読み込む `.ugoe` のパス (`res://` など)。書き出すときは、エクスポートの設定で
    /// `*.ugoe` を「リソース以外のファイル」に含める。
    #[export(file = "*.ugoe")]
    #[var(set = set_model_path)]
    model_path: GString,
    /// 時間を進めるか。止めている間もパラメータの変更は反映する。
    #[export]
    playing: bool,
    /// 時間の進む速さ。
    #[export(range = (0.0, 4.0, 0.01, or_greater))]
    speed: f32,
    /// エディタの中でも時間を進めるか (目パチ・呼吸・物理)。
    #[export]
    play_in_editor: bool,
    /// マスクのテクスチャの解像度の倍率。1 で画面上の大きさと同じ解像度
    /// (`docs/spec/rendering.md#精度`)。下げると軽くなるが、マスクの縁が粗くなる。
    #[export(range = (0.1, 4.0, 0.05, or_greater))]
    #[var(set = set_mask_scale)]
    mask_scale: f32,

    loaded: Option<LoadedModel>,
    renderer: Option<Renderer>,
    /// インスペクタやスクリプトで設定したパラメータの値 (ID ごと)。
    /// シーンに保存し、読み込み直したときにも当てる。
    parameters: HashMap<String, f32>,
    /// `update` をまだ反映していない変更があるか (止めている間の描き直し用)。
    dirty: bool,
    base: Base<Node2D>,
}

impl std::fmt::Debug for UgoePuppet2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UgoePuppet2D")
            .field("model_path", &self.model_path)
            .field("loaded", &self.loaded.is_some())
            .finish_non_exhaustive()
    }
}

#[godot_api]
impl INode2D for UgoePuppet2D {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            model_path: GString::new(),
            playing: true,
            speed: 1.0,
            play_in_editor: false,
            mask_scale: 1.0,
            loaded: None,
            renderer: None,
            parameters: HashMap::new(),
            dirty: false,
            base,
        }
    }

    fn ready(&mut self) {
        if self.loaded.is_none() && !self.model_path.is_empty() {
            let path = self.model_path.clone();
            self.load_path(&path);
        }
    }

    fn process(&mut self, delta: f64) {
        if self.loaded.is_none() {
            return;
        }
        let editor = Engine::singleton().is_editor_hint();
        let running = self.playing && (!editor || self.play_in_editor);
        if running {
            self.step(delta as f32 * self.speed);
        } else if self.dirty {
            self.step(0.0);
        }
    }

    fn on_get_property_list(&mut self) -> Vec<PropertyInfo> {
        let Some(loaded) = &self.loaded else {
            return Vec::new();
        };
        let mut list = vec![PropertyInfo::new_group("Parameters", PARAMETER_PREFIX)];
        for p in &loaded.model().parameters {
            let mut info =
                PropertyInfo::new_export::<f32>(&format!("{PARAMETER_PREFIX}{}", p.id.as_str()))
                    .with_hint_info(PropertyHintInfo {
                        hint: PropertyHint::RANGE,
                        hint_string: GString::from(&format!("{},{},0.001", p.min, p.max)),
                    });
            // 設定した (既定値でない) パラメータだけをシーンに保存する。
            if !self.parameters.contains_key(p.id.as_str()) {
                info.usage = PropertyUsageFlags::EDITOR;
            }
            list.push(info);
        }
        list
    }

    fn on_get(&self, property: StringName) -> Option<Variant> {
        let id = property.to_string();
        let id = id.strip_prefix(PARAMETER_PREFIX)?;
        let value = self
            .parameters
            .get(id)
            .copied()
            .or_else(|| self.parameter_default(id))?;
        Some(value.to_variant())
    }

    fn on_set(&mut self, property: StringName, value: Variant) -> bool {
        let name = property.to_string();
        let Some(id) = name.strip_prefix(PARAMETER_PREFIX) else {
            return false;
        };
        let Ok(value) = value.try_to::<f32>() else {
            return false;
        };
        self.set_parameter(id.into(), value);
        true
    }

    fn on_property_get_revert(&self, property: StringName) -> Option<Variant> {
        let name = property.to_string();
        let id = name.strip_prefix(PARAMETER_PREFIX)?;
        self.parameter_default(id).map(|v| v.to_variant())
    }
}

#[godot_api]
impl UgoePuppet2D {
    /// モデルを読み込んだ。
    #[signal]
    fn model_loaded();

    /// 再生中のモーションのイベントが発火した (`value` はイベントの文字列)。
    #[signal]
    fn motion_event(value: GString);

    /// 読み込むパスを変える。木の中にあればすぐに読み込む (空ならモデルを外す)。
    #[func]
    pub fn set_model_path(&mut self, path: GString) {
        self.model_path = path.clone();
        if !self.base().is_inside_tree() {
            return;
        }
        if path.is_empty() {
            self.unload();
        } else {
            self.load_path(&path);
        }
    }

    #[func]
    pub fn set_mask_scale(&mut self, scale: f32) {
        self.mask_scale = scale.max(0.01);
        self.dirty = true;
    }

    /// `.ugoe` をパスから読み込む。成功したら true。
    #[func]
    pub fn load_model(&mut self, path: GString) -> bool {
        self.model_path = path.clone();
        self.load_path(&path)
    }

    /// `.ugoe` のバイト列から読み込む。成功したら true。
    #[func]
    pub fn load_model_from_buffer(&mut self, bytes: PackedByteArray) -> bool {
        match LoadedModel::from_bundle_bytes(bytes.as_slice()) {
            Ok(loaded) => {
                self.install(loaded);
                true
            }
            Err(e) => {
                godot_error!("ugoe: {e}");
                false
            }
        }
    }

    #[func]
    pub fn is_model_loaded(&self) -> bool {
        self.loaded.is_some()
    }

    /// モデルのキャンバスの大きさ (ピクセル)。
    #[func]
    pub fn get_canvas_size(&self) -> Vector2 {
        self.loaded.as_ref().map_or(Vector2::ZERO, |l| {
            let c = &l.model().meta.canvas;
            Vector2::new(c.width, c.height)
        })
    }

    /// モデルのキャンバスの原点 (キャンバス座標)。ノードの原点がここに対応する。
    #[func]
    pub fn get_canvas_origin(&self) -> Vector2 {
        self.loaded.as_ref().map_or(Vector2::ZERO, |l| {
            let o = l.model().meta.canvas.origin;
            Vector2::new(o.x, o.y)
        })
    }

    /// パラメータの ID をモデルの順に並べる。
    #[func]
    pub fn get_parameter_ids(&self) -> PackedStringArray {
        self.loaded
            .as_ref()
            .map(|l| {
                l.model()
                    .parameters
                    .iter()
                    .map(|p| GString::from(p.id.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[func]
    pub fn has_parameter(&self, id: GString) -> bool {
        self.parameter_index(&id.to_string()).is_some()
    }

    /// パラメータの今の値 (直前の更新の後の値。モーションや物理の結果を含む)。
    #[func]
    pub fn get_parameter(&self, id: GString) -> f32 {
        let id = id.to_string();
        match (&self.loaded, self.parameter_index(&id)) {
            (Some(l), Some(i)) => l.puppet.instance().parameters().get(i),
            _ => self.parameters.get(&id).copied().unwrap_or(0.0),
        }
    }

    /// パラメータの値を設定する (値域にクランプする)。モーションなどはこの値を基準にして重ねる。
    #[func]
    pub fn set_parameter(&mut self, id: GString, value: f32) {
        let id = id.to_string();
        let index = self.parameter_index(&id);
        if let (Some(loaded), Some(i)) = (&mut self.loaded, index) {
            let params = loaded.puppet.instance_mut().parameters_mut();
            params.set(i, value);
            let value = params.get(i);
            if value == params.default_value(i) {
                self.parameters.remove(&id);
            } else {
                self.parameters.insert(id, value);
            }
            self.dirty = true;
        } else {
            // 読み込む前 (シーンの読み込み中など) は覚えておき、読み込んだときに当てる。
            self.parameters.insert(id, value);
        }
    }

    /// パラメータの名前・範囲・既定値 (`{ "name", "min", "max", "default" }`)。無ければ空。
    #[func]
    pub fn get_parameter_info(&self, id: GString) -> VarDictionary {
        let id = id.to_string();
        let mut dict = VarDictionary::new();
        if let Some(p) = self
            .loaded
            .as_ref()
            .and_then(|l| l.model().parameters.iter().find(|p| p.id.as_str() == id))
        {
            dict.set("name", &GString::from(p.name.as_str()));
            dict.set("min", p.min);
            dict.set("max", p.max);
            dict.set("default", p.default);
        }
        dict
    }

    /// すべてのパラメータを既定値に戻す。
    #[func]
    pub fn reset_parameters(&mut self) {
        self.parameters.clear();
        if let Some(loaded) = &mut self.loaded {
            let params = loaded.puppet.instance_mut().parameters_mut();
            for i in 0..params.len() {
                let i = ParamIndex(i as u32);
                params.set(i, params.default_value(i));
            }
            self.dirty = true;
        }
        self.base_mut().notify_property_list_changed();
    }

    /// バンドルのモーションの名前。
    #[func]
    pub fn get_motion_names(&self) -> PackedStringArray {
        self.loaded
            .as_ref()
            .map(|l| {
                l.motions
                    .iter()
                    .map(|m| GString::from(m.name.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 名前でモーションを開始する。`priority` は 0 = 待機、1 = 普通、2 = 強制
    /// (`docs/spec/motion.md#モーションマネージャ`)。開始したら true。
    #[func]
    pub fn start_motion(&mut self, name: GString, priority: i32) -> bool {
        let name = name.to_string();
        let priority = match priority {
            i32::MIN..=0 => Priority::Idle,
            1 => Priority::Normal,
            _ => Priority::Force,
        };
        let Some(loaded) = &mut self.loaded else {
            return false;
        };
        let Some(motion) = loaded.motions.iter().find(|m| m.name == name).cloned() else {
            godot_warn!("ugoe: motion `{name}` not found");
            return false;
        };
        loaded.puppet.start_motion(motion, priority)
    }

    /// すべてのモーションを (フェードアウトをかけて) 止める。
    #[func]
    pub fn stop_motions(&mut self) {
        if let Some(loaded) = &mut self.loaded {
            loaded.puppet.motions_mut().stop_all();
        }
    }

    #[func]
    pub fn is_motion_playing(&self) -> bool {
        self.loaded
            .as_ref()
            .is_some_and(|l| l.puppet.motions().is_playing())
    }

    /// バンドルの表情の名前。
    #[func]
    pub fn get_expression_names(&self) -> PackedStringArray {
        self.loaded
            .as_ref()
            .map(|l| {
                l.expressions
                    .iter()
                    .map(|e| GString::from(e.name.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 名前で表情を設定する。見つかったら true。
    #[func]
    pub fn set_expression(&mut self, name: GString) -> bool {
        let name = name.to_string();
        let Some(loaded) = &mut self.loaded else {
            return false;
        };
        let Some(expression) = loaded.expressions.iter().find(|e| e.name == name).cloned() else {
            godot_warn!("ugoe: expression `{name}` not found");
            return false;
        };
        loaded.puppet.set_expression(expression);
        true
    }

    /// リップシンクの音量 (0〜1) を設定する。
    #[func]
    pub fn set_lip_sync_level(&mut self, level: f32) {
        if let Some(loaded) = &mut self.loaded {
            loaded.puppet.set_lip_sync_level(Some(level));
        }
    }

    /// リップシンクの入力をやめる。
    #[func]
    pub fn clear_lip_sync_level(&mut self) {
        if let Some(loaded) = &mut self.loaded {
            loaded.puppet.set_lip_sync_level(None);
        }
    }

    /// 物理の状態を初期化する。
    #[func]
    pub fn reset_physics(&mut self) {
        if let Some(loaded) = &mut self.loaded {
            loaded.puppet.reset_physics();
        }
    }

    /// ノードの座標 `local_position` が当たり判定 `hit_area` に入っているか。
    #[func]
    pub fn hit_test(&self, hit_area: GString, local_position: Vector2) -> bool {
        let Some(loaded) = &self.loaded else {
            return false;
        };
        let p = to_canvas(loaded, local_position);
        loaded.puppet.hit_test(&hit_area.to_string(), p)
    }

    /// ノードの座標 `local_position` を含む当たり判定の ID を、手前から順に並べる。
    #[func]
    pub fn get_hit_areas_at(&self, local_position: Vector2) -> PackedStringArray {
        let Some(loaded) = &self.loaded else {
            return PackedStringArray::new();
        };
        let p = to_canvas(loaded, local_position);
        loaded
            .puppet
            .hit_areas_at(p)
            .into_iter()
            .map(GString::from)
            .collect()
    }

    /// 時間を `delta` 秒進めて描く (`playing` が false のときに手で進める用)。
    #[func]
    pub fn advance(&mut self, delta: f32) {
        self.step(delta);
    }
}

fn to_canvas(loaded: &LoadedModel, local: Vector2) -> ugoe::core::Vec2 {
    let o = loaded.model().meta.canvas.origin;
    ugoe::core::Vec2::new(local.x + o.x, local.y + o.y)
}

impl UgoePuppet2D {
    fn parameter_index(&self, id: &str) -> Option<ParamIndex> {
        self.loaded
            .as_ref()?
            .puppet
            .instance()
            .parameters()
            .index_of(id)
    }

    fn parameter_default(&self, id: &str) -> Option<f32> {
        let loaded = self.loaded.as_ref()?;
        let i = self.parameter_index(id)?;
        Some(loaded.puppet.instance().parameters().default_value(i))
    }

    fn load_path(&mut self, path: &GString) -> bool {
        let bytes = FileAccess::get_file_as_bytes(path);
        if bytes.is_empty() {
            godot_error!(
                "ugoe: failed to read `{path}` ({:?})",
                FileAccess::get_open_error()
            );
            self.unload();
            return false;
        }
        match LoadedModel::from_bundle_bytes(bytes.as_slice()) {
            Ok(loaded) => {
                self.install(loaded);
                true
            }
            Err(e) => {
                godot_error!("ugoe: `{path}`: {e}");
                self.unload();
                false
            }
        }
    }

    fn install(&mut self, mut loaded: LoadedModel) {
        self.unload();
        for id in &loaded.masks.truncated {
            godot_warn!(
                "ugoe: mesh `{id}` reads more than {} masks; the rest are ignored",
                crate::model::MAX_MASKS
            );
        }
        // 覚えているパラメータを当てる (モデルに無い ID は捨てる)。
        let params = loaded.puppet.instance_mut().parameters_mut();
        self.parameters
            .retain(|id, value| match params.index_of(id) {
                Some(i) => {
                    params.set(i, *value);
                    true
                }
                None => false,
            });
        self.loaded = Some(loaded);
        self.rebuild_renderer();
        self.step(0.0);
        self.base_mut().notify_property_list_changed();
        self.signals().model_loaded().emit();
    }

    fn unload(&mut self) {
        if let Some(mut renderer) = self.renderer.take() {
            renderer.remove_viewports();
        }
        if self.loaded.take().is_some() {
            self.base_mut().notify_property_list_changed();
        }
    }

    fn rebuild_renderer(&mut self) {
        if let Some(mut renderer) = self.renderer.take() {
            renderer.remove_viewports();
        }
        let Some(loaded) = &self.loaded else {
            return;
        };
        let mut owner = self.to_gd().upcast::<Node2D>();
        self.renderer = Some(Renderer::new(&mut owner, loaded));
        // 次の `process` (か、読み込みの直後の `step`) で描く。
        self.dirty = true;
    }

    fn step(&mut self, dt: f32) {
        // マスクは画面上の大きさ (ビューポートの画素) に合わせた解像度で描く。
        let screen_scale = if self.base().is_inside_tree() {
            let s = self.base().get_global_transform_with_canvas().scale();
            s.x.abs().max(s.y.abs())
        } else {
            1.0
        };
        let mask_scale = self.mask_scale * screen_scale;
        let Some(loaded) = &mut self.loaded else {
            return;
        };
        loaded.puppet.update(dt);
        self.dirty = false;
        let events: Vec<GString> = loaded
            .puppet
            .motion_events()
            .iter()
            .map(|e| GString::from(e.value.as_str()))
            .collect();
        if let Some(renderer) = &mut self.renderer {
            renderer.draw(loaded, mask_scale);
        }
        for value in events {
            self.signals().motion_event().emit(&value);
        }
    }
}
