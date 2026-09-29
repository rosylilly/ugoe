---
id: T0637
title: エディタ: AI の提案のプレビューと採用
status: done
milestone: M6
crates: [ugoe-editor-core, apps/editor, docs/manual, docs/editor.md]
depends_on: [T0503, T0504]
assignee: claude-worker-T0637
---

# T0637: エディタ: AI の提案のプレビューと採用

## 目的

AI の `Proposal` (コマンドの列) を、エディタで差分として確かめ、採用 (1 回の Undo で戻せる) ・却下できるようにする。PSD の取り込みから「分類 → リギングの提案」までを画面から実行できるようにする。

## 受け入れ条件

- [x] 提案のパネル: 提案の一覧 (題名・理由)、採用・却下・一部だけの採用 (コマンドの単位)
- [x] プレビュー: 採用する前に、提案を適用した姿をビューポートで確かめられる (文書は変えない)。パラメータを動かして確かめられる
- [x] 採用は `Batch` として 1 回の Undo で戻る。採用後に手で直したセルは `authored` になる
- [x] PSD からの流れ: New from PSD の後に「AI でリギングを提案」を実行でき、進み具合と失敗の理由を出す。API キーがないときの案内
- [x] ヘッドレスのブラウザで、録画した応答を使って確かめる。操作マニュアルに AI の章を足す
- [x] `just ci`・`just gen`・エディタのビルドが通る

## 参考

- `docs/ai-integration.md`
- `docs/adr/0006-ai-proposals.md`

## メモ

- やったこと
  - `ugoe_editor_core::proposal` (`Proposals`): 提案の置き場・プレビュー・採用・却下。置く前に複製 (`Document::detached`。履歴なし) に
    `Batch` で適用して検証する。提案は作ったときの状態 (`Document::state_id`) に対するもので、変わったら stale (Undo で戻れば使える)。
    採用は 1 つの `Batch` (1 回の Undo)。採用した提案は隠すが、Undo で提案を作った状態に戻ると再び出る (AI に尋ね直さずに選び直せる)。
  - **一部だけの採用の扱い:** コマンドの単位で外せる。依存 (`ProposalCommand::requires`、`proposal::analyze`) は「前のコマンドが作った要素か、
    主に変える要素 (`id`・`target`・`element`) の ID を JSON のどこかで指す」で決める (同じ要素への後の編集は前の編集に頼るとみなす。
    推定の設定 → 推定、キーの追加 → セルの置き換えなど)。画面は外すと依存するものも外し、戻すと依存先も戻す。それでも失敗する選び方は
    断る (黙って直さない)。リグの提案で「どれを依存ごと外しても適用できる」ことをテストで確かめた。最初は「作った要素」だけで依存を
    決めていたが、ヘッドレスの確認で `AngleX` を外すと推定 (`infer_keyforms`) が「depth rotation の設定がない」で失敗したので広げた。
  - エディタ: `src-tauri/src/proposals.rs` (IPC 8 つ。本体は `classify_in`・`propose_in` に分けてテストで `AppState` を渡す)、右の列の「AI」タブ
    (`src/lib/ai/ProposalsPanel.svelte`・`proposals.svelte.ts`・`proposals.ts`)。PSD は「New from PSD…」「Import PSD…」で覚え、「Choose PSD…」で替えられる。
    「Propose rig」は未分類なら先に分類する (段階ごとに進み具合を出す)。一覧は同じ対象への続いたコマンドをまとめる (117 コマンドが約 40 行)。
    プレビューは複製のモデル (とバンドル) をビューポートに渡し、編集のツールを隠し、提案のパラメータのスライダーをパネルに出す。
  - 外部に送る: 分類・リグとも `prepare_claude_requests` を通す (T0638 の同意)。分類が複数のリクエストに分かれるときは合計を 1 回の確認で示す
    (`DisclosureView::combined`)。リグの問い直し (最大 2 回の送り直し) は `DisclosureView::retries` で確認に出す (ダイアログに 1 行)。
    「外部に送らない」・キーなしでは「Use Claude」を選べず、理由と「Settings…」を出す。AI なし (名前の規則 + テンプレートの既定値) は何も送らない。
  - 深度 (T0505) は推定器がないので `TurnDepth::Rules` (顔に内接する楕円体) の経路。
  - `EditorDocument.applyWith` (適用の報告を返す IPC を呼んで読み直す) を足した。
  - 文書: `docs/editor.md#ai-の提案`、操作マニュアル `docs/manual/ai.md` (画像 2 枚)、`screen.md` と目次にリンク。
- 確かめたこと
  - `just ci` が通る (rebase 後に `just wasm` → `just gen`)。editor-core のテスト 11・エディタの Rust のテスト 10 (+ `combined` 1)・Vitest 7 を足した。
    採用後に手で直した `derived` のセルが `authored` になることは、実際のリグで `SetKeyformCell` を送って確かめた (既存の規則のまま)。
  - ヘッドレス Chrome (ビルドした画面 + scratchpad の使い捨ての HTTP ブリッジ。本物の `Document`・`Proposals`・`ugoe_ai` を使い、Claude は
    `ugoe-ai/tests/cassettes` の `classify_layers_bustup.json` と `auto_rig_retry.json` を `ReplayTransport` で再生。同意の規則は `ai.rs` を真似た):
    New from PSD (bustup) → AI タブ → Use Claude → Propose rig → 確認 2 回 (同意の前は 0 回・後は 3 回送信 = 分類 1 + リグ 2 (1 回問い直し)) →
    提案 (117 コマンド) → Preview (ドキュメントのパラメータは空のまま、AngleX = 30 で振り向き、EyeLOpen = 0・MouthOpenY = 1 でウィンクと開いた口) →
    `AngleX` のまとまりを外す (12 コマンドが外れ、プレビューが作り直される) → 戻す → Adopt → パラメータ 6 つ → Undo で空・提案が再び出る。
    キーなし (`ai_settings` を差し替え) で「Use Claude」が無効・案内が出る・AI なしで提案できて送信 0 回。
  - 確かめていないこと: 実際の Tauri のウィンドウ (`tauri build` の束ね) と実際の Claude API (キーがない)。
- 範囲外の変更: ルートの `Cargo.lock` (エディタが `ugoe-psd` に依存したため。ワークスペース内のクレートで外部の依存は増えていない)。Command の enum は足していない。
- 後続の候補
  - 分類の結果を画面で直す (ラベルの付け替え) と、分類をプロジェクトに保存する形。
  - 提案の差分の表示 (追加・変更される要素の一覧や、前後の見比べ)。今はコマンドの一覧とプレビューだけ。
  - ugoe-ai の提案の理由 (rationale) が日本語で、画面の英語と混ざる。
  - 深度推定器ができたら「深度を推定する」段階を足す。MCP サーバの提案 (`propose_edit`) を実行中のエディタの一覧に出す形。
  - 提案の中身 (117 コマンド) を段階 (頭・目・口) ごとに分けた提案にすると、一部だけの採用が分かりやすくなる (ugoe-ai の変更)。
