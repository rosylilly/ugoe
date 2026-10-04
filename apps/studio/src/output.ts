// 出力ウィンドウ (output.html) の入口。文言を出さないので、@ugoe/ui と @ugoe/i18n は読まない。
import { mount } from "svelte";
import { studioHost } from "./lib/host";
import Output from "./Output.svelte";

const target = document.getElementById("app");
// i18n-ignore: 開発者向け (index.html の誤り。起こらない)
if (!target) throw new Error("#app not found");

export default mount(Output, { target, props: { feed: studioHost } });
