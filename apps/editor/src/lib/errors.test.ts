import { describe, expect, it } from "vitest";
import { errorCode, errorText, isMessage } from "./errors";
import type { Message } from "./ipc";

const message: Message = {
  code: "project.not_a_project",
  args: { path: "/work/a.ugoeproj" },
  message: "/work/a.ugoeproj: not a ugoe project (project.json is missing)",
};

describe("backend errors", () => {
  it("reads the code and the English text of a Message", () => {
    expect(isMessage(message)).toBe(true);
    expect(errorCode(message)).toBe("project.not_a_project");
    expect(errorText(message)).toBe(message.message);
  });

  it("falls back to the text of other errors", () => {
    expect(isMessage("disk full")).toBe(false);
    expect(errorCode("disk full")).toBeNull();
    expect(errorText("disk full")).toBe("disk full");
    expect(errorText(new Error("broken"))).toBe("broken");
    expect(errorText({ code: 1 })).toBe("[object Object]");
  });
});
