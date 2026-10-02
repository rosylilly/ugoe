import { describe, expect, it } from "vitest";
import { latestOnly } from "./latest";

describe("latestOnly", () => {
  it("sends the first value and only the last of the values that came while sending", async () => {
    const sent: number[] = [];
    const waits: (() => void)[] = [];
    const send = latestOnly((value: number) => {
      sent.push(value);
      return new Promise<void>((resolve) => waits.push(resolve));
    });
    send(1);
    send(2);
    send(3);
    expect(sent).toEqual([1]);
    waits.shift()?.();
    await Promise.resolve();
    await Promise.resolve();
    expect(sent).toEqual([1, 3]);
    waits.shift()?.();
    await new Promise((resolve) => setTimeout(resolve, 0));
    send(4);
    expect(sent).toEqual([1, 3, 4]);
  });

  it("keeps going after a failure", async () => {
    const sent: number[] = [];
    const send = latestOnly(async (value: number) => {
      sent.push(value);
      if (value === 1) throw new Error("closed");
    });
    send(1);
    await new Promise((resolve) => setTimeout(resolve, 0));
    send(2);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(sent).toEqual([1, 2]);
  });
});
