---
id: T0626
title: エディタのファイル選択ダイアログ
status: in-progress
milestone: M2
crates: [apps/editor]
depends_on: [T0623]
assignee: claude-worker-T0626
---

# T0626: エディタのファイル選択ダイアログ

## 目的

今のエディタは、開く・保存・書き出し・PSD の取り込みで、パスを入力欄に打ち込む。
T0619 の確認では、Save As に相対パス (`test.ugoeproj`) を入れると、開発サーバーの作業ディレクトリ (`apps/editor/src-tauri`) を基準に解釈され、リポジトリの中の思わぬ場所に保存された。
OS のファイル選択ダイアログを使い、パスの手入力をなくす。

## 受け入れ条件

- [ ] Tauri のダイアログプラグイン (`tauri-plugin-dialog` / `@tauri-apps/plugin-dialog`、ライセンスを確認する) を入れ、権限 (capabilities) を最小限に設定する
- [ ] New from PSD… / Import PSD… / Open / Save As / Export が OS のダイアログを使う (PSD・`.ugoeproj`・`.ugoe` のフィルタ、既定のファイル名、`.ugoeproj` はディレクトリの選択または作成)
- [ ] パスの入力欄は、ダイアログが使えない環境の予備としてだけ残すか、取り除く (決めて書く)。残す場合、相対パスはホームディレクトリ基準で解釈し、どこに保存したかを表示する
- [ ] ヘッドレスのブラウザでの確認のため、ダイアログの呼び出しを差し替えられるようにする (テストではダイアログの代わりに固定のパスを返す)
- [ ] `docs/editor.md` と操作マニュアル (`docs/manual/`、T0624) の該当箇所を更新する
- [ ] `just ci`・`just licenses`・エディタのビルドが通る

## メモ

(作業中に分かったこと、残した課題、後続タスクの候補を書く)
