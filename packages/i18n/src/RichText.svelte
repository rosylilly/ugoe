<script lang="ts">
// 訳文の中の印 (`<name>中身</name>`) を、同じ名前のスニペットで描く (rich.ts)。印の名前に合うスニペットが無ければ、
// 中身をただの文として出す。例:
//   <RichText text={t("notice.manual")}>{#snippet link(text)}<a href={url}>{text}</a>{/snippet}</RichText>
import type { Snippet } from "svelte";
import { parseRich } from "./rich";

interface Props {
  /** 訳文 (`t()` の結果)。 */
  text: string;
  /** 印の名前 → 中身の文を受け取るスニペット。 */
  [name: string]: Snippet<[string]> | string;
}

let { text, ...snippets }: Props = $props();
const parts = $derived(parseRich(text));
</script>

{#each parts as part, i (i)}
  {#if part.kind === "tag" && typeof snippets[part.name] === "function"}
    {@render (snippets[part.name] as Snippet<[string]>)(part.text)}
  {:else}
    {part.text}
  {/if}
{/each}
