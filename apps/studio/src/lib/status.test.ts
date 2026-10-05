import { describe, expect, it } from "vitest";
import { EMPTY_API } from "./api";
import type { StatusMessage } from "./host";
import { t } from "./i18n";
import {
  apiBadge,
  hotkeysBadge,
  lipSyncBadge,
  loadBadge,
  type StatusInputs,
  sourceBadge,
  statusBadges,
  trackingBadge,
} from "./status";

const idle = { status: "idle", faceFound: false, fps: 0, paused: false } as const;

function load(patch: Partial<StatusMessage> = {}): StatusMessage {
  return {
    kind: "status",
    seq: 1,
    values: [],
    tickMs: 0.2,
    outputs: 1,
    tickRate: 60,
    measuredRate: 60,
    maxIntervalMs: 17,
    renderers: [],
    ...patch,
  } as StatusMessage;
}

describe("status badges", () => {
  it("gives each tracking state its own shape", () => {
    expect(trackingBadge(idle).tone).toBe("off");
    expect(trackingBadge({ ...idle, status: "loading" }).tone).toBe("off");
    expect(trackingBadge({ ...idle, status: "error" }).tone).toBe("danger");
    expect(trackingBadge({ ...idle, status: "running", faceFound: true, fps: 30 })).toMatchObject({
      tone: "ok",
      label: t("status.tracking.face", { fps: 30 }),
    });
    expect(trackingBadge({ ...idle, status: "running" }).tone).toBe("warning");
    // 一時停止はトラッキングの様子より先に出す (動いていても止めている)。
    expect(trackingBadge({ ...idle, status: "running", faceFound: true, paused: true }).tone).toBe(
      "warning",
    );
  });

  it("shows lip sync and hotkeys", () => {
    expect(lipSyncBadge({ running: false, paused: false }).tone).toBe("off");
    expect(lipSyncBadge({ running: true, paused: false }).tone).toBe("ok");
    expect(lipSyncBadge({ running: true, paused: true }).tone).toBe("warning");
    expect(hotkeysBadge({ count: 0, problems: 0, wayland: false }).tone).toBe("off");
    expect(hotkeysBadge({ count: 3, problems: 0, wayland: false })).toMatchObject({
      tone: "ok",
      label: t("status.hotkeys.on", { count: 3 }),
    });
    expect(hotkeysBadge({ count: 3, problems: 1, wayland: false })).toMatchObject({
      tone: "warning",
      label: t("status.hotkeys.problems", { count: 1 }),
    });
    expect(hotkeysBadge({ count: 3, problems: 0, wayland: true }).tone).toBe("warning");
  });

  it("warns about load with the same numbers", () => {
    const fine = loadBadge(load());
    expect(fine.tone).toBe("ok");
    const slow = loadBadge(load({ measuredRate: 40 }));
    expect(slow.tone).toBe("warning");
    expect(slow.label).toBe(fine.label);
  });

  it("shows the API and the browser source, also when they are off", () => {
    expect(apiBadge(EMPTY_API).tone).toBe("off");
    expect(sourceBadge(EMPTY_API).tone).toBe("off");
    const on = {
      ...EMPTY_API,
      enabled: true,
      listening: true,
      source: { enabled: true, listening: true, connections: 2 },
    };
    expect(apiBadge(on).tone).toBe("ok");
    expect(sourceBadge(on).tone).toBe("ok");
    expect(apiBadge({ ...on, listening: false }).tone).toBe("danger");
    expect(
      apiBadge({ ...on, pending: [{ id: "x", name: "Deck", permission: "control" }] }),
    ).toMatchObject({ tone: "warning", label: t("status.api.pending", { count: 1 }) });
    expect(sourceBadge({ ...on, source: { ...on.source, listening: false } }).tone).toBe("danger");
  });

  it("orders the badges and leaves out load until it arrives", () => {
    const inputs: StatusInputs = {
      tracking: idle,
      lipSync: { running: false, paused: false },
      hotkeys: { count: 0, problems: 0, wayland: false },
      load: null,
      api: EMPTY_API,
    };
    expect(statusBadges(inputs).map((b) => b.id)).toEqual([
      "tracking",
      "lipSync",
      "hotkeys",
      "api",
      "source",
    ]);
    expect(statusBadges({ ...inputs, load: load() }).map((b) => b.id)).toEqual([
      "tracking",
      "lipSync",
      "hotkeys",
      "load",
      "api",
      "source",
    ]);
    // どの印にも説明 (Tooltip) がある。
    for (const b of statusBadges({ ...inputs, load: load() })) expect(b.tooltip).not.toBe("");
  });
});
