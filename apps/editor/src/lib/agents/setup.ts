// 「Connect agents」の画面に出す、クライアントごとの登録の手順 (docs/manual/mcp.md)。
// 同梱の ugoe-mcp のパス (Sidecar) を埋め、そのまま写して使えるコマンドと設定を作る。画面から切り離して Vitest で確かめる。
import type { McpBridgeStatus, Sidecar } from "../ipc";

/** クライアントに登録するサーバの名前。 */
export const SERVER_NAME = "ugoe-editor";

/** 写せる 1 つの手順。 */
export interface SetupStep {
  /** 何をするか (1 行)。 */
  label: string;
  /** 写す中身 (コマンドか設定の JSON)。 */
  text: string;
  /** 補足。 */
  note?: string;
}

/** クライアントごとのまとまり。 */
export interface SetupSection {
  id: "claude" | "claude-desktop" | "opencode" | "chatgpt" | "wsl" | "http";
  title: string;
  /** まとまりの説明。 */
  summary: string;
  steps: SetupStep[];
}

/** POSIX のシェル (bash・zsh・fish) で 1 語として渡るよう引用する。 */
export function shQuote(s: string): string {
  if (/^[\w@%+=:,./-]+$/.test(s)) return s;
  return `'${s.replaceAll("'", `'\\''`)}'`;
}

/** Windows の cmd と PowerShell の両方で 1 語として渡るよう引用する (パスに `"` は入らない)。 */
export function winQuote(s: string): string {
  return /[\s&()^;,'`]/.test(s) ? `"${s}"` : s;
}

/** エディタと同じ OS のシェルでの引用。 */
function quoteFor(sidecar: Sidecar): (s: string) => string {
  return sidecar.platform === "windows" ? winQuote : shQuote;
}

/** 同梱の ugoe-mcp のパス (見つからなければ PATH の `ugoe-mcp`)。 */
export function relayCommand(sidecar: Sidecar): string {
  return sidecar.path ?? "ugoe-mcp";
}

/** OpenCode の設定 (`opencode.json`) の `mcp` の項目。 */
export function opencodeConfig(command: string[]): string {
  const config = {
    $schema: "https://opencode.ai/config.json",
    mcp: {
      [SERVER_NAME]: { type: "local", command, enabled: true, timeout: 180000 },
    },
    permission: {
      [`${SERVER_NAME}_apply_proposal`]: "ask",
      [`${SERVER_NAME}_save_project`]: "ask",
    },
  };
  return JSON.stringify(config, null, 2);
}

/** Claude Desktop の設定 (`claude_desktop_config.json`) の `mcpServers` の項目 (.mcpb を使わないとき)。 */
export function claudeDesktopConfig(command: string): string {
  const config = { mcpServers: { [SERVER_NAME]: { command, args: ["--editor"] } } };
  return JSON.stringify(config, null, 2);
}

/** 登録の手順 (エディタの OS と同梱の ugoe-mcp のパスに合わせる)。 */
export function setupSections(status: McpBridgeStatus): SetupSection[] {
  const { sidecar } = status;
  const exe = relayCommand(sidecar);
  const q = quoteFor(sidecar);
  // tunnel-client は --mcp-command を 1 つの文字列で受け取る。空白を含むパスは中で引用する。
  const tunnelExe = /\s/.test(exe) ? `'${exe}'` : exe;
  const sections: SetupSection[] = [
    {
      id: "claude",
      title: "Claude Code",
      summary:
        "Register the bundled ugoe-mcp as a stdio server. It relays to this editor while the bridge is on.",
      steps: [
        {
          label: "Run once in a terminal",
          text: `claude mcp add --scope user ${SERVER_NAME} -- ${q(exe)} --editor`,
          note: "Then check it with “claude mcp list”, or with /mcp inside Claude Code.",
        },
      ],
    },
    {
      id: "claude-desktop",
      title: "Claude Desktop",
      summary:
        "Install the editor as a Claude Desktop extension (.mcpb). It contains a copy of the bundled ugoe-mcp " +
        "and relays to this editor while the bridge is on.",
      steps: [
        {
          label:
            "Alternative: add it to claude_desktop_config.json (Claude Desktop: Settings → Developer → Edit Config)",
          text: claudeDesktopConfig(exe),
          note: "Merge it into the existing “mcpServers”, then quit and restart Claude Desktop.",
        },
      ],
    },
    {
      id: "opencode",
      title: "OpenCode",
      summary:
        "Add a local MCP server to opencode.json (the project folder or ~/.config/opencode/).",
      steps: [
        {
          label: "opencode.json",
          text: opencodeConfig([exe, "--editor"]),
          note: "“ask” makes OpenCode confirm applying and saving too; the editor asks you in any case.",
        },
      ],
    },
    {
      id: "chatgpt",
      title: "ChatGPT (Secure MCP Tunnel)",
      summary:
        "ChatGPT connects from OpenAI's servers. OpenAI's tunnel-client starts the relay on this computer and " +
        "connects out, so nothing is exposed to the internet. Needs ChatGPT developer mode and a tunnel ID.",
      steps: [
        {
          label: "Create the tunnel profile",
          text:
            `tunnel-client init --sample sample_mcp_stdio_local --profile ${SERVER_NAME} ` +
            `--tunnel-id <tunnel_id> --mcp-command "${tunnelExe} --editor"`,
        },
        {
          label: "Check and start the tunnel",
          text: `tunnel-client doctor --profile ${SERVER_NAME} --explain\ntunnel-client run --profile ${SERVER_NAME}`,
        },
      ],
    },
  ];
  if (sidecar.platform === "windows") {
    const wslExe = sidecar.wsl_path ?? "/mnt/c/<path to the editor>/ugoe-mcp.exe";
    sections.push({
      id: "wsl",
      title: "Agents in WSL",
      summary:
        "Recommended: register the Windows ugoe-mcp.exe from inside WSL. WSL can start Windows programs, the " +
        "connection stays inside Windows, and paths are converted between WSL and Windows.",
      steps: [
        {
          label: "Claude Code in WSL (run inside WSL)",
          text: `claude mcp add --scope user ${SERVER_NAME} -- ${shQuote(wslExe)} --editor --wsl-distro "$WSL_DISTRO_NAME"`,
        },
        {
          label:
            "OpenCode in WSL (opencode.json; replace <distro> with the output of “echo $WSL_DISTRO_NAME”)",
          text: opencodeConfig([wslExe, "--editor", "--wsl-distro", "<distro>"]),
        },
        {
          label: "Alternative: the Linux ugoe-mcp inside WSL",
          text: `claude mcp add --scope user ${SERVER_NAME} -- ugoe-mcp --editor`,
          note:
            "Works only with WSL's mirrored networking (networkingMode=mirrored in %UserProfile%\\.wslconfig). " +
            "With the default NAT networking it cannot reach the editor; use ugoe-mcp.exe instead.",
        },
      ],
    });
  }
  if (status.url) {
    const port = status.port;
    sections.push({
      id: "http",
      title: "Other clients (HTTP)",
      summary:
        "Clients that speak Streamable HTTP can connect directly with the token. Set a fixed port first, " +
        "because the port changes each time the editor starts otherwise.",
      steps: [
        {
          label: port ? "Claude Code over HTTP" : "Claude Code over HTTP (set a fixed port first)",
          text: `claude mcp add --scope user --transport http ${SERVER_NAME} ${status.url} --header "Authorization: Bearer <token>"`,
          note: "Replace <token> with the copied token. Prefer the stdio relay above: it needs no token.",
        },
      ],
    });
  }
  return sections;
}

/** クライアントの一覧に出す、握手してからの時間。 */
export function connectedAgo(connectedAt: number, now: number): string {
  const seconds = Math.max(0, Math.round((now - connectedAt) / 1000));
  if (seconds < 60) return "just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  return `${hours} h ago`;
}
