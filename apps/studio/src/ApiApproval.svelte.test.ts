// @vitest-environment happy-dom
// 操作 API の承認の知らせ (ApiApproval.svelte) を happy-dom の上で描いて確かめる。
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import ApiApproval from "./ApiApproval.svelte";
import type { ApiPendingClient, ApiPermission } from "./lib/api";
import { t } from "./lib/i18n";

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

function radio(root: Element, index: number): HTMLElement {
  const found = root.querySelectorAll<HTMLElement>('[role="radio"]')[index];
  if (found === undefined) throw new Error(`no radio ${index}`);
  return found;
}

describe("ApiApproval", () => {
  it("keeps the narrowed permission when the api view is refreshed", () => {
    const props = $state({
      client: { id: "deck", name: "Deck", permission: "control" } as ApiPendingClient,
      onapprove: (_permission: ApiPermission) => {},
      ondeny: () => {},
      onlater: () => {},
    });
    const approved: ApiPermission[] = [];
    props.onapprove = (permission) => approved.push(permission);
    const target = document.createElement("div");
    document.body.append(target);
    const instance = mount(ApiApproval, { target, props });
    cleanups.push(() => {
      unmount(instance);
      target.remove();
    });
    flushSync();
    // 求めたのは操作する権限。利用者が読むだけに狭める。
    expect(radio(target, 1).getAttribute("aria-checked")).toBe("true");
    radio(target, 0).click();
    flushSync();
    expect(radio(target, 0).getAttribute("aria-checked")).toBe("true");
    // 別のクライアントがつないだなどで、同じクライアントの様子が新しいもので届く。
    props.client = { id: "deck", name: "Deck", permission: "control" };
    flushSync();
    expect(radio(target, 0).getAttribute("aria-checked")).toBe("true");
    const approve = [...target.querySelectorAll("button")].find(
      (b) => b.textContent?.trim() === t("api.approve"),
    );
    expect(approve).toBeDefined();
    approve?.click();
    flushSync();
    expect(approved).toEqual(["read"]);
  });

  it("puts the approval last, after later and deny (the order of the editor's dialogs)", () => {
    const target = document.createElement("div");
    document.body.append(target);
    const instance = mount(ApiApproval, {
      target,
      props: {
        client: { id: "deck", name: "Deck", permission: "read" },
        onapprove: () => {},
        ondeny: () => {},
        onlater: () => {},
      },
    });
    cleanups.push(() => {
      unmount(instance);
      target.remove();
    });
    flushSync();
    const labels = [...target.querySelectorAll(".ugoe-notice__actions button")].map((b) =>
      b.textContent?.trim(),
    );
    expect(labels).toEqual([t("api.later"), t("api.deny"), t("api.approve")]);
  });
});
