<!--
  操作 API (docs/studio.md#外部-api、T0805): 外のツール (Stream Deck・チャットのボットなど) から操作させる待ち受けのスイッチ・
  ポート・トークン (写す・作り直す)・承認したクライアントの一覧 (権限・切断・取り消し)。既定は切で、ループバックだけで待つ。
  承認を待つクライアントは、操作ウィンドウの上の知らせ (ApiApproval.svelte) で承認する。「後で」で隠した知らせは、ここから出し直す。
-->
<script lang="ts">
import { formatDate } from "@ugoe/i18n";
import { AlertDialog, Button, Notice, NumberField, Toggle } from "@ugoe/ui";
import { API_PORT_RANGE, type ApiClient, type ApiView, apiUrl, isApiPort } from "./lib/api";
import { t } from "./lib/i18n";

interface Props {
  api: ApiView;
  disabled?: boolean;
  onenabled: (enabled: boolean) => void;
  onport: (port: number) => void;
  /** トークンを取る (写すため)。 */
  token: () => Promise<string>;
  onregenerate: () => void;
  ondisconnect: (id: string) => void;
  onrevoke: (id: string) => void;
  /** 「後で」で隠した承認の知らせを出し直す。 */
  onreview: (id: string) => void;
}

let {
  api,
  disabled = false,
  onenabled,
  onport,
  token,
  onregenerate,
  ondisconnect,
  onrevoke,
  onreview,
}: Props = $props();

/** 写した・写せなかったもの。 */
let copied = $state<"url" | "token" | "failed" | null>(null);
let confirmRegenerate = $state(false);
let revoking = $state<ApiClient | null>(null);

async function copy(kind: "url" | "token", text: () => Promise<string>) {
  try {
    await navigator.clipboard.writeText(await text());
    copied = kind;
  } catch {
    copied = "failed";
  }
}

function changePort(port: number) {
  if (isApiPort(port) && port !== api.port) onport(port);
}

function seen(client: ApiClient): string {
  if (client.connections > 0) return t("api.connected", { n: client.connections });
  if (client.lastSeen === null) return t("api.neverSeen");
  return t("api.lastSeen", {
    when: formatDate(client.lastSeen * 1000, { dateStyle: "medium", timeStyle: "short" }),
  });
}
</script>

<div class="api">
  <Toggle
    label={t("api.enable")}
    checked={api.enabled}
    {disabled}
    onchange={onenabled}
  />
  <p class="api__hint">{t("api.hint")}</p>

  {#if api.error !== null}
    <Notice tone="danger">{t("api.error", { message: api.error })}</Notice>
  {/if}

  {#if api.enabled && api.listening}
    <div class="api__row">
      <span class="api__label">{t("api.address")}</span>
      <code class="api__value ugoe-selectable">{apiUrl(api.port)}</code>
      <Button small onclick={() => copy("url", async () => apiUrl(api.port))}>{t("api.copy")}</Button>
    </div>
    <p class="api__hint">{t("api.connections", { count: api.connections })}</p>
  {/if}

  <NumberField
    label={t("api.port")}
    value={api.port}
    min={API_PORT_RANGE.min}
    max={API_PORT_RANGE.max}
    step={1}
    decimals={0}
    {disabled}
    onchange={changePort}
  />

  <div class="api__row">
    <span class="api__label">{t("api.token")}</span>
    <Button small {disabled} onclick={() => copy("token", token)}>{t("api.copyToken")}</Button>
    <Button small {disabled} onclick={() => (confirmRegenerate = true)}>{t("api.regenerate")}</Button>
  </div>
  <p class="api__hint">{t("api.tokenHint")}</p>
  {#if copied !== null}
    <p class="api__hint" role="status">
      {copied === "failed" ? t("api.copyFailed") : copied === "url" ? t("api.copiedUrl") : t("api.copiedToken")}
    </p>
  {/if}

  {#if api.pending.length > 0}
    <h3 class="api__heading">{t("api.pending")}</h3>
    <ul class="api__clients">
      {#each api.pending as client (client.id)}
        <li class="api__client">
          <span class="api__client-text api__client-name">{client.name}</span>
          <Button small {disabled} onclick={() => onreview(client.id)}>{t("api.review")}</Button>
        </li>
      {/each}
    </ul>
  {/if}

  <h3 class="api__heading">{t("api.clients")}</h3>
  {#if api.clients.length === 0}
    <p class="api__hint">{t("api.noClients")}</p>
  {:else}
    <ul class="api__clients">
      {#each api.clients as client (client.id)}
        <li class="api__client">
          <div class="api__client-text">
            <span class="api__client-name">{client.name}</span>
            <span class="api__hint">
              {t("api.clientSummary", {
                permission: t(client.permission === "control" ? "api.permissionControl" : "api.permissionRead"),
                seen: seen(client),
              })}
            </span>
          </div>
          {#if client.connections > 0}
            <Button small {disabled} onclick={() => ondisconnect(client.id)}>{t("api.disconnect")}</Button>
          {/if}
          <Button small {disabled} onclick={() => (revoking = client)}>{t("api.revoke")}</Button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<AlertDialog
  open={confirmRegenerate}
  title={t("api.regenerateTitle")}
  description={t("api.regenerateBody")}
  confirmLabel={t("api.regenerate")}
  onconfirm={() => {
    confirmRegenerate = false;
    copied = null;
    onregenerate();
  }}
  oncancel={() => (confirmRegenerate = false)}
/>

<AlertDialog
  open={revoking !== null}
  title={t("api.revokeTitle", { name: revoking?.name ?? "" })}
  description={t("api.revokeBody")}
  confirmLabel={t("api.revoke")}
  onconfirm={() => {
    const client = revoking;
    revoking = null;
    if (client !== null) onrevoke(client.id);
  }}
  oncancel={() => (revoking = null)}
/>

<style>
  .api {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  .api__row {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
    flex-wrap: wrap;
  }

  .api__label {
    color: var(--ugoe-color-text-muted);
  }

  .api__value {
    font-family: var(--ugoe-font-mono);
    font-size: var(--ugoe-font-size-sm);
    overflow-wrap: anywhere;
  }

  .api__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }

  .api__heading {
    margin: var(--ugoe-space-2) 0 0;
    font-size: var(--ugoe-font-size-md);
    font-weight: var(--ugoe-font-weight-strong);
  }

  .api__clients {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
  }

  .api__client {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
  }

  .api__client-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .api__client-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
