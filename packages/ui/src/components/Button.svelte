<!--
  Button: 操作を 1 つ実行するボタン (docs/ui.md#部品の一覧)。土台は Bits UI の Button
  (Toolbar の中では Toolbar.Button。矢印でボタンの間を移れる)。
-->
<script lang="ts">
import { Button, Toolbar } from "bits-ui";
import type { Snippet } from "svelte";
import { isInToolbar } from "../context";
import type { ButtonBaseProps } from "../types";

interface Props extends ButtonBaseProps {
  "aria-label"?: string;
  children?: Snippet;
}

let {
  variant,
  small = false,
  pressed,
  type = "button",
  class: className = "",
  children,
  ...rest
}: Props = $props();

const inToolbar = isInToolbar();
const resolved = $derived(variant ?? (inToolbar ? "ghost" : "secondary"));
const classes = $derived(
  ["ugoe-button", `ugoe-button--${resolved}`, small && "ugoe-button--small", className]
    .filter(Boolean)
    .join(" "),
);
</script>

{#if inToolbar}
  <Toolbar.Button {...rest} {type} class={classes} aria-pressed={pressed}>
    {@render children?.()}
  </Toolbar.Button>
{:else}
  <Button.Root {...rest} {type} class={classes} aria-pressed={pressed}>
    {@render children?.()}
  </Button.Root>
{/if}

<style>
  :global {
    .ugoe-button {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: var(--ugoe-space-2);
      box-sizing: border-box;
      min-width: var(--ugoe-target-min);
      min-height: max(var(--ugoe-control-height), var(--ugoe-target-min));
      padding: 0 var(--ugoe-space-3);
      border: 1px solid var(--ugoe-color-control-border);
      border-radius: var(--ugoe-radius-md);
      background: var(--ugoe-color-surface);
      color: var(--ugoe-color-text);
      font: inherit;
      font-family: var(--ugoe-font-sans);
      font-size: var(--ugoe-font-size-md);
      line-height: var(--ugoe-line-height-control);
      white-space: nowrap;
      cursor: pointer;
      transition:
        background-color var(--ugoe-duration-fast),
        border-color var(--ugoe-duration-fast);
    }

    .ugoe-button:hover:not(:disabled) {
      background: var(--ugoe-color-sunken);
    }

    .ugoe-button:focus-visible {
      outline: var(--ugoe-focus-ring);
      outline-offset: 2px;
    }

    .ugoe-button:disabled {
      cursor: default;
      opacity: 0.5;
    }

    .ugoe-button--small {
      min-height: var(--ugoe-target-min);
      padding: 0 var(--ugoe-space-2);
      font-size: var(--ugoe-font-size-sm);
    }

    .ugoe-button--primary {
      border-color: var(--ugoe-color-accent);
      background: var(--ugoe-color-accent);
      color: var(--ugoe-color-on-accent);
      font-weight: var(--ugoe-font-weight-strong);
    }

    .ugoe-button--primary:hover:not(:disabled) {
      border-color: color-mix(in srgb, var(--ugoe-color-accent) 85%, var(--ugoe-color-text));
      background: color-mix(in srgb, var(--ugoe-color-accent) 85%, var(--ugoe-color-text));
    }

    .ugoe-button--danger {
      border-color: var(--ugoe-color-danger);
      color: var(--ugoe-color-danger);
    }

    .ugoe-button--danger:hover:not(:disabled) {
      background: var(--ugoe-color-danger-soft);
    }

    .ugoe-button--ghost {
      border-color: transparent;
      background: transparent;
    }

    /* 押された状態は色だけでなく枠でも示す */
    .ugoe-button[aria-pressed="true"] {
      border-color: var(--ugoe-color-accent);
      background: var(--ugoe-color-accent-soft);
      color: var(--ugoe-color-text);
    }
  }
</style>
