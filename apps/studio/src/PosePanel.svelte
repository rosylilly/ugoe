<!--
  ポーズ (docs/studio.md#ポーズの要素、T0842): 配信しているシーンで、ポーズのグループごとに表示する要素を選ぶ。
  選ぶとバックエンドの `show_pose` が出しているシーンの設定 (`pose`) に覚えて出し直し、受け手がフェードで切り替える
  (操作ウィンドウはモデルを描かない)。グループは指揮役のモデル (と付けているモジュール) のもので、表示している要素は
  `scene` の通知の `pose` で届く。送っている間は選んだ要素を先に出す。要素が 5 個以上なら Select (ビューアのポーズのパネルと同じ。
  docs/ui.md#部品の一覧)。
-->
<script lang="ts">
import { type PoseGroupInfo, poseEntryLabel, poseGroupNumbers } from "@ugoe/player";
import { SegmentedControl, Select } from "@ugoe/ui";
import { t } from "./lib/i18n";

interface Props {
  /** 配信しているモデルのポーズのグループ。 */
  groups: PoseGroupInfo[];
  /** グループごとに表示している要素 (`groups` と同じ順)。 */
  shown: string[];
  disabled?: boolean;
  /** 要素を選んだ (送り終えたら返る)。 */
  onchange: (part: string) => Promise<unknown>;
}

let { groups, shown, disabled = false, onchange }: Props = $props();

/** 送っている途中の要素 (グループの要素の並びを鍵にする。届いた通知より先に出す)。 */
let pending = $state<Record<string, string>>({});

const numbers = $derived(poseGroupNumbers(groups));

function keyOf(group: PoseGroupInfo): string {
  return group.parts.join("\n");
}

async function choose(group: PoseGroupInfo, part: string) {
  const key = keyOf(group);
  pending[key] = part;
  try {
    await onchange(part);
  } finally {
    if (pending[key] === part) delete pending[key];
  }
}
</script>

<div class="poses">
  {#if groups.length === 0}
    <p class="poses__hint">{t("poses.empty")}</p>
  {:else}
    <p class="poses__hint">{t("poses.hint")}</p>
    {#each groups as group, i (keyOf(group))}
      {@const n = numbers[i] ?? i + 1}
      {@const label =
        group.module === null ? t("poses.group", { n }) : t("poses.moduleGroup", { module: group.module, n })}
      {@const options = group.parts.map((part) => ({ value: part, label: poseEntryLabel(group, part) }))}
      {@const value = pending[keyOf(group)] ?? shown[i] ?? ""}
      <div class="poses__group">
        {#if options.length <= 4}
          <SegmentedControl {label} {value} {options} {disabled} onchange={(part) => void choose(group, part)} />
        {:else}
          <Select {label} {value} {options} {disabled} onchange={(part) => void choose(group, part)} />
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .poses {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  /* 要素の名前が長くても横にはみ出さず、折り返す。 */
  .poses :global(.ugoe-segmented),
  .poses :global(.ugoe-segmented__group) {
    flex-wrap: wrap;
  }

  .poses__group {
    min-width: 0;
  }

  .poses__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
