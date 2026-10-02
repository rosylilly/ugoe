<script lang="ts">
// 着せ替えのパネル (T0706)。スロットを持つモデルか、モジュールを読み込んだときに App.svelte が出す。
// スロットごとに、読み込んだモジュールを並べて付ける・外す・差し替える・一覧から除く。
// 付け外しは Player (WebPuppet の着せ替えの口) が行い、失敗の理由は `player.dressUpError`。
// 本体のアバタープロファイルがアプリに公開していないスロット (作者用) は出さない (T0715。`player.publishedSlots`)。
// モデルに前に付けていた組があれば、戻すか尋ねる (T0711。`player.restoreOffer`)。覚えた組は「覚えた組を忘れる」で忘れる (T0709)。
import { groupBySlot, type ModuleInfo, type Player, visibleSlots } from "@ugoe/player";
import { viewerHost } from "./host";

let { player, hasModel }: { player: Player; hasModel: boolean } = $props();

let open = $state(true);

let slots = $derived(visibleSlots(player.slots, player.publishedSlots));
let groups = $derived(groupBySlot(player.slots, player.modules, player.publishedSlots));
let attachedCount = $derived(
  player.modules.filter((m) => m.attached && slots.some((s) => s.id === m.slot)).length,
);
let status = $derived(
  slots.length === 0 ? "スロットなし" : `${attachedCount} / ${slots.length} スロット`,
);

async function openModules() {
  try {
    const files = await viewerHost().pickModuleFiles();
    for (const file of files) player.loadModule(file);
  } catch (e) {
    player.dressUpError = e instanceof Error ? e.message : String(e);
  }
}

let offer = $derived(player.restoreOffer);
let offerText = $derived(
  offer === null
    ? ""
    : offer.attached.length > 0
      ? offer.attached.join("・")
      : `付けていないモジュール ${offer.modules.length} 個`,
);

function describe(m: ModuleInfo): string {
  const lines = [`スロット: ${m.slot}`, `テクスチャ: ${m.textures} 枚`];
  const r = m.requires;
  if (r.avatar_level !== undefined)
    lines.push(`アバタープロファイル: レベル ${r.avatar_level} 以上`);
  if (r.slots?.length) lines.push(`必要なスロット: ${r.slots.join(", ")}`);
  if (r.parameters?.length) lines.push(`必要なパラメータ: ${r.parameters.join(", ")}`);
  return lines.join("\n");
}
</script>

<details class="panel dress-up" bind:open>
  <summary>
    <h2>着せ替え</h2>
    <span class="status">{status}</span>
  </summary>

  <div class="actions">
    <button type="button" onclick={openModules} title=".ugoemod を選んで読み込み、付ける (ウィンドウへのドロップでも開ける)"
      >モジュールを開く…</button
    >
    <label title="モデルを開いたとき、前にそのモデルに付けていた組を尋ねずに戻す">
      <input
        type="checkbox"
        checked={player.autoRestore}
        onchange={(e) => player.setAutoRestore(e.currentTarget.checked)}
      />
      前の組を自動で戻す
    </label>
  </div>

  {#if player.dressUpRemembered}
    <div class="memory">
      <span title="付けているモジュールの組はモデルごとに覚え、次に開いたときに戻せる">このモデル用に覚えた組があります</span>
      <button
        type="button"
        title="このモデルに覚えた組を忘れる (付けているモジュールはそのまま。次に着せ替えを操作すると、そのときの組をまた覚える)"
        onclick={() => player.forgetDressUp()}>覚えた組を忘れる</button
      >
    </div>
  {/if}

  {#if offer !== null}
    <div class="offer" role="status">
      <p>前にこのモデルに付けていた組があります: <strong>{offerText}</strong></p>
      <div class="offer-actions">
        <button type="button" onclick={() => void player.restoreDressUp()}>戻す</button>
        <button type="button" onclick={() => player.dismissRestore()}>今回は戻さない</button>
      </div>
    </div>
  {/if}

  {#if player.dressUpError}
    <p class="error" role="alert">{player.dressUpError}</p>
  {/if}
  {#if !hasModel}
    <p class="hint">モデルを開くと付けられます</p>
  {:else if player.slots.length === 0}
    <p class="hint">このモデルにはスロットがありません (着せ替えに対応していません)</p>
  {:else if slots.length === 0}
    <p class="hint">このモデルは着せ替えのスロットを公開していません</p>
  {/if}

  {#each groups as g (g.id)}
    <section class:missing={g.missing || g.hidden}>
      <h3>
        {g.id}
        {#if g.missing}
          <span class="badge" title="このモデルに無いスロット。このモジュールは付けられない">モデルに無い</span>
        {:else if g.hidden}
          <span
            class="badge"
            title="モデルの作者用のスロット (アバタープロファイルでアプリに公開していない)。このモジュールは付けられない"
            >公開されていない</span
          >
        {:else}
          <span class="current">{g.attached?.name ?? "なし"}</span>
        {/if}
      </h3>
      {#if g.modules.length === 0}
        <p class="empty">モジュールを読み込んでいません</p>
      {:else}
        <ul>
          {#each g.modules as m (m.name)}
            <li class:attached={m.attached}>
              <span class="name" title={describe(m)}>{m.name}</span>
              {#if m.warnings.length > 0}
                <span class="warning" title={m.warnings.join("\n")}>警告 {m.warnings.length}</span>
              {/if}
              {#if m.attached}
                <button type="button" onclick={() => player.detachModule(m.name)}>外す</button>
              {:else}
                <button
                  type="button"
                  disabled={g.missing || g.hidden || !hasModel}
                  title={g.attached ? `「${g.attached.name}」と差し替える` : "付ける"}
                  onclick={() => player.attachModule(m.name)}>{g.attached ? "差し替える" : "付ける"}</button
                >
              {/if}
              <button
                type="button"
                class="remove"
                title="一覧から除く (付けていれば外す)"
                aria-label="{m.name} を一覧から除く"
                onclick={() => player.unloadModule(m.name)}>×</button
              >
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/each}
</details>

<style>
  .dress-up {
    flex-shrink: 0;
    max-height: 40%;
    overflow-y: auto;
    font-size: 0.8rem;
  }
  .dress-up > summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
  }
  h2 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    display: inline;
  }
  h3 {
    margin: 0;
    font-size: 0.8rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .status,
  .current,
  .hint,
  .empty {
    color: var(--muted);
    font-weight: normal;
  }
  .hint,
  .empty {
    margin: 0.3rem 0 0;
    font-size: 0.75rem;
  }
  .actions {
    margin-top: 0.4rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
  }
  .actions label {
    display: flex;
    align-items: center;
    gap: 0.2rem;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .memory {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
    margin-top: 0.4rem;
    color: var(--muted);
    font-size: 0.75rem;
  }
  .memory button {
    font-size: 0.75rem;
    padding: 0.1rem 0.5rem;
  }
  .offer {
    margin-top: 0.4rem;
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--accent);
    border-radius: 4px;
  }
  .offer p {
    margin: 0;
  }
  .offer-actions {
    display: flex;
    gap: 0.3rem;
    margin-top: 0.3rem;
  }
  .offer-actions button {
    font-size: 0.75rem;
    padding: 0.1rem 0.5rem;
  }
  .actions button {
    font-size: 0.75rem;
    padding: 0.15rem 0.5rem;
  }
  .error {
    margin: 0.4rem 0 0;
    color: var(--danger);
    white-space: pre-line;
  }
  section {
    margin-top: 0.6rem;
  }
  section.missing h3 {
    color: var(--muted);
  }
  .badge {
    font-size: 0.7rem;
    font-weight: normal;
    color: var(--warning);
    border: 1px solid currentColor;
    border-radius: 3px;
    padding: 0 0.3rem;
  }
  ul {
    list-style: none;
    margin: 0.2rem 0 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.1rem 0;
  }
  li .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  li.attached .name {
    font-weight: 600;
    color: var(--accent);
  }
  li button {
    font-size: 0.75rem;
    padding: 0.1rem 0.45rem;
  }
  .remove {
    border: none;
    background: none;
    color: var(--muted);
  }
  .warning {
    color: var(--warning);
    font-size: 0.7rem;
  }
</style>
