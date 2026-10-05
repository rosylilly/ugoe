<!--
  Menu と ContextMenu の項目の並び (内部用)。Bits UI の DropdownMenu と ContextMenu は項目の部品を共有するので、両方で使う。
  ショートカットは控えめな文字 (Ctrl+S・⌘S) で見た目だけに出し (aria-hidden)、読み上げには aria-keyshortcuts で渡す。
-->
<script lang="ts">
import { DropdownMenu } from "bits-ui";
import { ariaKeyShortcuts, formatShortcut } from "../shortcut";
import type { MenuEntry } from "../types";
import Icon from "./Icon.svelte";
import Self from "./MenuEntries.svelte";

let { entries }: { entries: MenuEntry[] } = $props();
</script>

{#snippet shortcutOf(shortcut: string | undefined)}
  {#if shortcut}
    <kbd class="ugoe-menu__shortcut" aria-hidden="true">{formatShortcut(shortcut)}</kbd>
  {/if}
{/snippet}

{#each entries as entry, i (i)}
  {#if entry.kind === "separator"}
    <DropdownMenu.Separator class="ugoe-menu__separator" />
  {:else if entry.kind === "checkbox"}
    <DropdownMenu.CheckboxItem
      class="ugoe-menu__item"
      checked={entry.checked}
      disabled={entry.disabled}
      textValue={entry.label}
      aria-keyshortcuts={entry.shortcut ? ariaKeyShortcuts(entry.shortcut) : undefined}
      onCheckedChange={(v) => entry.onchange(v)}
    >
      {#snippet children({ checked })}
        <span class="ugoe-menu__check">{#if checked}<Icon name="check" />{/if}</span>
        <span class="ugoe-menu__label">{entry.label}</span>
        {@render shortcutOf(entry.shortcut)}
      {/snippet}
    </DropdownMenu.CheckboxItem>
  {:else if entry.kind === "submenu"}
    <DropdownMenu.Sub>
      <DropdownMenu.SubTrigger class="ugoe-menu__item" disabled={entry.disabled} textValue={entry.label}>
        <span class="ugoe-menu__check"></span>
        <span class="ugoe-menu__label">{entry.label}</span>
        <Icon name="chevronRight" />
      </DropdownMenu.SubTrigger>
      <DropdownMenu.SubContent class="ugoe-menu" aria-label={entry.label} sideOffset={2}>
        <Self entries={entry.entries} />
      </DropdownMenu.SubContent>
    </DropdownMenu.Sub>
  {:else}
    <DropdownMenu.Item
      class={`ugoe-menu__item${entry.danger ? " ugoe-menu__item--danger" : ""}`}
      disabled={entry.disabled}
      textValue={entry.label}
      aria-keyshortcuts={entry.shortcut ? ariaKeyShortcuts(entry.shortcut) : undefined}
      onSelect={() => entry.onselect()}
    >
      <span class="ugoe-menu__check"></span>
      <span class="ugoe-menu__label">{entry.label}</span>
      {@render shortcutOf(entry.shortcut)}
    </DropdownMenu.Item>
  {/if}
{/each}

<style>
  :global {
    .ugoe-menu {
      z-index: var(--ugoe-z-popover);
      box-sizing: border-box;
      min-width: 180px;
      max-height: var(--bits-dropdown-menu-content-available-height, 80vh);
      padding: var(--ugoe-space-1);
      overflow: auto;
      border: 1px solid var(--ugoe-color-border);
      border-radius: var(--ugoe-radius-lg);
      background: var(--ugoe-color-surface);
      box-shadow: var(--ugoe-shadow-overlay);
      color: var(--ugoe-color-text);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height-control);
      animation: ugoe-fade-in var(--ugoe-duration-fast) ease-out;
    }

    .ugoe-menu:focus-visible {
      outline: none;
    }

    .ugoe-menu__item {
      display: flex;
      align-items: center;
      gap: var(--ugoe-space-2);
      min-height: max(var(--ugoe-control-height), var(--ugoe-target-min));
      padding: 0 var(--ugoe-space-2);
      border-radius: var(--ugoe-radius-sm);
      cursor: pointer;
      user-select: none;
    }

    .ugoe-menu__item[data-highlighted] {
      background: var(--ugoe-color-accent-soft);
      outline: 1px solid var(--ugoe-color-accent);
      outline-offset: -1px;
    }

    .ugoe-menu__item[data-disabled] {
      cursor: default;
      opacity: var(--ugoe-opacity-disabled);
    }

    .ugoe-menu__item--danger:not([data-disabled]) {
      color: var(--ugoe-color-danger);
    }

    .ugoe-menu__check {
      display: inline-flex;
      flex: none;
      width: var(--ugoe-icon-size);
    }

    .ugoe-menu__label {
      flex: 1;
      white-space: nowrap;
    }

    .ugoe-menu__shortcut {
      margin-left: var(--ugoe-space-4);
      color: var(--ugoe-color-text-muted);
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-sm);
    }

    .ugoe-menu__separator {
      height: 1px;
      margin: var(--ugoe-space-1) 0;
      background: var(--ugoe-color-border);
    }
  }
</style>
