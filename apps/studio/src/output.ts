// 出力ウィンドウ (output.html) の入口。文言を出さないので、@ugoe/ui と @ugoe/i18n は読まない。
import { mount } from "svelte";
import { studioHost } from "./lib/host";
import Output from "./Output.svelte";

const target = document.getElementById("app");
if (!target) throw new Error("#app not found");

export default mount(Output, { target, props: { feed: studioHost } });
