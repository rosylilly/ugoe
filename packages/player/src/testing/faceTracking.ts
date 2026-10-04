// テスト用の部品 (配布物に入らない)。`@ugoe/face-tracking` の代わり。画面の部品と MediaPipe まで読み込むと遅いので、
// Player が使う関数だけにする。テストのファイルで `vi.mock("@ugoe/face-tracking", () => import("./testing/faceTracking"))`。

/** `setPuppetTracking` に渡したトラッキングの入力 (テストごとに空にする)。 */
export const tracked: unknown[] = [];

export function setPuppetTracking(_puppet: unknown, frame: unknown): void {
  tracked.push(frame);
}
