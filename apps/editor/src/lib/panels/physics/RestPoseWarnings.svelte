<!--
  静止姿勢で衝突体に当たっている鎖の警告 (docs/editor.md#物理の警告)。「Verify」と「Physics」のパネルが使う。
  モデルも物理も変えない。ドキュメントが変わるたびに調べ直すので、テンプレートの適用や衝突体の移動の直後にも出る。
  行を押すと `onSelect` でリグと衝突体を知らせる (選ぶのは呼び出し側)。
-->
<script lang="ts">
import type { EditorDocument } from "../../document.svelte";
import { errorText } from "../../errors";
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
  <p class="warn">
    {found.length === 1 ? "A rig hits a collider" : `${found.length} rig–collider pairs hit`} in the
    rest pose. The hair starts bent along the collider instead of hanging freely; move the collider
    out of the hanging hair unless this is intended.
  </p>
  <ul>
    {#each found as c (restPoseKey(c))}
      <li>
        <button
          class:current={selected?.rig === c.rig && selected?.collider === c.collider}
          onclick={() => onSelect(c.rig, c.collider)}
          title="Select the rig and the collider in the Physics panel"
        >
          <span>{restPoseTitle(c, doc.physics)}</span>
          <span class="muted small">{formatSegments(c.particles)}</span>
        </button>
      </li>
    {/each}
  </ul>
{:else if showEmpty && doc.physics}
  <p class="ok">No rig hits a collider in the rest pose.</p>
{:else if showEmpty}
  <p class="muted">No physics settings.</p>
{/if}

<style>
p {
  margin: 4px 0;
}

.warn,
.error {
  color: #dc2626;
}

.ok {
  color: #16a34a;
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
  margin: 2px 0;
  text-align: left;
  gap: 1px;
  overflow-wrap: anywhere;
}

button.current {
  border-color: var(--accent);
}

.small {
  font-size: 11px;
}
</style>
