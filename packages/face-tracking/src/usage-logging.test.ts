import { describe, expect, it, vi } from "vitest";
import { blockMediaPipeUsageLogging, isMediaPipeUsageLog } from "./usage-logging";

function fakeHost() {
  const inner = vi.fn(
    async (_input: Parameters<typeof fetch>[0], _init?: RequestInit) => new Response("ok"),
  );
  const host = { fetch: inner as unknown as typeof fetch };
  return { host, inner };
}

describe("isMediaPipeUsageLog", () => {
  it("matches the usage log endpoint in any request form", () => {
    const url = "https://odml.pa.googleapis.com/v1/log";
    expect(isMediaPipeUsageLog(url)).toBe(true);
    expect(isMediaPipeUsageLog(new URL(url))).toBe(true);
    expect(isMediaPipeUsageLog(new Request(url, { method: "POST" }))).toBe(true);
  });

  it("does not match other origins or relative paths", () => {
    expect(isMediaPipeUsageLog("https://example.com/odml.pa.googleapis.com/v1/log")).toBe(false);
    expect(isMediaPipeUsageLog("https://pa.googleapis.com/v1/log")).toBe(false);
    expect(isMediaPipeUsageLog("mediapipe/face_landmarker.task")).toBe(false);
  });
});

describe("blockMediaPipeUsageLogging", () => {
  it("rejects the usage log like a network failure without calling fetch", async () => {
    const { host, inner } = fakeHost();
    blockMediaPipeUsageLogging(host);
    // MediaPipe と同じ形の要求 (vision_bundle.mjs の送信)
    const sent = host.fetch("https://odml.pa.googleapis.com/v1/log", {
      method: "POST",
      headers: { "Content-Type": "application/x-protobuf" },
      body: new Uint8Array([1, 2, 3]),
    });
    await expect(sent).rejects.toBeInstanceOf(TypeError);
    expect(inner).not.toHaveBeenCalled();
  });

  it("passes other requests through with the same arguments", async () => {
    const { host, inner } = fakeHost();
    blockMediaPipeUsageLogging(host);
    const init = { method: "HEAD" };
    const response = await host.fetch("http://localhost/mediapipe/face_landmarker.task", init);
    expect(await response.text()).toBe("ok");
    expect(inner).toHaveBeenCalledWith("http://localhost/mediapipe/face_landmarker.task", init);
  });

  it("wraps only once", async () => {
    const { host, inner } = fakeHost();
    blockMediaPipeUsageLogging(host);
    const first = host.fetch;
    blockMediaPipeUsageLogging(host);
    expect(host.fetch).toBe(first);
    await host.fetch("http://localhost/a");
    expect(inner).toHaveBeenCalledTimes(1);
  });
});
