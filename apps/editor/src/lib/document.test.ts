// EditorDocument の読み直しと編集の列 (IPC は偽物)。重なった読み直しは、後から始めたものの結果を残す。
// 編集は前の適用と読み直しを待ってから、その時点の写しで組み立てる。
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { EditorDocument } from "./document.svelte";
import { type Command, ipc } from "./ipc";

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

describe("EditorDocument.applyWith", () => {
  it("shows the failure in the document", async () => {
    const doc = new EditorDocument();
    const report = await doc.applyWith(() => Promise.reject(new Error("refused")));
    expect(report).toBeNull();
    expect(doc.error).toBe("refused");
  });

  it("returns the failure to the caller with rethrow", async () => {
    const doc = new EditorDocument();
    await expect(
      doc.applyWith(() => Promise.reject(new Error("refused")), { rethrow: true }),
    ).rejects.toThrow("refused");
    expect(doc.error).toBeNull();
  });
});

describe("EditorDocument.send", () => {
  /** 適用のたびに 1 つ数を増やすバックエンドの偽物 (読み直すと今の数が `parameters` の ID になる)。 */
  function counter(): { applied: Command[]; release: () => void } {
    const applied: Command[] = [];
    const gate = deferred<void>();
    vi.mocked(ipc.applyCommand).mockImplementation(async (command) => {
      // 最初の適用だけ、release されるまで終わらない (適用中に次の編集が来る)。
      if (applied.length === 0) await gate.promise;
      applied.push(command);
      jsons.push(Promise.resolve(model(String(applied.length))));
      return { warnings: [], mirrored: [] };
    });
    return {
      applied,
      release: () => gate.resolve(),
    };
  }

  afterEach(() => {
    vi.mocked(ipc.applyCommand).mockImplementation(async () => ({ warnings: [], mirrored: [] }));
  });

  /** 今のパラメータの ID を名前にして消すコマンド (組み立てた時点の写しが分かる)。 */
  function removeCurrent(doc: EditorDocument): Command {
    return { op: "remove_parameter", id: doc.parameters[0]?.id ?? "none" };
  }

  it("waits for the edit in flight and builds the next command from the refreshed model", async () => {
    const backend = counter();
    const doc = new EditorDocument();
    const first = doc.send(() => removeCurrent(doc));
    const second = doc.send(() => removeCurrent(doc));
    expect(doc.pending).toBe(2);
    backend.release();
    expect(await first).not.toBeNull();
    expect(await second).not.toBeNull();
    // 2 つ目は捨てられず、1 つ目を読み直した後の写し ("1") から組み立てる。
    expect(backend.applied).toEqual([
      { op: "remove_parameter", id: "none" },
      { op: "remove_parameter", id: "1" },
    ]);
    expect(doc.parameters.map((p) => p.id)).toEqual(["2"]);
    expect(doc.pending).toBe(0);
  });

  it("keeps the order with apply, undo and redo", async () => {
    const backend = counter();
    const doc = new EditorDocument();
    const order: string[] = [];
    vi.mocked(ipc.undo).mockImplementationOnce(async () => {
      order.push(`undo after ${backend.applied.length}`);
      return true;
    });
    const edit = doc.apply({ op: "remove_parameter", id: "a" });
    const undo = doc.undo();
    backend.release();
    await Promise.all([edit, undo]);
    expect(order).toEqual(["undo after 1"]);
  });

  it("sends nothing when the builder returns null", async () => {
    const doc = new EditorDocument();
    vi.mocked(ipc.applyCommand).mockClear();
    expect(await doc.send(() => null)).toBeNull();
    expect(ipc.applyCommand).not.toHaveBeenCalled();
    expect(doc.error).toBeNull();
  });

  it("reports a failure through the return value and goes on with the next edit", async () => {
    const doc = new EditorDocument();
    vi.mocked(ipc.applyCommand).mockRejectedValueOnce(new Error("refused"));
    jsons = [Promise.resolve(model("After"))];
    const failed = doc.send(() => ({ op: "remove_parameter", id: "a" }));
    const next = doc.send(() => ({ op: "remove_parameter", id: "b" }));
    expect(await failed).toBeNull();
    expect(await next).not.toBeNull();
    expect(doc.parameters.map((p) => p.id)).toEqual(["After"]);
    expect(doc.error).toBeNull();
  });

  it("shows an error thrown by the builder", async () => {
    const doc = new EditorDocument();
    const report = await doc.send(() => {
      throw new Error("no such rig");
    });
    expect(report).toBeNull();
    expect(doc.error).toBe("no such rig");
  });
});
