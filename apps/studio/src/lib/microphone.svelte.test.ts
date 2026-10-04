import { afterEach, describe, expect, it, vi } from "vitest";
import { Microphone } from "./microphone.svelte";

/** 偽のマイク (getUserMedia の応答を手で返し、止めたトラックと閉じた AudioContext を数える)。 */
function fakeDevices() {
  const requests: ((stream: MediaStream) => void)[] = [];
  const stopped: string[] = [];
  const closed: number[] = [];
  let contexts = 0;
  const stream = (id: string) =>
    ({
      getTracks: () => [{ stop: () => stopped.push(id) }],
      getAudioTracks: () => [{ getSettings: () => ({ deviceId: id }) }],
    }) as unknown as MediaStream;
  vi.stubGlobal("navigator", {
    mediaDevices: {
      getUserMedia: () => new Promise<MediaStream>((resolve) => requests.push(resolve)),
      enumerateDevices: async () => [],
    },
  });
  vi.stubGlobal(
    "AudioContext",
    class {
      readonly id = contexts++;
      state = "running";
      createAnalyser() {
        return { fftSize: 0, getFloatTimeDomainData: () => {} };
      }
      createMediaStreamSource() {
        return { connect: () => {} };
      }
      async close() {
        closed.push(this.id);
      }
    },
  );
  return { requests, stopped, closed, stream };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("Microphone", () => {
  it("releases the stream that arrives after it was stopped", async () => {
    const devices = fakeDevices();
    const microphone = new Microphone(
      () => {},
      () => ({ sensitivity: 0.5, noiseGate: 0.3 }),
    );
    const starting = microphone.start();
    expect(microphone.status).toBe("loading");
    microphone.stop();
    devices.requests[0]?.(devices.stream("a"));
    await starting;
    expect(microphone.status).toBe("idle");
    expect(devices.stopped).toEqual(["a"]);
  });

  it("keeps only the last of the overlapping starts", async () => {
    const devices = fakeDevices();
    const microphone = new Microphone(
      () => {},
      () => ({ sensitivity: 0.5, noiseGate: 0.3 }),
    );
    const first = microphone.start();
    const second = microphone.start();
    devices.requests[0]?.(devices.stream("a"));
    devices.requests[1]?.(devices.stream("b"));
    await Promise.all([first, second]);
    expect(microphone.status).toBe("running");
    expect(microphone.deviceId).toBe("b");
    // 先に始めた方のマイクは止める (音を取る AudioContext は後の方の 1 つだけ)。
    expect(devices.stopped).toEqual(["a"]);
    expect(devices.closed).toEqual([]);
    microphone.stop();
    expect(devices.stopped).toEqual(["a", "b"]);
    expect(devices.closed).toEqual([0]);
  });
});
