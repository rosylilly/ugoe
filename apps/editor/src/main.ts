import "@ugoe/ui/tokens.css";
import { initLocale } from "@ugoe/i18n";
import { initTheme } from "@ugoe/ui";
import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { LOCALE_KEY, systemLocale } from "./lib/locale";

// 画面の言語: 覚えた設定 (既定は OS に合わせる)。OS の言語は tauri-plugin-os で取る (lib/locale.ts)。
void initLocale(LOCALE_KEY, systemLocale);

// テーマ: 覚えた設定 (既定は OS に合わせる)。index.html の <head> のスクリプトも同じキーを読む。
initTheme("ugoe-editor.theme");

const target = document.getElementById("app");
// i18n-ignore: 開発者向け (index.html の誤り。起こらない)
if (!target) throw new Error("#app not found");

export default mount(App, { target });
