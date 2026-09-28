---
id: T0616
title: M2 の受け入れテスト (PSD から書き出しまでの通し)
status: done
milestone: M2
crates: [ugoe-editor-core (tests/ のみ), ugoe-cli (tests/ のみ)]
depends_on: [T0605, T0606, T0607, T0608, T0611, T0612, T0613, T0614]
assignee: claude-worker-T0616
---

# T0616: M2 の受け入れテスト (PSD から書き出しまでの通し)

## 目的

ロードマップの M2 の完了条件を、GUI を使わずにコマンドだけで通して確かめる自動テストを作る。

> PSD を読み込み、自動メッシュを作り、デフォーマとパラメータを手で設定して、`.ugoeproj` に保存し、`.ugoe` として書き出せる。
> 端の形だけを作れば、中間・斜め・左右反転のキーフォームが推定される。

## 受け入れ条件

- [x] `ugoe-editor-core` の結合テストが、エディタと同じ `Command` だけを使って次を順に行う: PSD (`assets/fixtures/layers.psd`) の取り込み → ワープデフォーマと回転デフォーマの作成 → パラメータ `AngleX` / `AngleY` の作成 → キーの追加 → 端のセル (1 軸セル) の編集 → 加算合成による斜めのセルの推定 → 左右の対のミラー → 深度からの回転 → `.ugoeproj` への保存と読み込み → `.ugoe` の書き出し
- [x] 各段階で、`validate` が通ることと、推定したセルが仕様 (`docs/spec/keyform-inference.md`) の式どおりであることを確かめる
- [x] 書き出した `.ugoe` をランタイム (`ugoe::Puppet` または `ModelInstance`) で読み込み、中間・斜めのパラメータ値で評価できる
- [x] GPU があるとき (`gpu-tests`) は `ugoe render` で斜めの姿勢を PNG に描き、期待する画素を確かめる
- [x] 全体を 1 回の Undo で戻せない操作が無いこと (各段階の Undo / Redo) を確かめる
- [x] `just ci` が通る

## 参考

- `docs/roadmap.md#m2-エディタ-mvp`
- `docs/editor.md`

## メモ

- テスト: `crates/ugoe-editor-core/tests/m2_workflow.rs` (手順は `tests/m2/mod.rs`)、GPU は
  `crates/ugoe-cli/tests/m2_render_gpu.rs` (同じ `m2/mod.rs` を `#[path]` で読み込む)。
  出力はすべて `std::env::temp_dir()` の下に書く。
- 手順と検査:
  1. `import_psd` のコマンドで `layers.psd` を空のモデルに取り込み、既定値の見た目を記録する。
  2. `create_warp_deformer` で `Face` (`肌`・`影`・`目`、2×2)、`create_rotation_deformer` で `Face` を包む
     `Head`、`Face` の下に `EyeL` (`影`) / `EyeR` (`目`) を作る。各段階で既定値の見た目が変わらない
     (ランタイムで評価して 2e-3px 以内)。
  3. `AngleX` / `AngleY` (-30..30) を作り、`Face` に -30/0/30 のキーを加え、斜めのセルを
     `mark_keyform_derived`、端の 4 セルだけを `set_keyform_cell` で編集、`infer_keyforms` (`additive`)。
     斜めのセルが `B + Δ_X + Δ_Y` (制御点と不透明度) で由来が `additive`、端と基準が `authored`。
  4. `AngleX` を自分自身と対 (`set_parameter_mirror`)、`EyeL` ↔ `EyeR` を `set_mirror_pair`、
     `EyeR` の端を `derived` に戻して `mirror_keyforms`。`EyeR(k)` が `EyeL(−k)` の `x' = 2a − x` と
     列の反転で、由来が `mirror`、`mirror_source` が `EyeL`、基準のセルは変わらない。`EyeL` を編集すると
     `EyeR` が同じ Undo 単位でミラーし直され、`apply_with_report` の警告はない (全コマンドで検査)。
  5. 深度付きの 2×2 ワープ `FaceTurn` を `Head` の下に作り、`reparent_preserving_look` で `Face` をその下へ
     (見た目と `Face` の加算合成の関係が保たれる)。キーを加え、`set_keyform_inference` (深度からの回転)、
     基準以外を `mark_keyform_derived`、`infer_keyforms`。全セルが `R_y(yaw)·R_x(pitch)·(p − o) + o` と一致し
     由来が `depth_rotation`。中間のキー `AngleX = 15` を加えると `derived` で入り、推定し直すと式どおり。
  6. コマンドごとの状態を記録し、すべて Undo して各状態 (モデルとテクスチャ) に完全一致で戻り、
     空のプロジェクト (未変更) まで戻ること、Redo で各状態を完全に再現することを確かめる。
  7. `.ugoeproj` に保存・読み込み (等しい)、`.ugoe` に書き出して `read_bundle` (モデルが等しい)。
     `ModelInstance` で既定値 (取り込み直後と同じ見た目)・(15, 0)・(30, 30)・(−22.5, 7.5)・(−30, −30) を評価し、
     有限で、ドキュメントのモデルの評価と一致し、顔は動き背景は動かない。
  8. (`gpu-tests`) `ugoe render` で (30, 30) と既定値を 4 倍で描く。2 回の描画が同じバイト列、ほぼ全体が
     見える (背景の模様にはアルファ 0 の画素がある)、角の画素は既定値と同じ、顔の周りの画素が変わる。
     目視: 既定値では背景の模様の上に `胴` と `肌` (左上に `影`) の四角形。斜めでは `肌` と `影` が右上へ
     移って傾いた平行四辺形になり、キャンバスの上端で切れる。`胴` と背景は変わらない。
- **足りない部分 (後続タスクの候補):** 既存のメッシュ・ワープの深度 (`Mesh::depth` / `WarpDeformer::depth`)
  を設定するコマンドがない。テストでは深度付きのワープを `add_deformer` で作り、`reparent_preserving_look` で
  子を移して回避した (エディタの画面にも深度を塗る操作はない)。T0505 (AI の深度推定) の提案にも
  このコマンドが必要になる。
- 通しで見つかった不具合はなく、`#[ignore]` にしたテストはない。
- 使い方の注意 (仕様どおり): 推定のセルがない格子にキーを加えると新しいセルは `authored` になるので、
  推定させるには `mark_keyform_derived` が要る。
