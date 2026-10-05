//! `UgoePuppet2D`: `.ugoe` のモデル 1 体を表示して動かす 2D ノード。

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use godot::classes::{
    Engine, INode2D, Light2D, Node, Node2D, PointLight2D, RefCounted, Resource, ResourceLoader,
    XrServer,
};
use godot::prelude::*;
use godot::register::info::{PropertyHint, PropertyHintInfo, PropertyInfo, PropertyUsageFlags};
use ugoe::core::Rgb;
use ugoe::lighting::{Light, Lighting};
use ugoe::limits::ReadLimits;
use ugoe::model::avatar::ARKIT_BLEND_SHAPES;
use ugoe::motion::{BodyInput, Priority, Rotation, TrackingInput};
use ugoe::runtime::ParamIndex;
use ugoe::{DressUpChange, DressUpError, FieldValue, Toggles};

use crate::model::{
    ErrorField, LightUniforms, LoadError, LoadLimits, LoadedModel, ModuleData,
    directional_from_light2d, valid_color,
};
use crate::render::{Renderer, TextureCache};
use crate::xr::arkit_from_unified;

mod readback;
mod sdk;

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
    /// 時間の進む速さ。有限で負でない値だけを受け付ける。
    #[export(range = (0.0, 4.0, 0.01, or_greater))]
    #[var(set = set_speed)]
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
    /// シーンの `Light2D` (`PointLight2D`・`DirectionalLight2D`) も光源にするか (`lights` の後ろに足す)。
    /// 同じキャンバスにあり、有効で見えていて、`range_item_cull_mask` がこのノードの `light_mask` と重なるものを使う。
    #[export]
    #[var(set = set_use_scene_lights)]
    use_scene_lights: bool,
    /// フェイストラッキングの入力にする `XRFaceTracker` の名前 (`XRServer` に登録した名前。
    /// 標準は `/user/face_tracker`)。空なら使わない。毎フレーム読み、見つからなければ前の値を保つ。
    #[export_group(name = "Tracking")]
    #[export]
    xr_face_tracker: StringName,
    /// トラッキング中に、再生中のモーションが動かすパラメータではモーションを優先するか
    /// (`docs/spec/motion.md#トラッキング中のモーションの優先`)。既定は false。モデルを読み直しても保つ。
    #[export]
    #[var(set = set_motion_over_tracking)]
    motion_over_tracking: bool,
    /// `motion_over_tracking` でも、トラッキングのままにするパラメータの ID (目など)。モデルに無い ID は無視する。
    /// モデルを読み直しても保つ。
    #[export]
    #[var(set = set_tracking_kept)]
    tracking_kept: PackedStringArray,
    /// 圧縮したチャンク 1 つを展開した後の大きさの上限 (バイト。`docs/spec/file-formats.md#読み込みの上限`)。
    /// 次の読み込み (モデルと着せ替えのモジュール) から効く。負の値は拒む。
    #[export_group(name = "Load limits")]
    #[export]
    #[var(set = set_max_chunk_decompressed)]
    max_chunk_decompressed: i64,
    /// 1 つのファイルで、圧縮したチャンクを展開したバイトの合計の上限。次の読み込みから効く。負の値は拒む。
    #[export]
    #[var(set = set_max_total_decompressed)]
    max_total_decompressed: i64,
    /// テクスチャと法線マップの画像 (PNG・KTX2) の幅と高さそれぞれの上限 (画素)。次の読み込みから効く。
    /// 負の値と 2^32 以上は拒む。
    #[export]
    #[var(set = set_max_image_side)]
    max_image_side: i64,

    loaded: Option<LoadedModel>,
    /// 木の外で `model` を変えたので、木に戻ったときに読み込み直す (`ready` は最初の 1 回しか呼ばれないため)。
    model_pending: bool,
    /// `lights` を読んだもの (読めない要素は除く)。
    manual_lights: Vec<Light>,
    /// シーンの光源のノード (`use_scene_lights` のとき)。
    scene_lights: Vec<Gd<Light2D>>,
    /// 木にノードが足されたか外された (シーンの光源を探し直す)。木の `node_added` と `node_removed` で立てる。
    scene_changed: Rc<Cell<bool>>,
    /// 木の変化を見ている接続 (木から出るときに外す)。
    tree_watch: Option<Callable>,
    /// 光源をシェーダの形に直したもの。
    lighting: LightUniforms,
    /// トラッキングの入力 (モデルを読み直しても保つ)。
    tracking: Option<TrackingInput>,
    /// `update` の各段を動かすか (モデルを読み直しても保つ)。
    toggles: Toggles,
    /// 直前の読み込み (モデルか着せ替えのモジュール) の失敗 (`get_load_error()`)。成功したら空。
    load_error: VarDictionary,
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
        let limits = LoadLimits::default();
        Self {
            model: None,
            playing: true,
            speed: 1.0,
            play_in_editor: false,
            mask_scale: 1.0,
            modules: VarArray::new(),
            ambient_light: Color::WHITE,
            lights: VarArray::new(),
            use_scene_lights: false,
            xr_face_tracker: StringName::default(),
            motion_over_tracking: false,
            tracking_kept: PackedStringArray::new(),
            max_chunk_decompressed: to_i64(limits.read.max_chunk_decompressed),
            max_total_decompressed: to_i64(limits.read.max_total_decompressed),
            max_image_side: limits.max_image_side.into(),
            loaded: None,
            model_pending: false,
            manual_lights: Vec::new(),
            scene_lights: Vec::new(),
            scene_changed: Rc::new(Cell::new(true)),
            tree_watch: None,
            lighting: LightUniforms::new(&Lighting::default()),
            tracking: None,
            toggles: Toggles::default(),
            load_error: VarDictionary::new(),
            attached: Vec::new(),
            renderer: None,
            parameters: HashMap::new(),
            dirty: false,
            base,
        }
    }

    fn enter_tree(&mut self) {
        if self.use_scene_lights {
            self.watch_tree(true);
        }
        // 最初に木に入るときは `ready` で読む。木から出て戻ったときは `ready` が呼ばれないので、ここで読む。
        if self.model_pending && self.base().is_node_ready() {
            self.apply_model();
        }
    }

    fn exit_tree(&mut self) {
        self.watch_tree(false);
    }

    fn ready(&mut self) {
        self.model_pending = false;
        if self.loaded.is_none() && self.model.is_some() {
            self.load_resource();
        }
    }

    fn process(&mut self, delta: f64) {
        if self.loaded.is_none() {
            return;
        }
        let editor = Engine::singleton().is_editor_hint();
        if self.use_scene_lights {
            self.update_scene_lights();
        }
        if !self.xr_face_tracker.is_empty() {
            self.read_xr_face_tracker();
        }
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
            self.model_pending = true;
            return;
        }
        self.apply_model();
    }

    /// 時間の進む速さを変える。有限で負でない値でなければ、エラーを出して変えない。
    #[func]
    pub fn set_speed(&mut self, speed: f32) {
        if !(speed.is_finite() && speed >= 0.0) {
            godot_error!("ugoe: speed must be a finite non-negative number (got {speed})");
            return;
        }
        self.speed = speed;
    }

    /// マスクの解像度の倍率を変える (0.01 より小さければ 0.01)。有限でなければ、エラーを出して変えない。
    #[func]
    pub fn set_mask_scale(&mut self, scale: f32) {
        if !scale.is_finite() {
            godot_error!("ugoe: mask_scale must be finite (got {scale})");
            return;
        }
        self.mask_scale = scale.max(0.01);
        self.dirty = true;
    }

    /// マスクがアトラス (1 辺 4096 画素) に収まらずに、解像度に掛けている倍率 (1 なら下げていない)。
    /// 下げた後にマスクが小さくなれば 1 段ずつ戻る (README の「描画のしかた」)。
    #[func]
    pub fn get_mask_resolution_factor(&self) -> f32 {
        self.renderer.as_ref().map_or(1.0, Renderer::mask_shrink)
    }

    // --- 読み込みの上限 (docs/spec/file-formats.md#読み込みの上限)

    /// 展開した 1 チャンクの上限 (バイト) を変える。負ならエラーを出して変えない。
    #[func]
    pub fn set_max_chunk_decompressed(&mut self, bytes: i64) {
        if bytes < 0 {
            godot_error!("ugoe: max_chunk_decompressed must not be negative (got {bytes})");
            return;
        }
        self.max_chunk_decompressed = bytes;
    }

    /// 展開したバイトの合計の上限を変える。負ならエラーを出して変えない。
    #[func]
    pub fn set_max_total_decompressed(&mut self, bytes: i64) {
        if bytes < 0 {
            godot_error!("ugoe: max_total_decompressed must not be negative (got {bytes})");
            return;
        }
        self.max_total_decompressed = bytes;
    }

    /// 画像の一辺の上限 (画素) を変える。負か 2^32 以上ならエラーを出して変えない。
    #[func]
    pub fn set_max_image_side(&mut self, pixels: i64) {
        if u32::try_from(pixels).is_err() {
            godot_error!("ugoe: max_image_side must be in 0..2^32 (got {pixels})");
            return;
        }
        self.max_image_side = pixels;
    }

    /// 直前の読み込み (`model`・`load_model*`、着せ替えのモジュールの付け外し) の失敗。成功したら空の辞書。
    ///
    /// `{ "code", "error", ... }` で、`code` と補足のキーは wasm の読み込みのエラー (`LoadError` / `DressUpError` の
    /// `code` とプロパティ) と同じ。上限を超えたら `read_limit_exceeded` (`limit`・`chunk`・`max`) か
    /// `image_limit_exceeded` (`image`・`texture`・`width`・`height`・`max`)。`error` は英語の文 (安定した値ではない)。
    #[func]
    pub fn get_load_error(&self) -> VarDictionary {
        self.load_error.clone()
    }

    /// `.ugoe` をパスから (`ResourceLoader` で) 読み込み、`model` に設定する。成功したら true。
    #[func]
    pub fn load_model(&mut self, path: GString) -> bool {
        let Some(model) = ResourceLoader::singleton().load(&path) else {
            self.model = None;
            self.fail_invalid_file(&format!("`{path}` is not a .ugoe model"));
            return false;
        };
        self.model = Some(model);
        self.load_resource()
    }

    /// `.ugoe` のバイト列から読み込む。成功したら true。
    ///
    /// `model` は null になる (シーンには保存されない)。保存したければ `UgoeModel.from_buffer()` を `model` に設定する。
    #[func]
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
    /// NaN と、読み込んだモデルに無い ID は、エラーか警告を出して無視する。
    #[func]
    pub fn set_parameter(&mut self, id: GString, value: f32) {
        let id = id.to_string();
        if value.is_nan() {
            godot_error!("ugoe: parameter `{id}` cannot be NaN");
            return;
        }
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
        } else if self.loaded.is_some() {
            godot_warn!("ugoe: parameter `{id}` not found");
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

    /// リップシンクの音量 (0〜1) を設定する。負の値 (`-INF` を含む) は入力をやめる (`clear_lip_sync_level` と同じ。
    /// C ABI の `ugoe_puppet_set_lip_sync_level`・wasm・Unity と同じ)。NaN ならエラーを出して変えない。
    #[func]
    pub fn set_lip_sync_level(&mut self, level: f32) {
        if level.is_nan() {
            godot_error!("ugoe: lip sync level cannot be NaN");
            return;
        }
        if let Some(loaded) = &mut self.loaded {
            loaded
                .puppet
                .set_lip_sync_level((level >= 0.0).then_some(level));
        }
    }

    /// リップシンクの入力をやめる (`set_lip_sync_level` に負の値を渡すのと同じ)。
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

    /// `update` の段 `stage` (`"physics"`・`"eye_blink"`・`"breath"`・`"lip_sync"`・`"pose"`) を動かすかを設定する
    /// (`docs/spec/motion.md#段の有効無効`)。モデルを読み直しても保つので、読み込む前に設定すれば、読み込みの直後の
    /// 評価から効く。未知の名前なら false。
    #[func]
    pub fn set_stage_enabled(&mut self, stage: GString, enabled: bool) -> bool {
        let Some(flag) = stage_flag(&mut self.toggles, &stage.to_string()) else {
            godot_error!("ugoe: unknown stage `{stage}`");
            return false;
        };
        *flag = enabled;
        if let Some(loaded) = &mut self.loaded {
            loaded.puppet.set_toggles(self.toggles);
        }
        self.dirty = true;
        true
    }

    /// `update` の段 `stage` を動かしているか。未知の名前なら false。
    #[func]
    pub fn is_stage_enabled(&self, stage: GString) -> bool {
        let mut toggles = self.toggles;
        stage_flag(&mut toggles, &stage.to_string()).is_some_and(|f| *f)
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
        let mut list: Vec<(Gd<Resource>, Arc<ModuleData>)> = Vec::new();
        for r in self.module_resources() {
            if let Some(d) = self.parse_module(&r) {
                list.push((r, d));
            }
        }
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
        self.apply_parsed_modules(list)
    }

    /// 名前が `name` のモジュールを外す。付いていなければ false。
    #[func]
    pub fn detach_module(&mut self, name: GString) -> bool {
        let name = name.to_string();
        let resources = self.module_resources();
        let before = resources.len();
        let list: Vec<Gd<Resource>> = resources
            .into_iter()
            .filter(|r| self.read_module(r).ok().is_none_or(|d| d.name() != name))
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

    /// 取り付けているモジュールの名前 (合成の順)。モデルを読み込んでいれば、実際に付いているものだけ
    /// (読み込むときに付けられなかったモジュールは `modules` に残るが、ここには出ない)。
    #[func]
    pub fn get_module_names(&self) -> PackedStringArray {
        if let Some(loaded) = &self.loaded {
            return loaded
                .modules()
                .iter()
                .map(|d| GString::from(d.name()))
                .collect();
        }
        self.module_resources()
            .iter()
            .filter_map(|r| self.read_module(r).ok())
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

    /// 環境光を変える。色が有限で負でない値でなければ、エラーを出して変えない。
    #[func]
    pub fn set_ambient_light(&mut self, color: Color) {
        if !valid_color(Rgb::new(color.r, color.g, color.b)) {
            godot_error!("ugoe: ambient_light must be finite and non-negative (got {color})");
            return;
        }
        self.ambient_light = color;
        self.update_lighting();
    }

    #[func]
    pub fn set_lights(&mut self, lights: VarArray) {
        self.lights = lights;
        self.read_lights();
    }

    /// 平行光源を足す。`direction` は光が進む向き (キャンバス座標 + 奥行き。例: 左上の手前から照らすなら `(1, 1, -1)`)。
    #[func]
    pub fn add_directional_light(&mut self, direction: Vector3, color: Color) {
        let light = light_to_dictionary(&Light::Directional {
            direction: [direction.x, direction.y, direction.z],
            color: Rgb::new(color.r, color.g, color.b),
        });
        self.lights.push(&light.to_variant());
        self.read_lights();
    }

    /// 点光源を足す。`position` はキャンバス座標 + 奥行き (手前が正)。`range` で 0 になるように減衰する
    /// (`INF` なら減衰しない)。
    #[func]
    pub fn add_point_light(&mut self, position: Vector3, color: Color, range: f32) {
        let light = light_to_dictionary(&Light::Point {
            position: [position.x, position.y, position.z],
            color: Rgb::new(color.r, color.g, color.b),
            range,
        });
        self.lights.push(&light.to_variant());
        self.read_lights();
    }

    /// 光源をすべて外す (環境光とシーンの光源は変えない)。
    #[func]
    pub fn clear_lights(&mut self) {
        self.lights = VarArray::new();
        self.read_lights();
    }

    #[func]
    pub fn set_use_scene_lights(&mut self, enabled: bool) {
        self.use_scene_lights = enabled;
        self.scene_changed.set(true);
        if self.base().is_inside_tree() {
            self.watch_tree(enabled);
        }
        if enabled {
            self.update_scene_lights();
        } else {
            self.scene_lights.clear();
            self.update_lighting();
        }
    }

    /// シーンの `Light2D` から求めた今の光源 (`lights` と同じ形の辞書。キャンバス座標)。
    /// `use_scene_lights` が false なら空。
    #[func]
    pub fn get_scene_lights(&mut self) -> Array<VarDictionary> {
        if !self.use_scene_lights {
            return Array::new();
        }
        self.find_scene_lights();
        self.scene_light_values()
            .iter()
            .map(light_to_dictionary)
            .collect()
    }

    // --- トラッキング (docs/spec/avatar-profile.md#フェイストラッキングとの対応)

    /// 頭の回転 (度) を設定する。`x` = ヨー (キャラクターから見て左が正)、`y` = ピッチ (上が正)、
    /// `z` = ロール (時計回りが正)。トラッキングを始める。有限でなければ、エラーを出して変えない。
    #[func]
    pub fn set_tracking_head(&mut self, rotation: Vector3) {
        if !rotation.is_finite() {
            godot_error!("ugoe: the head rotation must be finite (got {rotation})");
            return;
        }
        self.tracking_mut().head = Rotation {
            yaw: rotation.x,
            pitch: rotation.y,
            roll: rotation.z,
        };
        self.push_tracking();
    }

    /// 体の入力を設定する。キーは `yaw` `pitch` `roll` (度) と `shoulder_left` `shoulder_right`
    /// (0 が中立、1 ですくめる)。無いキーと NaN は省略 (体の回転は頭の回転から、肩は書き込まない)。
    /// 無限大があれば、エラーを出して変えない。
    #[func]
    pub fn set_tracking_body(&mut self, body: VarDictionary) {
        let get = |key: &str| {
            body.get(key)
                .and_then(|v| v.try_to::<f32>().ok())
                .filter(|v| !v.is_nan())
        };
        let input = BodyInput {
            yaw: get("yaw"),
            pitch: get("pitch"),
            roll: get("roll"),
            shoulder_left: get("shoulder_left"),
            shoulder_right: get("shoulder_right"),
        };
        let values = [
            input.yaw,
            input.pitch,
            input.roll,
            input.shoulder_left,
            input.shoulder_right,
        ];
        if values.iter().flatten().any(|v| v.is_infinite()) {
            godot_error!("ugoe: the body input must be finite ({body})");
            return;
        }
        self.tracking_mut().body = input;
        self.push_tracking();
    }

    /// ARKit のブレンドシェイプの値 (0〜1) を名前で設定する (`{ "jawOpen": 0.5, ... }`)。
    /// MediaPipe Face Landmarker のカテゴリ名もそのまま渡せる。知らない名前と有限でない値は無視し、設定した数を返す。
    #[func]
    pub fn set_tracking_blend_shapes(&mut self, values: VarDictionary) -> i32 {
        let input = self.tracking_mut();
        let mut count = 0;
        for (key, value) in values.iter_shared() {
            let (Ok(name), Ok(value)) = (key.try_to::<GString>(), value.try_to::<f32>()) else {
                continue;
            };
            if !value.is_finite() {
                continue;
            }
            if input.set_blend_shape(&name.to_string(), value) {
                count += 1;
            }
        }
        self.push_tracking();
        count
    }

    /// ARKit のブレンドシェイプの値を 1 つ設定する。知らない名前か有限でない値なら false。
    #[func]
    pub fn set_tracking_blend_shape(&mut self, name: GString, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
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

    /// トラッキング中に、再生中のモーションが動かすパラメータではモーションを優先するかを設定する (T0722)。
    /// true の間はトラッキングの値をモーションの前に書き、モーションがフェードの重みでその上に重なる。
    #[func]
    pub fn set_motion_over_tracking(&mut self, enabled: bool) {
        self.motion_over_tracking = enabled;
        if let Some(loaded) = &mut self.loaded {
            loaded.puppet.set_motion_over_tracking(enabled);
            self.dirty = true;
        }
    }

    /// `motion_over_tracking` でも、トラッキングのままにするパラメータの ID を設定する (空なら無し)。
    #[func]
    pub fn set_tracking_kept(&mut self, ids: PackedStringArray) {
        self.tracking_kept = ids;
        self.push_tracking_kept();
    }

    /// `XRFaceTracker` (Unified Expressions) のブレンドシェイプを、ARKit の 52 ブレンドシェイプに直して設定する
    /// (対応は `src/xr.rs` と README の表)。頭の回転は `XRFaceTracker` に無いので変えない (`set_tracking_head` で渡す)。
    /// トラッキングを始める。`XRFaceTracker` でなければ false。
    #[func]
    pub fn set_tracking_from_xr_face_tracker(&mut self, tracker: Gd<RefCounted>) -> bool {
        if !tracker.is_class("XRFaceTracker") {
            godot_error!("ugoe: {tracker} is not an XRFaceTracker");
            return false;
        }
        // `XRFaceTracker` は godot-rust では実験的な API (機能 `experimental-godot-api`) なので、名前で呼ぶ。
        let values = tracker
            .clone()
            .upcast::<Object>()
            .call("get_blend_shapes", &[])
            .try_to::<PackedFloat32Array>()
            .unwrap_or_default();
        let input = self.tracking_mut();
        for (name, value) in arkit_from_unified(values.as_slice()) {
            input.set_blend_shape(name, value);
        }
        self.push_tracking();
        true
    }

    /// 時間を `delta` 秒進めて描く (`playing` が false のときに手で進める用)。
    /// `delta` が有限で負でない値でなければ、エラーを出して何もしない。
    #[func]
    pub fn advance(&mut self, delta: f32) {
        if !(delta.is_finite() && delta >= 0.0) {
            godot_error!("ugoe: delta must be a finite non-negative number (got {delta})");
            return;
        }
        self.step(delta);
    }
}

/// 段の名前 (`ugoe-wasm` の `set_enabled` と同じ) に対応する `Toggles` の値。
fn stage_flag<'a>(toggles: &'a mut Toggles, stage: &str) -> Option<&'a mut bool> {
    match stage {
        "physics" => Some(&mut toggles.physics),
        "eye_blink" => Some(&mut toggles.eye_blink),
        "breath" => Some(&mut toggles.breath),
        "lip_sync" => Some(&mut toggles.lip_sync),
        "pose" => Some(&mut toggles.pose),
        _ => None,
    }
}

/// 光源を辞書 (`lights` の要素の形) にする。
fn light_to_dictionary(light: &Light) -> VarDictionary {
    let mut d = VarDictionary::new();
    let (kind, key, [x, y, z], color) = match *light {
        Light::Directional { direction, color } => ("directional", "direction", direction, color),
        Light::Point {
            position, color, ..
        } => ("point", "position", position, color),
    };
    d.set("type", kind);
    d.set(key, Vector3::new(x, y, z));
    d.set("color", Color::from_rgb(color.r, color.g, color.b));
    if let Light::Point { range, .. } = *light {
        d.set("range", range);
    }
    d
}

/// 光源の辞書を読む。形が違えば `None`。
fn light_from_dictionary(d: &VarDictionary) -> Option<Light> {
    let color = d
        .get("color")
        .map_or(Ok(Color::WHITE), |v| v.try_to::<Color>())
        .ok()?;
    let color = Rgb::new(color.r, color.g, color.b);
    if !valid_color(color) {
        return None;
    }
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

/// 上限のバイト数をプロパティの整数にする (`i64` に収まらなければ最大値)。
fn to_i64(bytes: u64) -> i64 {
    i64::try_from(bytes).unwrap_or(i64::MAX)
}

/// `get_load_error()` の辞書: `{ "code", "error", ...fields }`。
fn error_dictionary(
    code: &str,
    error: &str,
    fields: impl IntoIterator<Item = (&'static str, Variant)>,
) -> VarDictionary {
    let mut d = VarDictionary::new();
    d.set("code", code);
    d.set("error", error);
    for (key, value) in fields {
        d.set(key, &value);
    }
    d
}

/// 読み込みの失敗 ([`LoadError::info`]) の辞書。
fn load_error_dictionary(error: &LoadError) -> VarDictionary {
    let (code, fields) = error.info();
    let fields = fields.into_iter().map(|(key, value)| {
        let value = match value {
            ErrorField::Int(i) => i.to_variant(),
            ErrorField::Text(t) => t.to_variant(),
        };
        (key, value)
    });
    error_dictionary(code, &error.to_string(), fields)
}

/// 着せ替えの失敗の補足の値 ([`DressUpError::fields`]) を Godot の値にする (番号は整数)。
fn field_variant(value: FieldValue<'_>) -> Variant {
    match value {
        FieldValue::Index(i) => i64::try_from(i).unwrap_or(i64::MAX).to_variant(),
        FieldValue::Text(t) => t.to_variant(),
        // 種類の一覧は wasm・C ABI と同じく `, ` でつないだ文 (T0168)。
        FieldValue::Codes(_) => value.to_string().to_variant(),
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

    /// `model` を当てる: あれば読み込み、null ならモデルを外す。
    fn apply_model(&mut self) {
        self.model_pending = false;
        if self.model.is_some() {
            self.load_resource();
        } else {
            self.unload();
        }
    }

    /// `model` のリソースから読み込む。
    fn load_resource(&mut self) -> bool {
        let Some(model) = &self.model else {
            self.unload();
            return false;
        };
        let path = model.get_path();
        let Ok(bytes) = model.get("data").try_to::<PackedByteArray>() else {
            self.fail_invalid_file(&format!("`{path}` is not a UgoeModel (it has no `data`)"));
            return false;
        };
        self.load_bytes(&bytes, &path.to_string())
    }

    /// `.ugoe` のバイト列から読み込む。`what` はエラーの表示に使う。
    fn load_bytes(&mut self, bytes: &PackedByteArray, what: &str) -> bool {
        match LoadedModel::from_bundle_bytes(bytes.as_slice(), self.load_limits()) {
            Ok(loaded) => {
                self.install(loaded);
                true
            }
            Err(e) => {
                self.fail_load(&e, &format!("`{what}`"));
                false
            }
        }
    }

    /// 読み込みの上限 (`max_*` のプロパティ)。
    fn load_limits(&self) -> LoadLimits {
        // 設定の関数が範囲を確かめているので、変換は失敗しない (しても既定値を使う)。
        let defaults = LoadLimits::default();
        LoadLimits {
            read: ReadLimits::default()
                .with_max_chunk_decompressed(
                    u64::try_from(self.max_chunk_decompressed)
                        .unwrap_or(defaults.read.max_chunk_decompressed),
                )
                .with_max_total_decompressed(
                    u64::try_from(self.max_total_decompressed)
                        .unwrap_or(defaults.read.max_total_decompressed),
                ),
            max_image_side: u32::try_from(self.max_image_side).unwrap_or(defaults.max_image_side),
        }
    }

    /// モデルの読み込みの失敗: エラーを出し、`get_load_error()` に残し、モデルを外す。`what` はエラーの表示に使う。
    pub(super) fn fail_load(&mut self, error: &LoadError, what: &str) {
        godot_error!("ugoe: {what}: {error}");
        self.load_error = load_error_dictionary(error);
        self.unload();
    }

    /// `UgoeModel` でないリソースなど、ファイルを読む前の失敗 (`invalid_file`)。
    fn fail_invalid_file(&mut self, message: &str) {
        godot_error!("ugoe: {message}");
        self.load_error = error_dictionary(DressUpError::INVALID_FILE, message, []);
        self.unload();
    }

    fn install(&mut self, mut loaded: LoadedModel) {
        self.unload();
        self.load_error = VarDictionary::new();
        for warning in &loaded.warnings {
            godot_warn!("ugoe: {warning}");
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
        loaded.puppet.set_tracking(self.tracking);
        loaded.puppet.set_toggles(self.toggles);
        loaded
            .puppet
            .set_motion_over_tracking(self.motion_over_tracking);
        self.loaded = Some(loaded);
        self.push_tracking_kept();
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
        self.model_pending = false;
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

    /// モジュールのリソースを読む。付けているものなら読み直さない。読めなければエラーを出し、
    /// `get_load_error()` に残して `None`。
    ///
    /// 読むたびに読み込みの警告を Godot の警告として出すので、1 回の操作で同じリソースを 2 度読まない
    /// (読んだものを [`Self::apply_parsed_modules`] に渡す)。
    fn parse_module(&mut self, resource: &Gd<Resource>) -> Option<Arc<ModuleData>> {
        match self.read_module(resource) {
            Ok(data) => Some(data),
            Err(error) => {
                self.load_error = error;
                None
            }
        }
    }

    /// [`Self::parse_module`] と同じだが、失敗を `get_load_error()` に残さない (エラーの辞書を返す)。
    fn read_module(&self, resource: &Gd<Resource>) -> Result<Arc<ModuleData>, VarDictionary> {
        if let Some((_, data)) = self.attached.iter().find(|(r, _)| r == resource) {
            return Ok(Arc::clone(data));
        }
        let path = resource.get_path();
        let Ok(bytes) = resource.get("data").try_to::<PackedByteArray>() else {
            let message = format!("`{path}` is not a UgoeModule (it has no `data`)");
            godot_error!("ugoe: {message}");
            return Err(error_dictionary(DressUpError::INVALID_FILE, &message, []));
        };
        match ModuleData::from_bytes(bytes.as_slice(), self.load_limits()) {
            Ok(data) => {
                for warning in &data.warnings {
                    godot_warn!("ugoe: module `{path}`: {warning}");
                }
                Ok(Arc::new(data))
            }
            Err(e) => {
                godot_error!("ugoe: module `{path}`: {e}");
                Err(load_error_dictionary(&e))
            }
        }
    }

    /// 取り付けるモジュールを `list` にする ([`Self::apply_parsed_modules`])。読めないリソースがあれば何も変えずに false。
    fn apply_modules(&mut self, list: Vec<Gd<Resource>>) -> bool {
        let mut parsed = Vec::with_capacity(list.len());
        for resource in list {
            let Some(data) = self.parse_module(&resource) else {
                return false;
            };
            parsed.push((resource, data));
        }
        self.apply_parsed_modules(parsed)
    }

    /// 読んだモジュールの組を取り付ける。読み込んだモデルがあれば付け直し、テクスチャを使い回して
    /// レンダラを作り直す。成功したら true (`get_load_error()` を空にする)。失敗したら何も変えない。
    fn apply_parsed_modules(&mut self, parsed: Vec<(Gd<Resource>, Arc<ModuleData>)>) -> bool {
        if let Some(loaded) = &mut self.loaded {
            let change =
                match loaded.set_modules(parsed.iter().map(|(_, d)| Arc::clone(d)).collect()) {
                    Ok(change) => change,
                    Err(e) => {
                        godot_error!("ugoe: dress-up failed: {e}");
                        let fields = e.fields().into_iter().map(|(k, v)| (k, field_variant(v)));
                        self.load_error = error_dictionary(e.code(), &e.to_string(), fields);
                        return false;
                    }
                };
            self.after_dress_up(&change);
        }
        self.modules = parsed.iter().map(|(r, _)| r.to_variant()).collect();
        self.attached = parsed;
        self.load_error = VarDictionary::new();
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

    /// `lights` を読み直してシェーダに渡す。
    fn read_lights(&mut self) {
        self.manual_lights = self
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
        self.update_lighting();
    }

    /// `ambient_light`・`lights`・シーンの光源をシェーダに渡す (変わっていなければ何もしない)。
    fn update_lighting(&mut self) {
        let mut lights = self.manual_lights.clone();
        lights.extend(self.scene_light_values());
        let c = self.ambient_light;
        let lighting = LightUniforms::new(&Lighting {
            ambient: Rgb::new(c.r, c.g, c.b),
            lights,
        });
        if lighting == self.lighting {
            return;
        }
        self.lighting = lighting;
        if let (Some(renderer), Some(loaded)) = (&mut self.renderer, &self.loaded) {
            renderer.set_lighting(loaded, &self.lighting);
        }
    }

    /// 木の `node_added` と `node_removed` を見る (`on`) か、見るのをやめる。
    ///
    /// 受け取る関数はノードを借りない (自分の中で木を変えたとき、たとえばマスクのビューポートを足したときにも呼ばれるため)。
    fn watch_tree(&mut self, on: bool) {
        let Some(mut tree) = self.base().get_tree_or_null() else {
            return;
        };
        if let Some(callable) = self.tree_watch.take() {
            for signal in ["node_added", "node_removed"] {
                if tree.is_connected(signal, &callable) {
                    tree.disconnect(signal, &callable);
                }
            }
        }
        if !on {
            return;
        }
        let changed = Rc::clone(&self.scene_changed);
        let callable = Callable::from_linked_fn("ugoe_watch_lights", &self.to_gd(), move |args| {
            let is_light = args
                .first()
                .and_then(|v| v.try_to::<Gd<Node>>().ok())
                .is_some_and(|n| n.is_class("Light2D"));
            if is_light {
                changed.set(true);
            }
        });
        for signal in ["node_added", "node_removed"] {
            tree.connect(signal, &callable);
        }
        self.tree_watch = Some(callable);
        self.scene_changed.set(true);
    }

    /// 木が変わっていれば、シーンの `Light2D` を探し直す (このノードのビューポートの中)。
    fn find_scene_lights(&mut self) {
        if !self.scene_changed.replace(false) {
            self.scene_lights.retain(|l| l.is_instance_valid());
            return;
        }
        self.scene_lights.clear();
        let Some(viewport) = self.base().get_viewport() else {
            return;
        };
        self.scene_lights = viewport
            .find_children_ex("*")
            .type_("Light2D")
            .owned(false)
            .done()
            .iter_shared()
            .filter_map(|n| n.try_cast::<Light2D>().ok())
            .collect();
    }

    /// シーンの光源を読み直し、変わっていればシェーダに渡す。
    fn update_scene_lights(&mut self) {
        self.find_scene_lights();
        self.update_lighting();
    }

    /// シーンの `Light2D` をキャンバス座標の光源に直す (`docs/spec/rendering.md#ライティング`)。
    ///
    /// - 色は `color × energy`。合成のモード (`blend_mode`)・影・Z と層の範囲は見ない (いつも足す)。
    /// - `DirectionalLight2D`: 光はノードの +Y の向きに進む。`height` (0〜1) で画面の奥へ傾ける
    ///   (0 で画面と平行、1 で正面から)。`max_distance` は見ない。
    /// - `PointLight2D`: 位置はノードの位置、奥行きは `height` (ピクセル、手前が正)。`range` はテクスチャの
    ///   半径 (幅と高さの大きい方の半分 × `texture_scale`。テクスチャが無ければ減衰しない)。
    ///   テクスチャの形 (明るさの分布) は使わず、参照レンダラと同じ式で減衰する。
    fn scene_light_values(&self) -> Vec<Light> {
        if !self.use_scene_lights || !self.base().is_inside_tree() {
            return Vec::new();
        }
        let Some(loaded) = &self.loaded else {
            return Vec::new();
        };
        let origin = loaded.model().meta.canvas.origin;
        let base = self.base();
        let canvas = base.get_canvas();
        let mask = base.get_light_mask();
        let editor = Engine::singleton().is_editor_hint();
        let to_local = base.get_global_transform().affine_inverse();
        // 全体の座標の 1 単位あたりの、このノードの座標の単位数 (高さと半径に掛ける)。
        let unit = to_local.determinant().abs().sqrt();
        self.scene_lights
            .iter()
            .filter(|l| {
                l.is_instance_valid()
                    && l.is_enabled()
                    && l.is_visible_in_tree()
                    && (editor || !l.is_editor_only())
                    && l.get_item_cull_mask() & mask != 0
                    && l.get_canvas() == canvas
            })
            .filter_map(|l| {
                let c = l.get_color() * l.get_energy();
                let color = Rgb::new(c.r, c.g, c.b);
                let xform = l.get_global_transform();
                if l.is_class("DirectionalLight2D") {
                    let down = to_local.basis_xform(xform.basis_xform(Vector2::DOWN));
                    let direction = directional_from_light2d([down.x, down.y], l.get_height())?;
                    return Some(Light::Directional { direction, color });
                }
                let point = l.clone().try_cast::<PointLight2D>().ok()?;
                let p = to_local * xform.origin;
                let range = point.get_texture().map_or(f32::INFINITY, |t| {
                    let size = t.get_size();
                    let s = xform.scale();
                    size.x.max(size.y) / 2.0
                        * point.get_texture_scale()
                        * s.x.abs().max(s.y.abs())
                        * unit
                });
                Some(Light::Point {
                    position: [p.x + origin.x, p.y + origin.y, l.get_height() * unit],
                    color,
                    range,
                })
            })
            .collect()
    }

    /// `xr_face_tracker` の名前の `XRFaceTracker` を読む。見つからなければ何もしない (前の値を保つ)。
    fn read_xr_face_tracker(&mut self) {
        let Some(tracker) = XrServer::singleton().get_tracker(&self.xr_face_tracker) else {
            return;
        };
        let tracker = tracker.upcast::<RefCounted>();
        if tracker.is_class("XRFaceTracker") {
            self.set_tracking_from_xr_face_tracker(tracker);
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

    fn push_tracking_kept(&mut self) {
        let ids: Vec<String> = self
            .tracking_kept
            .as_slice()
            .iter()
            .map(GString::to_string)
            .collect();
        if let Some(loaded) = &mut self.loaded {
            loaded.puppet.set_tracking_kept(ids);
            self.dirty = true;
        }
    }

    fn step(&mut self, dt: f32) {
        let Some(loaded) = &mut self.loaded else {
            return;
        };
        loaded.puppet.update(dt);
        let events: Vec<GString> = loaded
            .puppet
            .motion_events()
            .iter()
            .map(|e| GString::from(e.value.as_str()))
            .collect();
        self.draw();
        for value in events {
            self.signals().motion_event().emit(&value);
        }
    }

    /// 直前の評価の結果を描く (`update` か `follow` の後)。
    fn draw(&mut self) {
        // マスクは画面上の大きさ (ビューポートの画素) に合わせた解像度で描く。
        let screen_scale = if self.base().is_inside_tree() {
            let s = self.base().get_global_transform_with_canvas().scale();
            s.x.abs().max(s.y.abs())
        } else {
            1.0
        };
        let mask_scale = self.mask_scale * screen_scale;
        self.dirty = false;
        if let (Some(loaded), Some(renderer)) = (&self.loaded, &mut self.renderer) {
            renderer.draw(loaded, mask_scale);
        }
    }
}
