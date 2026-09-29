// 画面の文言。ビューアは日本語 (`ja`)、エディタは英語 (`en`) で表示する。

export interface TrackingStrings {
  title: string;
  loading: string;
  noFace: string;
  /** 記録の再生の位置 (`time` / `duration` 秒)。 */
  replayPosition(time: string, duration: string): string;
  /** 検出の頻度。 */
  fps(fps: number): string;
  camera: string;
  defaultCamera: string;
  /** 名前の分からない `index` 番目 (1 から) のカメラ。 */
  cameraNumber(index: number): string;
  start: string;
  stop: string;
  file: string;
  fileTitle: string;
  sourceKind: { camera: string; video: string; image: string; replay: string };
  mirrorView: string;
  mirrorViewTitle: string;
  landmarks: string;
  landmarksTitle: string;
  swapSides: string;
  swapSidesTitle: string;
  stopEyeBlink: string;
  stopEyeBlinkTitle: string;
  smoothing: string;
  smoothingTitle: string;
  calibrate: string;
  calibrateTitle: string;
  clearCalibration: string;
  clearCalibrationTitle: string;
  record: string;
  recordTitle: string;
  stopRecording(count: number): string;
  save(count: number): string;
  headAndBlendShapes: string;
  parameters: string;
  /** 記録の再生の既定の名前。 */
  recording: string;
  // --- エラー ---
  noMediaDevices: string;
  cameraNotAllowed: string;
  cameraNotFound: string;
  cameraBusy: string;
  cameraFailed: string;
  modelMissing: string;
  saveFailed: string;
}

export const ja: TrackingStrings = {
  title: "フェイストラッキング",
  loading: "準備中…",
  noFace: "顔が見つかりません",
  replayPosition: (time, duration) => `${time} / ${duration} 秒`,
  fps: (fps) => `${fps} fps`,
  camera: "カメラ",
  defaultCamera: "カメラ (既定)",
  cameraNumber: (index) => `カメラ ${index}`,
  start: "開始",
  stop: "停止",
  file: "ファイル…",
  fileTitle: "動画・画像、または記録した値 (JSON) で確かめる",
  sourceKind: { camera: "カメラ", video: "動画", image: "画像", replay: "記録の再生" },
  mirrorView: "鏡像で表示",
  mirrorViewTitle: "映像を鏡のように表示する (表示だけで、値は変わらない)",
  landmarks: "ランドマーク",
  landmarksTitle: "ランドマーク (顔の輪郭・目・眉・唇) を重ねて描く",
  swapSides: "左右を入れ替える",
  swapSidesTitle:
    "左右 (Left / Right) と、ヨー・ロールの符号を入れ替える。モデルを鏡に映った自分のように動かすとき、または入力の映像がすでに左右反転しているとき (反転済みの仮想カメラなど) に使う",
  stopEyeBlink: "目パチを止める",
  stopEyeBlinkTitle:
    "トラッキング中はモデルの自動の目パチを止める (目の開閉はトラッキングの値だけになる)",
  smoothing: "平滑化",
  smoothingTitle: "揺れを抑える強さ (One Euro フィルタ)。強いほど滑らかで、動きが遅れる",
  calibrate: "キャリブレーション",
  calibrateTitle: "今の顔を中立 (正面・無表情) にする",
  clearCalibration: "解除",
  clearCalibrationTitle: "キャリブレーションを取り消す",
  record: "記録",
  recordTitle: "検出した値を記録する (JSON に保存して、カメラの無い環境で再生できる)",
  stopRecording: (count) => `記録を止める (${count})`,
  save: (count) => `保存 (${count})`,
  headAndBlendShapes: "頭の向きとブレンドシェイプ",
  parameters: "変換後のパラメータ",
  recording: "記録",
  noMediaDevices: "この環境ではカメラを使えません (navigator.mediaDevices がありません)",
  cameraNotAllowed: "カメラの使用が許可されませんでした (OS やアプリの設定を確かめてください)",
  cameraNotFound: "カメラが見つかりません",
  cameraBusy: "カメラを開けません (ほかのアプリが使っているかもしれません)",
  cameraFailed: "カメラを開けません",
  modelMissing:
    "MediaPipe のモデルが同梱されていません (アプリのディレクトリで `pnpm mediapipe` を実行してください)",
  saveFailed: "保存できません",
};

export const en: TrackingStrings = {
  title: "Face tracking",
  loading: "Loading…",
  noFace: "No face found",
  replayPosition: (time, duration) => `${time} / ${duration} s`,
  fps: (fps) => `${fps} fps`,
  camera: "Camera",
  defaultCamera: "Camera (default)",
  cameraNumber: (index) => `Camera ${index}`,
  start: "Start",
  stop: "Stop",
  file: "File…",
  fileTitle: "Track a video, an image, or recorded values (JSON)",
  sourceKind: { camera: "Camera", video: "Video", image: "Image", replay: "Recording" },
  mirrorView: "Mirror view",
  mirrorViewTitle: "Show the video like a mirror (display only; the values do not change)",
  landmarks: "Landmarks",
  landmarksTitle: "Draw the landmarks (face oval, eyes, brows, lips) over the video",
  swapSides: "Swap sides",
  swapSidesTitle:
    "Swap Left / Right and the signs of yaw and roll. Use it to move the model like your reflection, or when the video is already mirrored (such as a mirrored virtual camera)",
  stopEyeBlink: "Stop eye blink",
  stopEyeBlinkTitle:
    "Stop the model's automatic eye blink while tracking (the eyes follow the tracking only)",
  smoothing: "Smoothing",
  smoothingTitle: "How much to suppress jitter (One Euro filter). Stronger is smoother but lags",
  calibrate: "Calibrate",
  calibrateTitle: "Use the current face as neutral (facing forward, no expression)",
  clearCalibration: "Clear",
  clearCalibrationTitle: "Undo the calibration",
  record: "Record",
  recordTitle: "Record the detected values (save them as JSON to replay without a camera)",
  stopRecording: (count) => `Stop recording (${count})`,
  save: (count) => `Save (${count})`,
  headAndBlendShapes: "Head and blend shapes",
  parameters: "Tracked parameters",
  recording: "Recording",
  noMediaDevices: "Cameras are not available here (navigator.mediaDevices is missing)",
  cameraNotAllowed: "The camera was not allowed (check the OS and app settings)",
  cameraNotFound: "No camera found",
  cameraBusy: "Cannot open the camera (another app may be using it)",
  cameraFailed: "Cannot open the camera",
  modelMissing: "The MediaPipe model is not bundled (run `pnpm mediapipe` in the app's directory)",
  saveFailed: "Cannot save",
};
