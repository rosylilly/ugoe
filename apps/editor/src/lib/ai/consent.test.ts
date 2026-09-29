import { describe, expect, it } from "vitest";
import type { AiReply, ConsentAnswer, DisclosureView } from "../ipc";
import { formatBytes, withConsent } from "./consent";

const disclosure: DisclosureView = {
  service: "Anthropic Claude API",
  endpoint: "https://api.anthropic.com/v1/messages",
  model: "claude-opus-5-5",
  image_count: 2,
  image_bytes: 4096,
  text_bytes: 120,
};

/** 同意がなければ `needs_consent` を返し、同意があれば `value` を返すコマンド。 */
function command(value: string) {
  const calls: (ConsentAnswer | null)[] = [];
  const call = async (consent: ConsentAnswer | null): Promise<AiReply<string>> => {
    calls.push(consent);
    return consent ? { status: "done", value } : { status: "needs_consent", disclosure };
  };
  return { call, calls };
}

describe("withConsent", () => {
  it("asks, then calls again with the answer", async () => {
    const { call, calls } = command("ok");
    const asked: DisclosureView[] = [];
    const result = await withConsent(call, async (d) => {
      asked.push(d);
      return { disclosure: d, remember: true };
    });
    expect(result).toEqual({ value: "ok" });
    expect(asked).toEqual([disclosure]);
    expect(calls).toEqual([null, { disclosure, remember: true }]);
  });

  it("does not send when the user declines", async () => {
    const { call, calls } = command("ok");
    expect(await withConsent(call, async () => null)).toBeNull();
    expect(calls).toEqual([null]);
  });

  it("does not ask when no consent is needed", async () => {
    let asked = false;
    const result = await withConsent(
      async () => ({ status: "done", value: 1 }) as AiReply<number>,
      async () => {
        asked = true;
        return null;
      },
    );
    expect(result).toEqual({ value: 1 });
    expect(asked).toBe(false);
  });

  it("refuses to loop when the data changed after consent", async () => {
    const call = async (): Promise<AiReply<string>> => ({ status: "needs_consent", disclosure });
    await expect(
      withConsent(call, async (d) => ({ disclosure: d, remember: false })),
    ).rejects.toThrow(/changed/);
  });

  it("passes errors through", async () => {
    const call = async (): Promise<AiReply<string>> => {
      throw "this project is set not to send data to external AI services";
    };
    await expect(withConsent(call, async () => null)).rejects.toMatch(/not to send/);
  });
});

describe("formatBytes", () => {
  it("uses binary units", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(200 * 1024)).toBe("200 KB");
    expect(formatBytes(3.25 * 1024 * 1024)).toBe("3.3 MB");
  });
});
