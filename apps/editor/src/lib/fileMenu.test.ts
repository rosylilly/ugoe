// ファイルメニューの流れ (IPC とファイルダイアログは偽物)。操作を重ねないこと・外での書き換えと本体の変化の確かめ・
// 本体の問題で開けないときの直し方 (docs/editor.md#外での書き換え、docs/editor.md#本体の変化、docs/editor.md#本体の問題の直し方)。
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import type { FileDialogs } from "./fileDialogs";
import { FileMenuFlow, type FileMenuHost } from "./fileMenu.svelte";
import type {
  ApplyReport,
  ExternalChange,
  ModuleProblem,
  ModuleStatus,
  ProjectStatus,
} from "./ipc";

const { ipc } = vi.hoisted(() => ({
  ipc: {
    projectStatus: vi.fn(),
    moduleStatus: vi.fn(),
    saveProject: vi.fn(),
    checkExternalChange: vi.fn(),
    checkBaseChange: vi.fn(),
    reloadProject: vi.fn(),
    reloadBase: vi.fn(),
    openProject: vi.fn(),
    moduleOpenProblem: vi.fn(),
    openModuleWithBase: vi.fn(),
    applyCommand: vi.fn(),
  },
}));

vi.mock("./ipc", () => ({ ipc }));

const saved: ProjectStatus = { name: "Avatar", path: "/work/Avatar.ugoeproj", modified: false };

function change(token: string): ExternalChange {
  return { files: ["model.json"], missing: false, token };
}

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve: (value: T) => void = () => {};
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

/** 本体の問題 (スロットが無い)。 */
function unknownSlot(base: string): ModuleProblem {
  return {
    kind: "unknown_slot",
    base,
    slot: "Hair",
    base_slots: ["Hat"],
    collider_references: [],
  } as unknown as ModuleProblem;
}

const moduleInfo = { name: "Hair", base: "/work/Base.ugoeproj" } as unknown as ModuleStatus;

let dialogs: FileDialogs;
let host: FileMenuHost & {
  onchange: Mock<FileMenuHost["onchange"]>;
  inform: Mock<FileMenuHost["inform"]>;
  onrenamemodule: Mock<(from: string, to: string) => void>;
};

/** 状態を読み終えたファイルメニュー。 */
async function flowWith(status: ProjectStatus, module: ModuleStatus | null = null) {
  ipc.projectStatus.mockResolvedValue(status);
  ipc.moduleStatus.mockResolvedValue(module);
  const flow = new FileMenuFlow(host);
  await flow.refresh();
  return flow;
}

beforeEach(() => {
  for (const fn of Object.values(ipc)) fn.mockReset();
  ipc.checkExternalChange.mockResolvedValue(null);
  ipc.checkBaseChange.mockResolvedValue(null);
  dialogs = {
    openFile: vi.fn(async () => null),
    openDirectory: vi.fn(async () => null),
    saveFile: vi.fn(async () => null),
    defaultDirectory: vi.fn(async () => "/home"),
  };
  host = {
    onchange: vi.fn<FileMenuHost["onchange"]>(),
    inform: vi.fn<FileMenuHost["inform"]>(),
    onrenamemodule: vi.fn<(from: string, to: string) => void>(),
    applyWith: (action) => action(),
    dialogs: () => dialogs,
  };
});

describe("FileMenuFlow: saving", () => {
  it("saves once when Save is pressed again while saving", async () => {
    const flow = await flowWith(saved);
    const done = deferred<ProjectStatus>();
    ipc.saveProject.mockReturnValue(done.promise);
    const first = flow.save();
    const second = flow.save();
    await second;
    expect(flow.busy).toBe(true);
    done.resolve(saved);
    await first;
    expect(ipc.saveProject).toHaveBeenCalledOnce();
    expect(host.inform).toHaveBeenCalledOnce();
    expect(flow.busy).toBe(false);
  });

  it("asks before saving when the project changed outside", async () => {
    const flow = await flowWith({ ...saved, modified: true });
    ipc.checkExternalChange.mockResolvedValue(change("a"));
    await flow.save();
    expect(ipc.saveProject).not.toHaveBeenCalled();
    expect(flow.external?.token).toBe("a");
    expect(flow.externalOnSave).toBe(true);
  });

  it("does not check for outside changes while saving", async () => {
    const flow = await flowWith(saved);
    const done = deferred<ProjectStatus>();
    ipc.saveProject.mockReturnValue(done.promise);
    const saving = flow.save();
    await vi.waitFor(() => expect(ipc.saveProject).toHaveBeenCalled());
    ipc.checkExternalChange.mockClear();
    await flow.checkExternal();
    expect(ipc.checkExternalChange).not.toHaveBeenCalled();
    done.resolve(saved);
    await saving;
  });

  it("overwrites only once when the choice is made twice", async () => {
    const flow = await flowWith({ ...saved, modified: true });
    const done = deferred<ProjectStatus>();
    ipc.saveProject.mockReturnValue(done.promise);
    const first = flow.overwriteExternal();
    const second = flow.overwriteExternal();
    await second;
    done.resolve(saved);
    await first;
    expect(ipc.saveProject).toHaveBeenCalledExactlyOnceWith(true);
  });
});

describe("FileMenuFlow: checking for outside changes", () => {
  it("reloads once when the focus and the interval check at the same time", async () => {
    const flow = await flowWith(saved);
    flow.setAutoReload(true);
    const found = deferred<ExternalChange | null>();
    ipc.checkExternalChange.mockReturnValueOnce(found.promise);
    ipc.reloadProject.mockResolvedValue(saved);
    const focus = flow.checkExternal();
    const interval = flow.checkExternal();
    found.resolve(change("a"));
    await Promise.all([focus, interval]);
    expect(ipc.checkExternalChange).toHaveBeenCalledOnce();
    expect(ipc.reloadProject).toHaveBeenCalledOnce();
    expect(host.onchange).toHaveBeenCalledExactlyOnceWith(false, true);
    flow.setAutoReload(false);
  });

  it("asks once when the focus and the interval check at the same time", async () => {
    const flow = await flowWith({ ...saved, modified: true });
    ipc.checkExternalChange.mockResolvedValue(change("a"));
    await Promise.all([flow.checkExternal(), flow.checkExternal()]);
    expect(ipc.checkExternalChange).toHaveBeenCalledOnce();
    expect(flow.external?.token).toBe("a");
    expect(flow.externalOnSave).toBe(false);
    // 「Later」の後は同じ変更を尋ねない
    flow.dismissExternal();
    await flow.checkExternal();
    expect(flow.external).toBeNull();
  });

  it("lets a Save pressed during a check wait for the check", async () => {
    const flow = await flowWith(saved);
    const found = deferred<ExternalChange | null>();
    ipc.checkExternalChange.mockReturnValueOnce(found.promise);
    ipc.saveProject.mockResolvedValue(saved);
    const checking = flow.checkExternal();
    const saving = flow.save();
    // 確かめが終わるまで保存は始まらない
    await Promise.resolve();
    expect(ipc.checkExternalChange).toHaveBeenCalledOnce();
    expect(ipc.saveProject).not.toHaveBeenCalled();
    found.resolve(null);
    await Promise.all([checking, saving]);
    expect(ipc.saveProject).toHaveBeenCalledOnce();
  });

  it("reloads the base once when the focus and the interval check at the same time", async () => {
    const flow = await flowWith(saved, moduleInfo);
    flow.setAutoReload(true);
    ipc.checkBaseChange.mockResolvedValueOnce(change("b"));
    ipc.reloadBase.mockResolvedValue({ status: saved, problem: null });
    await Promise.all([flow.checkExternal(), flow.checkExternal()]);
    expect(ipc.reloadBase).toHaveBeenCalledOnce();
    expect(host.onchange).toHaveBeenCalledExactlyOnceWith(false, true);
    flow.setAutoReload(false);
  });

  it("shows the fixes when the changed base can't be reloaded, and does not ask again", async () => {
    const flow = await flowWith(saved, moduleInfo);
    ipc.checkBaseChange.mockResolvedValue(change("b"));
    await flow.checkExternal();
    expect(flow.baseChange?.token).toBe("b");
    ipc.reloadBase.mockResolvedValue({
      status: saved,
      problem: unknownSlot("/work/Base.ugoeproj"),
    });
    await flow.reloadBase();
    expect(flow.problem).toMatchObject({ opening: false, project: saved.path });
    flow.dismissProblem();
    await flow.checkExternal();
    expect(flow.baseChange).toBeNull();
  });
});

describe("FileMenuFlow: a module project the base keeps from opening", () => {
  it("shows the fixes and reopens with the chosen base", async () => {
    const flow = await flowWith(saved);
    const project = "/work/Hair.ugoeproj";
    vi.mocked(dialogs.openDirectory).mockResolvedValueOnce(project);
    ipc.openProject.mockRejectedValue(new Error("the base has no slot Hair"));
    ipc.moduleOpenProblem.mockResolvedValue(unknownSlot("/work/Base.ugoeproj"));
    await flow.pick("open");
    expect(flow.problem).toMatchObject({ project, opening: true, base: null });
    expect(flow.message).toBeNull();
    expect(host.onchange).not.toHaveBeenCalled();

    // 選び直した本体でも開けなければ、その本体で確かめた次の問題を出す
    vi.mocked(dialogs.openDirectory).mockResolvedValueOnce("/work/Other.ugoeproj");
    ipc.openModuleWithBase.mockRejectedValueOnce(new Error("still no slot"));
    ipc.moduleOpenProblem.mockResolvedValue(unknownSlot("/work/Other.ugoeproj"));
    await flow.pick("locateBaseProject");
    expect(ipc.openModuleWithBase).toHaveBeenLastCalledWith(
      project,
      "/work/Other.ugoeproj",
      null,
      false,
    );
    expect(ipc.moduleOpenProblem).toHaveBeenLastCalledWith(project, "/work/Other.ugoeproj");
    expect(flow.problem).toMatchObject({ opening: true, base: "/work/Other.ugoeproj" });

    // スロットを選ぶと、前に選び直した本体で開き直す
    ipc.openModuleWithBase.mockResolvedValueOnce({ ...saved, path: project, modified: true });
    ipc.moduleStatus.mockResolvedValue(moduleInfo);
    await flow.useSlot("Hat");
    expect(ipc.openModuleWithBase).toHaveBeenLastCalledWith(
      project,
      "/work/Other.ugoeproj",
      "Hat",
      false,
    );
    expect(flow.problem).toBeNull();
    expect(flow.status?.modified).toBe(true);
    expect(host.onchange).toHaveBeenCalledExactlyOnceWith(true, false);
  });

  it("shows the failure when the problem can't be found", async () => {
    const flow = await flowWith(saved);
    vi.mocked(dialogs.openDirectory).mockResolvedValueOnce("/work/Broken.ugoeproj");
    ipc.openProject.mockRejectedValue(new Error("broken project"));
    ipc.moduleOpenProblem.mockResolvedValue(null);
    await flow.pick("open");
    expect(flow.problem).toBeNull();
    expect(flow.message).toBe("broken project");
  });

  it("moves an open module to another slot without reopening it", async () => {
    const flow = await flowWith(saved, moduleInfo);
    ipc.checkBaseChange.mockResolvedValue(change("b"));
    await flow.checkExternal();
    ipc.reloadBase.mockResolvedValueOnce({
      status: saved,
      problem: unknownSlot("/work/Base.ugoeproj"),
    });
    await flow.reloadBase();
    ipc.reloadBase.mockResolvedValueOnce({ status: { ...saved, modified: true }, problem: null });
    await flow.useSlot("Hat");
    expect(ipc.reloadBase).toHaveBeenLastCalledWith(null, "Hat");
    expect(ipc.openModuleWithBase).not.toHaveBeenCalled();
    expect(flow.problem).toBeNull();
    expect(host.onchange).toHaveBeenCalledExactlyOnceWith(false, true);
  });

  it("asks before discarding changes to open the base project", async () => {
    const flow = await flowWith({ ...saved, modified: true });
    vi.mocked(dialogs.openDirectory).mockResolvedValueOnce("/work/Hair.ugoeproj");
    ipc.openProject.mockRejectedValueOnce(new Error("no slot"));
    ipc.moduleOpenProblem.mockResolvedValue(unknownSlot("/work/Base.ugoeproj"));
    // 開く前にも確かめる
    void flow.pick("open");
    expect(flow.pendingDiscard).not.toBeNull();
    flow.confirmDiscard();
    await vi.waitFor(() => expect(flow.problem).not.toBeNull());
    await vi.waitFor(() => expect(flow.busy).toBe(false));
    ipc.openProject.mockResolvedValueOnce(saved);
    flow.openBase();
    expect(flow.pendingDiscard).not.toBeNull();
    expect(ipc.openProject).toHaveBeenCalledOnce();
    flow.confirmDiscard();
    await vi.waitFor(() => expect(flow.problem).toBeNull());
    expect(ipc.openProject).toHaveBeenLastCalledWith("/work/Base.ugoeproj");
  });
});

describe("FileMenuFlow: module settings", () => {
  it("applies through the document and renames the selection before it rereads", async () => {
    const flow = await flowWith(saved, moduleInfo);
    const order: string[] = [];
    const report: ApplyReport = { warnings: [] } as unknown as ApplyReport;
    ipc.applyCommand.mockImplementation(async () => {
      order.push("apply");
      return report;
    });
    host.onrenamemodule.mockImplementation(() => order.push("rename"));
    const applyWith = vi.fn(async (action: () => Promise<ApplyReport>) => {
      const result = await action();
      order.push("refresh");
      return result;
    });
    host.applyWith = applyWith;
    ipc.projectStatus.mockResolvedValue({ ...saved, modified: true });
    flow.openSettings();
    const command = {
      op: "set_module_settings",
      name: "Bangs",
      slot: "Hair",
      requires: { avatar_level: null, slots: [], parameters: [] },
      keep_look: false,
    } as const;
    await flow.applyModuleSettings(command as never);
    expect(applyWith).toHaveBeenCalledWith(expect.any(Function), { rethrow: true });
    expect(order).toEqual(["apply", "rename", "refresh"]);
    expect(host.onrenamemodule).toHaveBeenCalledWith("Hair", "Bangs");
    expect(flow.settingsOpen).toBe(false);
    expect(flow.status?.modified).toBe(true);
  });

  it("returns the failure to the dialog and keeps it open", async () => {
    const flow = await flowWith(saved, moduleInfo);
    ipc.applyCommand.mockRejectedValue(new Error("bad name"));
    flow.openSettings();
    await expect(
      flow.applyModuleSettings({ op: "set_module_settings", name: "Hair" } as never),
    ).rejects.toThrow("bad name");
    expect(flow.settingsOpen).toBe(true);
    expect(flow.busy).toBe(false);
    expect(host.onrenamemodule).not.toHaveBeenCalled();
  });
});
