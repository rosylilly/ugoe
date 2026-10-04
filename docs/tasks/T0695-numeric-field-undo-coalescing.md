---
id: T0695
title: 数値の欄の続けての確定で Undo の履歴が積み上がるのをまとめる
status: done
milestone: M9
crates: [ugoe-editor-core, apps/editor, packages/ui, docs]
depends_on: [T0671]
assignee: claude-worker-T0695
---

# T0695: 数値の欄の続けての確定で Undo の履歴が積み上がるのをまとめる

## 目的

T0671 (パネルのコードレビュー) の所見。`@ugoe/ui` の `NumberField` は、矢印キー・PageUp / PageDown の 1 回ごとに `onchange` で確定する
(docs/ui.md#数値の部品と-onchange)。エディタのパネルは `onchange` ごとにコマンドを 1 つ送るので、物理の係数・グルーの強さ・
キーフォームの描画順・タイムラインの時刻などで矢印キーを押し続けると、1 回ごとに Undo の履歴が 1 つ積まれ、適用と読み直しの IPC も
その回数だけ走る。スライダー (`Slider`) は `oncommit` で離したときだけ送るので、つまみでは起きない。

どこでまとめるかは設計の判断が要る (例: editor-core で、同じ対象・同じ種類の続けてのコマンドを短い時間の内なら 1 つの Undo にまとめる。
または `NumberField` に、キーを離したとき・フォーカスを外したときだけ確定する設定を足す)。

## 受け入れ条件

- [x] まとめる場所と規則を決め、必要なら ADR に書いた
- [x] 矢印キーを押し続けたときの Undo の履歴が 1 つ (または決めた規則どおり) になることをテストした
- [x] docs/ui.md (と docs/editor.md の Undo の節) を更新した
- [x] `just ci` が通る

## 参考

- docs/tasks/T0671-review-editor-panels.md (所見の表)
- docs/ui.md#数値の部品と-onchange、packages/ui/src/components/NumberField.svelte
- crates/ugoe-editor-core (Undo の履歴)

## メモ

- **決めたこと (docs/editor.md#undo):** 同じ数値の欄で、同じ種類のコマンドを、前の確定から 1 秒以内 (`COALESCE_WINDOW`) に続けて確定したものを
  1 つの Undo にまとめる。間隔は前の確定から測る (押し続けは何秒でも 1 つ)。ほかの適用・Undo・Redo・保存・本体の読み直しを挟んだらまとめない。
  `NumberField` に「離したときだけ確定」の設定を足す案は採らなかった (押している間に結果が見えなくなり、ビューア・スタジオの部品の意味も変わるため)。
  ADR は書いていない (ADR 0005 のスナップショット方式を覆さず、規則は editor.md に書いた)。
- **やったこと:**
  - editor-core: `Document::apply_coalescing(command, Coalesce { key, at })`・`Coalesce`・`COALESCE_WINDOW` (`document/history.rs`)。
    前のまとめる適用で作った状態のままか (状態の番号) で続きかを確かめる。テストは `undo_tests.rs` (50 回の押し続けで Undo 1 つ、間隔・別の欄・
    間の編集・Undo/Redo・保存・拒否)。
  - Tauri: `apply_command` に `coalesce: Option<String>` (受けた時刻で `apply_coalescing`)。ipc.ts の `applyCommand(command, coalesce?)`。
  - @ugoe/ui: `numberFieldCommit()` (`src/commit.ts`)。`NumberField` が `onchange` を呼ぶ間だけ欄ごとの印 (`$props.id()`) を返す。
    振る舞いは変えていない (ビューア・スタジオは印を読まない。`just ci` の両アプリの検査とテストは通った)。
  - エディタ: `EditorDocument.send`・`apply` が `onchange` の中で呼ばれたら `<印>:<op>` を付けて送り、列の末尾のまだ始まっていない同じ欄の確定は
    組み立てを置き換える (IPC も溜めない)。パネルは変えていない (onchange から同期的に `doc.send`/`doc.apply` まで届くことを確かめた)。テストは `document.test.ts`。
- **やらなかったこと・確かめていないこと:** 実機 (Tauri の画面) での押し続けは確かめていない (単体テストのみ)。
  - `SlotsPanel` の範囲の欄は自前の `busy` で、適用中の確定を捨てる (押し続けると途中の値が送られず、欄の表示とずれることがある)。前からの振る舞いで直していない。
  - `CurveGraph` のキーの時刻の欄は、適用中に次の確定をすると、古い時刻のキーを動かすコマンドになり NotFound になりうる (列で畳まれず走っている 1 つの後に並んだとき)。前から (T0693) の振る舞い。
- **起票の候補:** 上の `SlotsPanel` の `busy` で捨てる確定と、`CurveGraph` のキーの時刻の欄の押し続けを `doc.send` (送る直前に組み立てる) に直すこと。
