// @vitest-environment happy-dom
// 不意に出る確認 (外での書き換え・本体の変化) の初めのフォーカスが「後で」にあること (T0953。docs/editor.md#ダイアログ。
// docs/testing.md#エディタの部品のテスト)。
import { describe, expect, it, vi } from "vitest";
import BaseChangeDialog from "./BaseChangeDialog.svelte";
import ExternalChangeDialog from "./ExternalChangeDialog.svelte";
import type { ExternalChange } from "./ipc";
import { render } from "./viewport/tools/test/harness.svelte";

const changed: ExternalChange = { files: ["model.json"], missing: false, token: "1" };
const missing: ExternalChange = { files: [], missing: true, token: "2" };

/** 開いたダイアログの中でフォーカスを持つボタンの文言。 */
async function focusedButtonText(): Promise<string | undefined> {
  return vi.waitFor(() => {
    const active = document.activeElement;
    expect(active?.closest('[role="dialog"]')).not.toBeNull();
    expect(active?.tagName).toBe("BUTTON");
    return active?.textContent?.trim();
  });
}

describe("ExternalChangeDialog", () => {
  const handlers = {
    onautoreload: vi.fn(),
    onreload: vi.fn(),
    onoverwrite: vi.fn(),
    onsaveas: vi.fn(),
    ondismiss: vi.fn(),
  };

  it.each([
    ["no unsaved changes", changed, false],
    ["unsaved changes", changed, true],
    ["a missing project", missing, false],
  ])("focuses Later first with %s", async (_, change, modified) => {
    render(ExternalChangeDialog, {
      change,
      modified,
      saving: false,
      autoReload: false,
      ...handlers,
    });
    expect(await focusedButtonText()).toBe("Later");
  });

  it("is dismissed by the first Escape (the tooltip of Later does not take it)", async () => {
    const ondismiss = vi.fn();
    render(ExternalChangeDialog, {
      change: changed,
      modified: false,
      saving: false,
      autoReload: false,
      ...handlers,
      ondismiss,
    });
    await focusedButtonText();
    expect(document.querySelector('[role="tooltip"]')).toBeNull();
    (document.activeElement as HTMLElement).dispatchEvent(
      new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
    );
    await vi.waitFor(() => expect(ondismiss).toHaveBeenCalledOnce());
  });
});

describe("BaseChangeDialog", () => {
  it.each([
    ["a changed base", changed],
    ["a missing base", missing],
  ])("focuses Later first with %s", async (_, change) => {
    render(BaseChangeDialog, {
      change,
      base: "/work/Base.ugoeproj",
      autoReload: false,
      onautoreload: vi.fn(),
      onreload: vi.fn(),
      onlocate: vi.fn(),
      ondismiss: vi.fn(),
    });
    expect(await focusedButtonText()).toBe("Later");
  });
});
