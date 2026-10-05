<!--
  受け手 (出力ウィンドウ・仮想カメラ・ブラウザソースのページ) が今のシーンで読めなかったこと
  (docs/studio.md#受け手が読めなかったとき)。受け手ごとに 1 つの知らせにし、読めなかったものをすべて 1 行ずつ並べる (T0840)。
  閉じると、その受け手の知らせをすべて消す (消すのは呼び手)。
-->
<script lang="ts">
import { Notice } from "@ugoe/ui";
import type { ReceiverFailure, StudioScene } from "./lib/host";
import { describeLoadFailure, groupLoadFailures } from "./lib/receivers";

interface Props {
  /** `loadFailures` の通知の `failures` (受け手のラベルが同じものは並んでいる)。 */
  failures: readonly ReceiverFailure[];
  /** 今のシーン (読めなかったものの名前を引く)。 */
  scene: StudioScene | null;
  /** 受け手 `label` の知らせを閉じた。 */
  ondismiss: (label: string) => void;
}

let { failures, scene, ondismiss }: Props = $props();

const groups = $derived(groupLoadFailures(failures));
</script>

{#each groups as group (group.label)}
  <Notice tone="danger" ondismiss={() => ondismiss(group.label)}>
    {#each group.failures as failure (`${failure.asset}:${failure.index}`)}
      <p>{describeLoadFailure(failure, scene)}</p>
    {/each}
  </Notice>
{/each}
