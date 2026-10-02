// 操作 API (T0805、src-tauri/src/api) とブラウザソース (T0808) の様子の形と、パネルで使う小さな計算。
// 安全性の規則は ADR 0028 の 6.、メッセージは docs/studio-api.md。

/** クライアントに与える権限 (store.rs の `Permission`)。 */
export type ApiPermission = "read" | "control";

export const API_PERMISSIONS: readonly ApiPermission[] = ["read", "control"];

/** 承認したクライアント (mod.rs の `ClientView`)。時刻は Unix 時間の秒。 */
export interface ApiClient {
  id: string;
  name: string;
  permission: ApiPermission;
  approvedAt: number;
  lastSeen: number | null;
  /** いま接続している数。 */
  connections: number;
}

/** 承認を待つクライアント (mod.rs の `PendingClient`)。 */
export interface ApiPendingClient {
  id: string;
  name: string;
  /** クライアントが求めた権限 (承認の画面の初めの選択)。 */
  permission: ApiPermission;
}

/** 操作 API の様子 (mod.rs の `ApiView`。`Monitor::Api` で届く)。 */
export interface ApiView {
  enabled: boolean;
  /** 待ち受けている (有効でも、ポートを使えなければ `false`)。 */
  listening: boolean;
  port: number;
  /** 待ち受けられない・設定を読めない・書けない理由 (バックエンドの英語の文)。 */
  error: string | null;
  clients: ApiClient[];
  pending: ApiPendingClient[];
  /** 名乗る前と承認待ちも含めた接続の数。 */
  connections: number;
  /** ブラウザソース (T0808。待ち受けとポートを共有する)。 */
  source: SourceView;
}

/** ブラウザソースの様子 (mod.rs の `SourceView`)。URL はトークンを含むので `sourceUrl` で別に取る。 */
export interface SourceView {
  enabled: boolean;
  /** ページを出している (有効でも、ポートを使えなければ `false`)。 */
  listening: boolean;
  /** つないでいるページの数。 */
  connections: number;
}

export const EMPTY_API: ApiView = {
  enabled: false,
  listening: false,
  port: 28451,
  error: null,
  clients: [],
  pending: [],
  connections: 0,
  source: { enabled: false, listening: false, connections: 0 },
};

/** ブラウザソースの URL のトークンを伏せたもの (画面に出す。コピーするのは伏せていない URL)。 */
export function maskSourceUrl(url: string): string {
  return url.replace(/\/source\/[^/]+\//, "/source/••••••••/");
}

/** 選べるポート (1024 未満は OS の管理者の権限が要るので選ばせない)。 */
export const API_PORT_RANGE = { min: 1024, max: 65535 } as const;

/** クライアントに入れてもらう接続先。 */
export function apiUrl(port: number): string {
  return `ws://127.0.0.1:${port}/api`;
}

/** 選べるポートか。 */
export function isApiPort(port: number): boolean {
  return Number.isInteger(port) && port >= API_PORT_RANGE.min && port <= API_PORT_RANGE.max;
}

/**
 * 承認を待つクライアントのうち、まだ知らせていないもの (「後で」を押したものを除く)。
 * 知らせは操作ウィンドウのどのパネルを開いていても上に出す (docs/studio.md#操作ウィンドウ)。
 */
export function pendingToShow(view: ApiView, postponed: ReadonlySet<string>): ApiPendingClient[] {
  return view.pending.filter((p) => !postponed.has(p.id));
}
