// エディタの偽のホスト: Tauri の IPC (`window.__TAURI_INTERNALS__.invoke`) を、Node の側の読むだけの偽物に中継する。
//
// - 開くプロジェクトは `ugoe sample dress-up --project` で作った見本 (model.json・physics.json・motions/・expressions/) と、
//   同じ見本の .ugoe (ビューポートが描く)。どのパスを開いても、この見本を開いたことにする。
// - 編集 (apply_command など) と保存は `ui_audit.read_only` のエラーにする。画面を見るための偽物で、編集の流れは
//   Vitest (docs/testing.md#エディタの部品のテスト) と実機の e2e (`just e2e`) が確かめる。
// - 知らないコマンドは null を返し、`unknown` に名前を残す (報告に出す)。バックエンドのコマンドが増えたときに気づけるように。
//   戻り値の形は apps/editor/src/lib/generated/types.ts に合わせる。
// - 見本に無い状態 (AI の提案・送る前の確認・送信中・着せ替え・モジュールのプロジェクト) は `host.set({...})` で切り替える
//   (場面は lib/apps.mjs。中身は lib/editor-fixtures.mjs。T0676)。AI への送信 (`classify_psd_layers` に同意を付けたもの) は
//   答えを返さずに待ち、`cancel_ai_requests` で `ai.cancelled` で終わる (送信中と取り消しの画面を撮るため)。
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import {
  aiSettingsFixture,
  differencesFixture,
  disclosureFixture,
  moduleStatusFixture,
  proposalStateFixture,
  renameModule,
} from "./editor-fixtures.mjs";

const READ_ONLY = new Set([
  "apply_command",
  "undo",
  "redo",
  "save_project",
  "save_project_as",
  "export_bundle",
  "export_module",
  "import_psd",
  "new_project_from_psd",
  "adopt_proposal",
  "generate_texture_normal_map",
  "assign_texture_normal_map",
  "write_json_file",
  "write_png_file",
  "store_api_key",
  "delete_api_key",
]);

function readJsonDir(dir, suffix) {
  let names;
  try {
    names = readdirSync(dir);
  } catch {
    return [];
  }
  return names
    .filter((n) => n.endsWith(suffix))
    .sort()
    .map((n) => JSON.parse(readFileSync(join(dir, n), "utf8")));
}

/**
 * 偽のホストを作る。`project` は .ugoeproj のディレクトリ、`bundle` は同じモデルの .ugoe のパス。
 * 戻り値の `handle(cmd, args)` は {value} か {bytes: Buffer} か {error: Message} を返す。
 */
export function createEditorHost({ project, bundle }) {
  const sample = {
    name: JSON.parse(readFileSync(join(project, "project.json"), "utf8")).name,
    model: readFileSync(join(project, "model.json"), "utf8"),
    physics: (() => {
      try {
        return JSON.parse(readFileSync(join(project, "physics.json"), "utf8"));
      } catch {
        return null;
      }
    })(),
    motions: readJsonDir(join(project, "motions"), ".motion.json"),
    expressions: readJsonDir(join(project, "expressions"), ".expression.json"),
    bundle: readFileSync(bundle),
  };
  // 新しいプロジェクト (空のモデル): 見本のモデルの一覧をすべて空にし、省ける欄 (pose・effects・avatar など) を除く
  const emptyModel = JSON.stringify(
    Object.fromEntries(
      Object.entries(JSON.parse(sample.model)).flatMap(([k, v]) => {
        if (Array.isArray(v)) return [[k, []]];
        if (k === "meta") return [[k, { ...v, name: "Untitled" }]];
        return typeof v === "object" && v !== null ? [] : [[k, v]];
      }),
    ),
  );

  // 見本の .ugoemod (`ugoe sample dress-up` が .ugoe と同じ所に書く)
  const modulesDir = dirname(bundle);
  const moduleFiles = (() => {
    try {
      return readdirSync(modulesDir)
        .filter((n) => n.endsWith(".ugoemod"))
        .sort()
        .map((n) => join(modulesDir, n));
    } catch {
      return [];
    }
  })();
  /** モジュールのプロジェクトにしたときのモジュール (見本の Sailor。無ければ最初のもの)。 */
  const moduleFile = moduleFiles.find((p) => basename(p) === "sailor.ugoemod") ?? moduleFiles[0];

  const state = {
    open: false,
    path: null,
    exportSettings: { atlas: false, atlas_max_size: 2048, atlas_padding: 2 },
    // `set` で切り替える状態 (T0676)
    /** AI の提案・分類があり、外部に送れる (キーを保存し、プロジェクトが送ってよい)。 */
    ai: false,
    /** 分類のキャッシュを戻している最中。 */
    restoring: false,
    /** プレビューの着せ替えに見本のモジュール (と読めないファイル) を並べる。 */
    dressUp: false,
    /** モジュールのプロジェクト (本体は見本、モジュールは Sailor) を開いている。 */
    module: false,
  };
  /** 答えを待たせている AI への送信 (取り消すと `ai.cancelled` で終わる)。 */
  const pending = [];
  const unknown = new Set();
  const calls = [];

  const status = () => ({
    name: state.open ? sample.name : "Untitled",
    path: state.open ? state.path : null,
    modified: false,
  });
  const readOnly = (cmd) => ({
    error: {
      code: "ui_audit.read_only",
      args: { command: cmd },
      message: `${cmd} is not available in the UI audit (the fake host is read-only)`,
    },
  });

  const handlers = {
    app_info: () => ({ version: "0.0.0-ui-audit" }),
    project_status: status,
    new_project: () => {
      state.open = false;
      state.path = null;
      return status();
    },
    open_project: (a) => {
      state.open = true;
      state.path = a?.path ?? project;
      return status();
    },
    open_sample_project: (a) => {
      state.open = true;
      state.path = join(a?.parent ?? "/tmp", "ugoe-dress-up-sample", "dress-up.ugoeproj");
      return status();
    },
    reload_project: status,
    check_external_change: () => null,
    check_base_change: () => null,
    module_status: () =>
      state.open && state.module && moduleFile
        ? moduleStatusFixture({ name: "Sailor", base: project, model: JSON.parse(sample.model) })
        : null,
    module_open_problem: () => null,
    get_model_json: () => (state.open ? sample.model : emptyModel),
    get_animations: () =>
      state.open
        ? { motions: sample.motions, expressions: sample.expressions }
        : { motions: [], expressions: [] },
    get_physics: () => (state.open ? sample.physics : null),
    history_status: () => ({
      can_undo: false,
      can_redo: false,
      undo_count: 0,
      redo_count: 0,
      undo_limit: 100,
    }),
    set_undo_limit: (a) => ({
      can_undo: false,
      can_redo: false,
      undo_count: 0,
      redo_count: 0,
      undo_limit: a?.limit ?? 100,
    }),
    get_export_settings: () => state.exportSettings,
    set_export_settings: (a) => {
      state.exportSettings = a?.settings ?? state.exportSettings;
      return status();
    },
    get_preview_lighting: () => ({
      enabled: true,
      ambient: [1, 1, 1],
      ambient_intensity: 0.45,
      lights: [{ type: "directional", direction: [1, 1, -1], color: [1, 1, 1], intensity: 0.7 }],
    }),
    set_preview_lighting: status,
    get_preview_modules: () =>
      state.dressUp
        ? [
            // モジュールのプロジェクトでは、同じ名前 (Sailor) のファイルの行に今の編集との違いが出る
            ...moduleFiles.map((path) => ({ path, attached: true })),
            { path: join(modulesDir, "missing-ribbon.ugoemod"), attached: true },
          ]
        : [],
    set_preview_modules: status,
    get_normal_map_groups: () => [],
    check_physics: () => [],
    check_avatar_profile: () => ({ claimed: null, satisfied: 0, errors: [], warnings: [] }),
    find_overflows: () => [],
    verify_overflows: () => ({ findings: [], elements: 0, cells: 0, truncated: [] }),
    local_to_canvas: (a) => a?.points ?? [],
    canvas_to_local: (a) => a?.points ?? [],
    ai_settings: () => aiSettingsFixture({ ready: state.ai }),
    cancel_ai_requests: () => {
      const cancelled = pending.splice(0);
      for (const resolve of cancelled)
        resolve({
          error: {
            code: "ai.cancelled",
            args: {},
            message: "the request was cancelled",
          },
        });
      return cancelled.length;
    },
    get_proposals: () =>
      state.ai
        ? proposalStateFixture({ restoring: state.restoring })
        : {
            proposals: [],
            psd: null,
            classification: null,
            regenerable: [],
            sections: [],
            stale_rigs: [],
            restoring: state.restoring,
          },
    compare_preview_module: () => differencesFixture(),
    mcp_bridge_status: () => ({
      enabled: false,
      running: false,
      url: null,
      port: null,
      discovery: null,
      error: null,
      clients: [],
      pending: [],
      project: state.path,
      sidecar: { path: null, wsl_path: null, platform: "linux", in_wsl: false },
    }),
    clear_proposal_preview: () => null,
  };

  /** AI に送るコマンド: 同意が無ければ確認を求め、同意があれば取り消すまで答えない。 */
  const sending = (args) => {
    if (!args?.consent)
      return { value: { status: "needs_consent", disclosure: disclosureFixture() } };
    return new Promise((resolve) => pending.push(resolve));
  };

  return {
    unknown,
    calls,
    /** 見本に無い状態を切り替える (`ai`・`restoring`・`dressUp`・`module`)。 */
    set(changes) {
      Object.assign(state, changes);
    },
    /** 1 つのコマンドを処理する。答えを待たせるもの (AI への送信) は Promise を返す。 */
    handle(cmd, args) {
      calls.push(cmd);
      if (cmd === "get_model_bundle") {
        return state.open ? { bytes: sample.bundle } : readOnly(cmd);
      }
      if (cmd === "classify_psd_layers" && state.ai) return sending(args);
      if (cmd === "propose_rig_from_psd" && state.ai) {
        return {
          error: {
            code: "ai.time_limit",
            args: { minutes: 10 },
            message: "stopped after the time limit (10 min)",
          },
        };
      }
      if (cmd === "read_module_file") {
        const path = args?.path ?? "";
        return existsSync(path)
          ? { bytes: readFileSync(path) }
          : {
              error: {
                code: "editor.io",
                args: { path, detail: "No such file or directory (os error 2)" },
                message: `could not read ${path}: No such file or directory (os error 2)`,
              },
            };
      }
      if (cmd === "get_preview_base_bundle") return { bytes: sample.bundle };
      if (cmd === "get_preview_module" && moduleFile) {
        const bytes = readFileSync(moduleFile);
        return { bytes: args?.name ? renameModule(bytes, args.name) : bytes };
      }
      if (READ_ONLY.has(cmd)) return readOnly(cmd);
      const handler = handlers[cmd];
      if (!handler) {
        unknown.add(cmd);
        return { value: null };
      }
      return { value: handler(args) };
    },
  };
}

/**
 * ページの中で動かす初期化 (page.addInitScript に渡す)。`window.__ugoeAuditIpc` (page.exposeFunction) に中継する。
 * ファイルのダイアログは `window.__UGOE_FILE_DIALOGS__` で差し替え、`window.__uiAuditNextPath` を選んだことにする。
 */
export function editorPageInit({ osLocale, home }) {
  let callbackId = 1;
  window.__uiAuditNextPath = null;
  const nextPath = async () => window.__uiAuditNextPath;
  window.__UGOE_FILE_DIALOGS__ = {
    openFile: nextPath,
    openDirectory: nextPath,
    saveFile: nextPath,
    defaultDirectory: async () => home,
  };
  const toBase64 = (bytes) => {
    let s = "";
    for (const b of bytes) s += String.fromCharCode(b);
    return btoa(s);
  };
  window.__TAURI_INTERNALS__ = {
    transformCallback: () => callbackId++,
    unregisterCallback: () => {},
    convertFileSrc: (p) => p,
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { windowLabel: "main", label: "main" },
    },
    invoke: async (cmd, args) => {
      if (cmd === "plugin:os|locale") return osLocale;
      if (cmd === "plugin:path|resolve_directory") return home;
      if (cmd === "plugin:event|listen") return callbackId++;
      if (cmd.startsWith("plugin:")) return null;
      const body =
        args instanceof Uint8Array || args instanceof ArrayBuffer
          ? { __bytes: toBase64(new Uint8Array(args)) }
          : (args ?? {});
      const reply = JSON.parse(await window.__ugoeAuditIpc(cmd, JSON.stringify(body)));
      if ("error" in reply) throw reply.error;
      if ("bytes" in reply) {
        const raw = atob(reply.bytes);
        const out = new Uint8Array(raw.length);
        for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
        return out.buffer;
      }
      return reply.value;
    },
  };
}

/** `createEditorHost` の結果を、page.exposeFunction の関数 (文字列で受けて文字列で返す) に。 */
export function ipcBinding(host) {
  return async (cmd, json) => {
    const args = JSON.parse(json);
    const r = await host.handle(cmd, args);
    if (r.bytes) return JSON.stringify({ bytes: Buffer.from(r.bytes).toString("base64") });
    return JSON.stringify(r);
  };
}
