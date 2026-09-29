// AI の設定と同意の、画面全体で共有する状態 (docs/editor.md#ai-の設定と送信の同意)。
// AI の機能 (T0637 の提案など) は `aiSettings.canSend` で有効・無効を決め、送るときは
// `withConsent(call, consentPrompt.ask)` を使う。
import { type AiSettings, type ConsentAnswer, type DisclosureView, ipc } from "../ipc";
import { aiBlockedReason } from "./providers";

/** 確認のダイアログ (ConsentDialog.svelte) に出す問い合わせ。 */
export class ConsentPrompt {
  /** 答えを待っている問い合わせ。なければ `null`。 */
  pending = $state<{
    disclosure: DisclosureView;
    resolve: (a: ConsentAnswer | null) => void;
  } | null>(null);

  /** `withConsent` に渡す。前の問い合わせが残っていれば、それは「送らない」で閉じる。 */
  ask = (disclosure: DisclosureView): Promise<ConsentAnswer | null> => {
    this.pending?.resolve(null);
    return new Promise((resolve) => {
      this.pending = { disclosure, resolve };
    });
  };

  /** ダイアログの答え。`send` が偽なら送らない。 */
  answer(send: boolean, remember: boolean): void {
    const pending = this.pending;
    this.pending = null;
    pending?.resolve(send ? { disclosure: pending.disclosure, remember } : null);
  }
}

/** AI の設定 (選んだプロバイダ・API キーの状態とプロジェクトの「外部に送らない」)。キーの中身は持たない。 */
export class AiSettingsState {
  value = $state<AiSettings | null>(null);
  error = $state<string | null>(null);

  /**
   * 選んだプロバイダに送れるか (外部に出るならプロジェクトが許していて、キーが要るなら保存先か環境変数にある)。
   * 手元のサーバ (ループバック) は「外部に送らない」のプロジェクトでも送れる。
   */
  get canSend(): boolean {
    const s = this.value;
    return !!s && aiBlockedReason(s) === null;
  }

  refresh(): Promise<boolean> {
    return this.run(ipc.aiSettings);
  }

  /** 設定を変える IPC を呼び、戻り値で状態を置き換える。失敗は `error` に出して `false`。 */
  async run(task: () => Promise<AiSettings>): Promise<boolean> {
    try {
      this.value = await task();
      this.error = null;
      return true;
    } catch (e) {
      this.error = String(e);
      return false;
    }
  }
}

export const consentPrompt = new ConsentPrompt();
export const aiSettings = new AiSettingsState();
