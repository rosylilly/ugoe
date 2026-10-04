import { describe, expect, it, vi } from "vitest";
import type { ApprovalRequest } from "../ipc";
import { AgentBridge } from "./bridge.svelte";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => true) }));

function request(id: number): ApprovalRequest {
  return {
    id,
    action: "apply",
    client: "claude-code",
    proposal: 1,
    title: "Add AngleX",
    excluded: [],
    bundle: null,
    timeout_secs: 120,
  };
}

describe("AgentBridge", () => {
  it("queues approvals and drops them when they close or are answered", async () => {
    const bridge = new AgentBridge();
    const hooks = { document: vi.fn(), proposals: vi.fn() };
    bridge.handle({ kind: "approval", request: request(1) }, hooks);
    bridge.handle({ kind: "approval", request: request(2) }, hooks);
    bridge.handle({ kind: "approval", request: request(1) }, hooks);
    expect(bridge.approvals.map((r) => r.id)).toEqual([1, 2]);
    bridge.handle({ kind: "approval_closed", id: 1 }, hooks);
    expect(bridge.approvals.map((r) => r.id)).toEqual([2]);
    await bridge.answer(2, true);
    expect(bridge.approvals).toEqual([]);
  });

  it("remembers when each approval arrived (the queued ones count down from then)", async () => {
    const bridge = new AgentBridge();
    const hooks = { document: vi.fn(), proposals: vi.fn() };
    const now = vi.spyOn(Date, "now").mockReturnValue(1_000);
    try {
      bridge.handle({ kind: "approval", request: request(1) }, hooks);
      now.mockReturnValue(5_000);
      bridge.handle({ kind: "approval", request: request(2) }, hooks);
      now.mockReturnValue(9_000);
      await bridge.answer(1, false);
      expect(bridge.receivedAt(2)).toBe(5_000);
      expect(bridge.receivedAt(1)).toBeNull();
    } finally {
      now.mockRestore();
    }
  });

  it("passes document and proposal changes on", () => {
    const bridge = new AgentBridge();
    const hooks = { document: vi.fn(), proposals: vi.fn() };
    bridge.handle({ kind: "document" }, hooks);
    bridge.handle({ kind: "proposals" }, hooks);
    expect(hooks.document).toHaveBeenCalledOnce();
    expect(hooks.proposals).toHaveBeenCalledOnce();
  });
});
