<script lang="ts">
// モデルごとに覚えたもの (光源の設定 (T0709)・着せ替えの組 (T0711)・待機モーションの選択 (T0722)) の一覧 (T0714)。
// ツールバーの「覚えたもの」とライティングのパネルから開く。モデルごとに光源・組・待機モーションを忘れるか、すべて忘れる。
// パネルが出ないモデル (モジュールを外してライティングを受けなくなったモデルなど) や、開いていないモデルの分もここで忘れられる。
// 並びは覚えた日時の新しい順と名前の順から選ぶ (T0717。日時を持たない前の保存値は「日時なし」で最後)。
// 選んだ並びは覚えて、次に開いたときも使う (T0718)。
import { describeIdleChoice } from "./animation";
import { lightingLabel } from "./lighting";
import type { Player } from "./player.svelte";
import {
  describeDressUpSet,
  describeLocation,
  describeRememberedAt,
  loadRememberedOrder,
  type RememberedModel,
  type RememberedOrder,
  rememberedCount,
  saveRememberedOrder,
} from "./rememberedModels";

let { player }: { player: Player } = $props();

let dialog = $state<HTMLDialogElement | null>(null);
let rows = $state<RememberedModel[]>([]);
/** 並び (覚えた日時の新しい順か名前の順。T0717)。開いているモデルはどちらでも先頭。選択は覚える (T0718)。 */
let order = $state<RememberedOrder>(loadRememberedOrder());
/** 「すべて忘れる」を押して、確かめている間。 */
let confirming = $state(false);

export function open(): void {
  // ほかのウィンドウ (タブ) で選び直した並びも拾う。
  order = loadRememberedOrder();
  refresh();
  confirming = false;
  dialog?.showModal();
}

function refresh() {
  rows = player.rememberedModels(order);
}

function forget(
  row: RememberedModel,
  what: { lighting?: boolean; dressUp?: boolean; idle?: boolean },
) {
  player.forgetRemembered(row.model, what);
  refresh();
}

function forgetAll() {
  player.forgetAllRemembered();
  confirming = false;
  refresh();
}
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="memory-title"
  onclick={(e) => e.target === dialog && dialog?.close()}
>
  <header>
    <h2 id="memory-title">モデルごとに覚えたもの</h2>
    <button type="button" onclick={() => dialog?.close()}>閉じる</button>
  </header>
  <p class="lead">
    光源の設定・着せ替えの組・待機モーションの選択は、モデルごとにこのブラウザ (アプリ) に覚えています。忘れても、いま表示している光源・付けているモジュール・流している待機モーションは変わりません。
  </p>

  {#if rows.length > 1}
    <label class="order">
      並び
      <select
        value={order}
        onchange={(e) => {
          order = e.currentTarget.value as RememberedOrder;
          saveRememberedOrder(order);
          refresh();
        }}
      >
        <option value="recent">覚えた日時の新しい順</option>
        <option value="name">名前の順</option>
      </select>
    </label>
  {/if}

  {#if rows.length === 0}
    <p class="empty">覚えているモデルはありません。</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th scope="col">モデル</th>
          <th scope="col">光源</th>
          <th scope="col">着せ替えの組</th>
          <th scope="col">待機モーション</th>
          <th scope="col">覚えた日時</th>
          <th scope="col"><span class="visually-hidden">操作</span></th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row (`${row.model.location ?? ""}|${row.model.fingerprint}`)}
          <tr class:current={row.current} data-model={row.model.name}>
            <td>
              <div class="name">
                {row.model.name || "(名前なし)"}
                {#if row.current}<span class="badge">開いている</span>{/if}
              </div>
              <div class="location" title={row.model.location ?? ""}>{describeLocation(row.model)}</div>
            </td>
            <td>
              {#if row.lighting}
                <span>{lightingLabel(row.lighting)}</span>
                <button
                  type="button"
                  class="small"
                  aria-label="{row.model.name} の光源を忘れる"
                  onclick={() => forget(row, { lighting: true, dressUp: false, idle: false })}
                  >忘れる</button
                >
              {:else}
                <span class="none">—</span>
              {/if}
            </td>
            <td>
              {#if row.dressUp}
                <span>{describeDressUpSet(row.dressUp)}</span>
                <button
                  type="button"
                  class="small"
                  aria-label="{row.model.name} の着せ替えの組を忘れる"
                  onclick={() => forget(row, { lighting: false, dressUp: true, idle: false })}
                  >忘れる</button
                >
              {:else}
                <span class="none">—</span>
              {/if}
            </td>
            <td>
              {#if row.idle}
                <span>{describeIdleChoice(row.idle)}</span>
                <button
                  type="button"
                  class="small"
                  aria-label="{row.model.name} の待機モーションを忘れる"
                  onclick={() => forget(row, { lighting: false, dressUp: false, idle: true })}
                  >忘れる</button
                >
              {:else}
                <span class="none">—</span>
              {/if}
            </td>
            <td class="date" class:none={row.rememberedAt === null}>
              {describeRememberedAt(row.rememberedAt)}
            </td>
            <td>
              {#if rememberedCount(row) > 1}
                <button
                  type="button"
                  class="small"
                  aria-label="{row.model.name} に覚えたものをすべて忘れる"
                  onclick={() => forget(row, {})}>すべて忘れる</button
                >
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  <footer>
    {#if confirming}
      <span role="alert">{rows.length} 体のモデルに覚えたものをすべて忘れます。よろしいですか?</span>
      <button type="button" class="danger" onclick={forgetAll}>すべて忘れる</button>
      <button type="button" onclick={() => (confirming = false)}>やめる</button>
    {:else}
      <span class="hint">最後に使った光源・光源の印・前の組を自動で戻す・待機モーションを自動で流すの設定は残ります。</span>
      <button type="button" disabled={rows.length === 0} onclick={() => (confirming = true)}
        >すべて忘れる…</button
      >
    {/if}
  </footer>
</dialog>

<style>
  dialog {
    width: min(820px, 92vw);
    max-height: 85vh;
    padding: 1rem;
    color: var(--text);
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 0.85rem;
  }
  header,
  footer {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  header {
    justify-content: space-between;
  }
  footer {
    justify-content: flex-end;
    margin-top: 0.75rem;
  }
  footer span {
    flex: 1;
  }
  h2 {
    margin: 0;
    font-size: 1rem;
  }
  .lead,
  .hint,
  .location,
  .none,
  .empty {
    color: var(--muted);
  }
  .lead {
    font-size: 0.8rem;
  }
  .order {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.4rem;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .date {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    font-size: 0.75rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    text-align: left;
    vertical-align: top;
    padding: 0.35rem 0.4rem;
    border-bottom: 1px solid var(--border);
  }
  th {
    font-weight: 600;
    font-size: 0.75rem;
    color: var(--muted);
  }
  tr.current td {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }
  .name {
    font-weight: 600;
    word-break: break-all;
  }
  .location {
    font-size: 0.7rem;
    word-break: break-all;
  }
  .badge {
    margin-left: 0.3rem;
    padding: 0 0.3rem;
    font-size: 0.7rem;
    font-weight: normal;
    color: var(--accent);
    border: 1px solid var(--accent);
    border-radius: 3px;
  }
  td button.small {
    margin-left: 0.3rem;
    font-size: 0.75rem;
    padding: 0.05rem 0.4rem;
  }
  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .danger {
    color: var(--danger);
    border-color: var(--danger);
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
