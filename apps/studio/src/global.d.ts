// ヘッドレスのブラウザでの確認のために window に置くもの (Output.svelte)。
import type { Player } from "@ugoe/player";

declare global {
  interface Window {
    /** 出力ウィンドウの受け手 (状態を見るため)。 */
    __UGOE_STUDIO_OUTPUT__?: Player;
  }
}
