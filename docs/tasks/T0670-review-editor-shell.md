---
id: T0670
title: コードレビュー: エディタのフロントエンド (外枠・状態・AI)
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T0670
---

# T0670: コードレビュー: エディタのフロントエンド (外枠・状態・AI)

## 目的

エディタのフロントエンド (外枠・状態・AI) のコードを読み直し、誤り・規約の違反・テストの穴・重複を見つける。小さいものはこのタスクで直し、大きいものは後続のタスクにする。
手順と観点は [docs/workflow/code-review.md](../workflow/code-review.md)。

**範囲:** `apps/editor/src` のうち、App とルートの部品、`lib/` 直下の状態と補助 (document・selection・ipc・shortcuts・layout など)・ダイアログ、`lib/ai/`・`lib/agents/`

## 受け入れ条件

- [x] 範囲のコードを、テストを含めてすべて読んだ
- [x] docs/workflow/code-review.md の観点に加えて、次を確かめた:
  - 状態 (`document.svelte.ts`・`selection.svelte.ts`) の更新の流れ、`$effect` の後片付け・無限の更新
  - ショートカットとフォーカスの扱い、IPC の失敗時の表示
  - AI の提案の承認の流れ (承認の前に適用されないか)
- [x] 直すと決めた所見を直し、バグの修正には先に失敗するテストを足した
- [x] 直さない所見を起票した (milestone は M9。公開 API・設計・範囲の外・半日を超えるもの)
- [x] 「メモ」に所見の表 (重大度・場所・内容・対応) と「横断」の所見を書いた
- [x] `just ci` が通る

## 参考

- docs/workflow/code-review.md
- 対象の README.md と、関係する仕様書・ADR

## メモ

- 後続のタスクの ID の枠: T0689〜T0692 (並行するレビューと番号が衝突しないようにオーケストレーターが割り当てた。足りなければ、残りをメモに書いてオーケストレーターが起票する)

### 読んだもの

`App.svelte`・`main.ts`・`test-setup.ts`、`lib/` 直下 (document・selection・ipc・shortcuts・layout・errors・labels・
emptyState・exportSummary・externalChange・fileDialogs・filePaths・help・i18n・locale・moduleProject とそのテスト、
FileMenu・ExportPanel・BaseLock・HelpButton・LanguageSetting・ダイアログ 5 つ・ModuleProblemBar)、`lib/ai/`・`lib/agents/` の
すべてと、どこの範囲にも入っていない `lib/ui/` (TabStrip・TipButton・baseLock)。生成物 (`lib/generated/`) は読んでいない。
ipc.ts のコマンドの名前は、src-tauri の `generate_handler!` と過不足なく一致していた。

確かめたこと:

- 状態の流れ: 編集は `EditorDocument.apply`/`applyWith`/`undo`/`redo` → IPC → `refresh` で読み直し、`revision` の変化で
  ファイルメニュー・提案を取り直す。重なった読み直しの後勝ちが無かった (下の表)。`$effect` の後片付けは、FileMenu の間隔・
  App のブリッジの購読・ApprovalDialog と ConnectAgents の間隔ですべて返していた。無限の更新は見つからなかった。
- ショートカットとフォーカス: Undo / Redo は入力欄の中では効かないが、ダイアログの中で効いていた (直した)。
- IPC の失敗の表示: 文書は `doc.error`、ファイル操作は知らせの帯、AI は `view.error`/`aiSettings.error`、ブリッジは
  `agentBridge.error`。`appInfo` だけが reject を受けていなかった (直した)。
- AI の提案の承認: 分類・提案は文書を変えず (外部に送る前に `withConsent` の確認)、文書を変えるのは「Adopt」の
  `adoptProposal` だけで、`EditorDocument.applyWith` を通る。エージェントの適用・保存はバックエンドが ApprovalDialog の答えを
  待つ。プレビューは複製の姿だけで、作る間に文書が変わると古い姿を出していた (直した)。テストを足した (proposalsView.test.ts)。

### 所見

| 重大度 | 場所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 中 | `lib/document.svelte.ts` の `refresh` | 重なった読み直し (Undo の連打・エージェントの変更の知らせ・ファイル操作) で、先に始めた読み直しの古い結果が後から上書きしうる。5 つの IPC を 1 つずつ待っていた | 直した (最後に始めた読み直しだけが結果を置く。IPC は並べて待つ。document.test.ts) |
| 中 | `lib/ai/proposals.svelte.ts` の `showPreview` | プレビューを作る間に文書が変わる (エージェントの適用など) と、古い文書に適用した姿を新しい版のプレビューとして出し続ける | 直した (版が変わっていれば捨てる。proposalsView.test.ts) |
| 中 | `App.svelte` の `onKeydown` | 設定・確認・エージェントの求めなどのダイアログの中でも Ctrl/Cmd+Z が後ろの文書を Undo / Redo する | 直した (`shortcuts.ts` の `isInDialog`。docs/editor.md に追記) |
| 中 | `lib/ai/proposals.svelte.ts` | 作り直しの進み具合 (テンプレート文字列) と採用の失敗の文が英語の直書き (ADR 0029) | 直した (`proposals.busy.revising`・`proposals.adoptFailed`) |
| 中 | `lib/filePaths.ts` | OS のファイルダイアログに出るフィルタの名前が英語の直書き | 直した (読むたびに `t()`。`fileMenu.filters.*`) |
| 中 | `lib/ai/` のテスト | 提案の承認の流れ (送らないなら送らない・採用だけが適用する・外したコマンドを渡す) のテストが無い | テストを足した (proposalsView.test.ts) |
| 中 | `lib/FileMenu.svelte` | 保存・上書き・開き直しが `busy` を立てず、フォーカスと間隔の確認が重なりうる。流れのロジックが部品の中にありテストが無い。モジュールの設定の適用が `applyWith` を通らない | T0689 を起票 |
| 中 | `scripts/check-hardcoded-strings.mjs` | 上の直書き (テンプレート文字列・例外の文・フィルタの `name`) を見つけない | T0690 を起票 (範囲の外) |
| 低 | `lib/ipc.ts` | 法線マップの作成の doc コメントが `getNormalMapGroup` に付き、`importPsd` の doc コメントが 2 つに割れていた | 直した |
| 低 | `lib/agents/ConnectAgents.svelte` | ポートの入力欄が、状態の知らせ (クライアントの接続など) のたびに保存した値に戻る | 直した (保存したポートが変わったときだけ入れ直す) |
| 低 | `lib/agents/ConnectAgents.svelte` の `saveBundle` | 保存先の既定のパスを `/` でつなぐ (Windows) | 直した (`joinPath`) |
| 低 | `lib/agents/ConnectAgents.svelte` の `copyToken` | クリップボードに写せなかったことを出さない | 直した (`agents.token.copy.failed`) |
| 低 | `lib/agents/ApprovalDialog.svelte` | 残りの秒数を、求めを受け取った時でなく画面に出した時から数える (前の求めに答えている間に過ぎた時間を数えない) | 直した (`AgentBridge.receivedAt`。bridge.test.ts) |
| 低 | `lib/ai/consent.ts` の `formatBytes` | 1 MB の手前が `1024 KB` になる | 直した (丸めた値で単位を上げる。consent.test.ts) |
| 低 | `lib/NewModuleDialog.svelte` | 開き直すと前に選んだ本体・名前・エラーが残る | 直した (開くたびに戻す。ModuleSettingsDialog と同じ形) |
| 低 | `App.svelte` の `ipc.appInfo()` | 失敗が処理されない Promise の reject になる | 直した (console に出す) |
| 低 | `lib/preview/dressUp.ts` | `MODULE_FILTER` が `lib/filePaths.ts` と重複し、名前が英語のまま | T0690 に含めた (T0672 の範囲) |
| 低 | `lib/ExportPanel.svelte` の `update` | 続けて変えると、前の変更が終わる前の古い設定を元に送る | T0689 に含めた |
| 低 | `lib/agents/bridge.svelte.ts` の `answer` | 時間切れの後の答え (`answer_mcp_approval` が false) を知らせない | 見送り (時間切れは `approval_closed` で画面から消えるので、答えられる時間は短い) |
| 低 | `lib/layout.ts`・`lib/externalChange.ts`・`lib/locale.ts` | localStorage のキーの形が揃っていない (`ugoe.editor.*` と `ugoe-editor.locale`) | 見送り (変えると利用者の覚えた値が消える) |
| 低 | `lib/ai/ProposalsPanel.svelte`・`lib/agents/ConnectAgents.svelte` | 開いたときの読み込みを `$effect` で行う (状態を読まないので 1 回だけ走る) | 見送り (動きは正しい。`onMount` に変えるのは好み) |

起票の候補 (枠の残り T0691・T0692 は使っていない): なし。

### just ci

1 回目の `just ci` は `web-check` で落ちた。fmt-check〜spec はすべて通り、落ちたのは T0670 で触っていない
`src/lib/preview/tracking.test.ts` の `beforeAll` (`preview.svelte` の動的な import) の 10 秒の時間切れだけ
(`Hook timed out in 10000ms`。ロードアベレージ 37 で、並行する約 8 つのワーカーのビルドと重なっていた)。
このテストを単独で流すと、この worktree で 8.5 秒・main で 7.1 秒かかって通る (import が負荷に弱く時間切れの際にある。
T0672 の範囲)。`pnpm --filter @ugoe/editor test` を流し直すと 46 ファイル・360 件すべて通り、`web-check` の残り
(viewer・studio・face-tracking・player・i18n・runtime・ui の check と test)・`just i18n`・`just app-build-check` も通った。

### 横断 (T0008 向け)

- 非同期の読み直しの後勝ち: 画面の状態を IPC で読み直す `refresh` が、重なったときに古い結果で上書きしうる形はエディタの
  パネル・ビューア・スタジオにもありそう。最後に始めたものだけが結果を置く (世代の番号) 形を揃えるとよい。
- ウィンドウで受けるショートカットとダイアログ: `svelte:window onkeydown` で受けるもの (エディタのビューポートのツールの
  Ctrl+A・Delete など、T0672 の範囲) も、ダイアログの中で効いていないか確かめる。`lib/shortcuts.ts` の `isInDialog` が使える。
- 直書きの文言の検査の見落とし (テンプレート文字列・例外の文・フィルタの名前) は全アプリにかかる (T0690)。
- `preview/tracking.test.ts` の `beforeAll` の動的な import は、負荷が高いと既定の 10 秒を超える (並行の作業で `just ci` が
  落ちる)。重い import をするテストは `hookTimeout` を延ばすか import を軽くする。
- 画面の部品の中にある流れのロジック (FileMenu など) はテストが無い。`.svelte.ts` のクラスに出して IPC を偽物にした
  Vitest で確かめる形 (dressUp.test.ts・bridge.test.ts・document.test.ts) に寄せる。
