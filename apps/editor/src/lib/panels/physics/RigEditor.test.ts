// @vitest-environment happy-dom
// リグの編集: 数値の欄が物理の検証 (docs/spec/model.md#物理の検証) の範囲の外の値を送らないこと
// (docs/testing.md#エディタの部品のテスト)。
import { flushSync } from "svelte";
import { describe, expect, it } from "vitest";
import type { Command, Model, PhysicsRig, PhysicsSettings } from "../../ipc";
import { render } from "../../viewport/tools/test/harness.svelte";
import { newRig } from "./physics";
import RigEditor from "./RigEditor.svelte";

const model = { parameters: [], deformers: [], meshes: [] } as unknown as Model;

function setup() {
  const rig = newRig(null, "Hair");
  const settings: PhysicsSettings = {
    gravity: [0, 1],
    wind: [0, 0],
    fps: 60,
    rigs: [rig],
    colliders: [],
  };
  const commands: Command[] = [];
  const send = async (build: (s: PhysicsSettings | null) => Command | null) => {
    const command = build(settings);
    if (command) commands.push(command);
    return true;
  };
  const { target } = render(RigEditor, { rig, model, settings, send, busy: false });
  return { target, commands };
}

/** aria-labelledby が `suffix` で終わる数値の欄。 */
function field(target: HTMLElement, suffix: string): HTMLInputElement {
  const input = target.querySelector<HTMLInputElement>(`input[aria-labelledby$="${suffix}"]`);
  if (!input) throw new Error(`no field ${suffix}`);
  return input;
}

/** 欄に文字を入れて Enter で確定する。 */
function enter(input: HTMLInputElement, text: string): void {
  input.focus();
  input.value = text;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  flushSync();
  input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  flushSync();
}

/** 最後に送った粒子の鎖。 */
function sentParticles(commands: Command[]): PhysicsRig["particles"] {
  const command = commands.at(-1);
  if (command?.op !== "set_physics_particles") throw new Error(`unexpected ${command?.op}`);
  return command.particles;
}

describe("RigEditor particle bounds", () => {
  it.each([
    ["delay", "-0.5", 0],
    ["acceleration", "-2", 0],
    ["radius", "-10", 0],
    ["mobility", "-0.1", 0],
    ["mobility", "1.5", 1],
  ] as const)("clamps %s %s to %d", (name, text, expected) => {
    const { target, commands } = setup();
    const input = field(target, `-p1-${name}`);
    enter(input, text);
    expect(input.value).toBe(String(expected));
    expect(sentParticles(commands)[1]?.[name]).toBe(expected);
  });

  it("sends nothing when the clamped value is the current one", () => {
    const { target, commands } = setup();
    // 新しいリグの粒子の衝突の半径は 0
    const input = field(target, "-p1-collision_radius");
    enter(input, "-1");
    expect(input.value).toBe("0");
    expect(commands).toHaveLength(0);
  });

  it("leaves the position unbounded", () => {
    const { target, commands } = setup();
    enter(field(target, "-p1-x"), "-40");
    expect(sentParticles(commands)[1]?.position).toEqual([-40, 30]);
  });

  it("exposes the bounds to assistive technology", () => {
    const { target } = setup();
    const mobility = field(target, "-p1-mobility");
    expect(mobility.getAttribute("aria-valuemin")).toBe("0");
    expect(mobility.getAttribute("aria-valuemax")).toBe("1");
    expect(field(target, "-p1-delay").getAttribute("aria-valuemin")).toBe("0");
    expect(field(target, "-p1-x").hasAttribute("aria-valuemin")).toBe(false);
  });
});

describe("RigEditor normalization bounds", () => {
  /** 最後に送った正規化の範囲。 */
  function sentRange(commands: Command[]) {
    const command = commands.at(-1);
    if (command?.op !== "set_physics_normalization") throw new Error(`unexpected ${command?.op}`);
    return command.normalization;
  }

  /** 正規化の範囲 `which` の `name` の欄。 */
  function rangeField(target: HTMLElement, which: string, name: string): HTMLInputElement {
    const selector = `input[aria-labelledby*="-range-${which} "][aria-labelledby$="-range-${name}"]`;
    const input = target.querySelector<HTMLInputElement>(selector);
    if (!input) throw new Error(`no field ${which} ${name}`);
    return input;
  }

  // 既定の範囲は min -10・default 0・max 10
  it.each([
    ["min", "5", 0],
    ["default", "20", 10],
    ["default", "-20", -10],
    ["max", "-5", 0],
  ] as const)("clamps %s %s to %d by the other two", (name, text, expected) => {
    const { target, commands } = setup();
    enter(rangeField(target, "angle", name), text);
    const range = sentRange(commands);
    expect(range.angle[name]).toBe(expected);
    expect(range.angle.min).toBeLessThanOrEqual(range.angle.default);
    expect(range.angle.default).toBeLessThanOrEqual(range.angle.max);
  });
});
