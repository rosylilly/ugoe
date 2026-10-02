// 操作ウィンドウ (index.html) の入口。
import "@ugoe/ui/tokens.css";
import { setLocale, setSystemLanguages } from "@ugoe/i18n";
import { initTheme } from "@ugoe/ui";
import { mount } from "svelte";
import Control from "./Control.svelte";
import { studioHost } from "./lib/host";
import { loadLocalePreference, THEME_STORAGE_KEY } from "./lib/settings";

// 画面の言語: 覚えた設定 (既定は OS に合わせる)。OS の言語は WebView の navigator.languages ではなく、
// tauri-plugin-os の locale() で OS から取る (WebView によっては OS の言語と違う値を返すため。packages/i18n/README.md)。
setLocale(loadLocalePreference());
void studioHost()
  .then((host) => host.systemLocale())
  .then((locale) => {
    if (locale !== null) setSystemLanguages([locale]);
  })
  .catch(() => {
    // Tauri の外では navigator.languages のまま。
  });

// テーマ: 覚えた設定 (既定は OS に合わせる)。操作ウィンドウだけに効かせる (出力ウィンドウは透過のまま)。
initTheme(THEME_STORAGE_KEY);

const target = document.getElementById("app");
if (!target) throw new Error("#app not found");

export default mount(Control, { target });
