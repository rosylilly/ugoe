---
id: T0174
title: "CLI: ugoe build がモデル・モジュールの JSON からバンドル・モジュールを書けるようにする"
status: todo
milestone: M9
crates: [ugoe-cli, docs, spec]
depends_on: []
assignee:
---

# T0174: CLI: ugoe build がモデル・モジュールの JSON からバンドル・モジュールを書けるようにする

## 目的

`ugoe build` の入力はプロジェクトのディレクトリ・バンドル・`.ugoemod` だけで、手で書いたモデル・モジュールの JSON (`*.model.json`・
`*.module.json`) からバンドルやモジュールを作る道具が無い。T0472 では Unity のテストの資料 (`unknown-mask.ugoemod`) を作るために
Rust のテスト (`crates/ugoe-format/tests/unity_fixtures.rs`) を書いた。spec/examples/bundles の `short-hair.ugoemod` も JSON から作る道具が無い (T0472 の候補)。

## 受け入れ条件

- [ ] `ugoe build <file>.model.json -o <out>.ugoe` と `ugoe build <file>.module.json -o <out>.ugoemod` が書ける (読み込みの検証を通すこと。
      壊れた JSON・検証の失敗は今の `build` と同じ形のエラー)
- [ ] テクスチャ・法線マップの画像の置き場 (JSON の隣の名前で探す・画像なし) を決め、docs (crates/ugoe-cli/README.md と docs/spec/file-formats.md の
      該当の節があればそこ) に書いた
- [ ] テストで、JSON から書いたバンドル・モジュールを読み直すとモデル・モジュールが同じになることと、`--atlas` との組み合わせ (使えないなら分かるエラー) を確かめる
- [ ] T0472 の `unity_fixtures.rs` の作り直しや spec/examples の資料をこのコマンドで作れることを確かめ、作り方を書いた文書 (bindings/unity/tests/Fixtures/README.md など) を
      直すかをメモに書いた (範囲の外の文書は 1 行の直しだけ)
- [ ] `just ci` が通る

## 参考

- T0472 のメモ (起票の候補)
- crates/ugoe-cli/src/main.rs の `run_build`・`rebuild_module`

## メモ

- T0038 で起票した (元の候補: T0472)。
