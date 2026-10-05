// 部品の props で使う型。

/** Button・IconButton の種類 (docs/ui.md#部品の一覧)。`primary` は画面に 1 つまで。 */
export type ButtonVariant = "primary" | "secondary" | "danger" | "ghost";

/** 選択肢 (Select・SegmentedControl・Tabs)。`label` は翻訳を通した文言を渡す。 */
export interface Option<T extends string = string> {
  value: T;
  label: string;
  disabled?: boolean;
}

/** Button と IconButton が共通で受ける props。 */
export interface ButtonBaseProps {
  /** 種類。既定は `secondary` (Toolbar の中では `ghost`)。 */
  variant?: ButtonVariant;
  /** 小さいボタン (行の中など)。高さは押せる大きさの下限 (24px) を下回らない。 */
  small?: boolean;
  /** 押された状態 (入・切のボタン)。`aria-pressed` になる。 */
  pressed?: boolean;
  disabled?: boolean;
  /** 実行中 (押すと二重になる操作の待ち)。無効にし、`aria-busy` を付ける。文言 (「保存しています…」) は呼び手が変える。 */
  busy?: boolean;
  type?: "button" | "submit" | "reset";
  id?: string;
  class?: string;
  /** 補足の説明を持つ要素の id (無効の理由など)。 */
  "aria-describedby"?: string;
  /** 読み上げの名前を持つ要素の id (見えるラベルを別に置くとき)。 */
  "aria-labelledby"?: string;
  "aria-controls"?: string;
  "aria-expanded"?: boolean;
  "aria-haspopup"?: "menu" | "dialog" | "listbox" | boolean;
  onclick?: (event: MouseEvent) => void;
}

/** 知らせ (Notice・Toast) の種類。色に加えて印 (アイコン) と読み上げの言葉でも区別する。 */
export type Tone = "info" | "success" | "warning" | "danger";

/** 重ねて出す部品 (Tooltip・Popover・Menu) を、トリガーのどちら側に出すか。 */
export type Side = "top" | "right" | "bottom" | "left";

/** Menu・ContextMenu の項目。`label` は翻訳を通した文言を渡す。 */
export type MenuEntry =
  | {
      kind?: "item";
      label: string;
      onselect: () => void;
      /** ショートカット (`"Mod+S"`。Kbd の書き方)。項目の右に出す。 */
      shortcut?: string;
      disabled?: boolean;
      /** 取り消せない操作 (削除など)。 */
      danger?: boolean;
    }
  | {
      kind: "checkbox";
      label: string;
      checked: boolean;
      onchange: (checked: boolean) => void;
      shortcut?: string;
      disabled?: boolean;
    }
  | { kind: "separator" }
  | { kind: "submenu"; label: string; entries: MenuEntry[]; disabled?: boolean };
