<!--
  ポーズ (docs/studio.md#ポーズの要素、T0842): 配信しているシーンで、ポーズのグループごとに表示する要素を選ぶ。
  選ぶとバックエンドの `show_pose` が出しているシーンの設定 (`pose`) に覚えて出し直し、受け手がフェードで切り替える
  (操作ウィンドウはモデルを描かない)。グループは指揮役のモデル (と付けているモジュール) のもので、表示している要素は
  `scene` の通知の `pose` で届く。欄は @ugoe/player の PosePicker (ビューアと共有。T0948)
  で、送っている間は選んだ要素を先に出す。
-->
<script lang="ts">
import { type PoseGroupInfo, PosePicker } from "@ugoe/player";
import { t } from "./lib/i18n";

interface Props {
  /** 配信しているモデルのポーズのグループ。 */
  groups: PoseGroupInfo[];
  /** グループごとに表示している要素 (`groups` と同じ順)。 */
  shown: string[];
  disabled?: boolean;
  /** 要素を選んだ (送り終えたら返る。それまで欄は選んだ要素を先に出す)。 */
  onchange: (part: string) => Promise<unknown>;
}

let { groups, shown, disabled = false, onchange }: Props = $props();
</script>

<div class="poses">
  {#if groups.length === 0}
    <p class="poses__hint">{t("poses.empty")}</p>
  {:else}
    <p class="poses__hint">{t("poses.hint")}</p>
    <PosePicker {groups} {shown} {disabled} {onchange} />
  {/if}
</div>

<style>
  .poses {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  .poses__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
