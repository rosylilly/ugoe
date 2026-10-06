// パラメータの一覧の区切りの開閉を localStorage に覚える (docs/editor.md#パラメータのグループ と docs/editor.md#覚える値)。
// 覚えるのは閉じた区切りのグループの ID だけ (既定は開く)。プロジェクトを分けずに覚えるので、同じ ID のグループは
// どのプロジェクトでも同じ開閉になる。DOM に依存しない部分だけを置く (sections.test.ts で検査する)。
import { readStoredItem, writeStoredItem } from "@ugoe/i18n";
import { STORAGE_KEYS } from "../../storageKeys";

/** グループの無いパラメータの区切り (「Other」) の印。グループの ID は空にならないので重ならない。 */
export const UNGROUPED_SECTION = "";

/** 覚える ID の数の上限 (古いプロジェクトのグループで際限なく増えないように、新しく閉じたものを残す)。 */
export const MAX_CLOSED_SECTIONS = 200;

/** 区切りの印 (`groupParameters` の区切りの `id`)。 */
export function sectionKey(id: string | null): string {
  return id ?? UNGROUPED_SECTION;
}

/** 閉じた区切りの印。覚えていない・読めなければ空 (すべて開く)。 */
export function loadClosedSections(): Set<string> {
  const stored = readStoredItem(STORAGE_KEYS.parameterSectionsClosed);
  if (stored === null) return new Set();
  try {
    const value: unknown = JSON.parse(stored);
    if (!Array.isArray(value)) return new Set();
    return new Set(
      value.filter((v): v is string => typeof v === "string").slice(-MAX_CLOSED_SECTIONS),
    );
  } catch {
    return new Set();
  }
}

/** 覚えられなくても表示は変える。 */
export function saveClosedSections(closed: ReadonlySet<string>): void {
  const list = [...closed].slice(-MAX_CLOSED_SECTIONS);
  writeStoredItem(STORAGE_KEYS.parameterSectionsClosed, JSON.stringify(list));
}

/** 区切り `key` を開く・閉じたあとの集合 (新しく閉じたものを最後に置く)。 */
export function withSectionOpen(
  closed: ReadonlySet<string>,
  key: string,
  open: boolean,
): Set<string> {
  const next = new Set(closed);
  next.delete(key);
  if (!open) next.add(key);
  return next;
}
