<script lang="ts">
// 光源の向きや位置をドラッグで選ぶ小さな面。`x`・`y` は -1..1 (右・下が正)。
// `round` なら円の中 (平行光源の向き。中心が真正面)、そうでなければ四角の中 (点光源の位置)。
// ドラッグの途中は `onchange` の `preview` を真にし、離したときに `oncommit` を呼ぶ (途中で毎回覚えないため)。
let {
  x,
  y,
  round = false,
  disabled = false,
  label,
  onchange,
  oncommit,
}: {
  x: number;
  y: number;
  round?: boolean;
  disabled?: boolean;
  label: string;
  onchange: (x: number, y: number, preview: boolean) => void;
  oncommit?: () => void;
} = $props();

let pad = $state<HTMLDivElement | null>(null);

function pick(e: PointerEvent) {
  if (pad === null) return;
  const rect = pad.getBoundingClientRect();
  let px = ((e.clientX - rect.left) / rect.width) * 2 - 1;
  let py = ((e.clientY - rect.top) / rect.height) * 2 - 1;
  if (round) {
    const r = Math.hypot(px, py);
    if (r > 1) {
      px /= r;
      py /= r;
    }
  } else {
    px = Math.min(1, Math.max(-1, px));
    py = Math.min(1, Math.max(-1, py));
  }
  onchange(px, py, true);
}

function onPointerDown(e: PointerEvent) {
  if (disabled || e.button !== 0) return;
  pad?.setPointerCapture(e.pointerId);
  pick(e);
}

function onPointerMove(e: PointerEvent) {
  if (pad?.hasPointerCapture(e.pointerId)) pick(e);
}

function onPointerUp(e: PointerEvent) {
  if (pad?.hasPointerCapture(e.pointerId)) oncommit?.();
}

function onKeyDown(e: KeyboardEvent) {
  const step = e.shiftKey ? 0.2 : 0.05;
  const moves: Record<string, [number, number]> = {
    ArrowLeft: [-step, 0],
    ArrowRight: [step, 0],
    ArrowUp: [0, -step],
    ArrowDown: [0, step],
  };
  const move = moves[e.key];
  if (disabled || move === undefined) return;
  e.preventDefault();
  let [nx, ny] = [x + move[0], y + move[1]];
  const r = Math.hypot(nx, ny);
  if (round && r > 1) [nx, ny] = [nx / r, ny / r];
  onchange(Math.min(1, Math.max(-1, nx)), Math.min(1, Math.max(-1, ny)), false);
}
</script>

<div
  bind:this={pad}
  class="pad"
  class:round
  class:disabled
  role="slider"
  tabindex={disabled ? -1 : 0}
  aria-label={label}
  aria-valuetext="{Math.round(x * 100)}, {Math.round(y * 100)}"
  aria-valuenow={x}
  aria-disabled={disabled}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
  onkeydown={onKeyDown}
>
  <span class="dot" style:left="{(x + 1) * 50}%" style:top="{(y + 1) * 50}%"></span>
</div>

<style>
  .pad {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    background:
      linear-gradient(var(--ugoe-color-border), var(--ugoe-color-border)) center / 1px 100% no-repeat,
      linear-gradient(var(--ugoe-color-border), var(--ugoe-color-border)) center / 100% 1px no-repeat,
      var(--ugoe-color-sunken);
    border: 1px solid var(--ugoe-color-control-border);
    border-radius: var(--ugoe-radius-md);
    touch-action: none;
    cursor: crosshair;
  }
  .pad.round {
    border-radius: 50%;
  }
  .pad.disabled {
    opacity: var(--ugoe-opacity-disabled);
    cursor: default;
  }
  .pad:focus-visible {
    outline: var(--ugoe-focus-ring);
    outline-offset: 2px;
  }
  .dot {
    position: absolute;
    width: 12px;
    height: 12px;
    margin: -6px 0 0 -6px;
    border-radius: var(--ugoe-radius-full);
    background: var(--ugoe-color-surface);
    border: 2px solid var(--ugoe-color-accent);
    pointer-events: none;
  }
</style>
