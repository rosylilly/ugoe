---
id: T0608
title: エディタ: パラメータとキーフォームの編集
status: done
milestone: M2
crates: [ugoe-editor-core, apps/editor]
depends_on: [T0601, T0402]
assignee: claude-worker-T0608
---

# T0608: エディタ: パラメータとキーフォームの編集

## 目的

パラメータを作り、要素 (メッシュ・デフォーマ・パーツ) にキーを追加し、各キーの上で形を編集できるようにする。Live2D のリギング作業の中心となる操作。

## 受け入れ条件

- [x] パラメータの作成・削除・範囲の変更を UI から行える
- [x] 選択した要素に、パラメータのキー (例: -30 / 0 / 30) を追加・削除する `Command` がある。キーを追加すると、既存のセルから補間した形で新しいセルを埋める (見た目が変わらない)
- [x] 現在のパラメータ値がすべての軸でキーの上にあるとき、ビューポートでの編集 (頂点の移動など) がそのセルに書き込まれる。キーの上にないときは編集を拒否し、最も近いキーへ移動する操作を出す
- [x] 右パネルに、選択した要素のキーの格子 (どのセルがあるか) を表示する
- [x] コマンドにテストがある
- [x] `just ci` が通る

## 参考

- `docs/spec/evaluation.md#1-キーフォームの重み`
- `docs/spec/keyform-inference.md` (推定の UI は T0613)

## メモ

- **コマンド** (`crates/ugoe-editor-core/src/keyform.rs`・`grid.rs`): `add_keyform_key` / `remove_keyform_key` /
  `set_keyform_cell`。対象は T0611 の `KeyformTarget` (`{"kind", "id"}`)。キーの追加は既存の格子を
  評価と同じ重みで補間して新しいセルを埋める (範囲外は端のセル)。新しいセルは格子に `derived` があれば `derived`。
  値は `ugoe_rigging::KeyformValues` で平らにして補間する。`set_keyform_cell` は `touched` に積むので
  T0611 の自動の作り直しが走る。追加・削除は積まない (見た目を保つため。docs/editor.md に理由)。
- **描画順** (リードの依頼、T0602 の並べ替え用): `set_mesh_draw_order { id, draw_order }` はメッシュの格子の
  すべてのセルの `draw_order` を同じ値にする (由来は変えない)。インスペクタの「Draw order」→「Set for all cells」。
  ツリーのドラッグで描画順の違うメッシュを並べ替えるときにも使える。
- **座標の変換** (`space.rs`、Tauri の `local_to_canvas` / `canvas_to_local`): 親の祖先だけを残したモデルに
  仮のメッシュを置いてランタイムで評価し、逆はニュートン法。T0606 の `mesh_edit.rs` の `parent_jacobians`
  (既定値のパラメータでのヤコビ行列) と同じ「評価用のメッシュを足す」やり方だが、こちらは任意のパラメータ値で
  写像と逆写像を返す。統合するなら mesh_edit 側を `space` に寄せられる (後続の候補)。
- **UI**: `ParametersPanel.svelte` (作成・削除・名前・値域・既定値、キーの印、Snap to keys、行ごとのキーの追加・削除)、
  `panels/inspector/keyforms/KeyformInspector.svelte` (軸とキー・セルの格子と由来・今のセル・キーの追加と削除)、
  `viewport/tools/form/FormTool.svelte` (「Form edit」。メッシュの頂点・ワープの制御点・回転の原点をドラッグし、
  離したら `set_keyform_cell` 1 つ。Shift で全点。キーの上にないと拒否して「Move to nearest key」)。
  純粋な関数は `keyforms.ts` (Vitest 12 件)。
- **テスト**: Rust 19 件 (`keyform_tests.rs`: 補間で見た目が変わらないこと (1 軸・範囲外・新しい軸・2 軸の格子を
  多数のパラメータ値でランタイム評価して比較)、由来、拒否、Undo/Redo、JSON、座標の往復)。
- **検証したこと**: ヘッドレス Chrome (WebGL2、SwiftShader) で、`vite build` した画面に Tauri の IPC を
  本物の `ugoe_editor_core::Document` へ中継する使い捨ての HTTP ブリッジ (スクラッチ領域、リポジトリ外) をつないで操作した。
  選択は検証用のビルドだけで window に出した `Selection` から設定。確かめたこと: キーの間にいるときの表示、
  「+ 0.00」でのキー追加 (今のセルの表示に変わる)、軸の追加、2 軸の格子の表示、メッシュ `Palm` の頂点のドラッグ
  (画面で +30,+20px → 親 (回転 0.5 倍) の座標で +20.2,+13.5 と書き込まれ、ハンドルがちょうど 30,20px 動いた)、
  キーの間では拒否されハンドルが消え、「Move to nearest key」で 12 → 0 に移ること、回転デフォーマの原点のドラッグ、
  回転デフォーマ (30 度) の子のワープの制御点のドラッグ (局所座標で回転を戻した量が書かれた)、キーの印の数、
  Snap to keys、パラメータの作成・削除・名前と値域の変更 (batch 1 つ) と Undo。
- **検証できなかったこと**: 実際の Tauri の WebView での操作。ドラッグ中はオーバーレイ (ハンドルと辺) だけが
  動き、wasm の描画は離すまで変わらない (ランタイムに一時的な形を渡す API がないため)。
- **後続の候補**: 回転デフォーマの角度・拡大率の編集 (T0607 のギズモ)。複数の頂点の選択 (今は 1 点か Shift で全点)。
  ドラッグ中の wasm でのプレビュー。パーツの不透明度・メッシュの色などキーフォームの数値フィールドのインスペクタでの編集。
