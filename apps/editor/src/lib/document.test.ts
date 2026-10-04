// EditorDocument の読み直しと編集の列 (IPC は偽物)。重なった読み直しは、後から始めたものの結果を残す。
// 編集は前の適用と読み直しを待ってから、その時点の写しで組み立てる。
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { EditorDocument, parseModelJson } from "./document.svelte";
import { type Command, type HistoryStatus, ipc } from "./ipc";

/** `numberFieldCommit` が返す印 (数値の欄の onchange の中を真似る)。 */
let committing: string | null = null;

vi.mock("@ugoe/ui", () => ({ numberFieldCommit: () => committing }));

/** 呼ばれるたびに `get_model_json` が返すもの (先頭から順に使う)。 */
let jsons: Promise<string>[] = [];

/** 戻れる数と進める数から作る履歴の状態 (上限 100)。 */
function historyOf(undo: number, redo: number, limit = 100): HistoryStatus {
  return {
    can_undo: undo > 0,
    can_redo: redo > 0,
    undo_count: undo,
    redo_count: redo,
    undo_limit: limit,
  };
}

/** `history_status` が返すもの (バックエンドの今の履歴)。 */
let backendHistory: HistoryStatus = historyOf(0, 0);

vi.mock("./ipc", () => ({
  ipc: {
    getModelJson: vi.fn(() => jsons.shift() ?? Promise.reject(new Error("no model"))),
    getModelBundle: vi.fn(async () => new Uint8Array()),
    getAnimations: vi.fn(async () => ({ motions: [], expressions: [] })),
    getPhysics: vi.fn(async () => null),
    moduleStatus: vi.fn(async () => null),
    historyStatus: vi.fn(async () => backendHistory),
    setUndoLimit: vi.fn(async () => backendHistory),
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
  committing = null;
  backendHistory = historyOf(0, 0);
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

describe("EditorDocument.parsedModel", () => {
  it("parses the model JSON once and keeps it while the JSON stays the same", async () => {
    const json = model("AngleX");
    jsons = [Promise.resolve(json), Promise.resolve(json), Promise.resolve(model("AngleY"))];
    const doc = new EditorDocument();
    expect(doc.parsedModel).toBeNull();
    await doc.refresh();
    const first = doc.parsedModel;
    expect(first?.parameters.map((p) => p.id)).toEqual(["AngleX"]);
    // 読み直しで同じ JSON なら同じオブジェクト ($effect を関係のない読み直しで走らせない)
    await doc.refresh();
    expect(doc.revision).toBe(2);
    expect(doc.parsedModel).toBe(first);
    await doc.refresh();
    expect(doc.parsedModel).not.toBe(first);
    expect(doc.parsedModel?.parameters.map((p) => p.id)).toEqual(["AngleY"]);
  });

  it("is null for a model JSON that cannot be read, and the refresh shows the failure", async () => {
    jsons = [Promise.resolve("{ broken")];
    const doc = new EditorDocument();
    await doc.refresh();
    expect(doc.parsedModel).toBeNull();
    expect(doc.error).not.toBeNull();
    expect(doc.revision).toBe(0);
  });

  it("reads only JSON objects", () => {
    expect(parseModelJson(undefined)).toBeNull();
    expect(parseModelJson("")).toBeNull();
    expect(parseModelJson("{ broken")).toBeNull();
    expect(parseModelJson("null")).toBeNull();
    expect(parseModelJson("5")).toBeNull();
    expect(parseModelJson(model("A"))?.parameters[0]?.id).toBe("A");
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

  /** 数値の欄 `field` の onchange の中で `send` を呼ぶ。 */
  function commit(doc: EditorDocument, field: string, id: string): Promise<unknown> {
    committing = field;
    try {
      return doc.send(() => ({ op: "remove_parameter", id }));
    } finally {
      committing = null;
    }
  }

  it("folds the waiting commits of one number field into one send that the backend coalesces", async () => {
    const backend = counter();
    vi.mocked(ipc.applyCommand).mockClear();
    const doc = new EditorDocument();
    const first = doc.send(() => ({ op: "remove_parameter", id: "before" }));
    // 適用中に同じ欄で 3 回確定する (矢印キーの押し続け)。
    const steps = ["1", "2", "3"].map((id) => commit(doc, "field-a", id));
    expect(doc.pending).toBe(2);
    backend.release();
    await first;
    const results = await Promise.all(steps);
    expect(results[0]).not.toBeNull();
    expect(results[1]).toBe(results[0]);
    expect(results[2]).toBe(results[0]);
    // 待っている間の確定は最後のものだけを送り、欄と種類の印を付ける。
    expect(backend.applied).toEqual([
      { op: "remove_parameter", id: "before" },
      { op: "remove_parameter", id: "3" },
    ]);
    expect(vi.mocked(ipc.applyCommand).mock.calls.map((c) => c[1])).toEqual([
      undefined,
      "field-a:remove_parameter",
    ]);
    expect(doc.pending).toBe(0);
  });

  it("does not fold commits across another edit or another field", async () => {
    const backend = counter();
    const doc = new EditorDocument();
    const edits = [
      commit(doc, "field-a", "1"),
      doc.send(() => ({ op: "remove_parameter", id: "other" })),
      commit(doc, "field-a", "2"),
      commit(doc, "field-b", "3"),
    ];
    backend.release();
    await Promise.all(edits);
    expect(backend.applied.map((c) => (c.op === "remove_parameter" ? c.id : c.op))).toEqual([
      "1",
      "other",
      "2",
      "3",
    ]);
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

describe("EditorDocument.history", () => {
  it("cannot undo or redo before the first refresh", () => {
    const doc = new EditorDocument();
    expect(doc.history).toBeNull();
    expect(doc.canUndo).toBe(false);
    expect(doc.canRedo).toBe(false);
  });

  it("reads whether undo and redo are possible after each edit, undo and redo", async () => {
    const doc = new EditorDocument();
    jsons = [Promise.resolve(model("A"))];
    await doc.refresh();
    expect([doc.canUndo, doc.canRedo]).toEqual([false, false]);

    vi.mocked(ipc.applyCommand).mockImplementationOnce(async () => {
      backendHistory = historyOf(1, 0);
      return { warnings: [], mirrored: [] };
    });
    jsons = [Promise.resolve(model("B"))];
    await doc.apply({ op: "remove_parameter", id: "A" });
    expect([doc.canUndo, doc.canRedo]).toEqual([true, false]);

    vi.mocked(ipc.undo).mockImplementationOnce(async () => {
      backendHistory = historyOf(0, 1);
      return true;
    });
    jsons = [Promise.resolve(model("A"))];
    await doc.undo();
    expect([doc.canUndo, doc.canRedo]).toEqual([false, true]);

    vi.mocked(ipc.redo).mockImplementationOnce(async () => {
      backendHistory = historyOf(1, 0);
      return true;
    });
    jsons = [Promise.resolve(model("B"))];
    await doc.redo();
    expect([doc.canUndo, doc.canRedo]).toEqual([true, false]);
  });

  it("keeps the status when an edit is refused", async () => {
    const doc = new EditorDocument();
    backendHistory = historyOf(2, 1);
    jsons = [Promise.resolve(model("A"))];
    await doc.refresh();
    vi.mocked(ipc.applyCommand).mockRejectedValueOnce(new Error("refused"));
    await doc.apply({ op: "remove_parameter", id: "missing" });
    expect(doc.history).toEqual(historyOf(2, 1));
  });

  it("sets the undo limit after the queued edits and shows the trimmed history", async () => {
    const doc = new EditorDocument();
    const order: string[] = [];
    vi.mocked(ipc.applyCommand).mockImplementationOnce(async () => {
      order.push("apply");
      backendHistory = historyOf(5, 0);
      return { warnings: [], mirrored: [] };
    });
    vi.mocked(ipc.setUndoLimit).mockImplementationOnce(async (limit) => {
      order.push(`limit ${limit}`);
      backendHistory = historyOf(3, 0, limit);
      return backendHistory;
    });
    jsons = [Promise.resolve(model("A"))];
    const edit = doc.apply({ op: "remove_parameter", id: "a" });
    const limit = doc.setUndoLimit(3);
    await Promise.all([edit, limit]);
    expect(order).toEqual(["apply", "limit 3"]);
    expect(doc.history).toEqual(historyOf(3, 0, 3));
    expect(doc.pending).toBe(0);
  });

  it("rethrows a refused undo limit", async () => {
    const doc = new EditorDocument();
    vi.mocked(ipc.setUndoLimit).mockRejectedValueOnce(new Error("out of range"));
    await expect(doc.setUndoLimit(0)).rejects.toThrow("out of range");
    expect(doc.pending).toBe(0);
  });
});
