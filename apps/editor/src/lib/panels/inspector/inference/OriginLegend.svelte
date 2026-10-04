<!-- キーの格子のセルの由来の凡例 (記号と色は inference.ts の ORIGIN_MARKS)。 -->
<script lang="ts">
import { Tooltip } from "@ugoe/ui";
import { LEGEND_ORIGINS, originStyle } from "./inference";
</script>

<ul class="legend">
  {#each LEGEND_ORIGINS as origin (origin)}
    {@const style = originStyle(origin)}
    <li>
      <Tooltip text={style.description}>
        {#snippet children(attrs)}
          <!-- 説明をキーボードでも出せるように、項目にフォーカスを受けさせる -->
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <span class="item" tabindex="0" {...attrs}>
            <span class="swatch" class:derived={origin !== "authored"} style="--origin: {style.color}"
              >{style.symbol}</span
            >
            {style.name}
          </span>
        {/snippet}
      </Tooltip>
    </li>
  {/each}
</ul>

<style>
.legend {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 10px;
  margin: 8px 0 0;
  padding: 0;
  list-style: none;
  font-size: 11px;
  color: var(--ugoe-color-text-muted);
}

.item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.swatch {
  display: inline-grid;
  place-items: center;
  width: 16px;
  height: 16px;
  border: 1px solid var(--ugoe-color-border);
  border-radius: 3px;
  /* 記号は文字なので、色を本文の色へ寄せて 4.5:1 にする (枠と地は由来の色のまま。T0907) */
  color: color-mix(in srgb, var(--origin) 60%, var(--ugoe-color-text));
}

.swatch.derived {
  border: 1px dashed var(--origin);
  background: color-mix(in srgb, var(--origin) 14%, transparent);
}
</style>
