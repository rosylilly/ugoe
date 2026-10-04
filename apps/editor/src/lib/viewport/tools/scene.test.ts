import { describe, expect, it } from "vitest";
import type { EditorDocument } from "../../document.svelte";
import type { ViewportController } from "../controller.svelte";
import type { WebPuppet } from "../wasm";
import { ViewportScene } from "./scene.svelte";

const mesh = (id: string) => ({
  id,
  name: id,
  texture: 0,
  uvs: [
    [0, 0],
    [1, 0],
    [0, 1],
  ],
  indices: [0, 1, 2],
  blend: "normal",
  culling: false,
  masks: [],
  mask_mode: "and",
  mask_only: false,
});

/** 3 つのメッシュの偽の WebPuppet。`drawable_ids` を呼んだ回数を数える。 */
function setup() {
  let idCalls = 0;
  const puppet = {
    drawable_ids: () => {
      idCalls += 1;
      return ["A", "B", "C"];
    },
    drawable_positions: () => new Float32Array([0, 0, 10, 0, 0, 10]),
    hit_test_all: () => new Uint32Array([2, 0]),
  } as unknown as WebPuppet;
  const controller = {
    puppet,
    status: "ready",
    transform: { scale: 2, offsetX: 1, offsetY: 0 },
    screenToCanvas: (x: number, y: number) => [x, y],
  } as unknown as ViewportController;
  const doc = {
    model: {
      json: JSON.stringify({ meshes: ["A", "B", "C"].map(mesh), textures: [] }),
      bundle: null,
    },
  } as unknown as EditorDocument;
  return { scene: new ViewportScene(controller, doc), calls: () => idCalls };
}

describe("ViewportScene", () => {
  it("reads the evaluated positions on the screen", () => {
    const { scene } = setup();
    expect(scene.screenPositions("B")).toEqual([1, 0, 21, 0, 1, 20]);
    expect(scene.screenPositions("Missing")).toBeNull();
    expect(scene.hitAll(0, 0)).toEqual(["C", "A"]);
  });

  it("asks the WebPuppet for the drawable IDs once until it is invalidated", () => {
    const { scene, calls } = setup();
    for (const id of ["A", "B", "C"]) scene.canvasPositions(id);
    scene.hitAll(0, 0);
    expect(calls()).toBe(1);
    scene.invalidate();
    scene.canvasPositions("A");
    expect(calls()).toBe(2);
  });
});
