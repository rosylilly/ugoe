//! 舞台: 開いているプロファイルと、指揮役に出しているシーン (`docs/studio.md#シーンとプロファイル`、ADR 0028 の 4.)。
//!
//! シーンの追加・複製・並べ替え・削除・編集と、シーンの切り替え (ファイルを読んで [`Conductor::present`] に渡す) を受け持つ。
//! 受け手が取るファイルの中身 (モデル・モジュール・背景の画像) は、今のシーンの分だけをここで持つ
//! (任意のパスは読ませない)。Tauri には依存しない (コマンドは `lib.rs`)。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ugoe::model::module::Module;

use crate::conductor::{self, Conductor, NewModel};
use crate::hotkeys::{self, Hotkey};
use crate::input::{InputSettings, MotionSettings};
use crate::profile::{Background, Profile, ProfileError, RecentProfiles, SceneSettings};
use crate::stream::Look;

/// モデルとモジュールのファイルの大きさの上限 (誤って巨大なファイルを選んだときにメモリを使い果たさないため)。
pub const MAX_MODEL_BYTES: u64 = 512 * 1024 * 1024;
/// 背景の画像の大きさの上限。
pub const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;
/// モデルの拡張子。
pub const MODEL_EXTENSIONS: &[&str] = &["ugoe", "json"];
/// モジュールの拡張子。
pub const MODULE_EXTENSIONS: &[&str] = &["ugoemod"];
/// 背景の画像の拡張子。
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// シーンのファイルの種類 (エラーの表示用。受け手が読めなかったことの報告 (`load_failure.rs`) にも使う)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Asset {
    Model,
    Module,
    Background,
}

/// シーンを出せない。画面は `code` で文言を選ぶ (ADR 0029 の 4.)。
#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum SceneError {
    #[error("no scene `{id}`")]
    #[serde(rename = "unknownScene")]
    UnknownScene { id: String },
    /// ファイルが見つからない (別の計算機で作ったプロファイルなど)。画面は選び直させる。
    #[error("{path}: not found")]
    #[serde(rename = "missing")]
    Missing { asset: Asset, path: PathBuf },
    #[error("{path}: {message}")]
    #[serde(rename = "unreadable")]
    Unreadable {
        asset: Asset,
        path: PathBuf,
        message: String,
    },
    /// 拡張子が違う。
    #[error("{path}: unsupported file type")]
    #[serde(rename = "wrongType")]
    WrongType { asset: Asset, path: PathBuf },
    #[error("{path}: the file is too large ({bytes} bytes)")]
    #[serde(rename = "tooLarge")]
    TooLarge {
        asset: Asset,
        path: PathBuf,
        bytes: u64,
    },
    /// モジュールを付けられない (スロットが無いなど)。
    #[error("cannot dress up: {message}")]
    #[serde(rename = "dressUp")]
    DressUp { message: String },
}

/// 操作ウィンドウに出すプロファイルの状態。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileView<'a> {
    pub profile: &'a Profile,
    /// 保存先。まだ保存していなければ `None`。
    pub path: Option<&'a Path>,
    /// 保存した後に変えたか。
    pub dirty: bool,
    /// 指揮役に出しているシーンの ID。
    pub live: Option<&'a str>,
    /// 最近のプロファイル (新しいものが先頭)。
    pub recent: &'a [PathBuf],
}

/// 今のシーンのファイルの中身。
#[derive(Debug, Default)]
struct Assets {
    model: Option<(PathBuf, Arc<Vec<u8>>)>,
    modules: Vec<LoadedModule>,
    background: Option<(PathBuf, Arc<Vec<u8>>)>,
}

#[derive(Debug, Clone)]
struct LoadedModule {
    path: PathBuf,
    bytes: Arc<Vec<u8>>,
    /// 同じパスのモジュールは同じ `Arc` を渡す (指揮役は `Arc` で同じ組かを見分ける)。
    module: Arc<Module>,
}

/// 舞台。
#[derive(Debug, Default)]
pub struct Stage {
    profile: Profile,
    path: Option<PathBuf>,
    dirty: bool,
    live: Option<String>,
    assets: Assets,
    recent: RecentProfiles,
    /// 最近のプロファイルを書く場所 (アプリのデータの場所)。`None` なら覚えない。
    recent_path: Option<PathBuf>,
}

impl Stage {
    /// 空のシーンを 1 つ持つ、まだ保存していないプロファイルで始める。`recent_path` から最近のプロファイルを読む。
    pub fn new(recent_path: Option<PathBuf>) -> Self {
        let recent = recent_path
            .as_deref()
            .map(RecentProfiles::read)
            .unwrap_or_default();
        Self {
            recent,
            recent_path,
            ..Self::default()
        }
    }

    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// 指揮役に出しているシーンの ID。
    pub fn live(&self) -> Option<&str> {
        self.live.as_deref()
    }

    pub fn recent(&self) -> &[PathBuf] {
        &self.recent.profiles
    }

    pub fn view(&self) -> ProfileView<'_> {
        ProfileView {
            profile: &self.profile,
            path: self.path(),
            dirty: self.dirty,
            live: self.live(),
            recent: self.recent(),
        }
    }

    // --- プロファイル ---

    /// 新しいプロファイルにする (空のシーンを 1 つ。指揮役のモデルは外す)。
    pub fn new_profile(&mut self, conductor: &mut Conductor) {
        self.profile = Profile::new();
        self.path = None;
        self.dirty = false;
        self.live = None;
        self.assets = Assets::default();
        conductor.clear(Look::default());
        self.apply_motion_settings(conductor);
    }

    /// プロファイルを開いて、最後に使ったシーンに切り替える。プロファイルを読めなければ何も変えない。
    /// 開けたがシーンを出せなかったときは、`Ok(Some(エラー))` (プロファイルは開いたまま、指揮役は前のシーンのまま)。
    pub fn open_profile(
        &mut self,
        path: &Path,
        conductor: &mut Conductor,
    ) -> Result<Option<SceneError>, ProfileError> {
        let profile = match Profile::read(path) {
            Ok(profile) => profile,
            Err(e) => {
                // 消えたファイルは最近の一覧から外す。
                if matches!(&e, ProfileError::Io { .. }) && !path.exists() {
                    self.recent.forget(path);
                    self.write_recent();
                }
                return Err(e);
            }
        };
        self.profile = profile;
        self.path = Some(path.to_owned());
        self.dirty = false;
        self.live = None;
        self.touch_recent(path);
        self.apply_motion_settings(conductor);
        let Some(id) = self.profile.active_scene.clone() else {
            self.assets = Assets::default();
            conductor.clear(Look::default());
            return Ok(None);
        };
        let error = self.activate(&id, conductor).err();
        // 開いた直後は保存したものと同じ (切り替えで active_scene を書き直しても変わらない)。
        self.dirty = false;
        Ok(error)
    }

    /// 保存する。`path` を省くと今の保存先 (無ければ `None` を返して何もしない)。保存したら保存先を返す。
    pub fn save_profile(&mut self, path: Option<&Path>) -> Result<Option<PathBuf>, ProfileError> {
        let Some(path) = path.map(Path::to_owned).or_else(|| self.path.clone()) else {
            return Ok(None);
        };
        self.profile.write(&path)?;
        self.path = Some(path.clone());
        self.dirty = false;
        self.touch_recent(&path);
        Ok(Some(path))
    }

    /// 最近の一覧から外す。
    pub fn forget_recent(&mut self, path: &Path) {
        self.recent.forget(path);
        self.write_recent();
    }

    /// 出力ウィンドウの大きさのプリセットを変える。
    pub fn set_output_size(&mut self, size: &str) {
        if self.profile.output.size != size {
            self.profile.output.size = size.to_owned();
            self.dirty = true;
        }
    }

    /// ホットキーの割り当てを置き換える (T0803)。
    pub fn set_hotkeys(&mut self, mut list: Vec<Hotkey>) {
        hotkeys::sanitize(&mut list);
        if self.profile.hotkeys != list {
            self.profile.hotkeys = list;
            self.dirty = true;
        }
    }

    /// 入力の機器と調整を置き換える (T0803)。
    pub fn set_input_settings(&mut self, mut settings: InputSettings) {
        settings.sanitize();
        if self.profile.input != settings {
            self.profile.input = settings;
            self.dirty = true;
        }
    }

    /// モーションの設定を置き換え、指揮役に渡す (T0803)。
    pub fn set_motion_settings(&mut self, mut settings: MotionSettings, conductor: &mut Conductor) {
        settings.sanitize();
        if self.profile.motion != settings {
            self.profile.motion = settings;
            self.dirty = true;
            self.apply_motion_settings(conductor);
        }
    }

    fn apply_motion_settings(&self, conductor: &mut Conductor) {
        let settings = self.profile.motion.clone();
        conductor.perform(|animation, puppet| animation.set_settings(puppet, settings));
    }

    fn touch_recent(&mut self, path: &Path) {
        self.recent.touch(path);
        self.write_recent();
    }

    fn write_recent(&self) {
        if let Some(recent_path) = &self.recent_path {
            // 最近の一覧は失っても困らないので、書けなくても続ける。
            let _ = self.recent.write(recent_path);
        }
    }

    // --- シーンの一覧 ---

    /// モデルの無いシーンを最後に加え、その ID を返す。
    pub fn add_scene(&mut self, name: &str) -> String {
        let id = self.profile.fresh_id();
        let mut scene = SceneSettings::new(id.clone(), name.to_owned());
        scene.sanitize();
        self.profile.scenes.push(scene);
        self.dirty = true;
        id
    }

    /// シーン `id` の写しをすぐ後ろに加え、その ID を返す。
    pub fn duplicate_scene(&mut self, id: &str, name: &str) -> Result<String, SceneError> {
        let index = self.index_of(id)?;
        let mut copy = self.profile.scenes[index].clone();
        copy.id = self.profile.fresh_id();
        copy.name = name.to_owned();
        copy.sanitize();
        let new_id = copy.id.clone();
        self.profile.scenes.insert(index + 1, copy);
        self.dirty = true;
        Ok(new_id)
    }

    /// シーン `id` を `to` 番目 (0 から。範囲外は端) に動かす。
    pub fn move_scene(&mut self, id: &str, to: usize) -> Result<(), SceneError> {
        let from = self.index_of(id)?;
        let to = to.min(self.profile.scenes.len() - 1);
        if from != to {
            let scene = self.profile.scenes.remove(from);
            self.profile.scenes.insert(to, scene);
            self.dirty = true;
        }
        Ok(())
    }

    /// シーン `id` を消す。出しているシーンなら、指揮役はそのまま (次に切り替えるまで出し続ける)。
    pub fn remove_scene(&mut self, id: &str) -> Result<(), SceneError> {
        let index = self.index_of(id)?;
        self.profile.scenes.remove(index);
        if self.live.as_deref() == Some(id) {
            self.live = None;
        }
        if self.profile.active_scene.as_deref() == Some(id) {
            self.profile.active_scene = None;
        }
        self.dirty = true;
        Ok(())
    }

    /// シーンの設定を置き換える (`scene.id` で探す。値は収め直す)。出しているシーンなら出し直し、出せなければ元の設定に戻す。
    pub fn update_scene(
        &mut self,
        mut scene: SceneSettings,
        conductor: &mut Conductor,
    ) -> Result<(), SceneError> {
        scene.sanitize();
        let index = self.index_of(&scene.id)?;
        if self.profile.scenes[index] == scene {
            return Ok(());
        }
        let previous = std::mem::replace(&mut self.profile.scenes[index], scene);
        self.dirty = true;
        if self.live.as_deref() != Some(previous.id.as_str()) {
            return Ok(());
        }
        let id = previous.id.clone();
        self.activate(&id, conductor).inspect_err(|_| {
            self.profile.scenes[index] = previous;
            // 戻したので、出しているシーンはそのまま。
            self.live = Some(id.clone());
        })
    }

    fn index_of(&self, id: &str) -> Result<usize, SceneError> {
        self.profile
            .index_of(id)
            .ok_or_else(|| SceneError::UnknownScene { id: id.to_owned() })
    }

    // --- 切り替え ---

    /// シーン `id` に切り替える (指揮役に出す)。ファイルを読めなければ何も変えない (指揮役は前のシーンのまま)。
    ///
    /// 同じモデル (同じパス) のシーンどうしでは、モデルを読み直さず、パラメータの値や再生中のモーションを引き継ぐ。
    /// 背景の画像が読めないときは透過にして続ける (配信中に切り替えを止めない)。
    pub fn activate(&mut self, id: &str, conductor: &mut Conductor) -> Result<(), SceneError> {
        let scene = self.profile.scenes[self.index_of(id)?].clone();
        let mut look = Look {
            lighting: scene.lighting.clone(),
            transform: scene.transform,
            background: scene.background.clone(),
        };
        let background = match &scene.background {
            Background::Image { path } => match self.background_bytes(path) {
                Ok(bytes) => Some((path.clone(), bytes)),
                Err(_) => {
                    look.background = Background::Transparent;
                    None
                }
            },
            _ => None,
        };
        let Some(model_path) = &scene.model else {
            conductor.clear(look);
            self.assets = Assets {
                background,
                ..Assets::default()
            };
            self.set_live(id);
            return Ok(());
        };
        let same_model = conductor.scene().is_some_and(|s| s.model_revision != 0)
            && self
                .assets
                .model
                .as_ref()
                .is_some_and(|(path, _)| path == model_path);
        let (model, new_model) = if same_model {
            (self.assets.model.clone(), None)
        } else {
            let bytes = read_asset(Asset::Model, model_path, MODEL_EXTENSIONS, MAX_MODEL_BYTES)?;
            let bundle = conductor::read_model(&bytes).map_err(|e| SceneError::Unreadable {
                asset: Asset::Model,
                path: model_path.clone(),
                message: e.to_string(),
            })?;
            let name = file_name(model_path);
            (
                Some((model_path.clone(), Arc::new(bytes))),
                Some(NewModel { name, bundle }),
            )
        };
        let modules = scene
            .modules
            .iter()
            .map(|path| self.module(path))
            .collect::<Result<Vec<_>, _>>()?;
        conductor
            .present(
                new_model,
                Some(modules.iter().map(|m| Arc::clone(&m.module)).collect()),
                look,
            )
            .map_err(|e| match e {
                conductor::LoadError::DressUp(e) => SceneError::DressUp {
                    message: e.to_string(),
                },
                e => SceneError::Unreadable {
                    asset: Asset::Model,
                    path: model_path.clone(),
                    message: e.to_string(),
                },
            })?;
        self.assets = Assets {
            model,
            modules,
            background,
        };
        self.set_live(id);
        Ok(())
    }

    /// 出しているシーンの着せ替え (ホットキー、T0803): スロット `slot` に付けているモジュールを外し、`module` があれば付ける。
    /// シーンの設定 (`modules`) を書き換えて出し直す。出せなければ元に戻してエラー。出しているシーンが無ければ何もしない。
    pub fn dress_up(
        &mut self,
        slot: &str,
        module: Option<&Path>,
        conductor: &mut Conductor,
    ) -> Result<(), SceneError> {
        let Some(id) = self.live.clone() else {
            return Ok(());
        };
        let mut scene = self.profile.scenes[self.index_of(&id)?].clone();
        let mut modules = Vec::with_capacity(scene.modules.len() + 1);
        for path in &scene.modules {
            if self.module(path)?.module.slot.as_str() != slot {
                modules.push(path.clone());
            }
        }
        if let Some(path) = module {
            let loaded = self.module(path)?;
            if loaded.module.slot.as_str() != slot {
                // 割り当てたスロットと違うスロットのモジュール: そのモジュールのスロットを空けてから付ける。
                let other = loaded.module.slot.as_str().to_owned();
                let mut kept = Vec::with_capacity(modules.len());
                for p in modules {
                    if self.module(&p)?.module.slot.as_str() != other {
                        kept.push(p);
                    }
                }
                modules = kept;
            }
            modules.push(path.to_owned());
        }
        scene.modules = modules;
        self.update_scene(scene, conductor)
    }

    fn set_live(&mut self, id: &str) {
        self.live = Some(id.to_owned());
        if self.profile.active_scene.as_deref() != Some(id) {
            self.profile.active_scene = Some(id.to_owned());
            self.dirty = true;
        }
    }

    /// モジュールを読む (今のシーンで読んであれば、それを使う)。
    fn module(&self, path: &Path) -> Result<LoadedModule, SceneError> {
        if let Some(loaded) = self.assets.modules.iter().find(|m| m.path == path) {
            return Ok(loaded.clone());
        }
        let bytes = read_asset(Asset::Module, path, MODULE_EXTENSIONS, MAX_MODEL_BYTES)?;
        let file = ugoe::format::read_module(&bytes).map_err(|e| SceneError::Unreadable {
            asset: Asset::Module,
            path: path.to_owned(),
            message: e.to_string(),
        })?;
        Ok(LoadedModule {
            path: path.to_owned(),
            bytes: Arc::new(bytes),
            module: Arc::new(file.module),
        })
    }

    fn background_bytes(&self, path: &Path) -> Result<Arc<Vec<u8>>, SceneError> {
        if let Some((loaded, bytes)) = &self.assets.background
            && loaded == path
        {
            return Ok(Arc::clone(bytes));
        }
        read_asset(Asset::Background, path, IMAGE_EXTENSIONS, MAX_IMAGE_BYTES).map(Arc::new)
    }

    // --- 受け手が取るファイル (今のシーンの分だけ) ---

    /// 今のシーンのモデルのバイト列。
    pub fn model_bytes(&self) -> Option<Arc<Vec<u8>>> {
        self.assets.model.as_ref().map(|(_, b)| Arc::clone(b))
    }

    /// 今のシーンの `index` 番目 (合成の順) のモジュールのバイト列。
    pub fn module_bytes(&self, index: usize) -> Option<Arc<Vec<u8>>> {
        self.assets.modules.get(index).map(|m| Arc::clone(&m.bytes))
    }

    /// 今のシーンの背景の画像のバイト列。
    pub fn background_image(&self) -> Option<Arc<Vec<u8>>> {
        self.assets.background.as_ref().map(|(_, b)| Arc::clone(b))
    }

    /// 今のシーンに付けているモジュールのパスと中身 (合成の順。外部 API の着せ替えの一覧、T0805)。
    pub fn live_modules(&self) -> impl Iterator<Item = (&Path, &Module)> {
        self.assets
            .modules
            .iter()
            .map(|m| (m.path.as_path(), m.module.as_ref()))
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// 拡張子と大きさを確かめてから読む。
pub fn read_asset(
    asset: Asset,
    path: &Path,
    extensions: &[&str],
    max_bytes: u64,
) -> Result<Vec<u8>, SceneError> {
    let ok = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| extensions.iter().any(|x| e.eq_ignore_ascii_case(x)));
    if !ok {
        return Err(SceneError::WrongType {
            asset,
            path: path.to_owned(),
        });
    }
    let io = |e: std::io::Error| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SceneError::Missing {
                asset,
                path: path.to_owned(),
            }
        } else {
            SceneError::Unreadable {
                asset,
                path: path.to_owned(),
                message: e.to_string(),
            }
        }
    };
    let bytes = std::fs::metadata(path).map_err(io)?.len();
    if bytes > max_bytes {
        return Err(SceneError::TooLarge {
            asset,
            path: path.to_owned(),
            bytes,
        });
    }
    std::fs::read(path).map_err(io)
}

#[cfg(test)]
mod tests {
    use ugoe::format::ModuleFile;
    use ugoe::model::fixtures;

    use super::*;
    use crate::profile::Transform;

    /// テスト用のファイルを置く場所 (テストごとに分ける)。
    struct Dir(PathBuf);

    impl Dir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("ugoe-studio-stage-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn model(&self, name: &str, model: ugoe::model::Model) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, ugoe::format::model_to_json(&model).unwrap()).unwrap();
            path
        }

        fn module(&self, name: &str, module: Module) -> PathBuf {
            let path = self.0.join(name);
            // 書き出しはテクスチャの数と先頭のシグネチャだけを確かめる (中身はデコードしない)。
            let mut file = ModuleFile::new(module);
            file.textures = vec![
                ugoe::format::TextureData {
                    format: ugoe::format::TextureFormat::Png,
                    bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
                };
                file.module.textures.len()
            ];
            let bytes = ugoe::format::write_module(&file).unwrap();
            std::fs::write(&path, bytes).unwrap();
            path
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn scene(stage: &mut Stage, model: Option<&Path>, modules: &[&Path]) -> SceneSettings {
        let id = stage.add_scene("s");
        let mut settings = stage.profile().scene(&id).unwrap().clone();
        settings.model = model.map(Path::to_owned);
        settings.modules = modules.iter().map(|p| p.to_path_buf()).collect();
        settings
    }

    #[test]
    fn edits_the_scene_list() {
        let mut stage = Stage::new(None);
        let first = stage.profile().scenes[0].id.clone();
        let second = stage.add_scene("Game");
        assert!(stage.dirty());
        let copy = stage.duplicate_scene(&first, "Copy").unwrap();
        let ids = |s: &Stage| -> Vec<String> {
            s.profile().scenes.iter().map(|s| s.id.clone()).collect()
        };
        assert_eq!(ids(&stage), [first.clone(), copy.clone(), second.clone()]);
        stage.move_scene(&second, 0).unwrap();
        assert_eq!(ids(&stage), [second.clone(), first.clone(), copy.clone()]);
        stage.move_scene(&second, 99).unwrap();
        assert_eq!(ids(&stage), [first.clone(), copy.clone(), second.clone()]);
        stage.remove_scene(&copy).unwrap();
        assert_eq!(ids(&stage), [first, second]);
        assert!(matches!(
            stage.remove_scene("nope"),
            Err(SceneError::UnknownScene { .. })
        ));
    }

    #[test]
    fn switches_scenes_and_keeps_the_model_when_it_is_the_same() {
        let dir = Dir::new("switch");
        let base = dir.model("base.json", fixtures::dress_up_base());
        let hair = dir.module("hair.ugoemod", fixtures::long_hair_module());
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();

        let a = scene(&mut stage, Some(&base), &[]);
        let mut b = scene(&mut stage, Some(&base), &[&hair]);
        b.transform = Transform {
            rotation: 15.0,
            ..Transform::default()
        };
        stage.update_scene(a.clone(), &mut conductor).unwrap();
        stage.update_scene(b.clone(), &mut conductor).unwrap();

        stage.activate(&a.id, &mut conductor).unwrap();
        assert_eq!(stage.live(), Some(a.id.as_str()));
        assert_eq!(stage.profile().active_scene.as_deref(), Some(a.id.as_str()));
        let first = conductor.scene().unwrap().clone();
        assert_eq!(first.model_name, "base.json");
        assert!(stage.model_bytes().is_some());
        conductor.set_parameter("AngleX", 10.0);

        stage.activate(&b.id, &mut conductor).unwrap();
        let second = conductor.scene().unwrap().clone();
        assert_eq!(second.model_revision, first.model_revision);
        assert_eq!(second.modules.len(), 1);
        assert_eq!(second.look.transform.rotation, 15.0);
        assert!(stage.module_bytes(0).is_some());
        // 同じモデルなので値を引き継ぐ。
        let index = second
            .parameter_ids
            .iter()
            .position(|id| id == "AngleX")
            .unwrap();
        assert_eq!(conductor.tick(0.016).unwrap().values[index], 10.0);

        // 見せ方だけを変えても、モジュールは読み直さない (同じ Arc なので着せ替えにならない)。
        b.transform.rotation = 30.0;
        stage.update_scene(b.clone(), &mut conductor).unwrap();
        let third = conductor.scene().unwrap().clone();
        assert_eq!(third.modules_revision, second.modules_revision);
        assert!(third.revision > second.revision);
    }

    #[test]
    fn dressing_up_swaps_the_module_in_the_slot() {
        let dir = Dir::new("dress-up");
        let base = dir.model("base.json", fixtures::dress_up_base());
        let long = fixtures::long_hair_module();
        let slot = long.slot.as_str().to_owned();
        let mut short = long.clone();
        short.name = "ShortHair".into();
        let long = dir.module("long.ugoemod", long);
        let short = dir.module("short.ugoemod", short);
        let dress = dir.module("dress.ugoemod", fixtures::dress_module());
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();
        // 出しているシーンが無ければ何もしない。
        stage.dress_up(&slot, Some(&long), &mut conductor).unwrap();

        let live = scene(&mut stage, Some(&base), &[&long, &dress]);
        stage.update_scene(live.clone(), &mut conductor).unwrap();
        stage.activate(&live.id, &mut conductor).unwrap();
        let modules = |stage: &Stage| stage.profile().scene(&live.id).unwrap().modules.clone();

        stage.dress_up(&slot, Some(&short), &mut conductor).unwrap();
        assert_eq!(modules(&stage), [dress.clone(), short.clone()]);
        assert_eq!(conductor.scene().unwrap().modules.len(), 2);
        assert!(conductor.tick(0.016).unwrap().reset);

        stage.dress_up(&slot, None, &mut conductor).unwrap();
        assert_eq!(modules(&stage), std::slice::from_ref(&dress));
        assert_eq!(conductor.scene().unwrap().modules.len(), 1);
        assert!(stage.dirty());

        // 読めないモジュールなら元のまま。
        let error = stage.dress_up(&slot, Some(&dir.0.join("gone.ugoemod")), &mut conductor);
        assert!(matches!(error, Err(SceneError::Missing { .. })));
        assert_eq!(modules(&stage), [dress]);
    }

    #[test]
    fn a_missing_model_leaves_the_live_scene_alone() {
        let dir = Dir::new("missing");
        let base = dir.model("base.json", fixtures::dress_up_base());
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();
        let good = scene(&mut stage, Some(&base), &[]);
        let gone = scene(&mut stage, Some(&dir.0.join("gone.ugoe")), &[]);
        stage.update_scene(good.clone(), &mut conductor).unwrap();
        stage.update_scene(gone.clone(), &mut conductor).unwrap();
        stage.activate(&good.id, &mut conductor).unwrap();
        let revision = conductor.scene().unwrap().revision;

        let error = stage.activate(&gone.id, &mut conductor).unwrap_err();
        assert!(matches!(
            error,
            SceneError::Missing {
                asset: Asset::Model,
                ..
            }
        ));
        assert_eq!(
            serde_json::to_value(&error).unwrap()["code"],
            serde_json::json!("missing")
        );
        assert_eq!(conductor.scene().unwrap().revision, revision);
        assert_eq!(stage.live(), Some(good.id.as_str()));

        // 出しているシーンの編集で出せなくなったら、元の設定に戻す。
        let mut broken = good.clone();
        broken.modules = vec![dir.0.join("nope.ugoemod")];
        assert!(stage.update_scene(broken, &mut conductor).is_err());
        assert_eq!(stage.profile().scene(&good.id).unwrap(), &good);
        assert_eq!(stage.live(), Some(good.id.as_str()));

        // 付けられないモジュール (本体にスロットが無い)。
        let lone = dir.model("lone.json", fixtures::single_quad());
        let hair = dir.module("hair.ugoemod", fixtures::long_hair_module());
        let mut lone_scene = scene(&mut stage, Some(&lone), &[&hair]);
        lone_scene.name = "lone".into();
        stage
            .update_scene(lone_scene.clone(), &mut conductor)
            .unwrap();
        assert!(matches!(
            stage.activate(&lone_scene.id, &mut conductor),
            Err(SceneError::DressUp { .. })
        ));
        assert_eq!(conductor.scene().unwrap().revision, revision);
    }

    #[test]
    fn checks_file_types() {
        let dir = Dir::new("types");
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();
        let text = dir.0.join("notes.txt");
        std::fs::write(&text, "hi").unwrap();
        let s = scene(&mut stage, Some(&text), &[]);
        stage.update_scene(s.clone(), &mut conductor).unwrap();
        assert!(matches!(
            stage.activate(&s.id, &mut conductor),
            Err(SceneError::WrongType { .. })
        ));
        let not_model = dir.0.join("x.json");
        std::fs::write(&not_model, "{}").unwrap();
        let s2 = scene(&mut stage, Some(&not_model), &[]);
        stage.update_scene(s2.clone(), &mut conductor).unwrap();
        assert!(matches!(
            stage.activate(&s2.id, &mut conductor),
            Err(SceneError::Unreadable { .. })
        ));
    }

    #[test]
    fn a_missing_background_falls_back_to_transparent() {
        let dir = Dir::new("background");
        let base = dir.model("base.json", fixtures::dress_up_base());
        let image = dir.0.join("bg.png");
        std::fs::write(&image, b"\x89PNG\r\n\x1a\n").unwrap();
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();
        let mut s = scene(&mut stage, Some(&base), &[]);
        s.background = Background::Image {
            path: image.clone(),
        };
        stage.update_scene(s.clone(), &mut conductor).unwrap();
        stage.activate(&s.id, &mut conductor).unwrap();
        assert_eq!(conductor.scene().unwrap().look.background, s.background);
        assert!(stage.background_image().is_some());

        s.background = Background::Image {
            path: dir.0.join("gone.png"),
        };
        stage.update_scene(s.clone(), &mut conductor).unwrap();
        assert_eq!(
            conductor.scene().unwrap().look.background,
            Background::Transparent
        );
        assert!(stage.background_image().is_none());
    }

    #[test]
    fn a_scene_without_a_model_clears_the_conductor() {
        let dir = Dir::new("empty");
        let base = dir.model("base.json", fixtures::dress_up_base());
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();
        let s = scene(&mut stage, Some(&base), &[]);
        stage.update_scene(s.clone(), &mut conductor).unwrap();
        stage.activate(&s.id, &mut conductor).unwrap();
        let empty = stage.profile().scenes[0].id.clone();
        stage.activate(&empty, &mut conductor).unwrap();
        assert_eq!(conductor.scene().unwrap().model_revision, 0);
        assert!(stage.model_bytes().is_none());
        // モデルを外した後でも、同じモデルに戻れば読み直す。
        stage.activate(&s.id, &mut conductor).unwrap();
        assert_ne!(conductor.scene().unwrap().model_revision, 0);
    }

    #[test]
    fn opens_saves_and_remembers_profiles() {
        let dir = Dir::new("profiles");
        let base = dir.model("base.json", fixtures::dress_up_base());
        let recent = dir.0.join("data/recent-profiles.json");
        let mut stage = Stage::new(Some(recent.clone()));
        let mut conductor = Conductor::new();
        let s = scene(&mut stage, Some(&base), &[]);
        stage.update_scene(s.clone(), &mut conductor).unwrap();
        stage.activate(&s.id, &mut conductor).unwrap();
        // 保存先が無ければ何もしない。
        assert_eq!(stage.save_profile(None).unwrap(), None);
        let path = dir.0.join("profile.json");
        assert_eq!(stage.save_profile(Some(&path)).unwrap(), Some(path.clone()));
        assert!(!stage.dirty());
        assert_eq!(stage.recent(), std::slice::from_ref(&path));

        stage.new_profile(&mut conductor);
        assert_eq!(stage.profile().scenes.len(), 1);
        assert!(conductor.scene().unwrap().parameter_ids.is_empty());

        // 起動し直しても最近の一覧は残り、開くと最後のシーンになる。
        let mut stage = Stage::new(Some(recent.clone()));
        assert_eq!(stage.recent(), std::slice::from_ref(&path));
        assert_eq!(stage.open_profile(&path, &mut conductor).unwrap(), None);
        assert_eq!(stage.live(), Some(s.id.as_str()));
        assert!(!stage.dirty());
        assert_eq!(conductor.scene().unwrap().model_name, "base.json");

        // 未来の版は開かない。
        let future = dir.0.join("future.json");
        std::fs::write(
            &future,
            r#"{"format": "ugoe-studio-profile", "version": 99}"#,
        )
        .unwrap();
        assert!(matches!(
            stage.open_profile(&future, &mut conductor),
            Err(ProfileError::FutureVersion { version: 99, .. })
        ));
        assert_eq!(stage.path(), Some(path.as_path()));

        // 消えたプロファイルは最近の一覧から外す。
        std::fs::remove_file(&path).unwrap();
        assert!(stage.open_profile(&path, &mut conductor).is_err());
        assert!(stage.recent().is_empty());
        assert!(RecentProfiles::read(&recent).profiles.is_empty());
    }

    #[test]
    fn opening_a_profile_with_a_missing_model_still_opens_it() {
        let dir = Dir::new("open-missing");
        let mut profile = Profile::new();
        profile.scenes[0].model = Some(dir.0.join("gone.ugoe"));
        let path = dir.0.join("p.json");
        profile.write(&path).unwrap();
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();
        let error = stage.open_profile(&path, &mut conductor).unwrap();
        assert!(matches!(error, Some(SceneError::Missing { .. })));
        assert_eq!(stage.path(), Some(path.as_path()));
        assert_eq!(stage.live(), None);
    }
}
