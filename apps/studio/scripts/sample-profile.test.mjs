import { existsSync } from "node:fs";
import { isAbsolute } from "node:path";
import { PRESETS } from "@ugoe/player";
import { describe, expect, it } from "vitest";
import { SAMPLES_DIR, sampleProfile } from "./sample-profile.mjs";

describe("sampleProfile", () => {
  for (const lang of ["ja", "en"]) {
    it(`points at sample files that exist (${lang})`, () => {
      const profile = sampleProfile(SAMPLES_DIR, lang);
      const files = [
        ...profile.scenes.flatMap((s) => [s.model, ...s.modules]),
        ...profile.hotkeys.flatMap((h) => (h.action.kind === "dressUp" ? [h.action.module] : [])),
      ];
      for (const file of files) {
        expect(isAbsolute(file), file).toBe(true);
        expect(existsSync(file), file).toBe(true);
      }
    });
  }

  it("refers to its own scenes and gives every hotkey a different key", () => {
    const profile = sampleProfile(SAMPLES_DIR);
    const ids = profile.scenes.map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toContain(profile.activeScene);
    for (const h of profile.hotkeys)
      if (h.action.kind === "scene") expect(ids).toContain(h.action.scene);
    const keys = profile.hotkeys.map((h) => h.keys);
    expect(new Set(keys).size).toBe(keys.length);
  });

  it("uses the sunset lighting preset as it is", () => {
    const evening = sampleProfile(SAMPLES_DIR).scenes.find((s) => s.id === "evening");
    expect(evening?.lighting).toEqual(PRESETS.find((p) => p.id === "sunset")?.settings);
  });
});
