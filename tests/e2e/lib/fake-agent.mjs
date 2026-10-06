// 偽のエージェント: エディタの中の MCP サーバ (ブリッジ。apps/editor/src-tauri/src/mcp.rs) に、MCP の Streamable HTTP で
// つなぐ小さなクライアント。Claude Code や Claude Desktop の代わりに、つながったエージェントの一覧・提案・適用の確認を
// 画面に出すために使う (マニュアルの画像。tests/e2e/README.md#マニュアルの画像)。本物のモデルには何も送らない。
//
// - 送り先とトークンは、画面の表示の代わりにバックエンドのコマンド (`mcp_bridge_status`・`get_mcp_bridge_token`) で読む。
// - 握手をする版 (2025-11-25) で `initialize` を送り、`clientInfo` の名前と版を「Connected agents」に出させる。
//   セッションは無い (crates/ugoe-mcp/src/http.rs) ので、要求は 1 つずつ POST で送り、JSON の答えを読む。
// - ツールの結果は `structuredContent` (無ければ最初の文の JSON) を返し、`isError` ならエラーにする。

/** 握手をする版 (crates/ugoe-mcp/src/server.rs の `LEGACY_VERSIONS` の先頭)。 */
const PROTOCOL_VERSION = "2025-11-25";

/**
 * ブリッジにつなぎ、`initialize` を送る。
 * @param {{ url: string, token: string, name: string, version: string }} options
 * @returns {Promise<{ call: (tool: string, args: object) => Promise<any> }>}
 */
export async function connectAgent({ url, token, name, version }) {
  let id = 0;
  const post = async (body) => {
    const res = await fetch(url, {
      method: "POST",
      headers: {
        authorization: `Bearer ${token}`,
        "content-type": "application/json",
        accept: "application/json, text/event-stream",
        "mcp-protocol-version": PROTOCOL_VERSION,
      },
      body: JSON.stringify(body),
    });
    const text = await res.text();
    if (!res.ok && res.status !== 202) {
      throw new Error(`${body.method}: HTTP ${res.status}: ${text}`);
    }
    return text === "" ? null : JSON.parse(text);
  };
  const request = async (method, params) => {
    const reply = await post({ jsonrpc: "2.0", id: ++id, method, params });
    if (reply.error) throw new Error(`${method}: ${JSON.stringify(reply.error)}`);
    return reply.result;
  };

  await request("initialize", {
    protocolVersion: PROTOCOL_VERSION,
    capabilities: {},
    clientInfo: { name, version },
  });
  await post({ jsonrpc: "2.0", method: "notifications/initialized" });
  return {
    async call(tool, args) {
      const result = await request("tools/call", { name: tool, arguments: args });
      const text = result.content?.find((c) => c.type === "text")?.text;
      const value = result.structuredContent ?? (text ? JSON.parse(text) : null);
      if (result.isError) throw new Error(`${tool}: ${text ?? JSON.stringify(result)}`);
      return value;
    },
  };
}
