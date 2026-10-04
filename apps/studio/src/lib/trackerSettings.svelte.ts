// トラッキングのパネル (@ugoe/face-tracking) で変えた調整と、使ったカメラを、プロファイルの入力の設定へ書き戻す (InputPanel.svelte)。
import type { FaceTracker } from "@ugoe/face-tracking";
import { untrack } from "svelte";
import type { InputSettings } from "./inputs";

/** 書き戻すときに読むトラッカーの値。 */
export type TrackerView = Pick<
  FaceTracker,
  "smoothing" | "swapSides" | "trackBody" | "stopEyeBlink" | "running" | "source" | "cameraId"
>;

/**
 * トラッカーの値が変わるたびに、変わった入力の設定を `onsettings` に渡す ($effect を作るので、部品の初期化の中で呼ぶ)。
 * 同じ設定なら渡さない (送り合いが止まらなくならないように)。カメラは使い始めた (替えた) ときだけ渡す。
 * 動いている間に別のプロファイルを開いても、そのプロファイルのカメラを、動いているカメラで書き換えない。
 */
export function rememberTrackerSettings(
  getTracker: () => TrackerView,
  settings: () => InputSettings,
  onsettings: (settings: InputSettings) => void,
): void {
  /** 前に見た、動いているカメラの ID。 */
  let seenCamera: string | null = null;
  $effect(() => {
    const tracker = getTracker();
    const patch: Partial<InputSettings> = {
      smoothing: tracker.smoothing,
      swapSides: tracker.swapSides,
      trackBody: tracker.trackBody,
      stopEyeBlink: tracker.stopEyeBlink,
    };
    const camera =
      tracker.running && tracker.source?.kind === "camera" && tracker.cameraId !== ""
        ? { id: tracker.cameraId, label: tracker.source.label }
        : null;
    if (camera !== null && camera.id !== seenCamera) patch.camera = camera;
    seenCamera = camera?.id ?? null;
    // 設定 (プロファイル) が変わっただけでは書き戻さない。
    untrack(() => {
      const current = $state.snapshot(settings()) as InputSettings;
      const next = { ...current, ...patch };
      if (JSON.stringify(next) !== JSON.stringify(current)) onsettings(next);
    });
  });
}
