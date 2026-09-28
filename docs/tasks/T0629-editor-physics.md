---
id: T0629
title: エディタ: 物理の設定画面
status: done
milestone: M3
crates: [ugoe-editor-core, ugoe-wasm, apps/editor, docs/manual]
depends_on: [T0305, T0309]
assignee: claude-worker-T0629
---

# T0629: エディタ: 物理の設定画面

## 目的

エディタで物理 (パラメータへの出力と動的物理) を設定し、その場で揺れを確かめられるようにする。

## 受け入れ条件

- [x] 物理の設定を編集する `Command` (設定の追加・削除、入力・出力・粒子・係数の変更、衝突体の追加・削除・編集、重力の基準) と、検証・Undo のテスト
- [x] 物理の設定のパネル: 入力と出力 (パラメータ・デフォーマの角度・オフセット・頂点)、粒子の鎖、係数、衝突体 (円・カプセル) の一覧と編集
- [x] ビューポートで粒子と衝突体を表示し、衝突体をドラッグで動かせる
- [x] 再生中の物理のプレビュー (パラメータや頭を動かすと揺れる)。止めると編集中の姿に戻る
- [x] 操作マニュアルに物理の章を足す
- [x] `just ci`・`just gen` (差分なし)・エディタのビルドが通る

## 参考

- `docs/spec/physics.md`、`docs/adr/0011-dynamic-physics.md`
- `docs/editor.md`

## メモ

- **物理をドキュメントへ移した:** `PhysicsSettings` を Tauri の `ProjectSession` から `ugoe_editor_core::Document` の
  スナップショットに移した (`Document::physics()`・`with_physics`)。モデルと同じ Undo の単位になる。
  T0630 (モーション・表情を `Document` へ移した) の後に rebase し、`Project::from_document(meta, &doc)` になった
  (`ugoe-cli`・テストの呼び出しも合わせた)。`apply_to` は `anim` の後の末尾に `physics` を取る。`OP_COUNT` は 84。
- **コマンド (`physics_edit.rs`、`Command` の末尾に 14 個):** `set_physics_world`・`add/create/remove/rename_physics_rig`・
  `set_physics_inputs/outputs/particles/normalization/rig_colliders/frame`・`add/set/remove_physics_collider`。入力・出力・
  粒子は ID を持たないので並びごと置き換える。適用後に `validate_physics` で新しいエラーを拒否し、値の範囲 (有限・
  `mobility` 0〜1・負の半径・粒子 0 個・出力の粒子 0 など) は `EditError::InvalidPhysics`。`create_physics_rig` は回転
  デフォーマの鎖から動的物理のリグを作る (先端は最後の原点から重力の向きに子孫の最も深い所まで。静止姿勢 = 初期位置)。
- **モデルの編集との関係:** 消したパラメータ・デフォーマ・メッシュを参照する入力・出力を消し、`frame` を外し、取り付け先の
  消えた衝突体はキャンバスに固定して位置を保つ。頂点の追加・削除は頂点の出力の重みを合わせる。ほかの頂点数の変更
  (`regenerate_mesh`) は重みの数の検証エラーで拒否する。テスト 16 個 (`physics_edit_tests.rs`)。
- **ugoe-wasm (team-lead の承認を得て `crates:` に追加):** ビューポートに粒子と衝突体を描き、衝突体のドラッグを
  局所座標に戻すには、粒子の今の位置とデフォーマの写像が要るが、WebPuppet に読む手段がなかった。読み取りだけの
  `WebPuppet.deformer_point`・`deformer_jacobian`・`physics_particles` を新しいファイル (`src/physics_view.rs` と
  `src/web/physics.rs`。ほかのワーカーが編集中のファイルは `web.rs` の `mod` の 1 行だけ) に足した (テストあり、README に記載)。
- **画面:** 右の列の上の区画を「Inspector」/「Physics」のタブにした (`panels/physics/`)。World・リグの一覧・「+ Rig」・
  「+ From selected deformers」・リグの編集 (粒子と係数・入力・出力・重力の基準・Collides with・Input range)・衝突体。
  ビューポートに粒子の鎖と衝突体を重ね (`viewport/tools/physics/PhysicsOverlay.svelte`)、衝突体をドラッグで動かす
  (カプセルは端も)。プレビューは `ViewportController.startPhysicsPreview` (毎フレーム `update(dt)`、物理の段だけ有効)。
  止めると `reset_physics`・物理の段を止める・スライダーの値を書き戻して編集中の姿に戻る。ビューポートの左上にも「Stop physics」。
  純粋な関数は `physics.ts`・`physicsGeometry.ts` (Vitest 20 件)。
- **確かめたこと:** T0624 と同じ方法 (vite build + `just wasm` + 本物の `Document` への HTTP ブリッジ + ヘッドレス Chrome、
  スクラッチ領域) で、`character.psd` から: AngleZ で傾く `頭の回転`、後ろ髪・前髪を「Wrap in deformer」の「Rotation」で包む
  (中心の移動と付け替えはコマンドで代用)、ツリーで選んで「+ From selected deformers」でリグ 2 つ、「Preview」で AngleZ を
  30 にすると根元が頭と動き先端が遅れてついてくる (先端の x が 420 → 526px へ滑らかに追従)、「Stop preview」で
  ビューポートの画像が開始前とバイト単位で一致、「+ Capsule」/「+ Circle」を頭に付けてドラッグ (本体・端) → 局所座標の
  値が期待どおり、Ctrl + Z で戻る、「Collides with」の印、頭を傾けると衝突体もついてくる、Save As (`physics.json`)・Export。
  マニュアルの画像 2 枚 (`physics-rig.png`・`physics-preview.png`)。
- **確かめていないこと:** 実際の Tauri のウィンドウでの操作。衝突体に粒子が当たって押し出される様子の画面での確認
  (押し出し自体は T0305 のテスト)。パラメータ出力だけの従来の物理の画面でのプレビュー。
- **後続の候補:** 頂点の出力の重みの編集 (ブラシで塗る)、粒子の初期位置のビューポートでのドラッグ、プレビュー中の
  スライダーへの物理の出力の表示、モーションの再生と物理のプレビューの同時実行 (T0630 のタイムラインとの統合)、
  `create_physics_rig` の ID を名前からでなく髪らしい名前 (`HairBack` など) で提案する。
