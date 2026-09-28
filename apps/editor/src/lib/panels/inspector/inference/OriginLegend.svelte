<!-- キーの格子のセルの由来の凡例 (記号と色は inference.ts の ORIGIN_STYLES)。 -->
<script lang="ts">
import { LEGEND_ORIGINS, originStyle } from "./inference";
</script>

<ul class="legend">
  {#each LEGEND_ORIGINS as origin (origin)}
    {@const style = originStyle(origin)}
    <li title={style.description}>
      <span class="swatch" class:derived={origin !== "authored"} style="--origin: {style.color}">{style.symbol}</span>
      {style.name}
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
  color: var(--muted);
}

li {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.swatch {
  display: inline-grid;
  place-items: center;
  width: 16px;
  height: 16px;
  border: 1px solid var(--border);
  border-radius: 3px;
  color: var(--origin);
}

.swatch.derived {
  border: 1px dashed var(--origin);
  background: color-mix(in srgb, var(--origin) 14%, transparent);
}
</style>
