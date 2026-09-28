---
id: T0632
title: エディタ: 親デフォーマからのはみ出しの表示と一覧
status: done
milestone: M3.5
crates: [ugoe-editor-core, apps/editor, docs/manual]
depends_on: []
assignee: claude-worker-T0632
---

# T0632: エディタ: 親デフォーマからのはみ出しの表示と一覧

## 目的

高可動域のリグでは、パラメータを動かすと頂点や子のデフォーマが親のワープの格子の外に出ることが多い。ugoe は範囲外を外挿で追従させるので壊れはしないが、
外挿は格子の中ほど思いどおりに曲がらない。Live2D の実践では「親デフォーマからはみ出した頂点をハイライト」「デフォーマの検証」で確かめている (参考の記事 3)。

## 受け入れ条件

- [x] 今のパラメータの値で、親のワープの格子 ([0,1]²) の外にある頂点・制御点・回転の原点を求める関数 (editor-core。ランタイムの評価結果を使う) とテスト
- [x] ビューポートで、はみ出した点を色で示す表示の切り替え
- [x] 「検証」パネル: すべてのキーフォームのセル (と、軸の端の組み合わせ) を調べ、はみ出す要素・セル・量を一覧にする。押すとその要素とセルへ移動する
- [x] 操作マニュアル。`just ci`・`just gen`・エディタのビルドが通る

## 参考

- 乾物ひもの (髪と小物): https://note.com/himono_vtuber/n/ne6f7c0fdfdaa
- `docs/spec/evaluation.md#範囲外の外挿`

## メモ

- **検出** (`crates/ugoe-editor-core/src/overflow.rs`): ワープの子の局所座標 (= 親の UV) は子自身のキーフォームだけで決まるので、
  子の格子とブレンドシェイプ格子を `ugoe_runtime::keyform::weights` (公開 API) で補間して調べた。ランタイム
  (`ModelInstance::mesh_local_positions`) との一致はテストで確かめている。ワープの制御点の局所座標を返すランタイムの API は
  ないが、この方法なら要らないので ugoe-runtime は変えていない。量は [0,1]² からの UV の距離、既定の許容距離は 0.001。
- **すべてのセル:** 格子とブレンドシェイプ格子の軸のキーのすべての組み合わせ。補間はセル内で各軸について線形なので、UV の距離
  (凸関数) の最大はキーの組み合わせで起きる。キーの外はクランプなので「軸の端の組み合わせ」も含まれる (docs/editor.md に理由を
  書いた)。1 要素 4096 通りを超えたら両端のキーだけ (`truncated`)。サンプルのアバターで 32 要素・269 セル、release で約 50 µs。
- **曲線補間 (T0314) との関係:** rebase 後、軸は `ResolvedBinding::new(param, keys, grid.interpolation)` で作り、ランタイムと同じ
  曲線の重みを使う。曲線の軸では重みが負になり上の凸性の議論が成り立たないので、キーの間を 3 点 (1/4・1/2・3/4) ずつ足して
  調べる (近似。テスト `smooth_axes_are_checked_between_keys`)。ワープの制御点も、親のワールド変換で写すとランタイムの
  `warp_world_points` と一致することをテストした。
- **画面:** 右の列に「Verify」タブ (`panels/verify/`)。「Show overflows in viewport」で今の値のはみ出しに赤い丸
  (`viewport/tools/overflow/OverflowOverlay.svelte`)、「Verify all cells」で要素ごとの一覧、行を押すと要素を選んでスライダーを
  セルの値にする。一度調べた後はモデルが変わるたびに自動で調べ直す。Command は増やしていない (読むだけ)。
  IPC は `find_overflows` / `verify_overflows` (`src-tauri/src/overflow.rs`、`ipc.ts`)。App.svelte と ToolOverlay.svelte の変更は
  タブとオーバーレイの差し込みだけ。
- **確かめたこと:** vite build + `just wasm` + 本物の `Document` への HTTP ブリッジ (スクラッチ領域、リポジトリ外) + ヘッドレス
  Chrome で、サンプルのアバターのプロジェクトの `model.json` に、眉 (`eyebrow_l`) の `BrowLY = -1` のセルを顔のワープの上へ 6% 出す
  変更と、`HeadRotation` の `AngleZ = 30` の原点を体のワープの右へ 12% 出す変更を入れて開き、「Verify all cells」で
  `26 elements · 236 cells`・2 要素 7 セル、行を押して選択・スライダー (`BrowLY = -1` / `AngleZ = 30`)・赤い丸の数 (4 / 5) を確認。
  名前の変更 → Undo → Redo で一覧が自動で調べ直されることも確認。マニュアルの画像はこのときのもの。
- **確かめていないこと:** 実際の Tauri のウィンドウでの表示。
- **後続の候補:** 行を押したときに、ほかのパラメータを既定値に戻す選択肢。はみ出した頂点をメッシュ編集・ギズモの選択
  (`Selection.vertices` / `controlPoints`) に入れる。格子の折り返し (ヤコビアンの符号の反転) の検出。
