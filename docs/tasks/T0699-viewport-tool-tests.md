---
id: T0699
title: ビューポートのツールの操作をコンポーネントのテストで確かめる
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0672]
assignee: claude-worker-T0699
---

# T0699: ビューポートのツールの操作をコンポーネントのテストで確かめる

## 目的

ビューポートのツール (`apps/editor/src/lib/viewport/tools/` の `DeformerGizmo`・`FormTool`・`MeshEditTool`・`PhysicsOverlay`・
`LightingOverlay`・`GlueOverlay`) は、幾何の純粋な関数 (`geometry.ts`・`gizmo.ts`・`form.ts`・`physicsGeometry.ts`) だけに
テストがあり、ポインタの押す・動かす・離す・取り消しから送るコマンド (`set_keyform_cell`・`move_mesh_vertices`・
`set_physics_collider` など) までの流れはテストが無い。T0672 で直した誤り (取り消しでコマンドを送る・送っている間の
押下で表示がずれたまま残る・回転が ±180 度で折り返す) は、どれもこの流れの中にあった。

エディタのテストはノードの環境で動いている (`apps/editor/vite.config.ts`)。`packages/ui` のブラウザでのテスト
(`overlays.test.ts`) と同じ仕組みか、ノードの DOM の実装でツールを描き、偽の `ViewportController`・`EditorDocument` で
コマンドの列を確かめる。

## 受け入れ条件

- [x] ツールのコンポーネントを描いてポインタの操作を送れるテストの仕組みを決め、docs/testing.md に書く
- [x] ギズモ (ワープの点・回転・拡大率・原点)・フォーム編集・メッシュ編集・衝突体のドラッグについて、送るコマンドと取り消しを確かめる
- [x] ダイアログの中のキー操作 (Esc・Delete・Ctrl (Cmd) + A) がツールに届かないことを確かめる
- [x] `just ci` が通る

## 参考

- T0672 のメモ (所見の表)
- docs/testing.md

## メモ

- 仕組み: エディタの Vitest はノードの環境のまま、部品を描くファイルだけ `// @vitest-environment happy-dom` で happy-dom にした
  (`apps/editor` の devDependencies に happy-dom を足した。`packages/ui` と同じ版)。テストのときだけ Svelte のブラウザ向けの実装を読む
  (`vite.config.ts` の `resolve.conditions` を `process.env.VITEST` のときだけ `["browser"]` にする。ビルドと開発サーバの解決は変えない)。
  既存のノードのテストはすべてそのまま通る。docs/testing.md の「エディタの部品のテスト」に書いた。
- 道具: `apps/editor/src/lib/viewport/tools/test/harness.svelte.ts` (`render`・`pointer`・`drag`・`press`・`dialogButton`・
  `FakeController`・`FakeDocument` (`commands` に記録、`pending` で送っている間を作る)・`fakePuppet`・テストのモデル)。
- テスト: `MeshEditTool.test.ts`・`form/FormTool.test.ts`・`deformer/DeformerGizmo.test.ts` (ワープの点・矩形選択・回転 (3/4 周で 270 度・Shift で 15 度刻み)・
  拡大率・原点 (今のセル / Shift ですべてのセル))・`physics/PhysicsOverlay.test.ts` (円・カプセルの端・取り消し・Alt)・
  `ToolOverlay.test.ts` (メッシュ編集の Delete・Ctrl (Cmd) + A・Esc とダイアログ・入力欄)・`glue/GlueOverlay.test.ts` (Esc とダイアログ)。
  T0672 の直しのうち、メッシュ編集の送っている間の押下・物理の取り消し・ダイアログの中のキーは、直しを外すとテストが落ちることを確かめた。
- テストで見つけて直した誤り: フォーム編集とギズモは、送っている間 (`busy`) でも `pointermove`・`pointerup` を受けていたので、
  離した後の移動と別の押下の離す操作で、送っている最中のドラッグを新しい移動量で **もう一度送っていた** (Undo が 2 つになる)。
  メッシュ編集は送り直さないが、送り終わるまで残すプレビューが離した後の移動で動いていた。3 つとも送っている間は移動と離す操作を受けない
  ようにした (本体の変更はこの 2 行ずつだけ)。
- やらなかったこと: `LightingOverlay` のドラッグ (受け入れ条件に無い。`pointercancel` の直しは T0672 で入っている)、`DepthOverlay`・
  `OverflowOverlay` (操作を受けない)。happy-dom はレイアウトと CSS の `pointer-events` を扱わないので、どの要素が押下を受けるか
  (重なり) は確かめていない。
- T0698 (モデルの JSON の共有) が `doc.model` の読み方を変えたら、`FakeDocument` に同じものを足す必要がある (rebase の時点では未マージ)。

### 起票の候補

- `LightingOverlay` の光源のドラッグのテスト (この仕組みで書ける)。
- パネルのドラッグ (タイムラインなど) も、送っている間の `pointermove`・`pointerup` を受けて送り直していないかを見る (T0672 の横断の所見と同じ形)。

### just ci

- `git rebase main` の後に `just ci` が通った (前面で実行、終了コード 0。エディタの Vitest は 58 ファイル・444 件)。
