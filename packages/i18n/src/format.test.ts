import { afterEach, describe, expect, it } from "vitest";
import { formatDate, formatList, formatNumber, formatRelativeTime } from "./format";
import { setLocale } from "./i18n.svelte";

afterEach(() => setLocale("system"));

describe("format", () => {
  it("uses the current locale", () => {
    setLocale("en");
    expect(formatNumber(1234.5)).toBe("1,234.5");
    expect(formatList(["A", "B", "C"])).toBe("A, B, and C");
    expect(formatRelativeTime(-3, "minute")).toBe("3 minutes ago");
    setLocale("ja");
    expect(formatList(["A", "B", "C"])).toBe("A、B、C");
    expect(formatRelativeTime(-3, "minute")).toBe("3 分前");
    expect(formatDate(Date.UTC(2026, 9, 2), { dateStyle: "long", timeZone: "UTC" })).toBe(
      "2026年10月2日",
    );
  });
});
