// ugoe studio の仮想カメラ (macOS の Camera Extension。docs/studio.md#macos-camera-extension、T0809)。
//
// 1 つの機器「ugoe studio」に、2 つのストリームを持たせる。
//
// - 出す側 (source): 会議アプリなどが開くカメラの画。フレームレートごとに、スタジオから届いた最新の画を出す
//   (まだ無い・スタジオが止まったときは黒)。アプリが選んだ大きさと違えば VideoToolbox で拡大縮小する。
// - 受ける側 (sink): スタジオ (apps/studio/vcam/src/cmio.rs) が CoreMediaIO の C の API で NV12 ('420v') の画を積む。
//   出す側が動いている間だけ取り出す。取り出さない間はキューが埋まるので、スタジオは「受け取るアプリがいない」と分かる。
//
// 値 (機器とストリームの名前・ID・選べる大きさとフレームレート) は cmio.rs と合わせる。

import CoreMedia
import CoreMediaIO
import CoreVideo
import Foundation
import IOKit.audio
import VideoToolbox

/// スタジオと合わせる値。
enum Ids {
    static let device = UUID(uuidString: "07DA1CFA-007F-4E78-873E-5FFA646E00B4")!
    static let source = UUID(uuidString: "502067A1-15E8-403E-BC29-30361C09F2C2")!
    static let sink = UUID(uuidString: "37214D7C-3598-4E83-A798-3AB8C8325C83")!
    static let name = "ugoe studio"
    /// スタジオは受ける側のストリームをこの名前で探す。
    static let sinkName = "ugoe studio input"
}

/// 選べる大きさ (`RESOLUTIONS`) とフレームレート (`FRAME_RATES`)。並びは書式の番号になる (0 番が既定の 1280×720)。
let resolutions: [(Int32, Int32)] = [(1280, 720), (1920, 1080)]
let frameRates: [Int32] = [30, 60]

/// スタジオと同じ BT.709・限定範囲の NV12。
let pixelFormat = kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange

func makeFormats() -> [CMIOExtensionStreamFormat] {
    let durations = frameRates.map { CMTime(value: 1, timescale: $0) }
    let extensions: [CFString: Any] = [
        kCMFormatDescriptionExtension_YCbCrMatrix: kCMFormatDescriptionYCbCrMatrix_ITU_R_709_2,
        kCMFormatDescriptionExtension_ColorPrimaries: kCMFormatDescriptionColorPrimaries_ITU_R_709_2,
        kCMFormatDescriptionExtension_TransferFunction: kCMFormatDescriptionTransferFunction_ITU_R_709_2,
    ]
    return resolutions.map { width, height in
        var description: CMFormatDescription?
        CMVideoFormatDescriptionCreate(
            allocator: kCFAllocatorDefault,
            codecType: pixelFormat,
            width: width,
            height: height,
            extensions: extensions as CFDictionary,
            formatDescriptionOut: &description
        )
        return CMIOExtensionStreamFormat(
            formatDescription: description!,
            maxFrameDuration: durations.first!,
            minFrameDuration: durations.last!,
            validFrameDurations: durations
        )
    }
}

func hostTimeNanoseconds() -> UInt64 {
    let now = CMClockGetTime(CMClockGetHostTimeClock())
    return UInt64(CMTimeConvertScale(now, timescale: 1_000_000_000, method: .default).value)
}

/// 機器。2 つのストリームと、画の受け渡し (すべて `queue` の上で行う) を持つ。
final class Camera: NSObject, CMIOExtensionDeviceSource {
    private(set) var device: CMIOExtensionDevice!
    private var source: CameraStream!
    private var sink: CameraStream!

    private let queue = DispatchQueue(label: "app.ugoe.studio.camera.frames")
    private var timer: DispatchSourceTimer?
    /// 受ける側を開いているスタジオ。
    private var sinkClient: CMIOExtensionClient?
    /// スタジオから最後に届いた画。
    private var latest: CVPixelBuffer?
    private var consuming = false
    /// 出す大きさの画の置き場 (拡大縮小と黒の画に使う)。
    private var pool: CVPixelBufferPool?
    private var poolSize: (Int32, Int32) = (0, 0)
    private var black: CVPixelBuffer?
    private var transfer: VTPixelTransferSession?

    init(localizedName: String) {
        super.init()
        device = CMIOExtensionDevice(
            localizedName: localizedName, deviceID: Ids.device, legacyDeviceID: nil, source: self)
        let formats = makeFormats()
        source = CameraStream(
            localizedName: localizedName, streamID: Ids.source, direction: .source, formats: formats,
            camera: self)
        sink = CameraStream(
            localizedName: Ids.sinkName, streamID: Ids.sink, direction: .sink, formats: formats,
            camera: self)
        do {
            try device.addStream(source.stream)
            try device.addStream(sink.stream)
        } catch {
            fatalError("failed to add the streams: \(error.localizedDescription)")
        }
    }

    var availableProperties: Set<CMIOExtensionProperty> {
        [.deviceTransportType, .deviceModel]
    }

    func deviceProperties(forProperties properties: Set<CMIOExtensionProperty>) throws
        -> CMIOExtensionDeviceProperties
    {
        let result = CMIOExtensionDeviceProperties(dictionary: [:])
        if properties.contains(.deviceTransportType) {
            result.transportType = kIOAudioDeviceTransportTypeVirtual
        }
        if properties.contains(.deviceModel) {
            result.model = Ids.name
        }
        return result
    }

    func setDeviceProperties(_ deviceProperties: CMIOExtensionDeviceProperties) throws {}

    // MARK: 出す側

    func startSource() {
        queue.async {
            self.timer?.cancel()
            let timer = DispatchSource.makeTimerSource(queue: self.queue)
            let interval = 1.0 / Double(self.source.frameRate)
            timer.schedule(deadline: .now(), repeating: interval, leeway: .milliseconds(1))
            timer.setEventHandler { [weak self] in self?.tick() }
            self.timer = timer
            timer.resume()
        }
    }

    func stopSource() {
        queue.async {
            self.timer?.cancel()
            self.timer = nil
        }
    }

    /// フレームレートごとに 1 枚を出し、次の画を受ける側から取り出す。
    private func tick() {
        let (width, height) = source.size
        if let frame = output(width: width, height: height) {
            send(frame)
        }
        consumeNext()
    }

    private func output(width: Int32, height: Int32) -> CVPixelBuffer? {
        if let latest, CVPixelBufferGetWidth(latest) == Int(width),
            CVPixelBufferGetHeight(latest) == Int(height)
        {
            return latest
        }
        guard let target = makeBuffer(width: width, height: height) else { return nil }
        if let latest {
            if transfer == nil {
                VTPixelTransferSessionCreate(allocator: kCFAllocatorDefault, pixelTransferSessionOut: &transfer)
            }
            if let transfer, VTPixelTransferSessionTransferImage(transfer, from: latest, to: target) == noErr {
                return target
            }
        }
        return blackBuffer(width: width, height: height)
    }

    private func makeBuffer(width: Int32, height: Int32) -> CVPixelBuffer? {
        if pool == nil || poolSize != (width, height) {
            let attributes: [CFString: Any] = [
                kCVPixelBufferPixelFormatTypeKey: pixelFormat,
                kCVPixelBufferWidthKey: width,
                kCVPixelBufferHeightKey: height,
                kCVPixelBufferIOSurfacePropertiesKey: [:] as [CFString: Any],
            ]
            pool = nil
            black = nil
            CVPixelBufferPoolCreate(kCFAllocatorDefault, nil, attributes as CFDictionary, &pool)
            poolSize = (width, height)
        }
        guard let pool else { return nil }
        var buffer: CVPixelBuffer?
        CVPixelBufferPoolCreatePixelBuffer(kCFAllocatorDefault, pool, &buffer)
        if let buffer {
            CVBufferSetAttachment(
                buffer, kCVImageBufferYCbCrMatrixKey, kCVImageBufferYCbCrMatrix_ITU_R_709_2, .shouldPropagate)
            CVBufferSetAttachment(
                buffer, kCVImageBufferColorPrimariesKey, kCVImageBufferColorPrimaries_ITU_R_709_2, .shouldPropagate)
            CVBufferSetAttachment(
                buffer, kCVImageBufferTransferFunctionKey, kCVImageBufferTransferFunction_ITU_R_709_2,
                .shouldPropagate)
        }
        return buffer
    }

    /// 黒 (Y = 16、Cb = Cr = 128) の画。大きさが変わるまで使い回す。
    private func blackBuffer(width: Int32, height: Int32) -> CVPixelBuffer? {
        if let black { return black }
        guard let buffer = makeBuffer(width: width, height: height) else { return nil }
        CVPixelBufferLockBaseAddress(buffer, [])
        for (plane, value) in [(0, UInt8(16)), (1, UInt8(128))] {
            if let base = CVPixelBufferGetBaseAddressOfPlane(buffer, plane) {
                let bytes = CVPixelBufferGetBytesPerRowOfPlane(buffer, plane) * CVPixelBufferGetHeightOfPlane(buffer, plane)
                memset(base, Int32(value), bytes)
            }
        }
        CVPixelBufferUnlockBaseAddress(buffer, [])
        black = buffer
        return buffer
    }

    private func send(_ buffer: CVPixelBuffer) {
        var format: CMFormatDescription?
        CMVideoFormatDescriptionCreateForImageBuffer(
            allocator: kCFAllocatorDefault, imageBuffer: buffer, formatDescriptionOut: &format)
        guard let format else { return }
        let now = hostTimeNanoseconds()
        var timing = CMSampleTimingInfo(
            duration: CMTime(value: 1, timescale: source.frameRate),
            presentationTimeStamp: CMTime(value: CMTimeValue(now), timescale: 1_000_000_000),
            decodeTimeStamp: .invalid
        )
        var sample: CMSampleBuffer?
        CMSampleBufferCreateReadyWithImageBuffer(
            allocator: kCFAllocatorDefault, imageBuffer: buffer, formatDescription: format,
            sampleTiming: &timing, sampleBufferOut: &sample)
        guard let sample else { return }
        source.stream.send(sample, discontinuity: [], hostTimeInNanoseconds: now)
    }

    // MARK: 受ける側

    func startSink(client: CMIOExtensionClient) {
        queue.async { self.sinkClient = client }
    }

    func stopSink() {
        queue.async {
            self.sinkClient = nil
            self.latest = nil
        }
    }

    /// 受ける側のキューから 1 枚を取り出す (前の取り出しが終わっていなければ何もしない)。
    private func consumeNext() {
        guard let client = sinkClient, !consuming else { return }
        consuming = true
        sink.stream.consumeSampleBuffer(from: client) { [weak self] sample, sequence, _, _, _ in
            guard let self else { return }
            self.queue.async {
                self.consuming = false
                guard let sample, let image = CMSampleBufferGetImageBuffer(sample) else { return }
                if self.sinkClient != nil {
                    self.latest = image
                }
                let output = CMIOExtensionScheduledOutput(
                    sequenceNumber: sequence, hostTimeInNanoseconds: hostTimeNanoseconds())
                self.sink.stream.notifyScheduledOutputChanged(output)
            }
        }
    }
}

/// ストリーム (出す側と受ける側で共通)。
final class CameraStream: NSObject, CMIOExtensionStreamSource {
    private(set) var stream: CMIOExtensionStream!
    let formats: [CMIOExtensionStreamFormat]
    private let direction: CMIOExtensionStream.Direction
    private unowned let camera: Camera
    private var activeFormatIndex = 0
    private(set) var frameRate: Int32 = frameRates[0]

    init(
        localizedName: String, streamID: UUID, direction: CMIOExtensionStream.Direction,
        formats: [CMIOExtensionStreamFormat], camera: Camera
    ) {
        self.formats = formats
        self.direction = direction
        self.camera = camera
        super.init()
        stream = CMIOExtensionStream(
            localizedName: localizedName, streamID: streamID, direction: direction, clockType: .hostTime,
            source: self)
    }

    /// いま選ばれている大きさ。
    var size: (Int32, Int32) {
        resolutions[activeFormatIndex]
    }

    var availableProperties: Set<CMIOExtensionProperty> {
        var properties: Set<CMIOExtensionProperty> = [.streamActiveFormatIndex, .streamFrameDuration]
        if direction == .sink {
            properties.formUnion([
                .streamSinkBufferQueueSize, .streamSinkBuffersRequiredForStartup,
                .streamSinkBufferUnderrunCount, .streamSinkEndOfData,
            ])
        }
        return properties
    }

    func streamProperties(forProperties properties: Set<CMIOExtensionProperty>) throws
        -> CMIOExtensionStreamProperties
    {
        let result = CMIOExtensionStreamProperties(dictionary: [:])
        if properties.contains(.streamActiveFormatIndex) {
            result.activeFormatIndex = activeFormatIndex
        }
        if properties.contains(.streamFrameDuration) {
            result.frameDuration = CMTime(value: 1, timescale: frameRate)
        }
        if properties.contains(.streamSinkBufferQueueSize) {
            // 小さくして、取り出されていないことがスタジオから早く分かるようにする。
            result.sinkBufferQueueSize = 2
        }
        if properties.contains(.streamSinkBuffersRequiredForStartup) {
            result.sinkBuffersRequiredForStartup = 1
        }
        return result
    }

    func setStreamProperties(_ streamProperties: CMIOExtensionStreamProperties) throws {
        if let index = streamProperties.activeFormatIndex, formats.indices.contains(index) {
            activeFormatIndex = index
        }
        if let duration = streamProperties.frameDuration, duration.value > 0 {
            let rate = Int32((Double(duration.timescale) / Double(duration.value)).rounded())
            frameRate = frameRates.min { abs($0 - rate) < abs($1 - rate) } ?? frameRates[0]
        }
    }

    func authorizedToStartStream(for client: CMIOExtensionClient) -> Bool {
        if direction == .sink {
            camera.startSink(client: client)
        }
        return true
    }

    func startStream() throws {
        if direction == .source {
            camera.startSource()
        }
    }

    func stopStream() throws {
        if direction == .source {
            camera.stopSource()
        } else {
            camera.stopSink()
        }
    }
}

/// 拡張の入口。機器を 1 つだけ持つ。
final class Provider: NSObject, CMIOExtensionProviderSource {
    private(set) var provider: CMIOExtensionProvider!
    private var camera: Camera!

    init(clientQueue: DispatchQueue?) {
        super.init()
        provider = CMIOExtensionProvider(source: self, clientQueue: clientQueue)
        camera = Camera(localizedName: Ids.name)
        do {
            try provider.addDevice(camera.device)
        } catch {
            fatalError("failed to add the device: \(error.localizedDescription)")
        }
    }

    func connect(to client: CMIOExtensionClient) throws {}

    func disconnect(from client: CMIOExtensionClient) {}

    var availableProperties: Set<CMIOExtensionProperty> {
        [.providerManufacturer]
    }

    func providerProperties(forProperties properties: Set<CMIOExtensionProperty>) throws
        -> CMIOExtensionProviderProperties
    {
        let result = CMIOExtensionProviderProperties(dictionary: [:])
        if properties.contains(.providerManufacturer) {
            result.manufacturer = "ugoe"
        }
        return result
    }

    func setProviderProperties(_ providerProperties: CMIOExtensionProviderProperties) throws {}
}
