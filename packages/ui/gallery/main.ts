// 部品の見本の画面。URL の引数でテーマ・密度・言語を決める (スクリーンショットの検査が使う):
// `?theme=light|dark&density=compact|comfortable&lang=en|ja`

import "../src/tokens.css";
import { mount, tick } from "svelte";
import Gallery from "./Gallery.svelte";

const root = document.documentElement;
const fail = (message: string) => {
  root.dataset.galleryError = message;
};
window.addEventListener("error", (e) => fail(String(e.message)));
window.addEventListener("unhandledrejection", (e) => fail(String(e.reason)));

const target = document.getElementById("app");
if (!target) throw new Error("#app が無い");
mount(Gallery, { target });
// 描き終えたことをスクリーンショットの検査に知らせる
void tick().then(() =>
  requestAnimationFrame(() => {
    root.dataset.galleryReady = "true";
  }),
);
