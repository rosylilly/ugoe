// ヘルプ (操作マニュアル) を OS の既定のブラウザで開く (tauri-plugin-opener)。
// 開ける URL は capabilities/default.json でマニュアルのリポジトリに限っている。
// ヘッドレスのブラウザでの確認のため差し替えられる: `window.__UGOE_OPEN_URL__` を置くと、
// プラグインの代わりにそれを呼ぶ (docs/editor.md#ヘルプ)。
import { openUrl } from "@tauri-apps/plugin-opener";

/** 操作マニュアル (docs/manual/) の目次。 */
export const MANUAL_URL = "https://github.com/rosylilly/ugoe/blob/main/docs/manual/README.md";

declare global {
  interface Window {
    /** テスト用の差し替え。 */
    __UGOE_OPEN_URL__?: (url: string) => Promise<void>;
  }
}

/** `url` をブラウザで開く。開けなければ例外 (呼び出し側が URL を写せる形で出す)。 */
export function openExternal(url: string): Promise<void> {
  return (window.__UGOE_OPEN_URL__ ?? openUrl)(url);
}
