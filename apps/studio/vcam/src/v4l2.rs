//! Linux: v4l2loopback の機器に YUYV の画を書く (ADR 0028 の 8.)。
//!
//! スタジオは v4l2loopback を入れない (カーネルモジュールで、入れるには管理者の権限が要る)。機器が無ければ
//! [`Error::NoDevice`] を返し、操作ウィンドウが入れ方を案内する (`docs/studio.md#linux-v4l2loopback`)。
//!
//! 書き方: `/dev/video*` のうちドライバが `v4l2 loopback` で、出力 (書き込み) を受ける機器を開き、
//! `VIDIOC_S_FMT` で YUYV と大きさを決め、`VIDIOC_S_PARM` でフレームレートを伝え、1 枚ずつ `write` する。
//! `unsafe` は ioctl の呼び出しだけ (構造体は `linux/videodev2.h` の形を写し、大きさをテストで確かめる)。

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

use crate::{Config, Delivery, Error};

/// v4l2loopback のドライバの名前 (`VIDIOC_QUERYCAP` の `driver`)。
const LOOPBACK_DRIVER: &[u8] = b"v4l2 loopback";

const V4L2_CAP_VIDEO_OUTPUT: u32 = 0x0000_0002;
const V4L2_CAP_DEVICE_CAPS: u32 = 0x8000_0000;
const V4L2_BUF_TYPE_VIDEO_OUTPUT: u32 = 2;
const V4L2_FIELD_NONE: u32 = 1;
const V4L2_COLORSPACE_REC709: u32 = 3;
const V4L2_YCBCR_ENC_709: u32 = 2;
const V4L2_QUANTIZATION_LIM_RANGE: u32 = 2;
const V4L2_PIX_FMT_YUYV: u32 = u32::from_le_bytes(*b"YUYV");

/// `struct v4l2_capability`。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Capability {
    driver: [u8; 16],
    card: [u8; 32],
    bus_info: [u8; 32],
    version: u32,
    capabilities: u32,
    device_caps: u32,
    reserved: [u32; 3],
}

/// `struct v4l2_pix_format`。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct PixFormat {
    width: u32,
    height: u32,
    pixelformat: u32,
    field: u32,
    bytesperline: u32,
    sizeimage: u32,
    colorspace: u32,
    priv_: u32,
    flags: u32,
    ycbcr_enc: u32,
    quantization: u32,
    xfer_func: u32,
}

/// `struct v4l2_format` の `fmt` の共用体。ポインタを含む型 (`v4l2_window`) があるので、ポインタの境界に揃う。
#[repr(C)]
#[derive(Clone, Copy)]
union FormatUnion {
    pix: PixFormat,
    raw: [u8; 200],
    _align: *const u8,
}

/// `struct v4l2_format`。
#[repr(C)]
#[derive(Clone, Copy)]
struct Format {
    type_: u32,
    fmt: FormatUnion,
}

/// `struct v4l2_fract`。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct Fract {
    numerator: u32,
    denominator: u32,
}

/// `struct v4l2_outputparm`。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct OutputParm {
    capability: u32,
    outputmode: u32,
    timeperframe: Fract,
    extendedmode: u32,
    writebuffers: u32,
    reserved: [u32; 4],
}

/// `struct v4l2_streamparm` の `parm` の共用体。
#[repr(C)]
#[derive(Clone, Copy)]
union StreamParmUnion {
    output: OutputParm,
    raw: [u8; 200],
}

/// `struct v4l2_streamparm`。
#[repr(C)]
#[derive(Clone, Copy)]
struct StreamParm {
    type_: u32,
    parm: StreamParmUnion,
}

/// `_IOC(dir, 'V', nr, size)` (`asm-generic/ioctl.h`)。
const fn ioc(dir: u32, nr: u32, size: usize) -> u32 {
    (dir << 30) | ((size as u32) << 16) | ((b'V' as u32) << 8) | nr
}
const IOC_READ: u32 = 2;
const IOC_READ_WRITE: u32 = 3;
const VIDIOC_QUERYCAP: u32 = ioc(IOC_READ, 0, size_of::<Capability>());
const VIDIOC_S_FMT: u32 = ioc(IOC_READ_WRITE, 5, size_of::<Format>());
const VIDIOC_S_PARM: u32 = ioc(IOC_READ_WRITE, 22, size_of::<StreamParm>());

/// `ioctl(fd, request, arg)`。`T` は `request` が読み書きする構造体の型。
fn ioctl<T>(file: &File, request: u32, arg: &mut T) -> std::io::Result<()> {
    // SAFETY: `file` は開いたままの機器のファイルなので fd は有効。`arg` は排他的に借りた `T` で、`request` は
    // `T` の大きさから作った番号 (VIDIOC_*) だけを渡すので、カーネルは `size_of::<T>()` バイトの範囲だけを読み書きする。
    let result = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            request as libc::Ioctl,
            std::ptr::from_mut(arg).cast::<libc::c_void>(),
        )
    };
    if result < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn query_capability(file: &File) -> std::io::Result<Capability> {
    let mut cap = Capability {
        driver: [0; 16],
        card: [0; 32],
        bus_info: [0; 32],
        version: 0,
        capabilities: 0,
        device_caps: 0,
        reserved: [0; 3],
    };
    ioctl(file, VIDIOC_QUERYCAP, &mut cap)?;
    Ok(cap)
}

/// NUL で終わる文字列の中身。
fn c_str(bytes: &[u8]) -> &[u8] {
    bytes.split(|b| *b == 0).next().unwrap_or_default()
}

impl Capability {
    fn is_loopback(&self) -> bool {
        c_str(&self.driver) == LOOPBACK_DRIVER
    }

    /// 書き込みを受けるか (`exclusive_caps=1` の機器は、ほかのアプリが書いている間は出力を外す)。
    fn accepts_output(&self) -> bool {
        let caps = if self.capabilities & V4L2_CAP_DEVICE_CAPS != 0 {
            self.device_caps
        } else {
            self.capabilities
        };
        caps & V4L2_CAP_VIDEO_OUTPUT != 0
    }
}

/// `/dev/video*` を番号順に並べる。
fn video_devices() -> Vec<PathBuf> {
    let mut devices: Vec<(u32, PathBuf)> = std::fs::read_dir("/dev")
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let name = entry.file_name();
                    let n = name.to_str()?.strip_prefix("video")?.parse().ok()?;
                    Some((n, entry.path()))
                })
                .collect()
        })
        .unwrap_or_default();
    devices.sort();
    devices.into_iter().map(|(_, path)| path).collect()
}

/// v4l2loopback の機器を探す。書ける機器が無ければ、[`not_found`] で理由を選ぶ。
fn find_device() -> Result<(PathBuf, File), Error> {
    let mut busy = false;
    let mut denied = false;
    for path in video_devices() {
        let file = match OpenOptions::new().read(true).write(true).open(&path) {
            Ok(file) => file,
            Err(e) => {
                // `EACCES` (と `EPERM`)。開けないので loopback の機器かは分からない。
                denied |= e.kind() == std::io::ErrorKind::PermissionDenied;
                continue;
            }
        };
        let Ok(cap) = query_capability(&file) else {
            continue;
        };
        if !cap.is_loopback() {
            continue;
        }
        if cap.accepts_output() {
            return Ok((path, file));
        }
        busy = true;
    }
    Err(not_found(busy, denied, module_loaded()))
}

/// 書ける loopback の機器が無かった理由。loopback の機器が書かれていれば [`Error::Busy`]。権限が無くて開けない機器があり、
/// v4l2loopback が読み込まれていれば (その機器が loopback のものかもしれない) [`Error::NoPermission`]。ほかは [`Error::NoDevice`]。
fn not_found(busy: bool, denied: bool, module_loaded: bool) -> Error {
    if busy {
        Error::Busy
    } else if denied && module_loaded {
        Error::NoPermission
    } else {
        Error::NoDevice
    }
}

/// v4l2loopback のモジュールが読み込まれているか。
fn module_loaded() -> bool {
    Path::new("/sys/module/v4l2loopback").exists()
}

/// 機器を開かずに、v4l2loopback の機器がありそうかを確かめる (モジュールが読み込まれているか)。
pub(crate) fn availability() -> Result<(), Error> {
    if module_loaded() {
        Ok(())
    } else {
        Err(Error::NoDevice)
    }
}

/// 開いた v4l2loopback の機器。
#[derive(Debug)]
pub(crate) struct Loopback {
    path: PathBuf,
    file: File,
}

impl Loopback {
    pub(crate) fn open(config: Config) -> Result<Self, Error> {
        let (path, file) = find_device()?;
        let pix = PixFormat {
            width: config.width,
            height: config.height,
            pixelformat: V4L2_PIX_FMT_YUYV,
            field: V4L2_FIELD_NONE,
            bytesperline: config.width * 2,
            sizeimage: config.width * config.height * 2,
            colorspace: V4L2_COLORSPACE_REC709,
            ycbcr_enc: V4L2_YCBCR_ENC_709,
            quantization: V4L2_QUANTIZATION_LIM_RANGE,
            ..PixFormat::default()
        };
        let mut format = Format {
            type_: V4L2_BUF_TYPE_VIDEO_OUTPUT,
            fmt: FormatUnion { raw: [0; 200] },
        };
        format.fmt.pix = pix;
        ioctl(&file, VIDIOC_S_FMT, &mut format)
            .map_err(|e| Error::os("setting the format of the v4l2loopback device", e))?;
        // SAFETY: `fmt` の共用体は `pix` で書き、VIDIOC_S_FMT (出力の種類) はカーネルが `pix` の形で書き戻す。
        // `PixFormat` は u32 だけの構造体なので、どのビット列でも有効な値になる。
        let actual = unsafe { format.fmt.pix };
        if (actual.width, actual.height, actual.pixelformat)
            != (config.width, config.height, V4L2_PIX_FMT_YUYV)
        {
            return Err(Error::os(
                "setting the format of the v4l2loopback device",
                format!(
                    "the device chose {}x{} {:?} instead of {}x{} YUYV",
                    actual.width,
                    actual.height,
                    actual.pixelformat.to_le_bytes().map(char::from),
                    config.width,
                    config.height,
                ),
            ));
        }
        // フレームレートは受け取るアプリに見せるだけ (書く速さはスタジオが決める)。古い v4l2loopback は受けないので無視する。
        let mut parm = StreamParm {
            type_: V4L2_BUF_TYPE_VIDEO_OUTPUT,
            parm: StreamParmUnion { raw: [0; 200] },
        };
        parm.parm.output = OutputParm {
            timeperframe: Fract {
                numerator: 1,
                denominator: config.fps,
            },
            ..OutputParm::default()
        };
        let _ = ioctl(&file, VIDIOC_S_PARM, &mut parm);
        Ok(Self { path, file })
    }

    pub(crate) fn device(&self) -> String {
        self.path.display().to_string()
    }

    pub(crate) fn send(&mut self, frame: &[u8]) -> Result<Delivery, Error> {
        self.file
            .write_all(frame)
            .map_err(|e| Error::os("writing to the v4l2loopback device", e))?;
        Ok(Delivery::Delivered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 構造体の大きさは `linux/videodev2.h` と同じ (ioctl の番号に入り、違うとカーネルが受けない)。
    #[test]
    fn structs_match_the_kernel_layout() {
        assert_eq!(size_of::<Capability>(), 104);
        assert_eq!(size_of::<PixFormat>(), 48);
        assert_eq!(size_of::<StreamParm>(), 204);
        if cfg!(target_pointer_width = "64") {
            assert_eq!(size_of::<Format>(), 208);
            assert_eq!(VIDIOC_S_FMT, 0xc0d0_5605);
        }
        assert_eq!(VIDIOC_QUERYCAP, 0x8068_5600);
        assert_eq!(VIDIOC_S_PARM, 0xc0cc_5616);
    }

    #[test]
    fn the_reason_tells_a_missing_permission_from_a_missing_module() {
        assert_eq!(not_found(false, false, false).code(), "noDevice");
        assert_eq!(not_found(false, false, true).code(), "noDevice");
        // 開けない機器があっても、v4l2loopback が無ければ入れ方を案内する。
        assert_eq!(not_found(false, true, false).code(), "noDevice");
        assert_eq!(not_found(false, true, true).code(), "noPermission");
        // 書かれている loopback の機器を開けたなら、権限ではなくほかのアプリのせい。
        assert_eq!(not_found(true, true, true).code(), "busy");
        assert_eq!(not_found(true, false, true).code(), "busy");
    }

    #[test]
    fn capabilities_decide_whether_the_device_takes_output() {
        let mut cap = Capability {
            driver: [0; 16],
            card: [0; 32],
            bus_info: [0; 32],
            version: 0,
            capabilities: V4L2_CAP_DEVICE_CAPS | V4L2_CAP_VIDEO_OUTPUT,
            device_caps: 1,
            reserved: [0; 3],
        };
        cap.driver[..LOOPBACK_DRIVER.len()].copy_from_slice(LOOPBACK_DRIVER);
        assert!(cap.is_loopback());
        // DEVICE_CAPS があれば機器ごとの能力 (ここでは取り込みだけ) を見る。
        assert!(!cap.accepts_output());
        cap.device_caps = V4L2_CAP_VIDEO_OUTPUT;
        assert!(cap.accepts_output());
        cap.driver[0] = b'u';
        assert!(!cap.is_loopback());
    }

    #[test]
    fn devices_are_sorted_by_number() {
        // 中身は環境による。並びだけを確かめる。
        let devices = video_devices();
        let numbers: Vec<u32> = devices
            .iter()
            .filter_map(|p| p.file_name()?.to_str()?.strip_prefix("video")?.parse().ok())
            .collect();
        assert!(numbers.windows(2).all(|w| w[0] < w[1]));
    }
}
