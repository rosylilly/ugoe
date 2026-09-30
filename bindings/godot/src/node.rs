//! `UgoePuppet2D`: `.ugoe` のモデル 1 体を表示して動かす 2D ノード。

use std::collections::HashMap;
use std::sync::Arc;

use godot::classes::{Engine, INode2D, Node2D, Resource, ResourceLoader};
use godot::prelude::*;
use godot::register::info::{PropertyHint, PropertyHintInfo, PropertyInfo, PropertyUsageFlags};
use ugoe::DressUpChange;
use ugoe::core::Rgb;
use ugoe::model::avatar::ARKIT_BLEND_SHAPES;
use ugoe::motion::{BodyInput, Priority, Rotation, TrackingInput};
use ugoe::render::{Light, Lighting};
use ugoe::runtime::ParamIndex;

use crate::model::{LightUniforms, LoadedModel, ModuleData};
use crate::render::{Renderer, TextureCache};

/// インスペクタに出すパラメータのプロパティ名の接頭辞 (`parameters/<ID>`)。
const PARAMETER_PREFIX: &str = "parameters/";

/// `.ugoe` のモデル 1 体を表示して動かす 2D ノード。
///
/// ノードの原点にモデルのキャンバスの原点 (`canvas.origin`) が来る。座標はキャンバスのピクセル単位 (Y 下向き) で、
/// Godot の 2D と同じ向き。毎フレーム `ugoe::Puppet::update` (モーション・表情・目パチ・呼吸・物理・リップシンク・
/// トラッキング・ポーズ・評価) を進めて描く。パラメータはインスペクタの `parameters/<ID>` か
/// [`UgoePuppet2D::set_parameter`] で設定する。
#[derive(GodotClass)]
#[class(base = Node2D, tool)]
pub(crate) struct UgoePuppet2D {
    /// 表示するモデル (`.ugoe` を `load()` した `UgoeModel`。`data` にバイト列を持つリソース)。
    #[var(set = set_model, hint = RESOURCE_TYPE, hint_string = "UgoeModel", usage_flags = [DEFAULT])]
    model: Option<Gd<Resource>>,
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
    /// 取り付ける着せ替えのモジュール (`.ugoemod` を `load()` した `UgoeModule`。合成の順)。
    /// `attach_module` などで変えると、この配列も変わる。
    #[var(set = set_modules, hint = TYPE_STRING, hint_string = "24/17:UgoeModule", usage_flags = [DEFAULT])]
    modules: VarArray,
    /// 環境光 (`docs/spec/rendering.md#ライティング`)。1 を超えてもよい。
    #[export_group(name = "Lighting")]
    #[export]
    #[var(set = set_ambient_light)]
    ambient_light: Color,
    /// 光源。`{ "type": "directional", "direction": Vector3, "color": Color }` か
    /// `{ "type": "point", "position": Vector3, "color": Color, "range": float }` (キャンバス座標 + 奥行き)。
    #[var(set = set_lights, hint = TYPE_STRING, hint_string = "27:", usage_flags = [DEFAULT])]
    lights: VarArray,

    loaded: Option<LoadedModel>,
    /// `ambient_light` と `lights` をシェーダの形に直したもの。
    lighting: LightUniforms,
    /// トラッキングの入力 (モデルを読み直しても保つ)。
    tracking: Option<TrackingInput>,
    /// 付けているモジュールのリソースと、読んだ中身 (付け外しで読み直さないため)。
    attached: Vec<(Gd<Resource>, Arc<ModuleData>)>,
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
            .field("model", &self.model)
            .field("loaded", &self.loaded.is_some())
            .finish_non_exhaustive()
    }
}

#[godot_api]
impl INode2D for UgoePuppet2D {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            model: None,
            playing: true,
            speed: 1.0,
            play_in_editor: false,
            mask_scale: 1.0,
            modules: VarArray::new(),
            ambient_light: Color::WHITE,
            lights: VarArray::new(),
            loaded: None,
            lighting: LightUniforms::new(&Lighting::default()),
            tracking: None,
            attached: Vec::new(),
            renderer: None,
            parameters: HashMap::new(),
            dirty: false,
            base,
        }
    }

    fn ready(&mut self) {
        if self.loaded.is_none() && self.model.is_some() {
            self.load_resource();
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

    /// 表示するモデルを変える。木の中にあればすぐに読み込む (null ならモデルを外す)。
    #[func]
    pub fn set_model(&mut self, model: Option<Gd<Resource>>) {
        self.model = model;
        if !self.base().is_inside_tree() {
            return;
        }
        if self.model.is_some() {
            self.load_resource();
        } else {
            self.unload();
        }
    }

    #[func]
    pub fn set_mask_scale(&mut self, scale: f32) {
        self.mask_scale = scale.max(0.01);
        self.dirty = true;
    }

    /// `.ugoe` をパスから (`ResourceLoader` で) 読み込み、`model` に設定する。成功したら true。
    #[func]
    pub fn load_model(&mut self, path: GString) -> bool {
        let Some(model) = ResourceLoader::singleton().load(&path) else {
            godot_error!("ugoe: `{path}` is not a .ugoe model");
            self.model = None;
            self.unload();
            return false;
        };
        self.model = Some(model);
        self.load_resource()
    }

    /// `.ugoe` のバイト列から読み込む。成功したら true。
    #[func]
    ///
    /// `model` は null になる (シーンには保存されない)。保存したければ `UgoeModel.from_buffer()` を `model` に設定する。
    pub fn load_model_from_buffer(&mut self, bytes: PackedByteArray) -> bool {
        self.model = None;
        self.load_bytes(&bytes, "(buffer)")
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

    // --- 着せ替え (docs/spec/modules.md#puppet-での着せ替え)

    /// 取り付けるモジュールの組を変える (空なら本体だけに戻す)。取り付けられなければ、エラーを出して元に戻す。
    #[func]
    pub fn set_modules(&mut self, modules: VarArray) {
        let list = modules
            .iter_shared()
            .filter_map(|v| v.try_to::<Gd<Resource>>().ok())
            .collect();
        self.apply_modules(list);
    }

    /// モジュール (`UgoeModule`) を取り付ける。同じスロットか同じ名前のモジュールが付いていれば、
    /// それと差し替える (合成の順の同じ位置に入る)。成功したら true。
    #[func]
    pub fn attach_module(&mut self, module: Gd<Resource>) -> bool {
        let Some(data) = self.parse_module(&module) else {
            return false;
        };
        let mut list: Vec<(Gd<Resource>, Arc<ModuleData>)> = self
            .module_resources()
            .into_iter()
            .filter_map(|r| self.parse_module(&r).map(|d| (r, d)))
            .collect();
        let (name, slot) = (data.name().to_owned(), data.module.slot.clone());
        let same = |d: &ModuleData| d.name() == name || d.module.slot == slot;
        match list.iter().position(|(_, d)| same(d)) {
            Some(i) => {
                list[i] = (module, data);
                let mut k = 0;
                list.retain(|(_, d)| {
                    k += 1;
                    k - 1 == i || !same(d)
                });
            }
            None => list.push((module, data)),
        }
        self.apply_modules(list.into_iter().map(|(r, _)| r).collect())
    }

    /// 名前が `name` のモジュールを外す。付いていなければ false。
    #[func]
    pub fn detach_module(&mut self, name: GString) -> bool {
        let name = name.to_string();
        let resources = self.module_resources();
        let before = resources.len();
        let list: Vec<Gd<Resource>> = resources
            .into_iter()
            .filter(|r| self.parse_module(r).is_none_or(|d| d.name() != name))
            .collect();
        if list.len() == before {
            godot_warn!("ugoe: module `{name}` is not attached");
            return false;
        }
        self.apply_modules(list)
    }

    /// すべてのモジュールを外す。
    #[func]
    pub fn clear_modules(&mut self) {
        self.apply_modules(Vec::new());
    }

    /// 取り付けているモジュールの名前 (合成の順)。
    #[func]
    pub fn get_module_names(&self) -> PackedStringArray {
        self.module_resources()
            .iter()
            .filter_map(|r| self.parse_module(r))
            .map(|d| GString::from(d.name()))
            .collect()
    }

    // --- ライティング (docs/spec/rendering.md#ライティング)

    /// このモデルにライティングを受けるメッシュがあるか (`Model::lighting` と `Mesh::lit` の両方が true)。
    /// 光源の操作を出すかの判断に使う。
    #[func]
    pub fn uses_lighting(&self) -> bool {
        self.loaded
            .as_ref()
            .is_some_and(|l| l.puppet.uses_lighting())
    }

    #[func]
    pub fn set_ambient_light(&mut self, color: Color) {
        self.ambient_light = color;
        self.update_lighting();
    }

    #[func]
    pub fn set_lights(&mut self, lights: VarArray) {
        self.lights = lights;
        self.update_lighting();
    }

    /// 平行光源を足す。`direction` は光が進む向き (キャンバス座標 + 奥行き。例: 左上の手前から照らすなら `(1, 1, -1)`)。
    #[func]
    pub fn add_directional_light(&mut self, direction: Vector3, color: Color) {
        let mut light = VarDictionary::new();
        light.set("type", "directional");
        light.set("direction", direction);
        light.set("color", color);
        self.lights.push(&light.to_variant());
        self.update_lighting();
    }

    /// 点光源を足す。`position` はキャンバス座標 + 奥行き (手前が正)。`range` で 0 になるように減衰する
    /// (`INF` なら減衰しない)。
    #[func]
    pub fn add_point_light(&mut self, position: Vector3, color: Color, range: f32) {
        let mut light = VarDictionary::new();
        light.set("type", "point");
        light.set("position", position);
        light.set("color", color);
        light.set("range", range);
        self.lights.push(&light.to_variant());
        self.update_lighting();
    }

    /// 光源をすべて外す (環境光は変えない)。
    #[func]
    pub fn clear_lights(&mut self) {
        self.lights = VarArray::new();
        self.update_lighting();
    }

    // --- トラッキング (docs/spec/avatar-profile.md#フェイストラッキングとの対応)

    /// 頭の回転 (度) を設定する。`x` = ヨー (キャラクターから見て左が正)、`y` = ピッチ (上が正)、
    /// `z` = ロール (時計回りが正)。トラッキングを始める。
    #[func]
    pub fn set_tracking_head(&mut self, rotation: Vector3) {
        self.tracking_mut().head = Rotation {
            yaw: rotation.x,
            pitch: rotation.y,
            roll: rotation.z,
        };
        self.push_tracking();
    }

    /// 体の入力を設定する。キーは `yaw` `pitch` `roll` (度) と `shoulder_left` `shoulder_right`
    /// (0 が中立、1 ですくめる)。無いキーは省略 (体の回転は頭の回転から、肩は書き込まない)。
    #[func]
    pub fn set_tracking_body(&mut self, body: VarDictionary) {
        let get = |key: &str| body.get(key).and_then(|v| v.try_to::<f32>().ok());
        self.tracking_mut().body = BodyInput {
            yaw: get("yaw"),
            pitch: get("pitch"),
            roll: get("roll"),
            shoulder_left: get("shoulder_left"),
            shoulder_right: get("shoulder_right"),
        };
        self.push_tracking();
    }

    /// ARKit のブレンドシェイプの値 (0〜1) を名前で設定する (`{ "jawOpen": 0.5, ... }`)。
    /// MediaPipe Face Landmarker のカテゴリ名もそのまま渡せる。知らない名前は無視し、設定した数を返す。
    #[func]
    pub fn set_tracking_blend_shapes(&mut self, values: VarDictionary) -> i32 {
        let input = self.tracking_mut();
        let mut count = 0;
        for (key, value) in values.iter_shared() {
            let (Ok(name), Ok(value)) = (key.try_to::<GString>(), value.try_to::<f32>()) else {
                continue;
            };
            if input.set_blend_shape(&name.to_string(), value) {
                count += 1;
            }
        }
        self.push_tracking();
        count
    }

    /// ARKit のブレンドシェイプの値を 1 つ設定する。知らない名前なら false。
    #[func]
    pub fn set_tracking_blend_shape(&mut self, name: GString, value: f32) -> bool {
        let ok = self
            .tracking_mut()
            .set_blend_shape(&name.to_string(), value);
        self.push_tracking();
        ok
    }

    /// トラッキングをやめる。最後に書き込んだ値は基準値として残る。
    #[func]
    pub fn clear_tracking(&mut self) {
        self.tracking = None;
        self.push_tracking();
    }

    #[func]
    pub fn is_tracking(&self) -> bool {
        self.tracking.is_some()
    }

    /// トラッキングで使う ARKit のブレンドシェイプの名前 (52 個)。
    #[func]
    pub fn get_tracking_blend_shape_names(&self) -> PackedStringArray {
        ARKIT_BLEND_SHAPES
            .iter()
            .map(|n| GString::from(*n))
            .collect()
    }

    /// 時間を `delta` 秒進めて描く (`playing` が false のときに手で進める用)。
    #[func]
    pub fn advance(&mut self, delta: f32) {
        self.step(delta);
    }
}

/// 光源の辞書を読む。形が違えば `None`。
fn light_from_dictionary(d: &VarDictionary) -> Option<Light> {
    let color = d
        .get("color")
        .map_or(Ok(Color::WHITE), |v| v.try_to::<Color>())
        .ok()?;
    let color = Rgb::new(color.r, color.g, color.b);
    let vector = |key: &str| {
        let v = d.get(key)?.try_to::<Vector3>().ok()?;
        Some([v.x, v.y, v.z])
    };
    match d
        .get("type")?
        .try_to::<GString>()
        .ok()?
        .to_string()
        .as_str()
    {
        "directional" => Some(Light::Directional {
            direction: vector("direction")?,
            color,
        }),
        "point" => Some(Light::Point {
            position: vector("position")?,
            color,
            range: d
                .get("range")
                .map_or(Ok(f32::INFINITY), |v| v.try_to::<f32>())
                .ok()?,
        }),
        _ => None,
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

    /// `model` のリソースから読み込む。
    fn load_resource(&mut self) -> bool {
        let Some(model) = &self.model else {
            self.unload();
            return false;
        };
        let path = model.get_path();
        let Ok(bytes) = model.get("data").try_to::<PackedByteArray>() else {
            godot_error!("ugoe: `{path}` is not a UgoeModel (it has no `data`)");
            self.unload();
            return false;
        };
        self.load_bytes(&bytes, &path.to_string())
    }

    /// `.ugoe` のバイト列から読み込む。`what` はエラーの表示に使う。
    fn load_bytes(&mut self, bytes: &PackedByteArray, what: &str) -> bool {
        match LoadedModel::from_bundle_bytes(bytes.as_slice()) {
            Ok(loaded) => {
                self.install(loaded);
                true
            }
            Err(e) => {
                godot_error!("ugoe: `{what}`: {e}");
                self.unload();
                false
            }
        }
    }

    fn install(&mut self, mut loaded: LoadedModel) {
        self.unload();
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
        loaded.puppet.set_tracking(self.tracking);
        self.loaded = Some(loaded);
        let modules = self.module_resources();
        if !modules.is_empty() && !self.apply_modules(modules) {
            godot_warn!("ugoe: showing the model without the modules");
        }
        self.rebuild_renderer(Vec::new());
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

    /// `textures` は使い回すデコード済みのテクスチャ。
    fn rebuild_renderer(&mut self, textures: TextureCache) {
        if let Some(mut renderer) = self.renderer.take() {
            renderer.remove_viewports();
        }
        let Some(loaded) = &self.loaded else {
            return;
        };
        let mut owner = self.to_gd().upcast::<Node2D>();
        self.renderer = Some(Renderer::new(&mut owner, loaded, textures, &self.lighting));
        // 次の `process` (か、読み込みの直後の `step`) で描く。
        self.dirty = true;
    }

    /// `modules` の配列のリソース (リソースでない要素は除く)。
    fn module_resources(&self) -> Vec<Gd<Resource>> {
        self.modules
            .iter_shared()
            .filter_map(|v| v.try_to::<Gd<Resource>>().ok())
            .collect()
    }

    /// モジュールのリソースを読む。付けているものなら読み直さない。読めなければエラーを出して `None`。
    fn parse_module(&self, resource: &Gd<Resource>) -> Option<Arc<ModuleData>> {
        if let Some((_, data)) = self.attached.iter().find(|(r, _)| r == resource) {
            return Some(Arc::clone(data));
        }
        let path = resource.get_path();
        let Ok(bytes) = resource.get("data").try_to::<PackedByteArray>() else {
            godot_error!("ugoe: `{path}` is not a UgoeModule (it has no `data`)");
            return None;
        };
        match ModuleData::from_bytes(bytes.as_slice()) {
            Ok(data) => Some(Arc::new(data)),
            Err(e) => {
                godot_error!("ugoe: module `{path}`: {e}");
                None
            }
        }
    }

    /// 取り付けるモジュールを `list` にする。読み込んだモデルがあれば付け直し、テクスチャを使い回して
    /// レンダラを作り直す。成功したら true。失敗したら何も変えない。
    fn apply_modules(&mut self, list: Vec<Gd<Resource>>) -> bool {
        let mut parsed = Vec::with_capacity(list.len());
        for resource in list {
            let Some(data) = self.parse_module(&resource) else {
                return false;
            };
            parsed.push((resource, data));
        }
        if let Some(loaded) = &mut self.loaded {
            let change =
                match loaded.set_modules(parsed.iter().map(|(_, d)| Arc::clone(d)).collect()) {
                    Ok(change) => change,
                    Err(e) => {
                        godot_error!("ugoe: dress-up failed: {e}");
                        return false;
                    }
                };
            self.after_dress_up(&change);
        }
        self.modules = parsed.iter().map(|(r, _)| r.to_variant()).collect();
        self.attached = parsed;
        true
    }

    /// 付け外しの後始末: テクスチャを使い回してレンダラを作り直し、描き直す。
    fn after_dress_up(&mut self, change: &DressUpChange) {
        let mut textures = self
            .renderer
            .as_mut()
            .map(Renderer::take_textures)
            .unwrap_or_default();
        for &i in &change.textures {
            if let Some(slot) = textures.get_mut(i as usize) {
                *slot = None;
            }
        }
        textures.truncate(change.texture_count as usize);
        if self.renderer.is_some() {
            self.rebuild_renderer(textures);
            self.step(0.0);
        }
    }

    /// `ambient_light` と `lights` をシェーダに渡す。
    fn update_lighting(&mut self) {
        let lights = self
            .lights
            .iter_shared()
            .filter_map(|v| {
                let light = v
                    .try_to::<VarDictionary>()
                    .ok()
                    .and_then(|d| light_from_dictionary(&d));
                if light.is_none() {
                    godot_warn!("ugoe: ignoring a light: {v}");
                }
                light
            })
            .collect();
        let c = self.ambient_light;
        self.lighting = LightUniforms::new(&Lighting {
            ambient: Rgb::new(c.r, c.g, c.b),
            lights,
        });
        if let (Some(renderer), Some(loaded)) = (&mut self.renderer, &self.loaded) {
            renderer.set_lighting(loaded, &self.lighting);
        }
    }

    fn tracking_mut(&mut self) -> &mut TrackingInput {
        self.tracking.get_or_insert_with(TrackingInput::default)
    }

    fn push_tracking(&mut self) {
        let tracking = self.tracking;
        if let Some(loaded) = &mut self.loaded {
            loaded.puppet.set_tracking(tracking);
            self.dirty = true;
        }
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
