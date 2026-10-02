<!--
  操作 API の承認の知らせ (docs/studio.md#操作ウィンドウ、ADR 0028 の 6.): 初めて接続したクライアントが名乗った名前を出し、
  利用者が権限 (読むだけ / 操作する) を選んで承認するまで、クライアントには何もさせない。「後で」は知らせを隠すだけで、
  クライアントは待ったまま (操作 API のパネルの「承認を待つアプリ」から出し直せる)。
-->
<script lang="ts">
import { Button, Notice, SegmentedControl } from "@ugoe/ui";
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

// 初めの選択はクライアントが求めた権限 (利用者が狭められる)。
let permission = $state<ApiPermission>("read");
$effect.pre(() => {
  permission = client.permission;
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
    <Button small variant="primary" {disabled} onclick={() => onapprove(permission)}>{t("api.approve")}</Button>
    <Button small {disabled} onclick={ondeny}>{t("api.deny")}</Button>
    <Button small onclick={onlater}>{t("api.later")}</Button>
  {/snippet}
</Notice>

<style>
  .approval__permission {
    margin-top: var(--ugoe-space-2);
  }
</style>
