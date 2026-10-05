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
  - 分類の確認の一覧 (確からしさが低い・名前と食い違う・直したレイヤー) では、ラベルと左右を直すか「Looks right」で
    今の答えを認める (印が消える)。直したものは「Reset」で分類の答えに戻す。どれも 1 回の Undo で戻り、プロジェクトに残る
    (`view.correctLayer`・`resetLayer`。docs/editor.md#分類の直し)。
  - AI に送っている間は進み具合の隣に「Cancel」を出す (`view.cancel`。応答を待たずに止め、「Cancelled」と出す。
    docs/ai-integration.md#取り消しと時間の上限)。
  - 「Normal maps from the shapes」(モデルのプロジェクトだけ): 部位のまとまり (顔と目口鼻眉・首と体と腕・髪など。意味ラベルから
    決める) ごとに含めるか・強さ・縁の丸み・ならしを選び、すべてのテクスチャの法線マップを 1 つの提案にする (AI なし。何も送らない)。
    縁の幅とならしはキャンバスの px で見せる (単独でふくらませるものはテクスチャの px。panels/lighting/lighting.ts)。
    提案のコマンドはまとまりごとに区切って出す (まとまりを外して採用できる)。docs/editor.md#法線マップの提案
-->
<script lang="ts">
import { formatList } from "@ugoe/i18n";
import { Button, Checkbox, Notice, SegmentedControl, Select, Slider, Tooltip } from "@ugoe/ui";
import { t } from "../i18n";
import type { LayerSide } from "../ipc";
import { bulgeForm, bulgeLimits, bulgeUnit } from "../panels/lighting/lighting";
import TipButton from "../ui/TipButton.svelte";
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

const uid = $props.id();

$effect(() => {
  void aiSettings.refresh();
  void view.refresh();
});

const panel = $derived(view.value);
const summary = $derived(
  panel?.classification ? classificationSummary(panel.classification) : null,
);
/** 確かめる・直したレイヤー (直すと確かめる印は消えるが、戻せるように一覧に残す)。 */
const review = $derived(
  panel?.classification?.layers.filter((l) => l.needs_review || l.corrected) ?? [],
);
/** 直すときに選べるラベル。 */
const labelOptions = $derived(
  (panel?.classification?.label_choices ?? []).map((l) => ({ value: l, label: l })),
);
/** 直すときの左右。 */
const sideOptions = $derived<{ value: LayerSide; label: string }[]>([
  { value: "left", label: t("proposals.correct.side.left") },
  { value: "center", label: t("proposals.correct.side.center") },
  { value: "right", label: t("proposals.correct.side.right") },
]);
const busy = $derived(view.busy !== null);

/** AI を使えない理由 (使えるなら null)。 */
const aiBlocked = $derived.by((): string | null => {
  const s = aiSettings.value;
  if (!s) return t("proposals.useAi.checking");
  return aiBlockedReason(s);
});
/** 選んだプロバイダの名前。 */
const aiName = $derived(
  aiSettings.value ? providerLabel(aiSettings.value) : t("proposals.aiName.short"),
);
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
  if (!view.useAi) return t("proposals.regenerate.blocked.useAi", { name: aiName });
  if (chars === 0) return t("proposals.regenerate.blocked.empty");
  if (tooLong) return t("proposals.regenerate.blocked.tooLong", { max: MAX_INSTRUCTION_CHARS });
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

/** 分類の要約の 1 行目 (AI か名前か・注文を添えたか)。 */
function classifiedText(layers: number, byAi: boolean, instructed: boolean): string {
  if (byAi) {
    return instructed
      ? t("proposals.summary.byAiInstructed", { count: layers })
      : t("proposals.summary.byAi", { count: layers });
  }
  return instructed
    ? t("proposals.summary.byNameInstructed", { count: layers })
    : t("proposals.summary.byName", { count: layers });
}

/** 分類の左右 (`left`・`right`・`center`。キャラクター自身の左右) の表示。 */
function sideText(side: string): string {
  switch (side) {
    case "left":
      return t("proposals.summary.side.left");
    case "right":
      return t("proposals.summary.side.right");
    default:
      return t("proposals.summary.side.center");
  }
}

/** まとまりのテクスチャの数 (法線マップを持つもの・向きだけのものも)。 */
function groupCounts(textures: number, mapped: number, oriented: number): string {
  const parts = [t("proposals.normal.counts.textures", { count: textures })];
  if (mapped > 0) parts.push(t("proposals.normal.counts.mapped", { count: mapped }));
  if (oriented > 0) parts.push(t("proposals.normal.counts.oriented", { count: oriented }));
  return formatList(parts, { type: "unit", style: "short" });
}
</script>

<section>
  <h3>{module ? t("proposals.autoRig.titleModule") : t("proposals.autoRig.title")}</h3>
  {#if view.staleRigs.length > 0}
    <Notice
      tone="warning"
      title={t("proposals.stale.title")}
      dismissLabel={t("proposals.stale.dismiss")}
      ondismiss={busy ? undefined : () => view.dismissStaleRigs()}
    >
      {t("proposals.stale.body")}
      <ul class="stale-list">
        {#each view.staleRigs as stale (stale.mesh)}<li>{stale.name}</li>{/each}
      </ul>
    </Notice>
  {/if}
  {#if module}
    <p class="ed-hint note">{t("proposals.module.note")}</p>
    <fieldset class="options" disabled={busy}>
      <legend class="ed-hint">{t("proposals.module.options.legend")}</legend>
      <Checkbox
        bind:checked={view.moduleOptions.direct}
        label={t("proposals.module.options.direct.label")}
        aria-describedby={`${uid}-direct`}
        disabled={busy}
      />
      <p id={`${uid}-direct`} class="ed-hint option-hint">{t("proposals.module.options.direct.description")}</p>
      <Checkbox
        bind:checked={view.moduleOptions.colliders}
        label={t("proposals.module.options.colliders.label")}
        aria-describedby={`${uid}-colliders`}
        disabled={busy}
      />
      <p id={`${uid}-colliders`} class="ed-hint option-hint">
        {t("proposals.module.options.colliders.description")}
      </p>
      <Checkbox
        bind:checked={view.moduleOptions.poses}
        label={t("proposals.module.options.poses.label")}
        aria-describedby={`${uid}-poses`}
        disabled={busy}
      />
      <p id={`${uid}-poses`} class="ed-hint option-hint">{t("proposals.module.options.poses.description")}</p>
    </fieldset>
  {/if}
  <div class="ed-buttons row">
    {#if panel?.psd}
      <span class="psd" title={panel.psd}>{fileName(panel.psd)}</span>
    {:else}
      <span class="ed-hint">{t("proposals.psd.empty")}</span>
    {/if}
    <Button onclick={() => view.choosePsd()} disabled={busy}>{t("proposals.psd.choose")}</Button>
  </div>
  <Checkbox
    bind:checked={view.useAi}
    disabled={!!aiBlocked || busy}
    label={aiLocal
      ? t("proposals.useAi.labelLocal", { name: aiName })
      : t("proposals.useAi.label", { name: aiName })}
    aria-describedby={aiBlocked && aiSettings.value ? `${uid}-blocked` : undefined}
  />
  {#if aiBlocked && aiSettings.value}
    <p class="ed-hint" id={`${uid}-blocked`}>
      {t("proposals.useAi.blocked", { reason: aiBlocked })}
      <Button variant="ghost" small onclick={onopensettings}>{t("proposals.useAi.settings")}</Button>
    </p>
  {/if}
  <label class="instructions" for={`${uid}-instructions`}>
    {t("proposals.instructions.label", { name: aiName })}
    <span class="muted">{t("proposals.instructions.optional")}</span>
  </label>
  <textarea
    id={`${uid}-instructions`}
    rows="3"
    bind:value={view.instructions}
    disabled={busy}
    placeholder={module
      ? t("proposals.instructions.placeholderModule")
      : t("proposals.instructions.placeholder")}
    aria-describedby={`${uid}-instructions-note`}
  ></textarea>
  <p id={`${uid}-instructions-note`} class="ed-hint">
    <span class:error={tooLong}>{chars} / {MAX_INSTRUCTION_CHARS}</span>{" · "}
    {view.useAi
      ? t("proposals.instructions.sent")
      : t("proposals.instructions.sentOnlyWithAi", { name: aiName })}
    {module ? t("proposals.instructions.scopeModule") : t("proposals.instructions.scope")}
  </p>
  {#if view.useAi && (panel?.regenerable.length ?? 0) > 0}
    <Checkbox bind:checked={view.reclassify} disabled={busy} label={t("proposals.reclassify")} />
  {/if}
  <div class="ed-buttons row">
    <Button onclick={() => view.classify()} disabled={busy || !panel?.psd || (view.useAi && tooLong)}>
      {t("proposals.classify")}
    </Button>
    <Button
      variant="primary"
      onclick={() => view.propose()}
      disabled={busy || !panel?.psd || (view.useAi && tooLong)}
    >
      {t("proposals.propose")}
    </Button>
  </div>
  {#if view.busy}
    <div class="progress-row">
      <p class="progress" role="status">
        {view.cancelling ? t("proposals.cancel.cancelling") : view.busy}
      </p>
      {#if view.sending}
        <TipButton
          onclick={() => view.cancel()}
          disabled={view.cancelling}
          tooltip={t("proposals.cancel.tooltip")}
        >{t("proposals.cancel.label")}</TipButton>
      {/if}
    </div>
  {/if}
  {#if view.notice}<p class="muted" role="status">{view.notice}</p>{/if}
  {#if view.error}<p class="error ugoe-selectable">{view.error}</p>{/if}
  {#if summary && panel?.classification}
    <p
      class="muted"
      title={panel.classification.instructions
        ? t("proposals.summary.instructionsTooltip", { instructions: panel.classification.instructions })
        : undefined}
    >
      {classifiedText(summary.layers, panel.classification.by_ai, !!panel.classification.instructions)}
      {#if summary.review > 0}{" · "}<span class="warn">{t("proposals.summary.review", { count: summary.review })}</span>{/if}
    </p>
    <details>
      <summary>{t("proposals.summary.labels")}</summary>
      <ul class="labels">
        {#each summary.labels as [label, count] (label)}
          <li><code>{label}</code> × {count}</li>
        {/each}
      </ul>
      {#if summary.variants.length > 0}
        <p class="ed-hint">{t("proposals.summary.variants")}</p>
        <ul class="labels">
          {#each summary.variants as names, i (i)}<li>{names.join(" / ")}</li>{/each}
        </ul>
      {/if}
      {#if review.length > 0}
        <p class="ed-hint">{t("proposals.summary.toReview")}</p>
        <p class="ed-hint">{t("proposals.correct.hint")}</p>
        <ul class="review">
          {#each review as layer (layer.path.join("\u0000"))}
            {@const name = layer.path.join(" / ")}
            <li>
              <span class="ugoe-selectable">{name}</span>
              {#if layer.corrected}
                {" · "}<span class="muted">{t("proposals.correct.corrected")}</span>
              {:else}
                ({Math.round(layer.confidence * 100)}%)
              {/if}
              {#if layer.name_disagreement}
                {@const byName = layer.name_disagreement}
                {" · "}<span class="warn">{t("proposals.summary.nameSays", {
                  label: byName.label ?? layer.label,
                  side: sideText(byName.side ?? layer.side),
                })}</span>
              {/if}
              <div class="correct">
                <Select
                  aria-label={t("proposals.correct.label.aria", { layer: name })}
                  value={layer.label}
                  options={labelOptions}
                  disabled={busy}
                  onchange={(label) => view.correctLayer(layer.path, label, layer.side as LayerSide)}
                />
                <SegmentedControl
                  aria-label={t("proposals.correct.side.aria", { layer: name })}
                  value={layer.side as LayerSide}
                  options={sideOptions}
                  disabled={busy}
                  onchange={(side) => view.correctLayer(layer.path, layer.label, side)}
                />
                {#if layer.corrected}
                  <TipButton
                    small
                    disabled={busy}
                    onclick={() => view.resetLayer(layer.path)}
                    tooltip={t("proposals.correct.reset.tooltip")}
                  >{t("proposals.correct.reset.label")}</TipButton>
                {:else}
                  <TipButton
                    small
                    disabled={busy}
                    onclick={() => view.correctLayer(layer.path, layer.label, layer.side as LayerSide)}
                    tooltip={t("proposals.correct.accept.tooltip")}
                  >{t("proposals.correct.accept.label")}</TipButton>
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </details>
  {/if}
</section>

{#if !module}
  <section aria-label={t("proposals.normal.title")}>
    <h3>{t("proposals.normal.title")}</h3>
    <p class="ed-hint">
      {normalFromClassification
        ? t("proposals.normal.descriptionClassification")
        : t("proposals.normal.descriptionNames")}
    </p>
    {#if view.normalGroups && view.normalGroups.length === 0}
      <p class="muted">{t("proposals.normal.empty")}</p>
    {:else if view.normalGroups}
      <ul class="normal-groups">
        {#each view.normalGroups as group (normalGroupKey(group))}
          {@const key = normalGroupKey(group)}
          {@const pick = view.normalPicks[key]}
          {@const unit = bulgeUnit(group)}
          {@const limits = bulgeLimits(group)}
          {@const name = surfaceGroupLabel(key)}
          {#if pick}
            <li>
              <span class="group-name">
                <Checkbox bind:checked={pick.include} disabled={busy} label={name} />
              </span>
              <span class="muted counts" title={group.textures.join(", ")}>
                {groupCounts(group.textures.length, group.with_normal_map.length, group.oriented.length)}
              </span>
              <span class="strength inline-slider">
                <Slider
                  label={t("proposals.normal.strength.aria", { group: name })}
                  bind:value={pick.strength}
                  min={0}
                  max={2}
                  step={0.05}
                  decimals={2}
                  disabled={busy || !pick.include}
                />
              </span>
              <details class="shape">
                <summary class="muted">
                  {pick.roundEdges
                    ? t("proposals.normal.shape.edges", {
                        width: Math.round(pick.edge),
                        smoothing: pick.smoothing.toFixed(1),
                        unit,
                      })
                    : t("proposals.normal.shape.whole", { smoothing: pick.smoothing.toFixed(1), unit })}
                </summary>
                <Checkbox
                  bind:checked={pick.roundEdges}
                  disabled={busy || !pick.include}
                  label={t("proposals.normal.roundEdges")}
                />
                {#if pick.roundEdges}
                  <Slider
                    label={t("proposals.normal.edgeWidth", { unit })}
                    bind:value={pick.edge}
                    min={1}
                    max={limits.edge}
                    step={1}
                    disabled={busy || !pick.include}
                  />
                {/if}
                <Slider
                  label={t("proposals.normal.smoothing", { unit })}
                  bind:value={pick.smoothing}
                  min={0}
                  max={limits.smoothing}
                  step={0.5}
                  disabled={busy || !pick.include}
                />
                <TipButton
                  small
                  disabled={busy}
                  onclick={() => Object.assign(pick, bulgeForm(group))}
                  tooltip={t("proposals.normal.defaults.tooltip")}
                >{t("proposals.normal.defaults.label")}</TipButton>
              </details>
            </li>
          {/if}
        {/each}
      </ul>
      <Checkbox
        bind:checked={view.replaceNormalMaps}
        disabled={busy}
        label={t("proposals.normal.replace")}
      />
      <div class="ed-buttons row">
        <TipButton
          variant="primary"
          onclick={() => view.proposeNormalMaps()}
          disabled={busy || normalTargets === 0}
          tooltip={t("proposals.normal.propose.tooltip")}
        >{normalTargets > 0
            ? t("proposals.normal.propose.labelCount", { count: normalTargets })
            : t("proposals.normal.propose.label")}</TipButton>
      </div>
    {/if}
  </section>
{/if}

<section>
  <h3>{t("proposals.list.title")}</h3>
  {#if !panel || panel.proposals.length === 0}
    <p class="ed-hint">{t("proposals.list.empty")}</p>
  {:else}
    {#each panel.proposals as proposal (proposal.id)}
      {@const excluded = view.excludedOf(proposal.id)}
      {@const selected = proposal.commands.length - excluded.size}
      {@const previewing = view.preview?.id === proposal.id}
      {@const regenerable = panel.regenerable.includes(proposal.id)}
      <article class="proposal" class:previewing aria-label={t("proposals.list.aria", { id: proposal.id })}>
        <header>
          <strong><span class="index">#{proposal.id}</span> {proposal.title}</strong>
          {#if proposal.revises !== null}
            {@render badge(
              "source",
              t("proposals.list.revision.label", { id: proposal.revises }),
              t("proposals.list.revision.tooltip", { id: proposal.revises }),
            )}
          {/if}
          {#if proposal.source}
            {@render badge(
              "source",
              t("proposals.list.from.label", { name: proposal.source }),
              t("proposals.list.from.tooltip", { name: proposal.source }),
            )}
          {/if}
          {#if proposal.stale}
            {@render badge("warn", t("proposals.list.outdated.label"), t("proposals.list.outdated.tooltip"))}
          {/if}
        </header>
        {#if proposal.instructions}
          <p class="note sent"><span class="muted">{t("proposals.list.instructions")}</span> {proposal.instructions}</p>
        {/if}
        {#if proposal.unapplied.length > 0}
          <div class="unapplied" role="note">
            <strong>{t("proposals.list.unapplied.title")}</strong> {t("proposals.list.unapplied.body")}
            <ul>
              {#each proposal.unapplied as item, i (i)}
                <li>{item}</li>
              {/each}
            </ul>
          </div>
        {/if}
        <p class="rationale">{proposal.rationale}</p>
        <details>
          <summary>
            {t("proposals.list.selected", { selected, count: proposal.commands.length })}
          </summary>
          <ul class="groups">
            {#each view.commandGroups(proposal.id) as group (group.indices[0])}
              {@const state = groupState(group, excluded)}
              {@const single =
                group.indices.length === 1 && !group.section ? proposal.commands[group.indices[0] as number] : null}
              <li>
                <div class="group">
                  <!-- 一覧の行の入・切は、番号とコマンドの表記 (code) を並べるため標準のチェックボックスのまま。 -->
                  <input
                    type="checkbox"
                    aria-label={t("proposals.list.include", { name: single ? commandLabel(single) : group.label })}
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
                      <summary><code>{group.label}</code> · {t("proposals.list.commands", { count: group.indices.length })}</summary>
                      <ol class="commands">
                        {#each group.indices as i (i)}
                          {@const command = proposal.commands[i]}
                          {#if command}
                            <li>
                              <label class="check" title={command.requires.length > 0
                                ? t("proposals.list.dependsOn", { list: command.requires.join(", #") })
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
        <div class="ed-buttons row">
          <TipButton
            pressed={previewing}
            onclick={() => view.togglePreview(proposal.id)}
            disabled={busy || proposal.stale || selected === 0}
            tooltip={t("proposals.list.preview.tooltip")}
          >{previewing ? t("proposals.list.preview.end") : t("proposals.list.preview.label")}</TipButton>
          <TipButton
            variant="primary"
            onclick={() => view.adopt(proposal.id)}
            disabled={busy || proposal.stale || selected === 0}
            tooltip={t("proposals.list.adopt.tooltip")}
          >{selected < proposal.commands.length
              ? t("proposals.list.adopt.labelCount", { count: selected })
              : t("proposals.list.adopt.label")}</TipButton>
          <Button onclick={() => view.reject(proposal.id)} disabled={busy}>{t("proposals.list.reject")}</Button>
          {#if regenerable}
            <TipButton
              onclick={() => view.regenerate(proposal.id)}
              disabled={busy || !!regenerateBlocked}
              aria-describedby={regenerateBlocked ? `${uid}-regen-${proposal.id}` : undefined}
              tooltip={t("proposals.regenerate.tooltip", { name: aiName })}
            >{t("proposals.regenerate.label")}</TipButton>
          {/if}
        </div>
        {#if regenerable && regenerateBlocked && !busy}
          <p class="ed-hint" id={`${uid}-regen-${proposal.id}`}>{regenerateBlocked}</p>
        {/if}
        {#if previewing && view.preview}
          <p class="ed-hint">{t("proposals.list.previewing")}</p>
          <div class="ed-buttons row">
            <TipButton
              pressed={view.physicsRunning}
              onclick={() => view.togglePhysics()}
              tooltip={t("proposals.list.physics.tooltip")}
            >{view.physicsRunning ? t("proposals.list.physics.stop") : t("proposals.list.physics.run")}</TipButton>
            {#if view.physicsRunning}<span class="ed-hint">{t("proposals.list.physics.hint")}</span>{/if}
          </div>
          <div class="sliders">
            {#each view.preview.parameters as p (p.id)}
              <Slider
                label={p.name}
                value={viewport.parameterValues[p.id] ?? p.default}
                min={p.min}
                max={p.max}
                step={(p.max - p.min) / 100}
                field={false}
                onchange={(v) => viewport.setParameter(p.id, v)}
              />
            {/each}
          </div>
        {/if}
      </article>
    {/each}
  {/if}
</section>

{#snippet badge(kind: "source" | "warn", text: string, tooltip: string)}
  <Tooltip text={tooltip}>
    {#snippet trigger(attrs)}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <span {...attrs} class={kind} tabindex="0">{text}</span>
    {/snippet}
  </Tooltip>
{/snippet}

<style>
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ugoe-space-2);
    margin: var(--ugoe-space-2) 0;
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
    gap: var(--ugoe-space-2);
  }
  .note {
    font-size: var(--ugoe-font-size-sm);
  }
  .option-hint {
    margin: 0 0 var(--ugoe-space-2) calc(var(--ugoe-space-5) + var(--ugoe-space-1));
  }
  .stale-list {
    list-style: disc;
    margin: var(--ugoe-space-1) 0 0;
    padding-left: 1.5em;
  }
  /* 行の中のスライダーは名前を見せない (読み上げの名前には使う)。 */
  .inline-slider :global(.ugoe-slider__label) {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .inline-slider :global(.ugoe-slider__root) {
    min-width: 60px;
  }
  .options {
    margin: var(--ugoe-space-2) 0;
    padding: var(--ugoe-space-1) var(--ugoe-space-3) var(--ugoe-space-2);
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-md);
  }
  .options legend {
    padding: 0 var(--ugoe-space-2);
  }
  .instructions {
    display: block;
    margin-top: var(--ugoe-space-2);
  }
  textarea {
    box-sizing: border-box;
    width: 100%;
    padding: var(--ugoe-space-1) var(--ugoe-space-2);
    resize: vertical;
    border: 1px solid var(--ugoe-color-control-border);
    border-radius: var(--ugoe-radius-md);
    background: var(--ugoe-color-surface);
    color: var(--ugoe-color-text);
    font: inherit;
    font-size: var(--ugoe-font-size-sm);
  }
  textarea:focus-visible {
    outline: var(--ugoe-focus-ring);
    outline-offset: 1px;
  }
  .sent {
    margin: var(--ugoe-space-2) 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .unapplied {
    margin: var(--ugoe-space-2) 0;
    padding: var(--ugoe-space-2);
    border-left: 3px solid var(--ugoe-color-warning);
    font-size: var(--ugoe-font-size-sm);
  }
  .unapplied ul {
    list-style: disc;
    margin: var(--ugoe-space-1) 0;
    padding-left: 1.5em;
  }
  .progress {
    color: var(--ugoe-color-accent);
  }
  .progress-row {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-3);
  }
  .progress-row .progress {
    flex: 1;
    min-width: 0;
  }
  .warn {
    color: var(--ugoe-color-warning);
  }
  .labels {
    margin: var(--ugoe-space-2) 0;
    padding-left: 1.5em;
    font-size: var(--ugoe-font-size-sm);
  }
  .review {
    list-style: none;
    margin: var(--ugoe-space-2) 0;
    padding: 0;
    font-size: var(--ugoe-font-size-sm);
  }
  .review li {
    padding: var(--ugoe-space-2) 0;
    border-top: 1px solid var(--ugoe-color-border);
    overflow-wrap: anywhere;
  }
  .review .correct {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ugoe-space-2);
    margin-top: var(--ugoe-space-1);
  }
  .proposal {
    border: 1px solid var(--ugoe-color-border);
    border-radius: var(--ugoe-radius-md);
    padding: var(--ugoe-space-2) var(--ugoe-space-3);
    margin: var(--ugoe-space-2) 0;
  }
  .proposal.previewing {
    border-color: var(--ugoe-color-accent);
  }
  .proposal header {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: var(--ugoe-space-2);
  }
  .source {
    padding: 0 var(--ugoe-space-2);
    border: 1px solid var(--ugoe-color-accent);
    border-radius: var(--ugoe-radius-full);
    color: var(--ugoe-color-accent);
    font-size: var(--ugoe-font-size-xs);
    white-space: nowrap;
  }
  .rationale {
    max-height: 10em;
    overflow-y: auto;
    white-space: pre-wrap;
    font-size: var(--ugoe-font-size-sm);
    margin: var(--ugoe-space-2) 0;
  }
  .commands {
    list-style: none;
    margin: var(--ugoe-space-1) 0;
    padding: 0;
  }
  .groups {
    list-style: none;
    margin: var(--ugoe-space-2) 0;
    padding: 0;
    max-height: 320px;
    overflow-y: auto;
    font-size: var(--ugoe-font-size-sm);
  }
  .group {
    display: flex;
    align-items: flex-start;
    gap: var(--ugoe-space-2);
  }
  .inner summary {
    cursor: pointer;
  }
  .index {
    color: var(--ugoe-color-text-muted);
    min-width: 2.5em;
  }
  .normal-groups {
    list-style: none;
    margin: var(--ugoe-space-2) 0;
    padding: 0;
    font-size: var(--ugoe-font-size-sm);
  }
  .normal-groups li {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    column-gap: var(--ugoe-space-2);
    padding: var(--ugoe-space-1) 0;
  }
  .normal-groups .group-name {
    min-width: 0;
  }
  .normal-groups .counts {
    grid-column: 1;
    padding-left: 22px;
    font-size: var(--ugoe-font-size-xs);
  }
  .normal-groups .strength {
    grid-column: 2;
    grid-row: 1 / span 2;
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-1);
    width: 150px;
  }
  .normal-groups .strength > :global(*) {
    flex: 1;
    min-width: 0;
  }
  .normal-groups .shape {
    display: grid;
    gap: var(--ugoe-space-1);
    grid-column: 1 / -1;
    padding-left: 22px;
    font-size: var(--ugoe-font-size-xs);
  }
  .normal-groups .shape summary {
    cursor: pointer;
  }
  .sliders {
    display: grid;
    gap: var(--ugoe-space-1);
  }
</style>
