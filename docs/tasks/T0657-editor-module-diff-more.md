---
id: T0657
title: エディタ: 物理・衝突体・グルーの違いを出す・ビューポートで強調する・.ugoemod にアトラスの対応表
status: done
milestone: M4
crates: [ugoe-editor-core, apps/editor, docs]
depends_on: [T0656]
assignee: claude-worker-T0657
---

# T0657: モジュールの違いの残り

## 目的

T0656 の後続の候補をまとめる。

## 受け入れ条件

- [x] 物理のリグ・衝突体・グルーの違いを中身まで出し、ツリーやパネルで選べるようにする
- [x] プレビューを出ずに、ビューポートで違いの要素を強調する
- [x] .ugoemod にアトラスの対応表を残すかを決める (ファイル形式の変更になるので、残すなら ADR・spec/・docs/spec・SCHEMA_VERSION・互換性の資料)
- [x] `just ci`・`just gen`・`cargo check -p ugoe-editor` が通る

## 参考

- `docs/tasks/T0656-editor-module-diff-details.md` のメモ

## メモ

- **違いの中身 (editor-core `module_compare.rs`):** グルー・物理のリグ・衝突体も要素として添える (`DifferenceKind` に `glue`・
  `physics_rig`・`collider`)。グルーは両端のメッシュ・頂点の組 (数か変わった組の数)・強さのキーフォーム、リグは名前・入力と出力 (数か
  変わった数)・粒子 (数か設定の変わった粒子の数)・正規化・衝突体 (足した・除いた ID)・重力の基準、衝突体は名前・取り付け先・形・位置・
  半径。**除いた要素も `items` に添える** (`text: "removed"`、`element: None`。T0656 では添えなかった。ファイルを付けたプレビューで
  強調できるように)。要素ごとに強調するメッシュ `DifferenceItem.meshes` (モジュールの中の ID。今の編集とファイルの和): パーツ・
  デフォーマはその下のメッシュ、グルーは両端、リグは出力先のデフォーマの下と頂点の出力先のメッシュ、衝突体は空。`element` は
  リグ・衝突体なら作業用の物理の ID、グルーは作業用のモデルのグルーの ID (本体のものは除く)。
- **ビューポートで強調 (`lib/preview/DifferenceOverlay.svelte`・`differenceHighlight.ts`):** 違いの要素の ID を押すと、プレビューの
  ままその要素を強調する (`DressUp.highlight`。もう一度押すか「Clear」でやめ、プレビューを出ると消え、比べ直すと同じ種類と ID の新しい
  結果に替わる)。描くのはスロットにいま付いている版 (`shownVersion`: `<名前> (editing)` か `<名前>`。比べていなければ作業用のモデル)
  の WebPuppet の要素: メッシュの輪郭 (作業用のモデルに同じ頂点数のメッシュがあれば三角形の外周、無ければ凸包)、デフォーマの制御点か
  回転の原点と軸、リグの粒子の鎖 (`physics_json` から合成後のリグを探し、`physics_particles` か初期位置)、衝突体の円・カプセル。
  左下の知らせに、どちらの版か、描けなければ見える方法を出す。
- **選ぶ (`differenceTarget`):** 行の「Select」でプレビューを出て選ぶ。パーツ・デフォーマ・メッシュはツリー、グルーは今の編集にある
  両端のメッシュをツリーで、リグと衝突体は Physics タブを開いて選ぶ (`PhysicsView.rig`・`collider`)。`differenceRef` は
  `differenceTarget` に置き換えた。
- **`.ugoemod` にアトラスの対応表は残さない** と決めた (ファイル形式は変えていない。ADR・spec/・SCHEMA_VERSION・互換性の資料はそのまま)。
  理由は docs/editor.md#プレビューの着せ替え に書いた: 対応表を使うのはこの比較だけでランタイムには要らない。残すと公開仕様・
  `SCHEMA_VERSION`・互換性の資料と `.ugoe` のアトラスとの揃え方まで決め直すことになる。T0656 の比べる側で戻す方法で書き出したファイルとは
  「同じ」と出せ、戻せないのは画像も UV も描き替えたメッシュだけで、そのときは注記して違いとして数えるので見落とさない。
- **IPC:** `compare_preview_module` の戻り値の `DifferenceItem` に `meshes` を足し、`DifferenceKind` を増やした (`just gen` の types.ts)。
  Command の enum は足していない (`OP_COUNT` は 101 のまま)。
- **テスト:** editor-core `module_compare_tests.rs` (`physics_differences_tell_what_changed`: リグの粒子と衝突体の付け外し・衝突体の
  半径・選ぶ ID・強調するメッシュ、デフォーマはその下のメッシュ / `glue_differences_tell_what_changed`: 組と強さ・除いたグルーは
  選べないが両端を強調 / `added_elements_can_be_selected` を除いた要素を添える形に)。Vitest `differenceHighlight.test.ts`
  (選ぶ先・グルーの両端・描いている版・知らせ・凸包)、`dressUp.test.ts` (強調の切り替え・比べ直しで替わる・無くなればやめる)。
- **確かめたこと:** main に rebase 後、`just wasm` 2 つ・`just gen` (差分なし)・`just ci`・`cargo check -p ugoe-editor`・`just test-gpu`
  (`CARGO_BUILD_JOBS=8`)。ヘッドレスの Chrome (SwiftShader) と本物の `Document` への使い捨ての HTTP ブリッジ (scratchpad/T0657/bridge。
  T0656 のものをパスとポートだけ変えた) で、`ugoe sample dress-up --project` の long-hair.ugoeproj: プレビューで見本の
  long-hair.ugoemod は「Same」→ エージェントの編集でリグ Sway の粒子を変え、衝突体 Shoulder (HeadWarp に付ける) を足してリグに付け、
  Back の描画順を 415 に → 「Meshes … Back: draw order 420 → 415」「Physics rigs … Sway: settings of 1 particle; colliders (added:
  Shoulder)」「Colliders: 0 → 1 … Shoulder: added」→ スロットを今の編集にして Sway を押すと粒子の鎖と Back・Front の輪郭
  (「Sway in your current edits」)、Shoulder は円、Back は輪郭。もう一度押すと消える → スロットをファイルにすると Shoulder は
  「Shoulder is not in the file. Put on your current edits to see it.」、Sway は「Sway in the file」→ Sway の「Select」でプレビューを
  出て Physics タブで Sway が選ばれる。横のはみ出し 0。スクリーンショット: scratchpad/T0657/shots/t0657-*.png (マニュアルに
  module-compare-highlight.png を足した)。vite preview とブリッジは止めた。
- **確かめていないこと:** 実際の Tauri のウィンドウ。グルーと除いた要素の強調を画面で (見本にグルーが無い。Vitest と editor-core の
  テストだけ)。デフォーマの強調の制御点・回転の軸の見た目。物理の段を切っているときのリグの鎖 (初期位置に戻るはず)。パラメータにだけ
  出すリグ (描かず「can't be drawn」と出る)。大きなモジュールで、毎フレーム `physics_json` を読み直す重さ (リグ・衝突体を強調している
  ときだけ)。
- **範囲外の変更:** なし (editor-core・apps/editor・docs だけ)。
- **後続の候補:**
  - ポーズ (グループ・追従) の違いも中身まで出す (今は「Pose groups or follows changed」の 1 行だけ)。
  - グルーを選べる画面 (今はツリーで両端のメッシュを選ぶだけ。グルーの一覧や編集の UI が無い)。
  - 強調中は、違いの行に付けたままビューポートをその要素に寄せる (Fit to element)。
