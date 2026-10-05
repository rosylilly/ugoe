<!--
  パラメータのパネルの「Groups」: パラメータのグループ (Model::parameter_groups。プレビューとビューアのスライダーの区切り) を
  作る・名前を変える・消す・並べ替える (docs/editor.md#パラメータのグループ)。どれも Command を 1 つ送る (Undo の 1 単位)。
  消してもパラメータは残り、割り当てだけが外れる。パラメータをグループに入れるのは各パラメータの編集欄の「Group」。
  モジュールのプロジェクトでは本体のものなので変えられない。
-->
<script lang="ts">
import { Button, IconButton, Section, Tooltip } from "@ugoe/ui";
import BaseLock from "../../BaseLock.svelte";
import type { EditorDocument, ParameterDef } from "../../document.svelte";
import { t } from "../../i18n";
import { elementLabel } from "../../labels";
import { addGroupCommand, memberCount, moveGroupCommand } from "./groups";

interface Props {
  doc: EditorDocument;
  parameters: ParameterDef[];
}

let { doc, parameters }: Props = $props();

const groups = $derived(doc.parsedModel?.parameter_groups ?? []);
const locked = $derived(doc.module !== null);

let open = $state(false);
let creating = $state(false);
let fresh = $state({ id: "", name: "" });
/** 名前を編集中のグループ (null なら閉じている)。 */
let editing = $state<string | null>(null);
let draftName = $state("");

const ids = $props.id();

function startCreating() {
  open = true;
  creating = true;
}

async function create(e: SubmitEvent) {
  e.preventDefault();
  const command = addGroupCommand(fresh.id, fresh.name);
  if (!command) return;
  if ((await doc.apply(command)) !== null) {
    creating = false;
    fresh = { id: "", name: "" };
  }
}

function openEditor(id: string, name: string) {
  editing = editing === id ? null : id;
  draftName = name;
}

async function rename(e: SubmitEvent, id: string, name: string) {
  e.preventDefault();
  const next = draftName.trim() || id;
  if (
    next === name ||
    (await doc.apply({ op: "rename_parameter_group", id, name: next })) !== null
  ) {
    editing = null;
  }
}

async function remove(id: string) {
  if ((await doc.apply({ op: "remove_parameter_group", id })) !== null) editing = null;
}

async function move(id: string, delta: -1 | 1) {
  const command = moveGroupCommand(groups, id, delta);
  if (command) await doc.apply(command);
}
</script>

<Section title={t("parameters.groups.title", { count: groups.length })} bind:open>
  {#snippet actions()}
    <IconButton
      small
      variant="secondary"
      onclick={startCreating}
      disabled={locked}
      label={t("parameters.groups.add.aria")}
      aria-describedby={locked ? `${ids}-note` : undefined}>+</IconButton
    >
  {/snippet}
  <BaseLock {locked} hint={t("parameters.groups.baseHint")}>
    <p class="ed-hint" id="{ids}-note">{t("parameters.groups.description")}</p>
    {#if creating}
      <form class="editor" onsubmit={create}>
        <label
          >{t("parameters.field.id")}
          <!-- i18n-ignore: グループの ID の例 (訳さない) -->
          <input bind:value={fresh.id} required placeholder="Face" /></label
        >
        <label
          >{t("parameters.field.name")}
          <input bind:value={fresh.name} placeholder={t("parameters.field.namePlaceholder")} /></label
        >
        <span class="ed-buttons">
          <Button small type="submit">{t("parameters.create.label")}</Button>
          <Button small onclick={() => (creating = false)}>{t("parameters.cancel.label")}</Button>
        </span>
      </form>
    {:else if groups.length === 0}
      <p class="ed-hint">{t("parameters.groups.empty")}</p>
      <span class="ed-buttons">
        <Button small onclick={startCreating}>{t("parameters.groups.add.label")}</Button>
      </span>
    {/if}
    <ul>
      {#each groups as g, i (g.id)}
        {@const label = elementLabel(g.name, g.id)}
        <li>
          <span class="row">
            <Tooltip text={t("parameters.groups.edit.tooltip", { id: g.id })}>
              {#snippet trigger(attrs)}
                <button
                  {...attrs}
                  class="name"
                  aria-expanded={editing === g.id}
                  onclick={(e) => {
                    (attrs.onclick as ((e: MouseEvent) => void) | undefined)?.(e);
                    openEditor(g.id, g.name);
                  }}>{label}</button
                >
              {/snippet}
            </Tooltip>
            <span class="count"
              >{t("parameters.groups.members", { count: memberCount(parameters, g.id) })}</span
            >
            <IconButton
              small
              disabled={i === 0}
              label={t("parameters.groups.moveUp.aria", { name: label })}
              onclick={() => move(g.id, -1)}>▲</IconButton
            >
            <IconButton
              small
              disabled={i === groups.length - 1}
              label={t("parameters.groups.moveDown.aria", { name: label })}
              onclick={() => move(g.id, 1)}>▼</IconButton
            >
          </span>
          {#if editing === g.id}
            <form class="editor" onsubmit={(e) => rename(e, g.id, g.name)}>
              <label>{t("parameters.field.name")} <input bind:value={draftName} /></label>
              <p class="ed-hint" id="{ids}-delete-{g.id}">{t("parameters.groups.delete.description")}</p>
              <span class="ed-buttons">
                <Button small type="submit">{t("parameters.apply.label")}</Button>
                <Button
                  small
                  variant="danger"
                  aria-describedby="{ids}-delete-{g.id}"
                  onclick={() => remove(g.id)}>{t("parameters.groups.delete.label")}</Button
                >
              </span>
            </form>
          {/if}
        </li>
      {/each}
    </ul>
  </BaseLock>
</Section>

<style>
ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

.row {
  display: flex;
  align-items: center;
  gap: var(--ugoe-space-1);
}

.name {
  flex: 1;
  min-width: 0;
  min-height: var(--ugoe-target-min);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  background: none;
  border: none;
  padding: 0;
  color: inherit;
  cursor: pointer;
  text-align: left;
}

.count {
  color: var(--ugoe-color-text-muted);
  font-size: var(--ugoe-font-size-sm);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.editor {
  display: flex;
  flex-direction: column;
  gap: var(--ugoe-space-1);
  padding: var(--ugoe-space-1) 0 var(--ugoe-space-2);
}
</style>
