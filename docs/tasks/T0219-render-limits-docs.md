---
id: T0219
title: ugoe render と render_png: 大きさ・時間・ファイルの上限を文書とスキーマに揃える
status: done
milestone: M9
crates: [ugoe-cli, ugoe-mcp, docs]
depends_on: [T0212]
assignee: claude-worker-T0219
---

# T0219: ugoe render と render_png: 大きさ・時間・ファイルの上限を文書とスキーマに揃える

## 目的

T0212 (ugoe-headless のコードレビュー) で、ヘッドレス描画に次の上限と検査を足した。描画の規則の正は
crates/ugoe-cli/README.md の `ugoe render` の節なので、そこと MCP サーバのツールの説明を揃える。

- 出力の大きさ: `size`・`width`・`height` を明示しても、一辺は 1..=16384 (`ugoe_headless::MAX_SIZE`)。
- 時間を進める回数: `floor(time * fps)` は `ugoe_headless::animate::MAX_UPDATES` (60 fps で 1 時間) まで。
- テクスチャ (`--texture` とバンドルの PNG) と法線マップ: 一辺 16384 まで。
- `--texture`・`--physics`・`--motion`・`--expression` のパス: 通常のファイルだけを読む (装置・FIFO・ディレクトリはエラー)。

また、MCP サーバの `render_png` の入力スキーマは `width`・`height` に `maximum: 4096` と書くが、サーバはそれを確かめない
(`RenderArgs` をそのまま `ugoe_headless` に渡すので、実際の上限は 16384)。スキーマに合わせて確かめるか、スキーマを直す。
`time` にも上限を書く。

## 受け入れ条件

- [x] crates/ugoe-cli/README.md の `ugoe render` の節に上の上限を書く
- [x] `render_png` のスキーマと実際の検査を揃える (どちらに揃えたかをメモに書く)。`time` の上限をスキーマに書く
- [x] 揃えた上限のテストを足す (CLI か MCP の引数の検査)
- [x] `just ci` が通る

## 参考

- crates/ugoe-headless/src/view.rs (`output_size`)、crates/ugoe-headless/src/animate.rs (`advance`)、crates/ugoe-headless/src/image.rs (`decode_png`)
- crates/ugoe-mcp/src/tools.rs (`render_png` のスキーマと `RenderArgs`)

## メモ

- やったこと:
  - crates/ugoe-cli/README.md の `ugoe render` に「上限」の項を足した (出力の一辺 1..=16384・`floor(time * fps)` ≤ `MAX_UPDATES`・
    テクスチャと法線マップの一辺 16384・パスは通常のファイルだけ・`render_png` の狭い上限)。crates/ugoe-mcp/README.md の「描画」にも書いた。
  - `render_png` は **スキーマに合わせてサーバが確かめる** ほうにした。明示した `width`・`height` は 1..=4096 (`render::MAX_SIZE`)、
    `time` は 0..=3600 秒 (`render::MAX_TIME` = `MAX_UPDATES / 60`。MCP は 60 fps で進める) を、`Renderer::render` の最初
    (GPU を使う前) で確かめる。スキーマの `maximum` は同じ定数から作る。理由: 描いた PNG を base64 でエージェントに返すので、
    16384² (読み戻し 1 GiB) を長く動くサーバで許す理由が無い。
  - テスト: `render::tests::checks_the_size_and_time_limits_before_touching_the_gpu` (超えた値・0・負・NaN はエラー、ちょうどは通る)、
    tests/stdio.rs の `render_reports_bad_arguments_before_touching_the_gpu` に `width: 4097`・`time: 3601` を足した。
- やらなかったこと: 大きさを指定しないとき (キャンバスの大きさ × 1) は `render_png` でも 4096 に狭めず、CLI と同じ 16384 まで (README に書いた)。
  キャンバスの大きいモデルで既定の描画がエラーになるのを避けるため。
- 起票の候補: `render_png` に大きさを指定しないとき、キャンバスが 4096 を超えるなら 4096 に収まるよう縮めて描く (エラーにせず、log に書く)。
- `just ci` は通った (main に rebase した後)。GPU を使うテスト (`--features gpu-tests`) は流していない (変えたのは GPU の前の検査だけ)。
