---
id: T0693
title: エディタのパネルの編集の送り方をそろえる (取りこぼし・古い写し・成否の判定)
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0671]
assignee: claude-worker-T0693
---

# T0693: エディタのパネルの編集の送り方をそろえる (取りこぼし・古い写し・成否の判定)

## 目的

T0671 (パネルのコードレビュー) で見つけた、パネルからコマンドを送る処理のばらつきと誤りを直す。

- **取りこぼし:** 物理 (`PhysicsPanel`・`RigEditor`・`ColliderSection`)・グルー・ポーズ・深度・法線マップは、適用中 (`busy`) に来た編集を
  何も言わずに捨てる。適用は読み直し (`getModelBundle` などの IPC) まで待つので、数値の欄で値を入れて Tab で次の欄に移り、すぐ確定すると
  後の編集が消える。欄は入れた値を出したままなので、利用者は消えたことに気づかない (スロットは欄を無効にしているので起きにくい)。
- **古い写し:** `RigEditor` の `setParticleField`・`patchOutput`、`PhysicsPanel` の `setWorld` などは、画面が持っている写し (`rig`・`world`) から
  リスト全体・ベクトル全体を組み立てる。取りこぼしを直して編集を待たせるだけだと、前の編集を読み直す前の写しから組み立てた
  コマンドが前の編集を戻してしまう。送る直前に最新の値から組み立てる必要がある。
- **成否の判定:** `if (!doc.error)` で判定するもの (ツリー・パラメータ・デフォーマ・タイムライン・表情) と、`apply` の戻り値
  (`null` なら失敗) で判定するものが混ざっている。`doc.error` は読み直しの失敗でも立ち、適用前のエラーが残ることもある。

## 受け入れ条件

- [x] `EditorDocument` に、コマンドを順に送る入口 (例: 前の適用を待ってから、その時点の最新の値でコマンドを組み立てる関数を受け取る) を足し、テストした
- [x] 上のパネルの `send` を置き換え、適用中の編集を捨てない・前の編集を戻さないようにした
- [x] 成否の判定を `apply` の戻り値にそろえた
- [x] `just ci` が通る

## 参考

- docs/tasks/T0671-review-editor-panels.md (所見の表)
- docs/editor.md、apps/editor/src/lib/document.svelte.ts

## メモ

- やったこと:
  - `EditorDocument` に編集の列を足した。`send(build)` は前の編集の適用と読み直しを待ち、`tick()` の後に `build` を呼んで
    その時点の最新の写しからコマンドを組み立てる (null なら送らない。`build` の例外は `error` に出す)。`apply`・`applyWith`・
    `undo`・`redo` も同じ列に並べる (前の失敗は次を止めない)。並べた数を `pending` に出す。テスト 5 件 (適用中の編集を捨てず、
    読み直した後の写しから組み立てる・Undo が前の適用を待つ・null は送らない・失敗は戻り値で返して次へ進む・`build` の例外)。
  - 物理 (`PhysicsPanel`・`RigEditor`・`ColliderSection`・`TemplateSection`): `send` を `SendPhysics` (送る直前の `doc.physics`
    を渡す) にし、リグ・衝突体は ID で覚えて `rigOf`・`colliderOf` で探し直す。粒子・入力・出力・正規化・世界の設定・衝突体は
    送る直前の値から組み立てる (`worldFieldCommand` を physics.ts に出してテスト)。
  - グルー (`updateGlue`)・ポーズ (`send(change)`)・深度 (送る直前の `depthElement`)・法線マップ (`applyWith` の `rethrow` で
    失敗を節に出す。T0689 と同じ形)・デフォーマの作成と外す操作 (`DeformerProperties`) も `doc.send` に。`busy` は
    `doc.pending > 0` にし、並べ替え・削除などのボタンだけを無効にする (数値の欄・選択は無効にしない。無効にすると Tab で移った
    先の欄からフォーカスが外れる)。
  - 成否の判定: ツリー・パラメータ・タイムライン (モーション・カーブ・表情)・デフォーマ、ビューポートのメッシュ編集・形・
    デフォーマのギズモの `doc.error` での判定を、`apply` の戻り値にそろえた。
  - T0671 の所見の「グルーの `keyValue` の完全一致」も直した (キーフォームの格子と同じ `keyTolerance`。テストを追加)。
  - docs/editor.md に「パネルからの編集」を足した。
- やらなかったこと:
  - スロット (`SlotsPanel`) は欄を `busy` で無効にしたまま (`if (busy) return` も残る)。名前の変更で ID が変わるので、ID で
    探し直す形に移すには少し考えが要る。ビューポートの道具 (`GlueOverlay`・`MeshEditTool`・`FormTool`・`DeformerGizmo`・
    `PhysicsOverlay`) のドラッグ中の `busy` も残した (ドラッグの確定は 1 つずつで、取りこぼしの所見の対象外)。
  - 番号しかない要素 (粒子・入力・出力・ポーズのグループと要素) は、並べ替え・削除のボタンを押した直後 (読み直す前) に欄を
    編集すると、編集は列の後ろで新しい並びの同じ番号に当たる。ボタンは `pending` の間無効なので、起きるのはボタンの直後の
    わずかな間だけ。
  - 実機 (`just editor`) で欄を Tab で続けて確定する操作は確かめていない (この環境では画面を操作できない)。列と組み立ての
    順序は Vitest で確かめた。
- 起票の候補:
  - スロットのパネルも `doc.send` に移し、欄を無効にするのをやめる (ID の変更は送る直前に今の ID を引く)。
  - `GlueOverlay` の組の追加・削除も、ドラッグの開始時のグルーでなく送る直前のグルーから `set_glue` を組み立てる。

