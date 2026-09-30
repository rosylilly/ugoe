import { describe, expect, it } from "vitest";
import type { McpBridgeStatus, Sidecar } from "../ipc";
import { connectedAgo, opencodeConfig, setupSections, shQuote, winQuote } from "./setup";

function status(sidecar: Partial<Sidecar>, patch: Partial<McpBridgeStatus> = {}): McpBridgeStatus {
  return {
    enabled: true,
    running: true,
    url: "http://127.0.0.1:40123/mcp",
    port: null,
    discovery: "/home/a/.local/state/app.ugoe.editor/editor-mcp.json",
    error: null,
    clients: [],
    pending: [],
    project: "/home/a/quad.ugoeproj",
    sidecar: { path: null, wsl_path: null, platform: "linux", in_wsl: false, ...sidecar },
    ...patch,
  };
}

function step(s: McpBridgeStatus, id: string, index = 0): string {
  const section = setupSections(s).find((x) => x.id === id);
  if (!section) throw new Error(`no ${id}`);
  return section.steps[index]?.text ?? "";
}

describe("quoting", () => {
  it("quotes only when needed", () => {
    expect(shQuote("/usr/bin/ugoe-mcp")).toBe("/usr/bin/ugoe-mcp");
    expect(shQuote("/opt/my apps/ugoe-mcp")).toBe("'/opt/my apps/ugoe-mcp'");
    expect(shQuote("/a/it's")).toBe("'/a/it'\\''s'");
    expect(winQuote("C:\\ugoe\\ugoe-mcp.exe")).toBe("C:\\ugoe\\ugoe-mcp.exe");
    expect(winQuote("C:\\Users\\A B\\ugoe-mcp.exe")).toBe('"C:\\Users\\A B\\ugoe-mcp.exe"');
  });
});

describe("setupSections", () => {
  it("fills in the bundled ugoe-mcp for Claude Code, OpenCode and ChatGPT", () => {
    const s = status({ path: "/usr/bin/ugoe-mcp" });
    expect(step(s, "claude")).toBe(
      "claude mcp add --scope user ugoe-editor -- /usr/bin/ugoe-mcp --editor",
    );
    const opencode = JSON.parse(step(s, "opencode"));
    expect(opencode.mcp["ugoe-editor"].command).toEqual(["/usr/bin/ugoe-mcp", "--editor"]);
    expect(opencode.permission["ugoe-editor_apply_proposal"]).toBe("ask");
    expect(step(s, "chatgpt")).toContain('--mcp-command "/usr/bin/ugoe-mcp --editor"');
    // Windows のエディタでなければ WSL の手順は出さない
    expect(setupSections(s).map((x) => x.id)).toEqual(["claude", "opencode", "chatgpt", "http"]);
  });

  it("falls back to ugoe-mcp on the PATH when nothing is bundled", () => {
    expect(step(status({}), "claude")).toContain("-- ugoe-mcp --editor");
  });

  it("shows ugoe-mcp.exe with the WSL path for agents in WSL", () => {
    const s = status({
      platform: "windows",
      path: "C:\\Users\\A B\\AppData\\Local\\ugoe\\ugoe-mcp.exe",
      wsl_path: "/mnt/c/Users/A B/AppData/Local/ugoe/ugoe-mcp.exe",
    });
    expect(step(s, "claude")).toBe(
      'claude mcp add --scope user ugoe-editor -- "C:\\Users\\A B\\AppData\\Local\\ugoe\\ugoe-mcp.exe" --editor',
    );
    expect(step(s, "wsl")).toBe(
      "claude mcp add --scope user ugoe-editor -- '/mnt/c/Users/A B/AppData/Local/ugoe/ugoe-mcp.exe' --editor " +
        '--wsl-distro "$WSL_DISTRO_NAME"',
    );
    expect(JSON.parse(step(s, "wsl", 1)).mcp["ugoe-editor"].command).toEqual([
      "/mnt/c/Users/A B/AppData/Local/ugoe/ugoe-mcp.exe",
      "--editor",
      "--wsl-distro",
      "<distro>",
    ]);
    expect(step(s, "chatgpt")).toContain(
      `--mcp-command "'C:\\Users\\A B\\AppData\\Local\\ugoe\\ugoe-mcp.exe' --editor"`,
    );
  });

  it("offers the HTTP form only while the bridge listens", () => {
    const off = status({}, { running: false, url: null });
    expect(setupSections(off).some((x) => x.id === "http")).toBe(false);
    expect(step(status({}), "http")).toContain(
      "--transport http ugoe-editor http://127.0.0.1:40123/mcp",
    );
  });

  it("writes valid OpenCode JSON", () => {
    expect(() => JSON.parse(opencodeConfig(["a b", "--editor"]))).not.toThrow();
  });
});

describe("connectedAgo", () => {
  it("rounds to minutes and hours", () => {
    expect(connectedAgo(0, 10_000)).toBe("just now");
    expect(connectedAgo(0, 5 * 60_000)).toBe("5 min ago");
    expect(connectedAgo(0, 3 * 3_600_000)).toBe("3 h ago");
  });
});
