// AI の提案のパネル (ProposalsPanel.svelte) とビューポートが共有する状態 (docs/editor.md#ai-の提案)。
// 提案はバックエンド (apps/editor/src-tauri/src/proposals.rs) が持ち、ここは表示と操作だけ。
//
// - 分類と提案は、AI を使うなら `withConsent` で送る前に確かめる。使わないなら何も送らない。
// - プレビューは、提案を複製に適用した姿を `preview.model` に置き、App がビューポートに渡す (ドキュメントは変えない)。
//   ドキュメントが変わったら (編集・Undo・採用) プレビューをやめる。
// - 外したコマンドの番号 (`excluded`) は提案ごとに持ち、プレビューと採用に渡す (一部だけの採用)。
// - 利用者の注文 (`instructions`) は AI を使うときだけ分類・提案・作り直しに添える。プロジェクトには保存しない
//   (エディタを開いている間だけ。docs/editor.md#ai-への注文と作り直し)。
// - 法線マップの提案 (AI なし): まとまりの一覧 (`normalGroups`) を読み、まとまりごとに含めるか・強さを選んで
//   (`normalPicks`) 提案する。提案のコマンドはまとまりごとに区切って出す (docs/editor.md#法線マップの提案)。
// - 分類の直し (`correctLayer`・`resetLayer`): ラベル・左右を直す・「これでよい」と認めるのは `set_layer_correction` の
//   コマンドで、1 回の Undo で戻り、プロジェクトに残る (docs/editor.md#分類の直し)。
// - AI に送っている間 (`sending`) は「取り消す」(`cancel`) で止められる。取り消したらエラーではなく「取り消しました」と
//   出す (`notice`。docs/ai-integration.md#取り消しと時間の上限)。

import type { EditorDocument } from "../document.svelte";
import { errorCode, errorText } from "../errors";
import { fileDialogs } from "../fileDialogs";
import { PSD_FILTER, parentDir } from "../filePaths";
import { t } from "../i18n";
import {
  type ConsentAnswer,
  ipc,
  type LayerSide,
  type Model,
  type ModuleRigChoice,
  type NormalMapGroupView,
  type Parameter,
  type ProposalState,
} from "../ipc";
import type { ViewportController, ViewportModel } from "../viewport/controller.svelte";
import { aiSettings, consentPrompt } from "./ai.svelte";
import { withConsent } from "./consent";
import {
  type CommandGroup,
  excludedList,
  groupCommands,
  instructionsToSend,
  type NormalMapPick,
  normalMapChoices,
  normalMapPicks,
  sectionGroups,
  toggleCommand,
  toggleGroup,
} from "./proposals";
import { providerLabel } from "./providers";

/** プレビュー中に物理を動かすためのビューポートの操作 (ViewportController)。 */
export type PhysicsPreview = Pick<
  ViewportController,
  "previewingPhysics" | "startPhysicsPreview" | "stopPhysicsPreview"
>;

/** 選んだ AI のプロバイダの名前 (進み具合の文言に使う)。 */
function aiName(): string {
  return aiSettings.value ? providerLabel(aiSettings.value) : t("proposals.aiName.inSentence");
}

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
  /** 失敗ではない知らせ (取り消したなど)。次の操作で消える。 */
  notice = $state<string | null>(null);
  /** AI に送っている (取り消せる) 間。 */
  sending = $state(false);
  /** 取り消しを頼んだ後、送信が終わるまで。 */
  cancelling = $state(false);
  /** 分類と提案に外部の AI を使うか (偽なら名前の規則とテンプレートの既定値)。 */
  useAi = $state(false);
  /** タブを開いていない間に、エディタにつないだエージェントが提案を置いた (タブに印を出す)。 */
  fresh = $state(false);
  /** AI への注文 (複数行の自由な文)。 */
  instructions = $state("");
  /** 作り直すときに、レイヤーも注文を添えて分類し直すか (レイヤーの画像をもう一度送る)。 */
  reclassify = $state(false);
  /**
   * モジュールのプロジェクトのリグの組み方 (AI には送らない。提案と作り直しに渡す。モデルのプロジェクトでは使わない)。
   * docs/editor.md#モジュールのプロジェクトでの-ai-の提案
   */
  moduleOptions = $state<ModuleRigChoice>({ direct: false, colliders: true, poses: true });
  /** 法線マップを作るテクスチャのまとまり (読む前・読めなければ null)。 */
  normalGroups = $state.raw<NormalMapGroupView[] | null>(null);
  /** まとまりごとの選び方 (鍵は `normalGroupKey`)。 */
  normalPicks = $state<Record<string, NormalMapPick>>({});
  /** 法線マップを持つテクスチャも作り直すか。 */
  replaceNormalMaps = $state(false);

  #doc: EditorDocument;
  #viewport: PhysicsPreview | null;
  /** プレビューを作ったときのドキュメントの版 (変わったらプレビューをやめる)。 */
  #previewRevision = -1;
  /**
   * 始めたプレビューの数。プレビューの IPC はメインのスレッドの外で並んで動くので (docs/editor.md)、続けて外す・戻すと
   * 前に始めたものが後から終わることがある。最後に始めたものだけが結果を置く。
   */
  #previewGeneration = 0;
  /** プレビューのために物理を動かし始めた (プレビューをやめるときに止める)。 */
  #physics = false;

  constructor(doc: EditorDocument, viewport: PhysicsPreview | null = null) {
    this.#doc = doc;
    this.#viewport = viewport;
  }

  /** プレビュー中に物理が動いているか。 */
  get physicsRunning(): boolean {
    return this.preview !== null && (this.#viewport?.previewingPhysics ?? false);
  }

  /**
   * プレビュー中の提案の物理 (揺れ) を動かす・止める。採用の前に揺れを確かめるため (スライダーを動かすと揺れる)。
   * プレビューをやめると止まる。
   */
  togglePhysics(): void {
    const viewport = this.#viewport;
    if (!viewport || !this.preview) return;
    if (viewport.previewingPhysics) {
      viewport.stopPhysicsPreview();
      this.#physics = false;
    } else {
      viewport.startPhysicsPreview();
      this.#physics = viewport.previewingPhysics;
    }
  }

  /** 採用を待っている提案の数 (タブの印)。 */
  get count(): number {
    return this.value?.proposals.length ?? 0;
  }

  /**
   * 本体の変形を写した後に形を編集した衣装のメッシュ (提案し直すよう知らせる。タブにも印を出す)。
   * docs/editor.md#モジュールのプロジェクトでの-ai-の提案
   */
  get staleRigs(): { mesh: string; name: string }[] {
    return this.value?.stale_rigs ?? [];
  }

  /** 形を編集した衣装のメッシュの知らせを消す (提案し直さずに今の形のまま使う)。 */
  async dismissStaleRigs(): Promise<void> {
    await this.#task(null, async () => {
      this.value = await ipc.dismissStaleRigs();
    });
  }

  excludedOf(id: number): ReadonlySet<number> {
    return this.excluded[id] ?? new Set();
  }

  async refresh(): Promise<void> {
    await this.#task(null, async () => {
      this.value = await ipc.getProposals();
    });
    await this.loadNormalGroups();
  }

  /** 提案 `id` のコマンドの一覧のまとまり (区切りがあればそれ、なければ同じ対象への続いたコマンド)。 */
  commandGroups(id: number): CommandGroup[] {
    const sections = this.value?.sections.find((s) => s.id === id);
    if (sections) return sectionGroups(sections.sections);
    const proposal = this.value?.proposals.find((p) => p.id === id);
    return proposal ? groupCommands(proposal.commands) : [];
  }

  /** 法線マップのまとまりを読み直す (ラベルは分類と編集で変わる)。選び方は残す。 */
  async loadNormalGroups(): Promise<void> {
    try {
      const groups = await ipc.getNormalMapGroups();
      this.normalGroups = groups;
      this.normalPicks = normalMapPicks(groups, this.normalPicks);
    } catch {
      this.normalGroups = null;
    }
  }

  /** 選んだまとまりの法線マップを提案する (AI なし。何も送らない)。 */
  async proposeNormalMaps(): Promise<void> {
    const groups = this.normalGroups;
    if (!groups) return;
    const choices = normalMapChoices(groups, this.normalPicks);
    const replace = this.replaceNormalMaps;
    await this.#task(t("proposals.busy.normalMaps"), async () => {
      await ipc.proposeNormalMaps(choices, replace);
      this.value = await ipc.getProposals();
    });
  }

  /** ドキュメントが読み直されたとき (App が呼ぶ)。プレビューをやめ、古くなった提案の印を取り直す。 */
  documentChanged(revision: number): void {
    if (this.preview && revision !== this.#previewRevision) this.endPreview();
    void this.refresh();
  }

  /** エディタにつないだエージェントが提案を置いた (lib/agents/)。一覧を取り直し、タブを開いていなければ印を出す。 */
  async agentProposed(): Promise<void> {
    if (!this.active) this.fresh = true;
    await this.refresh();
  }

  /**
   * エージェントが適用を求めた提案を、求めと同じコマンドを外してプレビューする (確認のダイアログの「Show in AI tab」)。
   * タブを開くのは呼び出し側。
   */
  async previewRequested(id: number, excluded: number[]): Promise<void> {
    this.excluded = { ...this.excluded, [id]: new Set(excluded) };
    await this.refresh();
    await this.showPreview(id);
  }

  /** 分類する PSD を選んで分類する。 */
  async choosePsd(): Promise<void> {
    const current = this.value?.psd;
    const path = await fileDialogs().openFile({
      title: t("proposals.psd.dialogTitle"),
      defaultPath: (current && parentDir(current)) || undefined,
      filters: [PSD_FILTER],
    });
    if (path) await this.classify(path);
  }

  /**
   * PSD のレイヤーを分類する (`psd` を省略すると取り込んだ PSD)。`revise` なら今の分類を前の答えとして AI に送る
   * (作り直しで「レイヤーも分類し直す」とき)。
   */
  async classify(psd: string | null = null, revise = false): Promise<boolean> {
    const useAi = this.useAi;
    const instructions = instructionsToSend(this.instructions, useAi);
    return this.#task(
      useAi
        ? t("proposals.busy.classifyingAi", { name: aiName() })
        : t("proposals.busy.classifyingNames"),
      async () => {
        const reply = await withConsent(
          (consent: ConsentAnswer | null) =>
            ipc.classifyPsdLayers(psd, useAi, instructions, revise, consent),
          consentPrompt.ask,
        );
        if (!reply) return false;
        this.value = await ipc.getProposals();
        // 分類はまとまりのラベルを変える。
        await this.loadNormalGroups();
        return true;
      },
      useAi,
    );
  }

  /** 分類の結果からリグを提案する。まだ分類していなければ先に分類する (2 段階)。 */
  async propose(): Promise<void> {
    if (!this.value?.classification) {
      const ok = await this.classify();
      if (!ok) return;
    }
    const useAi = this.useAi;
    const instructions = instructionsToSend(this.instructions, useAi);
    await this.#task(
      useAi ? t("proposals.busy.askingAi", { name: aiName() }) : t("proposals.busy.building"),
      async () => {
        const reply = await withConsent(
          (consent: ConsentAnswer | null) =>
            ipc.proposeRigFromPsd(useAi, instructions, consent, { ...this.moduleOptions }),
          consentPrompt.ask,
        );
        if (!reply) return;
        this.value = await ipc.getProposals();
      },
      useAi,
    );
  }

  /**
   * 提案を、前の答えと注文を添えて AI に作り直させる。新しい提案は前の提案の後ろに並ぶ (前の提案は残る)。
   * `reclassify` なら先にレイヤーも今の分類を前の答えとして分類し直す (確認はそれぞれに出る)。
   */
  async regenerate(id: number): Promise<void> {
    if (!this.useAi) return;
    const instructions = instructionsToSend(this.instructions, true);
    if (this.reclassify) {
      const ok = await this.classify(null, true);
      if (!ok) return;
    }
    await this.#task(
      t("proposals.busy.revising", { name: aiName(), id }),
      async () => {
        const reply = await withConsent(
          (consent: ConsentAnswer | null) =>
            ipc.regenerateRigProposal(id, instructions, consent, { ...this.moduleOptions }),
          consentPrompt.ask,
        );
        if (!reply) return;
        this.value = await ipc.getProposals();
      },
      true,
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
    const revision = this.#doc.revision;
    const generation = ++this.#previewGeneration;
    await this.#task(t("proposals.busy.preparing"), async () => {
      const result = await ipc.previewProposal(id, excludedList(this.excludedOf(id)));
      const bundle = await ipc.getProposalPreviewBundle().catch(() => null);
      // 後から始めたプレビューがあれば、それに任せる (バックエンドの複製もそちらで置き換わる)。
      if (generation !== this.#previewGeneration) return;
      // 作る間にドキュメントが変わった (エージェントの適用・Undo など) なら、古いドキュメントの姿は出さない。
      if (this.#doc.revision !== revision) {
        ipc.clearProposalPreview().catch(() => {});
        return;
      }
      const parsed = JSON.parse(result.model_json) as Model;
      this.#previewRevision = revision;
      this.preview = {
        id,
        model: { json: result.model_json, bundle },
        parameters: parsed.parameters,
      };
    });
  }

  endPreview(): void {
    // 作っている途中のプレビューも出さない
    this.#previewGeneration += 1;
    if (!this.preview) return;
    if (this.#physics) {
      this.#physics = false;
      this.#viewport?.stopPhysicsPreview();
    }
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
    await this.#task(t("proposals.busy.applying"), async () => {
      const report = await this.#doc.applyWith(() => ipc.adoptProposal(id, excluded));
      if (!report) throw new Error(this.#doc.error ?? t("proposals.adoptFailed"));
      this.value = await ipc.getProposals();
    });
  }

  /**
   * レイヤー `path` の分類を `label`・`side` にする (直す・今の答えを「これでよい」と認める)。1 回の Undo で戻る。
   * 直したレイヤーは確からしさ 1 になり、食い違いの印が消える。
   */
  async correctLayer(path: string[], label: string, side: LayerSide): Promise<void> {
    await this.#setCorrection(path, { label, side });
  }

  /** レイヤー `path` の直しを消し、分類の答えに戻す。1 回の Undo で戻る。 */
  async resetLayer(path: string[]): Promise<void> {
    await this.#setCorrection(path, null);
  }

  async #setCorrection(
    path: string[],
    correction: { label: string; side: LayerSide } | null,
  ): Promise<void> {
    await this.#task(null, async () => {
      const report = await this.#doc.send(() => ({
        op: "set_layer_correction",
        path,
        correction,
      }));
      if (!report) throw new Error(this.#doc.error ?? t("proposals.correct.failed"));
      this.value = await ipc.getProposals();
    });
  }

  async reject(id: number): Promise<void> {
    if (this.preview?.id === id) this.endPreview();
    await this.#task(null, async () => {
      this.value = await ipc.rejectProposal(id);
    });
  }

  /**
   * 実行中の AI への送信を取り消す (`sending` の間だけ)。送信は応答を待たずに `ai.cancelled` で終わり、
   * `notice` に「取り消しました」と出る。
   */
  async cancel(): Promise<void> {
    if (!this.sending || this.cancelling) return;
    this.cancelling = true;
    try {
      await ipc.cancelAiRequests();
    } catch (e) {
      this.cancelling = false;
      this.error = errorText(e);
    }
  }

  /**
   * 失敗を `error` に出す。`label` があれば実行中に `busy` に出す。`sending` なら AI に送る操作で、実行中は取り消せる
   * (取り消して終わったら、エラーではなく `notice` に出す)。
   */
  async #task<T>(
    label: string | null,
    action: () => Promise<T>,
    sending = false,
  ): Promise<T | false> {
    if (label) {
      this.busy = label;
      this.notice = null;
    }
    if (sending) {
      this.sending = true;
      this.cancelling = false;
    }
    try {
      const result = await action();
      this.error = null;
      return result;
    } catch (e) {
      if (errorCode(e) === "ai.cancelled") {
        this.error = null;
        this.notice = t("proposals.cancelled");
      } else {
        this.error = errorText(e);
      }
      return false;
    } finally {
      if (label) this.busy = null;
      if (sending) {
        this.sending = false;
        this.cancelling = false;
      }
    }
  }
}
