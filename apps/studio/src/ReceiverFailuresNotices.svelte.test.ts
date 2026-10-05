// @vitest-environment happy-dom
// 受け手が読めなかったこと (ReceiverFailuresNotices.svelte): 受け手ごとに 1 つの知らせにし、読めなかったものをすべて出す
// (T0840、docs/studio.md#受け手が読めなかったとき)。
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type { ReceiverFailure, StudioScene } from "./lib/host";
import { describeLoadFailure } from "./lib/receivers";
import ReceiverFailuresNotices from "./ReceiverFailuresNotices.svelte";

const cleanups: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
});

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
  background: { kind: "image", path: "/bg.png" },
  pose: [],
} as StudioScene;

function failure(patch: Partial<ReceiverFailure>): ReceiverFailure {
  return {
    label: "output",
    revision: 7,
    asset: "model",
    index: null,
    code: "unreadable",
    ...patch,
  };
}

function setup(failures: ReceiverFailure[]) {
  const props = $state({
    failures,
    scene: scene as StudioScene | null,
    ondismiss: (label: string) => {
      props.failures = props.failures.filter((f) => f.label !== label);
    },
  });
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(ReceiverFailuresNotices, { target, props });
  cleanups.push(() => {
    unmount(instance);
    target.remove();
  });
  flushSync();
  const notices = () =>
    [...target.querySelectorAll(".ugoe-notice")].map((n) =>
      [...n.querySelectorAll(".ugoe-notice__text p")].map((p) => p.textContent),
    );
  return { target, props, notices };
}

describe("ReceiverFailuresNotices", () => {
  it("shows every file a receiver could not read in one notice per receiver", () => {
    const output = [
      failure({ asset: "module", index: 0, code: "unavailable" }),
      failure({ asset: "module", index: 1 }),
      failure({ asset: "background", code: "unavailable" }),
    ];
    const camera = failure({ label: "virtualCamera" });
    const { notices } = setup([...output, camera]);
    expect(notices()).toEqual([
      output.map((f) => describeLoadFailure(f, scene)),
      [describeLoadFailure(camera, scene)],
    ]);
  });

  it("dismisses every failure of the receiver", () => {
    const { target, props, notices } = setup([
      failure({ asset: "module", index: 0 }),
      failure({ asset: "background" }),
      failure({ label: "browserSource:2" }),
    ]);
    target.querySelector<HTMLElement>(".ugoe-notice button")?.click();
    flushSync();
    expect(props.failures.map((f) => f.label)).toEqual(["browserSource:2"]);
    expect(notices()).toHaveLength(1);
  });
});
