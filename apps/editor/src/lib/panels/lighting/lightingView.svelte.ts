// ライティングのパネル・ビューポートの取っ手・プレビューのパネルが共有する、プレビューの光源の状態
// (docs/editor.md#ライティングの画面)。光源はモデルに入らないので、コマンドではなく project.json の
// editor.lighting に残す (ipc.setPreviewLighting。Undo の対象ではない)。
import { errorText } from "../../errors";
import { ipc, type PreviewLight, type PreviewLighting } from "../../ipc";
import { DEFAULT_PREVIEW_LIGHTING, lightingJson } from "./lighting";

export class LightingView {
  /** ライティングのパネルを開いているか (開いている間、ビューポートに光源の取っ手を重ねる)。 */
  active = $state(false);
  /** 表示中の光源の設定。ドラッグ中はここだけを変え、離したときにプロジェクトに残す。 */
  settings = $state.raw<PreviewLighting>(DEFAULT_PREVIEW_LIGHTING);
  /** 選んでいる光源の番号。 */
  selected = $state<number | null>(0);
  error = $state<string | null>(null);
  /** WebPuppet.set_lighting_json に渡す JSON (null なら渡さない)。 */
  readonly json = $derived(lightingJson(this.settings));

  /** 設定をプロジェクトに残した後 (未保存の印が変わる) に呼ぶ。 */
  onsaved: (() => void) | null = null;

  /** 開いているプロジェクトの設定を読み直す (起動・新規・開くの後)。 */
  async load(): Promise<void> {
    try {
      this.settings = await ipc.getPreviewLighting();
      this.error = null;
    } catch (e) {
      this.error = errorText(e);
    }
    if (this.selected !== null && this.selected >= this.settings.lights.length) {
      this.selected = this.settings.lights.length > 0 ? 0 : null;
    }
  }

  /** 表示だけを変える (ドラッグ中・スライダーを動かしている間)。 */
  preview(settings: PreviewLighting): void {
    this.settings = settings;
  }

  /** 変えてプロジェクトに残す。範囲の外なら元に戻して `error` に理由を出す。 */
  async commit(settings: PreviewLighting = this.settings): Promise<void> {
    this.settings = settings;
    try {
      await ipc.setPreviewLighting(settings);
      this.error = null;
      this.onsaved?.();
    } catch (e) {
      this.error = errorText(e);
      await this.load();
    }
  }

  /** `index` の光源を置き換えた設定。 */
  withLight(index: number, light: PreviewLight): PreviewLighting {
    return {
      ...this.settings,
      lights: this.settings.lights.map((l, i) => (i === index ? light : l)),
    };
  }
}
