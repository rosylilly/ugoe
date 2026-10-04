// 画面の文言 (@ugoe/i18n の名前空間 `face-tracking`)。カタログは locales/<言語>.json で、原文は英語。
// 言語はアプリが @ugoe/i18n で決める (どのアプリも設定の言語。既定は OS に合わせる)。
import { registerCatalogs, translator } from "@ugoe/i18n";
import en from "../locales/en.json";
import ja from "../locales/ja.json";

declare module "@ugoe/i18n" {
  interface CatalogTypes {
    "face-tracking": typeof en;
  }
}

registerCatalogs("face-tracking", { en, ja });

/** 今の言語で文言を引く。テンプレートや `$derived` の中で呼ぶと、言語の切り替えに追従する。 */
export const t = translator("face-tracking");
