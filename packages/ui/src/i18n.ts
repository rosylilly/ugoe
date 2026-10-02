// 部品の既定の文言 (@ugoe/i18n の名前空間 `ui`)。カタログは locales/<言語>.json で、原文は英語。
// 部品は、props で文言を渡されなかったときだけここから引く。言語はアプリが @ugoe/i18n で決める。
import { registerCatalogs, translator } from "@ugoe/i18n";
import en from "../locales/en.json";
import ja from "../locales/ja.json";

declare module "@ugoe/i18n" {
  interface CatalogTypes {
    ui: typeof en;
  }
}

registerCatalogs("ui", { en, ja });

/** 今の言語で文言を引く。テンプレートや `$derived` の中で呼ぶと、言語の切り替えに追従する。 */
export const t = translator("ui");
