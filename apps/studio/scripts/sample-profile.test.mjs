import { existsSync, readFileSync } from "node:fs";
import { isAbsolute } from "node:path";
import { PRESETS } from "@ugoe/player";
import { describe, expect, it } from "vitest";
import { SAMPLES_DIR, sampleProfile } from "./sample-profile.mjs";

/**
 * バンドル (`.ugoe`) のポーズの要素のパーツ (docs/spec/file-formats.md#バンドル-ugoe)。圧縮していない `MODL` の MessagePack から、
 * ポーズの要素 (`{ part, links }` のマップ) を拾う。
 * @param {string} path
 */
function poseParts(path) {
  const bytes = readFileSync(path);
  expect(bytes.toString("latin1", 0, 4)).toBe("UGOE");
  const count = bytes.readUInt32LE(12);
  for (let i = 0; i < count; i++) {
    const at = 16 + i * 24;
    if (bytes.toString("latin1", at, at + 4) !== "MODL") continue;
    expect(bytes.readUInt32LE(at + 4) & 1, "MODL is not compressed").toBe(0);
    const offset = Number(bytes.readBigUInt64LE(at + 8));
    const length = Number(bytes.readBigUInt64LE(at + 16));
    const model = bytes.toString("latin1", offset, offset + length);
    // 0x82 (2 つの組のマップ)・"part"・短い文字列・"links"
    const entry = /\x82\xa4part([\xa0-\xbf])/g;
    const parts = [];
    for (let m = entry.exec(model); m !== null; m = entry.exec(model)) {
      const end = entry.lastIndex + (m[1].charCodeAt(0) & 0x1f);
      if (model.startsWith("\xa5links", end)) parts.push(model.slice(entry.lastIndex, end));
    }
    return parts;
  }
  throw new Error(`${path}: no MODL chunk`);
}

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

  it("switches the dress-up scene's pose with hotkeys, including the waving hand", () => {
    const profile = sampleProfile(SAMPLES_DIR);
    const dressUp = profile.scenes.find((s) => s.id === "dress-up");
    const parts = poseParts(dressUp?.model ?? "");
    expect(parts).toEqual(["HandsDown", "Wave"]);
    const hotkeys = profile.hotkeys
      .filter((h) => h.action.kind === "pose")
      .map((h) => h.action.part);
    expect(hotkeys).toContain("Wave");
    for (const part of hotkeys) expect(parts).toContain(part);
  });

  it("uses the sunset lighting preset as it is", () => {
    const evening = sampleProfile(SAMPLES_DIR).scenes.find((s) => s.id === "evening");
    expect(evening?.lighting).toEqual(PRESETS.find((p) => p.id === "sunset")?.settings);
  });
});
