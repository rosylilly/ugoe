import { describe, expect, it } from "vitest";
import { acceptsToolKey } from "./keys";

/** `closest` が `selector` のどれかに当たるかだけを返す偽の要素。 */
function at(matches: string | null): EventTarget {
  return {
    closest: (selector: string) => (matches !== null && selector.includes(matches) ? {} : null),
  } as unknown as EventTarget;
}

describe("acceptsToolKey", () => {
  it("accepts keys on the page and the viewport", () => {
    expect(acceptsToolKey(at(null))).toBe(true);
    expect(acceptsToolKey(null)).toBe(true);
  });

  it("ignores keys in text fields and dialogs", () => {
    expect(acceptsToolKey(at("input"))).toBe(false);
    expect(acceptsToolKey(at("[contenteditable]"))).toBe(false);
    expect(acceptsToolKey(at('[role="dialog"]'))).toBe(false);
    expect(acceptsToolKey(at('[role="alertdialog"]'))).toBe(false);
  });
});
