<!--
  出力ウィンドウ (docs/studio.md#出力ウィンドウ、T0804): 表示・最前面・大きさのプリセットと、出力ウィンドウの様子
  (いまの大きさ・枠 (なし。ドラッグで動かす、T0813)・背景 (配信しているシーンの背景。透過など。透過が画面で効かなければ注意と代わりの案、T0846))。出力ウィンドウには何も重ねないので、
  様子は操作ウィンドウのここと「いまの状態」の印で示す (T0818)。配信ソフトへの取り込み方の案内も置く。
-->
<script lang="ts">
import { Notice, Select, Toggle } from "@ugoe/ui";
import type { OutputState } from "./lib/host";
import { t } from "./lib/i18n";
import { backgroundLabel } from "./lib/labels";
import type { Background } from "./lib/profile";
import { OUTPUT_SIZE_PRESETS, type OutputSizePreset, presetOf, presetSize } from "./lib/settings";
import { transparencyLost } from "./lib/status";

interface Props {
  /** 出力ウィンドウの様子 (まだ分からなければ `null`。部品を無効にする)。 */
  output: OutputState | null;
  /** プロファイルの大きさのプリセット。 */
  size: OutputSizePreset;
  /** 配信しているシーンの背景 (配信していなければ `null`)。 */
  background: Background | null;
  onvisible: (visible: boolean) => void;
  ontop: (onTop: boolean) => void;
  onsize: (preset: OutputSizePreset) => void;
}

let { output, size, background, onvisible, ontop, onsize }: Props = $props();

const sizeOptions = $derived(
  OUTPUT_SIZE_PRESETS.map((preset) => {
    const value = presetSize(preset);
    return {
      value: preset,
      label:
        value === null
          ? t("output.sizeFree")
          : t(value[1] > value[0] ? "output.sizePortrait" : "output.sizeValue", {
              width: value[0],
              height: value[1],
            }),
    };
  }),
);
/** 選んだ大きさより OS が小さくした。 */
const scaled = $derived(
  output !== null && size !== "free" && presetOf(output.width, output.height) !== size,
);
/** 透過のシーンなのに、透過が画面で効かない (T0846)。 */
const opaque = $derived(transparencyLost(output, background));
</script>

<div class="output-panel">
  <Toggle
    label={t("output.show")}
    checked={output?.visible ?? false}
    disabled={output === null}
    onchange={onvisible}
  />
  <Toggle
    label={t("output.onTop")}
    checked={output?.onTop ?? false}
    disabled={output === null}
    onchange={ontop}
  />
  <Select
    label={t("output.size")}
    value={size}
    options={sizeOptions}
    disabled={output === null}
    onchange={onsize}
  />
  {#if scaled}
    <Notice tone="warning">{t("output.scaled")}</Notice>
  {/if}
  <dl class="output-panel__facts">
    {#if output !== null}
      <dt>{t("output.currentLabel")}</dt>
      <dd>{t("output.currentValue", { width: output.width, height: output.height })}</dd>
    {/if}
    <dt>{t("output.frame")}</dt>
    <dd>{t("output.frameNone")}</dd>
    <dt>{t("output.background")}</dt>
    <dd>
      {background === null ? t("output.backgroundNoScene") : backgroundLabel(background)}
      <span class="output-panel__hint">{t("output.backgroundHint")}</span>
      {#if opaque}
        <Notice tone="warning" title={t("output.opaqueTitle")}>{t("output.opaque")}</Notice>
      {/if}
    </dd>
  </dl>
  <p class="output-panel__hint">{t("output.capture")}</p>
</div>

<style>
  .output-panel {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  .output-panel__facts {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    align-items: baseline;
    gap: var(--ugoe-space-1) var(--ugoe-space-3);
    margin: 0;
  }

  .output-panel__facts dt {
    color: var(--ugoe-color-text-muted);
  }

  .output-panel__facts dd {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-1);
    margin: 0;
    overflow-wrap: anywhere;
  }

  .output-panel__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
