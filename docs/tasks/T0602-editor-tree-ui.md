---
id: T0602
title: パーツとデフォーマのツリー UI
status: done
milestone: M2
crates: [apps/editor]
depends_on: [T0601]
assignee: claude-worker-T0602
---

# T0602: パーツとデフォーマのツリー UI

## 目的

左パネルにパーツツリーとデフォーマツリーを表示し、選択・名前の変更・親の付け替えができるようにする。

## 受け入れ条件

- [x] 操作はすべて `Command` を送ることで行う
- [x] `just web-check` が通る
- [x] `just ci` が通る

## 参考

- `docs/editor.md`

## メモ

- **構成**: `lib/panels/TreePanel.svelte` (状態・キーボード・ドラッグ) と `lib/panels/tree/`:
  `model.ts` (モデル JSON のうちツリーが読むフィールドの最小の型。TODO(T0603) で生成型に置き換える)、
  `build.ts` (ツリーの組み立て・平らな行への展開・祖先の探索)、`edit.ts` (ドロップ・名前の変更・削除・
  パーツの追加を `EditCommand` に変換。循環・描画順の違い・親なしにできない要素をここで弾く)、
  `TreeItem.svelte` / `NodeIcon.svelte` (1 行とアイコン)。仕様は `docs/editor.md#ツリー-左`。
- **IPC**: `ipc.ts` の `EditCommand` に、ツリーが送るコマンド (add/remove/rename の part・deformer・mesh、
  set_part_parent、set_deformer_parent / part、set_mesh_parent、set_mesh_properties { part }、reorder_mesh) を足しただけ。
  `EditorDocument` は変えていない (モデルは `doc.model.json` をパネル内で読む。失敗は `doc.error` で判定)。
- **ドラッグ & ドロップ** はポインタイベントで実装した (Windows の WebView2 では Tauri のファイルのドロップ
  (`dragDropEnabled`) が HTML5 の DnD を止めるため)。メッセージはツリーの下 (sticky) に出す
  (上に出すとドラッグ開始時に行がずれて落とし先が変わった。ヘッドレスの検証で見つけた)。
- **テスト**: Vitest (MIT) を入れた。`pnpm --filter @ugoe/editor test` (25 件: ツリーの組み立て・非表示の判定・
  壊れたデータ・キーフォームのセルの選択・ドロップ / 並べ替え / 削除 / 追加のコマンド)。`just web-check` と
  CI の `web` ジョブで実行する。vitest を入れると Biome の `noFocusedTests` が `puppet.fit()` を誤検出するので、
  `*.test.ts` 以外ではこの規則を切った (`biome.json`)。
- **検証したこと**: `just ci`、`pnpm --filter @ugoe/editor check` / `build` (main に rebase した後)。
  ヘッドレスの Chrome 146 で、Tauri の IPC をモック (JS でコマンドを最小限に実装) したビルドを CDP で操作し、
  次を確かめた: 2 つのツリーの表示 (アイコン・`hidden`・`mask`・描画順)、クリック / Shift クリック選択、
  上下左右キー、F2 → 入力 → Enter で `rename_part`、ダブルクリック → Escape で取り消し、パーツの循環と
  デフォーマの循環の拒否 (メッセージ表示)、メッシュをパーツへ (`set_mesh_properties`)、メッシュの並べ替え
  (`reorder_mesh`)、空いた所へ落としてデフォーマを最上位へ、子のあるパーツの削除 → 確認 → HasChildren の表示、
  パーツの追加 → 名前の変更の開始、Undo による読み直しの後も選択が残ること。
- **検証できなかったこと**: 実際の Tauri の WebView での操作。ビューポートからの選択 (T0606) に追従して
  祖先を開き、スクロールすること (ロジックは `pathTo` の単体テストのみ)。
- **残した課題・後続の候補**:
  - `reorder_mesh` は描画順が同じメッシュどうしの前後しか変えないので、描画順が違うメッシュの並べ替えは拒否する。
    PSD の取り込み (T0605) はメッシュごとに違う描画順を付けるので、取り込んだモデルではドラッグで並べ替えられない。
    描画順を変えるコマンド (例: 全キーフォームの `draw_order` をずらす `set_mesh_draw_order`) か、
    インスペクタでの描画順の編集 (T0608) が要る。
  - 親の付け替えは生のコマンドなので見た目が変わる。見た目を保つ付け替えは T0607 で差し替える
    (`edit.ts` の `moveInto` を置き換えればよい)。
  - エラーは `doc.error` (右パネル) とツリーの下の両方に出る。
  - 削除でマスクの参照が外れる・グルーが消えるなどの連鎖は確認に書いていない (バックエンドの規則のまま)。
