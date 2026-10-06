// @vitest-environment happy-dom
// 提案の一覧 (ProposalsPanel.svelte) の、リグの提案に添えた分類の直し: 続いた `set_layer_correction` を 1 つのまとまり
// (「分類を残す N 件」) で出し、開くと各行 (レイヤーのパスと残すラベル・左右。T1639) が見えること。まとまりのチェックでまとめて外す・戻し (リグは残る)、
// 行のチェックで 1 つだけ外せること (T1631。docs/editor.md#ai-の提案)。IPC は偽物。
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { EditorDocument } from "../document.svelte";
import type { ProposalState } from "../ipc";
import type { ViewportController } from "../viewport/controller.svelte";
import ProposalsPanel from "./ProposalsPanel.svelte";
import { ProposalsView } from "./proposals.svelte";

/** 0–2: 添えた分類の直し (2 は直しを消す)、3–4: リグ (4 は 3 を使う)。 */
const state = {
  proposals: [
    {
      id: 1,
      title: "Rig from layers",
      rationale: "",
      commands: [
        {
          op: "set_layer_correction",
          subject: "Face / Eye L",
          requires: [],
          correction: { label: "eye_white", side: "left" },
        },
        {
          op: "set_layer_correction",
          subject: "Face / Eye R",
          requires: [],
          correction: { label: "eye_white", side: "right" },
        },
        // 直しを消すコマンド (`correction` が無い)
        { op: "set_layer_correction", subject: "Face / Mouth", requires: [] },
        { op: "add_parameter", subject: "EyeOpen", requires: [] },
        { op: "add_keyform_key", subject: "EyeOpen", requires: [3] },
      ],
      stale: false,
      source: "agent",
      revises: null,
      instructions: null,
      unapplied: [],
    },
  ],
  psd: null,
  classification: null,
  regenerable: [],
  sections: [],
  stale_rigs: [],
  restoring: false,
} as unknown as ProposalState;

/** `getProposals` の答え (テストごとに替える)。 */
let current: ProposalState = state;

vi.mock("../ipc", () => ({
  ipc: {
    getProposals: vi.fn(async () => current),
    getNormalMapGroups: vi.fn(async () => []),
    aiSettings: vi.fn(async () => {
      throw new Error("no settings");
    }),
  },
}));

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
  current = state;
});

async function setup() {
  const view = new ProposalsView({ revision: 1 } as EditorDocument);
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(ProposalsPanel, {
    target,
    props: {
      view,
      viewport: {} as ViewportController,
      onopensettings: () => {},
    },
  });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  await vi.waitFor(() => expect(view.value).not.toBeNull());
  flushSync();
  return { view, target };
}

/** `<details>` を開く (閉じている間は中を描かないので、開いてから探す)。 */
function open(details: Element | null | undefined): void {
  if (!(details instanceof HTMLDetailsElement) || details.open) return;
  details.open = true;
  details.dispatchEvent(new Event("toggle"));
  flushSync();
}

/** 一覧のまとまりの行 (`.groups > li`)。コマンドの一覧と、まとまりの中の行を開いておく。 */
function groups(target: HTMLElement): HTMLElement[] {
  open(target.querySelector(".proposal > details"));
  const rows = [...target.querySelectorAll<HTMLElement>(".groups > li")];
  for (const row of rows) open(row.querySelector("details.inner"));
  return rows;
}

/** まとまりのチェックボックス。 */
function groupBox(row: HTMLElement): HTMLInputElement {
  return row.querySelector(".group > .box > input") as HTMLInputElement;
}

/** まとまりの中の行のチェックボックス。 */
function rowBoxes(row: HTMLElement): HTMLInputElement[] {
  return [...row.querySelectorAll<HTMLInputElement>(".commands input")];
}

describe("ProposalsPanel layer corrections", () => {
  it("shows the attached corrections as one group with the kept label and side per layer", async () => {
    const { target } = await setup();
    const rows = groups(target);
    expect(rows).toHaveLength(2);
    const [labels, rig] = rows as [HTMLElement, HTMLElement];
    expect(labels.querySelector("summary")?.textContent?.trim()).toBe(
      "Keep the classification of 3 layers",
    );
    expect([...labels.querySelectorAll(".commands code")].map((c) => c.textContent)).toEqual([
      "Face / Eye L → eye_white (left)",
      "Face / Eye R → eye_white (right)",
      "Face / Mouth → back to the classification",
    ]);
    expect(rig.querySelector("summary code")?.textContent).toBe("EyeOpen");
  });

  it("leaves out and brings back all corrections at once, keeping the rig", async () => {
    const { view, target } = await setup();
    const [labels, rig] = groups(target) as [HTMLElement, HTMLElement];
    groupBox(labels).click();
    await tick();
    flushSync();
    expect([...view.excludedOf(1)].sort()).toEqual([0, 1, 2]);
    expect(groupBox(labels).checked).toBe(false);
    expect(rowBoxes(labels).map((b) => b.checked)).toEqual([false, false, false]);
    expect(groupBox(rig).checked).toBe(true);
    expect(target.textContent).toContain("2 of 5 commands selected");
    groupBox(labels).click();
    await tick();
    flushSync();
    expect(view.excludedOf(1).size).toBe(0);
  });

  it("leaves out one correction on its own", async () => {
    const { view, target } = await setup();
    const [labels] = groups(target) as [HTMLElement];
    rowBoxes(labels)[1]?.click();
    await tick();
    flushSync();
    expect([...view.excludedOf(1)]).toEqual([1]);
    expect(groupBox(labels).indeterminate).toBe(true);
  });
});

describe("ProposalsPanel proposal cards", () => {
  it("says what adopting changes and that one Undo reverts it", async () => {
    const { target } = await setup();
    const card = target.querySelector(".proposal") as HTMLElement;
    expect(card.querySelector(".changes")?.textContent?.replace(/\s+/g, " ")).toContain(
      "Adopting changes: correct the classification of 3 layers · add 1 parameter · add 1 keyform key",
    );
    expect(card.textContent).toContain(
      "Adopt applies 5 of 5 commands as one edit. One Undo reverts it.",
    );
    // 決定 (採用) は並びの最後
    const buttons = [...card.querySelectorAll(".row button")].map((b) => b.textContent?.trim());
    expect(buttons.at(-1)).toBe("Adopt");
  });

  it("shows a command kind missing from the catalog by its name", async () => {
    const [proposal] = state.proposals;
    current = {
      ...state,
      proposals: [
        {
          ...proposal,
          commands: [
            { op: "add_parameter", subject: "AngleX", requires: [] },
            { op: "add_parameter", subject: "AngleY", requires: [] },
            { op: "future_command", subject: null, requires: [] },
          ],
        },
      ],
    } as unknown as ProposalState;
    const { target } = await setup();
    const changes = target.querySelector(".proposal .changes") as HTMLElement;
    expect(changes.textContent?.replace(/\s+/g, " ")).toContain(
      "add 2 parameters · future_command × 1",
    );
    expect([...changes.querySelectorAll("code")].map((c) => c.textContent)).toEqual([
      "future_command",
    ]);
  });

  it("explains an outdated proposal and keeps a single primary button", async () => {
    const [proposal] = state.proposals;
    current = {
      ...state,
      proposals: [
        { ...proposal, id: 1, stale: true },
        { ...proposal, id: 2 },
      ],
    } as ProposalState;
    const { target } = await setup();
    const [stale, fresh] = [...target.querySelectorAll<HTMLElement>(".proposal")];
    expect(stale?.querySelector(".ugoe-notice")?.textContent).toContain("Outdated");
    expect(fresh?.querySelector(".ugoe-notice")).toBeNull();
    const primary = [...target.querySelectorAll(".ugoe-button--primary")];
    expect(primary).toHaveLength(1);
    expect(fresh?.contains(primary[0] as Node)).toBe(true);
  });

  it("shows that the saved classification is being restored", async () => {
    current = { ...state, restoring: true } as ProposalState;
    const { target } = await setup();
    expect(target.querySelector("[role=status]")?.textContent).toContain(
      "Reading the layer classification saved with this project",
    );
  });
});
