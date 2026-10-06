// 画面に出す名前。
import { t } from "./i18n";
import { type Background, fileName, type SceneSettings } from "./profile";

/** シーンの表示名。名前が空なら、並びの番号から「シーン 1」などにする (`index` は 0 から)。 */
export function sceneLabel(scene: Pick<SceneSettings, "name">, index: number): string {
  return scene.name.trim() === "" ? t("scenes.defaultName", { n: index + 1 }) : scene.name;
}

/** 背景の種類と中身 (「単色 (#202020)」「画像 (stage.png)」。透過は種類だけ)。出力ウィンドウの様子に出す (T0818)。 */
export function backgroundLabel(background: Background): string {
  switch (background.kind) {
    case "transparent":
      return t("editor.bgTransparent");
    case "color":
      return t("output.backgroundValue", { kind: t("editor.bgColor"), detail: background.color });
    case "chroma":
      return t("output.backgroundValue", { kind: t("editor.bgChroma"), detail: background.color });
    case "image":
      return t("output.backgroundValue", {
        kind: t("editor.bgImage"),
        detail: fileName(background.path),
      });
  }
}
