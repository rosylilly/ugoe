// パラメータの左右の対 (Parameter::mirror) の選択肢と提案 (docs/editor.md#キーフォームの編集)。
// DOM に依存しない純粋な関数だけを置く (mirror.test.ts で検査する)。
import type { Command, Parameter } from "../../generated/types";
import { t } from "../../i18n";
import { elementLabel } from "../../labels";

/** 選択欄の値: 対なし (空文字列)・自分自身 (符号の反転)・相手のパラメータの ID。 */
export interface MirrorOption {
  value: string;
  label: string;
}

/** `self` の対の選択肢。先頭が「なし」、次が「自分自身」、残りは他のパラメータ (名前と ID)。 */
export function mirrorOptions(parameters: readonly Parameter[], self: string): MirrorOption[] {
  return [
    { value: "", label: t("parameters.mirror.none") },
    { value: self, label: t("parameters.mirror.itself") },
    ...parameters
      .filter((p) => p.id !== self)
      .map((p) => ({ value: p.id, label: elementLabel(p.name, p.id) })),
  ];
}

/** 対を設定するコマンド (空文字列なら外す)。対は双方向に張られる。 */
export function setMirrorCommand(id: string, value: string): Command {
  return { op: "set_parameter_mirror", id, mirror: value === "" ? null : value };
}

/** 左右を入れ替えた ID (`EyeLOpen` → `EyeROpen`、`ArmLeft` → `ArmRight`)。入れ替える所がなければ null。 */
export function swapSide(id: string): string | null {
  const swapped = id
    .replace(/Left|Right/g, (s) => (s === "Left" ? "Right" : "Left"))
    .replace(/(?<=[a-z0-9])[LR](?=[A-Z0-9]|$)/g, (s) => (s === "L" ? "R" : "L"));
  return swapped === id ? null : swapped;
}

/** 自分自身と対にする (値の符号を反転して対応させる) 標準のパラメータ: 左右の向き・傾き・目玉の左右。 */
const SELF_MIRRORED = /(AngleX|AngleZ|BallX)$/;

/**
 * 標準の命名から提案する対: 左右を入れ替えた ID のパラメータがあればそれ、左右の向き・傾きなら自分自身。
 * なければ null。
 */
export function suggestedMirror(id: string, ids: readonly string[]): string | null {
  const partner = swapSide(id);
  if (partner && ids.includes(partner)) return partner;
  return SELF_MIRRORED.test(id) ? id : null;
}
