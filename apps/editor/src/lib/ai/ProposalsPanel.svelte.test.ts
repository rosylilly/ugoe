// @vitest-environment happy-dom
// 提案の一覧 (ProposalsPanel.svelte) の、リグの提案に添えた分類の直し: 続いた `set_layer_correction` を 1 つのまとまり
// (「分類を残す N 件」) で出し、開くと各行が見えること。まとまりのチェックでまとめて外す・戻し (リグは残る)、
// 行のチェックで 1 つだけ外せること (T1631。docs/editor.md#ai-の提案)。IPC は偽物。
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { EditorDocument } from "../document.svelte";
import type { ProposalState } from "../ipc";
import type { ViewportController } from "../viewport/controller.svelte";
import ProposalsPanel from "./ProposalsPanel.svelte";
import { ProposalsView } from "./proposals.svelte";

/** 0–2: 添えた分類の直し、3–4: リグ (4 は 3 を使う)。 */
const state = {
  proposals: [
    {
      id: 1,
      title: "Rig from layers",
      rationale: "",
      commands: [
        { op: "set_layer_correction", subject: "Face / Eye L", requires: [] },
        { op: "set_layer_correction", subject: "Face / Eye R", requires: [] },
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
} as unknown as ProposalState;

vi.mock("../ipc", () => ({
  ipc: {
    getProposals: vi.fn(async () => state),
    getNormalMapGroups: vi.fn(async () => []),
    aiSettings: vi.fn(async () => {
      throw new Error("no settings");
    }),
  },
}));

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
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

/** 一覧のまとまりの行 (`.groups > li`)。 */
function groups(target: HTMLElement): HTMLElement[] {
  return [...target.querySelectorAll<HTMLElement>(".groups > li")];
}

/** まとまりのチェックボックス。 */
function groupBox(row: HTMLElement): HTMLInputElement {
  return row.querySelector(".group > input") as HTMLInputElement;
}

/** まとまりの中の行のチェックボックス。 */
function rowBoxes(row: HTMLElement): HTMLInputElement[] {
  return [...row.querySelectorAll<HTMLInputElement>(".commands input")];
}

describe("ProposalsPanel layer corrections", () => {
  it("shows the attached corrections as one group with a row per layer", async () => {
    const { target } = await setup();
    const rows = groups(target);
    expect(rows).toHaveLength(2);
    const [labels, rig] = rows as [HTMLElement, HTMLElement];
    expect(labels.querySelector("summary")?.textContent?.trim()).toBe(
      "Keep the classification of 3 layers",
    );
    expect([...labels.querySelectorAll(".commands code")].map((c) => c.textContent)).toEqual([
      "set_layer_correction Face / Eye L",
      "set_layer_correction Face / Eye R",
      "set_layer_correction Face / Mouth",
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
