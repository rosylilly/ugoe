//! Frame Server が読み込むメディアソース (ugoe_studio_vcam.dll)。
//!
//! 流れ: Frame Server が `DllGetClassObject` でクラスファクトリを取り、[`Activate`] (IMFActivate) を作らせ、属性 (カメラの
//! 機器のシンボリックリンクなど) を書いてから `ActivateObject` で [`Source`] を作る。受け取るアプリがカメラを開くと
//! `Start` → ストリームの `RequestSample` が繰り返し呼ばれ、共有メモリの最新の画を NV12 のサンプルにして返す。
//!
//! - 出す形は NV12 だけ。大きさとフレームレートは、スタジオがカメラの機器に付けた設定 ([`super::CONFIG_KEY`]) を読む。
//!   読めなければ選べる組をすべて出す。受け取るアプリが選んだ大きさと、届いた画の大きさが違えば最も近い画素で拡大縮小する。
//! - 画がまだ届いていなければ黒を出す。
//! - サービスの中で動くので、パニックを外へ出さない (COM の境界で `catch_unwind` する)。
//!
//! 参考 (挙動を自分の言葉で書いた。コードは写していない):
//! - <https://learn.microsoft.com/windows/win32/api/mfvirtualcamera/nf-mfvirtualcamera-mfcreatevirtualcamera>
//! - <https://learn.microsoft.com/windows-hardware/drivers/stream/frame-server-custom-media-source>

use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_Device_Interface_PropertyW, CR_SUCCESS,
};
use windows::Win32::Devices::Properties::DEVPROPTYPE;
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_FAIL, E_INVALIDARG, E_POINTER,
    ERROR_SET_NOT_FOUND, HMODULE, S_FALSE, S_OK,
};
use windows::Win32::Foundation::{HLOCAL, INVALID_HANDLE_VALUE, LocalFree};
use windows::Win32::Media::KernelStreaming::{
    IKsControl, IKsControl_Impl, KSIDENTIFIER, PINNAME_VIDEO_CAPTURE,
};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};
use windows::Win32::System::Memory::{CreateFileMappingW, PAGE_READWRITE};
use windows::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW,
};
use windows_core::{
    BOOL, GUID, HRESULT, HSTRING, IUnknown, Interface, PCWSTR, PWSTR, Ref, Result, implement, w,
};

use super::{CLSID_SOURCE, CONFIG_KEY, Mapping, SECTION_NAME, clsid_key};
use crate::shared::{self, REGION_BYTES, Reader};
use crate::{Config, FRAME_RATES, RESOLUTIONS};

/// 共有メモリの権限 (SDDL): SYSTEM・Local Service・管理者はすべて、対話でログオンした利用者 (スタジオ) は読み書き。
const SECTION_SDDL: PCWSTR = w!("D:P(A;;GA;;;SY)(A;;GA;;;LS)(A;;GA;;;BA)(A;;GRGW;;;IU)");

/// ストリームの番号 (1 本だけ)。
const STREAM_ID: u32 = 0;

/// パニックを COM の失敗に変える (サービスのプロセスを落とさない)。
fn guard<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    std::panic::catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| Err(E_FAIL.into()))
}

/// 毒された Mutex も使い続ける。
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// NV12 の media type を作る。
fn media_type(config: Config) -> Result<IMFMediaType> {
    let Config { width, height, fps } = config;
    // SAFETY: MFCreateMediaType は新しい media type を返し、以下はその属性を書くだけ (GUID は定数)。
    unsafe {
        let t = MFCreateMediaType()?;
        t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        t.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)?;
        t.SetUINT64(
            &MF_MT_FRAME_SIZE,
            (u64::from(width) << 32) | u64::from(height),
        )?;
        t.SetUINT64(&MF_MT_FRAME_RATE, (u64::from(fps) << 32) | 1)?;
        t.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1 << 32) | 1)?;
        t.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
        t.SetUINT32(&MF_MT_ALL_SAMPLES_INDEPENDENT, 1)?;
        t.SetUINT32(&MF_MT_DEFAULT_STRIDE, width)?;
        t.SetUINT32(&MF_MT_SAMPLE_SIZE, shared::nv12_bytes(width, height) as u32)?;
        t.SetUINT32(
            &MF_MT_AVG_BITRATE,
            shared::nv12_bytes(width, height) as u32 * 8 * fps,
        )?;
        t.SetUINT32(&MF_MT_YUV_MATRIX, MFVideoTransferMatrix_BT709.0 as u32)?;
        t.SetUINT32(&MF_MT_VIDEO_NOMINAL_RANGE, MFNominalRange_16_235.0 as u32)?;
        Ok(t)
    }
}

/// 出す組: スタジオの設定が読めればそれだけ、読めなければ選べる組をすべて (大きい順)。
fn offered(config: Option<Config>) -> Vec<Config> {
    match config {
        Some(config) => vec![config],
        None => RESOLUTIONS
            .iter()
            .flat_map(|&(width, height)| {
                FRAME_RATES
                    .iter()
                    .map(move |&fps| Config { width, height, fps })
            })
            .collect(),
    }
}

/// カメラの機器 (シンボリックリンク) に、スタジオが付けた設定を読む。
fn read_config(attributes: &IMFAttributes) -> Option<Config> {
    let mut link = PWSTR::null();
    let mut len = 0u32;
    // SAFETY: Frame Server が書いたシンボリックリンクの文字列を、CoTaskMemAlloc の領域で受け取る (下で解放する)。
    unsafe {
        attributes.GetAllocatedString(
            &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
            &mut link,
            &mut len,
        )
    }
    .ok()?;
    let mut buffer = [0u8; 16];
    let mut size = buffer.len() as u32;
    let mut kind = DEVPROPTYPE::default();
    // SAFETY: `link` は上で受け取った NUL で終わる文字列。`buffer` は `size` バイトの書ける領域。
    let result = unsafe {
        CM_Get_Device_Interface_PropertyW(
            PCWSTR(link.0),
            &CONFIG_KEY,
            &mut kind,
            Some(buffer.as_mut_ptr()),
            &mut size,
            0,
        )
    };
    // SAFETY: `link` は GetAllocatedString が CoTaskMemAlloc で確保したもので、ここで一度だけ解放する。
    unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(link.0.cast_const().cast())) };
    (result == CR_SUCCESS).then(|| shared::decode_config(&buffer[..size as usize]))?
}

/// 共有メモリを作る (すでにあれば開く。前のメディアソースやスタジオが開いたままのとき)。
fn create_section() -> Result<Reader<Mapping>> {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // SAFETY: SDDL は NUL で終わる定数。作られた記述子は LocalAlloc の領域で、下で解放する。
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            SECTION_SDDL,
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
    }?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };
    let size = REGION_BYTES as u64;
    // SAFETY: ページファイルを裏に持つ共有メモリを、名前と権限を付けて作る。`attributes` は呼び出しの間生きている。
    let handle = unsafe {
        CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            Some(&attributes),
            PAGE_READWRITE,
            (size >> 32) as u32,
            size as u32,
            SECTION_NAME,
        )
    };
    // SAFETY: `descriptor` は上で LocalAlloc されたもので、CreateFileMappingW の後は使わない。
    unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
    let mapping = Mapping::map(handle?, REGION_BYTES)?;
    Reader::new(mapping).map_err(|_| E_FAIL.into())
}

/// ストリームの状態。
#[derive(Debug)]
struct StreamState {
    state: MF_STREAM_STATE,
    shutdown: bool,
    reader: Option<Reader<Mapping>>,
    /// 共有メモリから読んだ画と、拡大縮小した画。
    frame: Vec<u8>,
    scaled: Vec<u8>,
    /// 最後に渡した時刻 (フレームレートを超えて渡さない)。
    last: Option<Instant>,
    /// まだ一度も画を読めていない。
    empty: bool,
}

/// 映像のストリーム (1 本)。
#[implement(IMFMediaStream2, IKsControl)]
struct Stream {
    queue: IMFMediaEventQueue,
    descriptor: IMFStreamDescriptor,
    source: Mutex<Option<IMFMediaSource>>,
    inner: Mutex<StreamState>,
}

impl Stream {
    fn check(&self) -> Result<()> {
        if lock(&self.inner).shutdown {
            Err(MF_E_SHUTDOWN.into())
        } else {
            Ok(())
        }
    }

    fn set_state(&self, state: MF_STREAM_STATE, event: MF_EVENT_TYPE) -> Result<()> {
        lock(&self.inner).state = state;
        // SAFETY: 自分のイベントの列に、値の無いイベントを積む。
        unsafe {
            self.queue
                .QueueEventParamVar(event.0 as u32, &GUID::zeroed(), S_OK, std::ptr::null())
        }
    }

    fn shutdown(&self) {
        let mut inner = lock(&self.inner);
        inner.shutdown = true;
        inner.reader = None;
        drop(inner);
        *lock(&self.source) = None;
        // SAFETY: 自分のイベントの列を閉じる (待っている呼び出しに MF_E_SHUTDOWN を返させる)。
        let _ = unsafe { self.queue.Shutdown() };
    }

    /// 今の media type の大きさとフレームレート。
    fn current(&self) -> Result<Config> {
        // SAFETY: 自分のストリーム記述子から、今の media type の属性を読むだけ。
        unsafe {
            let t = self
                .descriptor
                .GetMediaTypeHandler()?
                .GetCurrentMediaType()?;
            let size = t.GetUINT64(&MF_MT_FRAME_SIZE)?;
            let rate = t.GetUINT64(&MF_MT_FRAME_RATE)?;
            let (num, den) = ((rate >> 32) as u32, (rate as u32).max(1));
            Ok(Config {
                width: (size >> 32) as u32,
                height: size as u32,
                fps: (num / den).max(1),
            })
        }
    }

    /// 共有メモリの最新の画を、受け取るアプリが選んだ大きさの NV12 にして返す。
    fn next_frame(inner: &mut StreamState, want: Config) -> Vec<u8> {
        let StreamState {
            reader,
            frame,
            scaled,
            empty,
            ..
        } = inner;
        let bytes = shared::nv12_bytes(want.width, want.height);
        if reader.is_none() {
            *reader = create_section().ok();
        }
        let info = reader.as_ref().and_then(|r| r.read(frame));
        if let Some(r) = reader.as_ref()
            && info.is_some()
        {
            r.mark_delivered();
        }
        match info {
            Some(info) => {
                *empty = false;
                if (info.width, info.height) == (want.width, want.height) {
                    scaled.clear();
                    scaled.extend_from_slice(&frame[..bytes]);
                } else {
                    scaled.resize(bytes, 0);
                    shared::scale_nv12_nearest(
                        frame,
                        (info.width, info.height),
                        scaled,
                        (want.width, want.height),
                    );
                }
            }
            None if *empty || scaled.len() != bytes => {
                scaled.resize(bytes, 0);
                shared::fill_black_nv12(scaled, want.width, want.height);
            }
            // 書き換えの途中で読めなかった: 前の画を使い続ける。
            None => {}
        }
        scaled.clone()
    }
}

impl IMFMediaEventGenerator_Impl for Stream_Impl {
    fn GetEvent(&self, flags: MEDIA_EVENT_GENERATOR_GET_EVENT_FLAGS) -> Result<IMFMediaEvent> {
        // SAFETY: 自分のイベントの列への転送。
        guard(|| unsafe { self.queue.GetEvent(flags.0) })
    }

    fn BeginGetEvent(&self, callback: Ref<IMFAsyncCallback>, state: Ref<IUnknown>) -> Result<()> {
        // SAFETY: 自分のイベントの列への転送。引数は呼び出し元の参照をそのまま渡す。
        guard(|| unsafe { self.queue.BeginGetEvent(callback.as_ref(), state.as_ref()) })
    }

    fn EndGetEvent(&self, result: Ref<IMFAsyncResult>) -> Result<IMFMediaEvent> {
        // SAFETY: 自分のイベントの列への転送。
        guard(|| unsafe { self.queue.EndGetEvent(result.as_ref()) })
    }

    fn QueueEvent(
        &self,
        met: u32,
        extended: *const GUID,
        status: HRESULT,
        value: *const PROPVARIANT,
    ) -> Result<()> {
        // SAFETY: 自分のイベントの列への転送。ポインタは呼び出し元 (COM の約束で有効か NULL) のものをそのまま渡す。
        guard(|| unsafe { self.queue.QueueEventParamVar(met, extended, status, value) })
    }
}

impl IMFMediaStream_Impl for Stream_Impl {
    fn GetMediaSource(&self) -> Result<IMFMediaSource> {
        guard(|| {
            self.check()?;
            lock(&self.source)
                .clone()
                .ok_or_else(|| MF_E_SHUTDOWN.into())
        })
    }

    fn GetStreamDescriptor(&self) -> Result<IMFStreamDescriptor> {
        guard(|| {
            self.check()?;
            Ok(self.descriptor.clone())
        })
    }

    fn RequestSample(&self, token: Ref<IUnknown>) -> Result<()> {
        guard(|| {
            self.check()?;
            let want = self.current()?;
            // フレームレートより速く求められたら、次の時刻まで待つ (受け取るアプリの求めがそのまま速さになるため)。
            // 待つ間は状態を握らない (Stop や SetStreamState を止めない)。
            let interval = Duration::from_secs(1) / want.fps;
            let last = lock(&self.inner).last;
            if let Some(wait) = last.and_then(|last| interval.checked_sub(last.elapsed())) {
                std::thread::sleep(wait);
            }
            let mut inner = lock(&self.inner);
            if inner.shutdown {
                return Err(MF_E_SHUTDOWN.into());
            }
            if inner.state != MF_STREAM_STATE_RUNNING {
                return Err(MF_E_INVALIDREQUEST.into());
            }
            inner.last = Some(Instant::now());
            let pixels = Stream::next_frame(&mut inner, want);
            drop(inner);
            // SAFETY: 新しいサンプルとバッファを作り、Lock で得た `len` バイトの領域に `pixels` を写して Unlock する。
            // トークンは呼び出し元の参照をそのまま属性に付ける。
            unsafe {
                let len = pixels.len() as u32;
                let buffer = MFCreateMemoryBuffer(len)?;
                let mut data = std::ptr::null_mut();
                buffer.Lock(&mut data, None, None)?;
                std::ptr::copy_nonoverlapping(pixels.as_ptr(), data, pixels.len());
                buffer.Unlock()?;
                buffer.SetCurrentLength(len)?;
                let sample = MFCreateSample()?;
                sample.AddBuffer(&buffer)?;
                sample.SetSampleTime(MFGetSystemTime())?;
                sample.SetSampleDuration(10_000_000 / i64::from(want.fps))?;
                if let Some(token) = token.as_ref() {
                    sample.SetUnknown(&MFSampleExtension_Token, token)?;
                }
                self.queue.QueueEventParamUnk(
                    MEMediaSample.0 as u32,
                    &GUID::zeroed(),
                    S_OK,
                    &sample,
                )
            }
        })
    }
}

impl IMFMediaStream2_Impl for Stream_Impl {
    fn SetStreamState(&self, value: MF_STREAM_STATE) -> Result<()> {
        guard(|| {
            self.check()?;
            lock(&self.inner).state = value;
            Ok(())
        })
    }

    fn GetStreamState(&self) -> Result<MF_STREAM_STATE> {
        guard(|| {
            self.check()?;
            Ok(lock(&self.inner).state)
        })
    }
}

/// 独自の KS のプロパティ・メソッド・イベントは持たない (Frame Server は IKsControl を求める)。
impl IKsControl_Impl for Stream_Impl {
    fn KsProperty(
        &self,
        _: *const KSIDENTIFIER,
        _: u32,
        _: *mut c_void,
        _: u32,
        _: *mut u32,
    ) -> Result<()> {
        Err(ERROR_SET_NOT_FOUND.to_hresult().into())
    }

    fn KsMethod(
        &self,
        _: *const KSIDENTIFIER,
        _: u32,
        _: *mut c_void,
        _: u32,
        _: *mut u32,
    ) -> Result<()> {
        Err(ERROR_SET_NOT_FOUND.to_hresult().into())
    }

    fn KsEvent(
        &self,
        _: *const KSIDENTIFIER,
        _: u32,
        _: *mut c_void,
        _: u32,
        _: *mut u32,
    ) -> Result<()> {
        Err(ERROR_SET_NOT_FOUND.to_hresult().into())
    }
}

/// メディアソース。
#[implement(IMFMediaSource2, IMFGetService, IKsControl)]
struct Source {
    queue: IMFMediaEventQueue,
    presentation: IMFPresentationDescriptor,
    attributes: IMFAttributes,
    stream: windows_core::ComObject<Stream>,
    /// まだ MENewStream を送っていない (最初の Start で送る)。
    new_stream: Mutex<bool>,
    shutdown: Mutex<bool>,
}

impl Source {
    /// Frame Server が書いた属性 (`activate`) を読んでメディアソースを作る。
    fn create(activate: &IMFAttributes) -> Result<IMFMediaSource2> {
        let types = offered(read_config(activate))
            .into_iter()
            .map(|config| media_type(config).map(Some))
            .collect::<Result<Vec<_>>>()?;
        // SAFETY: 新しいイベントの列・属性・ストリーム記述子・プレゼンテーション記述子を作り、属性を書くだけ。
        let (queue, attributes, descriptor, presentation, stream_queue) = unsafe {
            let descriptor = MFCreateStreamDescriptor(STREAM_ID, &types)?;
            descriptor
                .GetMediaTypeHandler()?
                .SetCurrentMediaType(types[0].as_ref())?;
            descriptor.SetGUID(&MF_DEVICESTREAM_STREAM_CATEGORY, &PINNAME_VIDEO_CAPTURE)?;
            descriptor.SetUINT32(&MF_DEVICESTREAM_STREAM_ID, STREAM_ID)?;
            descriptor.SetUINT32(&MF_DEVICESTREAM_FRAMESERVER_SHARED, 1)?;
            descriptor.SetUINT32(
                &MF_DEVICESTREAM_ATTRIBUTE_FRAMESOURCE_TYPES,
                MFFrameSourceTypes_Color.0 as u32,
            )?;
            let presentation = MFCreatePresentationDescriptor(Some(&[Some(descriptor.clone())]))?;
            presentation.SelectStream(0)?;
            let mut attributes = None;
            MFCreateAttributes(&mut attributes, 1)?;
            (
                MFCreateEventQueue()?,
                attributes.ok_or(E_FAIL)?,
                descriptor,
                presentation,
                MFCreateEventQueue()?,
            )
        };
        let stream = windows_core::ComObject::new(Stream {
            queue: stream_queue,
            descriptor,
            source: Mutex::new(None),
            inner: Mutex::new(StreamState {
                state: MF_STREAM_STATE_STOPPED,
                shutdown: false,
                reader: None,
                frame: Vec::new(),
                scaled: Vec::new(),
                last: None,
                empty: true,
            }),
        });
        let source: IMFMediaSource2 = Source {
            queue,
            presentation,
            attributes,
            stream: stream.clone(),
            new_stream: Mutex::new(true),
            shutdown: Mutex::new(false),
        }
        .into();
        *lock(&stream.source) = Some(source.cast()?);
        Ok(source)
    }

    fn check(&self) -> Result<()> {
        if *lock(&self.shutdown) {
            Err(MF_E_SHUTDOWN.into())
        } else {
            Ok(())
        }
    }

    fn queue_empty(&self, event: MF_EVENT_TYPE) -> Result<()> {
        // SAFETY: 自分のイベントの列に、値の無いイベントを積む。
        unsafe {
            self.queue
                .QueueEventParamVar(event.0 as u32, &GUID::zeroed(), S_OK, std::ptr::null())
        }
    }
}

impl IMFMediaEventGenerator_Impl for Source_Impl {
    fn GetEvent(&self, flags: MEDIA_EVENT_GENERATOR_GET_EVENT_FLAGS) -> Result<IMFMediaEvent> {
        // SAFETY: 自分のイベントの列への転送。
        guard(|| unsafe { self.queue.GetEvent(flags.0) })
    }

    fn BeginGetEvent(&self, callback: Ref<IMFAsyncCallback>, state: Ref<IUnknown>) -> Result<()> {
        // SAFETY: 自分のイベントの列への転送。
        guard(|| unsafe { self.queue.BeginGetEvent(callback.as_ref(), state.as_ref()) })
    }

    fn EndGetEvent(&self, result: Ref<IMFAsyncResult>) -> Result<IMFMediaEvent> {
        // SAFETY: 自分のイベントの列への転送。
        guard(|| unsafe { self.queue.EndGetEvent(result.as_ref()) })
    }

    fn QueueEvent(
        &self,
        met: u32,
        extended: *const GUID,
        status: HRESULT,
        value: *const PROPVARIANT,
    ) -> Result<()> {
        // SAFETY: 自分のイベントの列への転送。ポインタは呼び出し元のものをそのまま渡す。
        guard(|| unsafe { self.queue.QueueEventParamVar(met, extended, status, value) })
    }
}

impl IMFMediaSource_Impl for Source_Impl {
    fn GetCharacteristics(&self) -> Result<u32> {
        guard(|| {
            self.check()?;
            Ok(MFMEDIASOURCE_IS_LIVE.0 as u32)
        })
    }

    fn CreatePresentationDescriptor(&self) -> Result<IMFPresentationDescriptor> {
        guard(|| {
            self.check()?;
            // SAFETY: 自分のプレゼンテーション記述子の写しを返す。
            unsafe { self.presentation.Clone() }
        })
    }

    fn Start(
        &self,
        presentation: Ref<IMFPresentationDescriptor>,
        _format: *const GUID,
        _position: *const PROPVARIANT,
    ) -> Result<()> {
        guard(|| {
            self.check()?;
            let presentation = presentation.as_ref().ok_or(E_INVALIDARG)?;
            let mut selected = BOOL::default();
            let mut descriptor = None;
            // SAFETY: 呼び出し元の記述子から、ストリーム 0 の選択の有無を読む。
            unsafe { presentation.GetStreamDescriptorByIndex(0, &mut selected, &mut descriptor) }?;
            if !selected.as_bool() {
                return Err(MF_E_INVALIDREQUEST.into());
            }
            let stream: IMFMediaStream2 = self.stream.to_interface();
            let event = if std::mem::replace(&mut *lock(&self.new_stream), false) {
                MENewStream
            } else {
                MEUpdatedStream
            };
            // SAFETY: 自分のイベントの列に、ストリームを値にしたイベントを積む。
            unsafe {
                self.queue
                    .QueueEventParamUnk(event.0 as u32, &GUID::zeroed(), S_OK, &stream)
            }?;
            self.stream
                .set_state(MF_STREAM_STATE_RUNNING, MEStreamStarted)?;
            self.queue_empty(MESourceStarted)
        })
    }

    fn Stop(&self) -> Result<()> {
        guard(|| {
            self.check()?;
            self.stream
                .set_state(MF_STREAM_STATE_STOPPED, MEStreamStopped)?;
            self.queue_empty(MESourceStopped)
        })
    }

    fn Pause(&self) -> Result<()> {
        // 生の映像は一時停止できない。
        Err(MF_E_INVALID_STATE_TRANSITION.into())
    }

    fn Shutdown(&self) -> Result<()> {
        guard(|| {
            *lock(&self.shutdown) = true;
            self.stream.shutdown();
            // SAFETY: 自分のイベントの列を閉じる。
            unsafe { self.queue.Shutdown() }
        })
    }
}

impl IMFMediaSourceEx_Impl for Source_Impl {
    fn GetSourceAttributes(&self) -> Result<IMFAttributes> {
        guard(|| {
            self.check()?;
            Ok(self.attributes.clone())
        })
    }

    fn GetStreamAttributes(&self, id: u32) -> Result<IMFAttributes> {
        guard(|| {
            self.check()?;
            if id != STREAM_ID {
                return Err(MF_E_INVALIDSTREAMNUMBER.into());
            }
            self.stream.descriptor.cast()
        })
    }

    fn SetD3DManager(&self, _manager: Ref<IUnknown>) -> Result<()> {
        // 画は CPU のメモリで渡すので、D3D の機器は使わない。
        Ok(())
    }
}

impl IMFMediaSource2_Impl for Source_Impl {
    fn SetMediaType(&self, id: u32, media_type: Ref<IMFMediaType>) -> Result<()> {
        guard(|| {
            self.check()?;
            if id != STREAM_ID {
                return Err(MF_E_INVALIDSTREAMNUMBER.into());
            }
            // SAFETY: 自分のストリーム記述子の今の media type を、呼び出し元が選んだもの (出した組のどれか) にする。
            unsafe {
                self.stream
                    .descriptor
                    .GetMediaTypeHandler()?
                    .SetCurrentMediaType(media_type.as_ref())
            }
        })
    }
}

impl IMFGetService_Impl for Source_Impl {
    fn GetService(
        &self,
        _service: *const GUID,
        _riid: *const GUID,
        _object: *mut *mut c_void,
    ) -> Result<()> {
        Err(MF_E_UNSUPPORTED_SERVICE.into())
    }
}

impl IKsControl_Impl for Source_Impl {
    fn KsProperty(
        &self,
        _: *const KSIDENTIFIER,
        _: u32,
        _: *mut c_void,
        _: u32,
        _: *mut u32,
    ) -> Result<()> {
        Err(ERROR_SET_NOT_FOUND.to_hresult().into())
    }

    fn KsMethod(
        &self,
        _: *const KSIDENTIFIER,
        _: u32,
        _: *mut c_void,
        _: u32,
        _: *mut u32,
    ) -> Result<()> {
        Err(ERROR_SET_NOT_FOUND.to_hresult().into())
    }

    fn KsEvent(
        &self,
        _: *const KSIDENTIFIER,
        _: u32,
        _: *mut c_void,
        _: u32,
        _: *mut u32,
    ) -> Result<()> {
        Err(ERROR_SET_NOT_FOUND.to_hresult().into())
    }
}

/// Frame Server に渡す IMFActivate。属性は内側の IMFAttributes に預け、`ActivateObject` でメディアソースを作る。
#[implement(IMFActivate)]
struct Activate {
    attributes: IMFAttributes,
    source: Mutex<Option<IMFMediaSource2>>,
}

impl Activate {
    fn new() -> Result<Self> {
        let mut attributes = None;
        // SAFETY: 新しい属性の入れ物を作るだけ。
        unsafe { MFCreateAttributes(&mut attributes, 1) }?;
        Ok(Self {
            attributes: attributes.ok_or(E_FAIL)?,
            source: Mutex::new(None),
        })
    }
}

impl IMFActivate_Impl for Activate_Impl {
    fn ActivateObject(&self, riid: *const GUID, ppv: *mut *mut c_void) -> Result<()> {
        guard(|| {
            if riid.is_null() || ppv.is_null() {
                return Err(E_POINTER.into());
            }
            let mut source = lock(&self.source);
            if source.is_none() {
                *source = Some(Source::create(&self.attributes)?);
            }
            let source = source.as_ref().ok_or(E_FAIL)?;
            // SAFETY: `riid` と `ppv` は NULL でないことを確かめた、呼び出し元の有効なポインタ。
            unsafe { source.query(riid, ppv) }.ok()
        })
    }

    fn ShutdownObject(&self) -> Result<()> {
        guard(|| {
            if let Some(source) = lock(&self.source).take() {
                // SAFETY: 作ったメディアソースを閉じる。
                unsafe { source.Shutdown() }?;
            }
            Ok(())
        })
    }

    fn DetachObject(&self) -> Result<()> {
        *lock(&self.source) = None;
        Ok(())
    }
}

/// IMFAttributes は内側の入れ物にそのまま渡す。
impl IMFAttributes_Impl for Activate_Impl {
    fn GetItem(&self, key: *const GUID, value: *mut PROPVARIANT) -> Result<()> {
        // SAFETY: 引数は呼び出し元 (COM の約束で有効なポインタ) のものを、内側の入れ物にそのまま渡す。以下の転送も同じ。
        unsafe {
            self.attributes
                .GetItem(key, (!value.is_null()).then_some(value))
        }
    }
    fn GetItemType(&self, key: *const GUID) -> Result<MF_ATTRIBUTE_TYPE> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.GetItemType(key) }
    }
    fn CompareItem(&self, key: *const GUID, value: *const PROPVARIANT) -> Result<BOOL> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.CompareItem(key, value) }
    }
    fn Compare(
        &self,
        theirs: Ref<IMFAttributes>,
        match_type: MF_ATTRIBUTES_MATCH_TYPE,
    ) -> Result<BOOL> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.Compare(theirs.as_ref(), match_type) }
    }
    fn GetUINT32(&self, key: *const GUID) -> Result<u32> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.GetUINT32(key) }
    }
    fn GetUINT64(&self, key: *const GUID) -> Result<u64> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.GetUINT64(key) }
    }
    fn GetDouble(&self, key: *const GUID) -> Result<f64> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.GetDouble(key) }
    }
    fn GetGUID(&self, key: *const GUID) -> Result<GUID> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.GetGUID(key) }
    }
    fn GetStringLength(&self, key: *const GUID) -> Result<u32> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.GetStringLength(key) }
    }
    fn GetString(&self, key: *const GUID, value: PWSTR, size: u32, length: *mut u32) -> Result<()> {
        let vtable = self.attributes.vtable();
        // SAFETY: 転送 (`GetItem` と同じ)。`value` は `size` 文字の領域で、生の関数にそのまま渡す。
        unsafe { (vtable.GetString)(self.attributes.as_raw(), key, value, size, length) }.ok()
    }
    fn GetAllocatedString(
        &self,
        key: *const GUID,
        value: *mut PWSTR,
        length: *mut u32,
    ) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.GetAllocatedString(key, value, length) }
    }
    fn GetBlobSize(&self, key: *const GUID) -> Result<u32> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.GetBlobSize(key) }
    }
    fn GetBlob(
        &self,
        key: *const GUID,
        buffer: *mut u8,
        size: u32,
        blob_size: *mut u32,
    ) -> Result<()> {
        let vtable = self.attributes.vtable();
        // SAFETY: 転送 (`GetItem` と同じ)。`buffer` は `size` バイトの領域で、生の関数にそのまま渡す。
        unsafe { (vtable.GetBlob)(self.attributes.as_raw(), key, buffer, size, blob_size) }.ok()
    }
    fn GetAllocatedBlob(
        &self,
        key: *const GUID,
        buffer: *mut *mut u8,
        size: *mut u32,
    ) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.GetAllocatedBlob(key, buffer, size) }
    }
    fn GetUnknown(&self, key: *const GUID, riid: *const GUID, ppv: *mut *mut c_void) -> Result<()> {
        let vtable = self.attributes.vtable();
        // SAFETY: 転送 (`GetItem` と同じ)。生の関数にそのまま渡す。
        unsafe { (vtable.GetUnknown)(self.attributes.as_raw(), key, riid, ppv) }.ok()
    }
    fn SetItem(&self, key: *const GUID, value: *const PROPVARIANT) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.SetItem(key, value) }
    }
    fn DeleteItem(&self, key: *const GUID) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.DeleteItem(key) }
    }
    fn DeleteAllItems(&self) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.DeleteAllItems() }
    }
    fn SetUINT32(&self, key: *const GUID, value: u32) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.SetUINT32(key, value) }
    }
    fn SetUINT64(&self, key: *const GUID, value: u64) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.SetUINT64(key, value) }
    }
    fn SetDouble(&self, key: *const GUID, value: f64) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.SetDouble(key, value) }
    }
    fn SetGUID(&self, key: *const GUID, value: *const GUID) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.SetGUID(key, value) }
    }
    fn SetString(&self, key: *const GUID, value: &PCWSTR) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.SetString(key, *value) }
    }
    fn SetBlob(&self, key: *const GUID, buffer: *const u8, size: u32) -> Result<()> {
        let vtable = self.attributes.vtable();
        // SAFETY: 転送 (`GetItem` と同じ)。`buffer` は `size` バイトの領域で、生の関数にそのまま渡す。
        unsafe { (vtable.SetBlob)(self.attributes.as_raw(), key, buffer, size) }.ok()
    }
    fn SetUnknown(&self, key: *const GUID, value: Ref<IUnknown>) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.SetUnknown(key, value.as_ref()) }
    }
    fn LockStore(&self) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.LockStore() }
    }
    fn UnlockStore(&self) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.UnlockStore() }
    }
    fn GetCount(&self) -> Result<u32> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.GetCount() }
    }
    fn GetItemByIndex(&self, index: u32, key: *mut GUID, value: *mut PROPVARIANT) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe {
            self.attributes
                .GetItemByIndex(index, key, (!value.is_null()).then_some(value))
        }
    }
    fn CopyAllItems(&self, dest: Ref<IMFAttributes>) -> Result<()> {
        // SAFETY: 転送 (`GetItem` と同じ)。
        unsafe { self.attributes.CopyAllItems(dest.as_ref()) }
    }
}

/// クラスファクトリ (Activate を作る)。
#[implement(IClassFactory)]
struct Factory;

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> Result<()> {
        guard(|| {
            if riid.is_null() || ppv.is_null() {
                return Err(E_POINTER.into());
            }
            if !outer.is_null() {
                return Err(CLASS_E_NOAGGREGATION.into());
            }
            let activate: IMFActivate = Activate::new()?.into();
            // SAFETY: `riid` と `ppv` は NULL でないことを確かめた、呼び出し元の有効なポインタ。
            unsafe { activate.query(riid, ppv) }.ok()
        })
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        Ok(())
    }
}

/// COM の入口: クラスファクトリを返す。
///
/// # Safety
///
/// COM の約束どおり、`clsid`・`riid` は有効な GUID を、`ppv` は書ける領域を指すこと (NULL は E_POINTER)。
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllGetClassObject(
    clsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if clsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_POINTER;
    }
    // SAFETY: `ppv` は NULL でない、呼び出し元の書ける領域 (関数の約束)。
    unsafe { *ppv = std::ptr::null_mut() };
    // SAFETY: `clsid` は NULL でない、呼び出し元の有効な GUID (関数の約束)。
    if unsafe { *clsid } != CLSID_SOURCE {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let factory: IClassFactory = Factory.into();
    // SAFETY: `riid` と `ppv` は NULL でない、呼び出し元の有効なポインタ (関数の約束)。
    unsafe { factory.query(riid, ppv) }
}

/// COM の入口: DLL を外してよいか。サービスの中で外されて作り直されるのを避けるため、いつも「外さない」。
#[unsafe(no_mangle)]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

/// この DLL のパス。
fn module_path() -> Result<String> {
    let mut module = HMODULE::default();
    // SAFETY: この関数のアドレスから、それを含む (この) DLL のハンドルを参照を増やさずに得る。
    unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(module_path as *const u16),
            &mut module,
        )
    }?;
    let mut path = [0u16; 1024];
    // SAFETY: `module` は上で得たこの DLL のハンドル。`path` は書ける領域で、長さを超えて書かれない。
    let len = unsafe { GetModuleFileNameW(Some(module), &mut path) } as usize;
    if len == 0 || len >= path.len() {
        return Err(windows_core::Error::from_thread());
    }
    Ok(String::from_utf16_lossy(&path[..len]))
}

/// `regsvr32` の入口: CLSID と DLL のパスを HKLM に書く (インストーラが管理者の権限で呼ぶ。ADR 0028 の 8.)。
#[unsafe(no_mangle)]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    guard(|| {
        let path = module_path()?;
        let key_name = HSTRING::from(clsid_key());
        let mut key = HKEY::default();
        // SAFETY: HKLM の下にキーを作って開く (閉じるのは下)。名前は呼び出しの間生きている HSTRING。
        unsafe {
            RegCreateKeyExW(
                HKEY_LOCAL_MACHINE,
                &key_name,
                None,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut key,
                None,
            )
        }
        .ok()?;
        let set = |name: PCWSTR, value: &str| -> Result<()> {
            // REG_SZ の値は NUL で終わる UTF-16 のバイト列。
            let data: Vec<u8> = value
                .encode_utf16()
                .chain([0])
                .flat_map(u16::to_le_bytes)
                .collect();
            // SAFETY: `key` は上で開いたキーで、`data` は呼び出しの間生きている。
            unsafe { RegSetValueExW(key, name, None, REG_SZ, Some(&data)) }.ok()
        };
        let result = set(PCWSTR::null(), &path).and_then(|()| set(w!("ThreadingModel"), "Both"));
        // SAFETY: 上で開いたキーを一度だけ閉じる。
        let _ = unsafe { RegCloseKey(key) };
        result
    })
    .map_or_else(|e| e.code(), |()| S_OK)
}

/// `regsvr32 /u` の入口: CLSID のキーを消す (アンインストーラが呼ぶ)。
#[unsafe(no_mangle)]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    let key = HSTRING::from(format!(
        "SOFTWARE\\Classes\\CLSID\\{}",
        super::clsid_string()
    ));
    // SAFETY: HKLM の下の、この DLL の CLSID のキーだけを消す。名前は呼び出しの間生きている HSTRING。
    let result = unsafe { RegDeleteTreeW(HKEY_LOCAL_MACHINE, &key) };
    if result.is_ok() || result == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND {
        S_OK
    } else {
        result.to_hresult()
    }
}
