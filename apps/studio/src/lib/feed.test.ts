import { describe, expect, it } from "vitest";
import { framesUrl, RECONNECT_MS, sourceFeed } from "./feed";
import type { StreamMessage } from "./host";

/** 偽の WebSocket (作ったものを覚え、届いたことにする口を持つ)。 */
class FakeSocket {
  static OPEN = 1;
  static made: FakeSocket[] = [];
  readyState = 0;
  binaryType = "blob";
  sent: string[] = [];
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onclose: (() => void) | null = null;
  constructor(readonly url: string) {
    FakeSocket.made.push(this);
  }
  send(text: string) {
    this.sent.push(text);
  }
  receive(data: unknown) {
    this.onmessage?.({ data });
  }
}

const BASE = "http://127.0.0.1:28451/source/abc/";

function feed() {
  FakeSocket.made = [];
  const fetched: string[] = [];
  const timers: [() => void, number][] = [];
  const f = sourceFeed({
    base: BASE,
    WebSocket: FakeSocket as unknown as typeof WebSocket,
    fetch: (async (url: string) => {
      fetched.push(url);
      return url.endsWith("/9/model")
        ? new Response(null, { status: 404 })
        : new Response(new Uint8Array([1, 2, 3]));
    }) as typeof fetch,
    setTimeout: (run, ms) => timers.push([run, ms]),
  });
  return { f, fetched, timers };
}

describe("sourceFeed", () => {
  it("connects to the frames socket next to the page", () => {
    expect(framesUrl(BASE)).toBe("ws://127.0.0.1:28451/source/abc/frames");
    expect(framesUrl("https://localhost:1/source/x/?a#b")).toBe(
      "wss://localhost:1/source/x/frames",
    );
  });

  it("passes scenes and frames and drops anything else", async () => {
    const { f } = feed();
    const got: StreamMessage[] = [];
    await f.subscribeFrames((m) => got.push(m));
    const socket = FakeSocket.made[0] as FakeSocket;
    expect(socket.binaryType).toBe("arraybuffer");
    const frame = new ArrayBuffer(4);
    socket.receive(JSON.stringify({ kind: "scene", revision: 3 }));
    socket.receive(frame);
    socket.receive("not json");
    socket.receive(JSON.stringify({ kind: "other" }));
    expect(got).toEqual([{ kind: "scene", revision: 3 }, frame]);
  });

  it("reconnects after the studio goes away", async () => {
    const { f, timers } = feed();
    await f.subscribeFrames(() => {});
    (FakeSocket.made[0] as FakeSocket).onclose?.();
    expect(timers.map(([, ms]) => ms)).toEqual([RECONNECT_MS]);
    timers[0]?.[0]();
    expect(FakeSocket.made).toHaveLength(2);
  });

  it("reads the scene files under the token and reports stats and failures when open", async () => {
    const { f, fetched } = feed();
    expect(await f.sceneModel(7)).toEqual(new Uint8Array([1, 2, 3]));
    await f.sceneModule(7, 1);
    await f.sceneBackground(7);
    await expect(f.sceneModel(9)).rejects.toThrow("404");
    expect(fetched).toEqual([
      `${BASE}scene/7/model`,
      `${BASE}scene/7/module/1`,
      `${BASE}scene/7/background`,
      `${BASE}scene/9/model`,
    ]);
    const stats = { fps: 60, renderMs: 1, renderMaxMs: 2, dropped: 0 };
    await f.reportRenderStats(stats);
    await f.subscribeFrames(() => {});
    const socket = FakeSocket.made[0] as FakeSocket;
    await f.reportRenderStats(stats);
    expect(socket.sent).toEqual([]);
    socket.readyState = FakeSocket.OPEN;
    await f.reportRenderStats(stats);
    const failure = { revision: 7, asset: "module", index: 1, code: "unreadable" } as const;
    await f.reportLoadFailure(failure);
    expect(socket.sent.map((s) => JSON.parse(s))).toEqual([
      { kind: "renderStats", ...stats },
      { kind: "loadFailure", ...failure },
    ]);
    expect(f.startDragging).toBeUndefined();
  });
});
