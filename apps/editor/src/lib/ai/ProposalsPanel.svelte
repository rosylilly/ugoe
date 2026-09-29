<!--
  AI の提案のパネル (docs/editor.md#ai-の提案)。右の列の「AI」タブ。
  - 提案の元にする PSD (「New from PSD…」「Import PSD…」で取り込んだもの。「Choose PSD…」で替えられる) の
    レイヤーを分類し (「Classify layers」)、分類からリグ (顔の角度・目・口) を提案する (「Propose rig」)。
    「Use Claude」なら外部の AI に画像を送る (送る前に確認のダイアログ)。使えないときは理由と設定への案内を出す。
  - 提案ごとに題名・理由・コマンドの一覧 (同じ対象への続いたコマンドはまとめる) を出す。コマンド (まとまり) の
    チェックを外すと、それに頼るコマンドも外れる (lib/ai/proposals.ts)。「Preview」は提案を複製に適用した姿をビューポートに出し (ドキュメントは変えない)、
    提案が足したパラメータのスライダーをここに出す。「Adopt」は 1 回の Undo で戻る編集として適用する。
-->
<script lang="ts">
import type { ViewportController } from "../viewport/controller.svelte";
import { aiSettings } from "./ai.svelte";
import { classificationSummary, commandLabel, groupCommands, groupState } from "./proposals";
import type { ProposalsView } from "./proposals.svelte";

interface Props {
  view: ProposalsView;
  viewport: ViewportController;
  /** AI の設定 (API キー) の画面を開く。 */
  onopensettings: () => void;
}

let { view, viewport, onopensettings }: Props = $props();

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

/** 外部の AI を使えない理由 (使えるなら null)。 */
const aiBlocked = $derived.by((): string | null => {
  const s = aiSettings.value;
  if (!s) return "Checking the AI settings…";
  if (!s.send_externally) return "This project is set not to send data to external AI services.";
  if (s.key !== "stored" && !s.env_key) {
    return "No Anthropic API key. Add one in Settings, or set ANTHROPIC_API_KEY before starting the editor.";
  }
  return null;
});

$effect(() => {
  if (aiBlocked && view.useAi) view.useAi = false;
});

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function onSlider(id: string, e: Event): void {
  viewport.setParameter(id, (e.currentTarget as HTMLInputElement).valueAsNumber);
}
</script>

<section>
  <h3>Auto rig from PSD</h3>
  <div class="row">
    {#if panel?.psd}
      <span class="psd" title={panel.psd}>{fileName(panel.psd)}</span>
    {:else}
      <span class="muted">No PSD yet. Use File → New from PSD…, or choose one.</span>
    {/if}
    <button onclick={() => view.choosePsd()} disabled={busy}>Choose PSD…</button>
  </div>
  <label class="check" title={aiBlocked ?? "Send the artwork to Claude to classify the layers and pick the rig values"}>
    <input type="checkbox" bind:checked={view.useAi} disabled={!!aiBlocked || busy} />
    Use Claude (sends images of the artwork)
  </label>
  {#if aiBlocked && aiSettings.value}
    <p class="muted note">
      {aiBlocked} Without AI, layers are classified by name and the rig uses the template defaults.
      <button class="link" onclick={onopensettings}>Settings…</button>
    </p>
  {/if}
  <div class="row">
    <button onclick={() => view.classify()} disabled={busy || !panel?.psd}>Classify layers</button>
    <button class="primary" onclick={() => view.propose()} disabled={busy || !panel?.psd}>
      Propose rig
    </button>
  </div>
  {#if view.busy}<p class="progress" role="status">{view.busy}</p>{/if}
  {#if view.error}<p class="error">{view.error}</p>{/if}
  {#if summary && panel?.classification}
    <p class="muted">
      {summary.layers} layers classified {panel.classification.by_ai ? "by Claude" : "by name"}
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
      <article class="proposal" class:previewing>
        <header>
          <strong>{proposal.title}</strong>
          {#if proposal.stale}
            <span class="warn" title="The model changed after this proposal was made. Undo back to that state, or propose again.">
              outdated
            </span>
          {/if}
        </header>
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
    justify-content: space-between;
    gap: 6px;
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
