import { Channel } from "@tauri-apps/api/core";
import { afterEach, describe, expect, it, vi } from "vitest";
import { safeChannelHandler } from "./channel";

/** Tauri の口を偽って Channel を作り、バックエンドが送ったことにする関数を返す。 */
function fakeChannel<T>(onmessage: (message: T) => void) {
  let deliver: ((raw: unknown) => void) | null = null;
  vi.stubGlobal("window", {
    __TAURI_INTERNALS__: {
      transformCallback: (callback: (raw: unknown) => void) => {
        deliver = callback;
        return 1;
      },
    },
  });
  const channel = new Channel<T>();
  channel.onmessage = onmessage;
  let index = 0;
  return (message: T) => deliver?.({ index: index++, message });
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("safeChannelHandler", () => {
  it("stops a raw Channel after the handler throws (the reason for the wrapper)", () => {
    const got: number[] = [];
    const send = fakeChannel<number>((n) => {
      got.push(n);
      if (n === 1) throw new Error("broken");
    });
    expect(() => send(0)).not.toThrow();
    expect(() => send(1)).toThrow("broken");
    send(2);
    send(3);
    expect(got).toEqual([0, 1]);
  });

  it("keeps delivering the next messages when the handler throws", () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => {});
    const got: number[] = [];
    const send = fakeChannel<number>(
      safeChannelHandler((n) => {
        got.push(n);
        if (n === 1) throw new Error("broken");
      }),
    );
    for (const n of [0, 1, 2, 3]) send(n);
    expect(got).toEqual([0, 1, 2, 3]);
    expect(error).toHaveBeenCalledTimes(1);
  });
});
