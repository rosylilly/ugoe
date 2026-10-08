// ヘッドレスのブラウザでの確認のために window に置くもの (Output.svelte・Control.svelte)。
import type { Player } from "@ugoe/player";
import type { StatusMessage } from "./lib/host";

declare global {
  interface Window {
    /** 出力ウィンドウの受け手 (状態を見るため)。 */
    __UGOE_STUDIO_OUTPUT__?: Player;
    /** 操作ウィンドウに直近に届いた状態 (負荷とトラッキングの待ちを e2e が集めるため。T0873)。 */
    __UGOE_STUDIO_STATUS__?: StatusMessage;
  }
}
