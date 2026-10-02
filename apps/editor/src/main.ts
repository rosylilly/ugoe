import "@ugoe/ui/tokens.css";
import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { initLocale } from "./lib/locale";

// 画面の言語: 覚えた設定 (既定は OS に合わせる)。OS の言語は tauri-plugin-os で取る (lib/locale.ts)。
initLocale();

const target = document.getElementById("app");
if (!target) throw new Error("#app not found");

export default mount(App, { target });
