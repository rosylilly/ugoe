import { showToast } from "@ugoe/ui";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ApprovalRequest } from "../ipc";
import { AgentBridge } from "./bridge.svelte";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => true) }));
vi.mock("@ugoe/ui", () => ({ showToast: vi.fn() }));

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
  afterEach(() => {
    vi.mocked(showToast).mockClear();
  });

  it("queues approvals and drops them when they close or are answered", async () => {
    const bridge = new AgentBridge();
    const hooks = { document: vi.fn(), proposals: vi.fn() };
    bridge.handle({ kind: "approval", request: request(1) }, hooks);
    bridge.handle({ kind: "approval", request: request(2) }, hooks);
    bridge.handle({ kind: "approval", request: request(1) }, hooks);
    expect(bridge.approvals.map((r) => r.id)).toEqual([1, 2]);
    bridge.handle({ kind: "approval_closed", id: 1, outcome: "timed_out" }, hooks);
    expect(bridge.approvals.map((r) => r.id)).toEqual([2]);
    await bridge.answer(2, true);
    expect(bridge.approvals).toEqual([]);
    bridge.handle({ kind: "approval_closed", id: 2, outcome: "approved" }, hooks);
    expect(showToast).not.toHaveBeenCalled();
  });

  it("tells the user when the agent cancels a request it is waiting on", () => {
    const bridge = new AgentBridge();
    const hooks = { document: vi.fn(), proposals: vi.fn() };
    bridge.handle({ kind: "approval", request: request(1) }, hooks);
    bridge.handle(
      { kind: "approval", request: { ...request(2), action: "save", client: null } },
      hooks,
    );
    bridge.handle({ kind: "approval_closed", id: 1, outcome: "cancelled" }, hooks);
    bridge.handle({ kind: "approval_closed", id: 2, outcome: "cancelled" }, hooks);
    expect(bridge.approvals).toEqual([]);
    expect(vi.mocked(showToast).mock.calls).toEqual([
      [
        "claude-code cancelled its request to apply a proposal. Nothing was applied.",
        { tone: "warning" },
      ],
      ["An agent cancelled its request to save. Nothing was saved.", { tone: "warning" }],
    ]);
    // 閉じた後の知らせ (答えた後など) は重ねて出さない。
    bridge.handle({ kind: "approval_closed", id: 1, outcome: "cancelled" }, hooks);
    expect(showToast).toHaveBeenCalledTimes(2);
  });

  it("does not report declines or timeouts as cancellations", () => {
    const bridge = new AgentBridge();
    const hooks = { document: vi.fn(), proposals: vi.fn() };
    bridge.handle({ kind: "approval", request: request(1) }, hooks);
    bridge.handle({ kind: "approval", request: request(2) }, hooks);
    bridge.handle({ kind: "approval_closed", id: 1, outcome: "declined" }, hooks);
    bridge.handle({ kind: "approval_closed", id: 2, outcome: "timed_out" }, hooks);
    expect(bridge.approvals).toEqual([]);
    expect(showToast).not.toHaveBeenCalled();
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
