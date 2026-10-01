<!--
  AI の提案のパネル (docs/editor.md#ai-の提案)。右の列の「AI」タブ。
  - 提案の元にする PSD (「New from PSD…」「Import PSD…」で取り込んだもの。「Choose PSD…」で替えられる) の
    レイヤーを分類し (「Classify layers」)、分類からリグ (顔の角度・目・口) を提案する (「Propose rig」)。
    モジュールのプロジェクト (`module`) では、衣装を本体の変形に追従させ、揺れるラベルの衣装に物理を付ける提案になる
    (パラメータを足さない。docs/editor.md#モジュールのプロジェクトでの-ai-の提案)。組み方 (追従の形・衝突体・差分のポーズ) は
    「How to rig」で選ぶ (`view.moduleOptions`。AI には送らない)。
    「Use <プロバイダ>」なら選んだ AI のプロバイダに画像を送る (外部に出るなら送る前に確認のダイアログ)。
    使えないときは理由と設定への案内を出す。
  - エディタにつないだエージェント (lib/agents/) の提案は「from <名前>」の印を付けて同じ一覧に出す。
  - 提案ごとに題名・理由・コマンドの一覧 (同じ対象への続いたコマンドはまとめる) を出す。コマンド (まとまり) の
    チェックを外すと、それに頼るコマンドも外れる (lib/ai/proposals.ts)。「Preview」は提案を複製に適用した姿をビューポートに出し (ドキュメントは変えない)、
    提案が足したパラメータのスライダーをここに出す。「Run physics」でプレビューの物理を動かす (揺れを確かめる)。
    「Adopt」は 1 回の Undo で戻る編集として適用する。
  - AI への注文 (「Instructions for <プロバイダ>」): AI を使うときだけ分類・提案に添える。提案ごとの
    「Regenerate with instructions」は前の答えと注文を送って作り直し、新しい提案を後ろに並べる (前の提案は残る)。
    提案には添えた注文と、AI が反映できないと答えた注文 (「Not applied」) を出す。注文はプロジェクトに保存しない
    (docs/editor.md#ai-への注文と作り直し)。
  - 「Normal maps from the shapes」(モデルのプロジェクトだけ): 部位のまとまり (顔と目口鼻眉・首と体と腕・髪など。意味ラベルから
    決める) ごとに含めるか・強さ・縁の丸み・ならしを選び、すべてのテクスチャの法線マップを 1 つの提案にする (AI なし。何も送らない)。
    縁の幅とならしはキャンバスの px で見せる (単独でふくらませるものはテクスチャの px。panels/lighting/lighting.ts)。
    提案のコマンドはまとまりごとに区切って出す (まとまりを外して採用できる)。docs/editor.md#法線マップの提案
-->
<script lang="ts">
import { bulgeForm, bulgeLimits, bulgeUnit } from "../panels/lighting/lighting";
import type { ViewportController } from "../viewport/controller.svelte";
import { aiSettings } from "./ai.svelte";
import {
  classificationSummary,
  commandLabel,
  groupState,
  instructionChars,
  MAX_INSTRUCTION_CHARS,
  normalGroupKey,
  normalMapTargetCount,
  surfaceGroupLabel,
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

/** 法線マップを作るテクスチャの数 (選んだまとまり)。 */
const normalTargets = $derived(
  view.normalGroups
    ? normalMapTargetCount(view.normalGroups, view.normalPicks, view.replaceNormalMaps)
    : 0,
);
/** 法線マップのまとまりのラベルの出どころ。 */
const normalFromClassification = $derived(view.normalGroups?.[0]?.from_classification ?? false);

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function onSlider(id: string, e: Event): void {
  viewport.setParameter(id, (e.currentTarget as HTMLInputElement).valueAsNumber);
}
</script>

<section>
  <h3>{module ? "Auto rig the costume from PSD" : "Auto rig from PSD"}</h3>
  {#if view.staleRigs.length > 0}
    <div class="unapplied stale" role="status">
      <strong>Propose the rig again:</strong> these costume meshes were edited (vertices, triangles or parent) after an
      adopted rig proposal wrote the base's motion into them, so they no longer follow the base exactly.
      <ul>
        {#each view.staleRigs as stale (stale.mesh)}<li>{stale.name}</li>{/each}
      </ul>
      <button class="link" disabled={busy} onclick={() => view.dismissStaleRigs()}
        title="Keep the meshes as they are and hide this notice">Dismiss</button>
    </div>
  {/if}
  {#if module}
    <p class="muted note">
      Module project: the rig uses only the base's parameters. The costume follows the base deformers under the
      slot's anchor (for example the face turn) and base meshes that change shape under it (for example closing
      eyes), and hair, tails, earrings, neckwear and skirts sway with physics. No parameters are added.
    </p>
    <fieldset class="options" disabled={busy}>
      <legend class="note">How to rig (not sent to the AI)</legend>
      <label class="check note"
        title="Off: wrap the costume in a warp that approximates the base warp (keeps working if you redraw the costume). On: write the base warp's motion into the costume meshes' keyforms (exact at the keys; propose again after editing the mesh).">
        <input type="checkbox" bind:checked={view.moduleOptions.direct} />
        Follow the base warps exactly (write into the costume meshes)
      </label>
      <label class="check note" title="Swaying parts collide with the base colliders that the slot exposes, so they do not sink into the body">
        <input type="checkbox" bind:checked={view.moduleOptions.colliders} />
        Keep swaying parts out of the base's colliders
      </label>
      <label class="check note" title="Costume parts whose names differ only by a trailing A, B… (Sleeve A / Sleeve B) become a pose, following the base's variants when the slot exposes them. So do overlapping parts with the same label where the PSD shows one and hides the others.">
        <input type="checkbox" bind:checked={view.moduleOptions.poses} />
        Turn A / B variants into poses
      </label>
    </fieldset>
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
      {#if summary.variants.length > 0}
        <p class="muted note">Alternative drawings of the same part (one shows at a time; costume rigs make pose groups):</p>
        <ul class="labels">
          {#each summary.variants as names, i (i)}<li>{names.join(" / ")}</li>{/each}
        </ul>
      {/if}
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

{#if !module}
  <section aria-label="Normal maps from the shapes">
    <h3>Normal maps from the shapes</h3>
    <p class="muted note">
      No AI; nothing is sent. Each group of parts bulges as one surface, so eyes follow the face and the neck joins the
      body. Glasses take the face's direction without bulging. Groups come from
      {normalFromClassification ? "the layer classification above" : "the layer names (classify the layers to use AI labels)"}.
    </p>
    {#if view.normalGroups && view.normalGroups.length === 0}
      <p class="muted">No textures to make normal maps for.</p>
    {:else if view.normalGroups}
      <ul class="normal-groups">
        {#each view.normalGroups as group (normalGroupKey(group))}
          {@const key = normalGroupKey(group)}
          {@const pick = view.normalPicks[key]}
          {@const unit = bulgeUnit(group)}
          {@const limits = bulgeLimits(group)}
          {#if pick}
            <li>
              <label class="check" title={group.textures.join(", ")}>
                <input type="checkbox" bind:checked={pick.include} disabled={busy} />
                <span class="group-name">{surfaceGroupLabel(key)}</span>
              </label>
              <span class="muted counts">
                {group.textures.length} {group.textures.length === 1 ? "texture" : "textures"}{#if group.with_normal_map.length > 0}, {group.with_normal_map.length} mapped{/if}{#if group.oriented.length > 0}, {group.oriented.length} direction only{/if}
              </span>
              <span class="strength">
                <input
                  type="range"
                  min="0"
                  max="2"
                  step="0.05"
                  bind:value={pick.strength}
                  disabled={busy || !pick.include}
                  aria-label="Strength of {surfaceGroupLabel(key)}"
                />
                <span class="value">{pick.strength.toFixed(2)}</span>
              </span>
              <details class="shape">
                <summary class="muted">
                  {pick.roundEdges ? `edges ${Math.round(pick.edge)} ${unit}` : "whole shape"} · smoothing
                  {pick.smoothing.toFixed(1)} {unit}
                </summary>
                <label class="check">
                  <input type="checkbox" bind:checked={pick.roundEdges} disabled={busy || !pick.include} />
                  Round edges only
                </label>
                {#if pick.roundEdges}
                  <label class="setting">
                    <span>Edge width ({unit})</span>
                    <input
                      type="range"
                      min="1"
                      max={limits.edge}
                      step="1"
                      bind:value={pick.edge}
                      disabled={busy || !pick.include}
                      aria-label="Edge width of {surfaceGroupLabel(key)}"
                    />
                    <span class="value">{Math.round(pick.edge)}</span>
                  </label>
                {/if}
                <label class="setting">
                  <span>Smoothing ({unit})</span>
                  <input
                    type="range"
                    min="0"
                    max={limits.smoothing}
                    step="0.5"
                    bind:value={pick.smoothing}
                    disabled={busy || !pick.include}
                    aria-label="Smoothing of {surfaceGroupLabel(key)}"
                  />
                  <span class="value">{pick.smoothing.toFixed(1)}</span>
                </label>
                <button
                  disabled={busy}
                  onclick={() => Object.assign(pick, bulgeForm(group))}
                  title="Use this group's default strength, edges and smoothing"
                >Defaults</button>
              </details>
            </li>
          {/if}
        {/each}
      </ul>
      <label class="check note">
        <input type="checkbox" bind:checked={view.replaceNormalMaps} disabled={busy} />
        Replace existing normal maps
      </label>
      <div class="row">
        <button
          class="primary"
          onclick={() => view.proposeNormalMaps()}
          disabled={busy || normalTargets === 0}
          title="Make normal maps for the chosen groups as a proposal. Preview it to see the model lit by the Lighting tab's preview lights."
        >Propose normal maps{normalTargets > 0 ? ` (${normalTargets})` : ""}</button>
      </div>
    {/if}
  </section>
{/if}

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
            {#each view.commandGroups(proposal.id) as group (group.indices[0])}
              {@const state = groupState(group, excluded)}
              {@const single =
                group.indices.length === 1 && !group.section ? proposal.commands[group.indices[0] as number] : null}
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
                      <summary><code>{group.label}</code> · {group.indices.length}
                        {group.indices.length === 1 ? "command" : "commands"}</summary>
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
                                <code>{group.section && command.subject ? `${command.op} ${command.subject}` : command.op}</code>
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
          <div class="row">
            <button
              class:active={view.physicsRunning}
              onclick={() => view.togglePhysics()}
              title="Run the proposal's physics in the viewport. Move the sliders to see the sway before adopting."
            >{view.physicsRunning ? "Stop physics" : "Run physics"}</button>
            {#if view.physicsRunning}<span class="muted note">Move the sliders to see the sway.</span>{/if}
          </div>
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
  .options {
    margin: 6px 0;
    padding: 2px 8px 4px;
    border: 1px solid var(--border);
    border-radius: 4px;
  }
  .options legend {
    padding: 0 4px;
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
  .normal-groups {
    list-style: none;
    margin: 4px 0;
    padding: 0;
    font-size: 12px;
  }
  .normal-groups li {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    column-gap: 6px;
    padding: 2px 0;
  }
  .normal-groups .counts {
    grid-column: 1;
    padding-left: 22px;
    font-size: 11px;
  }
  .normal-groups .strength {
    grid-column: 2;
    grid-row: 1 / span 2;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .normal-groups input[type="range"] {
    width: 90px;
  }
  .normal-groups .shape {
    grid-column: 1 / -1;
    padding-left: 22px;
    font-size: 11px;
  }
  .normal-groups .shape summary {
    cursor: pointer;
  }
  .normal-groups .setting {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: 4px;
  }
  .normal-groups .value {
    width: 32px;
    text-align: right;
    font-variant-numeric: tabular-nums;
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
