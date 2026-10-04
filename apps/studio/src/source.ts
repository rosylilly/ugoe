// ブラウザソースのページ (source.html、T0808) の入口。スタジオの待ち受けが `/source/<トークン>/` で出す。
// 出力ウィンドウと同じ受け手 (Output.svelte) を、WebSocket の口 (lib/feed.ts の `sourceFeed`) で動かす。
// 文言を出さないので、@ugoe/ui と @ugoe/i18n は読まない。
import { mount } from "svelte";
import { type OutputFeed, sourceFeed } from "./lib/feed";
import Output from "./Output.svelte";

const target = document.getElementById("app");
// i18n-ignore: 開発者向け (index.html の誤り。起こらない)
if (!target) throw new Error("#app not found");

const feed: OutputFeed = sourceFeed({ base: new URL(".", window.location.href).href });

export default mount(Output, { target, props: { feed: async () => feed } });
