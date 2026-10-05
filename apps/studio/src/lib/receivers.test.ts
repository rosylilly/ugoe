import { describe, expect, it } from "vitest";
import type { ReceiverFailure, StudioScene } from "./host";
import { t } from "./i18n";
import { describeLoadFailure, groupLoadFailures, receiverName } from "./receivers";

const scene = {
  kind: "scene",
  revision: 7,
  modelRevision: 5,
  modulesRevision: 6,
  modelName: "avatar.ugoe",
  parameterIds: [],
  modules: ["CatEars", "Hat"],
  lighting: null,
  transform: { x: 0, y: 0, scale: 1, rotation: 0 },
  background: { kind: "transparent" },
  pose: [],
} as StudioScene;

function failure(patch: Partial<ReceiverFailure> = {}): ReceiverFailure {
  return {
    label: "output",
    revision: 7,
    asset: "model",
    index: null,
    code: "unreadable",
    ...patch,
  };
}

describe("receiverName", () => {
  it("names the output window, the virtual camera and the browser source pages", () => {
    expect(receiverName("output")).toBe(t("load.outputWindow"));
    expect(receiverName("virtualCamera")).toBe(t("load.virtualCamera"));
    expect(receiverName("browserSource")).toBe(t("load.browserSource"));
    expect(receiverName("browserSource:2")).toBe(t("load.browserSourceN", { n: 2 }));
    expect(receiverName("other")).toBe("other");
  });
});

describe("describeLoadFailure", () => {
  it("names the receiver and the file of the current scene", () => {
    expect(describeLoadFailure(failure(), scene)).toBe(
      t("receiverFailure.unreadable", {
        receiver: t("load.outputWindow"),
        asset: t("receiverFailure.model", { name: "avatar.ugoe" }),
      }),
    );
    const module = describeLoadFailure(
      failure({ label: "browserSource:2", asset: "module", index: 1, code: "unavailable" }),
      scene,
    );
    expect(module).toBe(
      t("receiverFailure.unavailable", {
        receiver: t("load.browserSourceN", { n: 2 }),
        asset: t("receiverFailure.module", { name: "Hat" }),
      }),
    );
    expect(describeLoadFailure(failure({ asset: "background" }), scene)).toContain(
      t("receiverFailure.background"),
    );
    expect(describeLoadFailure(failure({ code: "noRenderer" }), scene)).toBe(
      t("receiverFailure.noRenderer", {
        receiver: t("load.outputWindow"),
        asset: t("receiverFailure.model", { name: "avatar.ugoe" }),
      }),
    );
  });

  it("does not take names from another scene", () => {
    expect(describeLoadFailure(failure({ revision: 6 }), scene)).toBe(
      t("receiverFailure.unreadable", {
        receiver: t("load.outputWindow"),
        asset: t("receiverFailure.modelUnnamed"),
      }),
    );
    expect(describeLoadFailure(failure({ asset: "module", index: 5 }), scene)).toContain(
      t("receiverFailure.moduleUnnamed"),
    );
  });
});

describe("groupLoadFailures", () => {
  it("keeps every failure of a receiver together in the order received", () => {
    const model = failure();
    const module = failure({ asset: "module", index: 1 });
    const camera = failure({ label: "virtualCamera", asset: "background" });
    const background = failure({ asset: "background" });
    expect(groupLoadFailures([model, camera, module, background])).toEqual([
      { label: "output", failures: [model, module, background] },
      { label: "virtualCamera", failures: [camera] },
    ]);
    expect(groupLoadFailures([])).toEqual([]);
  });
});
