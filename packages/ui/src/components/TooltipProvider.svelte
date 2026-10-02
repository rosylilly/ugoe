<!--
  TooltipProvider: 中の Tooltip の出るまでの時間をそろえる (docs/ui.md#部品の一覧)。土台は Bits UI の Tooltip.Provider。
  アプリの根元に 1 つ置くと、ツールバーのボタンの間を動いたとき、2 つ目からは待たずに出る。
  置かなくても Tooltip は動く (Tooltip がそれぞれ自分の Provider を持つ)。
-->
<script lang="ts">
import { Tooltip } from "bits-ui";
import type { Snippet } from "svelte";
import { setInTooltipProvider } from "../context";

interface Props {
  /** 出るまでの時間 (ミリ秒)。既定は 500。 */
  delay?: number;
  children: Snippet;
}

let { delay = 500, children }: Props = $props();

setInTooltipProvider();
</script>

<Tooltip.Provider delayDuration={delay}>
  {@render children()}
</Tooltip.Provider>
