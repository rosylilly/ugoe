//! Windows 11: Media Foundation の仮想カメラ (ADR 0028 の 8.、`docs/studio.md#windows-media-foundation`)。
//!
//! - スタジオ側 ([`Camera`]): `MFCreateVirtualCamera` でカメラの機器を作り (このプロセスが終わると消える `Session` の寿命、
//!   今の利用者だけ)、メディアソースが作る共有メモリ ([`crate::shared`]) を開いて NV12 の画を書く。
//! - メディアソース (`source`): OS のサービス Frame Server がこのクレートの DLL を読み込んで作る。共有メモリを作り、
//!   受け取るアプリの求めに応じて最新の画を渡す。DLL の登録はインストーラが管理者の権限で `regsvr32` で行う。
//!
//! `MFCreateVirtualCamera` は Windows 11 にしか無い。リンクすると Windows 10 で exe が起動できなくなるので、実行時に引く。

mod source;

use std::ffi::c_void;
use std::time::{Duration, Instant};

use windows::Win32::Devices::Properties::DEVPROP_TYPE_BINARY;
use windows::Win32::Foundation::{CloseHandle, DEVPROPKEY, HANDLE};
use windows::Win32::Media::MediaFoundation::{
    IMFVirtualCamera, MF_VERSION, MFSTARTUP_FULL, MFShutdown, MFStartup, MFVirtualCameraAccess,
    MFVirtualCameraAccess_CurrentUser, MFVirtualCameraLifetime, MFVirtualCameraLifetime_Session,
    MFVirtualCameraType, MFVirtualCameraType_SoftwareCameraSource,
};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};
use windows::Win32::System::Memory::{
    FILE_MAP_READ, FILE_MAP_WRITE, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile, OpenFileMappingW,
    UnmapViewOfFile,
};
use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
use windows_core::{GUID, HRESULT, HSTRING, Interface, PCWSTR, s, w};

use crate::shared::{self, REGION_BYTES, Region, SharedError, Writer};
use crate::{Config, ConsumerWatch, Delivery, Error};

/// メディアソースの CLSID (DLL の登録と、カメラを作るときに渡す)。
pub(crate) const CLSID_SOURCE: GUID = GUID::from_u128(0x128a1a50_b227_494b_952e_0c011bb01869);
/// スタジオがカメラの機器に付ける設定 ([`shared::encode_config`]) のプロパティ。
pub(crate) const CONFIG_KEY: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0x7c3b36c7_605d_463b_b505_7ba18ab3c1c2),
    pid: 2,
};
/// 共有メモリの名前。メディアソースは Frame Server (セッション 0) で動くので `Global\` に置く。
pub(crate) const SECTION_NAME: PCWSTR = w!("Global\\ugoe-studio-virtual-camera-1");
/// カメラの一覧に出る名前。
const FRIENDLY_NAME: &str = "ugoe studio";
/// 共有メモリが無い (受け取るアプリがいない) ときに、開き直すまでの間隔。
const REOPEN_INTERVAL: Duration = Duration::from_secs(1);
/// クラスが登録されていない (`REGDB_E_CLASSNOTREG`)。
const REGDB_E_CLASSNOTREG: HRESULT = HRESULT(0x8004_0154_u32 as i32);

/// `{XXXXXXXX-...}` の形の CLSID。
pub(crate) fn clsid_string() -> String {
    format!("{{{CLSID_SOURCE:?}}}")
}

/// 登録したメディアソースの DLL の場所を書くレジストリのキー。
pub(crate) fn clsid_key() -> String {
    format!(
        "SOFTWARE\\Classes\\CLSID\\{}\\InprocServer32",
        clsid_string()
    )
}

type CreateVirtualCamera = unsafe extern "system" fn(
    MFVirtualCameraType,
    MFVirtualCameraLifetime,
    MFVirtualCameraAccess,
    PCWSTR,
    PCWSTR,
    *const GUID,
    u32,
    *mut *mut c_void,
) -> HRESULT;

/// `mfsensorgroup.dll` の `MFCreateVirtualCamera` を引く。無ければ Windows 11 より前。
fn create_virtual_camera_fn() -> Result<CreateVirtualCamera, Error> {
    // SAFETY: システムのフォルダの DLL を名前で読み込むだけ。返った HMODULE はプロセスの終わりまで解放しない
    // (下で取る関数のポインタを使い続けるため)。
    let module =
        unsafe { LoadLibraryExW(w!("mfsensorgroup.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32) }
            .map_err(|_| Error::WindowsTooOld)?;
    // SAFETY: `module` は上で読み込んだまま解放しない有効な HMODULE。名前は NUL で終わる定数。
    let proc = unsafe { GetProcAddress(module, s!("MFCreateVirtualCamera")) }
        .ok_or(Error::WindowsTooOld)?;
    // SAFETY: `MFCreateVirtualCamera` の型は mfvirtualcamera.h の宣言 (windows クレートの宣言と同じ) で、
    // 関数のポインタどうしは同じ大きさ。
    Ok(unsafe {
        std::mem::transmute::<unsafe extern "system" fn() -> isize, CreateVirtualCamera>(proc)
    })
}

/// メディアソースの DLL が登録されていて、ファイルがあるか。
fn source_registered() -> bool {
    let key = HSTRING::from(clsid_key());
    let mut path = [0u16; 1024];
    let mut size = std::mem::size_of_val(&path) as u32;
    // SAFETY: `path` は `size` バイトの書ける領域で、RegGetValueW は `size` を超えて書かない。
    let result = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            &key,
            PCWSTR::null(),
            RRF_RT_REG_SZ,
            None,
            Some(path.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if result.is_err() {
        return false;
    }
    let len = path.iter().position(|c| *c == 0).unwrap_or(path.len());
    std::path::Path::new(&String::from_utf16_lossy(&path[..len])).exists()
}

/// 仮想カメラを始められるか (Windows 11 か、メディアソースが登録されているか)。
pub(crate) fn availability() -> Result<(), Error> {
    create_virtual_camera_fn()?;
    if source_registered() {
        Ok(())
    } else {
        Err(Error::NotRegistered)
    }
}

/// 名前付きの共有メモリの見え方 (ビュー)。落とすとビューとハンドルを閉じる。
#[derive(Debug)]
pub(crate) struct Mapping {
    handle: HANDLE,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
    len: usize,
}

impl Mapping {
    /// `handle` (共有メモリ) の先頭から `len` バイトを読み書きできるように写す。失敗したら `handle` を閉じる。
    pub(crate) fn map(handle: HANDLE, len: usize) -> windows_core::Result<Self> {
        // SAFETY: `handle` は呼び出し元が開いた共有メモリで、`len` バイト以上ある (作るときに大きさを決めている)。
        let view = unsafe { MapViewOfFile(handle, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, len) };
        if view.Value.is_null() {
            let error = windows_core::Error::from_thread();
            // SAFETY: `handle` は呼び出し元から受け取った、まだ閉じていないハンドル。
            let _ = unsafe { CloseHandle(handle) };
            return Err(error);
        }
        Ok(Self { handle, view, len })
    }

    /// スタジオ側: メディアソースが作った共有メモリを開く。無ければ `None` (受け取るアプリがいない)。
    fn open() -> Option<Self> {
        // SAFETY: 名前は NUL で終わる定数。返ったハンドルは `Mapping` が閉じる。
        let handle =
            unsafe { OpenFileMappingW((FILE_MAP_READ | FILE_MAP_WRITE).0, false, SECTION_NAME) }
                .ok()?;
        Self::map(handle, REGION_BYTES).ok()
    }

    fn ptr(&self, offset: usize, len: usize) -> Option<*mut u8> {
        (offset.checked_add(len)? <= self.len)
            .then(|| self.view.Value.cast::<u8>().wrapping_add(offset))
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        // SAFETY: `view` と `handle` は `map` で得たもので、ここでだけ一度閉じる。
        unsafe {
            let _ = UnmapViewOfFile(self.view);
            let _ = CloseHandle(self.handle);
        }
    }
}

impl Region for Mapping {
    fn byte_len(&self) -> usize {
        self.len
    }

    fn load(&self, word: usize) -> u32 {
        let Some(ptr) = self.ptr(word * 4, 4) else {
            return 0;
        };
        // SAFETY: `ptr` はビューの中 (`ptr` が範囲を確かめた) で、ビューはページの境界から始まるので 4 バイトに揃う。
        // 別のプロセスも同じ語を原子的にしか触らない (`shared` の約束)。
        unsafe { std::sync::atomic::AtomicU32::from_ptr(ptr.cast::<u32>()) }
            .load(std::sync::atomic::Ordering::Acquire)
    }

    fn store(&self, word: usize, value: u32) {
        let Some(ptr) = self.ptr(word * 4, 4) else {
            return;
        };
        // SAFETY: `load` と同じ。
        unsafe { std::sync::atomic::AtomicU32::from_ptr(ptr.cast::<u32>()) }
            .store(value, std::sync::atomic::Ordering::Release);
    }

    fn write(&self, offset: usize, bytes: &[u8]) {
        let Some(ptr) = self.ptr(offset, bytes.len()) else {
            return;
        };
        // SAFETY: 書く先はビューの中 (`ptr` が範囲を確かめた) で、`bytes` とは重ならない (別の領域)。読み手が同時に
        // 読んでいても、枠の版 (seqlock) が合わなければ読み手はその画を使わない (`shared` の約束)。
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
    }

    fn read(&self, offset: usize, out: &mut [u8]) {
        let Some(ptr) = self.ptr(offset, out.len()) else {
            return;
        };
        // SAFETY: `write` と同じ (向きが逆)。
        unsafe { std::ptr::copy_nonoverlapping(ptr.cast_const(), out.as_mut_ptr(), out.len()) };
    }
}

/// スタジオ側の仮想カメラ。
#[derive(Debug)]
pub(crate) struct Camera {
    camera: Option<IMFVirtualCamera>,
    config: Config,
    writer: Option<Writer<Mapping>>,
    /// 最後に共有メモリを開こうとした時刻。
    opened_at: Option<Instant>,
    /// メディアソースが渡したフレームの数 (増えていれば受け取るアプリがいる)。
    delivered: u32,
    consumer: ConsumerWatch,
    /// CoInitializeEx が成功した (CoUninitialize で釣り合わせる)。
    com: bool,
    /// MFStartup が成功した (MFShutdown で釣り合わせる)。
    media_foundation: bool,
}

impl Camera {
    pub(crate) fn start(config: Config) -> Result<Self, Error> {
        let create = create_virtual_camera_fn()?;
        if !source_registered() {
            return Err(Error::NotRegistered);
        }
        let mut camera = Self {
            camera: None,
            config,
            writer: None,
            opened_at: None,
            delivered: 0,
            consumer: ConsumerWatch::new(Instant::now()),
            com: false,
            media_foundation: false,
        };
        // SAFETY: このスレッドの COM を初期化する。成功したら落とすときに CoUninitialize で釣り合わせる。
        // ほかの形で初期化済み (RPC_E_CHANGED_MODE) なら、そのまま使う。
        camera.com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
        // SAFETY: Media Foundation を初期化する。成功したら落とすときに MFShutdown で釣り合わせる。
        unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) }
            .map_err(|e| Error::os("starting Media Foundation", e))?;
        camera.media_foundation = true;

        let name = HSTRING::from(FRIENDLY_NAME);
        let clsid = HSTRING::from(clsid_string());
        let mut raw: *mut c_void = std::ptr::null_mut();
        // SAFETY: `create` は MFCreateVirtualCamera。文字列は呼び出しの間生きている HSTRING で、カテゴリは渡さない (既定)。
        // 成功すると `raw` に参照を 1 つ持った IMFVirtualCamera が入る。
        let hr = unsafe {
            create(
                MFVirtualCameraType_SoftwareCameraSource,
                MFVirtualCameraLifetime_Session,
                MFVirtualCameraAccess_CurrentUser,
                PCWSTR(name.as_ptr()),
                PCWSTR(clsid.as_ptr()),
                std::ptr::null(),
                0,
                &mut raw,
            )
        };
        hr.ok()
            .map_err(|e| start_error("creating the virtual camera", e))?;
        // SAFETY: 成功した MFCreateVirtualCamera が返した、参照を 1 つ持つ IMFVirtualCamera のポインタ。その参照を引き取る。
        let virtual_camera = unsafe { IMFVirtualCamera::from_raw(raw) };
        // SAFETY: `virtual_camera` は有効なカメラ。設定は 16 バイトのバイト列で、呼び出しの間生きている。
        unsafe {
            virtual_camera.AddProperty(
                &CONFIG_KEY,
                DEVPROP_TYPE_BINARY,
                &shared::encode_config(config),
            )
        }
        .map_err(|e| start_error("configuring the virtual camera", e))?;
        // SAFETY: `virtual_camera` は有効なカメラ。通知は受けない (None)。
        unsafe { virtual_camera.Start(None) }
            .map_err(|e| start_error("starting the virtual camera", e))?;
        camera.camera = Some(virtual_camera);
        Ok(camera)
    }

    pub(crate) fn device(&self) -> String {
        FRIENDLY_NAME.to_owned()
    }

    pub(crate) fn send(&mut self, frame: &[u8]) -> Result<Delivery, Error> {
        let now = Instant::now();
        if self.writer.is_none()
            && self
                .opened_at
                .is_none_or(|at| now.duration_since(at) >= REOPEN_INTERVAL)
        {
            self.opened_at = Some(now);
            self.writer = match Mapping::open().map(Writer::new) {
                None => None,
                Some(Ok(writer)) => Some(writer),
                // メディアソースが頭を書いている途中: 次に開き直す。
                Some(Err(SharedError::Magic)) => None,
                // 形が違う (メディアソースの DLL とスタジオの版が違う) のは待っても直らないので、止めて知らせる。
                Some(Err(e)) => {
                    return Err(Error::os("opening the virtual camera's shared memory", e));
                }
            };
            self.delivered = self.writer.as_ref().map_or(0, Writer::delivered);
            self.consumer = ConsumerWatch::new(now);
        }
        let Some(writer) = &mut self.writer else {
            return Ok(Delivery::NoConsumer);
        };
        let Config { width, height, fps } = self.config;
        writer
            .write(width, height, fps, frame)
            .map_err(|e| Error::os("writing to the virtual camera", e))?;
        let delivered = writer.delivered();
        let consumed = delivered != self.delivered;
        self.delivered = delivered;
        Ok(self.consumer.delivery(consumed, now))
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        self.writer = None;
        if let Some(camera) = self.camera.take() {
            // SAFETY: `camera` は有効なカメラ。止めてから閉じる (`Session` の寿命なので機器も消える)。
            unsafe {
                let _ = camera.Stop();
                let _ = camera.Shutdown();
            }
        }
        if self.media_foundation {
            // SAFETY: `start` の MFStartup と釣り合わせる。Media Foundation のオブジェクトは上で手放した。
            let _ = unsafe { MFShutdown() };
        }
        if self.com {
            // SAFETY: `start` の CoInitializeEx と同じスレッドで釣り合わせる (`VirtualCamera` は Send でないので、作ったスレッドで落ちる)。
            unsafe { CoUninitialize() };
        }
    }
}

/// カメラを作る・始めるときの失敗。クラスが無ければ登録されていない。
fn start_error(context: &'static str, error: windows_core::Error) -> Error {
    if error.code() == REGDB_E_CLASSNOTREG {
        Error::NotRegistered
    } else {
        Error::os(context, error)
    }
}
