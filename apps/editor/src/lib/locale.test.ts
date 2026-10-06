import { describe, expect, it } from "vitest";
import { systemLocale } from "./locale";

describe("locale preference", () => {
  it("leaves the OS language to navigator.languages outside Tauri", async () => {
    await expect(systemLocale()).resolves.toBeNull();
  });
});
