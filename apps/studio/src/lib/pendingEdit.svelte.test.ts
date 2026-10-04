// @vitest-environment happy-dom
// 送っている途中の設定 (pendingEdit.svelte.ts)。
import { describe, expect, it } from "vitest";
import { PendingEdit } from "./pendingEdit.svelte";

/** 送るたびに、終わらせる関数を積む送り手。 */
function deferredSender<T>() {
  const sent: T[] = [];
  const finish: (() => void)[] = [];
  const send = (value: T) => {
    sent.push(value);
    return new Promise<void>((resolve) => finish.push(resolve));
  };
  return { sent, finish, send };
}

/** 待っている Promise の続きを進める。 */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("PendingEdit", () => {
  it("shows the value being sent until the send finishes", async () => {
    const { sent, finish, send } = deferredSender<number>();
    const edit = new PendingEdit(send);
    expect(edit.current(1)).toBe(1);
    const done = edit.submit(2);
    expect(sent).toEqual([2]);
    expect(edit.current(1)).toBe(2);
    finish.shift()?.();
    await done;
    // 通知が追いついた値に戻す。
    expect(edit.current(3)).toBe(3);
  });

  it("sends one at a time and only the last value queued while sending", async () => {
    const { sent, finish, send } = deferredSender<number>();
    const edit = new PendingEdit(send);
    const done = edit.submit(1);
    void edit.submit(2);
    void edit.submit(3);
    expect(sent).toEqual([1]);
    expect(edit.current(0)).toBe(3);
    finish.shift()?.();
    await settle();
    expect(sent).toEqual([1, 3]);
    expect(edit.current(0)).toBe(3);
    finish.shift()?.();
    await done;
    expect(sent).toEqual([1, 3]);
    expect(edit.current(0)).toBe(0);
  });

  it("drops the value being sent when the send fails", async () => {
    const edit = new PendingEdit<number>(() => Promise.reject(new Error("closed")));
    await expect(edit.submit(1)).rejects.toThrow("closed");
    expect(edit.current(0)).toBe(0);
    // 次の変更は送れる。
    const sent: number[] = [];
    const next = new PendingEdit<number>(async (value) => {
      sent.push(value);
    });
    await next.submit(2);
    expect(sent).toEqual([2]);
  });
});
