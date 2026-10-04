import "@ugoe/ui/tokens.css";
import { initLocale } from "@ugoe/i18n";
import { initTheme } from "@ugoe/ui";
import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { systemLocale } from "./lib/host";
import { LOCALE_STORAGE_KEY, THEME_STORAGE_KEY } from "./lib/settings";

// テーマ: 覚えた設定 (既定は OS に合わせる)。OS のテーマの変化にも追従する。
initTheme(THEME_STORAGE_KEY);

const target = document.getElementById("app");
if (!target) throw new Error("#app not found");

// 画面の言語: 覚えた設定 (既定は OS に合わせる)。OS の言語は WebView の navigator.languages ではなく、
// tauri-plugin-os の locale() で OS から取る (WebView によっては OS の言語と違う値を返すため。packages/i18n/README.md)。
// 最初の画面から OS の言語で出すよう、取れてから描く (Tauri の外では navigator.languages のまま)。
// OS の言語が取れなくても、navigator.languages で動く。
await initLocale(LOCALE_STORAGE_KEY, systemLocale);

mount(App, { target });
