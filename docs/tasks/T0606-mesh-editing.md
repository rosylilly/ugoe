---
id: T0606
title: エディタ: メッシュの編集
status: done
milestone: M2
crates: [ugoe-editor-core, ugoe-mesher, ugoe-wasm, apps/editor]
depends_on: [T0601, T0402]
assignee: claude-worker-T0606
---

# T0606: エディタ: メッシュの編集

## 目的

ビューポートでメッシュの頂点を選択・移動・追加・削除し、三角形を作り直せるようにする。

## 受け入れ条件

- [x] ビューポートに、描画に重ねて選択枠・頂点・辺を表示するオーバーレイがある (以降のギズモもこの上に載せる)
- [x] ビューポートの座標とキャンバス座標の変換、頂点のヒットテストがある (評価後の頂点位置は wasm から取得する)
- [x] 頂点の移動・追加・削除と三角形の再生成を `Command` として実装する (UV も更新する)。ドラッグ中はフロントエンドでプレビューだけを行い、確定時に 1 コマンドを送る
- [x] コマンドにテストがある
- [x] `just ci` が通る

## 参考

- `docs/editor.md`

## メモ

- **コマンド** (`ugoe-editor-core`、`src/mesh_edit.rs`、テストは `src/mesh_edit_tests.rs`。式は `docs/editor.md#メッシュの編集`):
  `move_mesh_vertices { id, moves: [{ index, delta }] }`・`add_mesh_vertex { id, position }`・
  `remove_mesh_vertices { id, indices }`・`retriangulate_mesh { id }`。依頼の `mesh` / タプルの代わりに、既存の
  コマンドに合わせて対象は `id`、移動は `{ index, delta }` の構造体 (`VertexMove`) にした。
  - すべて **既定値のパラメータで評価した姿** (基準の姿) を基準にする。評価はモデルの複製に評価用のメッシュを
    足して `ugoe-runtime` で行う (ランタイムに公開 API を足さずに、親の変換と補間後の局所座標を得るため)。
  - 移動: キャンバスのずれを、頂点の局所座標での親の変換のヤコビ行列 (中心差分) の逆で局所に直し、全キーフォームに
    同じ量を足す。親なし・回転デフォーマ (祖先を含む) は厳密。ワープは一次近似 (双線形の交差項の分だけずれる)。
    UV は基準の姿の三角形の重心座標で移動先の点から求め直す (テクスチャはキャンバス上で動かない)。
  - 追加: 点を含む三角形 (外側なら最も近い境界辺の三角形) の重心座標で UV・各キーフォームの位置・深度を補間。
    外側と境界辺の上の点は輪郭に入れる。
  - 削除: 境界をたどって消した頂点を飛ばし、輪郭を保つ。深度とグルーのペアを詰め直す (マスクと当たり判定は
    メッシュ単位の参照なので影響なし)。3 頂点未満は `TooFewVertices`。
  - 作り直し: `ugoe_mesher::triangulate_points` (新設。制約付きドロネー、偶奇規則で輪郭の内側だけ残す) を UV に
    かける。制約辺は今の境界辺。三角形の向きは元のメッシュ (UV で見た向き) に合わせる。
  - 新しいエラー: `EditError::VertexOutOfRange`・`TooFewVertices`・`MeshGeometry`。
- **wasm**: `WebPuppet.hit_test_all(x, y)` (重なった要素を手前から順に。Alt + クリック用) を足した。
- **フロントエンド**: `ToolOverlay.svelte` (選択・輪郭・モード切り替え)、`viewport/tools/` (`scene.svelte.ts`:
  wasm の評価結果の読み出し、`geometry.ts`: 純粋な幾何 (Vitest)、`model.ts`: モデル JSON の要約、
  `MeshEditTool.svelte`: メッシュ編集モード)、`panels/inspector/MeshProperties.svelte`。
  `Selection` に `meshEdit` (編集中のメッシュ ID) を足した。メッシュ編集モードに入るときにパラメータを既定値に戻す
  (コマンドが基準の姿を基準にするため)。
- **検証したこと**: `cargo test -p ugoe-editor-core` (移動: 親なし・回転 (15 度・拡大率 0.5 の入れ子)・ワープ
  (正方格子と台形格子のヤコビ行列)、追加: 内側・外側・辺の上・ワープの子・重複の拒否、削除: 輪郭と凹みの保持・
  グルーと深度の詰め直し・拒否、作り直し: 向きの保持・同じ UV の拒否、JSON、Undo / Redo) と
  `cargo test -p ugoe-mesher` / `-p ugoe-wasm`、Vitest (`tools/geometry.test.ts`)、`just ci`・`just wasm-check`・
  `just wasm`・`pnpm --filter @ugoe/editor check` / `build`。
  ヘッドレスの Chrome (WSL、SwiftShader の WebGL2) で、ビルドしたフロントエンドの IPC を、本物の
  `ugoe_editor_core::Document` を持つ使い捨てのプロセスに中継して (リポジトリ外のスクリプト)、`warp_grid` と
  `nested_masks` のメッシュを合わせたモデルで確認した: クリック選択と Shift での追加、Alt + クリックで 5 枚の重なりを
  手前から順に巡回、「Edit mesh」とダブルクリック (合成した dblclick イベント) でモードに入る、頂点と辺の表示、
  頂点のドラッグ (離したときに `move_mesh_vertices` が 1 つ送られ、ワープの子の頂点が読み直し後に離した位置に来る)、
  矩形選択、空クリックでの解除、Ctrl + クリックでの内側・外側への追加、Delete での削除、Retriangulate、Undo、
  Esc で抜ける、インスペクタの合成モードの変更とマスクの追加。
- **検証していないこと**: 実際の Tauri の WebView での操作、WebGPU での描画 (T0402 と同じ環境の制約)、
  ドラッグ中のプレビューの見た目 (最終位置だけを確認)、キーボード操作のうち Ctrl + A。
- **残した課題・後続の候補**:
  - パラメータを既定値から動かしたまま編集すると、プレビュー (今のパラメータ) と結果 (基準の姿) がずれる。
    今のパラメータでの変換を使う方式にするなら、コマンドにパラメータ値を持たせる必要がある。
  - 重なった場所のダブルクリックは、最初のクリックでいちばん手前が選ばれるので、奥のメッシュの編集には
    Alt + クリックで選んでから「Edit mesh」を使う。
  - ミラーの相手のメッシュは頂点を増減しても追従しない (推定のときに対応が取れずエラーになりうる)。
  - 追加で輪郭の外へ出した頂点の UV は 0〜1 の外になりうる (テクスチャの端の色が伸びる)。
