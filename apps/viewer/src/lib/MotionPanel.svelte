<script lang="ts">
// モーションと表情のパネル (T0720)。モーションか表情を持つモデル (バンドル) のときに App.svelte が出す。
// モーションは優先度を選んで再生・停止し、表情は「これだけ」(set)・「重ねる」(add)・「外す」(stop) と「すべて止める」。
// 再生は Player (WebPuppet の start_motion・set_expression など) が行う。規則は docs/spec/motion.md。
// フェイストラッキング中は、トラッキングが書き込むパラメータでモーションの値が置き換わる (印を出す。README の「モーションと表情」)。
import {
  describeMotion,
  type ExpressionInfo,
  formatSeconds,
  MOTION_PRIORITIES,
  type MotionPriority,
  motionTrackingEffect,
  trackedOverlap,
} from "./animation";
import type { Player } from "./player.svelte";

let { player }: { player: Player } = $props();

let open = $state(true);

let tracked = $derived(new Set(player.tracking ? player.trackingParameterIds : []));
let status = $derived(
  player.currentMotion !== null
    ? `再生中: ${player.currentMotion}`
    : `${player.motions.length} モーション・${player.expressions.length} 表情`,
);

function expressionTitle(e: ExpressionInfo): string {
  const lines = [`フェード: ${formatSeconds(e.fadeIn)} / ${formatSeconds(e.fadeOut)}`];
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

<details class="panel motions" bind:open>
  <summary>
    <h2>モーションと表情</h2>
    <span class="status">{status}</span>
  </summary>

  {#if player.tracking}
    <p class="hint tracking-note">
      トラッキング中は、トラッキングが書き込むパラメータ (頭の向き・目・口など) でモーションの値が置き換わります。
      表情はトラッキングの上に重なります。
    </p>
  {/if}

  <section aria-label="モーション">
    <h3>
      モーション
      <span class="spacer"></span>
      <button
        type="button"
        onclick={() => player.stopMotions()}
        disabled={player.currentMotion === null}
        title="再生中のモーションをフェードアウトして止める (最後の姿勢は残る)">止める</button
      >
    </h3>
    {#if player.motions.length === 0}
      <p class="empty">このモデルにはモーションがありません</p>
    {:else}
      <div class="options">
        <label title={MOTION_PRIORITIES.find((p) => p.value === player.motionPriority)?.title}>
          優先度
          <select
            value={player.motionPriority}
            onchange={(e) => player.setMotionPriority(e.currentTarget.value as MotionPriority)}
          >
            {#each MOTION_PRIORITIES as p (p.value)}
              <option value={p.value} title={p.title}>{p.label}</option>
            {/each}
          </select>
        </label>
        <label title="ループしないモーションが終わったら、同じ優先度で初めから再生し直す">
          <input
            type="checkbox"
            checked={player.repeatMotion}
            onchange={(e) => player.setRepeatMotion(e.currentTarget.checked)}
          />
          繰り返す
        </label>
      </div>
      <ul>
        {#each player.motions as m (m.name)}
          {@const current = player.currentMotion === m.name}
          {@const effect = motionTrackingEffect(m, tracked)}
          <li class:current>
            <button
              type="button"
              class="play"
              aria-pressed={current}
              title={current ? `「${m.name}」を止める` : `「${m.name}」を再生する`}
              onclick={() => (current ? player.stopMotions() : player.playMotion(m.name))}
              >{current ? "■" : "▶"}</button
            >
            <div class="info">
              <div class="line">
                <span class="name" title={m.parameters.join(", ")}>{m.name}</span>
                {#if effect.kind !== "none"}
                  <span
                    class="badge"
                    title="トラッキングが上書きするパラメータ: {effect.overlap.join(', ')}"
                    >{effect.kind === "all" ? "トラッキングで隠れる" : "一部トラッキング"}</span
                  >
                {/if}
              </div>
              <span class="meta">{describeMotion(m)}</span>
            </div>
          </li>
        {/each}
      </ul>
      {#if player.motionEvents.length > 0}
        <p class="events" title="直近に発火したモーションのイベント (新しい順)">
          イベント: {player.motionEvents.join(", ")}
        </p>
      {/if}
    {/if}
  </section>

  {#if player.motionNotice}
    <p class="notice" role="status">{player.motionNotice}</p>
  {/if}

  <section aria-label="表情">
    <h3>
      表情
      <span class="spacer"></span>
      <button
        type="button"
        onclick={() => player.stopExpressions()}
        disabled={player.activeExpressions.length === 0}
        title="使っている表情をすべてフェードアウトする">すべて止める</button
      >
    </h3>
    {#if player.expressions.length === 0}
      <p class="empty">このモデルには表情がありません</p>
    {:else}
      <ul>
        {#each player.expressions as e (e.name)}
          {@const active = player.activeExpressions.includes(e.name)}
          {@const overlap = expressionOverlap(e)}
          <li class:current={active}>
            <span class="name" title={expressionTitle(e)}>{e.name}</span>
            {#if overlap.length > 0}
              <span class="badge" title="トラッキングの値に重なるパラメータ: {overlap.join(', ')}"
                >トラッキングに重なる</span
              >
            {/if}
            <button
              type="button"
              title="ほかの表情を止めて、この表情だけにする"
              onclick={() => player.setExpression(e.name)}>これだけ</button
            >
            {#if active}
              <button type="button" title="この表情をフェードアウトする" onclick={() => player.stopExpression(e.name)}
                >外す</button
              >
            {:else}
              <button
                type="button"
                title="使っている表情の上に重ねる"
                onclick={() => player.addExpression(e.name)}>重ねる</button
              >
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</details>

<style>
  .motions {
    flex-shrink: 0;
    max-height: 40%;
    overflow-y: auto;
    font-size: 0.8rem;
  }
  .motions > summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
  }
  h2 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    display: inline;
  }
  h3 {
    margin: 0;
    font-size: 0.8rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  h3 button {
    font-size: 0.75rem;
    font-weight: normal;
    padding: 0.1rem 0.5rem;
  }
  .spacer {
    flex: 1;
  }
  .status,
  .hint,
  .empty,
  .meta,
  .events {
    color: var(--muted);
    font-weight: normal;
  }
  .status {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 60%;
  }
  .hint,
  .empty,
  .events,
  .notice {
    margin: 0.3rem 0 0;
    font-size: 0.75rem;
  }
  .notice {
    color: var(--warning);
  }
  section {
    margin-top: 0.6rem;
  }
  .options {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    margin-top: 0.3rem;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .options label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }
  .options select {
    font-size: 0.75rem;
  }
  ul {
    list-style: none;
    margin: 0.2rem 0 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.1rem 0;
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
    gap: 0.3rem;
  }
  li .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  li.current .name {
    font-weight: 600;
    color: var(--accent);
  }
  li button {
    font-size: 0.75rem;
    padding: 0.1rem 0.45rem;
  }
  .play {
    min-width: 1.8rem;
  }
  .meta {
    font-size: 0.7rem;
    white-space: nowrap;
  }
  .badge {
    font-size: 0.65rem;
    color: var(--warning);
    border: 1px solid currentColor;
    border-radius: 3px;
    padding: 0 0.25rem;
    white-space: nowrap;
  }
</style>
