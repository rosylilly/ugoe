---
id: T0127
title: 公開仕様: 描画の例 (画素の期待値) と、独立の実装での検証
status: done
milestone: M5
crates: [spec, tools/spec-check, docs, ugoe-render, ugoe-format]
depends_on: [T0120]
assignee: claude-worker-T0127
---

# T0127: 公開仕様: 描画の例 (画素の期待値) と、独立の実装での検証

## 目的

T0120 の後続の候補。描画 (spec/rendering.md: 合成モード・マスク・乗算色とスクリーン色・ライティング) には例も独立の実装もない。

## 受け入れ条件

- [x] 作業者は `crates/` と `docs/spec/` を読まず、`spec/` だけを参照する
- [x] `spec/examples/` に描画の例 (小さな画像と画素の期待値・許容誤差) を足す。GPU の違いで揺れる部分 (ラスタライズの境目・フィルタ) を避ける形を決めて `spec/examples/README.md` に書く
- [x] 参照実装がそれらの例を満たすことを確かめる (`just test-gpu` のテストの追加は team-lead に相談する)
- [x] `tools/spec-check` に CPU の小さなラスタライザを足し、期待値と一致させる
- [x] 見つかった仕様の抜け・曖昧さの一覧と `spec/` への修正 (`docs/spec/` の同期は team-lead に頼む)
- [x] `just ci` が通る

## 参考

- `docs/tasks/T0120-spec-independent-reader.md`・`spec/rendering.md`

## メモ

### 独立性を保った手順

- 作業者 (claude-worker-T0127) は `crates/` と `docs/spec/` を読まず、`spec/`・`tools/spec-check/`・ワークフローの文書だけを参照した。
  参照実装は黒箱として `target/debug/ugoe render` と `--help` の出力 (PNG) だけを使い、`just ci` の失敗はテスト名とパニックの 1 行だけを見た。
- 期待値は参照実装の出力を写していない。調べる画素だけで仕様の式を直接計算する Python の転記 (作業者の scratchpad。コミットしていない) で出し、
  合成とマスクの係数は手で確かめた。TS のラスタライザはそれと別に書き、小数 2 桁の丸めの範囲で一致を確かめた (法線の緑を反転すると落ちることも確かめた)。

### やったこと

- `spec/examples/` に描画の例 5 つ (render-blend・render-masks・render-culling・render-lighting・render-texture)。`model.json` と `render.json`
  (テクスチャの画素・描画先の大きさ・光源・調べる画素の乗算済みの期待値)。形式・作り方の規則 (調べる画素の中心を三角形の辺から 1 画素以上離す、
  テクスチャは 1 色を基本にする)・許容誤差 (各チャンネル 3、すべての画素) を `spec/examples/README.md#rendering-examples` に書いた。
- `tools/spec-check`: CPU のラスタライザ (`render.ts`。画素の中心と左上の規則・乗算済みの双線形・マスク・裏面の除去・ライティング)、
  例の読み込みと照合 (`render-examples.ts`、`test/render.test.ts`。調べる画素が辺から離れていることも検査)、参照実装の `ugoe render` との
  黒箱の比較 (`render-reference.ts`、手で実行)。`png.ts` に任意の画素の PNG の書き出し、`roundtrip.ts` は描画の例の画像を使う。
- 黒箱の比較: PNG で比べられる画素はすべて一致した。
- docs/testing.md に描画の例の項を足した。

### 参照実装の側 (team-lead が足した)

- 参照実装の側は team-lead が足した (作業者は読んでいない): 例の構成の検査が render.json を受け入れるようにし、参照レンダラで各 case を描いて
  画素を照合する GPU のテスト (`just test-gpu`)。そのため crates: に ugoe-render と ugoe-format (テストだけ) を足した。docs/spec の同期も team-lead。
- 結果: 5 例の全 case・全画素が許容誤差 3 以内で一致した。PNG で比べられなかった 3 つも一致: 透明の上の加算 (alpha 0 のまま色が残る)、
  alpha を超える色 (保たれる)、無効な光源 (無視される)。参照レンダラは光源を先頭から 8 個まで使う。

### 見つけた仕様の抜け・曖昧さ (spec/rendering.md を直した)

| # | 箇所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 1 | Drawing a frame / Mesh color | 「三角形が覆う画素」の定義 (中心か面積か) と、共有する辺の上の画素の扱いが無い | 画素の中心で判定し、補間する値は中心で取る。共有する辺の上の中心は 1 つの三角形だけが覆う (SHOULD。左上の規則は informative) |
| 2 | Lights (点光源) | `p` が画素のどこか (隅か中心か) が曖昧。1 倍では最大 6/255 ほど変わる | 画素の中心をビュー変換で戻したキャンバスの位置 (参照実装も同じ) |
| 3 | Normals | 「せん断は法線を変えない」は誤り (極分解の回転で回る) | 軸に沿う拡大縮小は変えない、せん断は回すと直し、数値の例を書いた (参照実装も回す) |
| 4 | Mesh color | 乗算済みの色が alpha を超える結果 (透明の上の加算、1 を超える光) が正しいのか、非乗算への変換で失われること | informative で明記 |
| 5 | Lights | 「光源はいくつでも」と「参照実装は 8 個まで」が食い違い、適合の範囲が分からない | 8 個を超えたら 8 個だけ使ってよい (MAY、どれかは決めない)、Conformance は 8 個までのフレームを対象と明記 |
| 6 | Conformance | 「定義される出力」が実数か 8 ビットの丸めを含むか、描くたびの切り詰め | 実数で計算し描くたびに [0, 1] に切り詰める、許容誤差が 8 ビットの丸めを含む、例はすべての画素を満たす、と明記 |

### 確かめていないこと・残した課題

- ビュー変換が鏡映・回転するもの (CLI で指定できず、例の形式も拡大縮小だけ)、9 個以上の光源、モデルの `lighting: false`、`mask_only` でない可視のマスクを描く色は例にしていない。
- 期待値の導出に使った Python の転記はコミットしていない (T0126 と同じ)。
- `ugoe render` の CLI は長さ 0 の平行光を引数の誤りとして拒否する (描画の仕様の範囲外。黒箱の比較ではその光源を外した)。

### 後続の候補

- 例の形式にビュー変換 (鏡映・回転) を足し、裏面の除去の補正 (spec/rendering.md#back-face-culling) の例を作る。
- `ugoe build` が法線マップのあるバンドルで失敗する問題 (T0128 から。render-lighting も roundtrip.ts で同じ理由で失敗する)。
