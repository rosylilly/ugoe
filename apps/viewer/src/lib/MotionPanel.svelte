<script lang="ts">
// モーションと表情のパネル (T0720)。モーションか表情を持つモデル (バンドル) のときに App.svelte が出す。
// モーションは優先度を選んで再生・停止し、表情は「これだけ」(set)・「重ねる」(add)・「外す」(stop) と「すべて止める」。
// 再生は Player (WebPuppet の start_motion・set_expression など) が行う。規則は docs/spec/motion.md。
// フェイストラッキング中は、トラッキングが書き込むパラメータでモーションの値が置き換わる (印を出す。README の「モーションと表情」)。
// 「モーションを優先」を入れると、待機以外のモーションが動かすパラメータではモーションが勝つ (T0721)。
// 優先する部分 (頭の向き・目など) とパラメータは選べ、外したものはトラッキングのまま (T0722)。
// 待機モーションは自動で流し、ほかのモーションが終わると待機に戻る (T0721)。選んだ待機モーションはモデルごとに覚える (T0722)。
import { formatList } from "@ugoe/i18n";
import {
  describeMotion,
  type ExpressionInfo,
  formatSeconds,
  groupTrackingParameters,
  MOTION_PRIORITIES,
  type MotionPriority,
  motionTrackingEffect,
  type Player,
  trackedOverlap,
  trackingWins,
} from "@ugoe/player";
import {
  Button,
  Checkbox,
  Icon,
  IconButton,
  Section,
  SegmentedControl,
  Select,
  Toggle,
} from "@ugoe/ui";
import { t } from "./i18n";

let { player }: { player: Player } = $props();

let open = $state(true);
let parametersOpen = $state(false);

/** 待機モーションの選択の「(なし)」の値 (モーションの名前と重ならない)。 */
const NO_IDLE = "\u0000";

let tracked = $derived(new Set(player.tracking ? player.trackingParameterIds : []));

/**
 * モーション `name` の値をトラッキングが置き換えるパラメータ。「モーションを優先」なら、自動で流す待機モーションは
 * すべて、ほかのモーションはトラッキングのままにしたものだけ。
 */
function trackedFor(name: string): ReadonlySet<string> {
  if (player.motionOverTracking && player.autoIdle && name === player.idleMotion) return tracked;
  return trackingWins(tracked, player.motionOverTracking, player.trackingKept);
}

/** 優先を選べるパラメータ (トラッキングが書き込むもの) のまとまり。 */
let groups = $derived(groupTrackingParameters(player.trackingParameterIds));
let kept = $derived(new Set(player.trackingKept));
let preferredTotal = $derived(preferredCount(player.trackingParameterIds));

let priorityOptions = $derived(MOTION_PRIORITIES.map((p) => ({ value: p.value, label: p.label })));
let priorityTitle = $derived(
  MOTION_PRIORITIES.find((p) => p.value === player.motionPriority)?.title,
);
let idleOptions = $derived([
  { value: NO_IDLE, label: t("motions.idle.none") },
  ...player.motions.map((m) => ({ value: m.name, label: m.name })),
]);

/** まとまりのうちモーションを優先するものの数。 */
function preferredCount(ids: readonly string[]): number {
  return ids.filter((id) => !kept.has(id)).length;
}

/** まとまりをまとめて切り替える (すべて優先しているならトラッキングのままに、そうでなければすべて優先に)。 */
function toggleGroup(ids: readonly string[]) {
  const all = preferredCount(ids) === ids.length;
  const rest = player.trackingKept.filter((id) => !ids.includes(id));
  player.setTrackingKept(all ? [...rest, ...ids] : rest);
}

function toggleParameter(id: string, preferred: boolean) {
  const rest = player.trackingKept.filter((k) => k !== id);
  player.setTrackingKept(preferred ? rest : [...rest, id]);
}
let status = $derived(
  player.currentMotion !== null
    ? t("motions.status.playing", { name: player.currentMotion })
    : t("motions.status.counts", {
        motions: player.motions.length,
        expressions: player.expressions.length,
      }),
);

function expressionTitle(e: ExpressionInfo): string {
  const lines = [
    t("motions.expression.fade", {
      fadeIn: formatSeconds(e.fadeIn),
      fadeOut: formatSeconds(e.fadeOut),
    }),
  ];
  for (const p of e.parameters) lines.push(`${p.id} (${p.op})`);
  return lines.join("\n");
}

function expressionOverlap(e: ExpressionInfo): string[] {
  return trackedOverlap(
    e.parameters.map((p) => p.id),
    tracked,
  );
}
</script>

<div class="panel motions">
  <Section title={t("motions.title")} level={2} bind:open>
    {#snippet actions()}
      <span class="status">{status}</span>
    {/snippet}

    {#if player.tracking}
      <p class="hint tracking-note">
        {player.motionOverTracking
          ? t("motions.trackingNote.motionWins")
          : t("motions.trackingNote.trackingWins")}
      </p>
    {/if}

    <section aria-labelledby="motions-heading">
      <!-- ボタンは見出しの外に置く (見出しの名前にボタンの文が混ざらないように) -->
      <div class="heading">
        <h3 id="motions-heading">{t("motions.motions.title")}</h3>
        <span class="spacer"></span>
        <Button small onclick={() => player.stopMotions()} disabled={player.currentMotion === null}
          >{t("motions.stop.label")}</Button
        >
      </div>
      {#if player.motions.length === 0}
        <p class="hint">{t("motions.motions.empty")}</p>
      {:else}
        <div class="options">
          <SegmentedControl
            label={t("motions.priority.label")}
            value={player.motionPriority}
            options={priorityOptions}
            onchange={(value: MotionPriority) => player.setMotionPriority(value)}
          />
          {#if priorityTitle}<p class="hint">{priorityTitle}</p>{/if}
          <Toggle
            label={t("motions.repeat.label")}
            checked={player.repeatMotion}
            onchange={(on) => player.setRepeatMotion(on)}
          />
          <Toggle
            label={t("motions.autoIdle.label")}
            checked={player.autoIdle}
            onchange={(on) => player.setAutoIdle(on)}
          />
          <div class="idle">
            <Select
              label={t("motions.idle.label")}
              value={player.idleMotion ?? NO_IDLE}
              options={idleOptions}
              disabled={!player.autoIdle}
              onchange={(value) => player.setIdleMotion(value === NO_IDLE ? null : value)}
            />
            {#if player.idleRemembered}
              <Button small onclick={() => player.forgetIdleMotion()}
                >{t("motions.idle.forget.label")}</Button
              >
            {/if}
          </div>
          <Toggle
            label={t("motions.overTracking.label")}
            checked={player.motionOverTracking}
            onchange={(on) => player.setMotionOverTracking(on)}
          />
        </div>
        {#if player.motionOverTracking && groups.length > 0}
          <div class="over-groups" role="group" aria-labelledby="motions-over-groups">
            <span id="motions-over-groups">{t("motions.overTracking.groups.label")}</span>
            {#each groups as g (g.group)}
              {@const n = preferredCount(g.ids)}
              <Checkbox
                label={g.label}
                checked={n === g.ids.length}
                indeterminate={n > 0 && n < g.ids.length}
                onchange={() => toggleGroup(g.ids)}
              />
            {/each}
          </div>
          <p class="hint">{t("motions.overTracking.groups.description")}</p>
          <div class="over-parameters">
            <Section
              title={t("motions.overTracking.parameters.label", {
                preferred: preferredTotal,
                count: player.trackingParameterIds.length,
              })}
              level={4}
              bind:open={parametersOpen}
            >
              <div class="parameter-grid">
                {#each player.trackingParameterIds as id (id)}
                  <Checkbox
                    label={id}
                    checked={!kept.has(id)}
                    onchange={(checked) => toggleParameter(id, checked)}
                  />
                {/each}
              </div>
            </Section>
          </div>
        {/if}
        <ul>
          {#each player.motions as m (m.name)}
            {@const current = player.currentMotion === m.name}
            {@const effect = motionTrackingEffect(m, trackedFor(m.name))}
            <li class:current>
              <IconButton
                small
                pressed={current}
                label={current
                  ? t("motions.motion.stop.aria", { name: m.name })
                  : t("motions.motion.play.aria", { name: m.name })}
                onclick={() => (current ? player.stopMotions() : player.playMotion(m.name))}
              >
                {#if current}
                  <Icon name="stop" />
                {:else}
                  <Icon name="play" />
                {/if}
              </IconButton>
              <div class="info">
                <div class="line">
                  <span class="name" title={m.parameters.join(", ")}>{m.name}</span>
                  {#if player.autoIdle && m.name === player.idleMotion}
                    <span class="idle-tag">{t("motions.motion.idleTag")}</span>
                  {/if}
                  {#if effect.kind !== "none"}
                    <span
                      class="badge"
                      title={t("motions.motion.overlap.tooltip", { parameters: formatList(effect.overlap) })}
                      >{effect.kind === "all"
                        ? t("motions.motion.overlap.all")
                        : t("motions.motion.overlap.some")}</span
                    >
                  {/if}
                </div>
                <span class="meta">{describeMotion(m)}</span>
              </div>
            </li>
          {/each}
        </ul>
        {#if player.motionEvents.length > 0}
          <p class="hint events">
            {t("motions.events", {
              events: player.motionEvents
                .map((e) => t("motions.event", { value: e.value, motion: e.motion, number: e.number }))
                .join(", "),
            })}
          </p>
          <p class="hint events-description">{t("motions.eventsDescription")}</p>
        {/if}
      {/if}
    </section>

    {#if player.motionNotice}
      <p class="notice" role="status">{player.motionNotice}</p>
    {/if}

    <section aria-labelledby="expressions-heading">
      <div class="heading">
        <h3 id="expressions-heading">{t("motions.expressions.title")}</h3>
        <span class="spacer"></span>
        <Button
          small
          onclick={() => player.stopExpressions()}
          disabled={player.activeExpressions.length === 0}>{t("motions.expressions.stopAll.label")}</Button
        >
      </div>
      {#if player.expressions.length === 0}
        <p class="hint">{t("motions.expressions.empty")}</p>
      {:else}
        <ul>
          {#each player.expressions as e (e.name)}
            {@const active = player.activeExpressions.includes(e.name)}
            {@const overlap = expressionOverlap(e)}
            <li class:current={active}>
              <span class="name" title={expressionTitle(e)}>{e.name}</span>
              {#if overlap.length > 0}
                <span
                  class="badge"
                  title={t("motions.expression.overlap.tooltip", { parameters: formatList(overlap) })}
                  >{t("motions.expression.overlap.label")}</span
                >
              {/if}
              <Button small onclick={() => player.setExpression(e.name)}
                >{t("motions.expression.only.label")}</Button
              >
              {#if active}
                <Button small onclick={() => player.stopExpression(e.name)}
                  >{t("motions.expression.remove.label")}</Button
                >
              {:else}
                <Button small onclick={() => player.addExpression(e.name)}
                  >{t("motions.expression.add.label")}</Button
                >
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </Section>
</div>

<style>
  /* 高さは中身のまま (右の列の上の段 (App.svelte) がまとめて送る。T0725)。 */
  .motions {
    font-size: var(--ugoe-font-size-sm);
  }
  .motions :global(.ugoe-toggle),
  .motions :global(.ugoe-checkbox),
  .motions :global(.ugoe-select),
  .motions :global(.ugoe-segmented) {
    font-size: var(--ugoe-font-size-sm);
  }
  .heading {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
  }
  h3 {
    margin: 0;
    font-size: var(--ugoe-font-size-sm);
    font-weight: var(--ugoe-font-weight-strong);
  }
  .spacer {
    flex: 1;
  }
  .status,
  .hint,
  .meta {
    color: var(--ugoe-color-text-muted);
    font-weight: var(--ugoe-font-weight-normal);
  }
  .status {
    font-size: var(--ugoe-font-size-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint,
  .notice {
    margin: var(--ugoe-space-1) 0 0;
    font-size: var(--ugoe-font-size-xs);
  }
  .notice {
    color: var(--ugoe-color-warning);
  }
  section {
    margin-top: var(--ugoe-space-3);
  }
  .options {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--ugoe-space-2);
    margin-top: var(--ugoe-space-2);
  }
  .idle {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--ugoe-space-2);
  }
  .over-groups {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ugoe-space-2) var(--ugoe-space-3);
    margin-top: var(--ugoe-space-3);
  }
  .over-parameters {
    margin-top: var(--ugoe-space-1);
  }
  .parameter-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr));
    gap: var(--ugoe-space-1) var(--ugoe-space-2);
  }
  ul {
    list-style: none;
    margin: var(--ugoe-space-2) 0 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
    padding: var(--ugoe-space-1) 0;
  }
  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .line {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
  }
  li .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  li.current .name {
    font-weight: var(--ugoe-font-weight-strong);
    color: var(--ugoe-color-accent);
  }
  .meta {
    font-size: var(--ugoe-font-size-xs);
    white-space: nowrap;
  }
  .idle-tag,
  .badge {
    font-size: var(--ugoe-font-size-xs);
    border: 1px solid currentColor;
    border-radius: var(--ugoe-radius-sm);
    padding: 0 var(--ugoe-space-1);
    white-space: nowrap;
  }
  .idle-tag {
    color: var(--ugoe-color-text-muted);
  }
  .badge {
    color: var(--ugoe-color-warning);
  }
</style>
