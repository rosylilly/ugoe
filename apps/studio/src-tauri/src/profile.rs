//! プロファイル: シーンの一覧と出力の設定を持つ、版の番号付きの JSON の 1 ファイル (`docs/studio.md#プロファイルの形式`、ADR 0028 の 4.)。
//!
//! アプリの設定であって、モデルのファイル形式 (`docs/spec/file-formats.md`) ではない。読み込みでは `format` と `version` を
//! 確かめ、未来の版は [`ProfileError::FutureVersion`] にする (黙って読み落とさない)。値の範囲を外れたものは収め直す
//! ([`Profile::sanitize`])。ホットキー・入力・モーションの設定 (T0803) と、出力の刻みの速さ・仮想カメラの設定 (T0827) は
//! 同じ版のまま、省略できる項目として足した。
//!
//! ファイルのパス (シーンのモデル・モジュール・背景の画像と、ホットキーの着せ替えのモジュール) は、プロファイルのフォルダを
//! 基準にした相対パスでも持てる (`docs/studio.md#プロファイルのパス`、ADR 0033)。メモリの上では絶対パスに解いて持ち
//! ([`Profile::read`])、書くときにプロファイルのフォルダの下のものだけを相対パスに戻す ([`Profile::write`])。
//!
//! 見本の印 (`template`。T0866) のあるプロファイルは、開いても自動の保存で書き換えない (`docs/studio.md#書き換えないプロファイル`)。
//! 印は書かない (写しには付けない)。

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::hotkeys::{self, Hotkey, HotkeyAction};
use crate::input::{InputSettings, MotionSettings};
use crate::pacing::TickRate;
use crate::virtual_camera;
use ugoe_fs::{ReadError, read_regular_file};

/// `format` の値 (ほかの JSON、たとえば JSON のモデルを開いたときに分かるように)。
pub const PROFILE_FORMAT: &str = "ugoe-studio-profile";
/// 書き出す版。読めるのはこれ以下の版。
pub const PROFILE_VERSION: u32 = 1;
/// プロファイルのファイルの大きさの上限 (誤って巨大なファイルを選んだとき)。
const MAX_PROFILE_BYTES: u64 = 16 * 1024 * 1024;
/// 位置 (出力の大きさに対する割合) の範囲。
pub const POSITION_LIMIT: f64 = 2.0;
/// 大きさ (出力に合わせた大きさに対する倍率) の範囲。
pub const SCALE_RANGE: (f64, f64) = (0.05, 20.0);
/// 回転 (度) の範囲。
pub const ROTATION_LIMIT: f64 = 360.0;
/// 名前の長さの上限 (文字)。
const MAX_NAME_CHARS: usize = 100;
/// シーンが覚えるポーズの要素の数の上限 (プロファイルを手で書き換えたとき。T0842)。
pub const MAX_POSE_ENTRIES: usize = 256;

/// プロファイル。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// 常に [`PROFILE_FORMAT`]。
    pub format: String,
    /// 版 ([`PROFILE_VERSION`])。
    pub version: u32,
    /// 見本の印 (T0866)。印のあるプロファイルは自動の保存で書き換えず、保存は保存先を選ばせる。[`Self::write`] は書かない。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub template: bool,
    /// シーンの一覧 (並びは操作ウィンドウの並び)。
    #[serde(default)]
    pub scenes: Vec<SceneSettings>,
    /// 最後に使ったシーンの ID (開いたときにこのシーンにする)。
    #[serde(default)]
    pub active_scene: Option<String>,
    #[serde(default)]
    pub output: OutputSettings,
    /// ホットキーの割り当て (T0803)。
    #[serde(default)]
    pub hotkeys: Vec<Hotkey>,
    /// 入力の機器と調整 (T0803)。
    #[serde(default)]
    pub input: InputSettings,
    /// 待機モーションとトラッキング中のモーションの優先 (T0803)。
    #[serde(default)]
    pub motion: MotionSettings,
}

/// シーン: 1 体のアバターと、その見せ方。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneSettings {
    /// プロファイルの中で一意な ID (ホットキーや外部 API がシーンを指す)。
    pub id: String,
    pub name: String,
    /// モデル (`.ugoe` か JSON) のパス。まだ選んでいなければ `None`。ファイルの中ではプロファイルのフォルダからの相対パスでもよい
    /// (読むと絶対パスに解く。[`Profile::read`])。モジュール・背景の画像も同じ。
    #[serde(default)]
    pub model: Option<PathBuf>,
    /// 付けるモジュール (`.ugoemod`) のパス (合成の順)。
    #[serde(default)]
    pub modules: Vec<PathBuf>,
    /// 光源の設定 (`@ugoe/player` の `LightingSettings`。読むのは画面と受け手で、ここでは中身を見ない)。`None` は既定の光源。
    #[serde(default)]
    pub lighting: Option<serde_json::Value>,
    #[serde(default)]
    pub transform: Transform,
    #[serde(default)]
    pub background: Background,
    /// ポーズのグループごとに表示する要素のパーツの ID (着せ替えで付けたモジュールのパーツは `<モジュール>/<パーツ>`。T0842)。
    /// 出すときに、モデルに無い要素は捨てる (`Conductor::present`)。挙がっていないグループは先頭の要素を表示する。
    #[serde(default)]
    pub pose: Vec<String>,
}

/// モデルの位置と大きさと回転。出力の大きさに対する割合で持ち、出力の大きさを変えても構図を保つ。
/// 値は `f64` で持つ (画面の値と JSON の値がそのまま一致し、`f32` の丸めで値が揺れないように)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Transform {
    /// 出力に合わせて中央に置いた位置からのずれ (出力の幅に対する割合。右が正)。
    pub x: f64,
    /// 同じく出力の高さに対する割合 (下が正)。
    pub y: f64,
    /// 出力に合わせた大きさに対する倍率。
    pub scale: f64,
    /// 回転 (度。時計回りが正)。中心はモデルを置いた位置。
    pub rotation: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            rotation: 0.0,
        }
    }
}

/// 背景。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Background {
    /// 透過 (配信ソフトで重ねる)。
    #[default]
    Transparent,
    /// 単色 (`#rrggbb`)。
    Color { color: String },
    /// クロマキーの単色 (`#rrggbb`。配信ソフトで色を抜く)。見え方は単色と同じで、画面で色の選び方を変える。
    Chroma { color: String },
    /// 画像 (PNG・JPEG・WebP のパス)。出力を覆うように拡大する。
    Image { path: PathBuf },
}

/// 出力の設定。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OutputSettings {
    /// 出力ウィンドウの大きさのプリセット (`1920x1080` など。`free` は大きさを変えない)。
    pub size: String,
    /// 刻みの速さ (回/秒。30・60・120。T0827)。選べない値は読むときに既定 (60) にする。
    pub tick_rate: u32,
    /// 仮想カメラの設定 (T0827)。
    pub virtual_camera: VirtualCameraSettings,
    /// 出力ウィンドウとブラウザソースの描く解像度の倍率 ([`RENDER_SCALES`]。T0858)。選べない値は読むときに 1 にする。
    pub render_scale: f32,
}

/// 選べる描く解像度の倍率 (先頭が既定)。描画バッファを「表示の大きさ × 倍率」の画素にして描く (docs/studio.md#出力ウィンドウ)。
pub const RENDER_SCALES: [f32; 3] = [1.0, 0.75, 0.5];

/// 選べる描く解像度の倍率か。
pub fn is_render_scale(scale: f32) -> bool {
    RENDER_SCALES.contains(&scale)
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            size: "1920x1080".to_owned(),
            tick_rate: TickRate::DEFAULT.hz(),
            virtual_camera: VirtualCameraSettings::default(),
            render_scale: RENDER_SCALES[0],
        }
    }
}

impl OutputSettings {
    /// 選べない刻みの速さ・描く解像度の倍率・仮想カメラの設定を既定に戻す。
    pub fn sanitize(&mut self) {
        if TickRate::from_hz(self.tick_rate).is_none() {
            self.tick_rate = TickRate::DEFAULT.hz();
        }
        if !is_render_scale(self.render_scale) {
            self.render_scale = RENDER_SCALES[0];
        }
        self.virtual_camera.sanitize();
    }

    /// 刻みの速さ。
    pub fn tick_rate(&self) -> TickRate {
        TickRate::from_hz(self.tick_rate).unwrap_or(TickRate::DEFAULT)
    }
}

/// プロファイルに持つ仮想カメラの設定 (`virtual_camera.rs` の `Settings` と同じ値。色はほかの項目と同じく `#rrggbb` で持つ)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VirtualCameraSettings {
    /// 解像度 (1920×1080・1280×720)。
    pub width: u32,
    pub height: u32,
    /// フレームレート (30・60)。
    pub fps: u32,
    /// 透過のところを埋める色 (`#rrggbb`)。
    pub background: String,
}

impl Default for VirtualCameraSettings {
    fn default() -> Self {
        Self::from_settings(virtual_camera::Settings::default())
    }
}

impl VirtualCameraSettings {
    pub fn from_settings(settings: virtual_camera::Settings) -> Self {
        let [r, g, b] = settings.background;
        Self {
            width: settings.width,
            height: settings.height,
            fps: settings.fps,
            background: format!("#{r:02x}{g:02x}{b:02x}"),
        }
    }

    /// 仮想カメラに渡す設定 ([`Self::sanitize`] の後に呼ぶ。色を読めなければ既定の色)。
    pub fn settings(&self) -> virtual_camera::Settings {
        let background =
            parse_hex_color(&self.background).unwrap_or(virtual_camera::DEFAULT_BACKGROUND);
        virtual_camera::Settings {
            width: self.width,
            height: self.height,
            fps: self.fps,
            background,
        }
    }

    /// 選べない解像度・フレームレート・形の違う色を、それぞれ既定に戻す。
    pub fn sanitize(&mut self) {
        let default = virtual_camera::Settings::default();
        if !ugoe_studio_vcam::RESOLUTIONS.contains(&(self.width, self.height)) {
            self.width = default.width;
            self.height = default.height;
        }
        if !ugoe_studio_vcam::FRAME_RATES.contains(&self.fps) {
            self.fps = default.fps;
        }
        if is_hex_color(&self.background) {
            self.background.make_ascii_lowercase();
        } else {
            self.background = Self::from_settings(default).background;
        }
    }
}

/// プロファイルを読めない・書けない。画面は `code` で文言を選ぶ (ADR 0029 の 4.)。
#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum ProfileError {
    #[error("{path}: {message}")]
    #[serde(rename = "io")]
    Io { path: PathBuf, message: String },
    #[error("{path}: the file is too large ({bytes} bytes)")]
    #[serde(rename = "tooLarge")]
    TooLarge { path: PathBuf, bytes: u64 },
    #[error("{path}: not JSON: {message}")]
    #[serde(rename = "notJson")]
    NotJson { path: PathBuf, message: String },
    #[error("{path}: not an ugoe studio profile")]
    #[serde(rename = "notProfile")]
    NotProfile { path: PathBuf },
    #[error("{path}: profile version {version} is newer than this studio (up to {supported})")]
    #[serde(rename = "futureVersion")]
    FutureVersion {
        path: PathBuf,
        version: u64,
        supported: u32,
    },
    #[error("{path}: broken profile: {message}")]
    #[serde(rename = "invalid")]
    Invalid { path: PathBuf, message: String },
}

impl Default for Profile {
    fn default() -> Self {
        Self::new()
    }
}

impl Profile {
    /// 空のシーンを 1 つ持つ新しいプロファイル。
    pub fn new() -> Self {
        Self {
            format: PROFILE_FORMAT.to_owned(),
            version: PROFILE_VERSION,
            template: false,
            scenes: vec![SceneSettings::new("scene-1".to_owned(), String::new())],
            active_scene: Some("scene-1".to_owned()),
            output: OutputSettings::default(),
            hotkeys: Vec::new(),
            input: InputSettings::default(),
            motion: MotionSettings::default(),
        }
    }

    /// JSON の文字列から読む (`path` はエラーの表示用)。版を確かめてから形を読み、値を収め直す。
    pub fn from_json(text: &str, path: &Path) -> Result<Self, ProfileError> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|e| ProfileError::NotJson {
                path: path.to_owned(),
                message: e.to_string(),
            })?;
        if value.get("format").and_then(|f| f.as_str()) != Some(PROFILE_FORMAT) {
            return Err(ProfileError::NotProfile {
                path: path.to_owned(),
            });
        }
        let invalid = |message: String| ProfileError::Invalid {
            path: path.to_owned(),
            message,
        };
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| invalid("`version` is missing".to_owned()))?;
        if version == 0 {
            return Err(invalid("`version` must be 1 or later".to_owned()));
        }
        if version > u64::from(PROFILE_VERSION) {
            return Err(ProfileError::FutureVersion {
                path: path.to_owned(),
                version,
                supported: PROFILE_VERSION,
            });
        }
        let mut profile: Self =
            serde_json::from_value(value).map_err(|e| invalid(e.to_string()))?;
        profile.sanitize();
        Ok(profile)
    }

    /// ファイルから読む。相対パスはプロファイルのフォルダを基準に絶対パスに解く ([`resolve_path`])。
    pub fn read(path: &Path) -> Result<Self, ProfileError> {
        let io = |e: std::io::Error| ProfileError::Io {
            path: path.to_owned(),
            message: e.to_string(),
        };
        // 通常のファイルだけを読む (装置・FIFO・ディレクトリは読まない。`ugoe_fs::read_regular_file`)。
        let bytes = read_regular_file(path, MAX_PROFILE_BYTES).map_err(|e| match e {
            ReadError::TooLarge { len, .. } => ProfileError::TooLarge {
                path: path.to_owned(),
                bytes: len,
            },
            e => io(e.into()),
        })?;
        let text = String::from_utf8(bytes).map_err(|e| {
            io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.utf8_error(),
            ))
        })?;
        let mut profile = Self::from_json(&text, path)?;
        let base = base_dir(path);
        profile.map_paths(|p| resolve_path(&base, p));
        Ok(profile)
    }

    /// 整形した JSON (今の版で書く)。
    pub fn to_json(&self) -> String {
        let mut profile = self.clone();
        profile.format = PROFILE_FORMAT.to_owned();
        profile.version = PROFILE_VERSION;
        // 文字列と数と列挙だけなので失敗しない。
        serde_json::to_string_pretty(&profile).unwrap_or_default() + "\n"
    }

    /// ファイルに書く (一時ファイルに書いてから置き換え、途中で落ちても前のファイルを壊さない。`ugoe_fs::write_atomic`)。
    /// `path` のフォルダ (とその下) にあるファイルは相対パス、外のものは絶対パスで書く ([`relative_path`])。
    /// 別の場所へ保存すれば、その場所を基準に書き直す。見本の印 (`template`) は書かない (書いたものは見本の写し)。
    pub fn write(&self, path: &Path) -> Result<(), ProfileError> {
        let base = base_dir(path);
        let mut profile = self.clone();
        profile.template = false;
        profile.map_paths(|p| relative_path(&base, p).map_or_else(|| p.to_owned(), PathBuf::from));
        ugoe_fs::write_atomic(path, profile.to_json().as_bytes()).map_err(|e| ProfileError::Io {
            path: path.to_owned(),
            message: e.to_string(),
        })
    }

    /// 値の範囲を外れたものを収め直す。ID の重複と空の ID は付け直し、`active_scene` が無いシーンを指していれば先頭にする。
    pub fn sanitize(&mut self) {
        let mut seen = std::collections::HashSet::new();
        for i in 0..self.scenes.len() {
            let id = self.scenes[i].id.trim().to_owned();
            if id.is_empty() || !seen.insert(id.clone()) {
                let fresh = self.fresh_id();
                seen.insert(fresh.clone());
                self.scenes[i].id = fresh;
            } else {
                self.scenes[i].id = id;
            }
            self.scenes[i].sanitize();
        }
        if !self
            .active_scene
            .as_ref()
            .is_some_and(|id| self.scene(id).is_some())
        {
            self.active_scene = self.scenes.first().map(|s| s.id.clone());
        }
        self.output.sanitize();
        hotkeys::sanitize(&mut self.hotkeys);
        self.input.sanitize();
        self.motion.sanitize();
    }

    /// ファイルのパス (シーンのモデル・モジュール・背景の画像と、ホットキーの着せ替えのモジュール) をすべて `f` で置き換える。
    fn map_paths(&mut self, mut f: impl FnMut(&Path) -> PathBuf) {
        for scene in &mut self.scenes {
            if let Some(model) = &mut scene.model {
                *model = f(model);
            }
            for module in &mut scene.modules {
                *module = f(module);
            }
            if let Background::Image { path } = &mut scene.background {
                *path = f(path);
            }
        }
        for hotkey in &mut self.hotkeys {
            if let HotkeyAction::DressUp {
                module: Some(module),
                ..
            } = &mut hotkey.action
            {
                *module = f(module);
            }
        }
    }

    pub fn scene(&self, id: &str) -> Option<&SceneSettings> {
        self.scenes.iter().find(|s| s.id == id)
    }

    pub fn scene_mut(&mut self, id: &str) -> Option<&mut SceneSettings> {
        self.scenes.iter_mut().find(|s| s.id == id)
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.scenes.iter().position(|s| s.id == id)
    }

    /// まだ使っていない ID (`scene-<n>`)。
    pub fn fresh_id(&self) -> String {
        (1..)
            .map(|n| format!("scene-{n}"))
            .find(|id| self.scene(id).is_none())
            .unwrap_or_default()
    }
}

impl SceneSettings {
    /// モデルの無いシーン。
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            model: None,
            modules: Vec::new(),
            lighting: None,
            transform: Transform::default(),
            background: Background::default(),
            pose: Vec::new(),
        }
    }

    /// 値の範囲を外れたものを収め直す (名前の長さ・位置・大きさ・回転・色・ポーズの要素の空と重複と数)。
    pub fn sanitize(&mut self) {
        if self.name.chars().count() > MAX_NAME_CHARS {
            self.name = self.name.chars().take(MAX_NAME_CHARS).collect();
        }
        self.transform = self.transform.sanitized();
        match &mut self.background {
            Background::Color { color } | Background::Chroma { color } => {
                if !is_hex_color(color) {
                    *color = "#000000".to_owned();
                }
                color.make_ascii_lowercase();
            }
            Background::Transparent | Background::Image { .. } => {}
        }
        if self.lighting.as_ref().is_some_and(|l| !l.is_object()) {
            self.lighting = None;
        }
        let mut seen = std::collections::HashSet::new();
        self.pose
            .retain(|part| !part.is_empty() && seen.insert(part.clone()));
        self.pose.truncate(MAX_POSE_ENTRIES);
    }
}

impl Transform {
    /// 範囲に収めた写し (数でない値は既定に戻す)。
    pub fn sanitized(self) -> Self {
        let fit = |v: f64, lo: f64, hi: f64, default: f64| {
            if v.is_finite() {
                v.clamp(lo, hi)
            } else {
                default
            }
        };
        Self {
            x: fit(self.x, -POSITION_LIMIT, POSITION_LIMIT, 0.0),
            y: fit(self.y, -POSITION_LIMIT, POSITION_LIMIT, 0.0),
            scale: fit(self.scale, SCALE_RANGE.0, SCALE_RANGE.1, 1.0),
            rotation: fit(self.rotation, -ROTATION_LIMIT, ROTATION_LIMIT, 0.0),
        }
    }
}

/// `#rrggbb` か。
pub fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// `#rrggbb` の RGB。形が違えば `None`。
pub fn parse_hex_color(s: &str) -> Option<[u8; 3]> {
    if !is_hex_color(s) {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
    Some([channel(1)?, channel(3)?, channel(5)?])
}

/// プロファイルのファイル `path` に書けるか (開いたときに確かめる。書く口で開くだけで、中身も更新の時刻も変えない。T0866)。
///
/// 読み取り専用のファイル (Unix の権限・Windows の読み取り専用の属性) は書けない。フォルダに書けないことはここでは分からない
/// (保存は一時ファイルを同じフォルダに書いて置き換えるので、保存に失敗して分かる。`Stage::save_profile`)。
pub fn is_writable(path: &Path) -> bool {
    std::fs::OpenOptions::new().write(true).open(path).is_ok()
}

/// プロファイルのファイル `path` のあるフォルダ (相対パスの基準)。`path` が相対パスなら今のディレクトリから絶対パスにする。
fn base_dir(path: &Path) -> PathBuf {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    path.parent().map(Path::to_owned).unwrap_or_default()
}

/// プロファイルにあるパス `path` を、プロファイルのフォルダ `base` を基準に解く (`docs/studio.md#プロファイルのパス`)。
///
/// 絶対パスはそのまま。相対パスは `/` か `\` で区切り (どの OS でも同じに読む)、`base` の下のファイルを指すものだけを解く。
/// フォルダの外に出られるもの (`..`・点と空白だけの名前・区切りから始まるもの (ルートや UNC)・`:` を含むもの (ドライブ名)) と
/// 空のものは、相対パスのまま返す。相対パスのままのパスは読めないパスで、[`crate::stage::read_asset`] は読まずに
/// 見つからないとする (画面は選び直させる)。プロファイルに書き戻すときもそのまま書く。
///
/// 解くのは字面だけで、記号のリンクはたどらない (フォルダの下にある記号のリンクは、ほかのファイルと同じく OS がたどって読む。
/// 絶対パスはもともと受け付けるので、記号のリンクで読めるファイルは増えない)。
pub fn resolve_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_owned();
    }
    match path.to_str().and_then(relative_components) {
        Some(parts) => {
            let mut resolved = base.to_owned();
            for part in parts {
                resolved.push(part);
            }
            resolved
        }
        None => path.to_owned(),
    }
}

/// 相対パスの名前の並び。フォルダの外に出られるもの・空のものは `None` ([`resolve_path`])。`.` と空の名前 (`a//b`) は飛ばす。
fn relative_components(text: &str) -> Option<Vec<&str>> {
    if text.starts_with(['/', '\\']) || text.contains(':') {
        return None;
    }
    let mut parts = Vec::new();
    for part in text.split(['/', '\\']) {
        if part.is_empty() || part == "." {
            continue;
        }
        // `..` と、Windows が末尾の点と空白を落として `..` や `.` に読むかもしれない名前 (`...`・`.. `)。
        if part.trim_end_matches(['.', ' ']).is_empty() {
            return None;
        }
        parts.push(part);
    }
    (!parts.is_empty()).then_some(parts)
}

/// `path` がフォルダ `base` の下にあれば、`base` からの相対パス (区切りは `/`)。外にある・相対パスで書くと
/// 同じファイルに解けない (名前に `\` や `:` を含む) なら `None` (絶対パスのまま書く)。比べるのは字面だけ。
pub fn relative_path(base: &Path, path: &Path) -> Option<String> {
    if !path.is_absolute() {
        return None;
    }
    let rest = path.strip_prefix(base).ok()?;
    let parts = rest
        .components()
        .map(|c| match c {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    let text = parts.join("/");
    (resolve_path(base, Path::new(&text)) == path).then_some(text)
}

/// 最近のプロファイル (新しいものが先頭)。アプリのデータの場所の `recent-profiles.json` に置く。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProfiles {
    #[serde(default)]
    pub profiles: Vec<PathBuf>,
}

impl RecentProfiles {
    /// 覚える数。
    pub const MAX: usize = 10;

    /// 読む。無い・壊れているときは空 (最近の一覧は失っても困らない)。
    pub fn read(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Self>(&text).ok())
            .map(|mut recent| {
                recent.profiles.truncate(Self::MAX);
                recent
            })
            .unwrap_or_default()
    }

    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        ugoe_fs::write_atomic(
            path,
            serde_json::to_string_pretty(self)
                .unwrap_or_default()
                .as_bytes(),
        )
    }

    /// `path` を先頭にする (重複は除き、多ければ古いものを捨てる)。
    pub fn touch(&mut self, path: &Path) {
        self.profiles.retain(|p| p != path);
        self.profiles.insert(0, path.to_owned());
        self.profiles.truncate(Self::MAX);
    }

    pub fn forget(&mut self, path: &Path) {
        self.profiles.retain(|p| p != path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Result<Profile, ProfileError> {
        Profile::from_json(text, Path::new("p.json"))
    }

    #[test]
    fn round_trips() {
        let mut profile = Profile::new();
        let scene = &mut profile.scenes[0];
        scene.name = "雑談".into();
        scene.model = Some("/m/avatar.ugoe".into());
        scene.modules = vec!["/m/cat-ears.ugoemod".into()];
        scene.lighting = Some(serde_json::json!({ "enabled": true }));
        scene.transform = Transform {
            x: 0.1,
            y: -0.2,
            scale: 1.5,
            rotation: 10.0,
        };
        scene.background = Background::Image {
            path: "/m/bg.png".into(),
        };
        scene.pose = vec!["ArmB".into(), "CatEars/EarsUp".into()];
        profile.output.size = "1280x720".into();
        profile.output.tick_rate = 120;
        profile.output.render_scale = 0.75;
        profile.output.virtual_camera = VirtualCameraSettings {
            width: 1920,
            height: 1080,
            fps: 60,
            background: "#00ff80".into(),
        };
        assert_eq!(read(&profile.to_json()).unwrap(), profile);
    }

    /// 刻みの速さと仮想カメラの設定 (T0827) が無い、前のプロファイルは既定の値で読む。
    #[test]
    fn reads_profiles_without_the_output_settings_as_defaults() {
        let profile = read(
            r#"{"format": "ugoe-studio-profile", "version": 1, "output": {"size": "1280x720"}}"#,
        )
        .unwrap();
        assert_eq!(profile.output.size, "1280x720");
        assert_eq!(profile.output.tick_rate, 60);
        assert_eq!(profile.output.render_scale, 1.0);
        assert_eq!(
            profile.output.virtual_camera.settings(),
            virtual_camera::Settings::default()
        );
        assert_eq!(profile.output.virtual_camera.background, "#f0f0f0");
        let profile = read(r#"{"format": "ugoe-studio-profile", "version": 1}"#).unwrap();
        assert_eq!(profile.output, OutputSettings::default());
    }

    /// 後の版で `output` に足した項目は読み飛ばす (前の版のスタジオも同じく読み飛ばす)。
    #[test]
    fn ignores_unknown_output_settings() {
        let profile = read(
            r#"{"format": "ugoe-studio-profile", "version": 1,
                "output": {"tickRate": 30, "future": true, "virtualCamera": {"fps": 60, "future": 1}}}"#,
        )
        .unwrap();
        assert_eq!(profile.output.tick_rate(), TickRate::Fps30);
        assert_eq!(profile.output.virtual_camera.fps, 60);
        assert_eq!(profile.output.virtual_camera.width, 1280);
    }

    #[test]
    fn sanitizes_the_output_settings() {
        let profile = read(
            r##"{"format": "ugoe-studio-profile", "version": 1, "output": {"tickRate": 144, "renderScale": 0.6,
                "virtualCamera": {"width": 640, "height": 480, "fps": 25, "background": "green"}}}"##,
        )
        .unwrap();
        assert_eq!(profile.output.tick_rate, 60);
        assert_eq!(profile.output.render_scale, 1.0);
        let camera = &profile.output.virtual_camera;
        assert_eq!((camera.width, camera.height, camera.fps), (1280, 720, 30));
        assert_eq!(camera.background, "#f0f0f0");
        let profile = read(
            r##"{"format": "ugoe-studio-profile", "version": 1,
                "output": {"virtualCamera": {"width": 1920, "height": 720, "background": "#00FF00"}}}"##,
        )
        .unwrap();
        let camera = &profile.output.virtual_camera;
        assert_eq!((camera.width, camera.height), (1280, 720));
        assert_eq!(camera.background, "#00ff00");
        assert_eq!(camera.settings().background, [0, 255, 0]);
    }

    #[test]
    fn parses_hex_colors() {
        assert_eq!(parse_hex_color("#0080ff"), Some([0, 128, 255]));
        assert_eq!(parse_hex_color("#0080FF"), Some([0, 128, 255]));
        assert_eq!(parse_hex_color("0080ff"), None);
        assert_eq!(parse_hex_color("#0080f"), None);
        assert_eq!(parse_hex_color("#ｆｆ"), None);
    }

    #[test]
    fn reads_the_documented_example() {
        // docs/studio.md#プロファイルの形式 の例と同じ。
        let profile = read(
            r##"{
              "format": "ugoe-studio-profile",
              "version": 1,
              "scenes": [
                {
                  "id": "scene-1",
                  "name": "Chat",
                  "model": "/home/me/avatars/avatar.ugoe",
                  "modules": ["/home/me/avatars/cat-ears.ugoemod"],
                  "lighting": null,
                  "transform": { "x": 0, "y": 0.05, "scale": 1.2, "rotation": 0 },
                  "background": { "kind": "transparent" },
                  "pose": ["ArmB"]
                },
                {
                  "id": "scene-2",
                  "name": "Game",
                  "model": "/home/me/avatars/avatar.ugoe",
                  "background": { "kind": "chroma", "color": "#00ff00" }
                }
              ],
              "activeScene": "scene-1",
              "output": {
                "size": "1920x1080",
                "tickRate": 60,
                "virtualCamera": { "width": 1280, "height": 720, "fps": 30, "background": "#f0f0f0" }
              }
            }"##,
        )
        .unwrap();
        assert_eq!(profile.scenes.len(), 2);
        assert_eq!(profile.output, OutputSettings::default());
        assert_eq!(profile.scenes[0].transform.scale, 1.2);
        assert_eq!(profile.scenes[1].transform, Transform::default());
        assert!(profile.scenes[1].modules.is_empty());
        assert_eq!(profile.scenes[0].pose, ["ArmB"]);
        assert!(profile.scenes[1].pose.is_empty());
        assert_eq!(
            profile.scenes[1].background,
            Background::Chroma {
                color: "#00ff00".into()
            }
        );
    }

    #[test]
    fn checks_the_format_and_the_version() {
        assert!(matches!(read("{"), Err(ProfileError::NotJson { .. })));
        assert!(matches!(
            read(r#"{"version": 1}"#),
            Err(ProfileError::NotProfile { .. })
        ));
        assert!(matches!(
            read(r#"{"format": "ugoe-studio-profile"}"#),
            Err(ProfileError::Invalid { .. })
        ));
        assert!(matches!(
            read(r#"{"format": "ugoe-studio-profile", "version": 0}"#),
            Err(ProfileError::Invalid { .. })
        ));
        let future = read(r#"{"format": "ugoe-studio-profile", "version": 2, "scenes": "?"}"#);
        assert_eq!(
            future,
            Err(ProfileError::FutureVersion {
                path: "p.json".into(),
                version: 2,
                supported: 1
            })
        );
        // 画面は code で文言を選ぶ。
        assert_eq!(
            serde_json::to_value(future.unwrap_err()).unwrap(),
            serde_json::json!({ "code": "futureVersion", "path": "p.json", "version": 2, "supported": 1 })
        );
        assert!(matches!(
            read(r#"{"format": "ugoe-studio-profile", "version": 1, "scenes": [{"name": "x"}]}"#),
            Err(ProfileError::Invalid { .. })
        ));
    }

    #[test]
    fn sanitizes_values() {
        let profile = read(
            r##"{"format": "ugoe-studio-profile", "version": 1, "activeScene": "nope", "scenes": [
              {"id": "a", "name": "A", "transform": {"x": 9, "y": -9, "scale": 0, "rotation": 720},
               "background": {"kind": "color", "color": "red"}, "lighting": 3},
              {"id": "a", "name": "B", "background": {"kind": "chroma", "color": "#00FF00"},
               "pose": ["ArmB", "", "Hat/Off", "ArmB"]},
              {"id": " ", "name": "C"}
            ]}"##,
        )
        .unwrap();
        let ids: Vec<_> = profile.scenes.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["a", "scene-1", "scene-2"]);
        assert_eq!(profile.active_scene.as_deref(), Some("a"));
        let a = &profile.scenes[0];
        assert_eq!(
            a.transform,
            Transform {
                x: 2.0,
                y: -2.0,
                scale: 0.05,
                rotation: 360.0
            }
        );
        assert_eq!(
            a.background,
            Background::Color {
                color: "#000000".into()
            }
        );
        assert_eq!(a.lighting, None);
        assert_eq!(
            profile.scenes[1].background,
            Background::Chroma {
                color: "#00ff00".into()
            }
        );
        // ポーズの要素は空と重複を除く (T0842)。
        assert_eq!(profile.scenes[1].pose, ["ArmB", "Hat/Off"]);
        let mut many = SceneSettings::new("m".into(), String::new());
        many.pose = (0..MAX_POSE_ENTRIES + 5).map(|i| format!("P{i}")).collect();
        many.sanitize();
        assert_eq!(many.pose.len(), MAX_POSE_ENTRIES);
    }

    /// 見本の印は読むが、書かない (写しには付けない。T0866)。
    #[test]
    fn reads_the_template_mark_but_does_not_write_it() {
        let profile =
            read(r#"{"format": "ugoe-studio-profile", "version": 1, "template": true}"#).unwrap();
        assert!(profile.template);
        assert!(
            !read(r#"{"format": "ugoe-studio-profile", "version": 1}"#)
                .unwrap()
                .template
        );
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("copy.json");
        profile.write(&path).unwrap();
        assert!(!std::fs::read_to_string(&path).unwrap().contains("template"));
        assert!(!Profile::read(&path).unwrap().template);
        assert!(is_writable(&path));
        assert!(!is_writable(&tmp.path().join("missing.json")));
    }

    #[test]
    fn writes_and_reads_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let path = dir.join("p.json");
        let profile = Profile::new();
        profile.write(&path).unwrap();
        assert_eq!(Profile::read(&path).unwrap(), profile);
        // 一時ファイルは残らない。
        let names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["p.json"]);
        assert!(matches!(
            Profile::read(&dir.join("missing.json")),
            Err(ProfileError::Io { .. })
        ));
        // 通常のファイルでなければ読まない (T0011)。
        let folder = dir.join("folder.json");
        std::fs::create_dir_all(&folder).unwrap();
        assert!(matches!(
            Profile::read(&folder),
            Err(ProfileError::Io { .. })
        ));
    }

    /// ファイルのパスをすべて持つプロファイル (モデル・モジュール・背景の画像・ホットキーの着せ替えのモジュール)。
    fn profile_with_paths(model: &str, module: &str, background: &str, hotkey: &str) -> String {
        serde_json::json!({
            "format": "ugoe-studio-profile",
            "version": 1,
            "scenes": [{
                "id": "a",
                "name": "A",
                "model": model,
                "modules": [module],
                "background": { "kind": "image", "path": background }
            }],
            "hotkeys": [
                { "keys": "Ctrl+1", "action": { "kind": "dressUp", "slot": "hat", "module": hotkey } }
            ]
        })
        .to_string()
    }

    /// プロファイルが持つパス (モデル・モジュール・背景の画像・ホットキーのモジュールの順)。
    fn paths_of(profile: &Profile) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        let mut profile = profile.clone();
        profile.map_paths(|p| {
            paths.push(p.to_owned());
            p.to_owned()
        });
        paths
    }

    /// 相対パスはプロファイルのフォルダを基準に解く。フォルダごと別の場所へ写しても、写した先の同じファイルを指す (T0852)。
    #[test]
    fn resolves_relative_paths_against_the_profile_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("from");
        std::fs::create_dir_all(from.join("models")).unwrap();
        std::fs::write(
            from.join("p.json"),
            profile_with_paths(
                "models/avatar.ugoe",
                "./models//hat.ugoemod",
                "bg.png",
                r"models\cap.ugoemod",
            ),
        )
        .unwrap();
        let to = tmp.path().join("to/nested");
        std::fs::create_dir_all(&to).unwrap();
        std::fs::copy(from.join("p.json"), to.join("p.json")).unwrap();
        for dir in [&from, &to] {
            let profile = Profile::read(&dir.join("p.json")).unwrap();
            assert_eq!(
                paths_of(&profile),
                [
                    dir.join("models").join("avatar.ugoe"),
                    dir.join("models").join("hat.ugoemod"),
                    dir.join("bg.png"),
                    dir.join("models").join("cap.ugoemod"),
                ]
            );
        }
        // 絶対パスはそのまま。
        let absolute = tmp.path().join("elsewhere/avatar.ugoe");
        assert_eq!(resolve_path(&from, &absolute), absolute);
        // 相対パスのプロファイルを開いても、今のディレクトリではなくプロファイルのフォルダを基準にする。
        let relative = Path::new("p.json");
        assert_eq!(
            base_dir(relative),
            std::env::current_dir().unwrap(),
            "the base of a relative profile path is absolute"
        );
    }

    /// プロファイルのフォルダの外に出られる相対パスは解かない (相対パスのまま残し、読めないパスとして扱う)。T0852。
    #[test]
    fn leaves_relative_paths_that_escape_the_profile_folder_unresolved() {
        let base = Path::new("/home/me/show");
        for path in [
            "../secret.ugoe",
            "models/../../secret.ugoe",
            r"models\..\..\secret.ugoe",
            "models/.../secret.ugoe",
            "models/.. /secret.ugoe",
            r"\secret.ugoe",
            r"\\server\share\secret.ugoe",
            r"C:\secret.ugoe",
            "C:secret.ugoe",
            "C:/secret.ugoe",
            "models/a.ugoe:stream",
            "",
            ".",
            "./",
        ] {
            assert_eq!(
                resolve_path(base, Path::new(path)),
                Path::new(path),
                "{path:?}"
            );
        }
        // 名前の中の点は構わない。
        assert_eq!(
            resolve_path(base, Path::new("..models/a..ugoe")),
            base.join("..models").join("a..ugoe")
        );
        // 読んだプロファイルでも、すべての種類のパスでそのまま残る。
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("p.json");
        std::fs::write(
            &path,
            profile_with_paths("../a.ugoe", "../m.ugoemod", "../bg.png", "../c.ugoemod"),
        )
        .unwrap();
        let profile = Profile::read(&path).unwrap();
        assert_eq!(
            paths_of(&profile),
            ["../a.ugoe", "../m.ugoemod", "../bg.png", "../c.ugoemod"].map(PathBuf::from)
        );
        // 書き戻すときもそのまま書く (利用者が直せるように、黙って消さない)。
        profile.write(&path).unwrap();
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains(r#""model": "../a.ugoe""#)
        );
    }

    /// プロファイルのフォルダの下のファイルは相対パス (区切りは `/`)、外のものは絶対パスで書く。
    /// 別の場所へ保存し直すと、新しい場所を基準に書き直す (T0852)。
    #[test]
    fn writes_paths_under_the_profile_folder_as_relative_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let show = tmp.path().join("show");
        let outside = tmp.path().join("library/avatar.ugoe");
        let mut profile = Profile::new();
        profile.scenes[0].model = Some(show.join("models").join("avatar.ugoe"));
        profile.scenes[0].modules = vec![outside.clone()];
        profile.scenes[0].background = Background::Image {
            path: show.join("bg.png"),
        };
        profile.hotkeys = serde_json::from_value(serde_json::json!([
            { "keys": "Ctrl+1", "action": { "kind": "dressUp", "slot": "hat", "module": show.join("hat.ugoemod") } }
        ]))
        .unwrap();
        std::fs::create_dir_all(&show).unwrap();
        let path = show.join("p.json");
        profile.write(&path).unwrap();
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let scene = &written["scenes"][0];
        assert_eq!(scene["model"], "models/avatar.ugoe");
        assert_eq!(scene["modules"][0], outside.to_str().unwrap());
        assert_eq!(scene["background"]["path"], "bg.png");
        assert_eq!(written["hotkeys"][0]["action"]["module"], "hat.ugoemod");
        // 読み直すと同じプロファイル (メモリの上は絶対パス)。
        assert_eq!(Profile::read(&path).unwrap(), profile);

        // 1 つ下のフォルダへ保存し直すと、`show` の直下のファイルは外になるので絶対パスで書く。
        let deeper = show.join("models");
        std::fs::create_dir_all(&deeper).unwrap();
        let path = deeper.join("p.json");
        profile.write(&path).unwrap();
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let scene = &written["scenes"][0];
        assert_eq!(scene["model"], "avatar.ugoe");
        assert_eq!(
            scene["background"]["path"],
            show.join("bg.png").to_str().unwrap()
        );
        assert_eq!(Profile::read(&path).unwrap(), profile);
    }

    /// 相対パスにするかは字面で決める。名前に `\` や `:` を含むなど相対パスで書くと同じファイルに解けないもの、
    /// `..` を含む絶対パス、フォルダそのものは絶対パスのまま書く。
    #[test]
    fn writes_absolute_paths_when_a_relative_path_would_not_round_trip() {
        let base = Path::new("/home/me/show");
        assert_eq!(
            relative_path(base, Path::new("/home/me/show/m/a.ugoe")).as_deref(),
            Some("m/a.ugoe")
        );
        assert_eq!(
            relative_path(base, Path::new("/home/me/showcase/a.ugoe")),
            None
        );
        assert_eq!(
            relative_path(base, Path::new("/home/me/show/../a.ugoe")),
            None
        );
        assert_eq!(relative_path(base, Path::new("/home/me/show")), None);
        assert_eq!(relative_path(base, Path::new("a.ugoe")), None);
        #[cfg(unix)]
        {
            // Unix では `\` と `:` は名前に使える。相対パスで書くと区切りやドライブ名に読むので、絶対パスで書く。
            assert_eq!(
                relative_path(base, Path::new(r"/home/me/show/a\b.ugoe")),
                None
            );
            assert_eq!(
                relative_path(base, Path::new("/home/me/show/a:b.ugoe")),
                None
            );
        }
    }

    /// Windows のパス (区切りの `\`・ドライブ名) の組み立て。Windows でだけ確かめられる (T0852)。
    #[cfg(windows)]
    #[test]
    fn resolves_and_writes_windows_paths() {
        let base = Path::new(r"C:\Users\me\show");
        assert_eq!(
            resolve_path(base, Path::new("models/a.ugoe")),
            Path::new(r"C:\Users\me\show\models\a.ugoe")
        );
        assert_eq!(
            relative_path(base, Path::new(r"C:\Users\me\show\models\a.ugoe")).as_deref(),
            Some("models/a.ugoe")
        );
        assert_eq!(relative_path(base, Path::new(r"D:\show\a.ugoe")), None);
        assert_eq!(
            resolve_path(base, Path::new(r"D:\show\a.ugoe")),
            Path::new(r"D:\show\a.ugoe")
        );
        // ルートから始まる (ドライブ名の無い) パスと UNC は、相対パスとしては解かない。
        assert_eq!(
            resolve_path(base, Path::new(r"\a.ugoe")),
            Path::new(r"\a.ugoe")
        );
        assert_eq!(
            resolve_path(base, Path::new(r"\\server\share\a.ugoe")),
            Path::new(r"\\server\share\a.ugoe")
        );
    }

    /// 記号のリンクはたどらずに字面で解く: リンクしたフォルダから開いたプロファイルは、そのリンクのパスを基準にする。
    /// フォルダの下にある記号のリンクは、そのパスのまま (OS がたどって読む)。書くときもリンクを解かずに比べる (T0852)。
    #[cfg(unix)]
    #[test]
    fn resolves_paths_without_following_symbolic_links() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(
            real.join("p.json"),
            profile_with_paths("link.ugoe", "m.ugoemod", "bg.png", "c.ugoemod"),
        )
        .unwrap();
        let outside = tmp.path().join("outside.ugoe");
        std::fs::write(&outside, "x").unwrap();
        std::os::unix::fs::symlink(&outside, real.join("link.ugoe")).unwrap();
        let linked = tmp.path().join("linked");
        std::os::unix::fs::symlink(&real, &linked).unwrap();

        let profile = Profile::read(&linked.join("p.json")).unwrap();
        let model = profile.scenes[0].model.clone().unwrap();
        assert_eq!(model, linked.join("link.ugoe"));
        // リンクの先 (フォルダの外) は読める (絶対パスで書いたときと同じ)。
        assert_eq!(std::fs::read_to_string(&model).unwrap(), "x");
        // 書き戻すと、リンクのパスのまま相対パスにする。
        profile.write(&linked.join("p.json")).unwrap();
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(real.join("p.json")).unwrap()).unwrap();
        assert_eq!(written["scenes"][0]["model"], "link.ugoe");
        // リンクを解いたパスは、リンクから開いたフォルダの下とはみなさない (絶対パスで書く)。
        assert_eq!(relative_path(&linked, &real.join("m.ugoemod")), None);
    }

    #[test]
    fn recent_profiles_keep_the_newest_first() {
        let mut recent = RecentProfiles::default();
        for i in 0..12 {
            recent.touch(Path::new(&format!("/p/{i}.json")));
        }
        assert_eq!(recent.profiles.len(), RecentProfiles::MAX);
        assert_eq!(recent.profiles[0], Path::new("/p/11.json"));
        recent.touch(Path::new("/p/5.json"));
        assert_eq!(recent.profiles[0], Path::new("/p/5.json"));
        assert_eq!(recent.profiles.len(), RecentProfiles::MAX);
        recent.forget(Path::new("/p/5.json"));
        assert!(!recent.profiles.contains(&PathBuf::from("/p/5.json")));
        assert_eq!(
            RecentProfiles::read(Path::new("/nonexistent/recent.json")),
            RecentProfiles::default()
        );
    }
}
