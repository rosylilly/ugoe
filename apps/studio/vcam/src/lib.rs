//! スタジオの仮想カメラ (ADR 0028 の 5.・8.、`docs/studio.md#仮想カメラ`)。
//!
//! スタジオのバックエンドは [`VirtualCamera`] だけを使う。OS ごとの受け渡しは中に隠し、外へは safe な API だけを出す。
//!
//! - **Windows 11:** Media Foundation の仮想カメラ (`MFCreateVirtualCamera`)。OS のサービス (Frame Server) がこのクレートの
//!   DLL (メディアソース。インストール時に管理者の権限で登録する) を読み込み、スタジオは共有メモリ ([`shared`]) で NV12 の画を渡す。
//! - **Linux:** v4l2loopback の機器に YUYV (YUY2) の画を書く。機器はスタジオからは作らない (入れ方を案内する)。
//! - **macOS:** T0809。いまは [`Error::Unsupported`]。
//!
//! **`unsafe` の例外 (ADR 0028 の 8.):** ワークスペースで `unsafe` を許すのは `ugoe-ffi` とこのクレートだけ。`unsafe` は
//! COM と OS の境界 (インターフェースの実装・ポインタの受け渡し・共有メモリ・ioctl) に閉じ込め、どのブロックにも
//! 成り立たせている前提を `// SAFETY:` で書く (clippy の `undocumented_unsafe_blocks` で検査する)。

#[cfg(windows)]
mod mf;
pub mod shared;
#[cfg(target_os = "linux")]
mod v4l2;

/// 選べる解像度 (`docs/studio.md#仮想カメラ`)。
pub const RESOLUTIONS: [(u32, u32); 2] = [(1920, 1080), (1280, 720)];
/// 選べるフレームレート。
pub const FRAME_RATES: [u32; 2] = [30, 60];

/// 仮想カメラの設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            fps: 30,
        }
    }
}

impl Config {
    /// 選べる解像度とフレームレートか。
    pub fn validate(&self) -> Result<(), Error> {
        if RESOLUTIONS.contains(&(self.width, self.height)) && FRAME_RATES.contains(&self.fps) {
            Ok(())
        } else {
            Err(Error::InvalidConfig(*self))
        }
    }
}

/// OS の部品が受け取る画素の形。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelLayout {
    /// Y の面の後に Cb Cr を交互に並べた面 (Windows)。
    Nv12,
    /// 横の 2 画素ごとに Y0 Cb Y1 Cr (Linux の YUYV)。
    Yuy2,
}

/// この OS の仮想カメラが受け取る画素の形。
pub fn pixel_layout() -> PixelLayout {
    if cfg!(windows) {
        PixelLayout::Nv12
    } else {
        PixelLayout::Yuy2
    }
}

/// 仮想カメラを使えない・止まった理由。操作ウィンドウは [`Error::code`] で文言を選ぶ (ADR 0029 の 4.)。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// この OS ではまだ出せない (macOS は T0809)。
    #[error("virtual cameras are not supported on this OS yet")]
    Unsupported,
    /// Windows 11 より前の Windows (Media Foundation の仮想カメラが無い)。
    #[error("the Media Foundation virtual camera needs Windows 11")]
    WindowsTooOld,
    /// メディアソースの DLL が登録されていない (インストーラで入れ直す)。
    #[error("the virtual camera driver is not registered; reinstall the studio")]
    NotRegistered,
    /// v4l2loopback の機器が無い (入れ方を案内する)。
    #[error("no v4l2loopback device was found")]
    NoDevice,
    /// v4l2loopback の機器はあるが、どれもほかのアプリが書いている。
    #[error("every v4l2loopback device is in use by another program")]
    Busy,
    /// 選べない解像度・フレームレート。
    #[error("unsupported virtual camera settings: {}x{} at {} fps", .0.width, .0.height, .0.fps)]
    InvalidConfig(Config),
    /// 画の大きさが設定と合わない (呼び出し側の誤り)。
    #[error("the frame has {actual} bytes but {expected} were expected")]
    FrameSize { expected: usize, actual: usize },
    /// OS の呼び出しの失敗。
    #[error("{context}: {detail}")]
    Os {
        context: &'static str,
        detail: String,
    },
}

impl Error {
    /// 操作ウィンドウが文言を選ぶための短い名前 (`apps/studio/locales/*.json` の `virtualCamera.errors.*`)。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::WindowsTooOld => "windowsTooOld",
            Self::NotRegistered => "notRegistered",
            Self::NoDevice => "noDevice",
            Self::Busy => "busy",
            Self::InvalidConfig(_) => "invalidConfig",
            Self::FrameSize { .. } | Self::Os { .. } => "os",
        }
    }

    #[cfg(any(target_os = "linux", windows))]
    pub(crate) fn os(context: &'static str, detail: impl std::fmt::Display) -> Self {
        Self::Os {
            context,
            detail: detail.to_string(),
        }
    }
}

/// 1 枚を渡した結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// OS の部品に渡した。受け取るアプリがいるかは分からない (Linux)、またはいる (Windows)。
    Delivered,
    /// 渡したが、受け取るアプリがいない (Windows でカメラを開いているアプリが無い)。
    NoConsumer,
}

/// 動いている仮想カメラ。落とすと止まる (Windows ではカメラの機器が消える)。
#[derive(Debug)]
pub struct VirtualCamera {
    config: Config,
    #[cfg(target_os = "linux")]
    inner: v4l2::Loopback,
    #[cfg(windows)]
    inner: mf::Camera,
    #[cfg(not(any(target_os = "linux", windows)))]
    inner: Never,
}

/// 仮想カメラの無い OS の [`VirtualCamera`] の中身 (作れないので、値は存在しない)。
#[cfg(not(any(target_os = "linux", windows)))]
#[derive(Debug)]
enum Never {}

#[cfg(not(any(target_os = "linux", windows)))]
impl Never {
    fn device(&self) -> String {
        match *self {}
    }

    fn send(&mut self, _frame: &[u8]) -> Result<Delivery, Error> {
        match *self {}
    }
}

impl VirtualCamera {
    /// 仮想カメラを始める。使えない環境では理由を返す。
    pub fn start(config: Config) -> Result<Self, Error> {
        config.validate()?;
        #[cfg(target_os = "linux")]
        return Ok(Self {
            config,
            inner: v4l2::Loopback::open(config)?,
        });
        #[cfg(windows)]
        return Ok(Self {
            config,
            inner: mf::Camera::start(config)?,
        });
        #[cfg(not(any(target_os = "linux", windows)))]
        Err(Error::Unsupported)
    }

    pub fn config(&self) -> Config {
        self.config
    }

    /// 画を書いている先 (Linux は `/dev/videoN`、Windows はカメラの名前)。
    pub fn device(&self) -> String {
        self.inner.device()
    }

    /// 1 枚を渡す。画は [`pixel_layout`] の形で、設定の大きさ。
    pub fn send(&mut self, frame: &[u8]) -> Result<Delivery, Error> {
        let Config { width, height, .. } = self.config;
        let pixels = width as usize * height as usize;
        let expected = match pixel_layout() {
            PixelLayout::Nv12 => pixels * 3 / 2,
            PixelLayout::Yuy2 => pixels * 2,
        };
        if frame.len() != expected {
            return Err(Error::FrameSize {
                expected,
                actual: frame.len(),
            });
        }
        self.inner.send(frame)
    }
}

/// 仮想カメラを始められるか (始める前に操作ウィンドウに理由を出すため)。機器は開かない。
pub fn availability() -> Result<(), Error> {
    #[cfg(target_os = "linux")]
    return v4l2::availability();
    #[cfg(windows)]
    return mf::availability();
    #[cfg(not(any(target_os = "linux", windows)))]
    Err(Error::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_presets_are_accepted() {
        assert!(Config::default().validate().is_ok());
        assert!(
            Config {
                width: 1920,
                height: 1080,
                fps: 60
            }
            .validate()
            .is_ok()
        );
        let bad = Config {
            width: 640,
            height: 480,
            fps: 30,
        };
        assert_eq!(bad.validate().unwrap_err().code(), "invalidConfig");
        assert!(
            Config {
                fps: 120,
                ..Config::default()
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn layout_follows_the_os() {
        let expected = if cfg!(windows) {
            PixelLayout::Nv12
        } else {
            PixelLayout::Yuy2
        };
        assert_eq!(pixel_layout(), expected);
    }
}
