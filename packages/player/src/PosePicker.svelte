<!--
  ポーズの要素を選ぶ欄 (T0948)。ビューアのポーズのパネル (Player の `poseGroups`・`poseShown`・`showPose`。T0735) と、スタジオの
  操作ウィンドウのポーズのパネル (バックエンドの `scene` の通知と `show_pose`。T0842) で共有する。グループごとに、要素が 2〜4 個なら
  SegmentedControl、5 個以上なら Select で選ぶ (docs/ui.md#部品の一覧)。見出しは「グループ n」(モジュールのグループは「<モジュール> のグループ n」)、
  要素の表示名はモジュールの接頭辞を除いたもの。どちらも props (`groupLabel`・`entryLabel`) で差し替えられる (エディタのプレビューは
  パーツの名前で出す。T1640)。
  欄は `shown` をそのまま出すので、呼び手が表示中の要素に合わせれば (ビューアはモーションによる切り替えに追従する。T0736) 欄も追従する。
  `onchange` が Promise を返す間 (送っている途中) は、届いた `shown` より選んだ要素を先に出す。
-->
<script lang="ts">
import { SegmentedControl, Select } from "@ugoe/ui";
import { t } from "./i18n";
import { type PoseGroupInfo, poseEntryLabel, poseGroupNumbers } from "./pose";

interface Props {
  /** ポーズのグループ (本体と、着せ替えで付けたモジュールのもの)。 */
  groups: readonly PoseGroupInfo[];
  /** グループごとに表示している要素 (`groups` と同じ順)。 */
  shown: readonly string[];
  disabled?: boolean;
  /**
   * グループの見出し。`n` は本体とモジュールごとに 1 から数えた番号 (`poseGroupNumbers`)。既定は「グループ n」
   * (モジュールのグループは「<モジュール> のグループ n」)。
   */
  groupLabel?: (group: PoseGroupInfo, n: number) => string;
  /** 要素の表示名。既定は `poseEntryLabel` (モジュールの接頭辞を除いたもの)。 */
  entryLabel?: (group: PoseGroupInfo, part: string) => string;
  /** 要素を選んだ (`index` は選んだグループの `groups` での位置)。Promise を返せば、終わるまで選んだ要素を先に出す。 */
  onchange: (part: string, index: number) => unknown;
}

let {
  groups,
  shown,
  disabled = false,
  groupLabel = defaultGroupLabel,
  entryLabel = poseEntryLabel,
  onchange,
}: Props = $props();

/** 送っている途中の要素 (グループの要素の並びを鍵にする。届いた `shown` より先に出す)。 */
let pending = $state<Record<string, string>>({});

const numbers = $derived(poseGroupNumbers(groups));

function defaultGroupLabel(group: PoseGroupInfo, n: number): string {
  return group.module === null
    ? t("pose.group", { n })
    : t("pose.moduleGroup", { module: group.module, n });
}

function keyOf(group: PoseGroupInfo): string {
  return group.parts.join("\n");
}

function choose(group: PoseGroupInfo, index: number, part: string) {
  const result = onchange(part, index);
  if (!(result instanceof Promise)) return;
  const key = keyOf(group);
  pending[key] = part;
  const settle = () => {
    if (pending[key] === part) delete pending[key];
  };
  result.then(settle, settle);
}
</script>

<div class="ugoe-pose-picker">
  {#each groups as group, i (`${i}:${keyOf(group)}`)}
    {@const n = numbers[i] ?? i + 1}
    {@const label = groupLabel(group, n)}
    {@const options = group.parts.map((part) => ({ value: part, label: entryLabel(group, part) }))}
    {@const value = pending[keyOf(group)] ?? shown[i] ?? ""}
    <div class="ugoe-pose-picker__group">
      {#if options.length <= 4}
        <SegmentedControl {label} {value} {options} {disabled} onchange={(part) => choose(group, i, part)} />
      {:else}
        <Select {label} {value} {options} {disabled} onchange={(part) => choose(group, i, part)} />
      {/if}
    </div>
  {/each}
</div>

<style>
  .ugoe-pose-picker {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-2);
  }

  .ugoe-pose-picker__group {
    min-width: 0;
  }
</style>
