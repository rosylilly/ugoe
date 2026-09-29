// AI の提案のパネル (ProposalsPanel.svelte) とビューポートが共有する状態 (docs/editor.md#ai-の提案)。
// 提案はバックエンド (apps/editor/src-tauri/src/proposals.rs) が持ち、ここは表示と操作だけ。
//
// - 分類と提案は、AI を使うなら `withConsent` で送る前に確かめる。使わないなら何も送らない。
// - プレビューは、提案を複製に適用した姿を `preview.model` に置き、App がビューポートに渡す (ドキュメントは変えない)。
//   ドキュメントが変わったら (編集・Undo・採用) プレビューをやめる。
// - 外したコマンドの番号 (`excluded`) は提案ごとに持ち、プレビューと採用に渡す (一部だけの採用)。
import type { EditorDocument } from "../document.svelte";
import { fileDialogs } from "../fileDialogs";
import { PSD_FILTER, parentDir } from "../filePaths";
import { type ConsentAnswer, ipc, type Model, type Parameter, type ProposalState } from "../ipc";
import type { ViewportModel } from "../viewport/controller.svelte";
import { consentPrompt } from "./ai.svelte";
import { withConsent } from "./consent";
import { type CommandGroup, excludedList, toggleCommand, toggleGroup } from "./proposals";

/** プレビュー中の提案。 */
export interface ProposalPreviewState {
  id: number;
  /** 提案を適用した複製 (ビューポートに渡す)。 */
  model: ViewportModel;
  /** 複製のパラメータ (提案が足したものを含む。パネルのスライダーに出す)。 */
  parameters: Parameter[];
}

export class ProposalsView {
  /** 「AI」のタブを開いているか。 */
  active = $state(false);
  value = $state.raw<ProposalState | null>(null);
  /** 提案ごとの、外したコマンドの番号 (採用して隠れた提案の分も残す。Undo で再び出たときに使う)。 */
  excluded = $state.raw<Record<number, ReadonlySet<number>>>({});
  preview = $state.raw<ProposalPreviewState | null>(null);
  /** 実行中の段階の説明 (進み具合)。なければ null。 */
  busy = $state<string | null>(null);
  error = $state<string | null>(null);
  /** 分類と提案に外部の AI を使うか (偽なら名前の規則とテンプレートの既定値)。 */
  useAi = $state(false);

  #doc: EditorDocument;
  /** プレビューを作ったときのドキュメントの版 (変わったらプレビューをやめる)。 */
  #previewRevision = -1;

  constructor(doc: EditorDocument) {
    this.#doc = doc;
  }

  /** 採用を待っている提案の数 (タブの印)。 */
  get count(): number {
    return this.value?.proposals.length ?? 0;
  }

  excludedOf(id: number): ReadonlySet<number> {
    return this.excluded[id] ?? new Set();
  }

  async refresh(): Promise<void> {
    await this.#task(null, async () => {
      this.value = await ipc.getProposals();
    });
  }

  /** ドキュメントが読み直されたとき (App が呼ぶ)。プレビューをやめ、古くなった提案の印を取り直す。 */
  documentChanged(revision: number): void {
    if (this.preview && revision !== this.#previewRevision) this.endPreview();
    void this.refresh();
  }

  /** 分類する PSD を選んで分類する。 */
  async choosePsd(): Promise<void> {
    const current = this.value?.psd;
    const path = await fileDialogs().openFile({
      title: "Choose the PSD to classify",
      defaultPath: (current && parentDir(current)) || undefined,
      filters: [PSD_FILTER],
    });
    if (path) await this.classify(path);
  }

  /** PSD のレイヤーを分類する (`psd` を省略すると取り込んだ PSD)。 */
  async classify(psd: string | null = null): Promise<boolean> {
    const useAi = this.useAi;
    return this.#task(
      useAi ? "Classifying the layers with Claude…" : "Classifying the layers by name…",
      async () => {
        const reply = await withConsent(
          (consent: ConsentAnswer | null) => ipc.classifyPsdLayers(psd, useAi, consent),
          consentPrompt.ask,
        );
        if (!reply) return false;
        this.value = await ipc.getProposals();
        return true;
      },
    );
  }

  /** 分類の結果からリグを提案する。まだ分類していなければ先に分類する (2 段階)。 */
  async propose(): Promise<void> {
    if (!this.value?.classification) {
      const ok = await this.classify();
      if (!ok) return;
    }
    const useAi = this.useAi;
    await this.#task(
      useAi
        ? "Asking Claude for the rig values (it may ask again if an answer fails validation)…"
        : "Building the rig from the templates…",
      async () => {
        const reply = await withConsent(
          (consent: ConsentAnswer | null) => ipc.proposeRigFromPsd(useAi, consent),
          consentPrompt.ask,
        );
        if (!reply) return;
        this.value = await ipc.getProposals();
      },
    );
  }

  /** コマンドを外す・戻す (依存するものも一緒に)。プレビュー中ならプレビューも作り直す。 */
  async toggleCommand(id: number, index: number): Promise<void> {
    const proposal = this.value?.proposals.find((p) => p.id === id);
    if (!proposal) return;
    this.excluded = {
      ...this.excluded,
      [id]: toggleCommand(proposal.commands, this.excludedOf(id), index),
    };
    if (this.preview?.id === id) await this.showPreview(id);
  }

  /** 同じ対象へのコマンドのまとまりをまとめて外す・戻す。 */
  async toggleGroup(id: number, group: CommandGroup): Promise<void> {
    const proposal = this.value?.proposals.find((p) => p.id === id);
    if (!proposal) return;
    this.excluded = {
      ...this.excluded,
      [id]: toggleGroup(proposal.commands, this.excludedOf(id), group),
    };
    if (this.preview?.id === id) await this.showPreview(id);
  }

  /** 提案を複製に適用した姿をビューポートに出す。 */
  async showPreview(id: number): Promise<void> {
    await this.#task("Preparing the preview…", async () => {
      const result = await ipc.previewProposal(id, excludedList(this.excludedOf(id)));
      const bundle = await ipc.getProposalPreviewBundle().catch(() => null);
      const parsed = JSON.parse(result.model_json) as Model;
      this.#previewRevision = this.#doc.revision;
      this.preview = {
        id,
        model: { json: result.model_json, bundle },
        parameters: parsed.parameters,
      };
    });
  }

  endPreview(): void {
    if (!this.preview) return;
    this.preview = null;
    ipc.clearProposalPreview().catch(() => {});
  }

  async togglePreview(id: number): Promise<void> {
    if (this.preview?.id === id) this.endPreview();
    else await this.showPreview(id);
  }

  /** 採用する (外したコマンドを除いて 1 回の Undo で戻る編集として)。 */
  async adopt(id: number): Promise<void> {
    const excluded = excludedList(this.excludedOf(id));
    this.endPreview();
    await this.#task("Applying the proposal…", async () => {
      const report = await this.#doc.applyWith(() => ipc.adoptProposal(id, excluded));
      if (!report) throw new Error(this.#doc.error ?? "the proposal could not be applied");
      this.value = await ipc.getProposals();
    });
  }

  async reject(id: number): Promise<void> {
    if (this.preview?.id === id) this.endPreview();
    await this.#task(null, async () => {
      this.value = await ipc.rejectProposal(id);
    });
  }

  /** 失敗を `error` に出す。`label` があれば実行中に `busy` に出す。 */
  async #task<T>(label: string | null, action: () => Promise<T>): Promise<T | false> {
    if (label) this.busy = label;
    try {
      const result = await action();
      this.error = null;
      return result;
    } catch (e) {
      this.error = String(e instanceof Error ? e.message : e);
      return false;
    } finally {
      if (label) this.busy = null;
    }
  }
}
