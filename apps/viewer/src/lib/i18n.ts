// 画面の文言 (@ugoe/i18n の名前空間 `viewer`)。カタログは locales/<言語>.json で、原文は英語。
import { registerCatalogs, translator } from "@ugoe/i18n";
import en from "../../locales/en.json";
import ja from "../../locales/ja.json";

declare module "@ugoe/i18n" {
  interface CatalogTypes {
    viewer: typeof en;
  }
}

registerCatalogs("viewer", { en, ja });

/** 今の言語で文言を引く。テンプレートや `$derived` の中で呼ぶと、言語の切り替えに追従する。 */
export const t = translator("viewer");
