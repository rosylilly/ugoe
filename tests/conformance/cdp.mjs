// ヘッドレスの Chrome の CDP (DevTools のプロトコル) に繋ぐもの (run.mjs と example.mjs で共有する)。
// playwright は使わず、Node の組み込みの WebSocket で繋ぐ (T0484)。

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

/**
 * `--remote-debugging-port=0` で起動した Chrome のブラウザの CDP に WebSocket で繋ぎ、命令を送るもの。最初の `send` で
 * プロファイルの DevToolsActivePort (1 行目がポート、2 行目がブラウザの口のパス) を読んで繋ぐ。
 *
 * ページの命令は、`Target.attachToTarget` (`flatten: true`) で得たセッションの ID を `send` の 3 つ目の引数に渡して送る。
 * イベントは `on(method, listener)` で受け取る (`listener(params, sessionId)`)。
 */
export function browserCdp(profile) {
  let connection = null;
  let id = 0;
  const pending = new Map();
  const listeners = new Map();
  const connect = async () => {
    const file = join(profile, "DevToolsActivePort");
    for (let i = 0; !existsSync(file); i++) {
      if (i >= 100) throw new Error("Chrome did not write DevToolsActivePort");
      await new Promise((r) => setTimeout(r, 100));
    }
    const [port, path] = readFileSync(file, "utf8").split("\n");
    const socket = new WebSocket(`ws://127.0.0.1:${port.trim()}${path.trim()}`);
    await new Promise((resolve, reject) => {
      socket.addEventListener("open", resolve, { once: true });
      socket.addEventListener("error", () => reject(new Error("could not connect to CDP")), {
        once: true,
      });
    });
    socket.addEventListener("message", (e) => {
      const message = JSON.parse(String(e.data));
      if (message.method) {
        for (const listener of listeners.get(message.method) ?? []) {
          listener(message.params, message.sessionId);
        }
        return;
      }
      const request = pending.get(message.id);
      if (!request) return;
      pending.delete(message.id);
      if (message.error) request.reject(new Error(`CDP: ${message.error.message}`));
      else request.resolve(message.result);
    });
    socket.addEventListener("close", () => {
      for (const request of pending.values()) request.reject(new Error("CDP was closed"));
      pending.clear();
    });
    return socket;
  };
  return {
    async send(method, params = {}, sessionId = undefined) {
      connection ??= connect();
      const socket = await connection;
      const request = ++id;
      const result = new Promise((resolve, reject) => pending.set(request, { resolve, reject }));
      socket.send(JSON.stringify({ id: request, method, params, sessionId }));
      return result;
    },
    on(method, listener) {
      listeners.set(method, [...(listeners.get(method) ?? []), listener]);
    },
    close() {
      connection?.then((socket) => socket.close()).catch(() => {});
    },
  };
}
