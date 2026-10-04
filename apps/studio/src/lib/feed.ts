// 受け手 (Output.svelte) が配信を受け取る口。出力ウィンドウは Tauri の StudioHost (host.ts) をそのまま使い、
// ブラウザソースのページ (source.html、T0808) は、スタジオの待ち受けの WebSocket と HTTP を使う `sourceFeed` を使う
// (docs/studio.md#ブラウザソース、apps/studio/src-tauri/src/api/source.rs)。
import type { LoadFailure, RenderStats, StudioHost } from "./host";

/** 受け手が使う口 (StudioHost の一部)。ウィンドウを動かす口は出力ウィンドウにだけある。 */
export type OutputFeed = Pick<
  StudioHost,
  | "subscribeFrames"
  | "sceneModel"
  | "sceneModule"
  | "sceneBackground"
  | "reportRenderStats"
  | "reportLoadFailure"
> &
  Partial<Pick<StudioHost, "startDragging">>;

/** 接続が切れてから、つなぎ直すまでの時間 (ミリ秒)。スタジオを起動し直しても OBS のページはそのまま戻る。 */
export const RECONNECT_MS = 1000;

export interface SourceFeedOptions {
  /** ページの URL (`http://127.0.0.1:<port>/source/<トークン>/`)。ファイルとフレームはこの下から取る。 */
  base: string;
  /** テストで置き換える。 */
  WebSocket?: typeof WebSocket;
  fetch?: typeof fetch;
  setTimeout?: (run: () => void, ms: number) => unknown;
}

/** フレームの WebSocket の URL (`ws://…/source/<トークン>/frames`)。 */
export function framesUrl(base: string): string {
  const url = new URL("frames", base);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.hash = "";
  url.search = "";
  return url.href;
}

/**
 * ブラウザソースのページの口。シーンの通知 (テキストの JSON) とフレーム (バイナリ) を WebSocket で受け、
 * 今のシーンのファイルを HTTP で取る。切れたら `RECONNECT_MS` 後につなぎ直す (つないだときに今のシーンが最初に届く。
 * 同じシーンなら受け手は見せ方だけを当て直し、`seq` が飛んだので物理をリセットして続ける)。
 * 描画の負荷と読めなかったことは同じ WebSocket で返す (操作ウィンドウに出る)。
 */
export function sourceFeed(options: SourceFeedOptions): OutputFeed {
  const Socket = options.WebSocket ?? WebSocket;
  const get = options.fetch ?? ((input, init) => fetch(input, init));
  const later = options.setTimeout ?? ((run, ms) => setTimeout(run, ms));
  let socket: WebSocket | null = null;

  /** つながっていれば送る (切れている間の報告は捨てる。つなぎ直すと今のシーンが届き、受け手は読み直す)。 */
  function send(message: object) {
    if (socket !== null && socket.readyState === Socket.OPEN) socket.send(JSON.stringify(message));
  }

  async function bytes(path: string): Promise<Uint8Array> {
    const response = await get(new URL(path, options.base).href, { cache: "no-store" });
    if (!response.ok) throw new Error(`${path}: ${response.status}`);
    return new Uint8Array(await response.arrayBuffer());
  }

  return {
    async subscribeFrames(onMessage) {
      const open = () => {
        const ws = new Socket(framesUrl(options.base));
        ws.binaryType = "arraybuffer";
        ws.onmessage = (event: MessageEvent) => {
          if (event.data instanceof ArrayBuffer) {
            onMessage(event.data);
            return;
          }
          if (typeof event.data !== "string") return;
          try {
            const message = JSON.parse(event.data);
            if (message?.kind === "scene") onMessage(message);
          } catch {
            // 読めないものは捨てる (スタジオとページは同じ版なので、ふつうは起きない)。
          }
        };
        ws.onclose = () => {
          if (socket === ws) socket = null;
          later(open, RECONNECT_MS);
        };
        socket = ws;
      };
      open();
    },
    sceneModel: (revision) => bytes(`scene/${revision}/model`),
    sceneModule: (revision, index) => bytes(`scene/${revision}/module/${index}`),
    sceneBackground: (revision) => bytes(`scene/${revision}/background`),
    async reportRenderStats(stats: RenderStats) {
      send({ kind: "renderStats", ...stats });
    },
    async reportLoadFailure(failure: LoadFailure) {
      send({ kind: "loadFailure", ...failure });
    },
  };
}
