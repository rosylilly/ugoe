---
id: T0120
title: 公開仕様だけで書いた独立の読み込み器による検証
status: done
milestone: M5
crates: [spec, tools/spec-check, docs, justfile, biome.json]
depends_on: [T0116, T0117, T0119]
assignee: claude-worker-T0120
---

# T0120: 公開仕様だけで書いた独立の読み込み器による検証

## 目的

公開仕様が実装から独立して読めることを確かめるため、ugoe の Rust のコードを読まずに、`spec/` だけを見て別の言語 (例: TypeScript) で `.ugoe` を読み、例のファイルの期待値を再現する小さな実装を作る。仕様の抜けや曖昧さを見つけて直す。

## 受け入れ条件

- [x] 作業者 (エージェント) は `crates/` と `docs/spec/` を読まず、`spec/` だけを参照する (手順として明記する)
- [x] `.ugoe` の読み込みと、例のファイルの評価 (少なくともワープ・回転・補間・ブレンドシェイプ) で期待値と一致する
- [x] 見つかった仕様の抜け・曖昧さの一覧と、`spec/` への修正
- [x] `just ci` が通る (独立の実装のテストを CI に入れるかは決めて書く)

## 参考

- `docs/adr/0021-public-specification.md`

## メモ

### 独立性を保った手順

- 作業者 (claude-worker-T0120) は `crates/` と `docs/spec/` を読まず (grep・ビルドの出力を含む)、`spec/` とワークフローの文書 (AGENTS.md・docs/tasks・docs/adr・docs/workflow・docs/testing.md・justfile・scripts/) だけを参照した。手順は `tools/spec-check/README.md#独立性を保つ手順` に書いた。
- 期待値は `spec/examples/` の手で導いた値だけを使った。新しい例 `smooth-keys` の期待値も、閉じた式 (3 キーは放物線そのもの、4 キーは Hermite の基底と接線を手で計算) から出した (生成に使った式は expected.json の summary と README の表に対応する)。
- 例外として、参照実装の **外から見える挙動** は使った: `assets/compat/` のバイナリを読むこと、`target/debug/ugoe validate` に細工したファイルを与えて受理・拒否を見ること (null・重複・`1.0`・余りのバイトの扱いを仕様に書く前の確認)。ソースとログは読んでいない。

### やったこと

- `tools/spec-check/`: 依存のない TypeScript (Node 24 以上で直接実行。zstd は `node:zlib`、MessagePack は自前)。コンテナの読み込み (spec/bundle.md#reading の手順 1〜8)、JSON の読み込み、モデル・物理・モジュールの検証、評価 (線形・滑らかな補間、ブレンドシェイプ、ワープ・回転、不透明度、グルー、描画順)、モジュールの取り付け。
- テスト 42 件 (`node --test tools/spec-check/test/`): `spec/examples/` の全 8 例が許容誤差内で一致。`assets/compat/dev/` の全ファイルが警告なしに読めて評価でき、`future/` は docs/testing.md の表どおり (任意の機能は警告付きで読めて評価の結果が元と同じ、必須の機能と新しい schema_version は拒否)。コンテナの誤り・機能の宣言・MessagePack・JSON の細かい場合。
- `spec/examples/smooth-keys` を追加 (滑らかな補間の例がなかった)。

### 見つけた仕様の抜け・曖昧さ

| # | 箇所 | 内容 | 対応 |
| --- | --- | --- | --- |
| 1 | model.md Types | JSON で整数のフィールドに `1.0` と書けるか書かれていない (JSON には整数の型がなく、JavaScript の `JSON.parse` は区別できない) | 書き出しは `1` と書く MUST、読み込み器は拒否する SHOULD (参照実装は拒否)。MessagePack の float は値が整数でも float と明記 |
| 2 | model.md Presence | 既定値のある (非 nullable の) フィールドに `null` があるときの読み込み器の扱いがない | 拒否する MUST (参照実装も拒否) |
| 3 | model.md Objects | 同じフィールドが 2 回あるオブジェクトの扱いがない | 書かない MUST、知っているフィールドの重複は拒否 MUST、知らないフィールドの重複は拒否してよい |
| 4 | model.md Types | `rgb` の `r` `g` `b` が全部必須か書かれていない | 必須と明記 (参照実装も一部だけのものを拒否) |
| 5 | model.md Parts | デフォーマの `part` が評価に影響するか、式からしか読み取れない | 影響しないと明記 |
| 6 | bundle.md Payload encoding | MessagePack のペイロードの後ろに余りのバイトがあるときの扱いがない | 書かない MUST、読み込み器は拒否しても無視してもよい (参照実装は無視、この実装は拒否) |
| 7 | bundle.md Chunk table | 長さ 0 のペイロードの重なりと位置の規則 | 何とも重ならないが、整列とファイル長の規則は受けると明記 |
| 8 | bundle.md Reading 手順 4 | `schema_version` の先読みを、圧縮されたペイロードの展開の後に行うこと | 明記 |
| 9 | examples | 滑らかな補間 (`smooth`) の例がない | `smooth-keys` を追加 |
| 10 | model.md Modules | `requires.avatar_level` があり、本体にアバタープロファイルがないときに満たすか | 満たさない (取り付けは失敗する)、比べるのはプロファイルが宣言する `level` と明記。参照実装の挙動は team-lead が確認した |
| 11 | model.md Composition 手順 3 | 複数のスロットの `replaces` から作る `#replaced/…` のパーツを足す順 (スロットの順か) | 未修正。パーツの配列の順はランタイムに意味がない (Arrays and order) ので実害はない。この実装はモデルのスロットの順 |

- spec/ を直した分 (1〜8) の `docs/spec/` の日本語版の同期は team-lead が行った (927e1ef)。10 の追記の同期は未了 (team-lead が行う)。
- 参照実装の `cargo test -p ugoe-format --test spec_examples` は spec/examples/ をすべて読むので、smooth-keys も参照実装と照合され、通った (team-lead が確認)。

### CI

- 独立の実装のテストを CI に入れた: justfile の `spec` レシピ (`just ci` に含まれる) に `node --test tools/spec-check/test/` を足した。依存がなく 1 秒ほどで終わり、公開仕様の例と互換性の資料が変わるたびに、仕様だけで読めることを確かめられるため。
- biome.json の files.includes に `tools/**` を足した (tools/spec-check の TypeScript を整形・lint の対象にするため)。
- justfile と biome.json は範囲外だったため team-lead の承認を得て crates: に足した。
- 型の検査 (`tsc`) は CI に入れていない (`@types/node` が要り、依存を増やさないため)。README に手で行う方法を書いた。

### 後続の候補

- 物理・モーション・効果の独立の実装と、それらの例のファイル (期待値) を `spec/examples/` に足す (いまの例は評価だけ)。
- 描画 (rendering.md) の例 (画素の期待値) と独立の実装。
- `.ugoe` の例のファイルを `spec/examples/` に置く (いまのバイナリの資料は `assets/compat/` で、仕様の一部ではない)。
- 書き出し (spec/bundle.md#writing) の独立の実装と、参照実装との往復の試験。
