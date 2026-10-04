---
id: T0219
title: ugoe render と render_png: 大きさ・時間・ファイルの上限を文書とスキーマに揃える
status: in-progress
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

- [ ] crates/ugoe-cli/README.md の `ugoe render` の節に上の上限を書く
- [ ] `render_png` のスキーマと実際の検査を揃える (どちらに揃えたかをメモに書く)。`time` の上限をスキーマに書く
- [ ] 揃えた上限のテストを足す (CLI か MCP の引数の検査)
- [ ] `just ci` が通る

## 参考

- crates/ugoe-headless/src/view.rs (`output_size`)、crates/ugoe-headless/src/animate.rs (`advance`)、crates/ugoe-headless/src/image.rs (`decode_png`)
- crates/ugoe-mcp/src/tools.rs (`render_png` のスキーマと `RenderArgs`)

## メモ
