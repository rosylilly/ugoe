// EditorDocument の読み直し (IPC は偽物)。重なった読み直しは、後から始めたものの結果を残す。
import { beforeEach, describe, expect, it, vi } from "vitest";
import { EditorDocument } from "./document.svelte";

/** 呼ばれるたびに `get_model_json` が返すもの (先頭から順に使う)。 */
let jsons: Promise<string>[] = [];

vi.mock("./ipc", () => ({
  ipc: {
    getModelJson: vi.fn(() => jsons.shift() ?? Promise.reject(new Error("no model"))),
    getModelBundle: vi.fn(async () => new Uint8Array()),
    getAnimations: vi.fn(async () => ({ motions: [], expressions: [] })),
    getPhysics: vi.fn(async () => null),
    moduleStatus: vi.fn(async () => null),
    applyCommand: vi.fn(async () => ({ warnings: [] })),
    undo: vi.fn(async () => true),
    redo: vi.fn(async () => true),
  },
}));

/** パラメータを 1 つだけ持つモデルの JSON。 */
function model(parameter: string): string {
  return JSON.stringify({
    parameters: [{ id: parameter }],
    parts: [],
    meshes: [],
    deformers: [],
    textures: [],
  });
}

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve: (value: T) => void = () => {};
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

beforeEach(() => {
  jsons = [];
});

describe("EditorDocument.refresh", () => {
  it("reads the model and counts the revision", async () => {
    jsons = [Promise.resolve(model("AngleX"))];
    const doc = new EditorDocument();
    await doc.refresh();
    expect(doc.parameters.map((p) => p.id)).toEqual(["AngleX"]);
    expect(doc.empty).toBe(true);
    expect(doc.revision).toBe(1);
    expect(doc.error).toBeNull();
  });

  it("keeps the result of the latest refresh when an older one finishes later", async () => {
    const older = deferred<string>();
    jsons = [older.promise, Promise.resolve(model("New"))];
    const doc = new EditorDocument();
    const first = doc.refresh();
    const second = doc.refresh();
    await second;
    expect(doc.parameters.map((p) => p.id)).toEqual(["New"]);
    older.resolve(model("Old"));
    await first;
    expect(doc.parameters.map((p) => p.id)).toEqual(["New"]);
    expect(JSON.parse(doc.model?.json ?? "{}").parameters[0].id).toBe("New");
    expect(doc.revision).toBe(1);
  });

  it("shows the failure of a refresh", async () => {
    const doc = new EditorDocument();
    await doc.refresh();
    expect(doc.error).toBe("no model");
    expect(doc.revision).toBe(0);
  });
});
