// エディタの中の MCP サーバ (エージェントをつなぐ) の、画面全体で共有する状態 (docs/editor.md#エージェントをつなぐ)。
// ブリッジはバックエンド (apps/editor/src-tauri/src/mcp.rs) が持ち、ここは状態の表示・設定の変更・確認への答えだけ。
// バックエンドのイベント (`mcp-bridge`) は App が `handle` に渡す。
import { type ApprovalRequest, type BridgeEvent, ipc, type McpBridgeStatus } from "../ipc";

/** イベントを受けて App がすること。 */
export interface BridgeHooks {
  /** エージェントが文書を変えた・保存した (読み直す)。 */
  document: () => void;
  /** エージェントが提案を置いた。 */
  proposals: () => void;
}

export class AgentBridge {
  status = $state.raw<McpBridgeStatus | null>(null);
  error = $state<string | null>(null);
  busy = $state(false);
  /** 答えを待っているエージェントの求め (古い順。先頭をダイアログに出す)。 */
  approvals = $state.raw<ApprovalRequest[]>([]);

  /** 待ち受けていて、握手したクライアントがいるか (ヘッダーの印)。 */
  get connected(): number {
    return this.status?.running ? this.status.clients.length : 0;
  }

  async refresh(): Promise<void> {
    await this.#run(ipc.mcpBridgeStatus);
  }

  setEnabled(enabled: boolean): Promise<boolean> {
    return this.#run(() => ipc.setMcpBridgeEnabled(enabled));
  }

  setPort(port: number | null): Promise<boolean> {
    return this.#run(() => ipc.setMcpBridgePort(port));
  }

  regenerateToken(): Promise<boolean> {
    return this.#run(ipc.regenerateMcpBridgeToken);
  }

  /** 確認に答える。答えたものは一覧から外す (閉じた知らせを待たない)。 */
  async answer(id: number, approved: boolean): Promise<void> {
    this.approvals = this.approvals.filter((r) => r.id !== id);
    try {
      await ipc.answerMcpApproval(id, approved);
    } catch (e) {
      this.error = String(e);
    }
  }

  /** バックエンドのイベント。 */
  handle(event: BridgeEvent, hooks: BridgeHooks): void {
    switch (event.kind) {
      case "status":
        void this.refresh();
        break;
      case "approval":
        if (!this.approvals.some((r) => r.id === event.request.id)) {
          this.approvals = [...this.approvals, event.request];
        }
        break;
      case "approval_closed":
        this.approvals = this.approvals.filter((r) => r.id !== event.id);
        break;
      case "document":
        hooks.document();
        break;
      case "proposals":
        hooks.proposals();
        break;
    }
  }

  async #run(task: () => Promise<McpBridgeStatus>): Promise<boolean> {
    this.busy = true;
    try {
      const status = await task();
      this.status = status;
      // 画面を開き直したときも、待っている確認を出し直す
      const known = new Set(this.approvals.map((r) => r.id));
      const pending = status.pending.filter((r) => !known.has(r.id));
      const live = new Set(status.pending.map((r) => r.id));
      this.approvals = [...this.approvals.filter((r) => live.has(r.id)), ...pending];
      this.error = null;
      return true;
    } catch (e) {
      this.error = String(e);
      return false;
    } finally {
      this.busy = false;
    }
  }
}

export const agentBridge = new AgentBridge();
