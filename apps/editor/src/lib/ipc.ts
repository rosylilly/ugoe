// Tauri バックエンド (apps/editor/src-tauri/src/commands.rs など) への型付きラッパー。
// バックエンドのコマンドを変えたら、ここも同じ PR で更新すること。
// 引数・戻り値の型は Rust から生成した ./generated/types.ts を使う (`just gen`、docs/editor.md#型の生成)。
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AiReply,
  AiSettings,
  AnimationData,
  AppInfo,
  ApplyReport,
  AutoMeshOptions,
  AvatarCheck,
  BaseReload,
  BaseSummary,
  BridgeEvent,
  BulgeSettings,
  Classification,
  Command,
  ConsentAnswer,
  ElementOverflow,
  ExportReport,
  ExportSettings,
  ExternalChange,
  KeyformTarget,
  McpBridgeStatus,
  ModuleDifference,
  ModuleExportReport,
  ModuleProblem,
  ModuleRigChoice,
  ModuleStatus,
  NormalMapGroupChoice,
  NormalMapGroupView,
  OverflowReport,
  PhysicsSettings,
  PreviewLighting,
  PreviewModule,
  ProjectStatus,
  ProposalPreview,
  ProposalState,
  ProposalView,
  ProviderSettings,
  PsdImportWarning,
  RestPoseCollision,
} from "./generated/types";
import type { Point } from "./panels/inspector/keyforms/keyforms";

export type * from "./generated/types";

/** ugoe_editor_core::Command の JSON 表現。`Command` の別名 (既存のコードとの互換のため)。 */
export type EditCommand = Command;

export const ipc = {
  appInfo: () => invoke<AppInfo>("app_info"),
  /** 適用はしたが知らせたいこと (自動のミラーのやり直しの失敗) を返す。 */
  applyCommand: (command: Command) => invoke<ApplyReport>("apply_command", { command }),
  /** ugoe_model::Model の JSON (JSON.parse すると `Model`)。 */
  getModelJson: () => invoke<string>("get_model_json"),
  /** ドキュメントのモーションと表情 (タイムラインの表示用。編集は applyCommand)。 */
  getAnimations: () => invoke<AnimationData>("get_animations"),
  /** 物理の設定 (ugoe_model::physics::PhysicsSettings)。なければ null。 */
  getPhysics: () => invoke<PhysicsSettings | null>("get_physics"),
  undo: () => invoke<boolean>("undo"),
  redo: () => invoke<boolean>("redo"),
  projectStatus: () => invoke<ProjectStatus>("project_status"),
  newProject: () => invoke<ProjectStatus>("new_project"),
  /** apps/editor/src-tauri/src/project.rs。PSD から新しいプロジェクトを作り、取り込みの警告を返す。 */
  newProjectFromPsd: (path: string, meshOptions?: Partial<AutoMeshOptions>) =>
    invoke<PsdImportWarning[]>("new_project_from_psd", { path, meshOptions: meshOptions ?? null }),
  openProject: (path: string) => invoke<ProjectStatus>("open_project", { path }),
  /**
   * apps/editor/src-tauri/src/sample.rs。着せ替えの見本を、`parent` の中の新しいフォルダー
   * (`ugoe-dress-up-sample`、あれば `-2` など) に本体とモジュールのプロジェクトとして書き出し、本体を開く。
   */
  openSampleProject: (parent: string) => invoke<ProjectStatus>("open_sample_project", { parent }),
  /** apps/editor/src-tauri/src/module.rs。モジュールのプロジェクトなら本体とスロット、でなければ null。 */
  moduleStatus: () => invoke<ModuleStatus | null>("module_status"),
  /** 本体の候補 (.ugoeproj のフォルダーか .ugoe) の名前とスロット。 */
  readModuleBase: (path: string) => invoke<BaseSummary>("read_module_base", { path }),
  /** 本体のスロットに付ける空のモジュールのプロジェクトにする (未保存)。 */
  newModuleProject: (base: string, slot: string, name: string) =>
    invoke<ProjectStatus>("new_module_project", { base, slot, name }),
  /**
   * モジュールのプロジェクトを、本体を `base` に、スロットを `slot` に付け替えて開く (null ならそのまま。
   * 本体が見つからない・本体からスロットがなくなったとき)。
   */
  openModuleWithBase: (path: string, base: string | null, slot: string | null = null) =>
    invoke<ProjectStatus>("open_module_with_base", { path, base, slot }),
  /**
   * `path` のモジュールのプロジェクトが本体の問題で開けないなら、その問題と直すための情報 (openProject の失敗の後に呼ぶ)。
   * `base` なら project.json の本体の代わりにそれで確かめる (選び直した本体)。
   * 開ける・モジュールのプロジェクトでない・本体と関係のないエラーなら null。
   */
  moduleOpenProblem: (path: string, base: string | null = null) =>
    invoke<ModuleProblem | null>("module_open_problem", { path, base }),
  /** モジュールのプロジェクトの本体のファイルが、開いた (読み直した) 後で変わったか。変わっていなければ null。 */
  checkBaseChange: () => invoke<ExternalChange | null>("check_base_change"),
  /**
   * 本体をファイルから読み直す (保存していないモジュールの編集は保ち、Undo の履歴は消す)。`base` なら本体を、
   * `slot` ならスロットを付け替える (開き直さずに直す。保存するまでファイルに書かず、未保存の印が付く)。
   */
  reloadBase: (base: string | null = null, slot: string | null = null) =>
    invoke<BaseReload>("reload_base", { base, slot }),
  /**
   * モジュールのプロジェクトの本体だけの .ugoe (プレビューで今の編集のモジュールと比べる)。バックエンドのセッションが
   * 覚えておき、本体が変わるまで作り直さない。プレビューは本体の版 (ModuleStatus.base_revision) が描いているものと
   * 違うときだけ取る。
   */
  getPreviewBaseBundle: async () =>
    new Uint8Array(await invoke<ArrayBuffer>("get_preview_base_bundle")),
  /**
   * モジュールのプロジェクトの今の編集の .ugoemod (ファイルには書かない。書き出しと同じ検査に通らなければエラー)。
   * `name` ならモジュールの名前をそれにする (前に書き出した同じ名前のファイルと並べて読み込む)。
   */
  getPreviewModule: async (name: string | null = null) =>
    new Uint8Array(await invoke<ArrayBuffer>("get_preview_module", { name })),
  /**
   * モジュールのプロジェクトの今の編集と、書き出した .ugoemod のバイト列の違い (1 行ずつの英語と、足した・変えた
   * 要素ごとの詳細。同じなら空。「ファイル → 今の編集」の向き。T0654・T0656)。バイト列はそのまま本文で送る。
   */
  comparePreviewModule: (bytes: Uint8Array) =>
    invoke<ModuleDifference[]>("compare_preview_module", bytes),
  /**
   * モジュールのプロジェクトを .ugoemod に書き出す。書き出しの設定 (getExportSettings) でアトラス化を選んでいれば、
   * モジュールのテクスチャをアトラスにまとめる。
   */
  exportModule: (path: string) => invoke<ModuleExportReport>("export_module", { path }),
  /**
   * 開いた場所に保存する。開いた後で外で書き換えられていたら、`overwrite` でない限り書かずにエラー
   * (lib/externalChange.ts の `isExternalChangeError`)。
   */
  saveProject: (overwrite = false) => invoke<ProjectStatus>("save_project", { overwrite }),
  /** 開いているプロジェクトを保存先から開き直す (未保存の変更は捨てる)。 */
  reloadProject: () => invoke<ProjectStatus>("reload_project"),
  /** 開いた (最後に保存した) 後で、保存先のファイルが外で変わったか。変わっていなければ null。 */
  checkExternalChange: () => invoke<ExternalChange | null>("check_external_change"),
  /** 拡張子 .ugoeproj がなければバックエンドが付ける。 */
  saveProjectAs: (path: string) => invoke<ProjectStatus>("save_project_as", { path }),
  /** プロジェクトに残した書き出しの設定 (アトラス化の有無・最大の大きさ・余白)。 */
  getExportSettings: () => invoke<ExportSettings>("get_export_settings"),
  /** 書き出しの設定をプロジェクトに残す (保存で書き込む)。範囲の外ならエラー。 */
  setExportSettings: (settings: ExportSettings) =>
    invoke<ProjectStatus>("set_export_settings", { settings }),
  /** プロジェクトの書き出しの設定で .ugoe に書き出し、前後のテクスチャの枚数と状態の切り替えを返す。 */
  exportBundle: (path: string) => invoke<ExportReport>("export_bundle", { path }),
  /** 今のドキュメントの .ugoe バンドル (テクスチャ付き)。生のバイト列で届く。 */
  getModelBundle: async () => new Uint8Array(await invoke<ArrayBuffer>("get_model_bundle")),
  /** プロジェクトに残したプレビューの光源 (`project.json` の `editor.lighting`)。 */
  getPreviewLighting: () => invoke<PreviewLighting>("get_preview_lighting"),
  /** プレビューの光源をプロジェクトに残す (保存で書き込む。Undo の対象ではない)。範囲の外ならエラー。 */
  setPreviewLighting: (lighting: PreviewLighting) =>
    invoke<ProjectStatus>("set_preview_lighting", { lighting }),
  /**
   * apps/editor/src-tauri/src/lighting.rs。テクスチャのアルファの形からふくらみの法線マップを作って付ける
   * (AI は使わない。モデルのライティングが無効なら有効にする。1 回の Undo で戻る)。
   */
  /** テクスチャの法線マップを作る部位のまとまりと、その既定の設定。 */
  getNormalMapGroup: (texture: string) =>
    invoke<NormalMapGroupView>("get_normal_map_group", { texture }),
  /**
   * 法線マップを作るすべてのテクスチャの、まとまりの一覧と既定の設定 (AI タブ)。単独でふくらませるもの
   * (`alone` と、ラベルのない `group: null`) はそれぞれ 1 行にまとめて最後に置く。
   */
  getNormalMapGroups: () => invoke<NormalMapGroupView[]>("get_normal_map_groups"),
  /** まとまりの形から法線マップを作る (`settings` が null ならまとまりの既定、`wholeGroup` ならまとまり全体)。 */
  generateTextureNormalMap: (
    texture: string,
    settings: BulgeSettings | null,
    wholeGroup: boolean,
  ) => invoke<ApplyReport>("generate_texture_normal_map", { texture, settings, wholeGroup }),
  /** PNG ファイル (絶対パス) をテクスチャの法線マップとして付ける。`flipGreen` は DirectX 形式の画像。 */
  assignTextureNormalMap: (texture: string, path: string, flipGreen: boolean) =>
    invoke<ApplyReport>("assign_texture_normal_map", { texture, path, flipGreen }),
  /** テクスチャか法線マップの画像の PNG。生のバイト列で届く。 */
  getTexturePng: async (name: string) =>
    new Uint8Array(await invoke<ArrayBuffer>("get_texture_png", { name })),
  /** apps/editor/src-tauri/src/psd.rs。PSD を今のドキュメントに取り込み、警告を返す。 */
  /** `meshOptions` の省略したフィールドは既定値。 */
  importPsd: (path: string, meshOptions?: Partial<AutoMeshOptions>) =>
    invoke<PsdImportWarning[]>("import_psd", { path, meshOptions: meshOptions ?? null }),
  /** apps/editor/src-tauri/src/space.rs。`target` の親の座標系の点をキャンバス座標に写す。 */
  localToCanvas: (target: KeyformTarget, parameters: Record<string, number>, points: Point[]) =>
    invoke<Point[]>("local_to_canvas", { target, parameters, points }),
  /** キャンバス座標の点を `target` の親の座標系に写す。`guesses` は反復の初期値 (親の座標系)。 */
  canvasToLocal: (
    target: KeyformTarget,
    parameters: Record<string, number>,
    points: Point[],
    guesses: Point[],
  ) => invoke<Point[]>("canvas_to_local", { target, parameters, points, guesses }),
  /**
   * apps/editor/src-tauri/src/overflow.rs。パラメータの値 (含まないものは既定値) で、親のワープの格子の
   * 外にある点を要素ごとに返す。`tolerance` は UV の許容距離 (省略すると既定値)。
   */
  findOverflows: (parameters: Record<string, number>, tolerance?: number) =>
    invoke<ElementOverflow[]>("find_overflows", { parameters, tolerance: tolerance ?? null }),
  /** 親がワープのすべての要素について、キーフォームのすべてのセルではみ出しを調べる。 */
  verifyOverflows: (tolerance?: number) =>
    invoke<OverflowReport>("verify_overflows", { tolerance: tolerance ?? null }),
  /**
   * apps/editor/src-tauri/src/physics_check.rs。パラメータの既定値での静止姿勢で衝突体に当たっている
   * リグの鎖 (リグと衝突体の組ごと)。物理の設定がなければ空。
   */
  checkPhysics: () => invoke<RestPoseCollision[]>("check_physics"),
  /**
   * apps/editor/src-tauri/src/avatar.rs。アバタープロファイルへの準拠の検査の結果 (名乗るレベル・満たすレベル・
   * エラーと警告)。モデルは変えない。
   */
  checkAvatarProfile: () => invoke<AvatarCheck>("check_avatar_profile"),
  /**
   * apps/editor/src-tauri/src/ai.rs。AI の設定 (選んだプロバイダ・API キーの状態・開いているプロジェクトの
   * 「外部に送らない」)。キーの中身は返さない (docs/editor.md#ai-の設定と送信の同意)。
   */
  aiSettings: () => invoke<AiSettings>("ai_settings"),
  /** プロバイダ (Claude / OpenAI 互換)・モデル・ベースの URL を変える (エディタ全体の設定として保存する)。 */
  setAiProvider: (provider: ProviderSettings) =>
    invoke<AiSettings>("set_ai_provider", { provider }),
  /** 選んでいるプロバイダの API キーを OS の資格情報の保存先に保存する。 */
  storeApiKey: (key: string) => invoke<AiSettings>("store_api_key", { key }),
  /** 選んでいるプロバイダの API キーを消す。 */
  deleteApiKey: () => invoke<AiSettings>("delete_api_key"),
  /** 開いているプロジェクトのデータを外部の AI に送ってよいか (プロジェクトと一緒に保存する)。 */
  setAiSendExternally: (allowed: boolean) =>
    invoke<AiSettings>("set_ai_send_externally", { allowed }),
  /** 「今後は確かめない」を取り消す。 */
  forgetAiConsent: () => invoke<AiSettings>("forget_ai_consent"),
  /**
   * 選んでいるプロバイダの API キーと接続を確かめる (画像なしの小さなリクエスト)。外部に送る他のコマンドと同じく、
   * 同意がなければ送らずに `needs_consent` を返す。lib/ai/consent.ts の `withConsent` で呼ぶ。
   */
  testAiConnection: (consent: ConsentAnswer | null) =>
    invoke<AiReply<null>>("test_ai_connection", { consent }),
  /**
   * apps/editor/src-tauri/src/proposals.rs。AI の提案のパネルの状態 (提案の一覧・元にする PSD・分類の結果)。
   * docs/editor.md#ai-の提案
   */
  getProposals: () => invoke<ProposalState>("get_proposals"),
  /**
   * PSD のレイヤーを分類して覚える。`psd` を省略すると取り込んだ PSD。`useAi` なら画像を外部に送るので
   * `withConsent` で呼ぶ (偽なら名前の規則だけで、何も送らない)。`instructions` は AI に添える利用者の注文、
   * `revise` なら今の分類を前の答えとして送る (分類の作り直し)。
   */
  classifyPsdLayers: (
    psd: string | null,
    useAi: boolean,
    instructions: string | null,
    revise: boolean,
    consent: ConsentAnswer | null,
  ) =>
    invoke<AiReply<Classification>>("classify_psd_layers", {
      psd,
      useAi,
      instructions,
      revise,
      consent,
    }),
  /**
   * 分類の結果からリグを提案し、一覧に置く (ドキュメントは変えない)。`useAi` なら原画を外部に送るので
   * `withConsent` で呼ぶ (偽ならテンプレートの既定値)。`instructions` は AI に添える利用者の注文。
   * `moduleOptions` はモジュールのプロジェクトのリグの組み方 (null なら既定。docs/editor.md#モジュールのプロジェクトでの-ai-の提案)。
   */
  proposeRigFromPsd: (
    useAi: boolean,
    instructions: string | null,
    consent: ConsentAnswer | null,
    moduleOptions: ModuleRigChoice | null = null,
  ) =>
    invoke<AiReply<ProposalView>>("propose_rig_from_psd", {
      useAi,
      instructions,
      consent,
      moduleOptions,
    }),
  /**
   * apps/editor/src-tauri/src/proposals.rs。選んだまとまりのテクスチャに、まとまりごとの設定で法線マップを作る提案を
   * 一覧に置く (AI は使わず、何も送らない。ドキュメントは変えない)。コマンドはまとまりごとに区切る
   * (`ProposalState.sections`)。`replaceExisting` なら法線マップを持つテクスチャも作り直す。docs/editor.md#法線マップの提案
   */
  proposeNormalMaps: (choices: NormalMapGroupChoice[], replaceExisting: boolean) =>
    invoke<ProposalView>("propose_normal_maps", { choices, replaceExisting }),
  /**
   * 提案を、前の答えと注文を AI に送って作り直す (いつも AI。`withConsent` で呼ぶ)。新しい提案は前の提案の後ろに並ぶ。
   * docs/editor.md#ai-への注文と作り直し
   */
  regenerateRigProposal: (
    id: number,
    instructions: string | null,
    consent: ConsentAnswer | null,
    moduleOptions: ModuleRigChoice | null = null,
  ) =>
    invoke<AiReply<ProposalView>>("regenerate_rig_proposal", {
      id,
      instructions,
      consent,
      moduleOptions,
    }),
  /** `excluded` の番号のコマンドを除いた提案を、ドキュメントの複製に適用した姿 (ドキュメントは変えない)。 */
  previewProposal: (id: number, excluded: number[]) =>
    invoke<ProposalPreview>("preview_proposal", { id, excluded }),
  /** 最後にプレビューした姿の .ugoe バンドル (テクスチャ付き)。 */
  getProposalPreviewBundle: async () =>
    new Uint8Array(await invoke<ArrayBuffer>("get_proposal_preview_bundle")),
  clearProposalPreview: () => invoke<void>("clear_proposal_preview"),
  /** `excluded` の番号のコマンドを除いて採用する (1 回の Undo で戻る)。 */
  adoptProposal: (id: number, excluded: number[]) =>
    invoke<ApplyReport>("adopt_proposal", { id, excluded }),
  rejectProposal: (id: number) => invoke<ProposalState>("reject_proposal", { id }),
  /**
   * 採用したモジュールのリグの提案が本体の変形を写した衣装のメッシュのうち、形を編集したものの知らせを消す
   * (今の形を覚え直す)。知らせは `ProposalState.stale_rigs`。
   */
  dismissStaleRigs: () => invoke<ProposalState>("dismiss_stale_rigs"),
  /**
   * apps/editor/src-tauri/src/mcp.rs。エディタの中の MCP サーバ (エージェントをつなぐ) の状態。トークンは含まない。
   * docs/editor.md#エージェントをつなぐ
   */
  mcpBridgeStatus: () => invoke<McpBridgeStatus>("mcp_bridge_status"),
  /** ブリッジを有効・無効にする (設定に残す)。 */
  setMcpBridgeEnabled: (enabled: boolean) =>
    invoke<McpBridgeStatus>("set_mcp_bridge_enabled", { enabled }),
  /** 待ち受けるポート (null なら起動のたびに空いているポート)。 */
  setMcpBridgePort: (port: number | null) =>
    invoke<McpBridgeStatus>("set_mcp_bridge_port", { port }),
  /** トークンを作り直す (HTTP で直接つないだクライアントは設定し直す)。 */
  regenerateMcpBridgeToken: () => invoke<McpBridgeStatus>("regenerate_mcp_bridge_token"),
  /** トークン (写すときだけ読む)。 */
  getMcpBridgeToken: () => invoke<string>("get_mcp_bridge_token"),
  /** エージェントの求め (適用・Undo・Redo・保存) に答える。待っていなければ (時間切れの後など) false。 */
  answerMcpApproval: (id: number, approved: boolean) =>
    invoke<boolean>("answer_mcp_approval", { id, approved }),
  /**
   * Claude Desktop 用の MCP Bundle (.mcpb。同梱の ugoe-mcp を --editor で動かす) をアプリのキャッシュに作り、
   * OS に開かせる (Claude Desktop がインストールの画面を出す)。作ったファイルのパスを返す。
   */
  openClaudeDesktopBundle: () => invoke<string>("open_claude_desktop_bundle"),
  /** Claude Desktop 用の .mcpb を `path` (保存のダイアログで選んだもの。拡張子は .mcpb) に書く。 */
  saveClaudeDesktopBundle: (path: string) => invoke<string>("save_claude_desktop_bundle", { path }),
  /** ブリッジからのイベント (確認の求め・文書や提案の変化)。戻り値で購読をやめる。 */
  onMcpBridgeEvent: (handler: (event: BridgeEvent) => void): Promise<UnlistenFn> =>
    listen<BridgeEvent>("mcp-bridge", (e) => handler(e.payload)),
  /**
   * apps/editor/src-tauri/src/tracking.rs。JSON のテキスト (フェイストラッキングの記録) を `path` に書く。
   * パスは保存のダイアログで選んだもの。拡張子は `.json` だけ。
   */
  writeJsonFile: (path: string, contents: string) =>
    invoke<void>("write_json_file", { path, contents }),
  /**
   * apps/editor/src-tauri/src/dress_up.rs。プロジェクトに残した、プレビューで試すモジュールの一覧
   * (`project.json` の `editor.dress_up`。パスは絶対パス)。
   */
  getPreviewModules: () => invoke<PreviewModule[]>("get_preview_modules"),
  /** プレビューで試すモジュールの一覧をプロジェクトに残す (保存で書き込む。Undo の対象ではない)。 */
  setPreviewModules: (modules: PreviewModule[]) =>
    invoke<ProjectStatus>("set_preview_modules", { modules }),
  /** `.ugoemod` (絶対パス) のバイト列。生のバイト列で届く。 */
  readModuleFile: async (path: string) =>
    new Uint8Array(await invoke<ArrayBuffer>("read_module_file", { path })),
};
