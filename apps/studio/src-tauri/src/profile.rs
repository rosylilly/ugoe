//! プロファイル: シーンの一覧と出力の設定を持つ、版の番号付きの JSON の 1 ファイル (`docs/studio.md#プロファイルの形式`、ADR 0028 の 4.)。
//!
//! アプリの設定であって、モデルのファイル形式 (`docs/spec/file-formats.md`) ではない。読み込みでは `format` と `version` を
//! 確かめ、未来の版は [`ProfileError::FutureVersion`] にする (黙って読み落とさない)。値の範囲を外れたものは収め直す
//! ([`Profile::sanitize`])。ホットキー・入力・モーションの設定 (T0803) は同じ版のまま、省略できる項目として足した。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::hotkeys::{self, Hotkey};
use crate::input::{InputSettings, MotionSettings};

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

/// プロファイル。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// 常に [`PROFILE_FORMAT`]。
    pub format: String,
    /// 版 ([`PROFILE_VERSION`])。
    pub version: u32,
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
    /// モデル (`.ugoe` か JSON) の絶対パス。まだ選んでいなければ `None`。
    #[serde(default)]
    pub model: Option<PathBuf>,
    /// 付けるモジュール (`.ugoemod`) の絶対パス (合成の順)。
    #[serde(default)]
    pub modules: Vec<PathBuf>,
    /// 光源の設定 (`@ugoe/player` の `LightingSettings`。読むのは画面と受け手で、ここでは中身を見ない)。`None` は既定の光源。
    #[serde(default)]
    pub lighting: Option<serde_json::Value>,
    #[serde(default)]
    pub transform: Transform,
    #[serde(default)]
    pub background: Background,
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
    /// 画像 (PNG・JPEG・WebP の絶対パス)。出力を覆うように拡大する。
    Image { path: PathBuf },
}

/// 出力の設定。刻みの速さなどは T0804 で足す。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OutputSettings {
    /// 出力ウィンドウの大きさのプリセット (`1920x1080` など。`free` は大きさを変えない)。
    pub size: String,
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            size: "1920x1080".to_owned(),
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

    /// ファイルから読む。
    pub fn read(path: &Path) -> Result<Self, ProfileError> {
        let io = |e: std::io::Error| ProfileError::Io {
            path: path.to_owned(),
            message: e.to_string(),
        };
        let bytes = std::fs::metadata(path).map_err(io)?.len();
        if bytes > MAX_PROFILE_BYTES {
            return Err(ProfileError::TooLarge {
                path: path.to_owned(),
                bytes,
            });
        }
        let text = std::fs::read_to_string(path).map_err(io)?;
        Self::from_json(&text, path)
    }

    /// 整形した JSON (今の版で書く)。
    pub fn to_json(&self) -> String {
        let mut profile = self.clone();
        profile.format = PROFILE_FORMAT.to_owned();
        profile.version = PROFILE_VERSION;
        // 文字列と数と列挙だけなので失敗しない。
        serde_json::to_string_pretty(&profile).unwrap_or_default() + "\n"
    }

    /// ファイルに書く (一時ファイルに書いてから置き換え、途中で落ちても前のファイルを壊さない)。
    pub fn write(&self, path: &Path) -> Result<(), ProfileError> {
        let io = |e: std::io::Error| ProfileError::Io {
            path: path.to_owned(),
            message: e.to_string(),
        };
        let mut temp = path.as_os_str().to_owned();
        temp.push(".tmp");
        let temp = PathBuf::from(temp);
        std::fs::write(&temp, self.to_json()).map_err(io)?;
        std::fs::rename(&temp, path).map_err(io)
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
        hotkeys::sanitize(&mut self.hotkeys);
        self.input.sanitize();
        self.motion.sanitize();
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
        }
    }

    /// 値の範囲を外れたものを収め直す (名前の長さ・位置・大きさ・回転・色)。
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
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap_or_default())
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
        profile.output.size = "1280x720".into();
        assert_eq!(read(&profile.to_json()).unwrap(), profile);
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
                  "background": { "kind": "transparent" }
                },
                {
                  "id": "scene-2",
                  "name": "Game",
                  "model": "/home/me/avatars/avatar.ugoe",
                  "background": { "kind": "chroma", "color": "#00ff00" }
                }
              ],
              "activeScene": "scene-1",
              "output": { "size": "1920x1080" }
            }"##,
        )
        .unwrap();
        assert_eq!(profile.scenes.len(), 2);
        assert_eq!(profile.scenes[0].transform.scale, 1.2);
        assert_eq!(profile.scenes[1].transform, Transform::default());
        assert!(profile.scenes[1].modules.is_empty());
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
              {"id": "a", "name": "B", "background": {"kind": "chroma", "color": "#00FF00"}},
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
    }

    #[test]
    fn writes_and_reads_files() {
        let dir = std::env::temp_dir().join(format!("ugoe-studio-profile-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("p.json");
        let profile = Profile::new();
        profile.write(&path).unwrap();
        assert_eq!(Profile::read(&path).unwrap(), profile);
        assert!(!dir.join("p.json.tmp").exists());
        assert!(matches!(
            Profile::read(&dir.join("missing.json")),
            Err(ProfileError::Io { .. })
        ));
        std::fs::remove_dir_all(&dir).unwrap();
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
