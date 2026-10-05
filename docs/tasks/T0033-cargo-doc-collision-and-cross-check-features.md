---
id: T0033
title: cargo doc の出力先の衝突・wasm32 の cargo doc・cross-check の --all-features
status: todo
milestone: M9
crates: [ugoe-cli, scripts, justfile, .github/workflows, docs]
depends_on: []
assignee:
---

# T0033: cargo doc の出力先の衝突・wasm32 の cargo doc・cross-check の --all-features

## 目的

T0027 で `just doc-check`・`just doc-group` と windows-gnu の clippy を CI に入れたとき、次の 3 つが残った (T0027 の候補)。

1. `cargo doc --workspace` で、`ugoe-cli` の bin `ugoe` と `ugoe` クレートの lib のドキュメントの出力先が衝突する
   (`output filename collision at target/doc/ugoe/index.html`)。rustdoc の警告ではないので `-D warnings` では止まらず、
   `just doc` で開く `ugoe` のページが CLI のものに上書きされることがある。
2. `scripts/cross-check.mjs` の `features` は名前を並べて書くので、feature を足したときに書き漏れうる。「すべての feature」(`--all-features`) を書けるようにする。
3. wasm32 だけでビルドするコード (`WebPuppet` など、`ugoe-wasm` の `src/web.rs`・`src/web/`) の doc コメントは、今の `doc-check` (ホストのターゲット) では検査されない。

## 受け入れ条件

- [ ] `cargo doc --workspace --no-deps` が出力先の衝突の警告を出さない (bin に `doc = false` を付けるなど)。`just doc` で開く `ugoe` のページが SDK のものである
- [ ] `scripts/cross-check.mjs` の検査の組で `--all-features` を指定でき、今の書き並べた feature の組のうち置き換えられるものを置き換えた (テストは `just script-check`)
- [ ] wasm32 のターゲットで `cargo doc -p ugoe-wasm --no-deps` を `-D warnings` で流す検査が just のレシピと CI にある (今の警告は直した)
- [ ] docs/testing.md の検査の説明を更新した
- [ ] `just ci` が通る

## 参考

- T0027 のメモ (起票の候補)
- docs/testing.md#ci-のジョブ、docs/testing.md#別のターゲットの検査

## メモ

- T0032 で起票した (元の候補: T0027 の 3 つ)。
