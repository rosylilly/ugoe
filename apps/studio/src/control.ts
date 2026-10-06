// 操作ウィンドウ (index.html) の入口。
import "@ugoe/ui/tokens.css";
import { initLocale } from "@ugoe/i18n";
import { initTheme } from "@ugoe/ui";
import { mount } from "svelte";
import Control from "./Control.svelte";
import { studioHost } from "./lib/host";
import { STORAGE_KEYS } from "./lib/storageKeys";

// 画面の言語: 覚えた設定 (既定は OS に合わせる)。OS の言語は WebView の navigator.languages ではなく、
// tauri-plugin-os の locale() で OS から取る (WebView によっては OS の言語と違う値を返すため。packages/i18n/README.md)。
// Tauri の外では取れず、navigator.languages のまま。
void initLocale(STORAGE_KEYS.locale, () => studioHost().then((host) => host.systemLocale()));

// テーマ: 覚えた設定 (既定は OS に合わせる)。操作ウィンドウだけに効かせる (出力ウィンドウは透過のまま)。
initTheme(STORAGE_KEYS.theme);

const target = document.getElementById("app");
// i18n-ignore: 開発者向け (index.html の誤り。起こらない)
if (!target) throw new Error("#app not found");

export default mount(Control, { target });
