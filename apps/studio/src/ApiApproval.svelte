<!--
  操作 API の承認の知らせ (docs/studio.md#操作ウィンドウ、ADR 0028 の 6.): 初めて接続したクライアントが名乗った名前を出し、
  利用者が権限 (読むだけ / 操作する) を選んで承認するまで、クライアントには何もさせない。「後で」は知らせを隠すだけで、
  クライアントは待ったまま (操作 API のパネルの「承認を待つアプリ」から出し直せる)。
-->
<script lang="ts">
import { Button, Notice, SegmentedControl } from "@ugoe/ui";
import { untrack } from "svelte";
import { API_PERMISSIONS, type ApiPendingClient, type ApiPermission } from "./lib/api";
import { t } from "./lib/i18n";

interface Props {
  client: ApiPendingClient;
  disabled?: boolean;
  onapprove: (permission: ApiPermission) => void;
  ondeny: () => void;
  onlater: () => void;
}

let { client, disabled = false, onapprove, ondeny, onlater }: Props = $props();

// 初めの選択はクライアントが求めた権限 (利用者が狭められる)。様子の通知のたびに `client` は新しいものが届くので、
// 当て直すのは別のクライアントになったときだけ (利用者が狭めた選択を、承認の前に戻さない)。
let permission = $state<ApiPermission>("read");
let chosenFor: string | null = null;
$effect.pre(() => {
  if (client.id === chosenFor) return;
  chosenFor = client.id;
  permission = untrack(() => client.permission);
});

const options = $derived(
  API_PERMISSIONS.map((value) => ({
    value,
    label: t(value === "control" ? "api.permissionControl" : "api.permissionRead"),
  })),
);
</script>

<Notice tone="warning" title={t("api.approveTitle", { name: client.name })}>
  {t("api.approveBody")}
  <div class="approval__permission">
    <SegmentedControl label={t("api.permission")} {options} bind:value={permission} {disabled} />
  </div>
  {#snippet actions()}
    <!-- 並びはエディタのダイアログ・知らせと同じく、保留 → 断る → 決定 (primary は並びの最後。docs/ui.md#エディタの中の共通の形) -->
    <Button small onclick={onlater}>{t("api.later")}</Button>
    <Button small {disabled} onclick={ondeny}>{t("api.deny")}</Button>
    <Button small variant="primary" {disabled} onclick={() => onapprove(permission)}>{t("api.approve")}</Button>
  {/snippet}
</Notice>

<style>
  .approval__permission {
    margin-top: var(--ugoe-space-2);
  }

  /* 狭い幅 (360px) で「権限」の名前が字の途中で折れないよう、名前は折らずに選択肢を次の行へ送る (T0818) */
  .approval__permission :global(.ugoe-segmented) {
    flex-wrap: wrap;
  }

  .approval__permission :global(.ugoe-segmented__label) {
    white-space: nowrap;
  }
</style>
