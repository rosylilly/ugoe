# spec-check — 公開仕様だけで書いた独立の読み込み器

公開仕様 ([spec/](../../spec/README.md)) が実装から独立して読めることを確かめるための、TypeScript の小さな実装です
(T0120、[ADR 0021](../../docs/adr/0021-public-specification.md))。参照実装 (`crates/`) のコードは使わず、読みもせずに書きました。

できること:

- `.ugoe` と `.ugoemod` のコンテナの読み込み ([spec/bundle.md](../../spec/bundle.md#reading) の手順 1〜8): ヘッダとチャンクの表の検査、機能の宣言、
  `schema_version` の先読み、zstd の展開、MessagePack の復号、画像の数と署名、知らないチャンク・フィールド・値のフォールバックと警告
- JSON の文書 (`model.json`) の読み込み
- モデル・物理・モジュールの検証 ([spec/model.md](../../spec/model.md#validation))
- 評価 ([spec/evaluation.md](../../spec/evaluation.md)): 線形と滑らかな補間、ブレンドシェイプ、ワープ・回転デフォーマ (ワープの下の回転、外挿を含む)、パーツとデフォーマの不透明度、グルー、描画順
- 着せ替えのモジュールの取り付け ([spec/model.md](../../spec/model.md#composition))

しないこと: 物理・モーション・効果の実行、動的な物理のオフセット、パーツの不透明度の上書き、当たり判定、描画、書き出し。
物理の設定・モーション・表情は、読み込んで型と参照を確かめるだけです。

## 独立性を保つ手順

この実装を直すエージェント (と人間) は、次を守ります。守れない変更 (参照実装の挙動に合わせるなど) をするなら、先に仕様を直します。

1. **読んでよいもの:** `spec/` (本文・JSON Schema・例のファイル)、ワークフローの文書 (`AGENTS.md`・`docs/tasks/`・`docs/adr/`・`docs/workflow/`・`docs/testing.md`)、
   `justfile`・`package.json`・`scripts/` など実装のアルゴリズムを含まないもの。
2. **読んではならないもの:** `crates/` 以下と `docs/spec/` (日本語の開発用の仕様。実装の型名や経緯を含む)。grep や、ビルドの出力・ログで中身を覗くことも含みます。
3. 期待値は `spec/examples/` のもの (手で導いた値) だけを使います。参照実装の出力を期待値にしません。
4. 仕様が曖昧・不足で実装を決められないときは、推測で参照実装に合わせず、仕様の抜けとして記録して `spec/` を直します
   (`spec/` を直したら `docs/spec/` の同期が要ります。同期は `docs/spec/` を読める別の作業者が行います)。
5. 例外として、参照実装の **外から見える挙動** (`ugoe validate` などのコマンドにファイルを与えて受理・拒否を見ること、`assets/compat/` のバイナリを読むこと) は、
   仕様の曖昧さを見つけるために使ってよいとします。ソースやログは読みません。

## 使い方

Node.js 24 以上 (型の除去で `.ts` を直接実行する。zstd は `node:zlib` のもの) だけで動き、依存するパッケージはありません。

```sh
node --test tools/spec-check/test/                                  # テスト
node tools/spec-check/src/cli.ts spec/examples/single-quad/model.json X=0.5   # 評価の結果を JSON で表示
node tools/spec-check/src/cli.ts assets/compat/dev/smooth-arc.ugoe
```

型の検査は、エディタの TypeScript を借りて行えます (`@types/node` が要るため):

```sh
cd tools/spec-check && ../../apps/editor/node_modules/.bin/tsc -p . --typeRoots ../../apps/editor/node_modules/@types
```

## テスト

| ファイル | 内容 |
| --- | --- |
| `test/examples.test.ts` | `spec/examples/` のすべての例を評価し、[許容誤差](../../spec/examples/README.md#tolerances) で期待値と比べる。 |
| `test/compat.test.ts` | `assets/compat/` の資料を読む。`dev/` は警告なしに読めて評価できる、`future/` は仕様どおりに劣化して読める (評価の結果が元のファイルと同じ) か拒否される ([docs/testing.md](../../docs/testing.md#互換性テスト))。 |
| `test/reader.test.ts` | コンテナの誤り (ヘッダ・表・重なり・許されないチャンク・画像の数)、機能の宣言、MessagePack、JSON の整数と `null`、知らない値、検証。 |

### CI

テストは `just spec` (`just ci` に含まれる) で実行する。依存がなく 1 秒ほどで終わるので、公開仕様の例や `assets/compat/` の資料が変わったときに、
仕様だけで読めることが保たれているかを毎回確かめる。参照実装と食い違ったら、どちらが仕様に合っていないかを調べる
(仕様が曖昧なら仕様を直す)。

## 設計の選択

- 計算は倍精度で行う。仕様は単精度以上を求め、許容誤差の内ならよいとしている ([spec/evaluation.md](../../spec/evaluation.md#numbers-and-determinism))。
- 整数と浮動小数点数を区別するため、MessagePack の整数と、JSON の小数部も指数部もない数を `bigint` で読む
  (`JSON.parse` の reviver の `context.source` を使う)。`1.0` は整数のフィールドでは拒否する。
- この実装は書き出しをしないので、知らない値は読むときにフォールバックに置き換えて警告を残す。
  検証に影響する `Parameter.kind` とデフォーマの種類だけは「知らない値」のまま持つ。
- MessagePack の値の後ろに余りのバイトがあるペイロードは拒否する (仕様はどちらも許す。参照実装は無視する)。

## ライセンス

MIT (リポジトリのツールと同じ)。テストは `spec/` の例 (CC BY 4.0) を実行時に読むだけで、このディレクトリに写していません。
