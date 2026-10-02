import { describe, expect, it } from "vitest";
import { apiUrl, EMPTY_API, isApiPort, pendingToShow } from "./api";

describe("api", () => {
  it("points clients at the loopback endpoint", () => {
    expect(apiUrl(28451)).toBe("ws://127.0.0.1:28451/api");
  });

  it("accepts unprivileged ports only", () => {
    expect(isApiPort(1024)).toBe(true);
    expect(isApiPort(65535)).toBe(true);
    expect(isApiPort(80)).toBe(false);
    expect(isApiPort(65536)).toBe(false);
    expect(isApiPort(3000.5)).toBe(false);
  });

  it("hides the approvals postponed with Later", () => {
    const view = {
      ...EMPTY_API,
      pending: [
        { id: "a", name: "Deck", permission: "control" as const },
        { id: "b", name: "Bot", permission: "read" as const },
      ],
    };
    expect(pendingToShow(view, new Set(["a"])).map((p) => p.id)).toEqual(["b"]);
    expect(pendingToShow(view, new Set())).toHaveLength(2);
  });
});
