import { setLocale } from "@ugoe/i18n";
import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

// 画面の言語。言語の切り替えの画面ができるまでは、いままでどおり日本語で出す (@ugoe/i18n の README)。
setLocale("ja");

const target = document.getElementById("app");
if (!target) throw new Error("#app not found");

export default mount(App, { target });
