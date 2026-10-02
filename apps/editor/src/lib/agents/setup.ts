// 「Connect agents」の画面に出す、クライアントごとの登録の手順 (docs/manual/mcp.md)。
// 同梱の ugoe-mcp のパス (Sidecar) を埋め、そのまま写して使えるコマンドと設定を作る。画面から切り離して Vitest で確かめる。
import { t } from "../i18n";
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

/** 登録の手順 (エディタの OS と同梱の ugoe-mcp のパスに合わせる)。文言は呼ぶたびに今の言語で引く。 */
export function setupSections(status: McpBridgeStatus): SetupSection[] {
  const { sidecar } = status;
  const exe = relayCommand(sidecar);
  const q = quoteFor(sidecar);
  // tunnel-client は --mcp-command を 1 つの文字列で受け取る。空白を含むパスは中で引用する。
  const tunnelExe = /\s/.test(exe) ? `'${exe}'` : exe;
  const sections: SetupSection[] = [
    {
      id: "claude",
      // i18n-ignore: 製品名
      title: "Claude Code",
      summary: t("agents.setup.claude.summary"),
      steps: [
        {
          label: t("agents.setup.claude.run"),
          // i18n-ignore: シェルのコマンド
          text: `claude mcp add --scope user ${SERVER_NAME} -- ${q(exe)} --editor`,
          note: t("agents.setup.claude.runNote"),
        },
      ],
    },
    {
      id: "claude-desktop",
      // i18n-ignore: 製品名
      title: "Claude Desktop",
      summary: t("agents.setup.desktop.summary"),
      steps: [
        {
          label: t("agents.setup.desktop.config"),
          text: claudeDesktopConfig(exe),
          note: t("agents.setup.desktop.configNote"),
        },
      ],
    },
    {
      id: "opencode",
      // i18n-ignore: 製品名
      title: "OpenCode",
      summary: t("agents.setup.opencode.summary"),
      steps: [
        {
          label: "opencode.json",
          text: opencodeConfig([exe, "--editor"]),
          note: t("agents.setup.opencode.configNote"),
        },
      ],
    },
    {
      id: "chatgpt",
      // i18n-ignore: 製品名と機能の名前
      title: "ChatGPT (Secure MCP Tunnel)",
      summary: t("agents.setup.chatgpt.summary"),
      steps: [
        {
          label: t("agents.setup.chatgpt.init"),
          text:
            // i18n-ignore: シェルのコマンド
            `tunnel-client init --sample sample_mcp_stdio_local --profile ${SERVER_NAME} ` +
            // i18n-ignore: シェルのコマンド
            `--tunnel-id <tunnel_id> --mcp-command "${tunnelExe} --editor"`,
        },
        {
          label: t("agents.setup.chatgpt.run"),
          // i18n-ignore: シェルのコマンド
          text: `tunnel-client doctor --profile ${SERVER_NAME} --explain\ntunnel-client run --profile ${SERVER_NAME}`,
        },
      ],
    },
  ];
  if (sidecar.platform === "windows") {
    const wslExe = sidecar.wsl_path ?? "/mnt/c/<path to the editor>/ugoe-mcp.exe";
    sections.push({
      id: "wsl",
      title: t("agents.setup.wsl.title"),
      summary: t("agents.setup.wsl.summary"),
      steps: [
        {
          label: t("agents.setup.wsl.claude"),
          // i18n-ignore: シェルのコマンド
          text: `claude mcp add --scope user ${SERVER_NAME} -- ${shQuote(wslExe)} --editor --wsl-distro "$WSL_DISTRO_NAME"`,
        },
        {
          label: t("agents.setup.wsl.opencode"),
          text: opencodeConfig([wslExe, "--editor", "--wsl-distro", "<distro>"]),
        },
        {
          label: t("agents.setup.wsl.linux"),
          // i18n-ignore: シェルのコマンド
          text: `claude mcp add --scope user ${SERVER_NAME} -- ugoe-mcp --editor`,
          note: t("agents.setup.wsl.linuxNote"),
        },
      ],
    });
  }
  if (status.url) {
    const port = status.port;
    sections.push({
      id: "http",
      title: t("agents.setup.http.title"),
      summary: t("agents.setup.http.summary"),
      steps: [
        {
          label: port ? t("agents.setup.http.claude") : t("agents.setup.http.claudeNoPort"),
          // i18n-ignore: シェルのコマンド
          text: `claude mcp add --scope user --transport http ${SERVER_NAME} ${status.url} --header "Authorization: Bearer <token>"`,
          note: t("agents.setup.http.claudeNote"),
        },
      ],
    });
  }
  return sections;
}

/** クライアントの一覧に出す、握手してからの時間。 */
export function connectedAgo(connectedAt: number, now: number): string {
  const seconds = Math.max(0, Math.round((now - connectedAt) / 1000));
  if (seconds < 60) return t("agents.clients.justNow");
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return t("agents.clients.minutesAgo", { count: minutes });
  const hours = Math.round(minutes / 60);
  return t("agents.clients.hoursAgo", { count: hours });
}
