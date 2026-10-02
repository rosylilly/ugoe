//! macOS: CoreMediaIO の Camera Extension (`apps/studio/vcam/macos`、`docs/studio.md#macos-camera-extension`、T0809)。
//!
//! 拡張 (System Extension) はスタジオのアプリの束の `Contents/Library/SystemExtensions/` に入れて配る。スタジオは
//!
//! 1. 最初に始めるときに拡張を有効にし (`OSSystemExtensionRequest`。初回は利用者が「システム設定」で許可する)、
//! 2. 拡張の機器「ugoe studio」の受ける側 (sink) のストリームを開いて、NV12 ('420v') の画を `CMSampleBuffer` で積む。
//!
//! 拡張は、カメラを開いているアプリがある間だけ受ける側から取り出す。キューが埋まっていれば「受け取るアプリがいない」
//! ([`Delivery::NoConsumer`])。機器とストリームの名前・ID は `macos/Extension/Camera.swift` の `Ids` と合わせる。

use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr::{self, NonNull};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AnyThread, define_class, msg_send};
use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_media::{
    CMClock, CMSampleBuffer, CMSampleTimingInfo, CMSimpleQueue, CMTime, CMVideoFormatDescription,
    CMVideoFormatDescriptionCreateForImageBuffer, kCMTimeInvalid,
};
use objc2_core_media_io::{
    CMIODeviceStartStream, CMIODeviceStopStream, CMIOObjectGetPropertyData,
    CMIOObjectGetPropertyDataSize, CMIOObjectID, CMIOObjectPropertyAddress,
    CMIOStreamCopyBufferQueue, kCMIODevicePropertyDeviceUID, kCMIODevicePropertyStreams,
    kCMIOHardwareNoError, kCMIOHardwarePropertyDevices, kCMIOObjectPropertyElementMain,
    kCMIOObjectPropertyName, kCMIOObjectPropertyScopeGlobal, kCMIOObjectSystemObject,
};
use objc2_core_video::{
    CVAttachmentMode, CVPixelBuffer, CVPixelBufferGetBaseAddressOfPlane,
    CVPixelBufferGetBytesPerRowOfPlane, CVPixelBufferGetHeightOfPlane,
    CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags, CVPixelBufferPool,
    CVPixelBufferUnlockBaseAddress, kCVImageBufferColorPrimaries_ITU_R_709_2,
    kCVImageBufferColorPrimariesKey, kCVImageBufferTransferFunction_ITU_R_709_2,
    kCVImageBufferTransferFunctionKey, kCVImageBufferYCbCrMatrix_ITU_R_709_2,
    kCVImageBufferYCbCrMatrixKey, kCVPixelBufferHeightKey, kCVPixelBufferIOSurfacePropertiesKey,
    kCVPixelBufferPixelFormatTypeKey, kCVPixelBufferWidthKey,
    kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange, kCVReturnSuccess,
};
use objc2_foundation::{
    NSError, NSObject, NSObjectProtocol, NSOperatingSystemVersion, NSProcessInfo, NSString,
};
use objc2_system_extensions::{
    OSSystemExtensionErrorCode, OSSystemExtensionErrorDomain, OSSystemExtensionManager,
    OSSystemExtensionProperties, OSSystemExtensionReplacementAction, OSSystemExtensionRequest,
    OSSystemExtensionRequestDelegate, OSSystemExtensionRequestResult,
};

use crate::{Config, Delivery, Error};

/// 拡張の束の ID (`macos/Info.plist` の `CFBundleIdentifier`)。
const EXTENSION_ID: &str = "app.ugoe.studio.camera";
/// 機器の ID (`Ids.device`)。CoreMediaIO では機器の UID になる。
const DEVICE_UID: &str = "07DA1CFA-007F-4E78-873E-5FFA646E00B4";
/// 受ける側のストリームの名前 (`Ids.sinkName`)。
const SINK_NAME: &str = "ugoe studio input";
/// カメラの名前 (`Ids.name`)。
const CAMERA_NAME: &str = "ugoe studio";
/// Camera Extension が使える最も古い macOS。
const MINIMUM_OS: NSOperatingSystemVersion = NSOperatingSystemVersion {
    majorVersion: 12,
    minorVersion: 3,
    patchVersion: 0,
};
/// 有効にする求めの結果を待つ長さ (利用者の許可は待たない)。
const ACTIVATION_TIMEOUT: Duration = Duration::from_secs(10);
/// 有効にした後に、機器が CoreMediaIO に現れるのを待つ長さ。
const DEVICE_TIMEOUT: Duration = Duration::from_secs(5);

/// 始められるか。macOS の版と、アプリの束に拡張が入っているか (開発中の `just studio` と、
/// 拡張を入れていない配布物では入っていない) だけを見る。有効になっているかは始めるときに確かめる。
pub(crate) fn availability() -> Result<(), Error> {
    if !NSProcessInfo::processInfo().isOperatingSystemAtLeastVersion(MINIMUM_OS) {
        return Err(Error::MacosTooOld);
    }
    match bundled_extension() {
        Some(path) if path.is_dir() => Ok(()),
        _ => Err(Error::Unsupported),
    }
}

/// アプリの束の中の拡張の場所 (`<アプリ>.app/Contents/Library/SystemExtensions/<ID>.systemextension`)。
fn bundled_extension() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let contents = exe.parent()?.parent()?;
    Some(
        contents
            .join("Library")
            .join("SystemExtensions")
            .join(format!("{EXTENSION_ID}.systemextension")),
    )
}

/// 動いている仮想カメラ (拡張の受ける側を開いている)。
#[derive(Debug)]
pub(crate) struct Camera {
    config: Config,
    device: CMIOObjectID,
    stream: CMIOObjectID,
    queue: CFRetained<CMSimpleQueue>,
    pool: CFRetained<CVPixelBufferPool>,
}

// SAFETY: CMSimpleQueue と CVPixelBufferPool はスレッドをまたいで使える (CoreMedia と CoreVideo の決まり。CMSimpleQueue は
// 1 つの書き手と 1 つの読み手のときにロック無しで安全)。`Camera` は仮想カメラのスレッドだけが持ち、書き手は 1 つ。
unsafe impl Send for Camera {}

impl Camera {
    pub(crate) fn start(config: Config) -> Result<Self, Error> {
        availability()?;
        // プロセスで最初に始めるときは、機器があっても求める (スタジオを更新したら拡張も入れ替えるため)。
        let activated = activate(false);
        let device = match find_device() {
            Some(device) => device,
            None => {
                activated?;
                // 有効になっているはずなのに機器が無い (利用者が拡張を外したなど) ので、求め直す。
                activate(true)?;
                wait_for_device()?
            }
        };
        let stream = find_sink(device)?;
        let mut queue: *mut CMSimpleQueue = ptr::null_mut();
        // SAFETY: `stream` は CoreMediaIO から得たストリームの ID。キューが変わったことの知らせは受けない (None と null)。
        // 成功すると `queue` に、呼び手が解放する (Copy の規則) キューが入る。
        let status =
            unsafe { CMIOStreamCopyBufferQueue(stream, None, ptr::null_mut(), &mut queue) };
        check(status, "CMIOStreamCopyBufferQueue")?;
        let queue = NonNull::new(queue)
            .ok_or_else(|| Error::os("CMIOStreamCopyBufferQueue", "no queue"))?;
        // SAFETY: Copy の規則で得た +1 の参照なので、`from_raw` で所有権を受け取る。
        let queue = unsafe { CFRetained::from_raw(queue) };
        let pool = pixel_buffer_pool(config)?;
        // SAFETY: `device` と `stream` は CoreMediaIO から得た ID。止めるのは `Drop`。
        let status = unsafe { CMIODeviceStartStream(device, stream) };
        check(status, "CMIODeviceStartStream")?;
        Ok(Self {
            config,
            device,
            stream,
            queue,
            pool,
        })
    }

    pub(crate) fn device(&self) -> String {
        CAMERA_NAME.to_owned()
    }

    /// NV12 の画を 1 枚積む。キューが埋まっていれば (拡張が取り出していない = カメラを開いているアプリが無い) 積まない。
    pub(crate) fn send(&mut self, frame: &[u8]) -> Result<Delivery, Error> {
        // SAFETY: `queue` は生きているキュー。
        let (count, capacity) = unsafe { (self.queue.count(), self.queue.capacity()) };
        if capacity > 0 && count >= capacity {
            return Ok(Delivery::NoConsumer);
        }
        let pixels = self.pixel_buffer(frame)?;
        let sample = sample_buffer(&pixels, self.config.fps)?;
        // 積んだ `CMSampleBuffer` は、取り出した側 (CoreMediaIO) が解放する。
        let element = CFRetained::into_raw(sample).cast::<c_void>();
        // SAFETY: `element` は +1 の参照を持つ `CMSampleBuffer`。積めれば所有権はキューへ移る。
        let status = unsafe { self.queue.enqueue(element) };
        if status != 0 {
            // SAFETY: 積めなかったので、所有権は手元に残っている。取り戻して解放する。
            drop(unsafe { CFRetained::from_raw(element.cast::<CMSampleBuffer>()) });
            return Ok(Delivery::NoConsumer);
        }
        Ok(Delivery::Delivered)
    }

    /// 画を置き場の `CVPixelBuffer` (IOSurface) に写す。行の幅は CoreVideo が揃えるので、行ごとに写す。
    fn pixel_buffer(&self, frame: &[u8]) -> Result<CFRetained<CVPixelBuffer>, Error> {
        let mut out: *mut CVPixelBuffer = ptr::null_mut();
        // SAFETY: `pool` は生きている置き場。`out` は書き込める場所。
        let status = unsafe {
            CVPixelBufferPool::create_pixel_buffer(None, &self.pool, NonNull::from(&mut out))
        };
        if status != kCVReturnSuccess {
            return Err(Error::os("CVPixelBufferPoolCreatePixelBuffer", status));
        }
        let out = NonNull::new(out)
            .ok_or_else(|| Error::os("CVPixelBufferPoolCreatePixelBuffer", "no buffer"))?;
        // SAFETY: Create の規則で得た +1 の参照。
        let buffer = unsafe { CFRetained::from_raw(out) };
        let width = self.config.width as usize;
        let height = self.config.height as usize;
        let (luma, chroma) = frame.split_at(width * height);
        // SAFETY: `buffer` は生きている画。写し終えるまでロックする。
        let status = unsafe { CVPixelBufferLockBaseAddress(&buffer, CVPixelBufferLockFlags(0)) };
        if status != kCVReturnSuccess {
            return Err(Error::os("CVPixelBufferLockBaseAddress", status));
        }
        // Y の面 (幅 × 高さ) と、Cb Cr を交互に並べた面 (幅 × 高さ / 2)。どちらも 1 行は `width` バイト。
        for (plane, data) in [(0, luma), (1, chroma)] {
            let base = CVPixelBufferGetBaseAddressOfPlane(&buffer, plane).cast::<u8>();
            let stride = CVPixelBufferGetBytesPerRowOfPlane(&buffer, plane);
            let rows = CVPixelBufferGetHeightOfPlane(&buffer, plane);
            if base.is_null() || stride < width || rows * width != data.len() {
                // SAFETY: 上でロックした画。
                unsafe { CVPixelBufferUnlockBaseAddress(&buffer, CVPixelBufferLockFlags(0)) };
                return Err(Error::os("CVPixelBuffer", "unexpected plane layout"));
            }
            for (row, line) in data.chunks_exact(width).enumerate() {
                // SAFETY: ロックしている間、面は `stride * rows` バイト書ける。`row < rows` で `width <= stride`。
                unsafe {
                    ptr::copy_nonoverlapping(line.as_ptr(), base.add(row * stride), width);
                }
            }
        }
        // SAFETY: 上でロックした画。
        unsafe { CVPixelBufferUnlockBaseAddress(&buffer, CVPixelBufferLockFlags(0)) };
        set_color_attachments(&buffer);
        Ok(buffer)
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        // SAFETY: `start` で始めたストリーム。失敗しても (拡張が先に終わったなど) することは無い。
        unsafe { CMIODeviceStopStream(self.device, self.stream) };
    }
}

fn check(status: i32, context: &'static str) -> Result<(), Error> {
    if status == kCMIOHardwareNoError as i32 {
        Ok(())
    } else {
        Err(Error::os(context, format!("OSStatus {status}")))
    }
}

fn address(selector: u32) -> CMIOObjectPropertyAddress {
    CMIOObjectPropertyAddress {
        mSelector: selector,
        mScope: kCMIOObjectPropertyScopeGlobal,
        mElement: kCMIOObjectPropertyElementMain,
    }
}

/// `object` の、ID の並びの属性 (機器の一覧・ストリームの一覧)。
fn object_ids(object: CMIOObjectID, selector: u32) -> Vec<CMIOObjectID> {
    let address = address(selector);
    let mut size = 0u32;
    // SAFETY: `address` と `size` は書き込める場所。修飾のデータは無い (0 と null)。
    let status =
        unsafe { CMIOObjectGetPropertyDataSize(object, &address, 0, ptr::null(), &mut size) };
    if status != kCMIOHardwareNoError as i32 || size == 0 {
        return Vec::new();
    }
    let mut ids = vec![0 as CMIOObjectID; size as usize / size_of::<CMIOObjectID>()];
    let mut used = 0u32;
    // SAFETY: `ids` は `size` バイト以上ある (要素の大きさで割って切り捨てた数なので、書けるのは `ids` の範囲まで)。
    let status = unsafe {
        CMIOObjectGetPropertyData(
            object,
            &address,
            0,
            ptr::null(),
            (ids.len() * size_of::<CMIOObjectID>()) as u32,
            &mut used,
            ids.as_mut_ptr().cast(),
        )
    };
    if status != kCMIOHardwareNoError as i32 {
        return Vec::new();
    }
    ids.truncate(used as usize / size_of::<CMIOObjectID>());
    ids
}

/// `object` の、文字列の属性 (機器の UID・ストリームの名前)。
fn object_string(object: CMIOObjectID, selector: u32) -> Option<String> {
    let address = address(selector);
    let mut value: *const CFString = ptr::null();
    let mut used = 0u32;
    // SAFETY: 文字列の属性は CFStringRef を 1 つ書き、呼び手が解放する (+1)。`value` はその大きさ。
    let status = unsafe {
        CMIOObjectGetPropertyData(
            object,
            &address,
            0,
            ptr::null(),
            size_of::<*const CFString>() as u32,
            &mut used,
            (&raw mut value).cast(),
        )
    };
    if status != kCMIOHardwareNoError as i32 {
        return None;
    }
    let value = NonNull::new(value.cast_mut())?;
    // SAFETY: +1 の参照を受け取る。
    let value = unsafe { CFRetained::from_raw(value) };
    Some(value.to_string())
}

/// 拡張の機器 (UID で探す)。
fn find_device() -> Option<CMIOObjectID> {
    object_ids(kCMIOObjectSystemObject, kCMIOHardwarePropertyDevices)
        .into_iter()
        .find(|&device| {
            object_string(device, kCMIODevicePropertyDeviceUID).as_deref() == Some(DEVICE_UID)
        })
}

fn wait_for_device() -> Result<CMIOObjectID, Error> {
    let deadline = Instant::now() + DEVICE_TIMEOUT;
    loop {
        if let Some(device) = find_device() {
            return Ok(device);
        }
        if Instant::now() >= deadline {
            return Err(Error::ExtensionFailed(
                "the camera extension is active but its device did not appear".to_owned(),
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// 受ける側のストリーム (名前で探す)。
fn find_sink(device: CMIOObjectID) -> Result<CMIOObjectID, Error> {
    object_ids(device, kCMIODevicePropertyStreams)
        .into_iter()
        .find(|&stream| {
            object_string(stream, kCMIOObjectPropertyName).as_deref() == Some(SINK_NAME)
        })
        .ok_or_else(|| {
            Error::ExtensionFailed("the camera extension has no input stream".to_owned())
        })
}

/// 設定の大きさの NV12 (BT.709・限定範囲) の画の置き場。拡張へ渡すので IOSurface にする。
fn pixel_buffer_pool(config: Config) -> Result<CFRetained<CVPixelBufferPool>, Error> {
    let format = CFNumber::new_i32(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange as i32);
    let width = CFNumber::new_i32(config.width as i32);
    let height = CFNumber::new_i32(config.height as i32);
    let surface = CFDictionary::<CFString, CFType>::empty();
    // SAFETY: CoreVideo の定数の文字列は、読み込まれた後は変わらない。
    let keys: [&CFString; 4] = unsafe {
        [
            kCVPixelBufferPixelFormatTypeKey,
            kCVPixelBufferWidthKey,
            kCVPixelBufferHeightKey,
            kCVPixelBufferIOSurfacePropertiesKey,
        ]
    };
    let values: [&CFType; 4] = [&format, &width, &height, &surface];
    let attributes = CFDictionary::from_slices(&keys, &values);
    let mut pool: *mut CVPixelBufferPool = ptr::null_mut();
    // SAFETY: 属性の辞書は CFString から CFNumber / CFDictionary への辞書 (CoreVideo が求める型)。`pool` は書き込める場所。
    let status = unsafe {
        CVPixelBufferPool::create(
            None,
            None,
            Some(attributes.as_opaque()),
            NonNull::from(&mut pool),
        )
    };
    if status != kCVReturnSuccess {
        return Err(Error::os("CVPixelBufferPoolCreate", status));
    }
    let pool = NonNull::new(pool).ok_or_else(|| Error::os("CVPixelBufferPoolCreate", "no pool"))?;
    // SAFETY: Create の規則で得た +1 の参照。
    Ok(unsafe { CFRetained::from_raw(pool) })
}

/// 画が BT.709 であることを付ける (受け取ったアプリが色を正しく戻せるように)。
fn set_color_attachments(buffer: &CVPixelBuffer) {
    // SAFETY: CoreVideo の定数の文字列。値はどれも CFString で、それぞれのキーが求める型。
    unsafe {
        for (key, value) in [
            (
                kCVImageBufferYCbCrMatrixKey,
                kCVImageBufferYCbCrMatrix_ITU_R_709_2,
            ),
            (
                kCVImageBufferColorPrimariesKey,
                kCVImageBufferColorPrimaries_ITU_R_709_2,
            ),
            (
                kCVImageBufferTransferFunctionKey,
                kCVImageBufferTransferFunction_ITU_R_709_2,
            ),
        ] {
            buffer.set_attachment(key, value, CVAttachmentMode::ShouldPropagate);
        }
    }
}

/// 画を、今の時刻 (ホストの時計) の 1 枚の `CMSampleBuffer` にする。
fn sample_buffer(buffer: &CVPixelBuffer, fps: u32) -> Result<CFRetained<CMSampleBuffer>, Error> {
    let mut format: *const CMVideoFormatDescription = ptr::null();
    // SAFETY: `buffer` は生きている画。`format` は書き込める場所。
    let status = unsafe {
        CMVideoFormatDescriptionCreateForImageBuffer(None, buffer, NonNull::from(&mut format))
    };
    if status != 0 {
        return Err(Error::os(
            "CMVideoFormatDescriptionCreateForImageBuffer",
            status,
        ));
    }
    let format = NonNull::new(format.cast_mut())
        .ok_or_else(|| Error::os("CMVideoFormatDescriptionCreateForImageBuffer", "no format"))?;
    // SAFETY: Create の規則で得た +1 の参照。
    let format = unsafe { CFRetained::from_raw(format) };
    // SAFETY: ホストの時計は常にある。`kCMTimeInvalid` は CoreMedia の定数。
    let (now, invalid) = unsafe { (CMClock::host_time_clock().time(), kCMTimeInvalid) };
    let mut timing = CMSampleTimingInfo {
        duration: CMTime {
            value: 1,
            timescale: fps as i32,
            flags: now.flags,
            epoch: 0,
        },
        presentationTimeStamp: now,
        decodeTimeStamp: invalid,
    };
    let mut sample: *mut CMSampleBuffer = ptr::null_mut();
    // SAFETY: 画・書式・時刻は生きている。`sample` は書き込める場所。
    let status = unsafe {
        CMSampleBuffer::create_ready_with_image_buffer(
            None,
            buffer,
            &format,
            NonNull::from(&mut timing),
            NonNull::from(&mut sample),
        )
    };
    if status != 0 {
        return Err(Error::os(
            "CMSampleBufferCreateReadyWithImageBuffer",
            status,
        ));
    }
    let sample = NonNull::new(sample)
        .ok_or_else(|| Error::os("CMSampleBufferCreateReadyWithImageBuffer", "no sample"))?;
    // SAFETY: Create の規則で得た +1 の参照。
    Ok(unsafe { CFRetained::from_raw(sample) })
}

/// 拡張を有効にする求めの状態。プロセスで 1 つ。
#[derive(Debug, Clone)]
enum Activation {
    /// まだ求めていない (または前の求めが失敗した)。
    Idle,
    /// 求めて、結果を待っている。
    Pending,
    /// 利用者の許可を待っている (許可されると `Done` になる)。
    NeedsApproval,
    /// 有効になった。
    Done,
    /// 失敗した (次に始めるときに求め直す)。
    Failed(Failure),
}

#[derive(Debug, Clone)]
enum Failure {
    /// アプリが「アプリケーション」フォルダに無い。
    NotInApplications,
    Other(String),
}

static ACTIVATION: Mutex<Activation> = Mutex::new(Activation::Idle);
static ACTIVATION_CHANGED: Condvar = Condvar::new();

fn set_activation(state: Activation) {
    if let Ok(mut current) = ACTIVATION.lock() {
        *current = state;
        ACTIVATION_CHANGED.notify_all();
    }
}

/// 拡張を有効にする。すでに有効ならすぐに終わる (版が違えば入れ替える)。利用者の許可が要れば待たずに理由を返す。
/// このプロセスで有効にできていれば、`again` のときだけ求め直す。
fn activate(again: bool) -> Result<(), Error> {
    fn lock_error<T>(_: T) -> Error {
        Error::os("activation", "lock poisoned")
    }
    let mut state = ACTIVATION.lock().map_err(lock_error)?;
    let submit = match *state {
        Activation::Idle | Activation::Failed(_) => true,
        Activation::Done => again,
        Activation::Pending | Activation::NeedsApproval => false,
    };
    if submit {
        *state = Activation::Pending;
        submit_activation();
    }
    let deadline = Instant::now() + ACTIVATION_TIMEOUT;
    while matches!(*state, Activation::Pending) {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(Error::ExtensionFailed(
                "the activation request did not finish in time".to_owned(),
            ));
        }
        state = ACTIVATION_CHANGED
            .wait_timeout(state, left)
            .map_err(lock_error)?
            .0;
    }
    match &*state {
        Activation::Done => Ok(()),
        Activation::NeedsApproval => Err(Error::NeedsApproval),
        Activation::Failed(Failure::NotInApplications) => Err(Error::NotInApplications),
        Activation::Failed(Failure::Other(detail)) => Err(Error::ExtensionFailed(detail.clone())),
        Activation::Idle | Activation::Pending => Err(Error::os("activation", "unexpected state")),
    }
}

fn submit_activation() {
    let queue = DispatchQueue::new("app.ugoe.studio.camera.activation", None);
    let identifier = NSString::from_str(EXTENSION_ID);
    // SAFETY: 引数はどれも生きている。delegate のメソッドは `queue` の上で呼ばれ、状態の更新はロックで守る。
    unsafe {
        let request =
            OSSystemExtensionRequest::activationRequestForExtension_queue(&identifier, &queue);
        let delegate = ActivationDelegate::new();
        request.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        OSSystemExtensionManager::sharedManager().submitRequest(&request);
        // delegate は弱い参照で持たれるので、求めが終わるまで生かしておく。求めはプロセスで数回なので、手放さずに残す。
        std::mem::forget(delegate);
    }
}

define_class!(
    // SAFETY: NSObject を継承するときの決まりは無い。`Drop` を実装しない。
    #[unsafe(super(NSObject))]
    #[name = "UgoeStudioCameraActivationDelegate"]
    struct ActivationDelegate;

    unsafe impl NSObjectProtocol for ActivationDelegate {}

    unsafe impl OSSystemExtensionRequestDelegate for ActivationDelegate {
        #[unsafe(method(request:actionForReplacingExtension:withExtension:))]
        fn replace(
            &self,
            _request: &OSSystemExtensionRequest,
            _existing: &OSSystemExtensionProperties,
            _extension: &OSSystemExtensionProperties,
        ) -> OSSystemExtensionReplacementAction {
            // スタジオに入っている版へ入れ替える (更新したとき)。
            OSSystemExtensionReplacementAction::Replace
        }

        #[unsafe(method(requestNeedsUserApproval:))]
        fn needs_approval(&self, _request: &OSSystemExtensionRequest) {
            set_activation(Activation::NeedsApproval);
        }

        #[unsafe(method(request:didFinishWithResult:))]
        fn finished(
            &self,
            _request: &OSSystemExtensionRequest,
            result: OSSystemExtensionRequestResult,
        ) {
            set_activation(if result == OSSystemExtensionRequestResult::Completed {
                Activation::Done
            } else {
                // 再起動の後に有効になる (前の版を外すのに再起動が要るとき)。
                Activation::Failed(Failure::Other(
                    "the camera extension will be activated after a restart".to_owned(),
                ))
            });
        }

        #[unsafe(method(request:didFailWithError:))]
        fn failed(&self, _request: &OSSystemExtensionRequest, error: &NSError) {
            set_activation(Activation::Failed(failure(error)));
        }
    }
);

impl ActivationDelegate {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        // SAFETY: NSObject の init。
        unsafe { msg_send![super(this), init] }
    }
}

fn failure(error: &NSError) -> Failure {
    // SAFETY: SystemExtensions の定数の文字列。
    let domain = unsafe { OSSystemExtensionErrorDomain };
    if &*error.domain() == domain
        && error.code() == OSSystemExtensionErrorCode::UnsupportedParentBundleLocation.0
    {
        Failure::NotInApplications
    } else {
        Failure::Other(error.localizedDescription().to_string())
    }
}
