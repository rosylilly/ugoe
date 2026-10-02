// 画面に出す名前。
import { t } from "./i18n";
import type { SceneSettings } from "./profile";

/** シーンの表示名。名前が空なら、並びの番号から「シーン 1」などにする (`index` は 0 から)。 */
export function sceneLabel(scene: Pick<SceneSettings, "name">, index: number): string {
  return scene.name.trim() === "" ? t("scenes.defaultName", { n: index + 1 }) : scene.name;
}
