<!--
  AI の提案のパネル (docs/editor.md#ai-の提案)。右の列の「AI」タブ。
  - 提案の元にする PSD (「New from PSD…」「Import PSD…」で取り込んだもの。「Choose PSD…」で替えられる) の
    レイヤーを分類し (「Classify layers」)、分類からリグ (顔の角度・目・口) を提案する (「Propose rig」)。
    モジュールのプロジェクト (`module`) では、衣装を本体の変形に追従させ、揺れるラベルの衣装に物理を付ける提案になる
    (パラメータを足さない。docs/editor.md#モジュールのプロジェクトでの-ai-の提案)。
    「Use <プロバイダ>」なら選んだ AI のプロバイダに画像を送る (外部に出るなら送る前に確認のダイアログ)。
    使えないときは理由と設定への案内を出す。
  - エディタにつないだエージェント (lib/agents/) の提案は「from <名前>」の印を付けて同じ一覧に出す。
  - 提案ごとに題名・理由・コマンドの一覧 (同じ対象への続いたコマンドはまとめる) を出す。コマンド (まとまり) の
    チェックを外すと、それに頼るコマンドも外れる (lib/ai/proposals.ts)。「Preview」は提案を複製に適用した姿をビューポートに出し (ドキュメントは変えない)、
    提案が足したパラメータのスライダーをここに出す。「Adopt」は 1 回の Undo で戻る編集として適用する。
  - AI への注文 (「Instructions for <プロバイダ>」): AI を使うときだけ分類・提案に添える。提案ごとの
    「Regenerate with instructions」は前の答えと注文を送って作り直し、新しい提案を後ろに並べる (前の提案は残る)。
    提案には添えた注文と、AI が反映できないと答えた注文 (「Not applied」) を出す。注文はプロジェクトに保存しない
    (docs/editor.md#ai-への注文と作り直し)。
-->
<script lang="ts">
import type { ViewportController } from "../viewport/controller.svelte";
import { aiSettings } from "./ai.svelte";
import {
  classificationSummary,
  commandLabel,
  groupCommands,
  groupState,
  instructionChars,
  MAX_INSTRUCTION_CHARS,
} from "./proposals";
import type { ProposalsView } from "./proposals.svelte";
import { aiBlockedReason, providerLabel } from "./providers";

interface Props {
  view: ProposalsView;
  viewport: ViewportController;
  /** モジュールのプロジェクトか (提案の中身と説明が変わる)。 */
  module?: boolean;
  /** AI の設定 (API キー) の画面を開く。 */
  onopensettings: () => void;
}

let { view, viewport, module = false, onopensettings }: Props = $props();

$effect(() => {
  void aiSettings.refresh();
  void view.refresh();
});

const panel = $derived(view.value);
const summary = $derived(
  panel?.classification ? classificationSummary(panel.classification) : null,
);
const review = $derived(panel?.classification?.layers.filter((l) => l.needs_review) ?? []);
const busy = $derived(view.busy !== null);

/** AI を使えない理由 (使えるなら null)。 */
const aiBlocked = $derived.by((): string | null => {
  const s = aiSettings.value;
  if (!s) return "Checking the AI settings…";
  return aiBlockedReason(s);
});
/** 選んだプロバイダの名前。 */
const aiName = $derived(aiSettings.value ? providerLabel(aiSettings.value) : "AI");
/** 選んだプロバイダがこの計算機のサーバか (画像は外に出ない)。 */
const aiLocal = $derived(aiSettings.value?.local ?? false);

$effect(() => {
  if (aiBlocked && view.useAi) view.useAi = false;
});

/** 注文の文字数。 */
const chars = $derived(instructionChars(view.instructions));
const tooLong = $derived(chars > MAX_INSTRUCTION_CHARS);
/** 作り直せない理由 (作り直せるなら null)。 */
const regenerateBlocked = $derived.by((): string | null => {
  if (!view.useAi) return `Turn on "Use ${aiName}" to regenerate with instructions`;
  if (chars === 0) return "Write instructions for the AI above first";
  if (tooLong) return `The instructions are longer than ${MAX_INSTRUCTION_CHARS} characters`;
  return null;
});

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function onSlider(id: string, e: Event): void {
  viewport.setParameter(id, (e.currentTarget as HTMLInputElement).valueAsNumber);
}
</script>

<section>
  <h3>{module ? "Auto rig the costume from PSD" : "Auto rig from PSD"}</h3>
  {#if module}
    <p class="muted note">
      Module project: the rig uses only the base's parameters. The costume follows the base deformers under the
      slot's anchor (for example the face turn), and hair, tails, earrings, neckwear and skirts sway with physics.
      No parameters are added.
    </p>
  {/if}
  <div class="row">
    {#if panel?.psd}
      <span class="psd" title={panel.psd}>{fileName(panel.psd)}</span>
    {:else}
      <span class="muted">No PSD yet. Use File → New from PSD…, or choose one.</span>
    {/if}
    <button onclick={() => view.choosePsd()} disabled={busy}>Choose PSD…</button>
  </div>
  <label
    class="check"
    title={aiBlocked ?? `Send the artwork to ${aiName} to classify the layers and pick the rig values`}
  >
    <input type="checkbox" bind:checked={view.useAi} disabled={!!aiBlocked || busy} />
    Use {aiName} ({aiLocal ? "sends images to a server on this computer" : "sends images of the artwork"})
  </label>
  {#if aiBlocked && aiSettings.value}
    <p class="muted note">
      {aiBlocked} Without AI, layers are classified by name and the rig uses the template defaults.
      <button class="link" onclick={onopensettings}>Settings…</button>
    </p>
  {/if}
  <label class="instructions" for="ai-instructions">
    Instructions for {aiName} <span class="muted">(optional)</span>
  </label>
  <textarea
    id="ai-instructions"
    rows="3"
    bind:value={view.instructions}
    disabled={busy}
    placeholder={module
      ? "e.g. Let the earrings swing more. The ribbon is neckwear."
      : "e.g. Close the eyes a little narrower. Keep head turns subtle. The hat is headwear."}
    aria-describedby="ai-instructions-note"
  ></textarea>
  <p id="ai-instructions-note" class="muted note">
    <span class:error={tooLong}>{chars} / {MAX_INSTRUCTION_CHARS}</span>{" · "}
    {#if view.useAi}
      Sent with Classify, Propose rig and Regenerate.
    {:else}
      Only sent when "Use {aiName}" is on.
    {/if}
    {#if module}
      The AI can change layer labels and two sway values (how strongly and how slowly the swaying parts swing); it
      lists anything else as not applied. Not saved in the project.
    {:else}
      The AI can change layer labels and four rig values (head turn, where and how the eyes close, how far a single
      mouth opens); it lists anything else as not applied. Not saved in the project.
    {/if}
  </p>
  {#if view.useAi && (panel?.regenerable.length ?? 0) > 0}
    <label class="check note">
      <input type="checkbox" bind:checked={view.reclassify} disabled={busy} />
      When regenerating, also reclassify the layers with these instructions (sends the layer images again)
    </label>
  {/if}
  <div class="row">
    <button onclick={() => view.classify()} disabled={busy || !panel?.psd || (view.useAi && tooLong)}>
      Classify layers
    </button>
    <button class="primary" onclick={() => view.propose()} disabled={busy || !panel?.psd || (view.useAi && tooLong)}>
      Propose rig
    </button>
  </div>
  {#if view.busy}<p class="progress" role="status">{view.busy}</p>{/if}
  {#if view.error}<p class="error">{view.error}</p>{/if}
  {#if summary && panel?.classification}
    <p class="muted" title={panel.classification.instructions ? `With instructions: ${panel.classification.instructions}` : undefined}>
      {summary.layers} layers classified {panel.classification.by_ai ? "by AI" : "by name"}
      {#if panel.classification.instructions}{" "}with instructions{/if}
      {#if summary.review > 0}{" · "}<span class="warn">{summary.review} to review</span>{/if}
    </p>
    <details>
      <summary>Labels</summary>
      <ul class="labels">
        {#each summary.labels as [label, count] (label)}
          <li><code>{label}</code> × {count}</li>
        {/each}
      </ul>
      {#if review.length > 0}
        <p class="muted note">Low confidence (check these before adopting):</p>
        <ul class="labels">
          {#each review as layer, i (i)}
            <li>{layer.path.join(" / ")} → <code>{layer.label}</code> ({Math.round(layer.confidence * 100)}%)</li>
          {/each}
        </ul>
      {/if}
    </details>
  {/if}
</section>

<section>
  <h3>Proposals</h3>
  {#if !panel || panel.proposals.length === 0}
    <p class="muted">No proposals. Proposals change nothing until you adopt them.</p>
  {:else}
    {#each panel.proposals as proposal (proposal.id)}
      {@const excluded = view.excludedOf(proposal.id)}
      {@const selected = proposal.commands.length - excluded.size}
      {@const previewing = view.preview?.id === proposal.id}
      {@const regenerable = panel.regenerable.includes(proposal.id)}
      <article class="proposal" class:previewing aria-label="Proposal #{proposal.id}">
        <header>
          <strong><span class="index">#{proposal.id}</span> {proposal.title}</strong>
          {#if proposal.revises !== null}
            <span class="source" title="Regenerated from proposal #{proposal.revises}, which is kept so you can compare them"
              >revision of #{proposal.revises}</span>
          {/if}
          {#if proposal.source}
            <span class="source" title="Proposed by {proposal.source}, connected to this editor (Settings → Connect agents)"
              >from {proposal.source}</span>
          {/if}
          {#if proposal.stale}
            <span class="warn" title="The model changed after this proposal was made. Undo back to that state, or propose again.">
              outdated
            </span>
          {/if}
        </header>
        {#if proposal.instructions}
          <p class="note sent"><span class="muted">Instructions:</span> {proposal.instructions}</p>
        {/if}
        {#if proposal.unapplied.length > 0}
          <div class="unapplied" role="note">
            <strong>Not applied</strong> — the AI says the current rig template cannot do these:
            <ul>
              {#each proposal.unapplied as item, i (i)}
                <li>{item}</li>
              {/each}
            </ul>
          </div>
        {/if}
        <p class="rationale">{proposal.rationale}</p>
        <details>
          <summary>{selected} of {proposal.commands.length} commands selected</summary>
          <ul class="groups">
            {#each groupCommands(proposal.commands) as group (group.indices[0])}
              {@const state = groupState(group, excluded)}
              {@const single = group.indices.length === 1 ? proposal.commands[group.indices[0] as number] : null}
              <li>
                <div class="group">
                  <input
                    type="checkbox"
                    aria-label="Include {single ? commandLabel(single) : group.label}"
                    checked={state !== "none"}
                    indeterminate={state === "some"}
                    disabled={busy || proposal.stale}
                    onchange={() => view.toggleGroup(proposal.id, group)}
                  />
                  {#if single}
                    <span class="index">#{group.indices[0]}</span>
                    <code>{commandLabel(single)}</code>
                  {:else}
                    <details class="inner">
                      <summary><code>{group.label}</code> · {group.indices.length} commands</summary>
                      <ol class="commands">
                        {#each group.indices as i (i)}
                          {@const command = proposal.commands[i]}
                          {#if command}
                            <li>
                              <label class="check" title={command.requires.length > 0
                                ? `Depends on #${command.requires.join(", #")}`
                                : undefined}>
                                <input
                                  type="checkbox"
                                  checked={!excluded.has(i)}
                                  disabled={busy || proposal.stale}
                                  onchange={() => view.toggleCommand(proposal.id, i)}
                                />
                                <span class="index">#{i}</span>
                                <code>{command.op}</code>
                              </label>
                            </li>
                          {/if}
                        {/each}
                      </ol>
                    </details>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
        </details>
        <div class="row">
          <button
            class:active={previewing}
            onclick={() => view.togglePreview(proposal.id)}
            disabled={busy || proposal.stale || selected === 0}
            title="Show the model with this proposal applied, without changing it"
          >{previewing ? "End preview" : "Preview"}</button>
          <button
            class="primary"
            onclick={() => view.adopt(proposal.id)}
            disabled={busy || proposal.stale || selected === 0}
            title="Apply the selected commands as one edit (one Undo reverts it)"
          >Adopt{selected < proposal.commands.length ? ` ${selected}` : ""}</button>
          <button onclick={() => view.reject(proposal.id)} disabled={busy}>Reject</button>
          {#if regenerable}
            <button
              onclick={() => view.regenerate(proposal.id)}
              disabled={busy || !!regenerateBlocked}
              title={regenerateBlocked ??
                `Send this proposal's values and your instructions to ${aiName}, and add the result as a new proposal (this one is kept)`}
            >Regenerate with instructions</button>
          {/if}
        </div>
        {#if previewing && view.preview}
          <p class="muted note">Previewing. The model is unchanged until you adopt.</p>
          <div class="sliders">
            {#each view.preview.parameters as p (p.id)}
              <label>
                <span class="name" title={p.id}>{p.name}</span>
                <input
                  type="range"
                  min={p.min}
                  max={p.max}
                  step={(p.max - p.min) / 100}
                  value={viewport.parameterValues[p.id] ?? p.default}
                  oninput={(e) => onSlider(p.id, e)}
                />
              </label>
            {/each}
          </div>
        {/if}
      </article>
    {/each}
  {/if}
</section>

<style>
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin: 6px 0;
  }
  .psd {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 60%;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .note {
    font-size: 12px;
  }
  .instructions {
    display: block;
    margin-top: 6px;
  }
  textarea {
    box-sizing: border-box;
    width: 100%;
    resize: vertical;
    font: inherit;
    font-size: 12px;
  }
  .sent {
    margin: 4px 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .unapplied {
    margin: 4px 0;
    padding: 4px 6px;
    border-left: 3px solid #b45309;
    font-size: 12px;
  }
  .unapplied ul {
    list-style: disc;
    margin: 2px 0;
    padding-left: 18px;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
  }
  .progress {
    color: var(--accent);
  }
  .warn {
    color: #b45309;
  }
  .primary {
    color: var(--accent);
    border-color: var(--accent);
  }
  button.active {
    color: var(--accent);
    border-color: var(--accent);
  }
  .labels {
    margin: 4px 0;
    padding-left: 18px;
    font-size: 12px;
  }
  .proposal {
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 6px 8px;
    margin: 6px 0;
  }
  .proposal.previewing {
    border-color: var(--accent);
  }
  .proposal header {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 6px;
  }
  .source {
    padding: 0 6px;
    border: 1px solid var(--accent);
    border-radius: 8px;
    color: var(--accent);
    font-size: 11px;
    white-space: nowrap;
  }
  .rationale {
    max-height: 10em;
    overflow-y: auto;
    white-space: pre-wrap;
    font-size: 12px;
    margin: 4px 0;
  }
  .commands {
    list-style: none;
    margin: 2px 0;
    padding: 0;
  }
  .groups {
    list-style: none;
    margin: 4px 0;
    padding: 0;
    max-height: 320px;
    overflow-y: auto;
    font-size: 12px;
  }
  .group {
    display: flex;
    align-items: flex-start;
    gap: 6px;
  }
  .inner summary {
    cursor: pointer;
  }
  .index {
    color: var(--muted);
    min-width: 2.5em;
  }
  .sliders {
    display: grid;
    gap: 2px;
  }
  .sliders label {
    display: grid;
    grid-template-columns: 7em 1fr;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  .sliders .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
