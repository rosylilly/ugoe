<script lang="ts">
// 着せ替えのパネル (T0706)。スロットを持つモデルか、モジュールを読み込んだときに App.svelte が出す。
// スロットごとに、読み込んだモジュールを並べて付ける・外す・差し替える・一覧から除く。
// 付け外しは Player (WebPuppet の着せ替えの口) が行い、失敗の理由は `player.dressUpError`。
// 本体のアバタープロファイルがアプリに公開していないスロット (作者用) は出さない (T0715。`player.publishedSlots`)。
// モデルに前に付けていた組があれば、戻すか尋ねる (T0711。`player.restoreOffer`)。覚えた組は「覚えた組を忘れる」で忘れる (T0709)。
import { formatList } from "@ugoe/i18n";
import { groupBySlot, type ModuleInfo, type Player, visibleSlots } from "@ugoe/player";
import { Button, Icon, IconButton, Notice, Section, Toggle } from "@ugoe/ui";
import { errorText } from "./errors";
import { viewerHost } from "./host";
import { t } from "./i18n";

let { player, hasModel }: { player: Player; hasModel: boolean } = $props();

let open = $state(true);

let slots = $derived(visibleSlots(player.slots, player.publishedSlots));
let groups = $derived(groupBySlot(player.slots, player.modules, player.publishedSlots));
let attachedCount = $derived(
  player.modules.filter((m) => m.attached && slots.some((s) => s.id === m.slot)).length,
);
let status = $derived(
  slots.length === 0
    ? t("dressUp.status.noSlots")
    : t("dressUp.status.slots", { attached: attachedCount, count: slots.length }),
);

async function openModules() {
  try {
    const files = await viewerHost().pickModuleFiles();
    for (const file of files) player.loadModule(file);
  } catch (e) {
    player.dressUpError = errorText(e);
  }
}

let offer = $derived(player.restoreOffer);
let offerText = $derived(
  offer === null
    ? ""
    : offer.attached.length > 0
      ? formatList(offer.attached)
      : t("dressUp.restore.notAttached", { count: offer.modules.length }),
);

function describe(m: ModuleInfo): string {
  const lines = [
    t("dressUp.module.slot", { slot: m.slot }),
    t("dressUp.module.textures", { count: m.textures }),
  ];
  const r = m.requires;
  if (r.avatar_level !== undefined)
    lines.push(t("dressUp.module.avatarLevel", { level: r.avatar_level }));
  if (r.slots?.length) lines.push(t("dressUp.module.slots", { slots: formatList(r.slots) }));
  if (r.parameters?.length)
    lines.push(t("dressUp.module.parameters", { parameters: formatList(r.parameters) }));
  return lines.join("\n");
}
</script>

<div class="panel dress-up">
  <Section title={t("dressUp.title")} level={2} bind:open>
    {#snippet actions()}
      <span class="status">{status}</span>
    {/snippet}

    <div class="actions">
      <Button small onclick={openModules}>{t("dressUp.openModules.label")}</Button>
      <Toggle
        label={t("dressUp.autoRestore.label")}
        checked={player.autoRestore}
        onchange={(on) => player.setAutoRestore(on)}
      />
    </div>

    {#if player.dressUpRemembered}
      <div class="memory">
        <span>{t("dressUp.memory.remembered")}</span>
        <Button small onclick={() => player.forgetDressUp()}>{t("dressUp.memory.forget.label")}</Button>
      </div>
    {/if}

    {#if offer !== null}
      <div class="notice">
        <Notice title={t("dressUp.restore.title")}>
          {offerText}
          {#snippet actions()}
            <Button small variant="primary" onclick={() => void player.restoreDressUp()}
              >{t("dressUp.restore.accept.label")}</Button
            >
            <Button small onclick={() => player.dismissRestore()}
              >{t("dressUp.restore.dismiss.label")}</Button
            >
          {/snippet}
        </Notice>
      </div>
    {/if}

    {#if player.dressUpError}
      <div class="notice error">
        <Notice tone="danger">{player.dressUpError}</Notice>
      </div>
    {/if}
    {#if !hasModel}
      <p class="hint">{t("dressUp.hint.noModel")}</p>
    {:else if player.slots.length === 0}
      <p class="hint">{t("dressUp.hint.noSlots")}</p>
    {:else if slots.length === 0}
      <p class="hint">{t("dressUp.hint.noPublishedSlots")}</p>
    {/if}

    {#each groups as g (g.id)}
      <section class:missing={g.missing || g.hidden}>
        <h3>
          {g.id}
          {#if g.missing}
            <span class="badge">{t("dressUp.badge.missing")}</span>
          {:else if g.hidden}
            <span class="badge">{t("dressUp.badge.hidden")}</span>
          {:else}
            <span class="current">{g.attached?.name ?? t("dressUp.none")}</span>
          {/if}
        </h3>
        {#if g.missing || g.hidden}
          <p class="hint">
            {g.missing ? t("dressUp.badge.missingDescription") : t("dressUp.badge.hiddenDescription")}
          </p>
        {/if}
        {#if g.modules.length === 0}
          <p class="hint">{t("dressUp.empty")}</p>
        {:else}
          <ul>
            {#each g.modules as m (m.name)}
              <li class:attached={m.attached}>
                <span class="name" title={describe(m)}>{m.name}</span>
                {#if m.warnings.length > 0}
                  <span class="warning" title={m.warnings.join("\n")}
                    >{t("dressUp.module.warnings", { count: m.warnings.length })}</span
                  >
                {/if}
                {#if m.attached}
                  <Button small onclick={() => player.detachModule(m.name)}
                    >{t("dressUp.detach.label")}</Button
                  >
                {:else}
                  <Button
                    small
                    disabled={g.missing || g.hidden || !hasModel}
                    onclick={() => player.attachModule(m.name)}
                    >{g.attached ? t("dressUp.replace.label") : t("dressUp.attach.label")}</Button
                  >
                {/if}
                <IconButton
                  small
                  variant="ghost"
                  label={t("dressUp.unload.aria", { module: m.name })}
                  onclick={() => player.unloadModule(m.name)}
                >
                  <Icon name="close" />
                </IconButton>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/each}
  </Section>
</div>

<style>
  /* 高さは中身のまま (右の列の上の段 (App.svelte) がまとめて送る。T0725)。 */
  .dress-up {
    font-size: var(--ugoe-font-size-sm);
  }
  .dress-up :global(.ugoe-toggle) {
    font-size: var(--ugoe-font-size-sm);
  }
  h3 {
    margin: 0;
    font-size: var(--ugoe-font-size-sm);
    font-weight: var(--ugoe-font-weight-strong);
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
  }
  .status,
  .current,
  .hint,
  .memory {
    color: var(--ugoe-color-text-muted);
    font-weight: var(--ugoe-font-weight-normal);
  }
  .status {
    font-size: var(--ugoe-font-size-xs);
  }
  .hint {
    margin: var(--ugoe-space-1) 0 0;
    font-size: var(--ugoe-font-size-xs);
  }
  .actions,
  .memory {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--ugoe-space-2);
  }
  .memory {
    margin-top: var(--ugoe-space-2);
    font-size: var(--ugoe-font-size-xs);
  }
  .notice {
    margin-top: var(--ugoe-space-2);
  }
  .notice.error {
    white-space: pre-line;
  }
  section {
    margin-top: var(--ugoe-space-3);
  }
  section.missing h3 {
    color: var(--ugoe-color-text-muted);
  }
  .badge {
    font-size: var(--ugoe-font-size-xs);
    font-weight: var(--ugoe-font-weight-normal);
    color: var(--ugoe-color-warning);
    border: 1px solid currentColor;
    border-radius: var(--ugoe-radius-sm);
    padding: 0 var(--ugoe-space-1);
  }
  ul {
    list-style: none;
    margin: var(--ugoe-space-1) 0 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: var(--ugoe-space-2);
    padding: var(--ugoe-space-1) 0;
  }
  li .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  li.attached .name {
    font-weight: var(--ugoe-font-weight-strong);
    color: var(--ugoe-color-accent);
  }
  .warning {
    color: var(--ugoe-color-warning);
    font-size: var(--ugoe-font-size-xs);
  }
</style>
