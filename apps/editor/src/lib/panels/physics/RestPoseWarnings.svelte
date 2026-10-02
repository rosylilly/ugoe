<!--
  静止姿勢で衝突体に当たっている鎖の警告 (docs/editor.md#物理の警告)。「Verify」と「Physics」のパネルが使う。
  モデルも物理も変えない。ドキュメントが変わるたびに調べ直すので、テンプレートの適用や衝突体の移動の直後にも出る。
  行を押すと `onSelect` でリグと衝突体を知らせる (選ぶのは呼び出し側)。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import { errorText } from "../../errors";
import { t } from "../../i18n";
import { ipc, type RestPoseCollision } from "../../ipc";
import { formatSegments, restPoseKey, restPoseTitle } from "./restPose";

interface Props {
  doc: EditorDocument;
  onSelect: (rig: string, collider: string) => void;
  /** 当たっていないときの「No …」も出すか (Verify のパネル)。 */
  showEmpty?: boolean;
  /** 選択中の組 (枠で示す)。 */
  selected?: { rig: string | null; collider: string | null };
}

let { doc, onSelect, showEmpty = false, selected }: Props = $props();

let found = $state.raw<RestPoseCollision[]>([]);
let error = $state<string | null>(null);
/** 最後に送った問い合わせの番号 (古い応答を捨てる)。 */
let request = 0;

$effect(() => {
  // ドキュメントの版と物理の設定が変わるたびに調べ直す。
  void doc.revision;
  if (!doc.physics) {
    found = [];
    error = null;
    return;
  }
  const id = ++request;
  ipc
    .checkPhysics()
    .then((result) => {
      if (id !== request) return;
      found = result;
      error = null;
    })
    .catch((e) => {
      if (id !== request) return;
      found = [];
      error = errorText(e);
    });
});
</script>

{#if error}
  <p class="error">{error}</p>
{:else if found.length > 0}
  <p class="warn">{t("physics.restPose.hits", { count: found.length })}</p>
  <ul>
    {#each found as c (restPoseKey(c))}
      <li>
        <button
          class:current={selected?.rig === c.rig && selected?.collider === c.collider}
          onclick={() => onSelect(c.rig, c.collider)}
          title={t("physics.restPose.select")}
        >
          <span>{restPoseTitle(c, doc.physics)}</span>
          <span class="muted small">{formatSegments(c.particles)}</span>
        </button>
      </li>
    {/each}
  </ul>
{:else if showEmpty && doc.physics}
  <p class="ok">{t("physics.restPose.none")}</p>
{:else if showEmpty}
  <p class="ed-hint">{t("physics.restPose.noSettings")}</p>
{/if}

<style>
p {
  margin: var(--ugoe-space-2) 0;
}

.warn {
  color: var(--ugoe-color-warning);
}

.error {
  color: var(--ugoe-color-danger);
}

.ok {
  color: var(--ugoe-color-success);
}

ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

button {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  width: 100%;
  margin: var(--ugoe-space-1) 0;
  padding: var(--ugoe-space-1) var(--ugoe-space-2);
  border: 1px solid var(--ugoe-color-control-border);
  border-radius: var(--ugoe-radius-md);
  background: var(--ugoe-color-surface);
  color: var(--ugoe-color-text);
  font: inherit;
  text-align: left;
  gap: 1px;
  overflow-wrap: anywhere;
}

button.current {
  border-color: var(--ugoe-color-accent);
}

button:focus-visible {
  outline: var(--ugoe-focus-ring);
  outline-offset: 1px;
}

.small {
  font-size: var(--ugoe-font-size-xs);
}
</style>
