//! 舞台: 開いているプロファイルと、指揮役に出しているシーン (`docs/studio.md#シーンとプロファイル`、ADR 0028 の 4.)。
//!
//! シーンの追加・複製・並べ替え・削除・編集と、シーンの切り替え (ファイルを読んで [`Conductor::present`] に渡す) を受け持つ。
//! 切り替えのファイルは舞台のロックの外で読めるよう、始める・読む・差し込むの 3 つに分ける ([`Pending`]、T0834)。
//! 受け手が取るファイルの中身 (モデル・モジュール・背景の画像) は、今のシーンの分だけをここで持つ
//! (任意のパスは読ませない)。Tauri には依存しない (コマンドは `lib.rs`)。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ugoe::model::module::Module;

use crate::conductor::{self, Conductor, NewModel};
use crate::hotkeys::{self, Hotkey};
use crate::input::{InputSettings, MotionSettings};
use crate::pacing::TickRate;
use crate::profile::{
    Background, Profile, ProfileError, RecentProfiles, SceneSettings, VirtualCameraSettings,
};
use crate::stream::Look;
use ugoe_fs::{ReadError, read_regular_file};

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
    /// 最後に始めた切り替えの番号 (読んでいる間に後から始めた切り替えがあれば、読んだものを捨てる。T0834)。
    tickets: u64,
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
        // 読んでいる途中の切り替えは捨てる。
        self.tickets = self.tickets.wrapping_add(1);
        conductor.clear(Look::default());
        self.apply_motion_settings(conductor);
    }

    /// 読んだプロファイル `read` (`path` から [`Profile::read`] で、舞台のロックの外で読む) を開き、最後に使ったシーンへの切り替えを始める。
    /// 読めなかったら何も変えない。開けたがシーンを出せなかったときは `Ok(Err(エラー))`
    /// (プロファイルは開いたまま、指揮役は前のシーンのまま)。切り替えを差し込んでも未保存にはならない (`activeScene` のシーンなので)。
    pub fn begin_open_profile(
        &mut self,
        path: &Path,
        read: Result<Profile, ProfileError>,
        conductor: &mut Conductor,
    ) -> Result<Result<Option<Pending>, SceneError>, ProfileError> {
        let profile = match read {
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
        // 前のプロファイルの読んでいる途中の切り替えは捨てる。
        self.tickets = self.tickets.wrapping_add(1);
        self.touch_recent(path);
        self.apply_motion_settings(conductor);
        let Some(id) = self.profile.active_scene.clone() else {
            self.assets = Assets::default();
            conductor.clear(Look::default());
            return Ok(Ok(None));
        };
        Ok(self.begin_activate(&id, conductor).map(Some))
    }

    /// プロファイルを開いて、最後に使ったシーンに切り替える (読む間も舞台を握ったまま。テスト用)。
    #[cfg(test)]
    pub fn open_profile(
        &mut self,
        path: &Path,
        conductor: &mut Conductor,
    ) -> Result<Option<SceneError>, ProfileError> {
        let pending = self.begin_open_profile(path, Profile::read(path), conductor)?;
        Ok(pending
            .and_then(|pending| match pending {
                Some(pending) => self.settle(pending, conductor, &read_asset),
                None => Ok(()),
            })
            .err())
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

    /// 刻みの速さを変える (T0827)。
    pub fn set_tick_rate(&mut self, rate: TickRate) {
        if self.profile.output.tick_rate != rate.hz() {
            self.profile.output.tick_rate = rate.hz();
            self.dirty = true;
        }
    }

    /// 仮想カメラの設定を置き換える (T0827。選べない値は既定に戻す)。
    pub fn set_virtual_camera(&mut self, mut settings: VirtualCameraSettings) {
        settings.sanitize();
        if self.profile.output.virtual_camera != settings {
            self.profile.output.virtual_camera = settings;
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

    fn index_of(&self, id: &str) -> Result<usize, SceneError> {
        self.profile
            .index_of(id)
            .ok_or_else(|| SceneError::UnknownScene { id: id.to_owned() })
    }

    // --- 切り替え (docs/studio.md#シーンの切り替えと舞台のロック、T0834) ---
    //
    // 切り替えは 3 つに分ける: 舞台を握って始め (`begin_*`)、握らずに読み ([`Pending::load`])、握り直して差し込む ([`Stage::finish`])。
    // 始めた順の番号 (`tickets`) を持ち、差し込むときに後から始めた切り替えがあれば、読んだものを捨てる。

    /// シーン `id` への切り替えを始める (指揮役に出すのは [`Stage::finish`])。
    ///
    /// 同じモデル (同じパス) のシーンどうしでは、モデルを読み直さず、パラメータの値や再生中のモーションを引き継ぐ。
    /// 背景の画像が読めないときは透過にして続ける (配信中に切り替えを止めない)。
    pub fn begin_activate(
        &mut self,
        id: &str,
        conductor: &Conductor,
    ) -> Result<Pending, SceneError> {
        self.plan(id, Kind::Activate, conductor)
    }

    /// シーンの設定を置き換える (`scene.id` で探す。値は収め直す)。出しているシーンなら、出し直す切り替えを返す
    /// (出せなければ [`Stage::finish`] が元の設定に戻す)。
    pub fn begin_update_scene(
        &mut self,
        mut scene: SceneSettings,
        conductor: &Conductor,
    ) -> Result<Option<Pending>, SceneError> {
        scene.sanitize();
        let index = self.index_of(&scene.id)?;
        if self.profile.scenes[index] == scene {
            return Ok(None);
        }
        let previous = std::mem::replace(&mut self.profile.scenes[index], scene);
        self.dirty = true;
        if self.live.as_deref() != Some(previous.id.as_str()) {
            return Ok(None);
        }
        let id = previous.id.clone();
        self.plan(&id, Kind::Update { previous }, conductor)
            .map(Some)
    }

    /// 出しているシーンの着せ替え (ホットキー、T0803) を始める: スロット `slot` に付けているモジュールを外し、`module` があれば付ける。
    /// 付けるモジュールのスロットは読んでから分かるので、シーンの設定 (`modules`) は差し込むときに書き換える
    /// (出せなければ元のまま)。出しているシーンが無ければ何もしない。
    pub fn begin_dress_up(
        &mut self,
        slot: &str,
        module: Option<&Path>,
        conductor: &Conductor,
    ) -> Result<Option<Pending>, SceneError> {
        let Some(id) = self.live.clone() else {
            return Ok(None);
        };
        let kind = Kind::DressUp {
            slot: slot.to_owned(),
            module: module.map(Path::to_owned),
        };
        self.plan(&id, kind, conductor).map(Some)
    }

    /// 番号を進めて切り替えを始める。
    fn plan(&mut self, id: &str, kind: Kind, conductor: &Conductor) -> Result<Pending, SceneError> {
        self.index_of(id)?;
        self.tickets = self.tickets.wrapping_add(1);
        self.pending(self.tickets, id, kind, conductor)
    }

    fn pending(
        &self,
        ticket: u64,
        id: &str,
        kind: Kind,
        conductor: &Conductor,
    ) -> Result<Pending, SceneError> {
        let scene = self.profile.scenes[self.index_of(id)?].clone();
        // 指揮役がモデルを外していれば、同じパスでも読み直す。
        let model = conductor
            .scene()
            .is_some_and(|s| s.model_revision != 0)
            .then(|| self.assets.model.clone())
            .flatten();
        Ok(Pending {
            ticket,
            id: id.to_owned(),
            scene,
            kind,
            cached: Cached {
                model,
                modules: self.assets.modules.clone(),
                background: self.assets.background.clone(),
            },
        })
    }

    /// 読み終えた切り替えを差し込む (指揮役に出す)。
    ///
    /// - 読めなければ何も変えずにエラー (指揮役は前のシーンのまま。設定を置き換えた編集は、まだ置き換えたままなら元に戻す)。
    /// - 読んでいる間に後から切り替えが始まった (`begin_*`・プロファイルを開く・新しいプロファイル) なら、読んだものを捨てる
    ///   ([`Finish::Superseded`])。差し込むのは最後に始めた切り替えだけ。
    /// - 読んでいる間にシーンの設定が変わったら: 切り替えは今の設定で読み直す ([`Finish::Again`])。編集と着せ替えは捨てる。
    pub fn finish(
        &mut self,
        loaded: Loaded,
        conductor: &mut Conductor,
    ) -> Result<Finish, SceneError> {
        let Loaded {
            ticket,
            id,
            kind,
            expected,
            result,
        } = loaded;
        let latest = ticket == self.tickets;
        let unchanged = self.profile.scene(&id) == Some(&expected);
        if latest && !unchanged && matches!(kind, Kind::Activate) {
            return self
                .pending(ticket, &id, kind, conductor)
                .map(|next| Finish::Again(Box::new(next)));
        }
        let prepared = match result {
            Ok(prepared) => prepared,
            Err(e) => {
                if unchanged {
                    self.revert(&id, kind);
                }
                return Err(e);
            }
        };
        if !latest || !unchanged {
            return Ok(Finish::Superseded);
        }
        if matches!(kind, Kind::DressUp { .. }) && prepared.scene == expected {
            return Ok(Finish::Done);
        }
        let scene = prepared.scene.clone();
        if let Err(e) = self.present(&id, prepared, conductor) {
            self.revert(&id, kind);
            return Err(e);
        }
        if matches!(kind, Kind::DressUp { .. })
            && let Some(index) = self.profile.index_of(&id)
        {
            self.profile.scenes[index] = scene;
            self.dirty = true;
        }
        Ok(Finish::Done)
    }

    /// 出せなかった編集を元の設定に戻す (出しているシーンはそのまま)。
    fn revert(&mut self, id: &str, kind: Kind) {
        if let Kind::Update { previous } = kind
            && let Some(index) = self.profile.index_of(id)
        {
            self.profile.scenes[index] = previous;
        }
    }

    /// 読んだシーンを指揮役に出す。
    fn present(
        &mut self,
        id: &str,
        prepared: Prepared,
        conductor: &mut Conductor,
    ) -> Result<(), SceneError> {
        let Prepared {
            scene,
            look,
            model,
            new_model,
            modules,
            background,
        } = prepared;
        let Some(model_path) = &scene.model else {
            conductor.clear(look);
            self.assets = Assets {
                background,
                ..Assets::default()
            };
            self.set_live(id);
            return Ok(());
        };
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

    /// 始めた切り替えを、舞台を握ったまま読んで差し込む (読むファイルが無いとき・テスト)。
    pub fn settle(
        &mut self,
        mut pending: Pending,
        conductor: &mut Conductor,
        read: &ReadAsset<'_>,
    ) -> Result<(), SceneError> {
        loop {
            match self.finish(pending.load_with(read), conductor)? {
                Finish::Again(next) => pending = *next,
                Finish::Done | Finish::Superseded => return Ok(()),
            }
        }
    }

    /// シーン `id` に切り替える (読む間も舞台を握ったまま。テスト用)。
    #[cfg(test)]
    pub fn activate(&mut self, id: &str, conductor: &mut Conductor) -> Result<(), SceneError> {
        let pending = self.begin_activate(id, conductor)?;
        self.settle(pending, conductor, &read_asset)
    }

    /// [`Stage::begin_update_scene`] と差し込みを続けて行う (テスト用)。
    #[cfg(test)]
    pub fn update_scene(
        &mut self,
        scene: SceneSettings,
        conductor: &mut Conductor,
    ) -> Result<(), SceneError> {
        match self.begin_update_scene(scene, conductor)? {
            Some(pending) => self.settle(pending, conductor, &read_asset),
            None => Ok(()),
        }
    }

    /// [`Stage::begin_dress_up`] と差し込みを続けて行う (テスト用)。
    #[cfg(test)]
    pub fn dress_up(
        &mut self,
        slot: &str,
        module: Option<&Path>,
        conductor: &mut Conductor,
    ) -> Result<(), SceneError> {
        match self.begin_dress_up(slot, module, conductor)? {
            Some(pending) => self.settle(pending, conductor, &read_asset),
            None => Ok(()),
        }
    }

    fn set_live(&mut self, id: &str) {
        self.live = Some(id.to_owned());
        if self.profile.active_scene.as_deref() != Some(id) {
            self.profile.active_scene = Some(id.to_owned());
            self.dirty = true;
        }
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

/// ファイルを読む関数 ([`read_asset`] の形。テストでは差し替えて、読む間に舞台を握っていないことを確かめる)。
pub type ReadAsset<'a> = dyn Fn(Asset, &Path, &[&str], u64) -> Result<Vec<u8>, SceneError> + 'a;

/// 始めた切り替え。舞台のロックの外で [`Pending::load`] で読み、[`Stage::finish`] に渡す (T0834)。
#[derive(Debug)]
pub struct Pending {
    /// 始めた順の番号。
    ticket: u64,
    id: String,
    /// 始めたときのシーンの設定。
    scene: SceneSettings,
    kind: Kind,
    /// 今のシーンで読んであるもの (同じパスは読み直さない)。
    cached: Cached,
}

#[derive(Debug)]
enum Kind {
    Activate,
    /// 出しているシーンの設定を置き換えた。`previous` は元の設定 (出せなければ戻す)。
    Update {
        previous: SceneSettings,
    },
    /// 出しているシーンの着せ替え。
    DressUp {
        slot: String,
        module: Option<PathBuf>,
    },
}

#[derive(Debug, Default)]
struct Cached {
    /// 指揮役に出しているモデル (指揮役がモデルを外していれば `None`)。
    model: Option<(PathBuf, Arc<Vec<u8>>)>,
    modules: Vec<LoadedModule>,
    background: Option<(PathBuf, Arc<Vec<u8>>)>,
}

/// 読み終えた切り替え。
#[derive(Debug)]
pub struct Loaded {
    ticket: u64,
    id: String,
    kind: Kind,
    /// 差し込むときにプロファイルが持っているはずのシーンの設定 (違えば、読んだものは古い)。
    expected: SceneSettings,
    result: Result<Prepared, SceneError>,
}

/// 指揮役に出すシーン。
#[derive(Debug)]
struct Prepared {
    /// 出す設定 (着せ替えなら書き換えた後のもの)。
    scene: SceneSettings,
    look: Look,
    model: Option<(PathBuf, Arc<Vec<u8>>)>,
    /// 読んだモデル (同じモデルなら `None`)。
    new_model: Option<NewModel>,
    modules: Vec<LoadedModule>,
    background: Option<(PathBuf, Arc<Vec<u8>>)>,
}

/// 差し込んだ結果。
#[derive(Debug)]
pub enum Finish {
    /// 出した (着せ替えで変わらなければ何もしない)。
    Done,
    /// 読んでいる間に後から切り替えが始まったので、読んだものを捨てた。
    Superseded,
    /// 読んでいる間にシーンの設定が変わったので、今の設定で読み直す。
    Again(Box<Pending>),
}

impl Pending {
    /// 読むファイルがあるか (無ければ舞台を握ったまま差し込んでよい)。
    pub fn needs_reading(&self) -> bool {
        let cached = &self.cached;
        let model = self
            .scene
            .model
            .as_ref()
            .is_some_and(|path| cached.model.as_ref().is_none_or(|(p, _)| p != path));
        let module = |path: &Path| !cached.modules.iter().any(|m| m.path == path);
        let modules = self.scene.modules.iter().any(|p| module(p))
            || matches!(&self.kind, Kind::DressUp { module: Some(path), .. } if module(path));
        let background = matches!(&self.scene.background, Background::Image { path }
            if cached.background.as_ref().is_none_or(|(p, _)| p != path));
        model || modules || background
    }

    /// ファイルを読む (舞台のロックの外で呼ぶ)。
    pub fn load(self) -> Loaded {
        self.load_with(&read_asset)
    }

    /// `read` でファイルを読む。
    pub fn load_with(self, read: &ReadAsset<'_>) -> Loaded {
        let Pending {
            ticket,
            id,
            scene,
            kind,
            cached,
        } = self;
        let mut loader = Loader {
            cached,
            read,
            fresh: Vec::new(),
        };
        let result = match &kind {
            Kind::DressUp { slot, module } => loader.dress_up(&scene, slot, module.as_deref()),
            _ => Ok(scene.clone()),
        }
        .and_then(|target| loader.prepare(target));
        Loaded {
            ticket,
            id,
            kind,
            expected: scene,
            result,
        }
    }
}

/// 切り替えのファイルを読む (読んであるものは使う)。
struct Loader<'a> {
    cached: Cached,
    read: &'a ReadAsset<'a>,
    /// この切り替えで読んだモジュール (着せ替えで読んだものを、もう一度読まない)。
    fresh: Vec<LoadedModule>,
}

impl Loader<'_> {
    /// シーンのファイルを読む。
    fn prepare(&mut self, scene: SceneSettings) -> Result<Prepared, SceneError> {
        let mut look = Look {
            lighting: scene.lighting.clone(),
            transform: scene.transform,
            background: scene.background.clone(),
        };
        let background = match &scene.background {
            Background::Image { path } => match self.background(path) {
                Ok(bytes) => Some((path.clone(), bytes)),
                Err(_) => {
                    look.background = Background::Transparent;
                    None
                }
            },
            _ => None,
        };
        let Some(model_path) = scene.model.clone() else {
            return Ok(Prepared {
                scene,
                look,
                model: None,
                new_model: None,
                modules: Vec::new(),
                background,
            });
        };
        let (model, new_model) = match &self.cached.model {
            Some((path, bytes)) if *path == model_path => {
                (Some((model_path.clone(), Arc::clone(bytes))), None)
            }
            _ => {
                let bytes =
                    (self.read)(Asset::Model, &model_path, MODEL_EXTENSIONS, MAX_MODEL_BYTES)?;
                let bundle = conductor::read_model(&bytes).map_err(|e| SceneError::Unreadable {
                    asset: Asset::Model,
                    path: model_path.clone(),
                    message: e.to_string(),
                })?;
                let name = file_name(&model_path);
                (
                    Some((model_path.clone(), Arc::new(bytes))),
                    Some(NewModel { name, bundle }),
                )
            }
        };
        let modules = scene
            .modules
            .iter()
            .map(|path| self.module(path))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Prepared {
            scene,
            look,
            model,
            new_model,
            modules,
            background,
        })
    }

    /// 着せ替えた後の設定: スロット `slot` のモジュールを外し、`module` があれば付ける
    /// (割り当てたスロットと違うスロットのモジュールなら、そのモジュールのスロットを空けてから付ける)。
    fn dress_up(
        &mut self,
        base: &SceneSettings,
        slot: &str,
        module: Option<&Path>,
    ) -> Result<SceneSettings, SceneError> {
        let mut modules = Vec::with_capacity(base.modules.len() + 1);
        for path in &base.modules {
            if self.module(path)?.module.slot.as_str() != slot {
                modules.push(path.clone());
            }
        }
        if let Some(path) = module {
            let loaded = self.module(path)?;
            if loaded.module.slot.as_str() != slot {
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
        let mut scene = base.clone();
        scene.modules = modules;
        scene.sanitize();
        Ok(scene)
    }

    /// モジュールを読む (今のシーンかこの切り替えで読んであれば、それを使う)。
    fn module(&mut self, path: &Path) -> Result<LoadedModule, SceneError> {
        if let Some(loaded) = self
            .cached
            .modules
            .iter()
            .chain(&self.fresh)
            .find(|m| m.path == path)
        {
            return Ok(loaded.clone());
        }
        let bytes = (self.read)(Asset::Module, path, MODULE_EXTENSIONS, MAX_MODEL_BYTES)?;
        let file = ugoe::format::read_module(&bytes).map_err(|e| SceneError::Unreadable {
            asset: Asset::Module,
            path: path.to_owned(),
            message: e.to_string(),
        })?;
        let loaded = LoadedModule {
            path: path.to_owned(),
            bytes: Arc::new(bytes),
            module: Arc::new(file.module),
        };
        self.fresh.push(loaded.clone());
        Ok(loaded)
    }

    fn background(&self, path: &Path) -> Result<Arc<Vec<u8>>, SceneError> {
        if let Some((loaded, bytes)) = &self.cached.background
            && loaded == path
        {
            return Ok(Arc::clone(bytes));
        }
        (self.read)(Asset::Background, path, IMAGE_EXTENSIONS, MAX_IMAGE_BYTES).map(Arc::new)
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// 拡張子と大きさを確かめてから読む。通常のファイルだけを読む。
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
    // 通常のファイルだけを読む (装置・FIFO・ディレクトリは読まない。`ugoe_fs::read_regular_file`)。
    read_regular_file(path, max_bytes).map_err(|e| match e {
        ReadError::TooLarge { len, .. } => SceneError::TooLarge {
            asset,
            path: path.to_owned(),
            bytes: len,
        },
        e => io(e.into()),
    })
}

#[cfg(test)]
mod tests {
    use ugoe::format::ModuleFile;
    use ugoe::model::fixtures;

    use super::*;
    use crate::profile::Transform;

    /// テスト用のファイルを置く場所 (テストごとの一時ディレクトリ。破棄時に消す)。
    struct Dir(tempfile::TempDir);

    impl Dir {
        fn new() -> Self {
            Self(tempfile::tempdir().unwrap())
        }

        fn model(&self, name: &str, model: ugoe::model::Model) -> PathBuf {
            let path = self.0.path().join(name);
            std::fs::write(&path, ugoe::format::model_to_json(&model).unwrap()).unwrap();
            path
        }

        fn module(&self, name: &str, module: Module) -> PathBuf {
            let path = self.0.path().join(name);
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

    /// 刻みの速さと仮想カメラの設定はプロファイルに持ち、変えたら未保存にする (T0827)。
    #[test]
    fn keeps_the_tick_rate_and_the_virtual_camera_in_the_profile() {
        let dir = Dir::new();
        let path = dir.0.path().join("p.json");
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();
        stage.set_tick_rate(TickRate::DEFAULT);
        assert!(!stage.dirty());
        stage.set_tick_rate(TickRate::Fps120);
        assert!(stage.dirty());
        stage.set_virtual_camera(VirtualCameraSettings {
            width: 1920,
            height: 1080,
            fps: 60,
            background: "#00FF00".into(),
        });
        let camera = stage.profile().output.virtual_camera.clone();
        assert_eq!(camera.background, "#00ff00");
        stage.save_profile(Some(&path)).unwrap();
        // 選べない値は既定に戻す。
        stage.set_virtual_camera(VirtualCameraSettings {
            fps: 24,
            ..camera.clone()
        });
        assert_eq!(stage.profile().output.virtual_camera.fps, 30);
        assert!(stage.dirty());

        stage.new_profile(&mut conductor);
        assert_eq!(stage.profile().output.tick_rate(), TickRate::DEFAULT);
        stage.open_profile(&path, &mut conductor).unwrap();
        assert_eq!(stage.profile().output.tick_rate(), TickRate::Fps120);
        assert_eq!(stage.profile().output.virtual_camera, camera);
    }

    #[test]
    fn switches_scenes_and_keeps_the_model_when_it_is_the_same() {
        let dir = Dir::new();
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
        let dir = Dir::new();
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
        let error = stage.dress_up(
            &slot,
            Some(&dir.0.path().join("gone.ugoemod")),
            &mut conductor,
        );
        assert!(matches!(error, Err(SceneError::Missing { .. })));
        assert_eq!(modules(&stage), [dress]);
    }

    #[test]
    fn a_missing_model_leaves_the_live_scene_alone() {
        let dir = Dir::new();
        let base = dir.model("base.json", fixtures::dress_up_base());
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();
        let good = scene(&mut stage, Some(&base), &[]);
        let gone = scene(&mut stage, Some(&dir.0.path().join("gone.ugoe")), &[]);
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
        broken.modules = vec![dir.0.path().join("nope.ugoemod")];
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

    /// 読んでいる間に後から始めた切り替えがあれば、先に始めた方は読み終えた順によらず捨てる (T0834)。
    #[test]
    fn a_later_switch_discards_the_earlier_one() {
        let dir = Dir::new();
        let base = dir.model("base.json", fixtures::dress_up_base());
        let lone = dir.model("lone.json", fixtures::single_quad());
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();
        let a = scene(&mut stage, Some(&base), &[]);
        let b = scene(&mut stage, Some(&lone), &[]);
        stage.update_scene(a.clone(), &mut conductor).unwrap();
        stage.update_scene(b.clone(), &mut conductor).unwrap();

        // 読み終えた順: 先に始めた方が先・後の方が先。
        for earlier_first in [true, false] {
            let empty = stage.profile().scenes[0].id.clone();
            stage.activate(&empty, &mut conductor).unwrap();
            let revision = conductor.scene().unwrap().revision;
            let earlier = stage.begin_activate(&a.id, &conductor).unwrap();
            let later = stage.begin_activate(&b.id, &conductor).unwrap();
            assert!(earlier.needs_reading() && later.needs_reading());
            let (earlier, later) = (earlier.load(), later.load());
            if earlier_first {
                assert!(matches!(
                    stage.finish(earlier, &mut conductor),
                    Ok(Finish::Superseded)
                ));
                assert_eq!(conductor.scene().unwrap().revision, revision);
                assert!(matches!(
                    stage.finish(later, &mut conductor),
                    Ok(Finish::Done)
                ));
            } else {
                assert!(matches!(
                    stage.finish(later, &mut conductor),
                    Ok(Finish::Done)
                ));
                assert!(matches!(
                    stage.finish(earlier, &mut conductor),
                    Ok(Finish::Superseded)
                ));
            }
            assert_eq!(stage.live(), Some(b.id.as_str()));
            assert_eq!(conductor.scene().unwrap().model_name, "lone.json");
        }

        // 新しいプロファイルにしたら、読んでいる途中の切り替えは捨てる。
        let pending = stage.begin_activate(&a.id, &conductor).unwrap().load();
        stage.new_profile(&mut conductor);
        assert!(matches!(
            stage.finish(pending, &mut conductor),
            Ok(Finish::Superseded)
        ));
        assert_eq!(stage.live(), None);
        assert_eq!(conductor.scene().unwrap().model_revision, 0);
    }

    /// 読んでいる間にシーンの設定が変わったら、切り替えは今の設定で読み直す。出しているシーンの編集は、後の切り替えに捨てられても設定は残す (T0834)。
    #[test]
    fn a_scene_edited_while_loading_is_read_again() {
        let dir = Dir::new();
        let base = dir.model("base.json", fixtures::dress_up_base());
        let lone = dir.model("lone.json", fixtures::single_quad());
        let hair = dir.module("hair.ugoemod", fixtures::long_hair_module());
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();
        let mut a = scene(&mut stage, Some(&base), &[]);
        stage.update_scene(a.clone(), &mut conductor).unwrap();

        let pending = stage.begin_activate(&a.id, &conductor).unwrap().load();
        a.model = Some(lone.clone());
        stage.update_scene(a.clone(), &mut conductor).unwrap();
        let Ok(Finish::Again(again)) = stage.finish(pending, &mut conductor) else {
            panic!("expected to read the scene again");
        };
        assert!(matches!(
            stage.finish(again.load(), &mut conductor),
            Ok(Finish::Done)
        ));
        assert_eq!(conductor.scene().unwrap().model_name, "lone.json");

        // 出しているシーンの編集を読んでいる間に別のシーンへ切り替えた: 出すのは後のシーン、編集した設定は残す。
        a.model = Some(base.clone());
        a.modules = vec![hair.clone()];
        let b = stage.profile().scenes[0].id.clone();
        let edit = stage
            .begin_update_scene(a.clone(), &conductor)
            .unwrap()
            .unwrap()
            .load();
        stage.activate(&b, &mut conductor).unwrap();
        assert!(matches!(
            stage.finish(edit, &mut conductor),
            Ok(Finish::Superseded)
        ));
        assert_eq!(stage.live(), Some(b.as_str()));
        assert_eq!(stage.profile().scene(&a.id).unwrap().modules, [hair]);
    }

    #[test]
    fn checks_file_types() {
        let dir = Dir::new();
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();
        let text = dir.0.path().join("notes.txt");
        std::fs::write(&text, "hi").unwrap();
        let s = scene(&mut stage, Some(&text), &[]);
        stage.update_scene(s.clone(), &mut conductor).unwrap();
        assert!(matches!(
            stage.activate(&s.id, &mut conductor),
            Err(SceneError::WrongType { .. })
        ));
        let not_model = dir.0.path().join("x.json");
        std::fs::write(&not_model, "{}").unwrap();
        let s2 = scene(&mut stage, Some(&not_model), &[]);
        stage.update_scene(s2.clone(), &mut conductor).unwrap();
        assert!(matches!(
            stage.activate(&s2.id, &mut conductor),
            Err(SceneError::Unreadable { .. })
        ));
    }

    /// 名前がモデル・画像でも、通常のファイルでなければ (ディレクトリ・装置・FIFO) 読まない (T0011)。
    #[test]
    fn reads_only_regular_files() {
        let dir = Dir::new();
        let folder = dir.0.path().join("model.ugoe");
        std::fs::create_dir_all(&folder).unwrap();
        assert!(matches!(
            read_asset(Asset::Model, &folder, MODEL_EXTENSIONS, MAX_MODEL_BYTES),
            Err(SceneError::Unreadable { .. })
        ));
        let image = dir.0.path().join("bg.png");
        std::fs::write(&image, b"12345").unwrap();
        assert!(matches!(
            read_asset(Asset::Background, &image, IMAGE_EXTENSIONS, 4),
            Err(SceneError::TooLarge { bytes: 5, .. })
        ));
        assert_eq!(
            read_asset(Asset::Background, &image, IMAGE_EXTENSIONS, 5).unwrap(),
            b"12345"
        );
        #[cfg(unix)]
        {
            let zero = std::path::Path::new("/dev/zero");
            // 拡張子の検査を通すため、`/dev/zero` へのリンクを `.png` の名前で置く。
            let link = dir.0.path().join("zero.png");
            std::os::unix::fs::symlink(zero, &link).unwrap();
            assert!(matches!(
                read_asset(Asset::Background, &link, IMAGE_EXTENSIONS, MAX_IMAGE_BYTES),
                Err(SceneError::Unreadable { .. })
            ));
        }
    }

    #[test]
    fn a_missing_background_falls_back_to_transparent() {
        let dir = Dir::new();
        let base = dir.model("base.json", fixtures::dress_up_base());
        let image = dir.0.path().join("bg.png");
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
            path: dir.0.path().join("gone.png"),
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
        let dir = Dir::new();
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
        let dir = Dir::new();
        let base = dir.model("base.json", fixtures::dress_up_base());
        let recent = dir.0.path().join("data/recent-profiles.json");
        let mut stage = Stage::new(Some(recent.clone()));
        let mut conductor = Conductor::new();
        let s = scene(&mut stage, Some(&base), &[]);
        stage.update_scene(s.clone(), &mut conductor).unwrap();
        stage.activate(&s.id, &mut conductor).unwrap();
        // 保存先が無ければ何もしない。
        assert_eq!(stage.save_profile(None).unwrap(), None);
        let path = dir.0.path().join("profile.json");
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
        let future = dir.0.path().join("future.json");
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
        let dir = Dir::new();
        let mut profile = Profile::new();
        profile.scenes[0].model = Some(dir.0.path().join("gone.ugoe"));
        let path = dir.0.path().join("p.json");
        profile.write(&path).unwrap();
        let mut stage = Stage::new(None);
        let mut conductor = Conductor::new();
        let error = stage.open_profile(&path, &mut conductor).unwrap();
        assert!(matches!(error, Some(SceneError::Missing { .. })));
        assert_eq!(stage.path(), Some(path.as_path()));
        assert_eq!(stage.live(), None);
    }
}
