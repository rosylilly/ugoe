<!--
  Menu と ContextMenu の項目の並び (内部用)。Bits UI の DropdownMenu と ContextMenu は項目の部品を共有するので、両方で使う。
  ショートカットは控えめな文字 (Ctrl+S・⌘S) で見た目だけに出し (aria-hidden)、読み上げには aria-keyshortcuts で渡す。
  説明の行 (`description`) は名前の下に控えめな文字で出し、名前とは分けて説明として読ませる (aria-labelledby・aria-describedby)。
-->
<script lang="ts">
import { DropdownMenu } from "bits-ui";
import { markScrolling } from "../scrollGap";
import { ariaKeyShortcuts, formatShortcut } from "../shortcut";
import type { MenuEntry } from "../types";
import Icon from "./Icon.svelte";
import Self from "./MenuEntries.svelte";

let { entries }: { entries: MenuEntry[] } = $props();

const id = $props.id();

/** 説明の行があるときに、名前と説明を分けて結ぶ属性。 */
function described(entry: { description?: string }, i: number) {
  if (!entry.description) return {};
  return { "aria-labelledby": `${id}-${i}-label`, "aria-describedby": `${id}-${i}-description` };
}
</script>

{#snippet text(entry: { label: string; description?: string }, i: number)}
  {#if entry.description}
    <span class="ugoe-menu__text">
      <span class="ugoe-menu__label" id={`${id}-${i}-label`}>{entry.label}</span>
      <span class="ugoe-menu__description" id={`${id}-${i}-description`}>{entry.description}</span>
    </span>
  {:else}
    <span class="ugoe-menu__label">{entry.label}</span>
  {/if}
{/snippet}

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
      {...described(entry, i)}
    >
      {#snippet children({ checked })}
        <span class="ugoe-menu__check">{#if checked}<Icon name="check" />{/if}</span>
        {@render text(entry, i)}
        {@render shortcutOf(entry.shortcut)}
      {/snippet}
    </DropdownMenu.CheckboxItem>
  {:else if entry.kind === "submenu"}
    <DropdownMenu.Sub>
      <DropdownMenu.SubTrigger
        class="ugoe-menu__item"
        disabled={entry.disabled}
        textValue={entry.label}
        {...described(entry, i)}
      >
        <span class="ugoe-menu__check"></span>
        {@render text(entry, i)}
        <Icon name="chevronRight" />
      </DropdownMenu.SubTrigger>
      <DropdownMenu.SubContent
        class="ugoe-menu"
        aria-label={entry.label}
        sideOffset={2}
        {@attach markScrolling}
      >
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
      {...described(entry, i)}
    >
      <span class="ugoe-menu__check"></span>
      {@render text(entry, i)}
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
      /* 説明の行を折り返して、窓の幅 (狭い幅のビューア・スタジオ) に収める */
      max-width: min(28rem, var(--bits-dropdown-menu-content-available-width, calc(100vw - 16px)));
      max-height: var(--bits-dropdown-menu-content-available-height, 80vh);
      padding: var(--ugoe-space-1);
      /* 送るときの行末は、下の [data-scrolling] がスクロールバーの分空ける */
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

    /* 送るときだけ、行末 (ショートカット・チェック・下の段の印) をスクロールバーの分空ける (markScrolling。T0976) */
    .ugoe-menu[data-scrolling] {
      padding-inline-end: var(--ugoe-scrollbar-gap);
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

    /* 説明の行のある項目: 名前の下に控えめな文字で出す。説明は折り返す */
    .ugoe-menu__text {
      display: flex;
      flex: 1;
      flex-direction: column;
      gap: 2px;
      min-width: 0;
      padding: var(--ugoe-space-1) 0;
    }

    .ugoe-menu__text > .ugoe-menu__label {
      white-space: normal;
      overflow-wrap: anywhere;
    }

    .ugoe-menu__description {
      color: var(--ugoe-color-text-muted);
      font-size: var(--ugoe-font-size-sm);
      line-height: var(--ugoe-line-height);
      overflow-wrap: anywhere;
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
