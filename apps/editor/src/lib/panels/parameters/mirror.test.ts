import { describe, expect, it } from "vitest";
import type { Parameter } from "../../generated/types";
import { mirrorOptions, setMirrorCommand, suggestedMirror, swapSide } from "./mirror";

const param = (id: string, name = id): Parameter => ({
  id,
  name,
  min: 0,
  max: 1,
  default: 1,
  kind: "normal",
  mirror: null,
  group: null,
});

describe("parameter mirror", () => {
  it("offers none, itself and the other parameters", () => {
    const options = mirrorOptions([param("EyeLOpen", "左目"), param("EyeROpen")], "EyeLOpen");
    expect(options).toEqual([
      { value: "", label: "(none)" },
      { value: "EyeLOpen", label: "Itself (flip the sign)" },
      { value: "EyeROpen", label: "EyeROpen" },
    ]);
    expect(mirrorOptions([param("A", "左"), param("B", "右")], "B")[2]?.label).toBe("左 (A)");
  });

  it("builds the command", () => {
    expect(setMirrorCommand("EyeLOpen", "EyeROpen")).toEqual({
      op: "set_parameter_mirror",
      id: "EyeLOpen",
      mirror: "EyeROpen",
    });
    expect(setMirrorCommand("EyeLOpen", "")).toEqual({
      op: "set_parameter_mirror",
      id: "EyeLOpen",
      mirror: null,
    });
  });

  it("swaps left and right in standard IDs", () => {
    expect(swapSide("EyeLOpen")).toBe("EyeROpen");
    expect(swapSide("BrowRY")).toBe("BrowLY");
    expect(swapSide("ParamEyeLSmile")).toBe("ParamEyeRSmile");
    expect(swapSide("ArmLeft")).toBe("ArmRight");
    expect(swapSide("Hair2L")).toBe("Hair2R");
    expect(swapSide("AngleX")).toBeNull();
    expect(swapSide("LipSync")).toBeNull();
  });

  it("suggests a partner or itself", () => {
    const ids = ["AngleX", "AngleY", "AngleZ", "EyeLOpen", "EyeROpen", "BrowLY"];
    expect(suggestedMirror("EyeLOpen", ids)).toBe("EyeROpen");
    expect(suggestedMirror("EyeROpen", ids)).toBe("EyeLOpen");
    expect(suggestedMirror("AngleX", ids)).toBe("AngleX");
    expect(suggestedMirror("AngleZ", ids)).toBe("AngleZ");
    expect(suggestedMirror("AngleY", ids)).toBeNull();
    // 相手がいなければ提案しない。
    expect(suggestedMirror("BrowLY", ids)).toBeNull();
  });
});
