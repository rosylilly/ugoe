<script lang="ts">
// モデルごとに覚えたもの (光源の設定 (T0709)・着せ替えの組 (T0711)・待機モーションの選択 (T0722)) の一覧 (T0714)。
// ツールバーの「覚えたもの」とライティングのパネルから開く。モデルごとに光源・組・待機モーションを忘れるか、すべて忘れる。
// パネルが出ないモデル (モジュールを外してライティングを受けなくなったモデルなど) や、開いていないモデルの分もここで忘れられる。
// 並びは覚えた日時の新しい順と名前の順から選ぶ (T0717。日時を持たない前の保存値は「日時なし」で最後)。
// 選んだ並びは覚えて、次に開いたときも使う (T0718)。「すべて忘れる…」は確かめのダイアログ (AlertDialog) で確かめる。
import {
  describeDressUpSet,
  describeIdleChoice,
  describeLocation,
  describeRememberedAt,
  lightingLabel,
  loadRememberedOrder,
  type Player,
  type RememberedModel,
  type RememberedOrder,
  rememberedCount,
  saveRememberedOrder,
} from "@ugoe/player";
import { AlertDialog, Button, Dialog, Select } from "@ugoe/ui";
import { t } from "./i18n";

let { player }: { player: Player } = $props();

let shown = $state(false);
let rows = $state<RememberedModel[]>([]);
/** 並び (覚えた日時の新しい順か名前の順。T0717)。開いているモデルはどちらでも先頭。選択は覚える (T0718)。 */
let order = $state<RememberedOrder>(loadRememberedOrder());
/** 「すべて忘れる…」を押して、確かめている間。 */
let confirming = $state(false);

let orderOptions = $derived([
  { value: "recent" as RememberedOrder, label: t("memory.order.recent") },
  { value: "name" as RememberedOrder, label: t("memory.order.name") },
]);

export function open(): void {
  // ほかのウィンドウ (タブ) で選び直した並びも拾う。
  order = loadRememberedOrder();
  refresh();
  confirming = false;
  shown = true;
}

function refresh() {
  rows = player.rememberedModels(order);
}

function changeOrder(value: RememberedOrder) {
  order = value;
  saveRememberedOrder(order);
  refresh();
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

<Dialog bind:open={shown} title={t("memory.title")} description={t("memory.description")} size="lg">
  {#if rows.length > 1}
    <div class="order">
      <Select label={t("memory.order.label")} value={order} options={orderOptions} onchange={changeOrder} />
    </div>
  {/if}

  {#if rows.length === 0}
    <p class="empty">{t("memory.empty")}</p>
  {:else}
    <div class="table">
      <table>
        <thead>
          <tr>
            <th scope="col">{t("memory.column.model")}</th>
            <th scope="col">{t("memory.column.lighting")}</th>
            <th scope="col">{t("memory.column.dressUp")}</th>
            <th scope="col">{t("memory.column.idle")}</th>
            <th scope="col">{t("memory.column.date")}</th>
            <th scope="col"><span class="visually-hidden">{t("memory.column.actions")}</span></th>
          </tr>
        </thead>
        <tbody>
          {#each rows as row (`${row.model.location ?? ""}|${row.model.fingerprint}`)}
            {@const name = row.model.name || t("memory.unnamed")}
            <tr class:current={row.current} data-model={row.model.name}>
              <td>
                <div class="name">
                  {name}
                  {#if row.current}<span class="badge">{t("memory.current")}</span>{/if}
                </div>
                <div class="location" title={row.model.location ?? ""}>{describeLocation(row.model)}</div>
              </td>
              <td>
                {#if row.lighting}
                  <span>{lightingLabel(row.lighting)}</span>
                  <Button small onclick={() => forget(row, { lighting: true, dressUp: false, idle: false })}
                    ><span aria-hidden="true">{t("memory.forget.label")}</span><span class="visually-hidden"
                      >{t("memory.forget.lighting.aria", { model: name })}</span
                    ></Button
                  >
                {:else}
                  <span class="none">—</span>
                {/if}
              </td>
              <td>
                {#if row.dressUp}
                  <span>{describeDressUpSet(row.dressUp)}</span>
                  <Button small onclick={() => forget(row, { lighting: false, dressUp: true, idle: false })}
                    ><span aria-hidden="true">{t("memory.forget.label")}</span><span class="visually-hidden"
                      >{t("memory.forget.dressUp.aria", { model: name })}</span
                    ></Button
                  >
                {:else}
                  <span class="none">—</span>
                {/if}
              </td>
              <td>
                {#if row.idle}
                  <span>{describeIdleChoice(row.idle)}</span>
                  <Button small onclick={() => forget(row, { lighting: false, dressUp: false, idle: true })}
                    ><span aria-hidden="true">{t("memory.forget.label")}</span><span class="visually-hidden"
                      >{t("memory.forget.idle.aria", { model: name })}</span
                    ></Button
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
                  <Button small onclick={() => forget(row, {})}
                    ><span aria-hidden="true">{t("memory.forgetRow.label")}</span><span class="visually-hidden"
                      >{t("memory.forgetRow.aria", { model: name })}</span
                    ></Button
                  >
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  {#snippet actions()}
    <span class="hint">{t("memory.kept")}</span>
    <Button variant="danger" disabled={rows.length === 0} onclick={() => (confirming = true)}
      >{t("memory.forgetAll.label")}</Button
    >
  {/snippet}
</Dialog>

<AlertDialog
  bind:open={confirming}
  title={t("memory.forgetAll.title")}
  description={t("memory.forgetAll.description", { count: rows.length })}
  confirmLabel={t("memory.forgetAll.confirm")}
  onconfirm={forgetAll}
/>

<style>
  .order {
    margin-bottom: var(--ugoe-space-3);
  }
  .table {
    max-height: 55vh;
    overflow: auto;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--ugoe-font-size-sm);
  }
  th,
  td {
    text-align: left;
    vertical-align: top;
    padding: var(--ugoe-space-2);
    border-bottom: 1px solid var(--ugoe-color-border);
  }
  th {
    font-weight: var(--ugoe-font-weight-strong);
    font-size: var(--ugoe-font-size-xs);
    color: var(--ugoe-color-text-muted);
  }
  td :global(.ugoe-button) {
    margin-left: var(--ugoe-space-1);
  }
  tr.current td {
    background: var(--ugoe-color-accent-soft);
  }
  .name {
    font-weight: var(--ugoe-font-weight-strong);
    word-break: break-all;
  }
  .location {
    font-size: var(--ugoe-font-size-xs);
    word-break: break-all;
  }
  .date {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .hint,
  .location,
  .none,
  .empty {
    color: var(--ugoe-color-text-muted);
  }
  .hint {
    flex: 1;
    font-size: var(--ugoe-font-size-xs);
  }
  .badge {
    margin-left: var(--ugoe-space-1);
    padding: 0 var(--ugoe-space-1);
    font-size: var(--ugoe-font-size-xs);
    font-weight: var(--ugoe-font-weight-normal);
    color: var(--ugoe-color-accent);
    border: 1px solid var(--ugoe-color-accent);
    border-radius: var(--ugoe-radius-sm);
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
