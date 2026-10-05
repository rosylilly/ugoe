// パラメータのグループ (Model::parameter_groups) の選択肢とコマンド (docs/editor.md#パラメータのグループ)。
// DOM に依存しない純粋な関数だけを置く (groups.test.ts で検査する)。
import type { Command, Parameter, ParameterGroup } from "../../generated/types";
import { t } from "../../i18n";
import { elementLabel } from "../../labels";

/** 選択欄の値: グループなし (空文字列)・グループの ID。 */
export interface GroupOption {
  value: string;
  label: string;
}

/** パラメータのグループの選択肢。先頭が「なし」、残りはグループ (並び順のまま、名前と ID)。 */
export function groupOptions(groups: readonly ParameterGroup[]): GroupOption[] {
  return [
    { value: "", label: t("parameters.group.none") },
    ...groups.map((g) => ({ value: g.id, label: elementLabel(g.name, g.id) })),
  ];
}

/** パラメータをグループに入れるコマンド (空文字列なら外す)。 */
export function setGroupCommand(id: string, value: string): Command {
  return { op: "set_parameter_group", id, group: value === "" ? null : value };
}

/** グループを足すコマンド。ID は前後の空白を除き、名前が空なら ID を名前にする。ID が空なら null。 */
export function addGroupCommand(id: string, name: string): Command | null {
  const trimmed = id.trim();
  if (trimmed === "") return null;
  return { op: "add_parameter_group", group: { id: trimmed, name: name.trim() || trimmed } };
}

/**
 * グループ `id` を 1 つ上 (`-1`) か下 (`+1`) へ移すコマンド。端で動けなければ null。
 * `reorder_parameter_group` は `before` の直前へ移す (null なら末尾)。
 */
export function moveGroupCommand(
  groups: readonly ParameterGroup[],
  id: string,
  delta: -1 | 1,
): Command | null {
  const index = groups.findIndex((g) => g.id === id);
  const neighbour = index < 0 ? undefined : groups[index + delta];
  if (!neighbour) return null;
  // 上へは 1 つ上のものの直前へ。下へは 1 つ下のものの次の直前 (無ければ末尾) へ。
  const before = delta < 0 ? neighbour.id : (groups[index + 2]?.id ?? null);
  return { op: "reorder_parameter_group", id, before };
}

/** グループ `id` に入っているパラメータの数。 */
export function memberCount(parameters: readonly Parameter[], id: string): number {
  return parameters.filter((p) => p.group === id).length;
}
