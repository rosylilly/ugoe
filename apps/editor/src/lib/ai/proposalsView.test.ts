// ProposalsView (IPC は偽物): プレビューを作る間にドキュメントが変わったら古い姿を出さないこと、
// 提案と送信の確認はドキュメントを変えず、採用だけがドキュメントの適用を通ること。
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
  },
}));

const preview = (): ProposalPreview =>
  ({ model_json: JSON.stringify({ parameters: [] }) }) as unknown as ProposalPreview;

beforeEach(() => {
  cleared.mockClear();
  proposed.mockClear();
  adopted.mockClear();
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
