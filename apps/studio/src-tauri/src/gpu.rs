//! 使う GPU の設定 (T0860。docs/studio.md#使う-gpu)。
//!
//! 「自動」(希望なし)・「省電力」・「高性能」の 3 つ。GPU はその機械のものなので、プロファイルには入れず、機械ごとの設定ファイル
//! ([`SETTINGS_FILE`]。アプリの設定の場所) に持つ。効くところは受け手ごとに違う:
//!
//! - 出力ウィンドウ: 起動したときの値 (`launched`) で WebView の `powerPreference` を渡し、Windows の「高性能」では WebView2 を
//!   `--force-high-performance-gpu` で起動する ([`browser_args`])。WebView2 の引数は起動のときにしか渡せないので、変えたら次の起動から。
//! - 仮想カメラ: 始めるときの値で GPU を作る (`virtual_camera.rs` の `GpuCell`)。変えたら次に始めるときから。
//! - ブラウザソース: OBS が決める (スタジオからは選べない)。
//!
//! 同じファイルに、Windows の GPU の優先度 ([`GpuPriority`]。T0861、docs/studio.md#gpu-の優先度) も持つ。当てる先と当てられなかった
//! 理由は [`crate::gpu_priority`]、OS の呼び出しは `ugoe-studio-gpu-priority` (ADR 0034)。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ugoe_headless::PowerPreference;

use crate::error::CommandError;
use crate::gpu_priority::{ApplyPriority, PriorityState};

/// 設定のファイルの名前 (アプリの設定の場所に置く)。
pub const SETTINGS_FILE: &str = "studio-settings.json";

/// 設定のファイルの `format`。
const SETTINGS_FORMAT: &str = "ugoe-studio-settings";

/// 設定のファイルの版。
const SETTINGS_VERSION: u32 = 1;

/// 設定のファイルの大きさの上限 (手で書き換えた巨大なファイルを読まない)。
const MAX_SETTINGS_BYTES: u64 = 64 * 1024;

/// 両方のウィンドウの WebView2 に渡す引数 (隠れても止まらないように。docs/studio.md#隠れても止まらない)。
/// wry の既定の `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection` は、上書きすると消えるので書き写す。
pub const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,CalculateNativeWinOcclusion,IntensiveWakeUpThrottling --disable-background-timer-throttling --disable-backgrounding-occluded-windows --disable-renderer-backgrounding";

/// 「高性能」のときに WebView2 へ足す引数 (Chromium は `powerPreference` を見ないので、ブラウザ全体で外付けの GPU を使わせる)。
pub const HIGH_PERFORMANCE_ARG: &str = "--force-high-performance-gpu";

/// Windows の「グラフィックの設定」(アプリごとの GPU) を開く URI。
#[cfg(windows)]
pub const GRAPHICS_SETTINGS_URI: &str = "ms-settings:display-advancedgraphics";

/// 使う GPU の希望。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GpuPreference {
    /// 希望なし (ブラウザと wgpu に任せる。仮想カメラは環境変数 `WGPU_POWER_PREF` に従う)。
    #[default]
    Auto,
    /// 省電力 (内蔵の GPU)。
    LowPower,
    /// 高性能 (外付けの GPU)。
    HighPerformance,
}

impl GpuPreference {
    /// 仮想カメラの GPU を作るときの希望 (`ugoe_headless::Gpu::with_power_preference`)。
    pub fn native(self) -> Option<PowerPreference> {
        match self {
            Self::Auto => None,
            Self::LowPower => Some(PowerPreference::LowPower),
            Self::HighPerformance => Some(PowerPreference::HighPerformance),
        }
    }

    /// 出力ウィンドウの `WebPuppet.create` に渡す希望 (`@ugoe/player` の `PowerPreference`)。
    pub fn web(self) -> &'static str {
        match self {
            Self::Auto => "default",
            Self::LowPower => "low-power",
            Self::HighPerformance => "high-performance",
        }
    }
}

/// GPU の優先度 (Windows だけ。T0861)。既定は「通常」(プロセスを作ったときのまま。OS を呼ばない)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GpuPriority {
    /// 通常。
    #[default]
    Normal,
    /// 低い (ゲームや配信ソフトが GPU を使い切っている間は、スタジオの描画が後回しになる)。
    Low,
}

impl GpuPriority {
    /// OS に渡す優先度 (`ugoe-studio-gpu-priority`)。
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn native(self) -> ugoe_studio_gpu_priority::GpuPriority {
        match self {
            Self::Normal => ugoe_studio_gpu_priority::GpuPriority::Normal,
            Self::Low => ugoe_studio_gpu_priority::GpuPriority::BelowNormal,
        }
    }
}

/// 両方のウィンドウの WebView2 に渡す引数。2 つのウィンドウで同じでないと WebView2 を作れないので、どちらもこれを使う。
pub fn browser_args(preference: GpuPreference) -> String {
    match preference {
        GpuPreference::HighPerformance => format!("{BROWSER_ARGS} {HIGH_PERFORMANCE_ARG}"),
        GpuPreference::Auto | GpuPreference::LowPower => BROWSER_ARGS.to_owned(),
    }
}

/// 設定のファイルの形。
#[derive(Debug, Serialize, Deserialize)]
struct SettingsFile {
    format: String,
    version: u32,
    #[serde(default)]
    gpu: GpuPreference,
    /// GPU の優先度 (T0861)。
    #[serde(default)]
    priority: GpuPriority,
}

/// 設定のファイルに覚えたもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Saved {
    pub gpu: GpuPreference,
    pub priority: GpuPriority,
}

/// 設定のファイルを読む。無い・読めない・知らない値なら既定 (「自動」と「通常」。読めなかったことは標準エラーに書く)。
pub fn read(path: &Path) -> Saved {
    match ugoe_fs::read_regular_file(path, MAX_SETTINGS_BYTES) {
        Ok(bytes) => match serde_json::from_slice::<SettingsFile>(&bytes) {
            Ok(file) if file.format == SETTINGS_FORMAT => Saved {
                gpu: file.gpu,
                priority: file.priority,
            },
            Ok(_) => {
                eprintln!("{}: not a studio settings file", path.display());
                Saved::default()
            }
            Err(e) => {
                eprintln!("{}: {e}", path.display());
                Saved::default()
            }
        },
        Err(ugoe_fs::ReadError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            Saved::default()
        }
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            Saved::default()
        }
    }
}

/// 設定のファイルに書く (一時ファイルに書いてから置き換える)。
pub fn write(path: &Path, saved: Saved) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = SettingsFile {
        format: SETTINGS_FORMAT.to_owned(),
        version: SETTINGS_VERSION,
        gpu: saved.gpu,
        priority: saved.priority,
    };
    let json = serde_json::to_string_pretty(&file).map_err(std::io::Error::other)?;
    ugoe_fs::write_atomic(path, json.as_bytes())
}

/// 使う GPU の設定 (`Studio` が持つ)。
#[derive(Debug, Default)]
pub struct GpuSettings {
    /// 設定のファイル (`None` なら覚えない)。
    path: Option<PathBuf>,
    /// 今の希望 (仮想カメラは次に始めるときにこれを使う)。
    preference: GpuPreference,
    /// 起動したときの希望 (出力ウィンドウはこれで動いている)。
    launched: GpuPreference,
    /// 選んでいる GPU の優先度 (T0861)。
    priority: GpuPriority,
    /// 優先度を当てた様子 (Windows だけ。ほかの OS では使わない)。
    applied: PriorityState,
}

/// 操作ウィンドウに出す、使う GPU の様子。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuView {
    /// 今の希望。
    pub preference: GpuPreference,
    /// 起動したときの希望 (出力ウィンドウが使っている。今の希望と違えば、起動し直すと効く)。
    pub launched: GpuPreference,
    /// OS の「グラフィックの設定」を開けるか (Windows だけ。`open_graphics_settings`)。
    pub graphics_settings: bool,
    /// 選んでいる GPU の優先度 (T0861)。
    pub priority: GpuPriority,
    /// GPU の優先度を選べるか (Windows だけ)。
    pub priority_supported: bool,
    /// GPU の優先度を当てられなかった理由 (スタジオのプロセス・WebView2 の GPU のプロセスの順。当てられていれば空)。
    pub priority_errors: Vec<CommandError>,
}

impl GpuSettings {
    /// 起動したときに読む。`path` が `None` なら覚えない (「自動」)。
    pub fn load(path: Option<PathBuf>) -> Self {
        let saved = path.as_deref().map(read).unwrap_or_default();
        Self {
            path,
            preference: saved.gpu,
            launched: saved.gpu,
            priority: saved.priority,
            applied: PriorityState::default(),
        }
    }

    pub fn preference(&self) -> GpuPreference {
        self.preference
    }

    pub fn launched(&self) -> GpuPreference {
        self.launched
    }

    /// 希望を変えてファイルに書く。書けなければ変えない。
    pub fn set(&mut self, preference: GpuPreference) -> std::io::Result<()> {
        self.save(Saved {
            gpu: preference,
            priority: self.priority,
        })?;
        self.preference = preference;
        Ok(())
    }

    /// GPU の優先度を変えてファイルに書き、当てる。書けなければ変えない (当てない)。
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn set_priority(
        &mut self,
        priority: GpuPriority,
        apply: &mut impl ApplyPriority,
    ) -> std::io::Result<()> {
        self.save(Saved {
            gpu: self.preference,
            priority,
        })?;
        self.priority = priority;
        self.applied.set(priority.native(), apply);
        Ok(())
    }

    /// 起動したときに、覚えた優先度をスタジオのプロセスに当てる (「通常」なら何もしない)。
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn apply_priority(&mut self, apply: &mut impl ApplyPriority) {
        self.applied.set(self.priority.native(), apply);
    }

    /// WebView2 の GPU のプロセスが分かった・変わったので、今の優先度を当てる。
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn webview_changed(
        &mut self,
        processes: Result<Vec<u32>, ugoe_studio_gpu_priority::Error>,
        apply: &mut impl ApplyPriority,
    ) {
        self.applied.webview_changed(processes, apply);
    }

    fn save(&self, saved: Saved) -> std::io::Result<()> {
        match &self.path {
            Some(path) => write(path, saved),
            None => Ok(()),
        }
    }

    pub fn view(&self) -> GpuView {
        GpuView {
            preference: self.preference,
            launched: self.launched,
            graphics_settings: cfg!(windows),
            priority: self.priority,
            priority_supported: cfg!(windows),
            priority_errors: self.applied.errors(),
        }
    }
}

/// Windows の「グラフィックの設定」を開く (仮想カメラの GPU を Windows の側で選ぶ手順のため。OS の設定は書き換えない)。
#[cfg(windows)]
pub fn open_graphics_settings() -> std::io::Result<()> {
    // explorer.exe は ms-settings: の URI を設定のアプリで開く (終わりは待たない)。
    std::process::Command::new("explorer.exe")
        .arg(GRAPHICS_SETTINGS_URI)
        .spawn()
        .map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu_priority::Target;

    fn temp_dir() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config").join(SETTINGS_FILE);
        (dir, path)
    }

    #[test]
    fn preferences_map_to_the_native_and_web_values() {
        assert_eq!(GpuPreference::Auto.native(), None);
        assert_eq!(
            GpuPreference::LowPower.native(),
            Some(PowerPreference::LowPower)
        );
        assert_eq!(
            GpuPreference::HighPerformance.native(),
            Some(PowerPreference::HighPerformance)
        );
        assert_eq!(GpuPreference::Auto.web(), "default");
        assert_eq!(GpuPreference::LowPower.web(), "low-power");
        assert_eq!(GpuPreference::HighPerformance.web(), "high-performance");
        assert_eq!(
            serde_json::to_string(&GpuPreference::HighPerformance).unwrap(),
            r#""highPerformance""#
        );
    }

    #[test]
    fn only_high_performance_forces_the_discrete_gpu_in_webview2() {
        assert_eq!(browser_args(GpuPreference::Auto), BROWSER_ARGS);
        // 省電力は WebView2 の既定 (ふつうは内蔵の GPU) に任せる
        assert_eq!(browser_args(GpuPreference::LowPower), BROWSER_ARGS);
        let high = browser_args(GpuPreference::HighPerformance);
        assert!(high.starts_with(BROWSER_ARGS), "{high}");
        assert!(high.ends_with(" --force-high-performance-gpu"), "{high}");
    }

    #[test]
    fn settings_are_saved_and_read_back() {
        let (_dir, path) = temp_dir();
        // 無ければ自動
        let mut settings = GpuSettings::load(Some(path.clone()));
        assert_eq!(settings.preference(), GpuPreference::Auto);
        settings.set(GpuPreference::HighPerformance).unwrap();
        // 起動したときの値は変わらない (出力ウィンドウは次の起動から)
        assert_eq!(
            settings.view(),
            GpuView {
                preference: GpuPreference::HighPerformance,
                launched: GpuPreference::Auto,
                graphics_settings: cfg!(windows),
                priority: GpuPriority::Normal,
                priority_supported: cfg!(windows),
                priority_errors: Vec::new(),
            }
        );
        let reloaded = GpuSettings::load(Some(path.clone()));
        assert_eq!(reloaded.preference(), GpuPreference::HighPerformance);
        assert_eq!(reloaded.launched(), GpuPreference::HighPerformance);
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "format": "ugoe-studio-settings",
                "version": 1,
                "gpu": "highPerformance",
                "priority": "normal",
            })
        );
    }

    #[test]
    fn unreadable_settings_are_auto() {
        let (_dir, path) = temp_dir();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        for text in [
            "not json",
            r#"{"format":"something-else","version":1,"gpu":"lowPower"}"#,
            r#"{"format":"ugoe-studio-settings","version":1,"gpu":"quantum"}"#,
        ] {
            std::fs::write(&path, text).unwrap();
            assert_eq!(read(&path), Saved::default(), "{text}");
        }
        std::fs::write(&path, r#"{"format":"ugoe-studio-settings","version":1}"#).unwrap();
        assert_eq!(read(&path), Saved::default());
        std::fs::write(
            &path,
            r#"{"format":"ugoe-studio-settings","version":1,"gpu":"lowPower"}"#,
        )
        .unwrap();
        assert_eq!(
            read(&path),
            Saved {
                gpu: GpuPreference::LowPower,
                priority: GpuPriority::Normal,
            }
        );
        std::fs::write(
            &path,
            r#"{"format":"ugoe-studio-settings","version":1,"priority":"realtime"}"#,
        )
        .unwrap();
        assert_eq!(read(&path), Saved::default());
    }

    /// 優先度を当てる偽の口 (当てた先と優先度を記録する)。
    fn recorder(
        calls: &mut Vec<(Target, ugoe_studio_gpu_priority::GpuPriority)>,
    ) -> impl ApplyPriority + '_ {
        |target, priority| {
            calls.push((target, priority));
            Ok(())
        }
    }

    #[test]
    fn the_priority_is_saved_with_the_preference_and_applied() {
        use ugoe_studio_gpu_priority::GpuPriority as Native;
        let (_dir, path) = temp_dir();
        let mut calls = Vec::new();
        let mut settings = GpuSettings::load(Some(path.clone()));
        // 既定は「通常」で、起動したときに OS を呼ばない
        settings.apply_priority(&mut recorder(&mut calls));
        assert_eq!(settings.view().priority, GpuPriority::Normal);
        assert!(calls.is_empty());
        settings.set(GpuPreference::LowPower).unwrap();
        settings
            .set_priority(GpuPriority::Low, &mut recorder(&mut calls))
            .unwrap();
        assert_eq!(calls, [(Target::Studio, Native::BelowNormal)]);
        settings.webview_changed(Ok(vec![42]), &mut recorder(&mut calls));
        assert_eq!(calls[1..], [(Target::WebView(42), Native::BelowNormal)]);
        let view = settings.view();
        assert_eq!(view.priority, GpuPriority::Low);
        assert!(view.priority_errors.is_empty());
        // 希望を変えても優先度は残る
        settings.set(GpuPreference::HighPerformance).unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(json["gpu"], "highPerformance");
        assert_eq!(json["priority"], "low");
        // 起動し直すと、覚えた優先度を当てる
        let mut calls = Vec::new();
        let mut reloaded = GpuSettings::load(Some(path));
        assert_eq!(reloaded.view().priority, GpuPriority::Low);
        reloaded.apply_priority(&mut recorder(&mut calls));
        assert_eq!(calls, [(Target::Studio, Native::BelowNormal)]);
    }

    #[test]
    fn a_failed_save_does_not_apply_the_priority() {
        let (dir, _) = temp_dir();
        let blocker = dir.path().join("file");
        std::fs::write(&blocker, "").unwrap();
        let mut settings = GpuSettings::load(Some(blocker.join(SETTINGS_FILE)));
        let mut calls = Vec::new();
        assert!(
            settings
                .set_priority(GpuPriority::Low, &mut recorder(&mut calls))
                .is_err()
        );
        assert_eq!(settings.view().priority, GpuPriority::Normal);
        assert!(calls.is_empty());
    }

    #[test]
    fn view_reports_priority_failures() {
        let mut settings = GpuSettings::load(None);
        let mut deny = |_: Target, _: ugoe_studio_gpu_priority::GpuPriority| {
            Err(ugoe_studio_gpu_priority::Error::AccessDenied)
        };
        settings.set_priority(GpuPriority::Low, &mut deny).unwrap();
        let json = serde_json::to_value(settings.view()).unwrap();
        assert_eq!(json["priority"], "low");
        assert_eq!(
            json["priorityErrors"][0]["code"],
            "gpu_priority.access_denied"
        );
        assert_eq!(json["priorityErrors"][0]["args"]["process"], "ugoe-studio");
    }

    #[test]
    fn without_a_path_nothing_is_saved() {
        let mut settings = GpuSettings::load(None);
        settings.set(GpuPreference::LowPower).unwrap();
        assert_eq!(settings.preference(), GpuPreference::LowPower);
        assert_eq!(settings.launched(), GpuPreference::Auto);
    }

    #[test]
    fn a_failed_save_keeps_the_preference() {
        let (dir, _) = temp_dir();
        // 親がファイルなので書けない
        let blocker = dir.path().join("file");
        std::fs::write(&blocker, "").unwrap();
        let mut settings = GpuSettings::load(Some(blocker.join(SETTINGS_FILE)));
        assert!(settings.set(GpuPreference::HighPerformance).is_err());
        assert_eq!(settings.preference(), GpuPreference::Auto);
    }
}
