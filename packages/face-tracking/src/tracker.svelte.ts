// フェイストラッキング (T0703、エディタとの共有は T0636)。MediaPipe Face Landmarker でカメラ・動画・画像から
// 顔の値を得て、キャリブレーションと平滑化を通してから TrackingTarget に渡す (パラメータへの変換は Rust の対応付け)。
// 体のトラッキング (T0645) は同じフレームで Pose Landmarker (lite) を動かし、体の向きと肩の値 (pose.ts) を足す。
// カメラの無い環境では、記録した値の列 (recording.ts) を再生して確かめる。
//
// 入力 → [MediaPipe] → frameFromLandmarker (+ PoseConverter) → (左右を入れ替えるなら unmirrorFrame) → 記録
//      → applyCalibration → FrameSmoother → TrackingTarget.setTracking → WebPuppet.set_tracking (Rust)
//
// MediaPipe の wasm とモデルはアプリに同梱し (public/mediapipe/。scripts/mediapipe-assets.mjs)、
// 実行時にネットワークへ取りに行かない。
import type { FaceLandmarker, NormalizedLandmark, PoseLandmarker } from "@mediapipe/tasks-vision";
import { applyCalibration, averageFrames } from "./calibration";
import { FrameSmoother } from "./filter";
import { frameFromLandmarker, NO_BODY, type TrackingFrame, unmirrorFrame } from "./frame";
import { t } from "./i18n";
import { PoseConverter } from "./pose";
import {
  frameAt,
  parseRecording,
  type RecordedFrame,
  type Recording,
  recordingDuration,
  serializeRecording,
} from "./recording";

/**
 * トラッキングの値を受け取るもの (ビューアの Player、エディタのプレビューモード)。値は次のフレームの
 * `WebPuppet.set_tracking` に渡す。
 */
export interface TrackingTarget {
  /** 次のフレームで渡す値。`null` で止め、トラッキングが書き込んでいたパラメータを既定値に戻す。 */
  setTracking(frame: TrackingFrame | null): void;
  /** モデルの自動の目パチを有効・無効にする (トラッキング中は目の開閉に重なるため止められる)。 */
  setAutoEyeBlink(enabled: boolean): void;
}

/**
 * 入力の処理 (`step`) を繰り返し呼ぶ仕組み。止める関数を返す。`step` には `performance.now()` の時刻を渡す。
 */
export type TrackerSchedule = (step: (now: number) => void) => () => void;

export interface FaceTrackerOptions {
  /**
   * JSON のテキスト (記録) を保存する。`name` は既定のファイル名。保存したら `true`、取り消したら `false`。
   * 省くとブラウザのダウンロードで保存する。
   */
  saveJson?: (name: string, text: string) => Promise<boolean>;
  /**
   * 入力の処理を回す仕組み。省くと `requestAnimationFrame` (ページが隠れると止まる)。スタジオは、ウィンドウを最小化しても
   * 止まらないように、バックエンドの刻みで回す (docs/studio.md#隠れても止まらない、T0803)。
   */
  schedule?: TrackerSchedule;
}

/** `requestAnimationFrame` で回す (既定)。 */
export const animationFrames: TrackerSchedule = (step) => {
  let frame = 0;
  const loop = (now: number) => {
    frame = requestAnimationFrame(loop);
    step(now);
  };
  frame = requestAnimationFrame(loop);
  return () => cancelAnimationFrame(frame);
};

export type TrackerSource =
  | { kind: "camera"; label: string }
  | { kind: "video"; label: string }
  | { kind: "image"; label: string }
  | { kind: "replay"; label: string };

export type TrackerStatus = "idle" | "loading" | "running" | "error";

/** キャリブレーションで平均するフレームの数。 */
const CALIBRATION_FRAMES = 15;
/** 静止画を検出する間隔 (秒)。 */
const IMAGE_INTERVAL = 1 / 30;
/** 1 フレームで進める時間の上限 (秒)。 */
const MAX_DT = 0.1;

/** 同梱した MediaPipe のファイルの URL (アプリの public/mediapipe/)。 */
function assetUrl(path: string): string {
  return new URL(`mediapipe/${path}`, document.baseURI).href;
}

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** getUserMedia の失敗を、利用者に分かる言葉にする (元のメッセージも付ける)。 */
function cameraError(e: unknown): string {
  const name = e instanceof DOMException ? e.name : "";
  const reason =
    name === "NotAllowedError" || name === "SecurityError"
      ? t("errors.camera.notAllowed")
      : name === "NotFoundError" || name === "OverconstrainedError"
        ? t("errors.camera.notFound")
        : name === "NotReadableError"
          ? t("errors.camera.busy")
          : t("errors.camera.failed");
  return `${reason}: ${message(e)}`;
}

/** ブラウザのダウンロードで保存する。 */
async function downloadJson(name: string, text: string): Promise<boolean> {
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  return true;
}

export class FaceTracker {
  status = $state<TrackerStatus>("idle");
  error = $state<string | null>(null);
  source = $state<TrackerSource | null>(null);
  cameras = $state<{ id: string; label: string }[]>([]);
  cameraId = $state("");
  /** 映像を鏡像で表示する (表示だけ。値は変えない)。 */
  mirrorView = $state(true);
  /**
   * 左右 (Left / Right) とヨー・ロールの符号を入れ替える。既定 (入れ替えない) では、トラッキングされた人自身の
   * 左右がキャラクター自身の左右になる (docs/spec/avatar-profile.md)。入れ替えると、モデルが鏡に映った
   * 自分のように動く。入力の映像がすでに左右反転している (反転済みの仮想カメラなど) ときも、これで直す。
   */
  swapSides = $state(false);
  /** トラッキング中はモデルの自動の目パチを止める。 */
  stopEyeBlink = $state(true);
  /**
   * 体のトラッキング (Pose Landmarker) も行う。切ると検出が軽くなり、体の向きは頭の回転 × 0.3 になる
   * (docs/spec/avatar-profile.md)。
   */
  trackBody = $state(true);
  /** 直近のフレームで体 (肩) が見つかった。 */
  bodyFound = $state(false);
  /** 体のトラッキングが使えない理由 (モデルが無いなど)。顔のトラッキングは続ける。 */
  bodyError = $state<string | null>(null);
  /** ランドマークを重ねて描く。 */
  showLandmarks = $state(true);
  /** 平滑化の強さ (0〜1)。 */
  smoothing = $state(0.5);
  /** 直近のフレームで顔が見つかった。 */
  faceFound = $state(false);
  /** キャリブレーション済み (中立の顔を差し引いている)。 */
  calibrated = $state(false);
  /** TrackingTarget に渡した値 (キャリブレーション・平滑化の後)。 */
  output = $state<TrackingFrame | null>(null);
  /** 検出の頻度 (回/秒)。 */
  fps = $state(0);
  recording = $state(false);
  recordedCount = $state(0);
  /** 再生中の記録の長さ (秒) と位置。 */
  replayDuration = $state(0);
  replayTime = $state(0);

  readonly #target: TrackingTarget;
  readonly #saveJson: (name: string, text: string) => Promise<boolean>;
  readonly #schedule: TrackerSchedule;
  /** 入力の処理を止める (回していなければ `null`)。 */
  #cancel: (() => void) | null = null;
  #landmarker: FaceLandmarker | null = null;
  #landmarkerLoading: Promise<FaceLandmarker> | null = null;
  #pose: PoseLandmarker | null = null;
  #poseLoading: Promise<PoseLandmarker | null> | null = null;
  #poseConverter = new PoseConverter();
  #poseLandmarks: NormalizedLandmark[] | null = null;
  #drawing: typeof import("@mediapipe/tasks-vision") | null = null;
  #canvas: HTMLCanvasElement | null = null;
  #video: HTMLVideoElement | null = null;
  #image: HTMLImageElement | null = null;
  #stream: MediaStream | null = null;
  #objectUrl: string | null = null;
  #replay: Recording | null = null;
  #lastNow: number | null = null;
  #lastVideoTime = -1;
  #lastDetect = 0;
  #detectCount = 0;
  #fpsSince = 0;
  #landmarks: NormalizedLandmark[] | null = null;
  #recent: TrackingFrame[] = [];
  #neutral: TrackingFrame | null = null;
  #recorded: RecordedFrame[] = [];
  #recordStart = 0;
  #smoother = new FrameSmoother();

  constructor(target: TrackingTarget, options: FaceTrackerOptions = {}) {
    this.#target = target;
    this.#saveJson = options.saveJson ?? downloadJson;
    this.#schedule = options.schedule ?? animationFrames;
  }

  /** 映像とランドマークを描く canvas (パネルが渡す)。 */
  attachCanvas(canvas: HTMLCanvasElement | null): void {
    this.#canvas = canvas;
  }

  get running(): boolean {
    return this.status === "running";
  }

  /** 直近に検出した顔のランドマーク (正規化した画像座標)。確かめるためのもの。 */
  get landmarks(): readonly NormalizedLandmark[] | null {
    return this.#landmarks;
  }

  // --- 入力を選んで始める ---

  /** カメラの一覧を更新する。ラベルは一度カメラを許可した後でないと空のことがある。 */
  async refreshCameras(): Promise<void> {
    if (!navigator.mediaDevices?.enumerateDevices) return;
    const devices = await navigator.mediaDevices.enumerateDevices();
    this.cameras = devices
      .filter((d) => d.kind === "videoinput")
      .map((d, i) => ({
        id: d.deviceId,
        label: d.label || t("camera.numbered", { index: i + 1 }),
      }));
    if (!this.cameras.some((c) => c.id === this.cameraId))
      this.cameraId = this.cameras[0]?.id ?? "";
  }

  /** 選んでいるカメラで始める。 */
  async startCamera(): Promise<void> {
    await this.#start(async () => {
      if (!navigator.mediaDevices?.getUserMedia) {
        throw new Error(t("errors.noMediaDevices"));
      }
      const stream = await navigator.mediaDevices
        .getUserMedia({
          video: this.cameraId ? { deviceId: { exact: this.cameraId } } : true,
          audio: false,
        })
        .catch((e: unknown) => {
          throw new Error(cameraError(e));
        });
      this.#stream = stream;
      const video = this.#newVideo();
      video.srcObject = stream;
      await video.play();
      await this.refreshCameras();
      const track = stream.getVideoTracks()[0];
      const id = track?.getSettings().deviceId;
      if (id) this.cameraId = id;
      return { kind: "camera", label: track?.label || t("camera.fallbackName") };
    });
  }

  /** 動画・画像・記録 (JSON) のファイルで始める。種類は MIME と拡張子で決める。 */
  async startFile(file: File): Promise<void> {
    const name = file.name;
    if (file.type === "application/json" || name.toLowerCase().endsWith(".json")) {
      const text = await file.text();
      await this.startReplay(text, name);
      return;
    }
    await this.#start(async () => {
      const url = URL.createObjectURL(file);
      this.#objectUrl = url;
      if (file.type.startsWith("image/")) {
        const image = new Image();
        image.src = url;
        await image.decode();
        this.#image = image;
        return { kind: "image", label: name };
      }
      const video = this.#newVideo();
      video.loop = true;
      video.src = url;
      await video.play();
      return { kind: "video", label: name };
    });
  }

  /** 記録した値の列 (JSON のテキスト) を繰り返し再生する。MediaPipe は使わない。 */
  async startReplay(text: string, label = t("recording.defaultName")): Promise<void> {
    await this.#start(async () => {
      this.#replay = parseRecording(text);
      this.replayDuration = recordingDuration(this.#replay);
      return { kind: "replay", label };
    }, false);
  }

  stop(): void {
    this.#cancel?.();
    this.#cancel = null;
    if (this.recording) this.stopRecording();
    this.#releaseSource();
    this.#target.setTracking(null);
    if (this.stopEyeBlink) this.#target.setAutoEyeBlink(true);
    this.status = "idle";
    this.source = null;
    this.faceFound = false;
    this.bodyFound = false;
    this.output = null;
    this.fps = 0;
    this.#landmarks = null;
    this.#poseLandmarks = null;
    this.#poseConverter.reset();
    this.#clearCanvas();
  }

  // --- 調整 ---

  /** 今の顔を中立にする (直近のフレームの平均を差し引く)。 */
  calibrate(): void {
    const neutral = averageFrames(this.#recent);
    if (neutral === null) return;
    this.#neutral = neutral;
    this.calibrated = true;
    this.#smoother.reset();
  }

  clearCalibration(): void {
    this.#neutral = null;
    this.calibrated = false;
    this.#smoother.reset();
  }

  setStopEyeBlink(stop: boolean): void {
    this.stopEyeBlink = stop;
    if (this.running) this.#target.setAutoEyeBlink(!stop);
  }

  /** 体のトラッキングを切り替える。入れたときにモデルがまだ読まれていなければ読む。 */
  setTrackBody(on: boolean): void {
    this.trackBody = on;
    this.bodyFound = false;
    this.#poseLandmarks = null;
    this.#poseConverter.reset();
    if (on && this.running && this.#replay === null) void this.#loadPose();
  }

  // --- 記録 ---

  startRecording(): void {
    this.#recorded = [];
    this.recordedCount = 0;
    this.#recordStart = performance.now();
    this.recording = true;
  }

  stopRecording(): void {
    this.recording = false;
  }

  /** 記録を JSON のテキストにする (記録が無ければ `null`)。 */
  recordingJson(): string | null {
    return this.#recorded.length === 0 ? null : serializeRecording(this.#recorded);
  }

  async saveRecording(): Promise<void> {
    const text = this.recordingJson();
    if (text === null) return;
    try {
      await this.#saveJson("face-tracking.json", text);
    } catch (e) {
      this.error = `${t("errors.saveFailed")}: ${message(e)}`;
    }
  }

  // --- 内部 ---

  async #start(open: () => Promise<TrackerSource>, useLandmarker = true): Promise<void> {
    this.stop();
    this.error = null;
    this.status = "loading";
    try {
      if (useLandmarker) {
        await this.#loadLandmarker();
        if (this.trackBody) await this.#loadPose();
      }
      this.source = await open();
    } catch (e) {
      this.#releaseSource();
      this.status = "error";
      this.error = message(e);
      return;
    }
    this.#smoother.reset();
    this.#recent = [];
    this.#lastNow = null;
    this.#lastVideoTime = -1;
    this.#fpsSince = performance.now();
    this.#detectCount = 0;
    if (this.stopEyeBlink) this.#target.setAutoEyeBlink(false);
    this.status = "running";
    this.#cancel = this.#schedule(this.#tick);
  }

  #newVideo(): HTMLVideoElement {
    const video = document.createElement("video");
    video.muted = true;
    video.playsInline = true;
    this.#video = video;
    return video;
  }

  #releaseSource(): void {
    for (const track of this.#stream?.getTracks() ?? []) track.stop();
    this.#stream = null;
    if (this.#video) {
      this.#video.pause();
      this.#video.srcObject = null;
      this.#video.removeAttribute("src");
    }
    this.#video = null;
    this.#image = null;
    if (this.#objectUrl) URL.revokeObjectURL(this.#objectUrl);
    this.#objectUrl = null;
    this.#replay = null;
    this.replayDuration = 0;
    this.replayTime = 0;
  }

  async #loadLandmarker(): Promise<FaceLandmarker> {
    if (this.#landmarker) return this.#landmarker;
    this.#landmarkerLoading ??= (async () => {
      const modelUrl = assetUrl("face_landmarker.task");
      const head = await fetch(modelUrl, { method: "HEAD" }).catch(() => null);
      if (!head?.ok) {
        throw new Error(t("errors.modelMissing"));
      }
      const vision = await import("@mediapipe/tasks-vision");
      this.#drawing = vision;
      const fileset = await vision.FilesetResolver.forVisionTasks(assetUrl("wasm"));
      // MediaPipe は画像を WebGL で受け取る。canvas を渡さないと OffscreenCanvas を使うが、WebKitGTK
      // (Tauri の Linux) では OffscreenCanvas の WebGL が作れず検出に失敗したので、ふつうの canvas を渡す。
      const create = (delegate: "GPU" | "CPU") =>
        vision.FaceLandmarker.createFromOptions(fileset, {
          canvas: document.createElement("canvas"),
          baseOptions: { modelAssetPath: modelUrl, delegate },
          runningMode: "VIDEO",
          numFaces: 1,
          outputFaceBlendshapes: true,
          outputFacialTransformationMatrixes: true,
        });
      // GPU (WebGL) で作れなければ CPU で作る。
      const landmarker = await create("GPU").catch(() => create("CPU"));
      this.#landmarker = landmarker;
      return landmarker;
    })();
    try {
      return await this.#landmarkerLoading;
    } catch (e) {
      this.#landmarkerLoading = null;
      throw e;
    }
  }

  /**
   * 体の Pose Landmarker (lite) を読む。読めなければ `bodyError` に理由を書いて `null` を返す
   * (顔のトラッキングは続ける)。`#loadLandmarker` の後に呼ぶ (wasm を共有する)。
   */
  async #loadPose(): Promise<PoseLandmarker | null> {
    if (this.#pose) return this.#pose;
    this.#poseLoading ??= (async () => {
      const modelUrl = assetUrl("pose_landmarker_lite.task");
      const head = await fetch(modelUrl, { method: "HEAD" }).catch(() => null);
      if (!head?.ok) throw new Error(t("errors.poseModelMissing"));
      const vision = await import("@mediapipe/tasks-vision");
      const fileset = await vision.FilesetResolver.forVisionTasks(assetUrl("wasm"));
      // 顔と同じく、WebKitGTK のためにふつうの canvas を渡す。
      const create = (delegate: "GPU" | "CPU") =>
        vision.PoseLandmarker.createFromOptions(fileset, {
          canvas: document.createElement("canvas"),
          baseOptions: { modelAssetPath: modelUrl, delegate },
          runningMode: "VIDEO",
          numPoses: 1,
        });
      const pose = await create("GPU").catch(() => create("CPU"));
      this.#pose = pose;
      this.bodyError = null;
      return pose;
    })().catch((e: unknown) => {
      this.bodyError = message(e);
      this.#poseLoading = null;
      return null;
    });
    return await this.#poseLoading;
  }

  #tick = (now: number): void => {
    const dt = this.#lastNow === null ? 0 : Math.min((now - this.#lastNow) / 1000, MAX_DT);
    this.#lastNow = now;
    let raw: TrackingFrame | null | undefined;
    try {
      raw = this.#replay ? this.#replayFrame(dt) : this.#detect(now);
    } catch (e) {
      this.error = message(e);
      this.stop();
      this.status = "error";
      return;
    }
    // undefined: 新しい映像のフレームが無い (値は前のまま)。
    if (raw === undefined) return;
    this.faceFound = raw !== null;
    if (raw !== null) this.#push(raw, dt, now);
    this.#draw();
    this.#countFps(now);
  };

  #replayFrame(dt: number): TrackingFrame | null {
    const replay = this.#replay as Recording;
    const duration = this.replayDuration;
    this.replayTime = duration > 0 ? (this.replayTime + dt) % duration : 0;
    return frameAt(replay, this.replayTime);
  }

  /** 新しい映像のフレームがあれば検出する。無ければ `undefined`、顔が無ければ `null`。 */
  #detect(now: number): TrackingFrame | null | undefined {
    const landmarker = this.#landmarker;
    if (landmarker === null) return undefined;
    let source: HTMLVideoElement | HTMLImageElement;
    if (this.#video) {
      const video = this.#video;
      if (video.readyState < 2 || video.currentTime === this.#lastVideoTime) return undefined;
      this.#lastVideoTime = video.currentTime;
      source = video;
    } else if (this.#image) {
      if (now - this.#lastDetect < IMAGE_INTERVAL * 1000) return undefined;
      source = this.#image;
    } else {
      return undefined;
    }
    // detectForVideo の時刻は単調に増えなければならない。
    const timestamp = Math.max(now, this.#lastDetect + 1);
    this.#lastDetect = timestamp;
    const result = landmarker.detectForVideo(source, timestamp);
    this.#landmarks = result.faceLandmarks[0] ?? null;
    const frame = frameFromLandmarker(result);
    if (frame !== null && this.trackBody) {
      // 体が見つからない (・まだ読めていない) ときは、体の値をすべて無しにする (頭の回転 × 0.3 に戻る)。
      frame.body = this.#detectBody(source, timestamp) ?? { ...NO_BODY };
    }
    return frame && this.swapSides ? unmirrorFrame(frame) : frame;
  }

  /** 体を検出する。Pose Landmarker が無い・人が見つからなければ `null`。 */
  #detectBody(source: HTMLVideoElement | HTMLImageElement, timestamp: number) {
    const pose = this.#pose;
    if (pose === null) return null;
    const result = pose.detectForVideo(source, timestamp);
    this.#poseLandmarks = result.landmarks[0] ?? null;
    const width = source instanceof HTMLVideoElement ? source.videoWidth : source.naturalWidth;
    const height = source instanceof HTMLVideoElement ? source.videoHeight : source.naturalHeight;
    return this.#poseConverter.convert(result, width, height);
  }

  /** 検出した (または再生した) 値を記録し、キャリブレーションと平滑化を通して TrackingTarget に渡す。 */
  #push(raw: TrackingFrame, dt: number, now: number): void {
    this.bodyFound = raw.body !== undefined && raw.body.yaw !== null;
    this.#recent.push(raw);
    if (this.#recent.length > CALIBRATION_FRAMES) this.#recent.shift();
    if (this.recording) {
      this.#recorded.push({ t: (now - this.#recordStart) / 1000, ...raw });
      this.recordedCount = this.#recorded.length;
    }
    this.#smoother.strength = this.smoothing;
    const out = this.#smoother.smooth(applyCalibration(raw, this.#neutral), dt);
    this.output = out;
    this.#target.setTracking(out);
  }

  #countFps(now: number): void {
    this.#detectCount++;
    const elapsed = now - this.#fpsSince;
    if (elapsed >= 500) {
      this.fps = Math.round((this.#detectCount * 1000) / elapsed);
      this.#detectCount = 0;
      this.#fpsSince = now;
    }
  }

  #clearCanvas(): void {
    const canvas = this.#canvas;
    canvas?.getContext("2d")?.clearRect(0, 0, canvas.width, canvas.height);
  }

  /** 映像とランドマークを canvas に描く。 */
  #draw(): void {
    const canvas = this.#canvas;
    const media = this.#video ?? this.#image;
    if (canvas === null || media === null) return;
    const width = media instanceof HTMLVideoElement ? media.videoWidth : media.naturalWidth;
    const height = media instanceof HTMLVideoElement ? media.videoHeight : media.naturalHeight;
    if (width === 0 || height === 0) return;
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }
    const ctx = canvas.getContext("2d");
    if (ctx === null) return;
    ctx.drawImage(media, 0, 0, width, height);
    const landmarks = this.#landmarks;
    const vision = this.#drawing;
    if (!this.showLandmarks || landmarks === null || vision === null || !this.faceFound) return;
    const { DrawingUtils, FaceLandmarker } = vision;
    const draw = new DrawingUtils(ctx);
    const lineWidth = Math.max(1, width / 640);
    const pose = this.trackBody ? this.#poseLandmarks : null;
    if (pose !== null) {
      // 体の値に使う点 (耳・肩・腰) と、肩の線・胴。
      draw.drawConnectors(pose, BODY_CONNECTIONS, { color: "#30a0ff", lineWidth: lineWidth * 1.5 });
      draw.drawLandmarks(
        BODY_POINTS.map((i) => pose[i]).filter((p) => p !== undefined),
        { color: "#30a0ff", radius: lineWidth * 2 },
      );
    }
    draw.drawConnectors(landmarks, FaceLandmarker.FACE_LANDMARKS_TESSELATION, {
      color: "#c0c0c060",
      lineWidth: lineWidth * 0.5,
    });
    for (const [connections, color] of [
      [FaceLandmarker.FACE_LANDMARKS_FACE_OVAL, "#e0e0e0"],
      [FaceLandmarker.FACE_LANDMARKS_LEFT_EYE, "#30ff30"],
      [FaceLandmarker.FACE_LANDMARKS_LEFT_EYEBROW, "#30ff30"],
      [FaceLandmarker.FACE_LANDMARKS_LEFT_IRIS, "#30ff30"],
      [FaceLandmarker.FACE_LANDMARKS_RIGHT_EYE, "#ff3030"],
      [FaceLandmarker.FACE_LANDMARKS_RIGHT_EYEBROW, "#ff3030"],
      [FaceLandmarker.FACE_LANDMARKS_RIGHT_IRIS, "#ff3030"],
      [FaceLandmarker.FACE_LANDMARKS_LIPS, "#e0e0e0"],
    ] as const) {
      draw.drawConnectors(landmarks, connections, { color, lineWidth });
    }
  }
}

/** 体の値に使う Pose Landmarker の点 (左右の耳・肩・腰) と、描く線。 */
const BODY_POINTS = [7, 8, 11, 12, 23, 24];
const BODY_CONNECTIONS = [
  { start: 11, end: 12 },
  { start: 11, end: 23 },
  { start: 12, end: 24 },
  { start: 23, end: 24 },
];

declare global {
  interface Window {
    /** フェイストラッキング (ヘッドレスのブラウザでの確認のため)。 */
    __UGOE_TRACKER__?: FaceTracker;
  }
}
