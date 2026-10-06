// ProposalsView (IPC は偽物): プレビューを作る間にドキュメントが変わったら古い姿を出さないこと、
// 提案と送信の確認はドキュメントを変えず、採用だけがドキュメントの適用を通ること。AI に送っている間は取り消せること。
import { announce } from "@ugoe/ui";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { EditorDocument } from "../document.svelte";
import type { AiReply, ConsentAnswer, ProposalPreview, ProposalState } from "../ipc";
import { consentPrompt } from "./ai.svelte";
import { ProposalsView } from "./proposals.svelte";

/** 次の `preview_proposal` の答え。 */
let nextPreview: Promise<ProposalPreview>;
const cleared = vi.fn(async () => {});
/** 送ったリグの提案の求め (同意の有無)。 */
const proposed = vi.fn(
  async (_useAi: boolean, _instructions: string | null, consent: ConsentAnswer | null) =>
    (consent
      ? { status: "done", value: {} }
      : { status: "needs_consent", disclosure: {} }) as AiReply<unknown>,
);
const adopted = vi.fn(async (_id: number, _excluded: number[]) => ({ warnings: [] }));
const cancelled = vi.fn(async () => 1);
const state = { proposals: [], psd: "/a.psd", classification: {} } as unknown as ProposalState;

vi.mock("../ipc", () => ({
  ipc: {
    previewProposal: vi.fn(() => nextPreview),
    getProposalPreviewBundle: vi.fn(async () => new Uint8Array()),
    clearProposalPreview: () => cleared(),
    proposeRigFromPsd: (
      useAi: boolean,
      instructions: string | null,
      consent: ConsentAnswer | null,
    ) => proposed(useAi, instructions, consent),
    adoptProposal: (id: number, excluded: number[]) => adopted(id, excluded),
    getProposals: vi.fn(async () => state),
    getNormalMapGroups: vi.fn(async () => []),
    cancelAiRequests: () => cancelled(),
  },
}));

vi.mock("@ugoe/ui", async (original) => ({
  ...(await original<typeof import("@ugoe/ui")>()),
  announce: vi.fn(),
}));

const preview = (): ProposalPreview =>
  ({ model_json: JSON.stringify({ parameters: [] }) }) as unknown as ProposalPreview;
/** 見分けの付くプレビュー (`name` をモデルの JSON に入れる)。 */
const previewOf = (name: string): ProposalPreview =>
  ({ model_json: JSON.stringify({ name, parameters: [] }) }) as unknown as ProposalPreview;

beforeEach(() => {
  cleared.mockClear();
  proposed.mockClear();
  adopted.mockClear();
  cancelled.mockClear();
});

describe("ProposalsView preview", () => {
  it("shows the preview of the current document", async () => {
    nextPreview = Promise.resolve(preview());
    const doc = { revision: 1 } as EditorDocument;
    const view = new ProposalsView(doc);
    await view.showPreview(3);
    expect(view.preview?.id).toBe(3);
    expect(cleared).not.toHaveBeenCalled();
  });

  it("drops a preview of a document that changed while it was prepared", async () => {
    let resolve: (p: ProposalPreview) => void = () => {};
    nextPreview = new Promise((r) => {
      resolve = r;
    });
    const doc = { revision: 1 } as EditorDocument;
    const view = new ProposalsView(doc);
    const shown = view.showPreview(3);
    // エージェントの適用・Undo などでドキュメントが読み直された
    doc.revision = 2;
    resolve(preview());
    await shown;
    expect(view.preview).toBeNull();
    expect(cleared).toHaveBeenCalledOnce();
  });

  it("keeps the latest preview when an earlier one finishes later", async () => {
    // プレビューの IPC はメインのスレッドの外で並んで動くので、先に始めたものが後で終わることがある
    const pending: ((p: ProposalPreview) => void)[] = [];
    const later = () =>
      new Promise<ProposalPreview>((r) => {
        pending.push(r);
      });
    const doc = { revision: 1 } as EditorDocument;
    const view = new ProposalsView(doc);
    nextPreview = later();
    const first = view.showPreview(3);
    nextPreview = later();
    const second = view.showPreview(3);
    pending[1]?.(previewOf("second"));
    await second;
    pending[0]?.(previewOf("first"));
    await first;
    expect(view.preview?.model.json).toBe(previewOf("second").model_json);
  });

  it("does not show a preview that was ended while it was prepared", async () => {
    let resolve: (p: ProposalPreview) => void = () => {};
    nextPreview = new Promise((r) => {
      resolve = r;
    });
    const view = new ProposalsView({ revision: 1 } as EditorDocument);
    const shown = view.showPreview(3);
    view.endPreview();
    resolve(preview());
    await shown;
    expect(view.preview).toBeNull();
  });
});

describe("ProposalsView approval", () => {
  /** 適用は `applyWith` だけを通る偽のドキュメント。 */
  function fakeDoc() {
    const applyWith = vi.fn(async (action: () => Promise<{ warnings: [] }>) => action());
    return { doc: { revision: 1, applyWith } as unknown as EditorDocument, applyWith };
  }

  it("does not send or apply when the user declines to send", async () => {
    const { doc, applyWith } = fakeDoc();
    const view = new ProposalsView(doc);
    view.value = state;
    view.useAi = true;
    const ask = vi.spyOn(consentPrompt, "ask").mockResolvedValue(null);
    try {
      await view.propose();
    } finally {
      ask.mockRestore();
    }
    expect(proposed).toHaveBeenCalledOnce();
    expect(proposed.mock.calls[0]?.[2]).toBeNull();
    expect(adopted).not.toHaveBeenCalled();
    expect(applyWith).not.toHaveBeenCalled();
  });

  it("only proposes when the user agrees, and applies on adopt with the excluded commands", async () => {
    const { doc, applyWith } = fakeDoc();
    const view = new ProposalsView(doc);
    view.value = state;
    view.useAi = true;
    const ask = vi
      .spyOn(consentPrompt, "ask")
      .mockImplementation(async (disclosure) => ({ disclosure, remember: false }));
    try {
      await view.propose();
    } finally {
      ask.mockRestore();
    }
    expect(proposed).toHaveBeenCalledTimes(2);
    expect(applyWith).not.toHaveBeenCalled();
    view.excluded = { 3: new Set([2, 1]) };
    await view.adopt(3);
    expect(applyWith).toHaveBeenCalledOnce();
    expect(adopted).toHaveBeenCalledWith(3, [1, 2]);
  });
});

describe("ProposalsView cancel", () => {
  it("cancels while sending and reports it as a notice, not an error", async () => {
    const view = new ProposalsView({ revision: 1 } as EditorDocument);
    view.value = state;
    view.useAi = true;
    let reject: (e: unknown) => void = () => {};
    proposed.mockImplementationOnce(
      () =>
        new Promise((_, r) => {
          reject = r;
        }),
    );
    const running = view.propose();
    await vi.waitFor(() => expect(proposed).toHaveBeenCalledOnce());
    expect(view.sending).toBe(true);
    await view.cancel();
    expect(cancelled).toHaveBeenCalledOnce();
    expect(view.cancelling).toBe(true);
    // 取り消すのは一度だけ
    await view.cancel();
    expect(cancelled).toHaveBeenCalledOnce();
    reject({ code: "ai.cancelled", message: "the request was cancelled", args: {} });
    await running;
    expect(view.error).toBeNull();
    expect(view.notice).not.toBeNull();
    expect(view.sending).toBe(false);
    expect(view.cancelling).toBe(false);
    expect(view.busy).toBeNull();
  });

  it("remembers when the request was sent, and starts again after the confirmation", async () => {
    const view = new ProposalsView({ revision: 1 } as EditorDocument);
    view.value = state;
    view.useAi = true;
    let agree: (a: ConsentAnswer | null) => void = () => {};
    const ask = vi.spyOn(consentPrompt, "ask").mockImplementation(
      (disclosure) =>
        new Promise((r) => {
          agree = (a) => r(a ?? { disclosure, remember: false });
        }),
    );
    let finish: (r: AiReply<unknown>) => void = () => {};
    proposed.mockImplementationOnce(
      async () => ({ status: "needs_consent", disclosure: {} }) as AiReply<unknown>,
    );
    proposed.mockImplementationOnce(
      () =>
        new Promise((r) => {
          finish = r;
        }),
    );
    const now = vi.spyOn(Date, "now").mockReturnValue(1000);
    try {
      expect(view.sentAt).toBeNull();
      const running = view.propose();
      await vi.waitFor(() => expect(ask).toHaveBeenCalledOnce());
      expect(view.sentAt).toBe(1000);
      // 確認に答えて送り直したら、その時刻から数える
      now.mockReturnValue(5000);
      agree(null);
      await vi.waitFor(() => expect(proposed).toHaveBeenCalledTimes(2));
      expect(view.sentAt).toBe(5000);
      finish({ status: "done", value: {} } as AiReply<unknown>);
      await running;
      expect(view.sentAt).toBeNull();
    } finally {
      ask.mockRestore();
      now.mockRestore();
    }
  });

  it("cannot cancel what does not send to the AI", async () => {
    const view = new ProposalsView({ revision: 1 } as EditorDocument);
    view.value = state;
    view.useAi = false;
    proposed.mockImplementationOnce(
      async () => ({ status: "done", value: {} }) as AiReply<unknown>,
    );
    const running = view.propose();
    expect(view.sending).toBe(false);
    await view.cancel();
    await running;
    expect(cancelled).not.toHaveBeenCalled();
  });
});

describe("ProposalsView status place", () => {
  it("shows the waiting and the failure of a proposal's action on that proposal", async () => {
    const applyWith = vi.fn(async () => null);
    const doc = { revision: 1, applyWith, error: "boom" } as unknown as EditorDocument;
    const view = new ProposalsView(doc);
    view.value = state;
    const adopting = view.adopt(7);
    expect(view.scope).toBe(7);
    expect(view.busy).not.toBeNull();
    await adopting;
    expect(view.scope).toBe(7);
    expect(view.error).toBe("boom");
    // 読み直し (場所の無い操作) は、失敗を出す場所を変えない
    await view.refresh();
    expect(view.scope).toBe(7);
    // パネルの頭の操作に移る
    view.useAi = false;
    proposed.mockImplementationOnce(
      async () => ({ status: "done", value: {} }) as AiReply<unknown>,
    );
    await view.propose();
    expect(view.scope).toBeNull();
    view.dismiss();
    expect(view.error).toBeNull();
    expect(view.notice).toBeNull();
  });
});

describe("ProposalsView layer corrections", () => {
  /** `send` で組み立てたコマンドを覚える偽のドキュメント。 */
  const fakeDoc = (ok = true) => {
    const sent: unknown[] = [];
    const doc = {
      revision: 1,
      error: ok ? null : "rejected",
      send: vi.fn(async (build: () => unknown) => {
        sent.push(build());
        return ok ? { warnings: [], mirrored: [] } : null;
      }),
    } as unknown as EditorDocument;
    return { doc, sent };
  };

  it("sends a correction and an acknowledgement as one command each", async () => {
    const { doc, sent } = fakeDoc();
    const view = new ProposalsView(doc);
    await view.correctLayer(["Face", "Eye R"], "eye_white", "right");
    await view.resetLayer(["Face", "Eye R"]);
    expect(sent).toEqual([
      {
        op: "set_layer_correction",
        path: ["Face", "Eye R"],
        correction: { label: "eye_white", side: "right" },
      },
      { op: "set_layer_correction", path: ["Face", "Eye R"], correction: null },
    ]);
    expect(view.error).toBeNull();
  });

  it("shows why a correction was rejected", async () => {
    const { doc } = fakeDoc(false);
    const view = new ProposalsView(doc);
    await view.correctLayer(["x"], "eye", "left");
    expect(view.error).toBe("rejected");
  });
});

describe("ProposalsView announcements", () => {
  const listOf = (...items: [number, string][]) =>
    ({
      ...state,
      proposals: items.map(([id, title]) => ({ id, title })),
    }) as unknown as ProposalState;

  beforeEach(() => vi.mocked(announce).mockClear());

  it("reads out proposals that arrive after the first list, once each", () => {
    const view = new ProposalsView({ revision: 1 } as EditorDocument);
    // 開いたときからある提案は読まない
    view.value = listOf([1, "Face rig"]);
    expect(announce).not.toHaveBeenCalled();
    view.value = listOf([1, "Face rig"], [2, "Eye rig"]);
    expect(announce).toHaveBeenLastCalledWith("A new proposal is ready: Eye rig");
    view.value = listOf([1, "Face rig"], [2, "Eye rig"], [3, "Mouth"], [4, "Hair"]);
    expect(announce).toHaveBeenLastCalledWith("2 new proposals are ready. The first one: Mouth");
    // 採用して隠れた提案が Undo で戻っても、届いたことにはしない
    view.value = listOf([1, "Face rig"]);
    view.value = listOf([1, "Face rig"], [2, "Eye rig"]);
    expect(announce).toHaveBeenCalledTimes(2);
  });
});
