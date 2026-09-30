---
id: T0641
title: エディタ: 外でプロジェクトが書き換えられたことに気づく
status: done
milestone: M6
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0641
---

# T0641: エディタ: 外でプロジェクトが書き換えられたことに気づく

## 目的

ADR 0023 の 5。`ugoe-mcp` (ファイルを直接扱う形) や別のエディタがプロジェクトを書き換えたときに、エディタが気づいて利用者に知らせ、黙って上書きしない。

## 受け入れ条件

- [x] 開いているプロジェクトのファイルが外で変わったことを検出する (ファイルの監視、またはウィンドウに戻ったとき・保存の前の確認。WSL 越しのパス (`\\wsl.localhost\...`) やネットワークのドライブでも働く方法を選び、理由を書く)
- [x] 未保存の変更がなければ「開き直す」を促す (または自動で開き直す設定)。未保存の変更があれば、開き直す・自分の変更で上書きする・別名で保存する、を選ばせる
- [x] 保存の前に、開いた後で外で変わっていないかを確かめ、変わっていれば確認を求める
- [x] editor-core にテスト (変更の検出の判定)、ヘッドレスのブラウザで画面を確かめる。操作マニュアルに書く
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/adr/0023-editor-mcp-bridge.md`
- `docs/ai-integration.md#ugoeproj-を直接扱う`

## メモ

- **検出の方法: ファイルの指紋を比べる (監視はしない)。** `ugoe_editor_core::ProjectFingerprint` (`fingerprint.rs`) が `Project::load` の
  読むファイルの中身のハッシュと大きさを持ち、エディタはウィンドウに戻ったとき・見えている間は 5 秒ごと・保存の前に比べる。OS のファイルの通知は
  `\\wsl.localhost` (9P) や SMB で届かないことが多く、ADR 0023 の 4 (Windows のエディタ + WSL のエージェント) で働かないため。依存は増やしていない。
  PNG は大きさと更新日時が同じならハッシュを使い回す (JSON は毎回読む)。理由は `docs/editor.md#外での書き換え`。
- **自分の保存:** `ProjectSession::save_to` が保存の直後に指紋を取り直す。`check_external_change` は `project` のロックを持ったまま比べるので保存とぶつからない。
  開くときの指紋は読み込みの前に取る (`Project::load_tracked`)。
- **IPC の変更:** `save_project(overwrite?)` (外で変わっていれば `overwrite` なしでは書かずにエラー)、`reload_project`、`check_external_change`
  (`ExternalChange { files, missing, token }`)。`ipc.ts` と生成した型も更新。
- **画面:** `ExternalChangeDialog.svelte` (FileMenu が出す)。判定は `externalChange.ts` (Vitest)。自動で開き直す設定は localStorage (既定は切)。
  開き直しでは Fit しない (`onchange` に `reloaded` を足した。App.svelte の変更はこの 1 行だけ)。
- **確かめたこと:** ビルドした画面をヘッドレス Chrome + 本物の `Document`・`ProjectFingerprint` への HTTP ブリッジ (スクラッチ領域) で操作した:
  未保存なしで外で `project.json` を変える → 5 秒の確認でダイアログ → Reload で名前が変わる。「+ Part」の後に外で `model.json` を変える →
  focus でダイアログ (4 つの選択肢) → Later → 同じ変更は二度出ない → Save で再びダイアログ → Overwrite で外の変更が消え、自分の保存は変更と見なされない。
  自動で開き直す設定 → 尋ねずに開き直して通知。フォルダを移す → 「The project folder is gone」→ Save here again で作り直す。
- **確かめていないこと:** 実際の Tauri のウィンドウ (特に Windows の WebView2 での `focus` の届き方)、実際の `\\wsl.localhost` とネットワークのドライブでの速さ。
- **残した課題・後続の候補:**
  - `project.json` だけが消えてフォルダが残っているときの「Save here again」は、既存の安全策 (`NotEmpty`: 空でなくプロジェクトでもないフォルダには保存しない) でエラーになる。
  - 開き直すと Undo の履歴が消える (外の変更を 1 つの Command として取り込む形にはしていない)。T0642 (エディタの中の MCP サーバ) ではファイルを介さないので問題にならない。
  - 開き直しの後、ツリーの選択は前の ID のまま残る (「Open…」と同じ)。
