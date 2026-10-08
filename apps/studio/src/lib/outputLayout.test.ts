import { describe, expect, it } from "vitest";
import type { StudioScene } from "./host";
import { backgroundCss, planScene, stageLayout, zoomFactor } from "./outputLayout";

function scene(revision: number, modelRevision: number, modulesRevision: number): StudioScene {
  return {
    kind: "scene",
    revision,
    modelRevision,
    modulesRevision,
    modelName: "avatar.ugoe",
    parameterIds: [],
    partIds: [],
    modules: [],
    lighting: null,
    transform: { x: 0, y: 0, scale: 1, rotation: 0 },
    background: { kind: "transparent" },
    pose: [],
    renderScale: 1,
  };
}

describe("planScene", () => {
  it("reloads only when the model changed", () => {
    expect(planScene(null, scene(1, 1, 1))).toBe("reload");
    const followed = { modelRevision: 1, modulesRevision: 1 };
    expect(planScene(followed, scene(2, 1, 1))).toBe("look");
    expect(planScene(followed, scene(3, 1, 3))).toBe("modules");
    expect(planScene(followed, scene(4, 4, 4))).toBe("reload");
    expect(planScene(followed, scene(5, 0, 5))).toBe("empty");
    expect(planScene({ modelRevision: 0, modulesRevision: 5 }, scene(6, 6, 6))).toBe("reload");
  });
});

describe("stageLayout", () => {
  it("fills the output when not rotated", () => {
    expect(stageLayout(1920, 1080, { x: 0.25, y: -0.1, scale: 2, rotation: 0 })).toEqual({
      width: 1920,
      height: 1080,
      left: 0,
      top: 0,
      offsetX: 480,
      offsetY: -108,
      rotation: 0,
    });
  });

  it("uses a centered square that covers the output when rotated", () => {
    const l = stageLayout(1600, 1200, { x: 0.1, y: 0, scale: 1, rotation: 30 });
    // 対角線 2000 + ずれ 160 の 2 倍。
    expect(l.width).toBe(2320);
    expect(l.height).toBe(2320);
    expect(l.left).toBe((1600 - 2320) / 2);
    expect(l.top).toBe((1200 - 2320) / 2);
    expect(l.rotation).toBe(30);
  });
});

describe("zoomFactor", () => {
  it("keeps the size relative to the output", () => {
    expect(zoomFactor([1920, 1080], [1920, 1080], [1000, 1000], 1.5)).toBeCloseTo(1.5);
    // 舞台が大きくなると、そこに合わせた分だけ縮める。
    expect(zoomFactor([1920, 1080], [2203, 2203], [1000, 1000], 1)).toBeCloseTo(1080 / 2203);
  });
});

describe("backgroundCss", () => {
  it("draws colors and images, and waits for the image", () => {
    expect(backgroundCss({ kind: "transparent" }, null)).toBe("transparent");
    expect(backgroundCss({ kind: "chroma", color: "#00ff00" }, null)).toBe("#00ff00");
    expect(backgroundCss({ kind: "image", path: "/bg.png" }, null)).toBe("transparent");
    expect(backgroundCss({ kind: "image", path: "/bg.png" }, "blob:x")).toContain('url("blob:x")');
  });
});
