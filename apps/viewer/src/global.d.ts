// ヘッドレスのブラウザでの確認や、外からドライバを登録するために window に置くもの (App.svelte)。
import type { Player } from "@ugoe/player";

declare global {
  interface Window {
    /** 表示中のビューア (ヘッドレスのブラウザでの確認や、外からドライバを登録するため)。 */
    __UGOE_VIEWER__?: Player;
  }
}
