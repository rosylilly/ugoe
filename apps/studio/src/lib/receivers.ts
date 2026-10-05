// 受け手 (出力ウィンドウ・仮想カメラ・ブラウザソースのページ) の表示名と、受け手が読めなかったこと
// (docs/studio.md#受け手が読めなかったとき) の文。操作ウィンドウに出す (出力には何も重ねない)。
import type { ReceiverFailure, StudioScene } from "./host";
import { t } from "./i18n";

/** 受け手のラベル (lib.rs の `RendererLoad.label`・load_failure.rs の `ReceiverFailure.label`) の表示名。 */
export function receiverName(label: string): string {
  if (label === "output") return t("load.outputWindow");
  if (label === "virtualCamera") return t("load.virtualCamera");
  // ブラウザソースのページ (つないだ順に `browserSource`・`browserSource:2`…。api/source.rs の `LABEL`)。
  if (label === "browserSource") return t("load.browserSource");
  const page = /^browserSource:(\d+)$/.exec(label);
  if (page !== null) return t("load.browserSourceN", { n: Number(page[1]) });
  return label;
}

/** 読めなかったものの名前。名前は `scene` (今のシーン) から引く。 */
function assetName(failure: ReceiverFailure, scene: StudioScene | null): string {
  const current = scene !== null && scene.revision === failure.revision ? scene : null;
  switch (failure.asset) {
    case "model": {
      const name = current?.modelName;
      return name === undefined
        ? t("receiverFailure.modelUnnamed")
        : t("receiverFailure.model", { name });
    }
    case "module": {
      const name = failure.index === null ? undefined : current?.modules[failure.index];
      return name === undefined
        ? t("receiverFailure.moduleUnnamed")
        : t("receiverFailure.module", { name });
    }
    case "background":
      return t("receiverFailure.background");
  }
}

/** 受け手が読めなかったことを文にする。 */
export function describeLoadFailure(failure: ReceiverFailure, scene: StudioScene | null): string {
  return t(`receiverFailure.${failure.code}`, {
    receiver: receiverName(failure.label),
    asset: assetName(failure, scene),
  });
}

/** 1 つの受け手の、読めなかったことすべて (バックエンドが送る順。モデル・モジュール・背景の画像)。 */
export interface ReceiverFailures {
  label: string;
  failures: ReceiverFailure[];
}

/** 受け手ごとにまとめる (操作ウィンドウは受け手ごとに 1 つの知らせにする。T0840)。受け手は最初に出てきた順。 */
export function groupLoadFailures(failures: readonly ReceiverFailure[]): ReceiverFailures[] {
  const groups = new Map<string, ReceiverFailure[]>();
  for (const failure of failures) {
    const group = groups.get(failure.label);
    if (group === undefined) groups.set(failure.label, [failure]);
    else group.push(failure);
  }
  return [...groups].map(([label, failures]) => ({ label, failures }));
}
