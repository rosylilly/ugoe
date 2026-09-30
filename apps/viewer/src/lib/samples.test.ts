import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import type { ModelFile } from "./modelFile";
import { openSample, SAMPLES, type Sample, sampleFiles } from "./samples";

const file = (name: string): ModelFile => ({ name, bytes: new Uint8Array([name.length]) });

describe("samples", () => {
  it("lists every bundled file once, and each exists in assets/samples", () => {
    const files = sampleFiles();
    expect(new Set(files).size).toBe(files.length);
    expect(files).toContain("avatar.ugoe");
    expect(files).toContain("avatar-lit.ugoe");
    expect(files).toContain("dress-up/dress-up.ugoe");
    for (const path of files) {
      const url = new URL(`../../../../assets/samples/${path}`, import.meta.url);
      expect(existsSync(fileURLToPath(url)), path).toBe(true);
    }
  });

  it("offers a dress-up sample with a pair of modules for the same slot", () => {
    const dressUp = SAMPLES.find((s) => s.id === "dress-up");
    expect(dressUp?.modules.length).toBeGreaterThanOrEqual(2);
    expect(dressUp?.modules.filter((m) => !m.attach).length).toBeGreaterThan(0);
  });

  it("opens the model together with the modules and their attach flags", async () => {
    const sample: Sample = {
      id: "x",
      label: "x",
      description: "",
      model: "m.ugoe",
      modules: [
        { file: "a.ugoemod", attach: true },
        { file: "b.ugoemod", attach: false },
      ],
    };
    const calls: string[] = [];
    await openSample(
      sample,
      {
        async load(f, modules = []) {
          calls.push(`load ${f.name}`);
          for (const m of modules) calls.push(`module ${m.file.name} ${m.attach}`);
        },
      },
      async (path) => file(path),
    );
    expect(calls).toEqual(["load m.ugoe", "module a.ugoemod true", "module b.ugoemod false"]);
  });

  it("opens nothing when a file cannot be read", async () => {
    const calls: string[] = [];
    const target = {
      async load() {
        calls.push("load");
      },
    };
    const dressUp = SAMPLES.find((s) => s.id === "dress-up") as Sample;
    await expect(
      openSample(dressUp, target, async (path) => {
        if (path.endsWith(".ugoemod")) throw new Error(`404 ${path}`);
        return file(path);
      }),
    ).rejects.toThrow("404");
    expect(calls).toEqual([]);
  });
});
