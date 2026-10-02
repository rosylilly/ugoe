// 画面の文言 (@ugoe/i18n の名前空間 `editor`。ADR 0029、docs/ui.md#文言の書き方)。カタログは locales/<言語>.json で、原文は英語。
import { registerCatalogs, translator } from "@ugoe/i18n";
import en from "../../locales/en.json";
import ja from "../../locales/ja.json";

declare module "@ugoe/i18n" {
  interface CatalogTypes {
    editor: typeof en;
  }
}

registerCatalogs("editor", { en, ja });

/** 今の言語で文言を引く。テンプレートや `$derived` の中で呼ぶと、言語の切り替えに追従する。 */
export const t = translator("editor");
