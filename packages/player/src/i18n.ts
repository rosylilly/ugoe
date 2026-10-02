// 画面に出す文言 (@ugoe/i18n の名前空間 `player`)。カタログは locales/<言語>.json で、原文は英語。
// 言語はアプリが @ugoe/i18n で決める。`t()` は呼んだときの言語で引くので、Player の状態に入れた文
// (エラー・警告など) は、作ったときの言語のまま残る。
import { registerCatalogs, translator } from "@ugoe/i18n";
import en from "../locales/en.json";
import ja from "../locales/ja.json";

declare module "@ugoe/i18n" {
  interface CatalogTypes {
    player: typeof en;
  }
}

registerCatalogs("player", { en, ja });

/** 今の言語で文言を引く。テンプレートや `$derived` の中で呼ぶと、言語の切り替えに追従する。 */
export const t = translator("player");
